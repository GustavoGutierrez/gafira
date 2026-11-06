//! ARQ-42, ARQ-113: the preview and the export of one page agree in the
//! comparison space of 02 §10 — 8-bit sRGB **premultiplied** — and the export
//! is encoded with ARQ-117's rules before the two buffers are compared (D9).
//!
//! This file is the deliverable of `CB3`: the comparator is its critical code,
//! so it lives where a reader can check it and not behind the crate's own
//! helpers. The preview's bytes come from the backend's `Rgba8` read-back; the
//! export's `RgbaF32` surface is converted here, once, by the test. Quantising
//! the preview's `f32` surface with the same function the export's pixels go
//! through would be one code path grading itself, which is the fake this
//! delivery exists to prevent.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use gafira_color::{Color, LinearRgba, Rgba};
use gafira_doc::contract::CancelToken;
use gafira_doc::{
    BlendMode, Document, Dpi, ElementBase, GroupNode, Node, NodeId, NodeKind, Page, PageId,
    PageSize, Placement, ShapeGeometry, ShapeNode,
};
use gafira_geom::{IRect, Length, Unit};
use gafira_ports::common::{Bytes, UnixTime};
use gafira_ports::export::{PageRenderRequest, PageRenderer};
use gafira_ports::render::RenderQuality;
use gafira_ports::system::Clock;
use gafira_raster::PixelBuf;
use gafira_render::{BackendSelector, ContentTileCache, GraphPageRenderer};
use gafira_render_cpu::CpuBackend;

/// One 8-bit premultiplied sample as `(c × a + 127) / 255`, integer division
/// (ARQ-117). The comparator carries its own copy on purpose: a hand-computed
/// byte has to be able to contradict it.
fn premultiply8(c: u8, a: u8) -> u8 {
    let value = (u32::from(c) * u32::from(a) + 127) / 255;
    u8::try_from(value).unwrap_or(u8::MAX)
}

/// A channel of `[0, 1]` as 8 bits, `⌊v × 255 + 0,5⌋` clamped to `[0, 255]`
/// (ARQ-118's rounding, which ARQ-117's encode step shares).
fn quantise8(value: f32) -> u8 {
    let scaled = (value.clamp(0.0, 1.0) * 255.0 + 0.5).floor();
    u8::try_from(scaled.clamp(0.0, 255.0) as u32).unwrap_or(u8::MAX)
}

/// One `RgbaF32` export pixel as premultiplied sRGB 8-bit, in the order ARQ-117
/// fixes and `D9` restates for the comparator: unpremultiply in linear light,
/// encode with the sRGB curve, quantise, premultiply with `(c × a + 127) / 255`.
///
/// `a = 0` answers four zeroes and never divides by the alpha (ARQ-117,
/// FD-55 (b)). The alpha is quantised first, so an alpha that rounds to zero
/// takes the same branch the preview's own read-back takes.
fn encode_export_pixel(pixel: [f32; 4]) -> [u8; 4] {
    let alpha = pixel[3].clamp(0.0, 1.0);
    let a = quantise8(alpha);
    if a == 0 {
        // ARQ-117: with a = 0 the answer is (0, 0, 0, 0), and the alpha is
        // never a divisor.
        return [0, 0, 0, 0];
    }
    let straight = [pixel[0] / alpha, pixel[1] / alpha, pixel[2] / alpha];
    let encoded = LinearRgba::new(straight[0], straight[1], straight[2], alpha).to_srgb();
    [
        premultiply8(quantise8(encoded.r), a),
        premultiply8(quantise8(encoded.g), a),
        premultiply8(quantise8(encoded.b), a),
        a,
    ]
}

/// How strict a comparison is (ARQ-113).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// Every byte equal: a stored golden on the platform that generated it.
    Exact,
    /// ARQ-42's budget: at most `pixels / 1000` pixels over 1/255 on a channel.
    Tolerant,
}

/// The target a stored golden is generated on (ARQ-113).
fn mode_for(arch: &str, os: &str, env: &str) -> Mode {
    if (arch, os, env) == ("x86_64", "linux", "gnu") {
        Mode::Exact
    } else {
        Mode::Tolerant
    }
}

/// [`mode_for`] for the target this test binary was built for.
fn current_mode() -> Mode {
    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else {
        "other"
    };
    let os = if cfg!(target_os = "linux") {
        "linux"
    } else {
        "other"
    };
    let env = if cfg!(target_env = "gnu") {
        "gnu"
    } else {
        "other"
    };
    mode_for(arch, os, env)
}

