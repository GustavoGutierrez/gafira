//! The behaviour tests of `CB1`: the frame, the fill in both formats, the
//! cache, the cull, the cancellation and the progress, and the memory the
//! backend accounts for.

use std::sync::{Arc, Mutex};

use gafira_color::{Color, Rgba};
use gafira_doc::contract::{CancelToken, Progress};
use gafira_geom::{Affine, IRect, Point};
use gafira_ports::common::Bytes;
use gafira_ports::render::{
    BackendCaps, BackendKind, ContentKey, Deadline, FrameCx, Priority, RenderBackend, RenderError,
    RenderOp, RenderPlan, RenderQuality, RenderReport, TargetDesc, TileCache, TileKey, TileTask,
};
use gafira_raster::{ImageBuf, PixelBuf, PixelFormat, premultiply_u8};
use kurbo::{BezPath, PathEl};

use crate::CpuBackend;

// ---------------------------------------------------------------------------
// Scene helpers
// ---------------------------------------------------------------------------

fn target(width: u32, height: u32, origin: (i32, i32), format: PixelFormat) -> TargetDesc {
    TargetDesc {
        width,
        height,
        origin,
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
        let surface = run(target(256, 256, (0, 0), format), &scene);
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

// ARQ-117: the target's format decides the surface, and only `Rgba8` and
// `RgbaF32` are render targets.
#[test]
fn arq_117_a_target_that_is_not_rgba_is_refused() {
    let mut backend = CpuBackend::new();
    let refused = backend.begin_frame(&target(8, 8, (0, 0), PixelFormat::Alpha8));
    assert!(refused.is_err(), "Alpha8 is not a render target");
    for format in [PixelFormat::Rgba8, PixelFormat::RgbaF32] {
        let mut backend = CpuBackend::new();
        assert!(backend.begin_frame(&target(8, 8, (0, 0), format)).is_ok());
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
    let f32_surface = run(target(32, 32, (0, 0), PixelFormat::RgbaF32), &scene);
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
    let rgba8_surface = run(target(32, 32, (0, 0), PixelFormat::Rgba8), &scene);
    assert_eq!(bytes8(&rgba8_surface, 10, 10), expected);
    // The rule, pinned by hand: 51·128 and 153·128 are the two channels where
    // truncation and the ARQ-117 rounding disagree.
    assert_eq!(expected, [26, 51, 77, 128]);
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
        target(32, 32, (0, 0), PixelFormat::RgbaF32),
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
        target(128, 32, (0, 0), PixelFormat::RgbaF32),
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
        target(16, 16, (0, 0), PixelFormat::RgbaF32),
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
        let surface = run(target(256, 256, (0, 0), PixelFormat::RgbaF32), &scene);
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
    let target_desc = target(64, 64, (0, 0), PixelFormat::RgbaF32);
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
        target(32, 8, (0, 0), PixelFormat::RgbaF32),
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
        target(32, 8, (0, 0), PixelFormat::RgbaF32),
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
    let mut frame = Frame::open(backend, target(32, 8, (0, 0), PixelFormat::RgbaF32));
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
    let mut frame = Frame::open(backend, target(32, 8, (0, 0), PixelFormat::RgbaF32));
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
    let mut frame = Frame::open(backend, target(8, 8, (0, 0), PixelFormat::RgbaF32));
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
    let target_desc = target(16, 16, (0, 0), PixelFormat::RgbaF32);
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
    let target_desc = target(64, 64, (100, 200), PixelFormat::RgbaF32);
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

// CB1 renders no layer, image, mask, effect or transition: the closed
// `RenderOp` enum forces a reading, and the adapter reports the op instead of
// drawing a page it cannot compose. The layer stack is CB2, the image CB4 and
// the other three CB5.
#[test]
fn a_layer_op_is_reported_and_not_silently_flattened() {
    let ops = [
        RenderOp::PushLayer,
        RenderOp::PopLayer {
            blend: gafira_doc::BlendMode::Normal,
            opacity: 1.0,
        },
    ];
    for op in ops {
        let scene = plan(vec![task(1, IRect::new(0, 0, 8, 8), vec![op])]);
        let mut frame = Frame::open(
            CpuBackend::new(),
            target(8, 8, (0, 0), PixelFormat::RgbaF32),
        );
        let mut cache = RecordingCache::default();
        let error = frame
            .execute(&scene, &mut cache, &CancelToken::new())
            .expect_err("a layer op is not rendered by this delivery");
        assert!(
            matches!(error, RenderError::DriverBlocked),
            "reported as blocked, not drawn: {error:?}"
        );
    }
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
            target(64, 64, (0, 0), PixelFormat::Rgba8),
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
            target(64, 64, (32, 16), PixelFormat::RgbaF32),
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

// ---------------------------------------------------------------------------
// The surface the backend opens
// ---------------------------------------------------------------------------

// ARQ-44 (partial): the surface is bounded by the cap of `D-03`, so an
// FD-55-sized page does not try to allocate 4 GiB.
#[test]
fn arq_44_a_surface_beyond_the_budget_is_refused() {
    let mut backend = CpuBackend::new();
    let big = target(16_384, 16_384, (0, 0), PixelFormat::RgbaF32);
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
        .begin_frame(&target(16, 8, (0, 0), PixelFormat::RgbaF32))
        .expect("the frame opens");
    // 16 × 8 pixels, four `f32` channels.
    assert_eq!(backend.memory_usage().host, Bytes(16 * 8 * 16));
    assert_eq!(backend.memory_usage().device, Bytes(0));
}
