//! The fixed 1080p frame of `CB6`, shared by the CAL-11 bench and its guard so
//! both measure one thing (CAL-11, ARQ-111).
//!
//! The frame is built from constants only: a page background, two groups of
//! fills (one nested, with its own blend), a translucent fill on top and one
//! image, no effect. It is a `RenderPlan` and not a `Document` because no
//! document node produces `DrawImage` yet (ARQ-134); the tiles are the
//! planner's 256 px grid, clipped to the page, one task per tile, every task
//! carrying the whole op list so the backend's cull decides what each tile
//! draws (02 §6.2).
//!
//! The PNG is the export's `RgbaF32` page encoded with ARQ-117's order:
//! unpremultiply in linear light, the sRGB curve, round. PNG stores straight
//! alpha, so the bytes are not premultiplied again; the background is opaque,
//! so every pixel has `a = 255` and the two readings are the same bytes.

use std::sync::Arc;

use gafira_color::{Color, LinearRgba, Rgba};
use gafira_doc::BlendMode;
use gafira_doc::contract::CancelToken;
use gafira_geom::{Affine, IRect};
use gafira_ports::common::Bytes;
use gafira_ports::render::{
    ContentKey, Deadline, Priority, RenderBackend, RenderOp, RenderPlan, RenderQuality, TargetDesc,
    TileCache, TileKey, TileTask,
};
use gafira_raster::{ImageBuf, PixelBuf, PixelFormat};
use gafira_render::ContentTileCache;
use gafira_render_cpu::CpuBackend;
use kurbo::{BezPath, Circle, Rect, Shape};

/// The page, in pixels.
pub const WIDTH: u32 = 1920;
pub const HEIGHT: u32 = 1080;

/// The planner's tile side (`gafira_render::TILE_SIZE`).
const TILE: u32 = 256;

/// The page as a surface of `format`.
pub fn target(format: PixelFormat) -> TargetDesc {
    TargetDesc {
        width: WIDTH,
        height: HEIGHT,
        origin: (0, 0),
        page: (WIDTH, HEIGHT),
        format,
    }
}

/// The whole page, as a read-back region.
pub fn page() -> IRect {
    IRect::new(0, 0, WIDTH as i32, HEIGHT as i32)
}

/// A cache large enough for every tile of the frame in either format.
pub fn cache() -> ContentTileCache {
    ContentTileCache::new(Bytes(64 * 1024 * 1024))
}

fn color(r: f32, g: f32, b: f32, a: f32) -> Color {
    Color::Srgb(Rgba::new(r, g, b, a))
}

