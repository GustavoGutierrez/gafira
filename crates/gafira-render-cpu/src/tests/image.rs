//! The behaviour tests of `CB4`: `RenderOp::DrawImage`, the affine resampling,
//! the bilinear filter in both source formats, the destination clip and the
//! cull (02 §6.1; ARQ-121, ARQ-117, ARQ-149, ARQ-42).
//!
//! The expected numbers are computed by hand from the rules, not from the
//! sampler: the texels of AC1 come from ARQ-117's unpremultiply/premultiply
//! formulas and the sRGB curve §10 names, and the quarter weights of AC4 and
//! the permutation of AC5 come from the geometry of the transform alone.

use super::*;

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

// ---------------------------------------------------------------------------
// Image scene helpers
// ---------------------------------------------------------------------------

/// A task whose single op draws `image` through `transform`.
fn image_task(transform: Affine, image: &Arc<ImageBuf>) -> Vec<RenderOp> {
    vec![RenderOp::DrawImage {
        image: Arc::clone(image),
        transform,
    }]
}

/// An `Rgba8` source: premultiplied sRGB bytes, row by row.
fn rgba8_image(width: u32, height: u32, texels: &[[u8; 4]]) -> Arc<ImageBuf> {
    let mut data = Vec::with_capacity(texels.len() * 4);
    for texel in texels {
        data.extend_from_slice(texel);
    }
    Arc::new(ImageBuf::new(width, height, PixelBuf::Rgba8(data)).expect("the image"))
}

/// An `RgbaF32` source: linear-light premultiplied texels, row by row.
fn rgba32_image(width: u32, height: u32, texels: &[[f32; 4]]) -> Arc<ImageBuf> {
    let mut data = Vec::with_capacity(texels.len() * 4);
    for texel in texels {
        data.extend_from_slice(texel);
    }
    Arc::new(ImageBuf::new(width, height, PixelBuf::RgbaF32(data)).expect("the image"))
}

/// A frame whose surface covers `region` (the target's origin is the region's,
/// O13), draws `ops` over it and reads `region` back.
fn drawn(region: IRect, format: PixelFormat, ops: Vec<RenderOp>) -> ImageBuf {
    let width = region.width();
    let height = region.height();
    // The page is the smallest one that holds the surface: the surface starts
    // at the region's origin and ends at its far corner. These scenes have no
    // effect, so the page is never read (O18).
    let page = (region.x1.unsigned_abs(), region.y1.unsigned_abs());
    let scene = plan(vec![task(1, region, ops)]);
    let mut frame = Frame::open(
        CpuBackend::new(),
        target(width, height, (region.x0, region.y0), page, format),
    );
    let mut cache = RecordingCache::default();
    frame
        .execute(&scene, &mut cache, &CancelToken::new())
        .expect("the plan runs");
    frame.read(region).expect("the region reads back")
}

/// Asserts every channel of a hand-computed pixel. The expected value is the
/// one the formula gives in `f64`; the surface holds `f32`.
fn assert_close(actual: [f32; 4], expected: [f64; 4], what: &str) {
    for channel in 0..4 {
        assert!(
            (f64::from(actual[channel]) - expected[channel]).abs() <= 1e-6,
            "{what}: channel {channel} is {:?}, expected {:?}",
            actual,
            expected
        );
    }
}

// ---------------------------------------------------------------------------
// AC1 — an `Rgba8` texel is converted once, in the order of ARQ-117
// ---------------------------------------------------------------------------