/// One channel of an sRGB8 pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Channel {
    Red,
    Green,
    Blue,
    Alpha,
}

impl Channel {
    const fn at(index: usize) -> Self {
        match index {
            0 => Self::Red,
            1 => Self::Green,
            2 => Self::Blue,
            _ => Self::Alpha,
        }
    }
}

/// What a comparison found, whether it passed or not (AC6 asks the failure
/// report to name the worst channel and how many pixels are over 1/255).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Report {
    /// Pixels in each buffer.
    pixels: usize,
    /// Pixels outside the tolerance of the mode.
    differing: usize,
    /// How many of them the mode allows.
    allowed: usize,
    /// The channel with the largest difference anywhere in the image.
    worst: Channel,
    /// That difference, in 1/255.
    max_delta: u8,
}

impl Report {
    /// True when the comparison meets the mode's rule.
    const fn within_budget(self) -> bool {
        self.differing <= self.allowed
    }
}

/// Compares two premultiplied sRGB8 buffers of the same region.
///
/// The tolerance of §10 is a per-channel difference of more than 1/255, and
/// ARQ-42 lets one pixel in a thousand be over it; `Mode::Exact` lets none be
/// over a difference of zero. Both sides keep the bytes the renderers
/// produced, so the count is of *pixels*, never of channels.
fn compare(mode: Mode, reference: &[u8], actual: &[u8]) -> Report {
    assert_eq!(
        reference.len(),
        actual.len(),
        "both sides cover the same region of the same image"
    );
    assert_eq!(
        reference.len() % 4,
        0,
        "an sRGB8 premultiplied buffer is four bytes per pixel"
    );
    let pixels = reference.len() / 4;
    let (mut differing, mut worst, mut max_delta) = (0usize, Channel::Red, 0u8);
    for (expected, got) in reference.chunks_exact(4).zip(actual.chunks_exact(4)) {
        let mut over = false;
        for (index, (&expected, &got)) in expected.iter().zip(got).enumerate() {
            let delta = expected.abs_diff(got);
            if delta > max_delta {
                max_delta = delta;
                worst = Channel::at(index);
            }
            let over_here = match mode {
                Mode::Exact => delta != 0,
                Mode::Tolerant => delta > 1,
            };
            over |= over_here;
        }
        if over {
            differing += 1;
        }
    }
    let allowed = match mode {
        Mode::Exact => 0,
        // ARQ-42: at most one pixel in a thousand may be over 1/255.
        Mode::Tolerant => pixels / 1000,
    };
    Report {
        pixels,
        differing,
        allowed,
        worst,
        max_delta,
    }
}

/// [`compare`], and a panic that names the worst channel and the count when the
/// comparison is outside the mode's budget (AC6).
fn assert_agrees(mode: Mode, label: &str, reference: &[u8], actual: &[u8]) -> Report {
    let report = compare(mode, reference, actual);
    let rule = match mode {
        Mode::Exact => "differ",
        Mode::Tolerant => "differ by 2/255 or more",
    };
    assert!(
        report.within_budget(),
        "{label}: {} of {} pixels {rule} (allowed {}); worst channel {:?} at {}/255",
        report.differing,
        report.pixels,
        report.allowed,
        report.worst,
        report.max_delta
    );
    report
}

/// One sRGB8 buffer of `pixels` copies of `pixel`.
fn buffer(pixel: [u8; 4], pixels: usize) -> Vec<u8> {
    pixel.iter().copied().cycle().take(pixels * 4).collect()
}

/// A whole `RgbaF32` surface as premultiplied sRGB 8-bit.
fn encode_export(surface: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(surface.len());
    for pixel in surface.chunks_exact(4) {
        out.extend_from_slice(&encode_export_pixel([
            pixel[0], pixel[1], pixel[2], pixel[3],
        ]));
    }
    out
}

/// The side of the page of the comparison, in pixels.
const PAGE: u32 = 64;

/// The region both renderers read back, and the region of the golden: the whole
/// page at 100 %, so the two sides cover the same pixels (ARQ-42, FD-55 (c)).
const REGION: IRect = IRect::new(0, 0, 64, 64);

/// The scale both requests ask for. The export renders at level 0 whatever it
/// is given (02 §6.2), so the preview has to ask for 1.0 to meet it there.
const SCALE: f32 = 1.0;

/// A clock stuck at one instant; it never reads the system time (ARQ-02).
struct FixedClock;

impl Clock for FixedClock {
    fn now(&self) -> UnixTime {
        UnixTime(0)
    }