fn fill(path: BezPath, color: Color) -> RenderOp {
    RenderOp::FillPath {
        path: Arc::new(path),
        color,
        transform: Affine::IDENTITY,
    }
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> BezPath {
    Rect::new(x0, y0, x1, y1).to_path(0.1)
}

fn circle(x: f64, y: f64, radius: f64) -> BezPath {
    Circle::new((x, y), radius).to_path(0.1)
}

/// A 128 × 128 premultiplied `Rgba8` gradient, from integer arithmetic.
fn image() -> Arc<ImageBuf> {
    let side = 128u32;
    let mut data = Vec::with_capacity((side * side * 4) as usize);
    for y in 0..side {
        for x in 0..side {
            let a = 160 + ((x + y) % 96) as u8;
            let channel = |value: u32| ((value % 256) as u8).min(a);
            data.extend_from_slice(&[channel(x * 2), channel(y * 2), channel(x + y), a]);
        }
    }
    Arc::new(ImageBuf::new(side, side, PixelBuf::Rgba8(data)).expect("the image"))
}

/// The frame's ops, in plan order; every task shares the one image.
fn ops(image: &Arc<ImageBuf>) -> Vec<RenderOp> {
    vec![
        fill(rect(0.0, 0.0, 1920.0, 1080.0), color(0.96, 0.95, 0.92, 1.0)),
        RenderOp::PushLayer,
        fill(rect(120.0, 90.0, 980.0, 620.0), color(0.85, 0.2, 0.25, 0.9)),
        fill(circle(760.0, 520.0, 300.0), color(0.1, 0.55, 0.9, 0.6)),
        RenderOp::PopLayer {
            blend: BlendMode::Normal,
            opacity: 0.85,
        },
        RenderOp::PushLayer,
        fill(rect(900.0, 300.0, 1800.0, 980.0), color(0.2, 0.8, 0.4, 1.0)),
        RenderOp::PushLayer,
        fill(circle(1400.0, 600.0, 260.0), color(0.95, 0.75, 0.05, 0.8)),
        RenderOp::PopLayer {
            blend: BlendMode::Multiply,
            opacity: 0.7,
        },
        RenderOp::PopLayer {
            blend: BlendMode::Screen,
            opacity: 0.9,
        },
        RenderOp::DrawImage {
            image: Arc::clone(image),
            transform: Affine::translate((200.0, 640.0)) * Affine::rotate(0.1) * Affine::scale(2.5),
        },
        fill(
            rect(1500.0, 60.0, 1860.0, 260.0),
            color(0.05, 0.05, 0.1, 0.35),
        ),
    ]
}

/// The fixed frame: one task per tile of the planner's grid (ARQ-111).
pub fn plan(quality: RenderQuality) -> RenderPlan {
    let image = image();
    let mut tasks = Vec::new();
    for row in 0..HEIGHT.div_ceil(TILE) {
        for column in 0..WIDTH.div_ceil(TILE) {
            let region = IRect::new(
                (column * TILE) as i32,
                (row * TILE) as i32,
                ((column + 1) * TILE).min(WIDTH) as i32,
                ((row + 1) * TILE).min(HEIGHT) as i32,
            );
            tasks.push(TileTask {
                key: TileKey {
                    content: ContentKey(1),
                    level: 0,
                    column,
                    row,
                },
                priority: Priority::Visible,
                region,
                ops: ops(&image),
            });
        }
    }
    RenderPlan { quality, tasks }
}

/// One frame on `backend`: `begin_frame`, `execute` through `cache`, and the
/// page read back.
pub fn render(
    backend: &mut CpuBackend,
    plan: &RenderPlan,
    format: PixelFormat,
    cache: &mut dyn TileCache,
) -> ImageBuf {
    let mut cx = backend
        .begin_frame(&target(format))
        .expect("the frame opens");
    backend
        .execute(
            &mut cx,
            plan,
            cache,
            Deadline::Unbounded,
            &CancelToken::new(),
        )
        .expect("the frame runs");
    backend.read_back(&cx, page()).expect("the page reads back")
}

/// CAL-11's unit: one export frame from a cold cache, encoded to PNG bytes.
pub fn export_png(plan: &RenderPlan) -> Vec<u8> {
    let image = render(
        &mut CpuBackend::new(),
        plan,
        PixelFormat::RgbaF32,
        &mut cache(),
    );
    encode_png(&image)
}

/// One `RgbaF32` premultiplied linear pixel as straight sRGB 8-bit (ARQ-117).
fn straight_srgb8(pixel: &[f32]) -> [u8; 4] {
    let round = |value: f32| (value.clamp(0.0, 1.0) * 255.0 + 0.5).floor() as u8;
    let alpha = pixel[3].clamp(0.0, 1.0);
    if round(alpha) == 0 {
        return [0; 4];
    }
    let encoded =
        LinearRgba::new(pixel[0] / alpha, pixel[1] / alpha, pixel[2] / alpha, alpha).to_srgb();
    [
        round(encoded.r),
        round(encoded.g),
        round(encoded.b),
        round(alpha),
    ]
}

/// The export page as an 8-bit RGBA PNG.
fn encode_png(image: &ImageBuf) -> Vec<u8> {
    let PixelBuf::RgbaF32(values) = image.data() else {
        panic!("the export answers RgbaF32");
    };
    let bytes: Vec<u8> = values.chunks_exact(4).flat_map(straight_srgb8).collect();
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, image.width(), image.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("the PNG header");
    writer.write_image_data(&bytes).expect("the PNG data");
    writer.finish().expect("the PNG ends");
    out
}
