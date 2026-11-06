//! The behaviour tests of `CB5`: the mask over the open layer, the effect over
//! the grown tile and the transition between the layer and its background.
//!
//! The three readings are the ones 02 §4.6 settles (SP2, closed 2026-11-05),
//! so these tests pin the sentence each criterion quotes and not a choice of
//! this delivery.

use std::sync::Arc;

use gafira_doc::BlendMode;
use gafira_ports::render::RenderError;
use gafira_raster::{AlphaMask, PixelFormat};

use super::*;

/// A mask of `width × height` coverage values, row-major.
fn mask(width: u32, height: u32, data: Vec<u8>) -> Arc<AlphaMask> {
    Arc::new(AlphaMask::new(width, height, data).expect("the mask"))
}

/// A mask whose every pixel carries `coverage`.
fn solid_mask(width: u32, height: u32, coverage: u8) -> Arc<AlphaMask> {
    mask(
        width,
        height,
        vec![coverage; usize::try_from(width * height).unwrap()],
    )
}

fn masked(mask: Arc<AlphaMask>) -> RenderOp {
    RenderOp::ApplyMask { mask }
}

// ---------------------------------------------------------------------------
// AC1 — the four premultiplied channels take the mask's coverage
// ---------------------------------------------------------------------------

// ARQ-119, 02 §4.6: `ApplyMask` applies its coverage to the open layer's
// premultiplied pixels, all four channels, so the result stays premultiplied.
#[test]
fn arq_119_a_mask_scales_all_four_premultiplied_channels() {
    let region = IRect::new(0, 0, 4, 4);
    let colour = srgb(0.5, 0.5, 0.5, 0.5);
    let target_desc = target(4, 4, (0, 0), PixelFormat::RgbaF32);

    // The layer's own premultiplied content, from the same plan without a mask.
    let base_plan = plan(vec![task(
        1,
        region,
        vec![
            push(),
            fill(0.0, 0.0, 4.0, 4.0, colour),
            pop(BlendMode::Normal, 1.0),
        ],
    )]);
    let base = run(target_desc, &base_plan);

    // One coverage per column, so a mask indexed at the wrong stride or read
    // only at its first pixel cannot agree with the whole row.
    let coverages = [0u8, 64, 128, 255];
    let data = (0..4).flat_map(|_row| coverages).collect::<Vec<_>>();
    let scene = plan(vec![task(
        1,
        region,
        vec![
            push(),
            fill(0.0, 0.0, 4.0, 4.0, colour),
            masked(mask(4, 4, data)),
            pop(BlendMode::Normal, 1.0),
        ],
    )]);
    let got = run(target_desc, &scene);

    for (x, coverage) in coverages.iter().enumerate() {
        let x = u32::try_from(x).unwrap();
        let expected_scale = f32::from(*coverage) / 255.0;
        let before = channels(&base, x, 2);
        let after = channels(&got, x, 2);
        for channel in 0..4 {
            let expected = before[channel] * expected_scale;
            assert!(
                (after[channel] - expected).abs() <= 1e-6,
                "ARQ-119 column {x} channel {channel} with coverage {coverage}: \
                 the mask multiplies the premultiplied channel, got {} expected {expected}",
                after[channel]
            );
        }
        // The layer's colour and its alpha are different numbers, so a mask
        // that only touched the alpha (or only the colour) cannot pass.
        assert!(before[0] > 0.0 && before[3] > 0.0, "the fixture has colour");
        assert!(
            (before[0] - before[3]).abs() > 0.1,
            "the fixture tells the colour channels from the alpha"
        );
    }
    // Coverage 0 is transparent and 255 leaves the layer untouched.
    assert!(is_clear(&got, 0, 2), "coverage 0 clears the layer");
    assert_eq!(channels(&got, 3, 2), channels(&base, 3, 2), "255 keeps it");
}

// ---------------------------------------------------------------------------
// AC2 — a smaller mask masks only its own rectangle
// ---------------------------------------------------------------------------

// 02 §4.6: `ApplyMask` anchors at the layer's top-left with no rescale, so a
// mask smaller than the layer masks only its own rectangle and every pixel
// outside it keeps the value it had. This is the spec's own acceptance case
// (a 128 × 128 mask over a 256 × 256 layer).
#[test]
fn spec_02_4_6_a_mask_smaller_than_the_layer_masks_only_its_rectangle() {
    let region = IRect::new(0, 0, 256, 256);
    let target_desc = target(256, 256, (0, 0), PixelFormat::RgbaF32);
    let colour = srgb(0.25, 0.5, 0.75, 0.5);
    let ops_with = |mask: Arc<AlphaMask>| {
        vec![
            push(),
            fill(0.0, 0.0, 256.0, 256.0, colour),
            masked(mask),
            pop(BlendMode::Normal, 1.0),
        ]
    };
    let base = run(
        target_desc,
        &plan(vec![task(
            1,
            region,
            vec![
                push(),
                fill(0.0, 0.0, 256.0, 256.0, colour),
                pop(BlendMode::Normal, 1.0),
            ],
        )]),
    );
    let cleared = run(
        target_desc,
        &plan(vec![task(1, region, ops_with(solid_mask(128, 128, 0)))]),
    );
    let opaque = run(
        target_desc,
        &plan(vec![task(1, region, ops_with(solid_mask(128, 128, 255)))]),
    );

    assert_eq!(
        channels(&base, 10, 10),
        channels(&base, 200, 200),
        "the fixture is one flat colour"
    );
    assert!(!is_clear(&base, 200, 200), "the fixture draws the layer");
    for y in 0..256u32 {
        for x in 0..256u32 {
            let inside = x < 128 && y < 128;
            if inside {
                assert!(
                    is_clear(&cleared, x, y),
                    "({x}, {y}) is under the mask's 0 and must be transparent"
                );
            } else {
                assert_eq!(
                    channels(&cleared, x, y),
                    channels(&base, x, y),
                    "({x}, {y}) is outside the mask and keeps the layer"
                );
            }
            assert_eq!(
                channels(&opaque, x, y),
                channels(&base, x, y),
                "({x}, {y}) under a 255 mask is unchanged, byte for byte"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// AC3 — an `ApplyMask` outside a layer is refused
// ---------------------------------------------------------------------------

// ARQ-110, 02 §4.6: an `ApplyMask` outside a `PushLayer`/`PopLayer` pair is
// outside the plan's domain and its result is unspecified, so it is refused
// rather than given a behaviour.
#[test]
fn spec_02_4_6_a_mask_outside_a_layer_is_refused() {
    let scene = plan(vec![task(
        1,
        IRect::new(0, 0, 4, 4),
        vec![
            fill(0.0, 0.0, 4.0, 4.0, white()),
            masked(solid_mask(4, 4, 0)),
        ],
    )]);
    let mut cache = RecordingCache::default();
    let mut frame = Frame::open(
        CpuBackend::new(),
        target(4, 4, (0, 0), PixelFormat::RgbaF32),
    );
    let token = CancelToken::new();
    let outcome = frame.execute(&scene, &mut cache, &token);
    assert!(
        matches!(outcome, Err(RenderError::DriverBlocked)),
        "a mask with no open layer is out of the plan's domain: {outcome:?}"
    );
}