// The expected values below are computed by hand with the ARQ-117 formulas and
// the sRGB curve of 02 §10, in this order and only this order:
//
//   texel [128, 64, 32, 128]
//   unpremultiply (alpha 128): (128×255/128, 64×255/128, 32×255/128)
//                            = (255, 127.5 → 128, 63.75 → 64)
//   sRGB decode: (1.0, 0.50196078, 0.25098039)
//              → (1.0, 0.21586053, 0.05126945) in linear light
//   premultiply by a = 128/255 = 0.50196078:
//              (0.50196081, 0.10835353, 0.02573526, 0.50196078)
//
// Reading the premultiplied sRGB value as if it were already in that space (the
// error AC1 exists to catch) answers (0.21586053, 0.05126945, 0.01444384): 0.43,
// 0.47 and 0.56 of each channel, not a rounding difference.
const TEXEL_00: [f64; 4] = [
    0.501_960_813_999_176,
    0.108_353_525_4,
    0.025_735_262_78,
    0.501_960_813_999_176,
];
// [255, 128, 64, 255]: opaque, so the decode is the whole conversion.
const TEXEL_10: [f64; 4] = [1.0, 0.215_860_530_7, 0.051_269_464_19, 1.0];
// [10, 20, 30, 40]: unpremultiply → (64, 128, 191), decode → premultiply by 40/255.
const TEXEL_01: [f64; 4] = [0.008_042_269, 0.033_860_475, 0.081_724_808, 0.156_862_751];
// [200, 100, 50, 255]: opaque.
const TEXEL_11: [f64; 4] = [0.577_580_452, 0.127_437_681, 0.031_896_032, 1.0];
// `Color::Srgb(0.2, 0.4, 0.6, 1.0)` in linear light: its own sRGB decode.
const GREY: [f64; 4] = [0.033_104_766, 0.132_868_32, 0.318_546_802, 1.0];
// TEXEL_00 composited over GREY with `Normal`, premultiplied:
//   s + b × (1 − a) with a = 0.50196078 → b × 0.49803922.
const TEXEL_00_OVER_GREY: [f64; 4] = [0.518_448_293, 0.174_527_153, 0.184_384_048, 1.0];

fn ac1_source() -> Arc<ImageBuf> {
    rgba8_image(
        2,
        2,
        &[
            [128, 64, 32, 128],
            [255, 128, 64, 255],
            [10, 20, 30, 40],
            [200, 100, 50, 255],
        ],
    )
}

// ARQ-117: the conversion runs unpremultiply → sRGB decode → premultiply, once
// per texel, and the result is composited over the tile's backdrop. The
// transform lands the 2×2 image on the texel grid at page (2, 1), so every
// target pixel is exactly one source texel.
#[test]
fn arq_117_an_rgba8_texel_is_converted_once_in_the_order_arq_117_fixes() {
    let image = ac1_source();
    let region = IRect::new(0, 0, 6, 4);

    let export = drawn(
        region,
        PixelFormat::RgbaF32,
        image_task(Affine::translate((2.0, 1.0)), &image),
    );
    assert_close(channels(&export, 2, 1), TEXEL_00, "texel (0, 0)");
    assert_close(channels(&export, 3, 1), TEXEL_10, "texel (1, 0)");
    assert_close(channels(&export, 2, 2), TEXEL_01, "texel (0, 1)");
    assert_close(channels(&export, 3, 2), TEXEL_11, "texel (1, 1)");
    for (x, y) in [(0, 0), (1, 0), (0, 1), (4, 1), (2, 0), (5, 3)] {
        assert!(
            is_clear(&export, x, y),
            "page ({x}, {y}) is outside the image and stays transparent"
        );
    }

    // The same op over an opaque backdrop: the texel is composited, not
    // written over it, and a pixel the image does not cover keeps the backdrop.
    let preview = drawn(
        region,
        PixelFormat::RgbaF32,
        vec![
            fill(0.0, 0.0, 6.0, 4.0, srgb(0.2, 0.4, 0.6, 1.0)),
            RenderOp::DrawImage {
                image: Arc::clone(&image),
                transform: Affine::translate((2.0, 1.0)),
            },
        ],
    );
    assert_close(
        channels(&preview, 2, 1),
        TEXEL_00_OVER_GREY,
        "texel (0, 0) over the backdrop",
    );
    assert_close(
        channels(&preview, 0, 0),
        GREY,
        "the backdrop outside the image",
    );
}

