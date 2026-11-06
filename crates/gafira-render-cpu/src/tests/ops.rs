//! The behaviour tests of `CB5`: the mask over the open layer, the effect over
//! the grown tile and the transition between the layer and its background.
//!
//! The three readings are the ones 02 §4.6 settles (SP2, closed 2026-11-05),
//! so these tests pin the sentence each criterion quotes and not a choice of
//! this delivery.

use std::sync::{Arc, Mutex};

use gafira_doc::BlendMode;
use gafira_doc::contract::{Describe, Descriptor, ParamSchema, ParamValues};
use gafira_doc::msg::MessageKey;
use gafira_ports::render::RenderError;
use gafira_raster::transitions::{Dissolve, HandleReq, Transition};
use gafira_raster::{
    AlphaMask, BufferError, EffectCx, EffectError, ImageBuf, ImageView, ImageViewMut, PixelBuf,
    PixelData, PixelDataMut, PixelFormat,
};

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

/// A transition that records the `progress` it was handed and copies `to`.
struct ProbeTransition {
    descriptor: Descriptor,
    schema: ParamSchema,
    seen: Mutex<Vec<f32>>,
}

impl ProbeTransition {
    fn new() -> Self {
        Self {
            descriptor: Descriptor::new("test.probe.transition", MessageKey("test.probe"), 1),
            schema: ParamSchema::new(Vec::new()),
            seen: Mutex::new(Vec::new()),
        }
    }

    fn seen(&self) -> Vec<f32> {
        self.seen.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

impl Describe for ProbeTransition {
    fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }
}

impl Transition for ProbeTransition {
    fn params(&self) -> &ParamSchema {
        &self.schema
    }

    fn handles(&self) -> HandleReq {
        HandleReq::None
    }

