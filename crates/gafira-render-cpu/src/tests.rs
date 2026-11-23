//! The behaviour tests of `CB1` and `CB2`: the frame, the fill in both
//! formats, the cache, the cull, the cancellation, the progress and the memory
//! the backend accounts for; and the layer stack, its blend and opacity, its
//! nesting and the reuse of its buffers.
//!
//! The tests of `CB4` live in [`image`], a child module of this one: they reuse
//! the scene, the frame and the cache helpers below.

mod effects;
mod image;
mod ops;

use std::sync::{Arc, Mutex};

use gafira_color::{Color, LinearRgba, Rgba};
use gafira_doc::BlendMode;
use gafira_doc::contract::{CancelToken, Progress};
use gafira_geom::{Affine, IRect, Point};
use gafira_ports::common::Bytes;
use gafira_ports::render::{
    BackendCaps, BackendKind, ContentKey, Deadline, FrameCx, Priority, RenderBackend, RenderError,
    RenderOp, RenderPlan, RenderQuality, RenderReport, TargetDesc, TileCache, TileKey, TileTask,
};
use gafira_raster::{AlphaMask, ImageBuf, PixelBuf, PixelFormat, blend_f32, premultiply_u8};
use kurbo::{BezPath, PathEl};

use crate::CpuBackend;

// ---------------------------------------------------------------------------
// Scene helpers
// ---------------------------------------------------------------------------

/// A surface of `width × height` at `origin`, cut out of a `page`-sized page
/// (ARQ-149, O18). The page is named by the caller: a surface smaller than the
/// page still names the page it was cut from, so no descriptor claims a page
/// that cannot hold it.
fn target(
    width: u32,
    height: u32,
    origin: (i32, i32),
    page: (u32, u32),
    format: PixelFormat,
) -> TargetDesc {
    TargetDesc {
        width,
        height,
        origin,
        page,
        format,
    }
}

fn key(content: u128) -> TileKey {
    TileKey {
        content: ContentKey(content),
        level: 0,
        column: 0,
        row: 0,
    }
}

fn task(content: u128, region: IRect, ops: Vec<RenderOp>) -> TileTask {
    TileTask {
        key: key(content),
        priority: Priority::Visible,
        region,
        ops,
    }
}

fn plan(tasks: Vec<TileTask>) -> RenderPlan {
    RenderPlan {
        quality: RenderQuality::Preview,
        tasks,
    }
}

/// A rectangle path in page space.
fn rect_path(x0: f64, y0: f64, x1: f64, y1: f64) -> BezPath {
    BezPath::from_vec(vec![
        PathEl::MoveTo(Point::new(x0, y0)),
        PathEl::LineTo(Point::new(x1, y0)),
        PathEl::LineTo(Point::new(x1, y1)),
        PathEl::LineTo(Point::new(x0, y1)),
        PathEl::ClosePath,
    ])
}

fn fill(x0: f64, y0: f64, x1: f64, y1: f64, color: Color) -> RenderOp {
    RenderOp::FillPath {
        path: Arc::new(rect_path(x0, y0, x1, y1)),
        color,
        transform: Affine::IDENTITY,
    }
}

/// A layer opens on the ops that follow it (02 §6.1).
fn push() -> RenderOp {
    RenderOp::PushLayer
}

/// A layer closes and composites with these parameters (02 §6.1).
fn pop(blend: BlendMode, opacity: f32) -> RenderOp {
    RenderOp::PopLayer { blend, opacity }
}

fn srgb(r: f32, g: f32, b: f32, a: f32) -> Color {
    Color::Srgb(Rgba::new(r, g, b, a))
}

fn white() -> Color {
    srgb(1.0, 1.0, 1.0, 1.0)
}

// ---------------------------------------------------------------------------
// Surface helpers
// ---------------------------------------------------------------------------

/// A tile cache that records what the backend asked of it.
#[derive(Default)]
struct RecordingCache {
    tiles: Vec<(TileKey, Arc<ImageBuf>)>,
    gets: std::cell::RefCell<Vec<TileKey>>,
    puts: Vec<TileKey>,
}

impl RecordingCache {
    fn with(tiles: Vec<(TileKey, Arc<ImageBuf>)>) -> Self {
        Self {
            tiles,
            gets: std::cell::RefCell::new(Vec::new()),
            puts: Vec::new(),
        }
    }

    /// The last tile stored under `key`.
    fn stored(&self, key: TileKey) -> Option<Arc<ImageBuf>> {
        self.tiles
            .iter()
            .rev()
            .find(|(stored, _)| *stored == key)
            .map(|(_, tile)| Arc::clone(tile))
    }

    fn puts(&self) -> &[TileKey] {
        &self.puts
    }
}

impl TileCache for RecordingCache {
    fn get(&self, key: &TileKey) -> Option<Arc<ImageBuf>> {
        self.gets.borrow_mut().push(*key);
        self.stored(*key)
    }

    fn put(&mut self, key: TileKey, tile: Arc<ImageBuf>) {
        self.puts.push(key);
        self.tiles.push((key, tile));
    }

    fn bytes(&self) -> Bytes {
        let bytes: u64 = self
            .tiles
            .iter()
            .map(|(_, tile)| match tile.data() {
                PixelBuf::Rgba8(data) => u64::try_from(data.len()).unwrap_or(0),
                PixelBuf::RgbaF32(data) => u64::try_from(data.len()).unwrap_or(0) * 4,
                PixelBuf::Alpha8(data) => u64::try_from(data.len()).unwrap_or(0),
            })
            .sum();
        Bytes(bytes)
    }

    fn trim(&mut self, budget: Bytes) {
        while self.bytes() > budget {
            if self.tiles.pop().is_none() {
                break;
            }
        }
    }
}

/// A tile of one colour, in the target format.
fn solid_tile32(width: u32, height: u32, pixel: [f32; 4]) -> Arc<ImageBuf> {
    let count = width as usize * height as usize * 4;
    let mut data = vec![0.0f32; count];
    for chunk in data.chunks_exact_mut(4) {
        chunk.copy_from_slice(&pixel);
    }
    Arc::new(ImageBuf::new(width, height, PixelBuf::RgbaF32(data)).expect("the tile"))
}

fn solid_tile8(width: u32, height: u32, pixel: [u8; 4]) -> Arc<ImageBuf> {
    let count = width as usize * height as usize * 4;
    let mut data = vec![0u8; count];
    for chunk in data.chunks_exact_mut(4) {
        chunk.copy_from_slice(&pixel);
    }
    Arc::new(ImageBuf::new(width, height, PixelBuf::Rgba8(data)).expect("the tile"))
}

fn channels(image: &ImageBuf, x: u32, y: u32) -> [f32; 4] {
    let index = (usize::try_from(y).unwrap() * usize::try_from(image.width()).unwrap()
        + usize::try_from(x).unwrap())
        * 4;
    match image.data() {
        PixelBuf::RgbaF32(data) => [
            data[index],
            data[index + 1],
            data[index + 2],
            data[index + 3],
        ],
        PixelBuf::Rgba8(data) => [
            f32::from(data[index]),
            f32::from(data[index + 1]),
            f32::from(data[index + 2]),
            f32::from(data[index + 3]),
        ],
        PixelBuf::Alpha8(_) => panic!("an Alpha8 buffer is not a render target"),
    }
}

fn bytes8(image: &ImageBuf, x: u32, y: u32) -> [u8; 4] {
    let index = (usize::try_from(y).unwrap() * usize::try_from(image.width()).unwrap()
        + usize::try_from(x).unwrap())
        * 4;
    match image.data() {
        PixelBuf::Rgba8(data) => [
            data[index],
            data[index + 1],
            data[index + 2],
            data[index + 3],
        ],
        _ => panic!("the target asked for Rgba8"),
    }
}

/// ARQ-117's preview rule, written out: unpremultiply in linear light, encode
/// with the sRGB curve, round, and premultiply in 8 bits with
/// `(c × a + 127) / 255`. Deliberately not the crate's `quantise_srgb8`, so a
/// test compares the two paths instead of the implementation grading itself.
fn quantise_premultiplied_srgb(pixel: [f32; 4]) -> [u8; 4] {
    let round = |value: f32| {
        let scaled = (value.clamp(0.0, 1.0) * 255.0 + 0.5).floor();
        u8::try_from(scaled.clamp(0.0, 255.0) as u32).unwrap_or(u8::MAX)
    };
    let alpha = pixel[3].clamp(0.0, 1.0);
    let a = round(alpha);
    if a == 0 {
        return [0, 0, 0, 0];
    }
    let encoded =
        LinearRgba::new(pixel[0] / alpha, pixel[1] / alpha, pixel[2] / alpha, alpha).to_srgb();
    [
        premultiply_u8(round(encoded.r), a),
        premultiply_u8(round(encoded.g), a),
        premultiply_u8(round(encoded.b), a),
        a,
    ]
}

/// The bytes of an `Rgba8` image, row by row.
fn rgba8_bytes(image: &ImageBuf) -> Vec<u8> {
    match image.data() {
        PixelBuf::Rgba8(data) => data.clone(),
        _ => panic!("the target asked for Rgba8"),
    }
}

