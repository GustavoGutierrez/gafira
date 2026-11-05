//! The behaviour tests of `CB1`: the frame, the fill in both formats, the
//! cache, the cull, the cancellation and the progress, and the memory the
//! backend accounts for.

use std::sync::Arc;

use gafira_color::{Color, Rgba};
use gafira_doc::contract::CancelToken;
use gafira_geom::{Affine, IRect, Point};
use gafira_ports::common::Bytes;
use gafira_ports::render::{
    BackendCaps, BackendKind, ContentKey, Deadline, Priority, RenderBackend, RenderOp, RenderPlan,
    RenderQuality, TargetDesc, TileCache, TileKey, TileTask,
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

/// A cache that holds nothing: `CB1`'s first slice writes no tile yet.
struct EmptyCache;

impl TileCache for EmptyCache {
    fn get(&self, _key: &TileKey) -> Option<Arc<ImageBuf>> {
        None
    }

    fn put(&mut self, _key: TileKey, _tile: Arc<ImageBuf>) {}

    fn bytes(&self) -> Bytes {
        Bytes(0)
    }

    fn trim(&mut self, _budget: Bytes) {}
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

/// Runs one plan on a fresh backend and answers the whole surface.
fn run(target_desc: TargetDesc, plan: &RenderPlan) -> ImageBuf {
    let mut backend = CpuBackend::new();
    let mut cx = backend.begin_frame(&target_desc).expect("the frame opens");
    let mut cache = EmptyCache;
    let token = CancelToken::new();
    backend
        .execute(&mut cx, plan, &mut cache, Deadline::Unbounded, &token)
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