// ARQ-117: the weights run on the **converted** values, in linear
// premultiplied light, and a 1:1 translate never weighs two texels — so the
// test above pins the conversion's order but not the space the filter works in.
// A 1.5× scale puts the destination centres at fractional image coordinates:
//
//   (0.333, 0.333)  (1.000, 0.333)  (1.667, 0.333)
//   (0.333, 1.000)  (1.000, 1.000)  (1.667, 1.000)
//   (0.333, 1.667)  (1.000, 1.667)  (1.667, 1.667)
//
// The taps are taken at `coordinate − 0.5`, so the middle row and column weigh
// two texels 0.5/0.5 (page (1, 0) is half `TEXEL_00` and half `TEXEL_10`) and
// the centre weighs all four a quarter each; the outer row and column clamp to
// a single texel and repeat it. The two plausible wrong readings are visible
// here and nowhere else: filtering the premultiplied **sRGB** value and
// converting the mix once answers red `0.49064723` at page (1, 1), and
// filtering **straight alpha** and premultiplying at the end answers green
// `0.12878995` and blue `0.10891713` there — against the values below.
const AT_1_5X: [[f64; 4]; 9] = [
    TEXEL_00,
    [
        0.750_980_406_999_588,
        0.162_107_028_05,
        0.038_502_363_485,
        0.750_980_406_999_588,
    ],
    TEXEL_10,
    [
        0.255_001_541_499_588,
        0.071_107_000_2,
        0.053_730_035_39,
        0.329_411_782_499_588,
    ],
    [
        0.521_895_883_75,
        0.121_378_053_025,
        0.047_656_391_742_5,
        0.664_705_891_249_794,
    ],
    [0.788_790_226, 0.171_649_105_85, 0.041_582_748_095, 1.0],
    TEXEL_01,
    [
        0.292_811_360_5,
        0.080_649_078,
        0.056_810_42,
        0.578_431_375_5,
    ],
    TEXEL_11,
];

#[test]
fn arq_117_a_scaled_rgba8_image_is_filtered_after_each_texel_is_converted() {
    let image = ac1_source();
    let export = drawn(
        IRect::new(0, 0, 3, 3),
        PixelFormat::RgbaF32,
        image_task(Affine::scale(1.5), &image),
    );
    // The five mixing pixels first — they are the case this test exists for —
    // then the four whose samples clamp to a single texel, as the control that
    // the geometry still lands where the table says.
    let order = [
        (1, 1),
        (1, 0),
        (0, 1),
        (2, 1),
        (1, 2),
        (0, 0),
        (2, 0),
        (0, 2),
        (2, 2),
    ];
    for (x, y) in order {
        let index = usize::try_from(y * 3 + x).unwrap();
        assert_close(
            channels(&export, x, y),
            AT_1_5X[index],
            &format!("page ({x}, {y}) of the 1.5× scale"),
        );
    }
}

// ---------------------------------------------------------------------------
// AC2 — an `RgbaF32` texel is linear premultiplied already, and untouched
// ---------------------------------------------------------------------------

// ARQ-117: `RgbaF32` carries linear premultiplied values, so no conversion may
// run on them. The values are chosen so a stray sRGB decode would move every
// channel by a visible amount (0.25 would come back as 0.0509, 0.5 as 0.2140).
#[test]
fn arq_117_an_rgba_f32_texel_is_taken_as_linear_premultiplied() {
    let texels = [
        [0.25_f32, 0.125, 0.0625, 0.5],
        [0.5, 0.5, 0.5, 1.0],
        [0.125, 0.25, 0.5, 0.75],
        [1.0, 0.5, 0.25, 1.0],
    ];
    let image = rgba32_image(2, 2, &texels);
    let region = IRect::new(0, 0, 4, 2);
    let export = drawn(
        region,
        PixelFormat::RgbaF32,
        image_task(Affine::IDENTITY, &image),
    );
    for (index, texel) in texels.iter().enumerate() {
        let (x, y) = (
            u32::try_from(index % 2).unwrap(),
            u32::try_from(index / 2).unwrap(),
        );
        assert_eq!(
            channels(&export, x, y),
            *texel,
            "page ({x}, {y}) is the texel as it stands"
        );
    }
}

// ---------------------------------------------------------------------------
// AC3 — a 1×1 image scaled over the whole tile is that colour everywhere
// ---------------------------------------------------------------------------