    fn monotonic(&self) -> Duration {
        Duration::ZERO
    }
}

/// A colour with straight (unpremultiplied) channels, as the document stores it.
fn color(r: f32, g: f32, b: f32, a: f32) -> Color {
    Color::Srgb(Rgba::new(r, g, b, a))
}

/// The shared header of a node: its box on the page, its opacity and its blend.
fn element(id: u64, x: f64, y: f64, w: f64, h: f64, opacity: f32, blend: BlendMode) -> ElementBase {
    let mut base = ElementBase::new(NodeId::new(id), Placement::new(x, y, w, h));
    base.opacity = opacity;
    base.blend = blend;
    base
}

/// A filled shape of the scene, inside the box `[x, y, w, h]`.
fn shape(
    id: u64,
    rect: [f64; 4],
    geometry: ShapeGeometry,
    fill: Color,
    opacity: f32,
    blend: BlendMode,
) -> Node {
    Node::new(
        element(id, rect[0], rect[1], rect[2], rect[3], opacity, blend),
        NodeKind::Shape(ShapeNode {
            geometry,
            fill: Some(fill),
        }),
    )
}

/// A group of the scene: it only carries its opacity and its children.
fn group(id: u64, opacity: f32, children: &[u64]) -> Node {
    let mut node = Node::new(
        element(
            id,
            0.0,
            0.0,
            f64::from(PAGE),
            f64::from(PAGE),
            opacity,
            BlendMode::Normal,
        ),
        NodeKind::Group(GroupNode {}),
    );
    node.children = children.iter().copied().map(NodeId::new).collect();
    node
}

/// A page of the scene's nodes.
fn document(nodes: Vec<Node>, roots: &[u64]) -> Document {
    let mut page = Page::new(
        PageId::new(1),
        "ARQ-42".to_string(),
        PageSize {
            width: Length::new(f64::from(PAGE), Unit::Px),
            height: Length::new(f64::from(PAGE), Unit::Px),
        },
    );
    let mut next_id = 1;
    for node in nodes {
        let id = node.base.id;
        next_id = next_id.max(id.get() + 1);
        page.nodes.insert(id, Arc::new(node));
    }
    page.roots = roots.iter().copied().map(NodeId::new).collect();
    Document::from_parts(next_id, Dpi::default(), vec![Arc::new(page)])
        .expect("the document validates")
}

/// The scene of AC1 and of the stored golden: an opaque fill, a semi-
/// transparent fill, a blended fill, a group at reduced opacity, and a group
/// nested inside it.
///
/// Deliberately not a flat page: a single opaque fill would make every pixel
/// agree trivially, and a comparator that never sees a difference proves
/// nothing. Two corners of [`REGION`] stay empty, which is what FD-55 (b) reads
/// in the transparent render.
fn scene() -> Document {
    let nodes = vec![
        shape(
            2,
            [6.0, 8.0, 26.0, 22.0],
            ShapeGeometry::Rect,
            color(0.85, 0.15, 0.25, 1.0),
            1.0,
            BlendMode::Normal,
        ),
        shape(
            3,
            [22.0, 18.0, 34.0, 40.0],
            ShapeGeometry::Ellipse,
            color(0.1, 0.55, 0.9, 0.55),
            1.0,
            BlendMode::Normal,
        ),
        shape(
            9,
            [50.0, 10.0, 14.0, 14.0],
            ShapeGeometry::Rect,
            color(0.2, 0.85, 0.95, 0.6),
            1.0,
            BlendMode::Normal,
        ),
        group(4, 0.75, &[5, 6]),
        shape(
            5,
            [10.0, 44.0, 30.0, 18.0],
            ShapeGeometry::Rect,
            color(0.2, 0.9, 0.4, 1.0),
            1.0,
            BlendMode::Screen,
        ),
        group(6, 0.6, &[7, 8]),
        shape(
            7,
            [30.0, 30.0, 28.0, 26.0],
            ShapeGeometry::Ellipse,
            color(0.95, 0.75, 0.05, 0.9),
            1.0,
            BlendMode::Normal,
        ),
        shape(
            8,
            [40.0, 50.0, 22.0, 14.0],
            ShapeGeometry::Rect,
            color(0.05, 0.05, 0.1, 0.35),
            1.0,
            BlendMode::Multiply,
        ),
        group(10, 1.0, &[2, 3, 4, 9]),
    ];
    // One root group, as a document of a page usually is: the planner emits a
    // task per tile and root, and a tile is the full content of its region.
    document(nodes, &[10])
}