/// True when the pixel is the opaque white the scene fills.
fn is_white(image: &ImageBuf, x: u32, y: u32) -> bool {
    let [r, g, b, a] = channels(image, x, y);
    let opaque = match image.data() {
        PixelBuf::Rgba8(data) => f32::from(data[index_of(image, x, y) + 3]) == 255.0,
        _ => a == 1.0,
    };
    let full = match image.data() {
        PixelBuf::Rgba8(_) => r == 255.0,
        _ => r == 1.0,
    };
    opaque && full && r == g && g == b && b == a
}

/// True when every channel is zero, the transparent page.
fn is_clear(image: &ImageBuf, x: u32, y: u32) -> bool {
    channels(image, x, y).iter().all(|channel| *channel == 0.0)
}

fn index_of(image: &ImageBuf, x: u32, y: u32) -> usize {
    (usize::try_from(y).unwrap() * usize::try_from(image.width()).unwrap()
        + usize::try_from(x).unwrap())
        * 4
}

/// A frame of one backend, for the tests that need the error of `execute`.
struct Frame {
    backend: CpuBackend,
    cx: FrameCx,
}

impl Frame {
    fn open(backend: CpuBackend, target_desc: TargetDesc) -> Self {
        let mut backend = backend;
        let cx = backend.begin_frame(&target_desc).expect("the frame opens");
        Self { backend, cx }
    }

    fn execute(
        &mut self,
        plan: &RenderPlan,
        cache: &mut dyn TileCache,
        token: &CancelToken,
    ) -> Result<RenderReport, RenderError> {
        self.backend
            .execute(&mut self.cx, plan, cache, Deadline::Unbounded, token)
    }

    fn read(&mut self, region: IRect) -> Result<ImageBuf, RenderError> {
        self.backend.read_back(&self.cx, region)
    }
}

/// Receives the fractions a backend reports.
#[derive(Clone, Default)]
struct Recorder {
    fractions: Arc<Mutex<Vec<f32>>>,
}

impl Recorder {
    fn fractions(&self) -> Vec<f32> {
        self.fractions
            .lock()
            .map(|fractions| fractions.clone())
            .unwrap_or_default()
    }
}

impl Progress for Recorder {
    fn report(&self, fraction: f32) {
        if let Ok(mut fractions) = self.fractions.lock() {
            fractions.push(fraction);
        }
    }
}

/// A cache that cancels `token` when its `cancel_at`-th tile is written: it
/// models the caller cancelling the frame between two tiles.
struct CancellingCache {
    token: CancelToken,
    cancel_at: usize,
    puts: usize,
}

impl TileCache for CancellingCache {
    fn get(&self, _key: &TileKey) -> Option<Arc<ImageBuf>> {
        None
    }

    fn put(&mut self, _key: TileKey, _tile: Arc<ImageBuf>) {
        self.puts += 1;
        if self.puts == self.cancel_at {
            self.token.cancel();
        }
    }

    fn bytes(&self) -> Bytes {
        Bytes(0)
    }

    fn trim(&mut self, _budget: Bytes) {}
}

/// Four tiles side by side along one row of a 32 × 8 surface.
fn four_tiles() -> RenderPlan {
    plan(
        (0..4u128)
            .map(|index| {
                let column = i32::try_from(index).unwrap();
                task(
                    index,
                    IRect::new(column * 8, 0, column * 8 + 8, 8),
                    Vec::new(),
                )
            })
            .collect(),
    )
}

/// Runs one plan on a fresh backend and answers the whole surface.
fn run(target_desc: TargetDesc, plan: &RenderPlan) -> ImageBuf {
    let mut cache = RecordingCache::default();
    run_with(target_desc, plan, &mut cache)
}

/// Runs one plan with `cache` and answers the whole surface.
fn run_with(target_desc: TargetDesc, plan: &RenderPlan, cache: &mut RecordingCache) -> ImageBuf {
    let mut backend = CpuBackend::new();
    let mut cx = backend.begin_frame(&target_desc).expect("the frame opens");
    let token = CancelToken::new();
    backend
        .execute(&mut cx, plan, cache, Deadline::Unbounded, &token)
        .expect("the plan runs");
    let region = IRect::new(
        target_desc.origin.0,
        target_desc.origin.1,
        target_desc.origin.0 + i32::try_from(target_desc.width).unwrap(),
        target_desc.origin.1 + i32::try_from(target_desc.height).unwrap(),
    );
    backend.read_back(&cx, region).expect("the surface reads")
}

// ---------------------------------------------------------------------------
// AC1 — the fill lands where the task says, in both formats
// ---------------------------------------------------------------------------

// ARQ-119, ARQ-117: the tile carries the coverage of the path at the region the
// plan puts on the task, in the format the target asks for.
#[test]
fn arq_119_the_fill_is_drawn_at_the_region_of_the_task_in_both_formats() {
    for format in [PixelFormat::Rgba8, PixelFormat::RgbaF32] {
        let scene = plan(vec![task(
            1,
            IRect::new(64, 64, 192, 192),
            vec![fill(100.0, 100.0, 140.0, 140.0, white())],
        )]);
        let surface = run(target(256, 256, (0, 0), (256, 256), format), &scene);
        assert_eq!(
            (surface.width(), surface.height()),
            (256, 256),
            "{format:?}"
        );
        // Inside the fill, at the page position the plan gave it.
        assert!(is_white(&surface, 110, 110), "{format:?}: inside the fill");
        assert!(is_white(&surface, 138, 138), "{format:?}: inside the fill");
        // Inside the tile's region and outside the fill.
        assert!(is_clear(&surface, 70, 70), "{format:?}: before the fill");
        assert!(is_clear(&surface, 160, 160), "{format:?}: after the fill");
        assert!(is_clear(&surface, 90, 120), "{format:?}: left of the fill");
        // Outside the tile: a tile drawn at the origin instead of at its
        // region paints here, and a mask without the tile translation paints
        // the fill at page 164..204.
        assert!(is_clear(&surface, 50, 50), "{format:?}: outside the tile");
        assert!(is_clear(&surface, 180, 180), "{format:?}: no shift");
    }
}

// ARQ-119: the op's placement transform reaches the coverage, not only the
// path: a non-uniform scale is the shape whose coefficients cannot be
// transposed and still agree.
#[test]
fn arq_119_the_placement_transform_reaches_the_coverage() {
    let scene = plan(vec![task(
        1,
        IRect::new(0, 0, 64, 64),
        vec![RenderOp::FillPath {
            path: Arc::new(rect_path(0.0, 0.0, 10.0, 10.0)),
            color: white(),
            // 10 × 10 become 20 × 30 at the page origin.
            transform: Affine::new([2.0, 0.0, 0.0, 3.0, 0.0, 0.0]),
        }],
    )]);
    let surface = run(
        target(64, 64, (0, 0), (64, 64), PixelFormat::RgbaF32),
        &scene,
    );
    assert!(
        is_white(&surface, 15, 25),
        "page (15, 25) is inside 20 × 30"
    );
    assert!(
        is_white(&surface, 19, 29),
        "page (19, 29) is the far corner"
    );
    assert!(
        is_clear(&surface, 25, 15),
        "page (25, 15) is outside: a transposed scale would fill it"
    );
    assert!(is_clear(&surface, 5, 35), "page (5, 35) is outside");
}

// ARQ-119: the coverage is the **source alpha** of the fill. Over an opaque
// backdrop, half a pixel of an opaque white fill leaves the backdrop's alpha
// at one and moves its colour by the coverage alone.
#[test]
fn arq_119_the_coverage_is_the_source_alpha_of_the_fill() {
    let scene = plan(vec![task(
        1,
        IRect::new(0, 0, 2, 1),
        vec![
            fill(0.0, 0.0, 2.0, 1.0, srgb(0.0, 0.0, 0.0, 1.0)),
            // The left half of the first pixel only.
            fill(0.0, 0.0, 0.5, 1.0, white()),
        ],
    )]);
    let surface = run(target(2, 1, (0, 0), (2, 1), PixelFormat::RgbaF32), &scene);
    let [r, g, b, a] = channels(&surface, 0, 0);
    assert_eq!(
        a, 1.0,
        "the backdrop stays opaque: coverage is not its alpha"
    );
    assert!(r == g && g == b, "a grey source over black: {r} {g} {b}");
    assert!(
        (0.49..=0.51).contains(&r),
        "half a pixel of coverage moves the colour by about a half, not by a quarter: {r}"
    );
    assert_eq!(
        channels(&surface, 1, 0),
        [0.0, 0.0, 0.0, 1.0],
        "the second pixel is outside the fill and keeps the backdrop"
    );
}

// ARQ-117: the target's format decides the surface, and only `Rgba8` and
// `RgbaF32` are render targets.
#[test]
fn arq_117_a_target_that_is_not_rgba_is_refused() {
    let mut backend = CpuBackend::new();
    let refused = backend.begin_frame(&target(8, 8, (0, 0), (8, 8), PixelFormat::Alpha8));
    assert!(refused.is_err(), "Alpha8 is not a render target");
    for format in [PixelFormat::Rgba8, PixelFormat::RgbaF32] {
        let mut backend = CpuBackend::new();
        assert!(
            backend
                .begin_frame(&target(8, 8, (0, 0), (8, 8), format))
                .is_ok()
        );
    }
    assert_eq!(backend.caps().kind, BackendKind::Cpu);
}

// ---------------------------------------------------------------------------
// AC2 — the premultiplication rule of ARQ-117, in both formats
// ---------------------------------------------------------------------------