// ARQ-121: a single texel sampled at any position clamps to itself, so the tile
// is that colour with no filter artefact anywhere, edges included.
#[test]
fn arq_121_a_one_texel_image_scaled_over_the_tile_is_that_colour() {
    let texel = [0.1_f32, 0.2, 0.3, 0.5];
    let image = rgba32_image(1, 1, &[texel]);
    let region = IRect::new(0, 0, 8, 8);
    let export = drawn(
        region,
        PixelFormat::RgbaF32,
        image_task(Affine::scale(8.0), &image),
    );
    for y in 0..8 {
        for x in 0..8 {
            assert_eq!(
                channels(&export, x, y),
                texel,
                "page ({x}, {y}) is the one texel, with no artefact"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// AC4 — an integer 2× scale takes its weights at texel centres
// ---------------------------------------------------------------------------

// ARQ-121: the sample position is the target pixel's centre mapped into image
// space with the inverse transform, and the texel centres sit at +0.5. With a
// 2× scale the weights are exact quarters, which is what a hand computation of
// four columns [0, 0.25, 0.5, 1] over eight page pixels gives:
//
//   x: 0      1       2       3       4       5      6      7
//      0   0.0625  0.1875  0.3125  0.4375  0.625  0.875    1
//
// The first and last pixels are the image's own edge texels exactly: the sample
// point of x = 0 falls outside the texel grid and its taps clamp to column 0,
// which is what "clamped to the edge" means. Taking the centres at the corners
// instead (no −0.5 in the sample position) starts at 0.0625 and never reaches
// an edge texel — the same eight numbers, shifted by one page pixel.
const AT_2X: [f32; 8] = [0.0, 0.0625, 0.1875, 0.3125, 0.4375, 0.625, 0.875, 1.0];
/// What the corner reading of AC4 gives, for the contrast the test states.
const AT_2X_CORNERS: [f32; 8] = [0.0625, 0.1875, 0.3125, 0.4375, 0.625, 0.875, 1.0, 1.0];

#[test]
fn arq_121_an_integer_scale_takes_its_weights_at_texel_centres() {
    let image = rgba32_image(
        4,
        1,
        &[
            [0.0, 0.0, 0.0, 1.0],
            [0.25, 0.25, 0.25, 1.0],
            [0.5, 0.5, 0.5, 1.0],
            [1.0, 1.0, 1.0, 1.0],
        ],
    );
    let export = drawn(
        IRect::new(0, 0, 8, 1),
        PixelFormat::RgbaF32,
        image_task(Affine::scale(2.0), &image),
    );
    for x in 0..8u32 {
        let pixel = channels(&export, x, 0);
        let index = usize::try_from(x).unwrap();
        let expected = AT_2X[index];
        assert_eq!(
            pixel,
            [expected, expected, expected, 1.0],
            "page ({x}, 0) of the 2× scale"
        );
        // The corner reading of the same sample is a different number at every
        // pixel but the last, so this is the criterion's contrast, not taste.
        if index < 7 {
            assert_ne!(
                pixel[0], AT_2X_CORNERS[index],
                "page ({x}, 0) is not the corner reading of the sample"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// AC5 — a rotation of a checkerboard by 90 degrees is the rotated checkerboard
// ---------------------------------------------------------------------------

// ARQ-42: a quarter turn maps the texel centres onto themselves, so the sampler
// must return each texel exactly. The transform is the one the geometry gives,
// computed here by hand: rotating by π/2 about the centre (2, 2) of a 4×4 image
// maps page (x, y) to image (y, 3 − x), so the expected page (x, y) is the
// source texel (y, 3 − x) — a checkerboard with every colour inverted.
#[test]
fn arq_42_a_checkerboard_rotated_by_ninety_degrees_is_the_rotated_checkerboard() {
    let mut texels = Vec::with_capacity(16);
    for row in 0..4u32 {
        for column in 0..4u32 {
            let value = if (column + row) % 2 == 0 { 0.0 } else { 1.0 };
            texels.push([value, value, value, 1.0]);
        }
    }
    let image = rgba32_image(4, 4, &texels);
    let export = drawn(
        IRect::new(0, 0, 4, 4),
        PixelFormat::RgbaF32,
        image_task(
            Affine::rotate_about(FRAC_PI_2, Point::new(2.0, 2.0)),
            &image,
        ),
    );
    for y in 0..4u32 {
        for x in 0..4u32 {
            // Texel (column, row) of the source is `4·row + column`, and the
            // page pixel (x, y) carries the source texel (y, 3 − x).
            let source = (3 - x) * 4 + y;
            let expected = texels[usize::try_from(source).unwrap()];
            assert_eq!(
                channels(&export, x, y),
                expected,
                "page ({x}, {y}) is the source texel (y, {})",
                3 - x
            );
        }
    }
    // The page is not the source as it stands: the quarter turn is applied.
    assert_eq!(
        channels(&export, 0, 0),
        texels[12],
        "the top-left of the page is the bottom-left texel"
    );
}

// ---------------------------------------------------------------------------
// AC6 — the cull, and a tile an image only partly covers
// ---------------------------------------------------------------------------

// ARQ-149: an image whose page bounds miss the tile contributes no pixel. The
// cull and the per-pixel destination clip both guarantee it, so the assertion
// is on the surface: every pixel keeps what it had.
#[test]
fn arq_149_the_cull_skips_an_image_that_misses_the_tile() {
    let image = ac1_source();
    let export = drawn(
        IRect::new(0, 0, 8, 8),
        PixelFormat::RgbaF32,
        image_task(Affine::translate((100.0, 100.0)), &image),
    );
    for y in 0..8 {
        for x in 0..8 {
            assert!(
                is_clear(&export, x, y),
                "page ({x}, {y}) is not covered by an image at (100, 100)"
            );
        }
    }
}

// ARQ-149: an image that only partly covers the tile draws the pixels whose
// sample point falls inside it and leaves the rest alone, so the destination is
// clipped to the image rather than the tile flooded with its edge colour. The
// tile also sits away from the page origin, which is what the cull must use.
#[test]
fn arq_149_an_image_that_partly_covers_the_tile_draws_only_its_own_window() {
    let image = ac1_source();
    // Image (0, 0)-(2, 2) at page (-1, -1): only page pixel (0, 0) of the tile
    // is inside it, and the texel that lands there is the image's (1, 1).
    let covered = drawn(
        IRect::new(0, 0, 4, 4),
        PixelFormat::RgbaF32,
        image_task(Affine::translate((-1.0, -1.0)), &image),
    );
    assert_close(
        channels(&covered, 0, 0),
        TEXEL_11,
        "page (0, 0) is the image's texel (1, 1)",
    );
    for (x, y) in [(1, 0), (0, 1), (1, 1), (3, 3), (2, 0)] {
        assert!(
            is_clear(&covered, x, y),
            "page ({x}, {y}) is outside the image and stays transparent"
        );
    }

    // The same image placed by its transform over a tile that is not at the
    // page origin: the coverage is the image's page-space quad, not its own
    // rectangle at (0, 0).
    let offset = drawn(
        IRect::new(16, 8, 20, 12),
        PixelFormat::RgbaF32,
        image_task(Affine::translate((16.0, 8.0)), &image),
    );
    assert_close(
        channels(&offset, 0, 0),
        TEXEL_00,
        "the tile's top-left is the image's (0, 0)",
    );
    assert_close(channels(&offset, 1, 1), TEXEL_11, "and its (1, 1)");
    assert!(
        is_clear(&offset, 2, 2),
        "the rest of the tile is outside the image"
    );
}

// ARQ-149: the destination clip is the image's own quad, not its page-space
// bounding box. A 4×4 white image turned 45° about its centre (2, 2) has its
// box at (−0.828, −0.828)-(4.828, 4.828), and three of the box's pixels fall
// outside the quad: page (0, 0) maps to image (−0.121, 2), page (3, 0) to
// (2, −0.121) and page (4, 4) to (5.536, 2). Clipping to the box and clamping
// the sample — the plausible wrong reading of "clipped to the image" — paints
// those three with the edge texel's colour; the quad test leaves them alone,
// which is what a rotated image must do to the corners of its box.
#[test]
fn arq_149_a_rotated_image_paints_its_quad_not_the_corners_of_its_box() {
    let image = rgba32_image(4, 4, &[[1.0, 1.0, 1.0, 1.0]; 16]);
    let region = IRect::new(0, 0, 5, 5);
    let turned = Affine::rotate_about(FRAC_PI_4, Point::new(2.0, 2.0));
    let outside = [(0, 0), (3, 0), (4, 4)];

    let export = drawn(region, PixelFormat::RgbaF32, image_task(turned, &image));
    assert_eq!(
        channels(&export, 2, 2),
        [1.0; 4],
        "the centre of the quad is painted"
    );
    for (x, y) in outside {
        assert!(
            is_clear(&export, x, y),
            "page ({x}, {y}) is inside the image's box and outside its quad"
        );
    }

    // Over an opaque backdrop the same pixels keep the backdrop: the sampler
    // never writes them, so nothing of the image reaches the gap.
    let over = drawn(
        region,
        PixelFormat::RgbaF32,
        vec![
            fill(0.0, 0.0, 5.0, 5.0, srgb(0.2, 0.4, 0.6, 1.0)),
            RenderOp::DrawImage {
                image: Arc::clone(&image),
                transform: turned,
            },
        ],
    );
    for (x, y) in outside {
        assert_close(
            channels(&over, x, y),
            GREY,
            "page ({x}, {y}) keeps the backdrop the rotated image does not cover",
        );
    }
    assert_eq!(
        channels(&over, 2, 2),
        [1.0; 4],
        "and the quad is painted over it"
    );
}

// ---------------------------------------------------------------------------
// A source this delivery cannot sample
// ---------------------------------------------------------------------------

// ARQ-117: `Alpha8` has no colour, so it is not a drawing source. The backend
// reports it, in the vocabulary the crate already uses for a rejected format,
// instead of drawing a page with a guessed colour.
#[test]
fn an_alpha8_image_is_refused_rather_than_drawn() {
    let image = Arc::new(ImageBuf::new(1, 1, PixelBuf::Alpha8(vec![128])).expect("the image"));
    let scene = plan(vec![task(
        1,
        IRect::new(0, 0, 4, 4),
        image_task(Affine::scale(4.0), &image),
    )]);
    let mut frame = Frame::open(
        CpuBackend::new(),
        target(4, 4, (0, 0), (4, 4), PixelFormat::RgbaF32),
    );
    let mut cache = RecordingCache::default();
    let error = frame
        .execute(&scene, &mut cache, &CancelToken::new())
        .expect_err("an Alpha8 image is not a source");
    assert!(
        matches!(
            error,
            RenderError::Effect(gafira_raster::EffectError::UnsupportedFormat {
                format: PixelFormat::Alpha8
            })
        ),
        "reported as an unsupported format, not drawn: {error:?}"
    );
}

// ARQ-149 (W10): the cull runs before the source is read, so an `Alpha8` image
// whose page bounds miss the tile is skipped without a report, while the same
// image covering the tile is refused. This skip is the cull's only observable
// trace and the reason it is pinned here: a culled image changes no pixel
// whether the cull runs or not, because the per-pixel destination clip rejects
// the same sample points, so removing the cull leaves every page right and
// only turns this skip into an error.
#[test]
fn arq_149_an_off_tile_alpha8_image_is_culled_before_the_format_is_refused() {
    let image = Arc::new(ImageBuf::new(1, 1, PixelBuf::Alpha8(vec![128])).expect("the image"));
    let region = IRect::new(0, 0, 4, 4);

    // Off the tile at (100, 100): the op contributes nothing, so no texel is
    // read and the unsupported format is never reached.
    let mut frame = Frame::open(
        CpuBackend::new(),
        target(4, 4, (0, 0), (4, 4), PixelFormat::RgbaF32),
    );
    let mut cache = RecordingCache::default();
    let scene = plan(vec![task(
        1,
        region,
        image_task(Affine::translate((100.0, 100.0)), &image),
    )]);
    frame
        .execute(&scene, &mut cache, &CancelToken::new())
        .expect("a culled image is not sampled, so its format is not refused");
    let export = frame.read(region).expect("the region reads back");
    for y in 0..4u32 {
        for x in 0..4u32 {
            assert!(
                is_clear(&export, x, y),
                "page ({x}, {y}) is not covered by an image at (100, 100)"
            );
        }
    }

    // The control: over the tile, the same image reaches the sampler and is
    // refused, so the skip above is the cull and not a swallowed error.
    let mut frame = Frame::open(
        CpuBackend::new(),
        target(4, 4, (0, 0), (4, 4), PixelFormat::RgbaF32),
    );
    let mut cache = RecordingCache::default();
    let scene = plan(vec![task(
        2,
        region,
        image_task(Affine::scale(4.0), &image),
    )]);
    let error = frame
        .execute(&scene, &mut cache, &CancelToken::new())
        .expect_err("an Alpha8 image that covers the tile is refused");
    assert!(
        matches!(
            error,
            RenderError::Effect(gafira_raster::EffectError::UnsupportedFormat {
                format: PixelFormat::Alpha8
            })
        ),
        "reported as an unsupported format, not skipped: {error:?}"
    );
}