/// The scene of AC6: three groups nested one inside the other, each at a
/// semi-transparent opacity, with a shape at every level. `groups` is the
/// opacity of the outer, middle and inner group, in that order.
///
/// O11 records what the superseded 8-bit path did on this family: three groups
/// at 0,5 left 12,866 % of the pixels over 1/255. The 0,5/0,5/0,5 case is the
/// worst of XP2's measurements, so it is the one the test uses.
fn deep_scene(groups: [f32; 3]) -> Document {
    let nodes = vec![
        group(20, groups[0], &[21, 22]),
        shape(
            21,
            [4.0, 6.0, 24.0, 20.0],
            ShapeGeometry::Rect,
            color(0.9, 0.35, 0.1, 1.0),
            1.0,
            BlendMode::Normal,
        ),
        group(22, groups[1], &[23, 24]),
        shape(
            23,
            [14.0, 12.0, 28.0, 26.0],
            ShapeGeometry::Ellipse,
            color(0.15, 0.75, 0.45, 0.85),
            1.0,
            BlendMode::Normal,
        ),
        group(24, groups[2], &[25]),
        shape(
            25,
            [22.0, 22.0, 30.0, 28.0],
            ShapeGeometry::Rect,
            color(0.2, 0.3, 0.95, 0.9),
            1.0,
            BlendMode::Normal,
        ),
    ];
    document(nodes, &[20])
}

/// A renderer of one quality, on its own CPU backend and its own cache: the
/// two sides never share a cache and cannot agree through one.
fn renderer(document: Document, quality: RenderQuality) -> GraphPageRenderer {
    GraphPageRenderer::new(
        document,
        quality,
        BackendSelector::new(None, Box::new(CpuBackend::new())),
        ContentTileCache::new(Bytes(1024 * 1024)),
        Box::new(FixedClock),
    )
}

/// The one request of the comparison: the same page, instant, region and scale
/// for both qualities.
fn request(transparent: bool) -> PageRenderRequest {
    PageRenderRequest {
        page: PageId::new(1),
        time: Default::default(),
        scale: SCALE,
        region: REGION,
        transparent,
    }
}

/// The preview's own bytes: the `Rgba8` surface `CpuBackend::read_back` hands
/// a `Preview` frame (O11's one quantisation).
fn preview(document: Document, transparent: bool) -> Vec<u8> {
    let image = renderer(document, RenderQuality::Preview)
        .render(&request(transparent), &CancelToken::new())
        .expect("the preview renders");
    assert_eq!(image.width(), REGION.width(), "the region's width");
    assert_eq!(image.height(), REGION.height(), "the region's height");
    match image.data() {
        PixelBuf::Rgba8(bytes) => bytes.clone(),
        other => panic!("the preview answers Rgba8, not {other:?}"),
    }
}

/// The export's `RgbaF32` surface, converted here, once, to the comparison
/// space.
fn export_srgb8(document: Document, transparent: bool) -> Vec<u8> {
    let image = renderer(document, RenderQuality::Export)
        .render(&request(transparent), &CancelToken::new())
        .expect("the export renders");
    assert_eq!(image.width(), REGION.width(), "the region's width");
    assert_eq!(image.height(), REGION.height(), "the region's height");
    match image.data() {
        PixelBuf::RgbaF32(values) => encode_export(values),
        other => panic!("the export answers RgbaF32, not {other:?}"),
    }
}

/// How many distinct premultiplied sRGB8 pixels an image holds.
fn distinct(image: &[u8]) -> usize {
    image
        .chunks_exact(4)
        .map(|p| [p[0], p[1], p[2], p[3]])
        .collect::<BTreeSet<_>>()
        .len()
}

// ARQ-117, SP6 (02 §10): the comparator encodes the export's `RgbaF32` pixel in
// the order the comparison space fixes, and a hand-computed pixel matches byte
// for byte. The third row is where truncating instead of rounding shows.
#[test]
fn arq_117_the_export_pixel_is_encoded_in_the_order_of_the_comparison_space() {
    let cases: &[([f32; 4], [u8; 4], &str)] = &[
        ([0.5, 0.25, 0.1, 1.0], [188, 137, 89, 255], "opaque"),
        ([0.25, 0.1, 0.05, 0.5], [94, 62, 45, 128], "half alpha"),
        (
            [0.05, 0.125, 0.0125, 0.25],
            [31, 47, 16, 64],
            "quarter alpha",
        ),
    ];
    for (pixel, expected, label) in cases {
        assert_eq!(
            &encode_export_pixel(*pixel),
            expected,
            "{label}: linear premultiplied {pixel:?} as premultiplied sRGB8"
        );
    }
}