// ARQ-117: the same fill drawn to an `Rgba8` target and to an `RgbaF32` target
// is premultiplied in each format's own vocabulary: linear `f32` for the
// export, and the sRGB 8-bit rounding `(c × a + 127) / 255` for the preview,
// both premultiplied.
#[test]
fn arq_117_both_formats_premultiply_the_same_fill() {
    // A straight sRGB colour at half alpha, over the transparent page.
    let color = srgb(0.2, 0.4, 0.6, 0.5);
    let scene = plan(vec![task(
        1,
        IRect::new(0, 0, 32, 32),
        vec![fill(0.0, 0.0, 32.0, 32.0, color)],
    )]);
    let linear = color.to_linear_srgb();

    // `RgbaF32`: linear light, premultiplied, untouched by any encoding.
    let f32_surface = run(
        target(32, 32, (0, 0), (32, 32), PixelFormat::RgbaF32),
        &scene,
    );
    let [r, g, b, a] = channels(&f32_surface, 10, 10);
    assert_eq!(
        [r, g, b, a],
        [linear.r * 0.5, linear.g * 0.5, linear.b * 0.5, 0.5]
    );
    assert!(
        r <= a && g <= a && b <= a,
        "premultiplied: no channel over alpha"
    );

    // `Rgba8`: unpremultiply, encode with the sRGB curve, premultiply in
    // 8 bits with `(c × a + 127) / 255` on integer division.
    let encoded = linear.to_srgb();
    let alpha = 128u8;
    let byte =
        |value: f32| u8::try_from((value.clamp(0.0, 1.0) * 255.0 + 0.5).floor() as u32).unwrap();
    let expected = [
        premultiply_u8(byte(encoded.r), alpha),
        premultiply_u8(byte(encoded.g), alpha),
        premultiply_u8(byte(encoded.b), alpha),
        alpha,
    ];
    let rgba8_surface = run(target(32, 32, (0, 0), (32, 32), PixelFormat::Rgba8), &scene);
    assert_eq!(bytes8(&rgba8_surface, 10, 10), expected);
    // The rule, pinned by hand: 51·128 and 153·128 are the two channels where
    // truncation and the ARQ-117 rounding disagree.
    assert_eq!(expected, [26, 51, 77, 128]);
}

// ARQ-117, O14: an `Rgba8` surface holds the bytes the cache encoded, byte for
// byte. The write is a copy of the tile the backend encoded, and a window of
// the surface reads back as a copy of its own bytes — no second trip through
// the sRGB curve on either side.
#[test]
fn arq_117_an_rgba8_surface_holds_the_tile_the_cache_encoded() {
    let left = IRect::new(0, 0, 16, 16);
    let right = IRect::new(16, 0, 32, 16);
    let scene = plan(vec![
        task(
            1,
            left,
            // Half a tile: the top eight rows differ from the bottom eight, so
            // a row copied to the wrong place cannot hide.
            vec![fill(0.0, 0.0, 16.0, 8.0, srgb(1.0, 0.0, 0.0, 1.0))],
        ),
        task(
            2,
            right,
            vec![fill(16.0, 0.0, 32.0, 16.0, srgb(0.0, 0.5, 1.0, 1.0))],
        ),
    ]);
    let target_desc = target(32, 16, (0, 0), (32, 16), PixelFormat::Rgba8);
    let mut frame = Frame::open(CpuBackend::new(), target_desc);
    let mut cache = RecordingCache::default();
    frame
        .execute(&scene, &mut cache, &CancelToken::new())
        .expect("the plan runs");
    let surface = frame
        .read(IRect::new(0, 0, 32, 16))
        .expect("the surface reads");
    let packed = rgba8_bytes(&surface);

    // Each tile is on the surface at its own region, row by row.
    assert_eq!(cache.puts(), &[key(1), key(2)], "one tile per task");
    for (content, region) in [(1u128, left), (2u128, right)] {
        let tile = cache
            .stored(key(content))
            .expect("the tile was written back");
        let PixelBuf::Rgba8(bytes) = tile.data() else {
            panic!("a miss is encoded in the frame's format");
        };
        for row in 0..16usize {
            let source = &bytes[row * 16 * 4..(row + 1) * 16 * 4];
            let at = (row * 32 + usize::try_from(region.x0).unwrap()) * 4;
            assert_eq!(
                &packed[at..at + 64],
                source,
                "row {row} at the tile's place"
            );
        }
    }

    // A window smaller than the surface reads back the same bytes: the read
    // has the surface's stride and the window's own.
    let window = frame
        .read(IRect::new(4, 2, 28, 14))
        .expect("the window reads");
    let window = rgba8_bytes(&window);
    for row in 0..12usize {
        let from = ((row + 2) * 32 + 4) * 4;
        let at = row * 24 * 4;
        assert_eq!(
            &window[at..at + 96],
            &packed[from..from + 96],
            "window row {row}"
        );
    }
}

// ARQ-117, ARQ-42, O14: the same page through an `Rgba8` target is the
// `RgbaF32` page quantised exactly once. The expectation is written out from
// the colour space, not taken from the crate's encoder, so the two paths do
// not grade each other.
#[test]
fn arq_117_the_rgba8_read_is_the_f32_page_quantised_once() {
    let scene = plan(vec![
        task(
            1,
            IRect::new(0, 0, 16, 16),
            vec![fill(0.0, 0.0, 16.0, 16.0, srgb(0.9, 0.2, 0.1, 0.4))],
        ),
        task(
            2,
            IRect::new(16, 0, 32, 16),
            vec![
                fill(16.0, 0.0, 30.0, 16.0, white()),
                fill(20.0, 4.0, 28.0, 12.0, srgb(0.1, 0.7, 0.3, 0.75)),
            ],
        ),
    ]);
    let target_desc = |format| target(32, 16, (0, 0), (32, 16), format);

    let linear = run(target_desc(PixelFormat::RgbaF32), &scene);
    let PixelBuf::RgbaF32(values) = linear.data() else {
        panic!("the export surface is f32");
    };
    let once: Vec<u8> = values
        .chunks_exact(4)
        .flat_map(|pixel| quantise_premultiplied_srgb([pixel[0], pixel[1], pixel[2], pixel[3]]))
        .collect();

    let preview = rgba8_bytes(&run(target_desc(PixelFormat::Rgba8), &scene));
    assert_eq!(preview, once, "the preview is the export quantised once");
    assert!(
        once.chunks_exact(4).any(|pixel| pixel != [0, 0, 0, 0]),
        "the scene drew something"
    );

    // A warm cache answers the same bytes: a hit copies the encoded tile
    // instead of decoding it and quantising on the way out.
    let mut cache = RecordingCache::default();
    let first = rgba8_bytes(&run_with(
        target_desc(PixelFormat::Rgba8),
        &scene,
        &mut cache,
    ));
    let second = rgba8_bytes(&run_with(
        target_desc(PixelFormat::Rgba8),
        &scene,
        &mut cache,
    ));
    assert_eq!(first, second, "a reused encoded tile moves no byte");
    assert_eq!(second, once, "the warm path is the same page");
}

// ARQ-117, O14: an `RgbaF32` surface keeps the linear `f32` it was handed. The
// write and the read are copies, so a value no 8-bit curve can represent comes
// back untouched — the export path does not move.
#[test]
fn arq_117_an_rgba_f32_surface_is_the_linear_bytes_it_was_handed() {
    let pixel = [0.123_456_79, 0.456_789_03, 0.789_012_4, 0.9];
    let scene = plan(vec![task(
        1,
        IRect::new(0, 0, 8, 8),
        vec![fill(0.0, 0.0, 8.0, 8.0, white())],
    )]);
    let mut cache = RecordingCache::with(vec![(key(1), solid_tile32(8, 8, pixel))]);
    let surface = run_with(
        target(8, 8, (0, 0), (8, 8), PixelFormat::RgbaF32),
        &scene,
        &mut cache,
    );
    assert!(
        matches!(surface.data(), PixelBuf::RgbaF32(_)),
        "the export surface is f32"
    );
    assert_eq!(
        channels(&surface, 3, 3),
        pixel,
        "the cached f32 reaches the export surface unchanged"
    );
}

// ARQ-117, O14, W15: an `Rgba8` frame runs the sRGB curve exactly once per
// pixel of every tile it encodes, and nowhere else: the write copies the
// encoded tile, a hit copies the cached one, and `read_back` copies the
// surface. The count is of tile pixels, so the third tile's eight columns past
// the surface are encoded (the cache keeps the whole tile) and counted.
#[test]
fn arq_117_an_rgba8_frame_quantises_each_encoded_pixel_once() {
    let scene = plan(
        (0..3u128)
            .map(|index| {
                let x0 = i32::try_from(index).unwrap() * 16;
                task(
                    index,
                    IRect::new(x0, 0, x0 + 16, 16),
                    vec![fill(0.0, 0.0, 48.0, 16.0, srgb(0.9, 0.2, 0.1, 0.6))],
                )
            })
            .collect(),
    );
    let target_desc = target(40, 16, (0, 0), (40, 16), PixelFormat::Rgba8);
    let whole = IRect::new(0, 0, 40, 16);
    let mut cache = RecordingCache::default();

    // A cold frame: three misses of 16 × 16.
    let mut frame = Frame::open(CpuBackend::new(), target_desc);
    let before = crate::surface::quantised();
    frame
        .execute(&scene, &mut cache, &CancelToken::new())
        .expect("the plan runs");
    let encoded = crate::surface::quantised() - before;
    assert_eq!(encoded, 3 * 16 * 16, "once per encoded tile pixel");
    let before = crate::surface::quantised();
    frame.read(whole).expect("the surface reads");
    assert_eq!(
        crate::surface::quantised() - before,
        0,
        "read_back quantises nothing"
    );

    // A warm frame: three hits, so nothing is encoded at all.
    let mut frame = Frame::open(CpuBackend::new(), target_desc);
    let before = crate::surface::quantised();
    frame
        .execute(&scene, &mut cache, &CancelToken::new())
        .expect("the plan runs");
    frame.read(whole).expect("the surface reads");
    assert_eq!(
        crate::surface::quantised() - before,
        0,
        "a hit is copied, not decoded and encoded again"
    );
}