    fn run_cpu(
        &self,
        _cx: &EffectCx,
        _p: &ParamValues,
        progress: f32,
        _from: ImageView<'_>,
        to: ImageView<'_>,
        mut dst: ImageViewMut<'_>,
    ) -> Result<(), EffectError> {
        self.seen
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(progress);
        let source = match to.data() {
            PixelData::RgbaF32(data) => data.to_vec(),
            other => panic!("the transition runs on f32, not {other:?}"),
        };
        match dst.data_mut() {
            PixelDataMut::RgbaF32(data) => data.copy_from_slice(&source),
            _ => panic!("the transition writes f32"),
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// AC6 — a transition's two inputs
// ---------------------------------------------------------------------------

/// The raw premultiplied values of an `RgbaF32` surface, for byte-for-byte
/// comparisons.
fn values(image: &ImageBuf) -> Vec<f32> {
    match image.data() {
        PixelBuf::RgbaF32(data) => data.clone(),
        _ => panic!("the target asked for RgbaF32"),
    }
}

/// A `Dissolve` at `progress`; `02 §4.13` gives it no parameters.
fn dissolve(progress: f32) -> RenderOp {
    RenderOp::ApplyTransition {
        transition: Arc::new(Dissolve::new()) as Arc<dyn Transition>,
        params: ParamValues::new(),
        progress,
    }
}

/// The layer over a background, with a `Dissolve` at `progress` inside it.
fn crossfade(progress: f32) -> RenderPlan {
    let region = IRect::new(0, 0, 4, 4);
    plan(vec![task(
        1,
        region,
        vec![
            // The background: what the surface holds when the layer opens.
            fill(0.0, 0.0, 4.0, 4.0, srgb(0.5, 0.25, 0.5, 1.0)),
            push(),
            // The layer's content since the `PushLayer`, translucent so the pop
            // changes the page and the two inputs cannot be confused.
            fill(0.0, 0.0, 2.0, 4.0, srgb(0.25, 1.0, 0.5, 0.5)),
            dissolve(progress),
            pop(BlendMode::Normal, 1.0),
        ],
    )])
}

// 02 §4.6, ARQ-118: `to` is what was drawn since the `PushLayer`, `from` is the
// layer's background — the pixels `PopLayer` composites onto. Progress 0 gives
// the background exactly and 1 the drawn content exactly.
#[test]
fn spec_02_4_6_a_transition_takes_from_the_background_and_to_the_layer() {
    let target_desc = target(4, 4, (0, 0), PixelFormat::RgbaF32);
    let background = run(
        target_desc,
        &plan(vec![task(
            1,
            IRect::new(0, 0, 4, 4),
            vec![fill(0.0, 0.0, 4.0, 4.0, srgb(0.5, 0.25, 0.5, 1.0))],
        )]),
    );
    // The same layer with no transition: the content drawn since the push.
    let plain = plan(vec![task(
        1,
        IRect::new(0, 0, 4, 4),
        vec![
            fill(0.0, 0.0, 4.0, 4.0, srgb(0.5, 0.25, 0.5, 1.0)),
            push(),
            fill(0.0, 0.0, 2.0, 4.0, srgb(0.25, 1.0, 0.5, 0.5)),
            pop(BlendMode::Normal, 1.0),
        ],
    )]);
    let drawn = run(target_desc, &plain);

    let at_zero = run(target_desc, &crossfade(0.0));
    let at_one = run(target_desc, &crossfade(1.0));
    let at_half = run(target_desc, &crossfade(0.5));

    assert_eq!(
        values(&at_zero),
        values(&background),
        "progress 0 is the layer's background, byte for byte"
    );
    assert_eq!(
        values(&at_one),
        values(&drawn),
        "progress 1 is the content drawn since the PushLayer, byte for byte"
    );
    assert_ne!(
        values(&background),
        values(&drawn),
        "the fixture tells the two inputs apart"
    );
    // A midpoint is a genuine mix of both, not a copy of either.
    assert_ne!(values(&at_half), values(&background), "half is not `from`");
    assert_ne!(values(&at_half), values(&drawn), "half is not `to`");
}

// ---------------------------------------------------------------------------
// AC7 — the progress clip and the non-finite refusal
// ---------------------------------------------------------------------------

// ARQ-118: a finite `progress` outside `[0, 1]` is clipped to the range.
#[test]
fn arq_118_a_finite_progress_outside_the_range_is_clipped() {
    let target_desc = target(4, 4, (0, 0), PixelFormat::RgbaF32);
    for (outside, inside) in [(-0.5f32, 0.0f32), (1.5, 1.0)] {
        assert_eq!(
            values(&run(target_desc, &crossfade(outside))),
            values(&run(target_desc, &crossfade(inside))),
            "{outside} is clipped to {inside}"
        );
    }
}

// ARQ-118, ARQ-110: NaN and an infinity are refused, naming the field, and no
// partial surface is delivered.
#[test]
fn arq_110_a_non_finite_progress_is_refused() {
    let target_desc = target(4, 4, (0, 0), PixelFormat::RgbaF32);
    for progress in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut cache = RecordingCache::default();
        let mut frame = Frame::open(CpuBackend::new(), target_desc);
        let token = CancelToken::new();
        let outcome = frame.execute(&crossfade(progress), &mut cache, &token);
        assert!(
            matches!(
                outcome,
                Err(RenderError::Effect(EffectError::Buffer(
                    BufferError::NonFinite {
                        field: "progress",
                        ..
                    }
                )))
            ),
            "{progress} answers an error that names the field: {outcome:?}"
        );
        assert!(cache.puts().is_empty(), "{progress} writes no tile");
    }
}

// 02 §4.6, ARQ-118: `from` is the background the `PopLayer` composites onto, so
// for a nested layer it is the enclosing layer's pixels and not the page.
#[test]
fn spec_02_4_6_a_nested_transition_takes_from_the_layer_it_composites_onto() {
    let region = IRect::new(0, 0, 4, 4);
    let target_desc = target(4, 4, (0, 0), PixelFormat::RgbaF32);
    let outer = srgb(0.0, 1.0, 0.0, 0.5);
    let scene = plan(vec![task(
        1,
        region,
        vec![
            fill(0.0, 0.0, 4.0, 4.0, srgb(0.5, 0.25, 0.5, 1.0)),
            push(),
            fill(0.0, 0.0, 4.0, 4.0, outer),
            push(),
            fill(0.0, 0.0, 2.0, 4.0, white()),
            dissolve(0.0),
            pop(BlendMode::Normal, 1.0),
            pop(BlendMode::Normal, 1.0),
        ],
    )]);
    let got = channels(&run(target_desc, &scene), 3, 2);

    // The nested layer becomes the outer layer's content `L`; the inner pop
    // composites it over `L`, and the outer pop composites that over the page's
    // opaque `B`.
    let linear = outer.to_linear_srgb();
    let layer = [linear.r * 0.5, linear.g * 0.5, linear.b * 0.5, 0.5];
    let inner = blend_f32(BlendMode::Normal, layer, layer).expect("the blend");
    let backdrop = channels(
        &run(
            target_desc,
            &plan(vec![task(
                1,
                region,
                vec![fill(0.0, 0.0, 4.0, 4.0, srgb(0.5, 0.25, 0.5, 1.0))],
            )]),
        ),
        3,
        2,
    );
    let expected = blend_f32(BlendMode::Normal, backdrop, inner).expect("the blend");
    assert_eq!(
        got, expected,
        "at 0 the nested layer holds the layer it composites onto"
    );
    // The page's own pixels are not `from`: that reading leaves the inner layer
    // transparent, so the outer layer reaches the page as it was drawn.
    let page_only = blend_f32(BlendMode::Normal, backdrop, layer).expect("the blend");
    assert_ne!(
        expected, page_only,
        "the fixture tells the two readings apart"
    );
}

// ARQ-118: the op clips the progress it hands the transition, and it refuses a
// non-finite one before the transition runs — the core transitions check this
// themselves, but a transition is a plugin and the op's contract is its own.
#[test]
fn arq_118_the_backend_clips_the_progress_it_hands_a_transition() {
    let target_desc = target(4, 4, (0, 0), PixelFormat::RgbaF32);
    for (given, clipped) in [(-0.5f32, 0.0f32), (0.25, 0.25), (1.5, 1.0)] {
        let probe = Arc::new(ProbeTransition::new());
        let scene = plan(vec![task(
            1,
            IRect::new(0, 0, 4, 4),
            vec![
                fill(0.0, 0.0, 4.0, 4.0, srgb(0.5, 0.25, 0.5, 1.0)),
                push(),
                fill(0.0, 0.0, 2.0, 4.0, white()),
                RenderOp::ApplyTransition {
                    transition: Arc::clone(&probe) as Arc<dyn Transition>,
                    params: ParamValues::new(),
                    progress: given,
                },
                pop(BlendMode::Normal, 1.0),
            ],
        )]);
        run(target_desc, &scene);
        assert_eq!(
            probe.seen(),
            vec![clipped],
            "{given} reaches the op as {clipped}"
        );
    }
    for progress in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let probe = Arc::new(ProbeTransition::new());
        let scene = plan(vec![task(
            1,
            IRect::new(0, 0, 4, 4),
            vec![
                push(),
                RenderOp::ApplyTransition {
                    transition: Arc::clone(&probe) as Arc<dyn Transition>,
                    params: ParamValues::new(),
                    progress,
                },
                pop(BlendMode::Normal, 1.0),
            ],
        )]);
        let mut cache = RecordingCache::default();
        let mut frame = Frame::open(CpuBackend::new(), target_desc);
        let token = CancelToken::new();
        let outcome = frame.execute(&scene, &mut cache, &token);
        assert!(
            matches!(outcome, Err(RenderError::Effect(EffectError::Buffer(_)))),
            "{progress} is refused: {outcome:?}"
        );
        assert!(
            probe.seen().is_empty(),
            "{progress} is refused before the transition runs"
        );
    }
}