// ARQ-117, FD-55 (b): a pixel whose alpha is 0 answers four zeroes.
#[test]
fn arq_117_a_zero_alpha_pixel_answers_four_zeroes() {
    assert_eq!(
        encode_export_pixel([0.5, 0.25, 1.0, 0.0]),
        [0, 0, 0, 0],
        "a colour under zero alpha is not divided by it"
    );
    assert_eq!(
        encode_export_pixel([0.0, 0.0, 0.0, 0.0]),
        [0, 0, 0, 0],
        "the empty pixel"
    );
    assert_eq!(
        encode_export_pixel([0.5, 0.25, 1.0, 0.001]),
        [0, 0, 0, 0],
        "an alpha that quantises to 0 takes the same branch"
    );
}

// ARQ-113: the budget on 1 000 pixels — one pixel at 2/255 passes (99,9 %), two
// fail, and every pixel at 1/255 passes.
#[test]
fn arq_113_the_budget_is_one_pixel_in_a_thousand() {
    let reference = buffer([100, 100, 100, 255], 1000);

    let mut within = reference.clone();
    within[0] = 101;
    within[1] = 99;
    within[2] = 101;
    within[3] = 254;
    let report = compare(Mode::Tolerant, &reference, &within);
    assert_eq!(report.differing, 0, "every pixel at 1/255 stays inside");
    assert!(report.within_budget(), "1/255 is the tolerance, not a miss");

    let mut one_over = reference.clone();
    one_over[0] = 102;
    let report = compare(Mode::Tolerant, &reference, &one_over);
    assert_eq!(report.differing, 1, "one pixel at 2/255");
    assert_eq!(report.allowed, 1, "one in a thousand");
    assert!(report.within_budget(), "1 pixel of 1 000 is 99,9 %");

    let mut two_over = one_over;
    two_over[4] = 98;
    let report = compare(Mode::Tolerant, &reference, &two_over);
    assert_eq!(report.differing, 2, "two pixels at 2/255");
    assert!(!report.within_budget(), "2 of 1 000 is 99,8 %");
}

// ARQ-42, AC6: the report names the worst channel and counts the pixels over
// the tolerance, not the channels.
#[test]
fn arq_42_the_report_names_the_worst_channel_and_the_count() {
    let reference = [10u8; 16];
    let mut actual = reference;
    actual[5] = 12;
    actual[10] = 13;
    let report = compare(Mode::Tolerant, &reference, &actual);
    assert_eq!(report.pixels, 4, "four pixels, four channels each");
    assert_eq!(report.differing, 2, "two pixels are over the tolerance");
    assert_eq!(report.worst, Channel::Blue, "3/255 on blue beats 2/255");
    assert_eq!(report.max_delta, 3);
    assert_eq!(report.allowed, 0, "four pixels allow none");
}

#[test]
#[should_panic(expected = "worst channel Blue at 3/255")]
fn arq_42_the_failure_report_quotes_the_worst_channel() {
    let reference = [10u8; 16];
    let mut actual = reference;
    actual[5] = 12;
    actual[10] = 13;
    assert_agrees(Mode::Tolerant, "a synthetic pair", &reference, &actual);
}

// ARQ-113: a golden is exact on the platform that generated it and tolerant
// elsewhere, so the CI matrix does not turn a last-bit difference into a red.
#[test]
fn arq_113_the_golden_is_exact_only_on_the_platform_that_generates_it() {
    assert_eq!(mode_for("x86_64", "linux", "gnu"), Mode::Exact);
    assert_eq!(mode_for("aarch64", "macos", "other"), Mode::Tolerant);
    assert_eq!(mode_for("x86_64", "windows", "msvc"), Mode::Tolerant);
}