// ---------------------------------------------------------------------------
// AC3 — the cache: a hit is reused, a miss is written back at the plan's key
// ---------------------------------------------------------------------------

// ARQ-44 (partial): a tile the cache holds is reused as it stands and the path
// is not covered again; a tile it does not hold is covered and written back
// under the key of the plan.
#[test]
fn arq_44_a_cached_tile_is_reused_and_a_miss_is_written_back_at_the_key_of_the_plan() {
    // A hit: the cached tile answers every pixel, so the white fill never runs.
    let hit = plan(vec![task(
        1,
        IRect::new(0, 0, 32, 32),
        vec![fill(0.0, 0.0, 32.0, 32.0, white())],
    )]);
    let mut cache =
        RecordingCache::with(vec![(key(1), solid_tile32(32, 32, [1.0, 0.0, 0.0, 1.0]))]);
    let surface = run_with(
        target(32, 32, (0, 0), (32, 32), PixelFormat::RgbaF32),
        &hit,
        &mut cache,
    );
    assert_eq!(
        channels(&surface, 10, 10),
        [1.0, 0.0, 0.0, 1.0],
        "the cached tile answers, not the fill"
    );
    assert!(cache.puts().is_empty(), "a hit is not written back");
    assert_eq!(
        cache.gets.borrow().as_slice(),
        &[key(1)],
        "the cache is asked with the key of the plan"
    );

    // A miss: each tile is written under its own key, with its own pixels.
    let miss = plan(vec![
        task(
            10,
            IRect::new(0, 0, 64, 32),
            vec![fill(0.0, 0.0, 64.0, 32.0, srgb(1.0, 0.0, 0.0, 1.0))],
        ),
        task(
            11,
            IRect::new(64, 0, 128, 32),
            vec![fill(64.0, 0.0, 128.0, 32.0, srgb(0.0, 0.0, 1.0, 1.0))],
        ),
    ]);
    let mut cache = RecordingCache::default();
    let surface = run_with(
        target(128, 32, (0, 0), (128, 32), PixelFormat::RgbaF32),
        &miss,
        &mut cache,
    );
    assert_eq!(cache.puts(), &[key(10), key(11)], "one write per plan key");
    let first = cache.stored(key(10)).expect("the first tile");
    assert_eq!((first.width(), first.height()), (64, 32));
    assert_eq!(channels(&first, 5, 5), [1.0, 0.0, 0.0, 1.0]);
    let second = cache.stored(key(11)).expect("the second tile");
    assert_eq!(channels(&second, 5, 5), [0.0, 0.0, 1.0, 1.0]);
    // And the surface holds each tile at its own region.
    assert_eq!(channels(&surface, 10, 10), [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(channels(&surface, 74, 10), [0.0, 0.0, 1.0, 1.0]);
}

// ARQ-44 (partial), D5: a cached tile of the other format is a miss, because a
// `TileKey` carries no format and one cache may serve both qualities.
#[test]
fn arq_44_a_cached_tile_of_the_other_format_is_a_miss() {
    let scene = plan(vec![task(
        7,
        IRect::new(0, 0, 16, 16),
        vec![fill(0.0, 0.0, 16.0, 16.0, white())],
    )]);
    let mut cache = RecordingCache::with(vec![(key(7), solid_tile8(16, 16, [255, 0, 0, 255]))]);
    let surface = run_with(
        target(16, 16, (0, 0), (16, 16), PixelFormat::RgbaF32),
        &scene,
        &mut cache,
    );
    assert_eq!(
        channels(&surface, 4, 4),
        [1.0, 1.0, 1.0, 1.0],
        "the fill is covered instead of reusing the 8-bit tile"
    );
    let stored = cache.stored(key(7)).expect("the tile");
    assert!(
        matches!(stored.data(), PixelBuf::RgbaF32(_)),
        "the miss is written in the frame's format"
    );
}

// ---------------------------------------------------------------------------
// AC4 — the cull: an op whose bounds miss the tile is skipped
// ---------------------------------------------------------------------------

// 02 §6.2 (the cull, as corrected by the XP round): an operation that misses
// the tile is skipped, and skipping it changes no pixel. The cull must never
// drop an op that contributes, so a fill that only partly overlaps the tile is
// drawn — including one that starts inside the tile but not at its origin.
#[test]
fn spec_02_6_2_the_cull_keeps_a_fill_that_partly_overlaps_the_tile() {
    let cases = [
        // Starts after the tile's origin and ends outside it.
        (
            IRect::new(128, 128, 192, 192),
            (140.0, 140.0, 300.0, 300.0),
            (150u32, 150u32),
        ),
        // Starts before the tile and ends inside it.
        (
            IRect::new(128, 128, 192, 192),
            (100.0, 100.0, 160.0, 160.0),
            (135u32, 135u32),
        ),
        // Covers the tile completely and then some.
        (
            IRect::new(32, 32, 96, 96),
            (0.0, 0.0, 512.0, 512.0),
            (60u32, 60u32),
        ),
    ];
    for (region, (x0, y0, x1, y1), (x, y)) in cases {
        let scene = plan(vec![task(1, region, vec![fill(x0, y0, x1, y1, white())])]);
        let surface = run(
            target(256, 256, (0, 0), (256, 256), PixelFormat::RgbaF32),
            &scene,
        );
        assert!(
            is_white(&surface, x, y),
            "{region:?}: the fill at ({x0}, {y0})-({x1}, {y1}) overlaps and must be drawn"
        );
    }
}

// 02 §6.2: an op whose bounds miss the tile changes no pixel, so a plan with
// it and a plan without it give the same bytes.
#[test]
fn spec_02_6_2_the_cull_skips_an_op_that_misses_the_tile() {
    let region = IRect::new(0, 0, 64, 64);
    let without = plan(vec![task(
        1,
        region,
        vec![fill(8.0, 8.0, 56.0, 56.0, white())],
    )]);
    let with = plan(vec![task(
        1,
        region,
        vec![
            fill(8.0, 8.0, 56.0, 56.0, white()),
            fill(4096.0, 4096.0, 8192.0, 8192.0, srgb(1.0, 0.0, 0.0, 1.0)),
        ],
    )]);
    let target_desc = target(64, 64, (0, 0), (64, 64), PixelFormat::RgbaF32);
    let a = run(target_desc, &without);
    let b = run(target_desc, &with);
    assert_eq!(a.data(), b.data(), "the off-tile fill changes no pixel");
    assert!(is_white(&b, 20, 20), "the fill inside the tile is drawn");
    assert!(is_clear(&b, 2, 2), "and stays inside its own path");
}

// ---------------------------------------------------------------------------
// AC5 — a cancelled token stops the loop before the next tile
// ---------------------------------------------------------------------------

// ARQ-139: the token is checked before every tile, the loop stops with the
// cancellation the port fixes, and no partial surface is returned.
#[test]
fn arq_139_a_cancelled_token_stops_before_the_next_tile() {
    let token = CancelToken::new();
    let mut cache = CancellingCache {
        token: token.clone(),
        cancel_at: 1,
        puts: 0,
    };
    let scene = four_tiles();
    let mut frame = Frame::open(
        CpuBackend::new(),
        target(32, 8, (0, 0), (32, 8), PixelFormat::RgbaF32),
    );
    let error = frame
        .execute(&scene, &mut cache, &token)
        .expect_err("the cancelled frame fails");
    assert!(
        matches!(error, RenderError::Cancelled(_)),
        "the port's cancellation, not another error: {error:?}"
    );
    assert_eq!(error.to_string(), "operation cancelled");
    assert_eq!(cache.puts, 1, "the loop stopped before the second tile");
    assert!(
        frame.read(IRect::new(0, 0, 32, 8)).is_err(),
        "a cancelled frame returns no partial surface"
    );
}

// ARQ-139: a token cancelled before the first tile stops before drawing.
#[test]
fn arq_139_a_token_cancelled_before_the_first_tile_stops_at_once() {
    let token = CancelToken::new();
    token.cancel();
    let mut cache = RecordingCache::default();
    let mut frame = Frame::open(
        CpuBackend::new(),
        target(32, 8, (0, 0), (32, 8), PixelFormat::RgbaF32),
    );
    assert!(
        frame.execute(&four_tiles(), &mut cache, &token).is_err(),
        "a cancelled token stops at once"
    );
    assert!(cache.puts().is_empty(), "nothing was covered");
}

// ---------------------------------------------------------------------------
// AC6 — the progress follows the tiles done over the tiles planned
// ---------------------------------------------------------------------------

// ARQ-139: the fraction is non-decreasing in `[0, 1]`, follows the tiles done
// over the tiles planned, and only reaches `1.0` on a normal end.
#[test]
fn arq_139_the_progress_is_tiles_done_over_tiles_planned() {
    // A plan that ends normally walks the whole ladder and ends at `1.0`.
    let sink = Recorder::default();
    let backend = CpuBackend::new().with_progress(Arc::new(sink.clone()));
    let mut frame = Frame::open(
        backend,
        target(32, 8, (0, 0), (32, 8), PixelFormat::RgbaF32),
    );
    let mut cache = RecordingCache::default();
    let token = CancelToken::new();
    frame
        .execute(&four_tiles(), &mut cache, &token)
        .expect("the plan runs");
    assert_eq!(sink.fractions(), vec![0.25, 0.5, 0.75, 1.0]);

    // A plan that is cancelled half way never reports `1.0`, and nothing it
    // reported goes back.
    let sink = Recorder::default();
    let backend = CpuBackend::new().with_progress(Arc::new(sink.clone()));
    let mut frame = Frame::open(
        backend,
        target(32, 8, (0, 0), (32, 8), PixelFormat::RgbaF32),
    );
    let token = CancelToken::new();
    let mut cache = CancellingCache {
        token: token.clone(),
        cancel_at: 2,
        puts: 0,
    };
    assert!(frame.execute(&four_tiles(), &mut cache, &token).is_err());
    // The tile that finished before the cancel is reported; the one that did
    // not is not, and `1.0` never arrives (ARQ-112 checks before it reports).
    assert_eq!(sink.fractions(), vec![0.25]);
    assert!(
        sink.fractions().iter().all(|fraction| *fraction < 1.0),
        "a cancelled plan never reaches 1.0"
    );
    assert!(
        sink.fractions().windows(2).all(|pair| pair[0] <= pair[1]),
        "the fraction never goes back"
    );

    // A plan with no task is complete by definition.
    let sink = Recorder::default();
    let backend = CpuBackend::new().with_progress(Arc::new(sink.clone()));
    let mut frame = Frame::open(backend, target(8, 8, (0, 0), (8, 8), PixelFormat::RgbaF32));
    let mut cache = RecordingCache::default();
    frame
        .execute(&plan(Vec::new()), &mut cache, &CancelToken::new())
        .expect("an empty plan runs");
    assert_eq!(sink.fractions(), vec![1.0]);
}

// ---------------------------------------------------------------------------
// AC8 — the memory the backend holds, and the trim
// ---------------------------------------------------------------------------

// ARQ-143, ARQ-44 (partial): `memory_usage` answers the surface plus the cache,
// and `trim` drops what the backend holds to the budget given, without leaking
// a buffer.
#[test]
fn arq_143_memory_usage_is_the_surface_plus_the_cache_and_trim_drops_to_the_budget() {
    let target_desc = target(16, 16, (0, 0), (16, 16), PixelFormat::RgbaF32);
    let scene = plan(vec![task(
        1,
        IRect::new(0, 0, 16, 16),
        vec![fill(0.0, 0.0, 16.0, 16.0, white())],
    )]);
    let mut frame = Frame::open(CpuBackend::new(), target_desc);
    let mut cache = RecordingCache::default();
    frame
        .execute(&scene, &mut cache, &CancelToken::new())
        .expect("the plan runs");

    // The surface is 16 × 16 linear pixels, four channels each; the cache holds
    // the one tile the plan was made of, in the same shape.
    let surface = 16 * 16 * 16;
    let cached = cache.bytes().0;
    assert_eq!(cached, surface, "one tile of the same size");
    assert_eq!(frame.backend.memory_usage().host, Bytes(surface + cached));
    assert_eq!(frame.backend.memory_usage().device, Bytes(0));

    // A budget that covers what the backend holds drops nothing.
    frame.backend.trim(Bytes(surface + cached));
    assert_eq!(frame.backend.memory_usage().host, Bytes(surface + cached));
    assert!(
        frame.read(IRect::new(0, 0, 16, 16)).is_ok(),
        "an untouched frame stays readable"
    );

    // A tighter budget drops the buffers, and nothing leaks: the frame is gone
    // and the next one starts from a clean surface.
    frame.backend.trim(Bytes(surface));
    assert_eq!(frame.backend.memory_usage().host, Bytes(0));
    assert!(
        frame.read(IRect::new(0, 0, 16, 16)).is_err(),
        "a trimmed frame is not readable"
    );
    let freshly = Frame::open(CpuBackend::new(), target_desc);
    assert_eq!(freshly.backend.memory_usage().host, Bytes(surface));
    assert!(
        freshly.backend.memory_usage().host <= Bytes(surface + cached),
        "a new frame does not hold more than the one it replaced"
    );
}

// ---------------------------------------------------------------------------
// The place of the surface, and the ops this delivery does not render
// ---------------------------------------------------------------------------

// ARQ-149: the surface covers the page rectangle at the target's origin, a tile
// that runs outside it is clipped, and a read outside it answers transparent
// black in a buffer of the size asked for.
#[test]
fn arq_149_the_surface_sits_at_the_origin_and_clips_to_the_target() {
    let target_desc = target(64, 64, (100, 200), (164, 264), PixelFormat::RgbaF32);
    // The tile runs past the surface's right and bottom edges, and the fill
    // only meets the surface in a 4 × 4 corner.
    let scene = plan(vec![task(
        1,
        IRect::new(150, 250, 214, 314),
        vec![fill(160.0, 260.0, 200.0, 300.0, white())],
    )]);
    let surface = run(target_desc, &scene);
    assert!(
        is_white(&surface, 60, 60),
        "page (160, 260) is inside the fill"
    );
    assert!(
        is_clear(&surface, 50, 50),
        "page (150, 250) is outside the fill"
    );
    assert!(
        is_clear(&surface, 58, 62),
        "page (158, 262) is outside the fill"
    );

    // A read that reaches past the target keeps the size it asked for; the part
    // outside the target is transparent black.
    let mut frame = Frame::open(CpuBackend::new(), target_desc);
    let mut cache = RecordingCache::default();
    frame
        .execute(&scene, &mut cache, &CancelToken::new())
        .expect("the plan runs");
    let read = frame
        .read(IRect::new(150, 250, 200, 300))
        .expect("the region reads");
    assert_eq!((read.width(), read.height()), (50, 50));
    assert!(is_white(&read, 10, 10), "page (160, 260) inside the target");
    assert!(
        is_clear(&read, 20, 20),
        "page (170, 270) is outside the target"
    );
}

// ARQ-117, W16: a read wider and taller than the surface copies, row by row,
// the span the surface and the region share — the clip's, not the region's —
// and leaves every other pixel transparent black, in both storages.
#[test]
fn arq_117_a_read_wider_than_the_surface_copies_the_clip_and_nothing_else() {
    for format in [PixelFormat::Rgba8, PixelFormat::RgbaF32] {
        // A 6 × 4 surface at (2, 2), fully covered, read through a 12 × 8 region.
        let scene = plan(vec![task(
            1,
            IRect::new(2, 2, 8, 6),
            vec![fill(0.0, 0.0, 12.0, 8.0, white())],
        )]);
        let mut frame = Frame::open(CpuBackend::new(), target(6, 4, (2, 2), (12, 8), format));
        let mut cache = RecordingCache::default();
        frame
            .execute(&scene, &mut cache, &CancelToken::new())
            .expect("the plan runs");
        let read = frame
            .read(IRect::new(0, 0, 12, 8))
            .expect("the region reads");
        assert_eq!((read.width(), read.height()), (12, 8));
        for y in 0..8u32 {
            for x in 0..12u32 {
                let inside = (2..8).contains(&x) && (2..6).contains(&y);
                if inside {
                    assert!(
                        is_white(&read, x, y),
                        "{format:?}: ({x}, {y}) is on the surface"
                    );
                } else {
                    assert!(
                        is_clear(&read, x, y),
                        "{format:?}: ({x}, {y}) is off the surface"
                    );
                }
            }
        }
    }
}

// ARQ-149, ARQ-117, C3: a tile that runs past the surface's right and bottom
// edges writes, row by row, only the span it shares with the surface — the
// clip's, not the tile's — so every row of the clipped corner lands, the last
// one included, and nothing spills into the next row, in both storages.
#[test]
fn arq_149_a_tile_clipped_at_the_right_edge_writes_every_row_of_the_corner() {
    for format in [PixelFormat::Rgba8, PixelFormat::RgbaF32] {
        // The scene of `arq_149_the_surface_sits_at_the_origin_and_clips_to_the_target`.
        let scene = plan(vec![task(
            1,
            IRect::new(150, 250, 214, 314),
            vec![fill(160.0, 260.0, 200.0, 300.0, white())],
        )]);
        let surface = run(target(64, 64, (100, 200), (164, 264), format), &scene);
        for y in 0..64u32 {
            for x in 0..64u32 {
                let inside = (60..64).contains(&x) && (60..64).contains(&y);
                if inside {
                    assert!(is_white(&surface, x, y), "{format:?}: ({x}, {y}) is filled");
                } else {
                    assert!(is_clear(&surface, x, y), "{format:?}: ({x}, {y}) is clear");
                }
            }
        }
    }
}

// O13, ARQ-42: a rectangle is page space for `write` and for `read`, so an
// offset region answers the window the page holds and equals the same window
// of a full-surface read. Before O13 the read was clipped against its own
// offset and lost a band of `x0` columns and `y0` rows.
#[test]
fn arq_42_an_offset_region_reads_back_the_window_the_page_holds() {
    let full = run(
        target(96, 96, (0, 0), (96, 96), PixelFormat::Rgba8),
        &plan(vec![task(
            1,
            IRect::new(0, 0, 96, 96),
            vec![
                fill(0.0, 0.0, 48.0, 96.0, srgb(1.0, 0.0, 0.0, 1.0)),
                fill(48.0, 0.0, 96.0, 96.0, srgb(0.0, 0.0, 1.0, 1.0)),
            ],
        )]),
    );
    let full = rgba8_bytes(&full);

    let region = IRect::new(4, 6, 68, 70);
    let offset = run(
        target(64, 64, (4, 6), (96, 96), PixelFormat::Rgba8),
        &plan(vec![task(
            2,
            region,
            vec![
                fill(0.0, 0.0, 48.0, 96.0, srgb(1.0, 0.0, 0.0, 1.0)),
                fill(48.0, 0.0, 96.0, 96.0, srgb(0.0, 0.0, 1.0, 1.0)),
            ],
        )]),
    );
    let offset = rgba8_bytes(&offset);

    // The same window of the full-surface read, row by row.
    let mut expected = Vec::with_capacity(64 * 64 * 4);
    for y in region.y0..region.y1 {
        let start = (usize::try_from(y).unwrap() * 96 + 4) * 4;
        expected.extend_from_slice(&full[start..start + 64 * 4]);
    }
    assert_eq!(
        offset, expected,
        "the offset read is the window of the page, not its own offset"
    );
    assert!(
        offset.chunks_exact(4).all(|pixel| pixel[3] == 255),
        "an opaque page answers no transparent pixel"
    );
    assert_eq!(
        &offset[0..4],
        &[255, 0, 0, 255],
        "the first pixel of the region is page (4, 6), red"
    );
    assert_eq!(
        &offset[offset.len() - 4..],
        &[0, 0, 255, 255],
        "the last pixel of the region is page (67, 69), blue"
    );
}

// CB1 and CB2 render no image, mask, effect or transition: the closed
// `RenderOp` enum forces a reading, and the adapter reports an op it cannot
// compose instead of drawing a page it cannot finish. The image is CB4 and the
// other three CB5; the layer ops are rendered from CB2 on.
#[test]
fn an_op_of_a_later_delivery_is_reported_and_not_silently_flattened() {
    let mask = Arc::new(AlphaMask::new(2, 2, vec![255u8; 4]).expect("the mask"));
    let scene = plan(vec![task(
        1,
        IRect::new(0, 0, 8, 8),
        vec![RenderOp::ApplyMask { mask }],
    )]);
    let mut frame = Frame::open(
        CpuBackend::new(),
        target(8, 8, (0, 0), (8, 8), PixelFormat::RgbaF32),
    );
    let mut cache = RecordingCache::default();
    let error = frame
        .execute(&scene, &mut cache, &CancelToken::new())
        .expect_err("a mask op is not rendered by this delivery");
    assert!(
        matches!(error, RenderError::DriverBlocked),
        "reported as blocked, not drawn: {error:?}"
    );
}

// ---------------------------------------------------------------------------
// AC7 — the same plan and the same surface give the same bytes
// ---------------------------------------------------------------------------

// ARQ-111: the bytes are identical across runs. The worker-count clause of the
// card has no subject until `CB7`: this delivery creates no thread and holds no
// pool (D8), so there is only one worker count to compare.
#[test]
fn arq_111_the_same_plan_and_surface_give_the_same_bytes() {
    let scenes = [
        (
            target(64, 64, (0, 0), (64, 64), PixelFormat::Rgba8),
            plan(vec![task(
                1,
                IRect::new(0, 0, 64, 64),
                vec![
                    fill(0.0, 0.0, 64.0, 64.0, srgb(0.3, 0.6, 0.9, 0.7)),
                    fill(16.0, 16.0, 48.0, 48.0, srgb(0.9, 0.2, 0.1, 0.4)),
                ],
            )]),
        ),
        (
            target(64, 64, (32, 16), (96, 80), PixelFormat::RgbaF32),
            plan(vec![
                task(
                    2,
                    IRect::new(32, 16, 64, 48),
                    vec![fill(40.0, 20.0, 60.0, 44.0, srgb(0.5, 0.5, 0.5, 0.5))],
                ),
                task(
                    3,
                    IRect::new(32, 48, 64, 64),
                    vec![fill(32.0, 48.0, 64.0, 64.0, white())],
                ),
            ]),
        ),
    ];
    for (target_desc, scene) in scenes {
        let first = run(target_desc, &scene);
        let second = run(target_desc, &scene);
        assert_eq!(first.data(), second.data(), "{:?}", target_desc.format);
    }
}

// ARQ-111, AC5, O12: the merged tile of a region is content-addressed, so the
// same frame rendered twice answers the same bytes and the second run reuses
// the cached tile instead of covering both roots again.
#[test]
fn arq_111_a_second_frame_reuses_the_merged_tile() {
    let target_desc = target(32, 32, (0, 0), (32, 32), PixelFormat::RgbaF32);
    let scene = plan(vec![task(
        7,
        IRect::new(0, 0, 32, 32),
        vec![
            fill(0.0, 0.0, 16.0, 32.0, srgb(1.0, 0.0, 0.0, 1.0)),
            fill(16.0, 0.0, 32.0, 32.0, srgb(0.0, 0.0, 1.0, 1.0)),
        ],
    )]);
    let mut cache = RecordingCache::default();

    let first = run_with(target_desc, &scene, &mut cache);
    assert_eq!(cache.puts().len(), 1, "the merged tile is rendered once");
    assert_eq!(
        channels(&first, 8, 16),
        [1.0, 0.0, 0.0, 1.0],
        "the first root of the merged tile"
    );
    assert_eq!(
        channels(&first, 24, 16),
        [0.0, 0.0, 1.0, 1.0],
        "the second root of the merged tile"
    );

    let second = run_with(target_desc, &scene, &mut cache);
    assert_eq!(
        cache.puts().len(),
        1,
        "the cached tile is reused, not rendered again"
    );
    assert_eq!(
        first.data(),
        second.data(),
        "the two frames agree byte for byte"
    );
}

// ---------------------------------------------------------------------------
// The surface the backend opens
// ---------------------------------------------------------------------------

// ARQ-44 (partial): the surface is bounded by the cap of `D-03`, so an
// FD-55-sized page does not try to allocate 4 GiB.
#[test]
fn arq_44_a_surface_beyond_the_budget_is_refused() {
    let mut backend = CpuBackend::new();
    let big = target(
        16_384,
        16_384,
        (0, 0),
        (16_384, 16_384),
        PixelFormat::RgbaF32,
    );
    assert!(
        backend.begin_frame(&big).is_err(),
        "4 GiB does not fit the 1.5 GB cap"
    );
}

// The backend answers the port's caps, and the memory it holds is the surface
// it opened.
#[test]
fn the_caps_and_the_memory_of_an_open_surface() {
    let mut backend = CpuBackend::new();
    let caps: BackendCaps = backend.caps();
    assert_eq!(caps.formats, vec![PixelFormat::Rgba8, PixelFormat::RgbaF32]);
    assert!(!caps.gpu_effects);
    assert_eq!(backend.memory_usage().host, Bytes(0));

    backend
        .begin_frame(&target(16, 8, (0, 0), (16, 8), PixelFormat::RgbaF32))
        .expect("the frame opens");
    // 16 × 8 pixels, four `f32` channels.
    assert_eq!(backend.memory_usage().host, Bytes(16 * 8 * 16));
    assert_eq!(backend.memory_usage().device, Bytes(0));
}

// ARQ-44, O17, O14: an `Rgba8` surface stores one byte per channel — four per
// pixel — and the accounting reports what it holds, not the `f32` figure.
#[test]
fn arq_44_an_rgba8_surface_counts_one_byte_per_channel() {
    let mut backend = CpuBackend::new();
    backend
        .begin_frame(&target(16, 8, (0, 0), (16, 8), PixelFormat::Rgba8))
        .expect("the frame opens");
    // 16 × 8 pixels, four 8-bit channels.
    assert_eq!(backend.memory_usage().host, Bytes(16 * 8 * 4));
    assert_eq!(backend.memory_usage().device, Bytes(0));
}

// O17, D-03, ARQ-44, O14: the budget is compared against the bytes the surface
// stores. An `Rgba8` page of `n` bytes fits a cap of `n` and not one byte less;
// the `RgbaF32` surface of the same page is four times the bytes and does not
// fit that cap.
#[test]
fn arq_44_the_budget_counts_the_bytes_the_rgba8_surface_stores() {
    let page = target(16, 8, (0, 0), (16, 8), PixelFormat::Rgba8);
    let bytes = 16 * 8 * 4;
    let mut backend = CpuBackend::new().with_budget(Bytes(bytes));
    assert!(backend.begin_frame(&page).is_ok(), "the cap is inclusive");
    let mut backend = CpuBackend::new().with_budget(Bytes(bytes - 1));
    assert!(
        matches!(backend.begin_frame(&page), Err(RenderError::OutOfMemory)),
        "one byte over does not fit"
    );
    let mut backend = CpuBackend::new().with_budget(Bytes(bytes));
    let f32_page = target(16, 8, (0, 0), (16, 8), PixelFormat::RgbaF32);
    assert!(
        matches!(
            backend.begin_frame(&f32_page),
            Err(RenderError::OutOfMemory)
        ),
        "the f32 surface of the same page is four times as many bytes"
    );
}

// ---------------------------------------------------------------------------
// CB2 AC1 — a layer composites with its own blend and opacity
// ---------------------------------------------------------------------------

// ARQ-119, 02 §6.1: the ops between `PushLayer` and its `PopLayer` land in a
// buffer of their own, and the pop composites that buffer onto its parent with
// the pop's blend and opacity, not with `Normal`. The expectation is the
// raster crate's own formula called with the layer as source and the parent as
// backdrop: the adapter chooses the operands and `gafira-raster` owns the
// sixteen formulas (ARQ-119).
#[test]
fn arq_119_a_layer_composites_with_its_own_blend_and_opacity() {
    let region = IRect::new(0, 0, 4, 4);
    let backdrop_color = srgb(0.5, 0.5, 0.5, 1.0);
    let layer_color = srgb(0.25, 1.0, 0.5, 1.0);
    let target_desc = target(4, 4, (0, 0), (4, 4), PixelFormat::RgbaF32);

    // The parent's pixel, from the same fill without a layer around it.
    let backdrop_plan = plan(vec![task(
        1,
        region,
        vec![fill(0.0, 0.0, 4.0, 4.0, backdrop_color)],
    )]);
    let backdrop = channels(&run(target_desc, &backdrop_plan), 2, 2);

    let scene = plan(vec![task(
        1,
        region,
        vec![
            fill(0.0, 0.0, 4.0, 4.0, backdrop_color),
            push(),
            fill(0.0, 0.0, 4.0, 4.0, layer_color),
            pop(BlendMode::Multiply, 0.5),
        ],
    )]);
    let got = channels(&run(target_desc, &scene), 2, 2);

    // The layer as source, premultiplied and taken at the pop's opacity once.
    let linear = layer_color.to_linear_srgb();
    let scaled = [
        linear.r * 0.5,
        linear.g * 0.5,
        linear.b * 0.5,
        linear.a * 0.5,
    ];
    let expected = blend_f32(BlendMode::Multiply, backdrop, scaled).expect("the blend");
    assert_eq!(
        got, expected,
        "Multiply at half opacity, the layer as source"
    );
    // The fixture tells the mode and the opacity apart from the wrong answers a
    // pop can give.
    assert_ne!(
        expected,
        blend_f32(BlendMode::Normal, backdrop, scaled).expect("the blend"),
        "the fixture tells Multiply from Normal"
    );
    assert_ne!(
        expected,
        blend_f32(
            BlendMode::Multiply,
            backdrop,
            [linear.r, linear.g, linear.b, linear.a],
        )
        .expect("the blend"),
        "the fixture tells half opacity from none"
    );
}

// ARQ-119, 02 §6.1: a layer at `Normal` and full opacity changes no pixel,
// which is what lets the graph wrap every node in one.
#[test]
fn arq_119_a_normal_layer_at_full_opacity_changes_no_pixel() {
    let region = IRect::new(0, 0, 8, 8);
    let color = srgb(0.2, 0.4, 0.6, 0.5);
    let plain = plan(vec![task(1, region, vec![fill(0.0, 0.0, 8.0, 8.0, color)])]);
    let wrapped = plan(vec![task(
        1,
        region,
        vec![
            push(),
            fill(0.0, 0.0, 8.0, 8.0, color),
            pop(BlendMode::Normal, 1.0),
        ],
    )]);
    let target_desc = target(8, 8, (0, 0), (8, 8), PixelFormat::RgbaF32);
    let a = run(target_desc, &plain);
    let b = run(target_desc, &wrapped);
    assert_eq!(
        a.data(),
        b.data(),
        "a Normal layer at full opacity is an identity"
    );
}

// ---------------------------------------------------------------------------
// CB2 AC2 — the opacity applies once, to the composite
// ---------------------------------------------------------------------------

// ARQ-119, 02 §6.1: two opaque fills in a layer at 0,5 over the transparent
// page leave alpha 0,5 — and the blue alone — where they overlap. Applying the
// half to each child instead leaves 0,75 and a quarter of the red under the
// blue: the two numbers are stated here, and the second one is what a child-wise
// opacity would give.
#[test]
fn arq_119_the_opacity_applies_once_to_the_composite() {
    let scene = plan(vec![task(
        1,
        IRect::new(0, 0, 2, 1),
        vec![
            push(),
            fill(0.0, 0.0, 2.0, 1.0, srgb(1.0, 0.0, 0.0, 1.0)),
            fill(1.0, 0.0, 2.0, 1.0, srgb(0.0, 0.0, 1.0, 1.0)),
            pop(BlendMode::Normal, 0.5),
        ],
    )]);
    let surface = run(target(2, 1, (0, 0), (2, 1), PixelFormat::RgbaF32), &scene);
    assert_eq!(
        channels(&surface, 0, 0),
        [0.5, 0.0, 0.0, 0.5],
        "one fill at half opacity"
    );
    let overlap = channels(&surface, 1, 0);
    assert_eq!(
        overlap,
        [0.0, 0.0, 0.5, 0.5],
        "the composite at half opacity, once"
    );
    assert_ne!(
        overlap,
        [0.25, 0.0, 0.5, 0.75],
        "not half alpha on each child: that is 0,75 of alpha and a quarter of the red"
    );
}

// ---------------------------------------------------------------------------
// CB2 AC5 — the sixteen modes reach the raster formula
// ---------------------------------------------------------------------------

// ARQ-119, 02 §6.1: every mode of `BlendMode::ALL` reaches `gafira-raster`'s
// formula with the layer's buffer as **source** and the parent's pixels as
// **backdrop**. The adapter does not own the arithmetic: the expectation is the
// crate's own `blend_f32` called with those operands. An identical
// reimplementation would pass this test; what it pins is the mode that arrives
// and the order of the two operands.
#[test]
fn arq_119_all_sixteen_modes_composite_the_layer_as_source() {
    let region = IRect::new(0, 0, 4, 4);
    let backdrop_color = srgb(0.3, 0.6, 0.2, 1.0);
    let layer_color = srgb(0.9, 0.1, 0.7, 1.0);
    let target_desc = target(4, 4, (0, 0), (4, 4), PixelFormat::RgbaF32);
    let backdrop = channels(
        &run(
            target_desc,
            &plan(vec![task(
                1,
                region,
                vec![fill(0.0, 0.0, 4.0, 4.0, backdrop_color)],
            )]),
        ),
        2,
        2,
    );
    let linear = layer_color.to_linear_srgb();
    let source = [
        linear.r * linear.a,
        linear.g * linear.a,
        linear.b * linear.a,
        linear.a,
    ];

    for mode in BlendMode::ALL {
        let scene = plan(vec![task(
            1,
            region,
            vec![
                fill(0.0, 0.0, 4.0, 4.0, backdrop_color),
                push(),
                fill(0.0, 0.0, 4.0, 4.0, layer_color),
                pop(mode, 1.0),
            ],
        )]);
        let got = channels(&run(target_desc, &scene), 2, 2);
        let expected = blend_f32(mode, backdrop, source).expect("the blend");
        assert_eq!(got, expected, "{mode:?}: the layer is the source");
        let swapped = blend_f32(mode, source, backdrop).expect("the blend");
        if swapped != expected {
            assert_ne!(
                got, swapped,
                "{mode:?}: the parent is the backdrop, not the source"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// CB2 AC4 — nested layers, the reverse pop order, the unbalanced plan
// ---------------------------------------------------------------------------

// ARQ-119, 02 §6.1: the pops happen in the reverse order of the pushes, each
// with its own blend and opacity, and an inner layer does not overwrite the one
// it sits in: outside the inner fill the parent's ink survives, and where the
// inner layer covers it the half opacity of the inner pop is what shows.
#[test]
fn arq_119_nested_layers_pop_in_the_reverse_order_of_their_pushes() {
    let scene = plan(vec![task(
        1,
        IRect::new(0, 0, 4, 1),
        vec![
            push(),
            fill(0.0, 0.0, 4.0, 1.0, srgb(1.0, 0.0, 0.0, 1.0)),
            push(),
            fill(2.0, 0.0, 4.0, 1.0, srgb(1.0, 1.0, 1.0, 1.0)),
            pop(BlendMode::Normal, 0.5),
            pop(BlendMode::Normal, 1.0),
        ],
    )]);
    let surface = run(target(4, 1, (0, 0), (4, 1), PixelFormat::RgbaF32), &scene);
    assert_eq!(
        channels(&surface, 0, 0),
        [1.0, 0.0, 0.0, 1.0],
        "outside the inner layer the parent's red survives"
    );
    assert_eq!(
        channels(&surface, 3, 0),
        [1.0, 0.5, 0.5, 1.0],
        "the inner layer at half opacity over the red parent"
    );
}

// ARQ-119, 02 §6.1: a plan that pops without pushing, or leaves a layer open,
// is refused instead of drawn with a guess. The port carries no plan-validation
// error, so the refusal is the one the adapter uses for a plan it cannot
// execute: `DriverBlocked`.
#[test]
fn arq_119_an_unbalanced_plan_is_refused() {
    let cases = [
        ("a pop with no push", vec![pop(BlendMode::Normal, 1.0)]),
        (
            "a push with no pop",
            vec![push(), fill(0.0, 0.0, 8.0, 8.0, srgb(1.0, 0.0, 0.0, 1.0))],
        ),
    ];
    for (case, ops) in cases {
        let scene = plan(vec![task(1, IRect::new(0, 0, 8, 8), ops)]);
        let mut frame = Frame::open(
            CpuBackend::new(),
            target(8, 8, (0, 0), (8, 8), PixelFormat::RgbaF32),
        );
        let mut cache = RecordingCache::default();
        let error = frame
            .execute(&scene, &mut cache, &CancelToken::new())
            .expect_err(case);
        assert!(
            matches!(error, RenderError::DriverBlocked),
            "{case}: refused, not drawn: {error:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// CB2 AC3 and AC6 — the buffer is cleared when it is pushed, and a reused
// buffer leaves no residue
// ---------------------------------------------------------------------------

// ARQ-117: a layer buffer is cleared to transparent when it is pushed, so the
// previous layer of the same tile cannot come back. The first layer here lands
// at half opacity; the second takes its buffer back from the pool, draws only
// on the right, and must leave the half-transparent white of the first one
// alone on the left instead of compositing its full white again.
#[test]
fn arq_117_a_pushed_layer_is_cleared_to_transparent() {
    let scene = plan(vec![task(
        1,
        IRect::new(0, 0, 2, 1),
        vec![
            push(),
            fill(0.0, 0.0, 2.0, 1.0, white()),
            pop(BlendMode::Normal, 0.5),
            push(),
            fill(1.0, 0.0, 2.0, 1.0, srgb(1.0, 0.0, 0.0, 1.0)),
            pop(BlendMode::Normal, 1.0),
        ],
    )]);
    let surface = run(target(2, 1, (0, 0), (2, 1), PixelFormat::RgbaF32), &scene);
    assert_eq!(
        channels(&surface, 0, 0),
        [0.5, 0.5, 0.5, 0.5],
        "the second layer is transparent on the left: the first one's white cannot come back"
    );
    assert_eq!(
        channels(&surface, 1, 0),
        [1.0, 0.0, 0.0, 1.0],
        "and it holds only its own fill"
    );
}

// ARQ-117: a buffer a pop returned to the pool leaves no residue when the next
// tile's layer takes it. The second tile's layer draws nothing, so its pop must
// show the transparent page, not the first tile's white.
#[test]
fn arq_117_a_reused_layer_buffer_leaves_no_residue() {
    let scene = plan(vec![
        task(
            1,
            IRect::new(0, 0, 2, 1),
            vec![
                push(),
                fill(0.0, 0.0, 2.0, 1.0, white()),
                pop(BlendMode::Normal, 1.0),
            ],
        ),
        task(
            2,
            IRect::new(2, 0, 4, 1),
            vec![push(), pop(BlendMode::Normal, 1.0)],
        ),
    ]);
    let surface = run(target(4, 1, (0, 0), (4, 1), PixelFormat::RgbaF32), &scene);
    assert_eq!(
        channels(&surface, 0, 0),
        [1.0, 1.0, 1.0, 1.0],
        "the first tile paints its layer"
    );
    assert_eq!(
        channels(&surface, 3, 0),
        [0.0, 0.0, 0.0, 0.0],
        "the second tile's layer is empty: the pooled buffer carries no residue"
    );
}

// ARQ-143, ARQ-44 (partial): a layered frame keeps one buffer per depth in the
// pool instead of allocating per tile or per layer, the pool is part of what
// `memory_usage` answers, and `trim` drops it. Two 32 × 16 tiles of one layer
// each hold one 8 KiB layer buffer after the frame, not two.
#[test]
fn arq_143_the_layer_pool_is_accounted_and_trim_drops_it() {
    let target_desc = target(64, 16, (0, 0), (64, 16), PixelFormat::RgbaF32);
    let layer = |region: IRect, key: u128| {
        task(
            key,
            region,
            vec![
                push(),
                fill(0.0, 0.0, 64.0, 64.0, srgb(0.5, 0.25, 0.75, 1.0)),
                pop(BlendMode::Normal, 0.5),
            ],
        )
    };
    let scene = plan(vec![
        layer(IRect::new(0, 0, 32, 16), 1),
        layer(IRect::new(32, 0, 64, 16), 2),
    ]);
    let mut frame = Frame::open(CpuBackend::new(), target_desc);
    let mut cache = RecordingCache::default();
    frame
        .execute(&scene, &mut cache, &CancelToken::new())
        .expect("the plan runs");

    let surface = 64 * 16 * 16;
    let cached = cache.bytes().0;
    assert_eq!(cached, 2 * 32 * 16 * 16, "the cache holds both tiles");
    let pooled = 32 * 16 * 16;
    assert_eq!(
        frame.backend.memory_usage().host,
        Bytes(surface + cached + pooled),
        "the surface, the cache and one pooled layer buffer"
    );

    // A budget that covers it keeps the pool; a tighter one drops it with the
    // surface and the accounting of the cache (ARQ-44).
    frame.backend.trim(Bytes(surface + cached + pooled));
    assert_eq!(
        frame.backend.memory_usage().host,
        Bytes(surface + cached + pooled),
        "a budget that covers what is held drops nothing"
    );
    frame.backend.trim(Bytes(0));
    assert_eq!(frame.backend.memory_usage().host, Bytes(0));
}

// ---------------------------------------------------------------------------
// CB2 AC7 — determinism, and a pool that does not reach the pixels
// ---------------------------------------------------------------------------

// ARQ-111: a layered plan gives the same bytes twice, and a frame that leaves
// buffers in the pool first does not change them: the number of buffers the
// pool holds never reaches the pixels.
#[test]
fn arq_111_a_layered_plan_is_deterministic_and_the_pool_does_not_reach_the_pixels() {
    let scene = || {
        plan(vec![task(
            1,
            IRect::new(0, 0, 8, 8),
            vec![
                fill(0.0, 0.0, 8.0, 8.0, srgb(0.3, 0.6, 0.9, 1.0)),
                push(),
                fill(2.0, 2.0, 6.0, 6.0, srgb(0.9, 0.2, 0.1, 0.6)),
                pop(BlendMode::Multiply, 0.75),
            ],
        )])
    };
    let target_desc = target(8, 8, (0, 0), (8, 8), PixelFormat::Rgba8);
    let cold = run(target_desc, &scene());
    let again = run(target_desc, &scene());
    assert_eq!(
        cold.data(),
        again.data(),
        "the same plan gives the same bytes"
    );

    // Three nested layers in an earlier frame leave three buffers in the pool.
    let warm_up = plan(vec![task(
        9,
        IRect::new(0, 0, 8, 8),
        vec![
            push(),
            push(),
            push(),
            fill(0.0, 0.0, 8.0, 8.0, white()),
            pop(BlendMode::Normal, 1.0),
            pop(BlendMode::Normal, 1.0),
            pop(BlendMode::Normal, 1.0),
        ],
    )]);
    let mut backend = CpuBackend::new();
    let mut cache = RecordingCache::default();
    let token = CancelToken::new();
    let mut first = backend
        .begin_frame(&target_desc)
        .expect("the first frame opens");
    backend
        .execute(
            &mut first,
            &warm_up,
            &mut cache,
            Deadline::Unbounded,
            &token,
        )
        .expect("the warm-up runs");
    let mut second = backend
        .begin_frame(&target_desc)
        .expect("the second frame opens");
    backend
        .execute(
            &mut second,
            &scene(),
            &mut cache,
            Deadline::Unbounded,
            &token,
        )
        .expect("the plan runs");
    let warm = backend
        .read_back(&second, IRect::new(0, 0, 8, 8))
        .expect("the surface reads");
    assert_eq!(
        cold.data(),
        warm.data(),
        "the buffers the pool holds do not change the result"
    );
}

// S63-03: a gradient fills its path with the colour of each pixel's place
// in the shape's box: red at the start of a left-to-right gradient, blue at
// its end, and the box is taken through the op's transform.
#[test]
fn s63_03_a_gradient_fills_its_shape() {
    use gafira_doc::{Gradient, GradientKind, GradientStop};
    let gradient = Gradient {
        kind: GradientKind::Linear { angle_deg: 0.0 },
        stops: vec![
            GradientStop {
                offset: 0.0,
                color: srgb(1.0, 0.0, 0.0, 1.0),
            },
            GradientStop {
                offset: 1.0,
                color: srgb(0.0, 0.0, 1.0, 1.0),
            },
        ],
    };
    let scene = plan(vec![task(
        1,
        IRect::new(0, 0, 64, 64),
        vec![RenderOp::FillGradient {
            path: Arc::new(rect_path(0.0, 0.0, 25.0, 10.0)),
            gradient: Arc::new(gradient),
            size: (25.0, 10.0),
            // The 25 × 10 box drawn twice as large, 50 × 20.
            transform: Affine::scale(2.0),
        }],
    )]);
    let surface = run(
        target(64, 64, (0, 0), (64, 64), PixelFormat::RgbaF32),
        &scene,
    );
    let [r, _, b, a] = channels(&surface, 1, 10);
    assert!(
        r > 0.9 && b < 0.1 && a > 0.99,
        "red at the start: {r} {b} {a}"
    );
    let [r, _, b, _] = channels(&surface, 48, 10);
    assert!(b > 0.9 && r < 0.1, "blue at the end: {r} {b}");
    let [r, _, b, _] = channels(&surface, 25, 10);
    assert!(r > 0.1 && b > 0.1, "a mix in the middle: {r} {b}");
    assert!(is_clear(&surface, 55, 10), "outside the box");
}