// ARQ-42, AC1, FD-55 (c): the preview at 100 % and the export render the same
// region of the same page at the same scale, and their 8-bit sRGB
// premultiplied images agree to within 1/255 on 99,9 % of the pixels.
#[test]
fn arq_42_the_preview_and_the_export_agree_on_the_scene() {
    let preview = preview(scene(), false);
    let export = export_srgb8(scene(), false);
    let report = assert_agrees(
        Mode::Tolerant,
        "ARQ-42, the scene at 100 %",
        &preview,
        &export,
    );
    assert_eq!(
        report.pixels,
        REGION.width() as usize * REGION.height() as usize,
        "the two sides cover the region the request named"
    );
    assert_eq!(
        report.allowed,
        report.pixels / 1000,
        "ARQ-42's budget is one pixel in a thousand"
    );
    assert_eq!(
        report.differing, 0,
        "O11: the preview quantises the export's surface, so the paths meet exactly"
    );

    // The scene has to be able to disagree. A single opaque fill would make
    // every pixel equal for free, and this test would prove nothing.
    assert!(
        distinct(&preview) >= 8,
        "the scene is not one flat fill: {} distinct pixels",
        distinct(&preview)
    );
    assert!(
        preview.chunks_exact(4).all(|p| p[3] == 255),
        "without transparency the page background makes every pixel opaque"
    );
}

// FD-55 (b), ARQ-117: a transparent render has alpha 0 where nothing is drawn,
// on both sides of the comparison.
#[test]
fn fd_55_a_transparent_render_is_zero_where_nothing_is_drawn() {
    let preview = preview(scene(), true);
    let export = export_srgb8(scene(), true);
    // The first pixel of the region is page (4, 6), above the first shape.
    assert_eq!(
        &preview[0..4],
        &[0, 0, 0, 0],
        "the preview keeps alpha 0 where nothing is drawn"
    );
    assert_eq!(
        &export[0..4],
        &[0, 0, 0, 0],
        "the export converts an empty pixel to four zeroes, not to a division"
    );
    let report = assert_agrees(Mode::Tolerant, "FD-55 (b)", &preview, &export);
    assert_eq!(report.differing, 0, "the transparent scene agrees too");
    assert!(
        preview.chunks_exact(4).any(|p| p[3] != 0),
        "the region does draw content, so both sides are not blank"
    );
}

// ARQ-42, AC6: three nested semi-transparent layers still meet the 99,9 %
// budget, and the nesting reaches the pixels.
#[test]
fn arq_42_three_nested_semi_transparent_layers_agree() {
    let nested = preview(deep_scene([0.5, 0.5, 0.5]), false);
    let export = export_srgb8(deep_scene([0.5, 0.5, 0.5]), false);
    let report = assert_agrees(
        Mode::Tolerant,
        "ARQ-42, three nested layers at 0,5",
        &nested,
        &export,
    );
    assert_eq!(
        report.differing, 0,
        "the deepest scene of XP2 leaves no pixel over 1/255"
    );

    // The fixture has to nest: the same shapes with opaque groups are another
    // image, so the three opacities are what the comparison just proved.
    let opaque = preview(deep_scene([1.0, 1.0, 1.0]), false);
    assert_ne!(
        nested, opaque,
        "three semi-transparent groups change the composite"
    );
}

/// The stored golden of ARQ-113: the transparent preview of the whole page at
/// 100 %, raw premultiplied sRGB8 `RGBA`, one byte per channel, row-major.
///
/// ARQ-113 fixes the platform it is generated on (`x86_64-unknown-linux-gnu`),
/// the lossless format (raw bytes are the pixels, no encoder in between) and
/// the size, which the test's constants fix. No test writes it: a run only ever
/// reads it, and regenerating it is an explicit, one-off action.
const GOLDEN_SIDE: usize = 64;
const GOLDEN: &[u8] = include_bytes!("golden/arq_113_scene.rgba8");

// ARQ-113: the stored golden is the scene on the platform that generated it,
// and one altered byte fails there.
#[test]
fn arq_113_the_stored_golden_is_the_scene_on_its_platform() {
    assert_eq!(
        GOLDEN.len(),
        GOLDEN_SIDE * GOLDEN_SIDE * 4,
        "the golden has the size the test fixes"
    );
    let actual = preview(scene(), true);
    assert_agrees(
        current_mode(),
        "ARQ-113, the stored golden",
        GOLDEN,
        &actual,
    );
}

// ARQ-113: a reference with one byte altered fails on the platform that
// generated it, which is why the golden is compared exactly there.
#[test]
fn arq_113_one_altered_byte_fails_on_this_platform() {
    let mut reference = GOLDEN.to_vec();
    let at = reference.len() / 2;
    reference[at] ^= 1;
    let actual = preview(scene(), true);
    let report = compare(Mode::Exact, &reference, &actual);
    assert_eq!(report.differing, 1, "one altered byte is one pixel over");
    assert_eq!(report.max_delta, 1);
    assert!(
        !report.within_budget(),
        "a stored golden is exact on the platform that generated it"
    );
}
