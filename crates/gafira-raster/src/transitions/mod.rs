//! Transitions between two frames (02 §4.13, ARQ-118).

mod cut;
mod dissolve;
mod fade;
mod guard;
mod mix;
mod slide;
mod wipe;
mod zoom;

use gafira_doc::contract::{AudioCrossfade, Describe, ParamSchema, ParamValues};

pub use cut::Cut;
pub use dissolve::Dissolve;
pub use fade::Fade;
pub use slide::Slide;
pub use wipe::Wipe;
pub use zoom::Zoom;

use crate::{EffectCx, EffectError, ImageView, ImageViewMut};

/// The frame margin a transition asks of each clip (02 §4.13).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandleReq {
    /// Both clips.
    Both,
    /// Only the outgoing clip.
    From,
    /// Only the incoming clip.
    To,
    /// No margin.
    None,
}

/// Bytes handed to a GPU pipeline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UniformBlock(pub Vec<u8>);

/// The optional WGSL variant of a transition. No implementation in this cut.
pub trait GpuTransition: Send + Sync {
    /// The WGSL source.
    fn wgsl(&self) -> &'static str;
    /// Uniform bytes for resolved parameters (ARQ-110: `p` is resolved, `progress` finite and clipped).
    fn bind(&self, p: &ParamValues, progress: f32) -> UniformBlock;
}

/// An effect with two inputs.
pub trait Transition: Describe + Send + Sync {
    /// The parameters; the duration belongs to the clip.
    fn params(&self) -> &ParamSchema;
    /// The frame margin asked of each clip.
    fn handles(&self) -> HandleReq;
    /// Runs on whole frames: `progress` 0 gives `from`, 1 gives `to`.
    ///
    /// # Errors
    ///
    /// [`EffectError`], before anything is written to `dst`.
    fn run_cpu(
        &self,
        cx: &EffectCx,
        p: &ParamValues,
        progress: f32,
        from: ImageView<'_>,
        to: ImageView<'_>,
        dst: ImageViewMut<'_>,
    ) -> Result<(), EffectError>;
    /// The WGSL variant, if any.
    fn gpu(&self) -> Option<&dyn GpuTransition> {
        None
    }
    /// How the associated audio crossfades.
    fn audio(&self) -> AudioCrossfade {
        AudioCrossfade::EqualPower
    }
}

#[cfg(test)]
mod tests {
    use gafira_doc::contract::{ParamValue, ParamValues};

    use super::*;
    use crate::{BufferError, PixelData, PixelDataMut, PixelFormat};

    pub(crate) const SENTINEL_U8: u8 = 0xAB;
    pub(crate) const SENTINEL_F32: u32 = 0x7FC0_1234;

    /// Every transition of this delivery.
    pub(crate) fn all() -> Vec<Box<dyn Transition>> {
        vec![
            Box::new(Cut::new()),
            Box::new(Dissolve::new()),
            Box::new(Fade::new()),
            Box::new(Slide::new()),
            Box::new(Wipe::new()),
            Box::new(Zoom::new()),
        ]
    }

    pub(crate) fn frame_u8(n: usize, seed: u8) -> Vec<u8> {
        (0..n * 4)
            .map(|i| (i as u8).wrapping_mul(37).wrapping_add(seed))
            .collect()
    }

    /// Awkward values: signed zero, out of [0, 1], tiny, a transparent pixel with colour.
    pub(crate) fn frame_f32(n: usize, seed: f32) -> Vec<f32> {
        let odd = [-0.0f32, 1.5, 1.0e-30, -0.25, 0.3, 0.0, 2.0];
        (0..n * 4)
            .map(|i| odd[(i + seed as usize) % 7] + seed * 0.001)
            .collect()
    }

    pub(crate) fn run_u8(
        t: &dyn Transition,
        progress: f32,
        from: &[u8],
        to: &[u8],
        dst: &mut [u8],
        size: (u32, u32),
    ) -> Result<(), EffectError> {
        t.run_cpu(
            &EffectCx::whole(1.0),
            &ParamValues::new(),
            progress,
            ImageView::new(size.0, size.1, PixelData::Rgba8(from)).unwrap(),
            ImageView::new(size.0, size.1, PixelData::Rgba8(to)).unwrap(),
            ImageViewMut::new(size.0, size.1, PixelDataMut::Rgba8(dst)).unwrap(),
        )
    }

    pub(crate) fn run_f32(
        t: &dyn Transition,
        progress: f32,
        from: &[f32],
        to: &[f32],
        dst: &mut [f32],
        size: (u32, u32),
    ) -> Result<(), EffectError> {
        t.run_cpu(
            &EffectCx::whole(1.0),
            &ParamValues::new(),
            progress,
            ImageView::new(size.0, size.1, PixelData::RgbaF32(from)).unwrap(),
            ImageView::new(size.0, size.1, PixelData::RgbaF32(to)).unwrap(),
            ImageViewMut::new(size.0, size.1, PixelDataMut::RgbaF32(dst)).unwrap(),
        )
    }

    pub(crate) fn bits(v: &[f32]) -> Vec<u32> {
        v.iter().map(|x| x.to_bits()).collect()
    }

    fn sentinel_f32(n: usize) -> Vec<f32> {
        vec![f32::from_bits(SENTINEL_F32); n]
    }

    // FV-06 (a), ARQ-25, ARQ-118: p = 0 copies `from` and p = 1 copies `to`, byte for byte, for every
    // transition and both formats; a p outside [0, 1] clips to the nearest end; -0.0 is 0.
    #[test]
    fn arq_118_ends_are_exact_copies_for_every_transition() {
        let size = (3, 2);
        let (fu, tu) = (frame_u8(6, 1), frame_u8(6, 99));
        let (ff, tf) = (frame_f32(6, 1.0), frame_f32(6, 3.0));
        let ends: [(f32, bool); 8] = [
            (0.0, false),
            (-0.0, false),
            (-0.5, false),
            (f32::MIN, false),
            (1.0, true),
            (1.5, true),
            (f32::MAX, true),
            (1.000_000_1, true),
        ];
        for t in all() {
            let id = t.descriptor().id;
            for (p, is_to) in ends {
                let mut du = vec![SENTINEL_U8; 24];
                run_u8(t.as_ref(), p, &fu, &tu, &mut du, size).unwrap();
                assert_eq!(
                    du,
                    if is_to { tu.clone() } else { fu.clone() },
                    "{id} u8 p={p}"
                );
                let mut df = sentinel_f32(24);
                run_f32(t.as_ref(), p, &ff, &tf, &mut df, size).unwrap();
                let want = if is_to { &tf } else { &ff };
                assert_eq!(bits(&df), bits(want), "{id} f32 p={p}");
            }
        }
    }

    // ARQ-110, ARQ-118: NaN and both infinities in `progress` give Err naming `progress`,
    // before anything else is checked, and nothing is written.
    #[test]
    fn arq_110_non_finite_progress_is_err_and_writes_nothing() {
        for t in all() {
            for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                let (fu, tu) = (frame_u8(4, 1), frame_u8(4, 2));
                let mut du = vec![SENTINEL_U8; 16];
                let err = run_u8(t.as_ref(), bad, &fu, &tu, &mut du, (2, 2)).unwrap_err();
                assert_eq!(
                    err,
                    EffectError::Buffer(BufferError::NonFinite {
                        field: "progress",
                        index: 0
                    }),
                    "{} {bad}",
                    t.descriptor().id
                );
                assert!(du.iter().all(|b| *b == SENTINEL_U8));
                // Even when the sizes also differ, progress is checked first.
                let big = frame_u8(6, 3);
                let err = t
                    .run_cpu(
                        &EffectCx::whole(1.0),
                        &ParamValues::new(),
                        bad,
                        ImageView::new(2, 2, PixelData::Rgba8(&fu)).unwrap(),
                        ImageView::new(3, 2, PixelData::Rgba8(&big)).unwrap(),
                        ImageViewMut::new(2, 2, PixelDataMut::Rgba8(&mut du)).unwrap(),
                    )
                    .unwrap_err();
                assert!(matches!(err, EffectError::Buffer(_)));
            }
        }
    }

    // ARQ-118 (P15 item 6): Alpha8 is unsupported; a mix of formats names the first side
    // that differs from `from`; nothing is written.
    #[test]
    fn arq_118_formats_are_checked_before_writing() {
        for t in all() {
            let a8 = [1u8, 2, 3, 4];
            let r8 = frame_u8(4, 1);
            let rf = frame_f32(4, 1.0);
            let mut d8 = vec![SENTINEL_U8; 16];
            let mut dm = [SENTINEL_U8; 4];
            let mut df = sentinel_f32(16);
            let ctx = EffectCx::whole(1.0);
            let p = ParamValues::new();
            let id = t.descriptor().id;
            // from is Alpha8: unsupported, whatever the others are.
            let err = t
                .run_cpu(
                    &ctx,
                    &p,
                    0.3,
                    ImageView::new(2, 2, PixelData::Alpha8(&a8)).unwrap(),
                    ImageView::new(2, 2, PixelData::Alpha8(&a8)).unwrap(),
                    ImageViewMut::new(2, 2, PixelDataMut::Alpha8(&mut dm)).unwrap(),
                )
                .unwrap_err();
            assert_eq!(
                err,
                EffectError::UnsupportedFormat {
                    format: PixelFormat::Alpha8
                },
                "{id}"
            );
            assert_eq!(dm, [SENTINEL_U8; 4]);
            // to differs: field `to`.
            let err = t
                .run_cpu(
                    &ctx,
                    &p,
                    0.3,
                    ImageView::new(2, 2, PixelData::Rgba8(&r8)).unwrap(),
                    ImageView::new(2, 2, PixelData::RgbaF32(&rf)).unwrap(),
                    ImageViewMut::new(2, 2, PixelDataMut::Rgba8(&mut d8)).unwrap(),
                )
                .unwrap_err();
            assert_eq!(
                err,
                EffectError::FormatMismatch {
                    field: "to",
                    expected: PixelFormat::Rgba8,
                    actual: PixelFormat::RgbaF32
                },
                "{id}"
            );
            // to is Alpha8 under an Rgba8 `from`: a mix, not unsupported.
            let err = t
                .run_cpu(
                    &ctx,
                    &p,
                    0.3,
                    ImageView::new(2, 2, PixelData::Rgba8(&r8)).unwrap(),
                    ImageView::new(2, 2, PixelData::Alpha8(&a8)).unwrap(),
                    ImageViewMut::new(2, 2, PixelDataMut::Rgba8(&mut d8)).unwrap(),
                )
                .unwrap_err();
            assert_eq!(
                err,
                EffectError::FormatMismatch {
                    field: "to",
                    expected: PixelFormat::Rgba8,
                    actual: PixelFormat::Alpha8
                }
            );
            // dst differs: field `dst`; when both differ, `to` is named first.
            let err = t
                .run_cpu(
                    &ctx,
                    &p,
                    0.3,
                    ImageView::new(2, 2, PixelData::Rgba8(&r8)).unwrap(),
                    ImageView::new(2, 2, PixelData::Rgba8(&r8)).unwrap(),
                    ImageViewMut::new(2, 2, PixelDataMut::RgbaF32(&mut df)).unwrap(),
                )
                .unwrap_err();
            assert_eq!(
                err,
                EffectError::FormatMismatch {
                    field: "dst",
                    expected: PixelFormat::Rgba8,
                    actual: PixelFormat::RgbaF32
                }
            );
            let err = t
                .run_cpu(
                    &ctx,
                    &p,
                    0.3,
                    ImageView::new(2, 2, PixelData::Rgba8(&r8)).unwrap(),
                    ImageView::new(2, 2, PixelData::RgbaF32(&rf)).unwrap(),
                    ImageViewMut::new(2, 2, PixelDataMut::Alpha8(&mut dm)).unwrap(),
                )
                .unwrap_err();
            assert!(matches!(
                err,
                EffectError::FormatMismatch { field: "to", .. }
            ));
            // The check also runs at the ends (p = 0 and p = 1).
            for p_end in [0.0f32, 1.0] {
                let err = t
                    .run_cpu(
                        &ctx,
                        &p,
                        p_end,
                        ImageView::new(2, 2, PixelData::Rgba8(&r8)).unwrap(),
                        ImageView::new(2, 2, PixelData::RgbaF32(&rf)).unwrap(),
                        ImageViewMut::new(2, 2, PixelDataMut::Rgba8(&mut d8)).unwrap(),
                    )
                    .unwrap_err();
                assert!(matches!(err, EffectError::FormatMismatch { .. }));
            }
            assert!(d8.iter().all(|b| *b == SENTINEL_U8));
            assert!(bits(&df).iter().all(|b| *b == SENTINEL_F32));
        }
    }

    // ARQ-118: the three buffers have the same size; Err names `to` or `dst`; format is checked first.
    #[test]
    fn arq_118_sizes_are_checked_before_writing() {
        for t in all() {
            let id = t.descriptor().id;
            let (a, b) = (frame_u8(4, 1), frame_u8(6, 2));
            let mut d = vec![SENTINEL_U8; 16];
            let mut d6 = vec![SENTINEL_U8; 24];
            let ctx = EffectCx::whole(1.0);
            let p = ParamValues::new();
            for progress in [0.0f32, 0.3, 1.0] {
                let err = t
                    .run_cpu(
                        &ctx,
                        &p,
                        progress,
                        ImageView::new(2, 2, PixelData::Rgba8(&a)).unwrap(),
                        ImageView::new(3, 2, PixelData::Rgba8(&b)).unwrap(),
                        ImageViewMut::new(2, 2, PixelDataMut::Rgba8(&mut d)).unwrap(),
                    )
                    .unwrap_err();
                assert_eq!(
                    err,
                    EffectError::SizeMismatch {
                        field: "to",
                        expected: (2, 2),
                        actual: (3, 2)
                    },
                    "{id} p={progress}"
                );
                let err = t
                    .run_cpu(
                        &ctx,
                        &p,
                        progress,
                        ImageView::new(2, 2, PixelData::Rgba8(&a)).unwrap(),
                        ImageView::new(2, 2, PixelData::Rgba8(&a)).unwrap(),
                        ImageViewMut::new(3, 2, PixelDataMut::Rgba8(&mut d6)).unwrap(),
                    )
                    .unwrap_err();
                assert_eq!(
                    err,
                    EffectError::SizeMismatch {
                        field: "dst",
                        expected: (2, 2),
                        actual: (3, 2)
                    }
                );
                // Swapped sides (2x3 against 3x2) have the same pixel count and still differ.
                let err = t
                    .run_cpu(
                        &ctx,
                        &p,
                        progress,
                        ImageView::new(2, 3, PixelData::Rgba8(&b)).unwrap(),
                        ImageView::new(3, 2, PixelData::Rgba8(&b)).unwrap(),
                        ImageViewMut::new(2, 3, PixelDataMut::Rgba8(&mut d6)).unwrap(),
                    )
                    .unwrap_err();
                assert!(matches!(err, EffectError::SizeMismatch { field: "to", .. }));
            }
            assert!(d.iter().chain(&d6).all(|v| *v == SENTINEL_U8));
        }
    }

    // ARQ-118: each of width and height counts alone; formats are checked before sizes.
    #[test]
    fn arq_118_height_alone_differs_and_format_is_checked_before_size() {
        for t in all() {
            let ctx = EffectCx::whole(1.0);
            let p = ParamValues::new();
            let (a, b) = (frame_u8(4, 1), frame_u8(3, 2));
            let mut d = vec![SENTINEL_U8; 16];
            let err = t
                .run_cpu(
                    &ctx,
                    &p,
                    0.3,
                    ImageView::new(2, 2, PixelData::Rgba8(&a)).unwrap(),
                    ImageView::new(2, 2, PixelData::Rgba8(&a)).unwrap(),
                    ImageViewMut::new(1, 4, PixelDataMut::Rgba8(&mut d)).unwrap(),
                )
                .unwrap_err();
            assert_eq!(
                err,
                EffectError::SizeMismatch {
                    field: "dst",
                    expected: (2, 2),
                    actual: (1, 4)
                }
            );
            let err = t
                .run_cpu(
                    &ctx,
                    &p,
                    0.3,
                    ImageView::new(2, 2, PixelData::Rgba8(&a)).unwrap(),
                    ImageView::new(2, 1, PixelData::Rgba8(&b[..8])).unwrap(),
                    ImageViewMut::new(2, 2, PixelDataMut::Rgba8(&mut d)).unwrap(),
                )
                .unwrap_err();
            assert_eq!(
                err,
                EffectError::SizeMismatch {
                    field: "to",
                    expected: (2, 2),
                    actual: (2, 1)
                }
            );
            // Both wrong: the format error comes first.
            let f = frame_f32(2, 1.0);
            let err = t
                .run_cpu(
                    &ctx,
                    &p,
                    0.3,
                    ImageView::new(2, 2, PixelData::Rgba8(&a)).unwrap(),
                    ImageView::new(2, 1, PixelData::RgbaF32(&f)).unwrap(),
                    ImageViewMut::new(2, 2, PixelDataMut::Rgba8(&mut d)).unwrap(),
                )
                .unwrap_err();
            assert!(
                matches!(err, EffectError::FormatMismatch { field: "to", .. }),
                "{err:?}"
            );
            assert!(d.iter().all(|v| *v == SENTINEL_U8));
        }
    }

    // ARQ-110: a non-finite f32 pixel of `from` or `to` is Err naming the buffer and the element,
    // also with alpha 0 and at the ends; `dst` is not read, so its NaN is no error.
    #[test]
    fn arq_110_non_finite_pixels_are_err_and_write_nothing() {
        for t in all() {
            for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                for (which, field) in [(0, "from"), (1, "to")] {
                    for index in [0usize, 5, 7] {
                        for progress in [0.0f32, 0.3, 1.0] {
                            let mut f = frame_f32(2, 1.0);
                            let mut g = frame_f32(2, 2.0);
                            if which == 0 {
                                f[index] = bad
                            } else {
                                g[index] = bad
                            }
                            let mut d = sentinel_f32(8);
                            let err =
                                run_f32(t.as_ref(), progress, &f, &g, &mut d, (2, 1)).unwrap_err();
                            assert_eq!(
                                err,
                                EffectError::Buffer(BufferError::NonFinite { field, index }),
                                "{} {bad} {field}[{index}] p={progress}",
                                t.descriptor().id
                            );
                            assert!(bits(&d).iter().all(|b| *b == SENTINEL_F32));
                        }
                    }
                }
            }
            // The destination's old contents are never read.
            let (f, g) = (frame_f32(2, 1.0), frame_f32(2, 2.0));
            let mut d = vec![f32::NAN; 8];
            assert!(run_f32(t.as_ref(), 0.3, &f, &g, &mut d, (2, 1)).is_ok());
        }
    }

    // ARQ-118: an image of size 0 is Ok and writes nothing, but the checks still run first.
    #[test]
    fn arq_118_size_zero_is_ok_after_the_checks() {
        for t in all() {
            for size in [(0u32, 0u32), (0, 3), (3, 0)] {
                for progress in [0.0f32, 0.3, 1.0, 7.0] {
                    assert!(run_u8(t.as_ref(), progress, &[], &[], &mut [], size).is_ok());
                    assert!(run_f32(t.as_ref(), progress, &[], &[], &mut [], size).is_ok());
                }
                assert!(run_u8(t.as_ref(), f32::NAN, &[], &[], &mut [], size).is_err());
            }
            let err = t
                .run_cpu(
                    &EffectCx::whole(1.0),
                    &ParamValues::new(),
                    0.3,
                    ImageView::new(0, 3, PixelData::Rgba8(&[])).unwrap(),
                    ImageView::new(3, 0, PixelData::Rgba8(&[])).unwrap(),
                    ImageViewMut::new(0, 3, PixelDataMut::Rgba8(&mut [])).unwrap(),
                )
                .unwrap_err();
            assert!(matches!(err, EffectError::SizeMismatch { field: "to", .. }));
        }
    }

    // ARQ-110: ids the schema does not know are kept and never read, even with NaN.
    #[test]
    fn arq_110_unknown_parameter_ids_are_ignored() {
        let mut p = ParamValues::new();
        p.set("not_a_param", ParamValue::Float(f64::NAN));
        for t in all() {
            let (f, g) = (frame_u8(4, 1), frame_u8(4, 2));
            let run = |p: &ParamValues| {
                let mut d = vec![0u8; 16];
                t.run_cpu(
                    &EffectCx::whole(1.0),
                    p,
                    0.3,
                    ImageView::new(2, 2, PixelData::Rgba8(&f)).unwrap(),
                    ImageView::new(2, 2, PixelData::Rgba8(&g)).unwrap(),
                    ImageViewMut::new(2, 2, PixelDataMut::Rgba8(&mut d)).unwrap(),
                )
                .unwrap();
                d
            };
            assert_eq!(run(&p), run(&ParamValues::new()), "{}", t.descriptor().id);
        }
    }

    // ARQ-111: the same input twice gives the same bytes.
    #[test]
    fn arq_111_transitions_are_deterministic() {
        for t in all() {
            for progress in [0.1f32, 0.5, 0.77] {
                let (f, g) = (frame_u8(12, 1), frame_u8(12, 2));
                let (mut a, mut b) = (vec![0u8; 48], vec![1u8; 48]);
                run_u8(t.as_ref(), progress, &f, &g, &mut a, (4, 3)).unwrap();
                run_u8(t.as_ref(), progress, &f, &g, &mut b, (4, 3)).unwrap();
                assert_eq!(a, b);
                let (f, g) = (frame_f32(12, 1.0), frame_f32(12, 2.0));
                let (mut a, mut b) = (sentinel_f32(48), vec![0.0f32; 48]);
                run_f32(t.as_ref(), progress, &f, &g, &mut a, (4, 3)).unwrap();
                run_f32(t.as_ref(), progress, &f, &g, &mut b, (4, 3)).unwrap();
                assert_eq!(bits(&a), bits(&b));
            }
        }
    }

    // 02 §4.13: defaults of the trait.
    #[test]
    fn transition_defaults_have_no_gpu_variant_and_equal_power_audio() {
        for t in all() {
            assert!(t.gpu().is_none());
            assert_eq!(t.audio(), AudioCrossfade::EqualPower);
        }
    }

    // 02 §4.13 acceptance, C12: id, name key, handles and an empty schema that validates.
    #[test]
    fn cut_has_the_identity_and_schema_of_the_table() {
        let t = Cut::new();
        assert_eq!(t.descriptor().id, "transition.cut");
        assert_eq!(t.descriptor().name.0, "glossary.transition_cut");
        assert_eq!(t.descriptor().version, 1);
        assert_eq!(t.handles(), HandleReq::None);
        assert!(t.params().validate_definition().is_ok());
        assert!(t.params().params().is_empty());
    }

    // ARQ-118: Cut gives `from` below 0.5 and `to` from 0.5, f32 and 8 bits.
    #[test]
    fn arq_118_cut_switches_at_exactly_one_half() {
        let t = Cut::new();
        let (fu, tu) = (frame_u8(2, 1), frame_u8(2, 77));
        let (ff, tf) = (frame_f32(2, 1.0), frame_f32(2, 4.0));
        let below = f32::from_bits(0.5f32.to_bits() - 1);
        let above = f32::from_bits(0.5f32.to_bits() + 1);
        let cases = [
            (0.4999f32, false),
            (below, false),
            (0.25, false),
            (0.5, true),
            (above, true),
            (0.5001, true),
            (0.75, true),
            (0.000_001, false),
            (0.999_999, true),
        ];
        for (p, is_to) in cases {
            let mut du = vec![SENTINEL_U8; 8];
            run_u8(&t, p, &fu, &tu, &mut du, (2, 1)).unwrap();
            assert_eq!(du, if is_to { tu.clone() } else { fu.clone() }, "u8 p={p}");
            let mut df = sentinel_f32(8);
            run_f32(&t, p, &ff, &tf, &mut df, (2, 1)).unwrap();
            assert_eq!(bits(&df), bits(if is_to { &tf } else { &ff }), "f32 p={p}");
        }
    }

    // ---- Dissolve and Fade -------------------------------------------------------------

    use gafira_color::{Color, DisplayP3Rgba, LinearRgba, Rgba};
    use gafira_doc::contract::{ParamError, ParamKind};

    use super::mix::{MIX_SPACE, MixSpace};

    fn run_with(
        t: &dyn Transition,
        p: &ParamValues,
        progress: f32,
        from: [u8; 4],
        to: [u8; 4],
    ) -> [u8; 4] {
        let (f, g) = (from.to_vec(), to.to_vec());
        let mut d = vec![SENTINEL_U8; 4];
        t.run_cpu(
            &EffectCx::whole(1.0),
            p,
            progress,
            ImageView::new(1, 1, PixelData::Rgba8(&f)).unwrap(),
            ImageView::new(1, 1, PixelData::Rgba8(&g)).unwrap(),
            ImageViewMut::new(1, 1, PixelDataMut::Rgba8(&mut d)).unwrap(),
        )
        .unwrap();
        [d[0], d[1], d[2], d[3]]
    }

    fn run_with_f32(
        t: &dyn Transition,
        p: &ParamValues,
        progress: f32,
        from: [f32; 4],
        to: [f32; 4],
    ) -> [f32; 4] {
        let (f, g) = (from.to_vec(), to.to_vec());
        let mut d = vec![f32::from_bits(SENTINEL_F32); 4];
        t.run_cpu(
            &EffectCx::whole(1.0),
            p,
            progress,
            ImageView::new(1, 1, PixelData::RgbaF32(&f)).unwrap(),
            ImageView::new(1, 1, PixelData::RgbaF32(&g)).unwrap(),
            ImageViewMut::new(1, 1, PixelDataMut::RgbaF32(&mut d)).unwrap(),
        )
        .unwrap();
        [d[0], d[1], d[2], d[3]]
    }

    fn near(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    // 02 §4.13 acceptance, C12: Dissolve has the id, name, handles and empty schema of the table.
    #[test]
    fn dissolve_has_the_identity_and_schema_of_the_table() {
        let t = Dissolve::new();
        assert_eq!(t.descriptor().id, "transition.dissolve");
        assert_eq!(t.descriptor().name.0, "glossary.transition_dissolve");
        assert_eq!(t.descriptor().version, 1);
        assert_eq!(t.handles(), HandleReq::Both);
        assert!(t.params().validate_definition().is_ok());
        assert!(t.params().params().is_empty());
    }

    // ARQ-118: Dissolve at 0.25 over 0 and 1 gives 0.25, in every channel (f32).
    #[test]
    fn arq_118_dissolve_at_a_quarter_over_zero_and_one_is_a_quarter() {
        let t = Dissolve::new();
        let none = ParamValues::new();
        let got = run_with_f32(&t, &none, 0.25, [0.0; 4], [1.0; 4]);
        assert!(got.iter().all(|v| near(*v, 0.25, 1e-7)), "{got:?}");
        // Weights are not swapped: 0.8 * 0.75 + 0.4 * 0.25.
        let got = run_with_f32(&t, &none, 0.25, [0.8; 4], [0.4; 4]);
        assert!(got.iter().all(|v| near(*v, 0.7, 1e-6)), "{got:?}");
        // Out-of-range values are admitted and stay finite.
        let got = run_with_f32(&t, &none, 0.5, [2.0, -1.0, 0.0, 1.0], [0.0, 1.0, 0.0, 1.0]);
        assert_eq!(got, [1.0, 0.0, 0.0, 1.0]);
    }

    // P15 item 8 and 9: 8 bits black to white; the rows depend on MIX_SPACE (O5).
    #[test]
    fn arq_118_dissolve_8_bit_rows_follow_the_mix_space() {
        let t = Dissolve::new();
        let none = ParamValues::new();
        let (black, white) = ([0u8, 0, 0, 255], [255u8, 255, 255, 255]);
        let (q1, q3) = match MIX_SPACE {
            MixSpace::Linear => (137, 225),
            MixSpace::Encoded | MixSpace::AsStored => (64, 191),
        };
        assert_eq!(run_with(&t, &none, 0.25, black, white), [q1, q1, q1, 255]);
        assert_eq!(run_with(&t, &none, 0.75, black, white), [q3, q3, q3, 255]);
        // Alpha mixes premultiplied: transparent to opaque white at one half.
        assert_eq!(
            run_with(&t, &none, 0.5, [0; 4], white),
            [128, 128, 128, 128]
        );
        // A transparent pixel with colour adds none.
        assert_eq!(
            run_with(&t, &none, 0.5, [200, 10, 5, 0], black),
            [0, 0, 0, 128]
        );
    }

    // ARQ-118: close to the ends the result is within 1/255 of the end.
    #[test]
    fn arq_118_dissolve_and_fade_stay_within_1_255_of_the_ends_near_them() {
        let none = ParamValues::new();
        let (f8, t8) = ([200u8, 100, 50, 255], [10u8, 220, 90, 255]);
        let (ff, tf) = ([0.7f32, 0.2, 0.1, 1.0], [0.1f32, 0.9, 0.4, 1.0]);
        for t in [&Dissolve::new() as &dyn Transition, &Fade::new()] {
            for p in [0.0001f32, 0.9999] {
                let (end8, endf) = if p < 0.5 { (f8, ff) } else { (t8, tf) };
                let got = run_with(t, &none, p, f8, t8);
                let gotf = run_with_f32(t, &none, p, ff, tf);
                for k in 0..4 {
                    assert!(
                        got[k].abs_diff(end8[k]) <= 1,
                        "{} p={p} {got:?}",
                        t.descriptor().id
                    );
                    assert!(
                        near(gotf[k], endf[k], 1.0 / 255.0),
                        "{} p={p} {gotf:?}",
                        t.descriptor().id
                    );
                }
            }
        }
    }

    // ARQ-117: mixing premultiplied 8-bit pixels keeps c <= a.
    mod mix_property {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn dissolve_and_fade_keep_premultiplied_pixels_valid(
                c1 in prop::array::uniform3(any::<u8>()), a1 in any::<u8>(),
                c2 in prop::array::uniform3(any::<u8>()), a2 in any::<u8>(),
                p in 0.0f32..=1.0,
            ) {
                let px = |c: [u8; 3], a: u8| [
                    crate::premultiply_u8(c[0], a), crate::premultiply_u8(c[1], a),
                    crate::premultiply_u8(c[2], a), a,
                ];
                for t in [&Dissolve::new() as &dyn Transition, &Fade::new()] {
                    let out = run_with(t, &ParamValues::new(), p, px(c1, a1), px(c2, a2));
                    prop_assert!(out[..3].iter().all(|c| *c <= out[3]), "{} {out:?}", t.descriptor().id);
                }
            }
        }
    }

    // ---- Fade only ---------------------------------------------------------------------

    fn colour(c: Color) -> ParamValues {
        let mut p = ParamValues::new();
        p.set("color", ParamValue::Color(c));
        p
    }

    // 02 §4.13 acceptance, C12: Fade has the id, name, handles and the one `color` parameter.
    #[test]
    fn fade_has_the_identity_and_schema_of_the_table() {
        let t = Fade::new();
        assert_eq!(t.descriptor().id, "transition.fade");
        assert_eq!(t.descriptor().name.0, "glossary.transition_fade");
        assert_eq!(t.descriptor().version, 1);
        assert_eq!(t.handles(), HandleReq::Both);
        assert!(t.params().validate_definition().is_ok());
        let defs = t.params().params();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].id, "color");
        assert_eq!(defs[0].label.0, "transition.fade.color");
        assert!(!defs[0].animatable);
        assert_eq!(
            defs[0].kind,
            ParamKind::Color {
                default: Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0))
            }
        );
    }

    // ARQ-110: a bad `color` is Err naming it, before anything is written, also at the ends.
    #[test]
    fn arq_110_fade_rejects_a_bad_colour_before_writing() {
        let t = Fade::new();
        let (f, g) = (frame_u8(1, 1), frame_u8(1, 2));
        let cases: [(ParamValue, ParamError); 6] = [
            (
                ParamValue::Color(Color::Srgb(Rgba::new(f32::NAN, 0.0, 0.0, 1.0))),
                ParamError::NonFinite { param: "color" },
            ),
            (
                ParamValue::Color(Color::Srgb(Rgba::new(0.0, f32::INFINITY, 0.0, 1.0))),
                ParamError::NonFinite { param: "color" },
            ),
            (
                ParamValue::Color(Color::LinearSrgb(LinearRgba::new(
                    0.0,
                    0.0,
                    f32::NEG_INFINITY,
                    1.0,
                ))),
                ParamError::NonFinite { param: "color" },
            ),
            (
                ParamValue::Color(Color::DisplayP3(DisplayP3Rgba::new(
                    0.0,
                    0.0,
                    0.0,
                    f32::NAN,
                ))),
                ParamError::NonFinite { param: "color" },
            ),
            (
                ParamValue::Float(0.5),
                ParamError::WrongType { param: "color" },
            ),
            (
                ParamValue::Choice("black".into()),
                ParamError::WrongType { param: "color" },
            ),
        ];
        for (value, want) in cases {
            for progress in [0.0f32, 0.3, 1.0] {
                let mut p = ParamValues::new();
                p.set("color", value.clone());
                let mut d = vec![SENTINEL_U8; 4];
                let err = t
                    .run_cpu(
                        &EffectCx::whole(1.0),
                        &p,
                        progress,
                        ImageView::new(1, 1, PixelData::Rgba8(&f)).unwrap(),
                        ImageView::new(1, 1, PixelData::Rgba8(&g)).unwrap(),
                        ImageViewMut::new(1, 1, PixelDataMut::Rgba8(&mut d)).unwrap(),
                    )
                    .unwrap_err();
                assert_eq!(err, EffectError::Param(want), "{value:?} p={progress}");
                assert!(d.iter().all(|b| *b == SENTINEL_U8));
            }
        }
    }

    // ARQ-118: Fade at 0.5 gives the colour exactly; white gives 255 and 1.0 in every channel.
    #[test]
    fn arq_118_fade_at_one_half_is_the_colour_exactly() {
        let t = Fade::new();
        let white = colour(Color::Srgb(Rgba::new(1.0, 1.0, 1.0, 1.0)));
        assert_eq!(
            run_with(&t, &white, 0.5, [9, 8, 7, 255], [1, 2, 3, 255]),
            [255; 4]
        );
        assert_eq!(run_with_f32(&t, &white, 0.5, [0.3; 4], [0.6; 4]), [1.0; 4]);
        // The default is opaque black.
        let none = ParamValues::new();
        assert_eq!(
            run_with(&t, &none, 0.5, [9, 8, 7, 255], [1, 2, 3, 255]),
            [0, 0, 0, 255]
        );
        assert_eq!(
            run_with_f32(&t, &none, 0.5, [0.3; 4], [0.6; 4]),
            [0.0, 0.0, 0.0, 1.0]
        );
        // A translucent colour keeps its own alpha: sRGB bytes (204, 102, 51, 128), premultiplied.
        let c = colour(Color::Srgb(Rgba::new(0.8, 0.4, 0.2, 0.5)));
        assert_eq!(
            run_with(&t, &c, 0.5, [9, 8, 7, 255], [1, 2, 3, 255]),
            [102, 51, 26, 128]
        );
        let got = run_with_f32(&t, &c, 0.5, [0.3; 4], [0.6; 4]);
        for (g, w) in got.iter().zip([0.301_9, 0.066_4, 0.016_5, 0.5]) {
            assert!(near(*g, w, 1e-3), "{got:?}");
        }
        // Linear light and wide gamut are converted once: linear 0.5 is sRGB byte 188.
        let lin = colour(Color::LinearSrgb(LinearRgba::new(0.5, 0.5, 0.5, 1.0)));
        assert_eq!(
            run_with(&t, &lin, 0.5, [0; 4], [0; 4]),
            [188, 188, 188, 255]
        );
        assert_eq!(
            run_with_f32(&t, &lin, 0.5, [0.0; 4], [0.0; 4]),
            [0.5, 0.5, 0.5, 1.0]
        );
        // Display-P3 red is outside sRGB: bytes clamp, f32 keeps the out-of-gamut values.
        let p3 = colour(Color::DisplayP3(DisplayP3Rgba::new(1.0, 0.0, 0.0, 1.0)));
        assert_eq!(run_with(&t, &p3, 0.5, [0; 4], [0; 4]), [255, 0, 0, 255]);
        let got = run_with_f32(&t, &p3, 0.5, [0.0; 4], [0.0; 4]);
        assert!(
            near(got[0], 1.2249, 1e-3) && got[1] < 0.0 && got[2] < 0.0 && got[3] == 1.0,
            "{got:?}"
        );
        // An alpha outside [0, 1] is clipped.
        let big = colour(Color::Srgb(Rgba::new(1.0, 1.0, 1.0, 2.0)));
        assert_eq!(run_with(&t, &big, 0.5, [0; 4], [0; 4]), [255; 4]);
        let neg = colour(Color::Srgb(Rgba::new(1.0, 1.0, 1.0, -0.5)));
        assert_eq!(run_with(&t, &neg, 0.5, [9; 4], [9; 4]), [0; 4]);
    }

    // ARQ-118: below one half `from` goes to the colour with 2p; from one half the colour goes
    // to `to` with 2p - 1 (f32, black pictures, white colour, so the mix is the weight).
    #[test]
    fn arq_118_fade_halves_use_2p_and_2p_minus_1() {
        let t = Fade::new();
        let white = colour(Color::Srgb(Rgba::new(1.0, 1.0, 1.0, 1.0)));
        let black = [0.0f32, 0.0, 0.0, 1.0];
        for (p, want) in [
            (0.125f32, 0.25f32),
            (0.25, 0.5),
            (0.375, 0.75),
            (0.625, 0.75),
            (0.75, 0.5),
            (0.875, 0.25),
        ] {
            let got = run_with_f32(&t, &white, p, black, black);
            assert!(near(got[0], want, 1e-6) && got[3] == 1.0, "p={p}: {got:?}");
        }
        // The halves are independent: `from` fades out, `to` fades in.
        let got = run_with_f32(&t, &white, 0.25, [0.6, 0.6, 0.6, 1.0], [0.0, 0.0, 0.0, 1.0]);
        assert!(near(got[0], 0.5 * 0.6 + 0.5, 1e-6), "{got:?}");
        let got = run_with_f32(&t, &white, 0.75, [0.0, 0.0, 0.0, 1.0], [0.6, 0.6, 0.6, 1.0]);
        assert!(near(got[0], 0.5 * 0.6 + 0.5, 1e-6), "{got:?}");
        // 8 bits, white colour over the picture: a constant picture of the colour stays put.
        let c = colour(Color::Srgb(Rgba::new(0.2, 0.4, 0.6, 1.0)));
        let px = [51u8, 102, 153, 255];
        for p in [0.1f32, 0.3, 0.45, 0.55, 0.7, 0.9] {
            assert_eq!(run_with(&t, &c, p, px, px), px, "p={p}");
        }
    }

    // ---- Fix round: P15 step 1 (cx.scale), the order of the steps, Fade colour overflow -------

    fn run_cx(
        t: &dyn Transition,
        cx: &EffectCx,
        progress: f32,
        frames: (&[u8], &[u8], &mut [u8]),
        size: (u32, u32),
    ) -> Result<(), EffectError> {
        let (from, to, dst) = frames;
        t.run_cpu(
            cx,
            &ParamValues::new(),
            progress,
            ImageView::new(size.0, size.1, PixelData::Rgba8(from)).unwrap(),
            ImageView::new(size.0, size.1, PixelData::Rgba8(to)).unwrap(),
            ImageViewMut::new(size.0, size.1, PixelDataMut::Rgba8(dst)).unwrap(),
        )
    }

    // ARQ-110, P15 step 1: a non-finite `cx.scale` is Err naming `cx.scale`, first of all, even
    // though no transition reads it; a finite scale of any sign or size is no error.
    #[test]
    fn arq_110_non_finite_cx_scale_is_err_first_and_writes_nothing() {
        let want = EffectError::Buffer(BufferError::NonFinite {
            field: "cx.scale",
            index: 0,
        });
        for t in all() {
            for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                let (f, g) = (frame_u8(4, 1), frame_u8(4, 2));
                for progress in [0.0f32, 0.3, 1.0, f32::NAN] {
                    let mut d = vec![SENTINEL_U8; 16];
                    let err = run_cx(
                        t.as_ref(),
                        &EffectCx::whole(bad),
                        progress,
                        (&f, &g, &mut d),
                        (2, 2),
                    )
                    .unwrap_err();
                    assert_eq!(err, want, "{} {bad} p={progress}", t.descriptor().id);
                    assert!(d.iter().all(|b| *b == SENTINEL_U8));
                }
                // Before every later fault at once: sizes, formats and a bad pixel.
                let mut d = vec![SENTINEL_U8; 16];
                let err = t
                    .run_cpu(
                        &EffectCx::whole(bad),
                        &ParamValues::new(),
                        f32::NAN,
                        ImageView::new(2, 2, PixelData::Rgba8(&f)).unwrap(),
                        ImageView::new(1, 1, PixelData::RgbaF32(&[f32::NAN; 4])).unwrap(),
                        ImageViewMut::new(2, 2, PixelDataMut::Rgba8(&mut d)).unwrap(),
                    )
                    .unwrap_err();
                assert_eq!(err, want);
            }
            for ok in [0.0f32, -1.0, 1.0e30, f32::MIN_POSITIVE, -0.0] {
                let (f, g) = (frame_u8(4, 1), frame_u8(4, 2));
                let mut d = vec![0u8; 16];
                assert!(
                    run_cx(
                        t.as_ref(),
                        &EffectCx::whole(ok),
                        0.3,
                        (&f, &g, &mut d),
                        (2, 2)
                    )
                    .is_ok()
                );
            }
        }
    }

    // ARQ-110, P15: a huge finite colour that overflows in conversion is Err naming `color`,
    // nothing is written, and no inf or NaN reaches `dst` (f32 and bytes, any `progress`).
    #[test]
    fn arq_110_fade_colour_that_overflows_is_err_and_writes_nothing() {
        let t = Fade::new();
        let colours = [
            Color::Srgb(Rgba::new(1.0e30, 0.0, 0.0, 1.0)),
            Color::Srgb(Rgba::new(1.0e30, 0.0, 0.0, 0.0)),
            Color::Srgb(Rgba::new(-1.0e30, 0.0, 0.0, 1.0)),
            Color::DisplayP3(DisplayP3Rgba::new(f32::MAX, -f32::MAX, f32::MAX, 1.0)),
            Color::LinearSrgb(LinearRgba::new(0.0, f32::MAX, 0.0, 1.0)),
        ];
        for c in colours {
            for progress in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
                let p = colour(c);
                let (f, g) = (frame_f32(1, 1.0), frame_f32(1, 2.0));
                let mut d = sentinel_f32(4);
                let r = t.run_cpu(
                    &EffectCx::whole(1.0),
                    &p,
                    progress,
                    ImageView::new(1, 1, PixelData::RgbaF32(&f)).unwrap(),
                    ImageView::new(1, 1, PixelData::RgbaF32(&g)).unwrap(),
                    ImageViewMut::new(1, 1, PixelDataMut::RgbaF32(&mut d)).unwrap(),
                );
                let out_ok = d
                    .iter()
                    .all(|v| v.is_finite() || v.to_bits() == SENTINEL_F32);
                assert!(out_ok, "{c:?} p={progress}: non-finite in dst {d:?}");
                // The linear conversion of these colours overflows or not; if it does, it is Err.
                match r {
                    Ok(()) => assert!(d.iter().all(|v| v.is_finite()), "{c:?} p={progress}"),
                    Err(e) => {
                        assert!(
                            matches!(
                                e,
                                EffectError::Buffer(BufferError::NonFinite { field: "color", .. })
                            ),
                            "{e:?}"
                        );
                        assert!(
                            bits(&d).iter().all(|b| *b == SENTINEL_F32),
                            "written before Err"
                        );
                    }
                }
            }
        }
        // The case the verify report measured: 1e30 red in sRGB is Err at 0.25 and at 0.5.
        for progress in [0.25f32, 0.5] {
            let (f, g) = (frame_f32(1, 1.0), frame_f32(1, 2.0));
            let mut d = sentinel_f32(4);
            let err = t
                .run_cpu(
                    &EffectCx::whole(1.0),
                    &colour(Color::Srgb(Rgba::new(1.0e30, 0.0, 0.0, 1.0))),
                    progress,
                    ImageView::new(1, 1, PixelData::RgbaF32(&f)).unwrap(),
                    ImageView::new(1, 1, PixelData::RgbaF32(&g)).unwrap(),
                    ImageViewMut::new(1, 1, PixelDataMut::RgbaF32(&mut d)).unwrap(),
                )
                .unwrap_err();
            assert_eq!(
                err,
                EffectError::Buffer(BufferError::NonFinite {
                    field: "color",
                    index: 0
                })
            );
        }
    }

    // P15: the steps run in this order, so with two faults the earlier one is reported:
    // cx.scale, progress, parameters, formats, sizes, pixels.
    #[test]
    fn p15_order_of_the_checks_with_two_faults_at_once() {
        let f = Fade::new();
        let bad_colour = colour(Color::Srgb(Rgba::new(f32::NAN, 0.0, 0.0, 1.0)));
        let ctx = EffectCx::whole(1.0);
        let (a, b6) = (frame_u8(4, 1), frame_u8(6, 2));
        let (rf4, rf6) = (frame_f32(4, 1.0), frame_f32(6, 2.0));
        let mut d = vec![SENTINEL_U8; 16];
        let progress_err = EffectError::Buffer(BufferError::NonFinite {
            field: "progress",
            index: 0,
        });
        let param_err = EffectError::Param(ParamError::NonFinite { param: "color" });
        // 2 before 3: a bad progress and a bad colour.
        let err = f
            .run_cpu(
                &ctx,
                &bad_colour,
                f32::NAN,
                ImageView::new(2, 2, PixelData::Rgba8(&a)).unwrap(),
                ImageView::new(2, 2, PixelData::Rgba8(&a)).unwrap(),
                ImageViewMut::new(2, 2, PixelDataMut::Rgba8(&mut d)).unwrap(),
            )
            .unwrap_err();
        assert_eq!(err, progress_err);
        // 3 before 4 and 5: a bad colour with an Alpha8 pair, and with mismatched sizes.
        let a8 = [1u8, 2, 3, 4];
        let err = f
            .run_cpu(
                &ctx,
                &bad_colour,
                0.3,
                ImageView::new(2, 2, PixelData::Alpha8(&a8)).unwrap(),
                ImageView::new(2, 2, PixelData::Alpha8(&a8)).unwrap(),
                ImageViewMut::new(2, 2, PixelDataMut::Alpha8(&mut [0u8; 4])).unwrap(),
            )
            .unwrap_err();
        assert_eq!(err, param_err);
        for progress in [0.0f32, 0.3, 1.0] {
            let err = f
                .run_cpu(
                    &ctx,
                    &bad_colour,
                    progress,
                    ImageView::new(2, 2, PixelData::Rgba8(&a)).unwrap(),
                    ImageView::new(3, 2, PixelData::Rgba8(&b6)).unwrap(),
                    ImageViewMut::new(2, 2, PixelDataMut::Rgba8(&mut d)).unwrap(),
                )
                .unwrap_err();
            assert_eq!(err, param_err, "p={progress}");
        }
        // 5 before 6: mismatched sizes with a NaN pixel in `from` (and in `to`).
        for t in all() {
            let mut bad = rf4.clone();
            bad[0] = f32::NAN;
            let mut df = sentinel_f32(16);
            let err = t
                .run_cpu(
                    &ctx,
                    &ParamValues::new(),
                    0.3,
                    ImageView::new(2, 2, PixelData::RgbaF32(&bad)).unwrap(),
                    ImageView::new(3, 2, PixelData::RgbaF32(&rf6)).unwrap(),
                    ImageViewMut::new(2, 2, PixelDataMut::RgbaF32(&mut df)).unwrap(),
                )
                .unwrap_err();
            assert!(
                matches!(err, EffectError::SizeMismatch { field: "to", .. }),
                "{} {err:?}",
                t.descriptor().id
            );
            let err = t
                .run_cpu(
                    &ctx,
                    &ParamValues::new(),
                    0.3,
                    ImageView::new(2, 2, PixelData::RgbaF32(&rf4)).unwrap(),
                    ImageView::new(2, 2, PixelData::RgbaF32(&bad)).unwrap(),
                    ImageViewMut::new(2, 3, PixelDataMut::RgbaF32(&mut sentinel_f32(24))).unwrap(),
                )
                .unwrap_err();
            assert!(
                matches!(err, EffectError::SizeMismatch { field: "dst", .. }),
                "{err:?}"
            );
        }
        assert!(d.iter().all(|v| *v == SENTINEL_U8));
    }

    // ARQ-118: Fade through the public API just below and just above one half: `2p` and `2p - 1`
    // still apply, so the result is not the colour yet and is not extrapolated.
    #[test]
    fn arq_118_fade_boundary_is_exactly_one_half() {
        let t = Fade::new();
        let none = ParamValues::new(); // opaque black
        let one = [1.0f32; 4];
        let zero_rgb = [0.0f32, 0.0, 0.0, 1.0];
        // Below: from 1 fades out with t = 2p; to is not involved.
        for p in [0.4999f32, 0.499_95, f32::from_bits(0.5f32.to_bits() - 1)] {
            let got = run_with_f32(&t, &none, p, one, zero_rgb);
            let want = (1.0 - 2.0 * f64::from(p)) as f32;
            assert!(near(got[0], want, 1e-6), "p={p}: {got:?} want {want}");
        }
        // Above: `to` 1 fades in with t = 2p - 1; from is not involved.
        for p in [0.500_05f32, 0.5001, f32::from_bits(0.5f32.to_bits() + 1)] {
            let got = run_with_f32(&t, &none, p, zero_rgb, one);
            let want = (2.0 * f64::from(p) - 1.0) as f32;
            assert!(near(got[0], want, 1e-6), "p={p}: {got:?} want {want}");
        }
        // And in bytes, away from the colour by more than a step on both sides.
        let black = [0u8, 0, 0, 255];
        let white = [255u8, 255, 255, 255];
        let got = run_with(&t, &none, 0.4999, white, black);
        assert!(got[3] == 255);
        assert_eq!(run_with(&t, &none, 0.5, white, white), black);
    }

    // ARQ-118 deviation 2: the colour's own alpha is clipped to [0, 1] in f32 as well as in bytes.
    #[test]
    fn arq_118_fade_colour_alpha_is_clipped_in_f32() {
        let t = Fade::new();
        let big = colour(Color::Srgb(Rgba::new(1.0, 0.5, 0.0, 2.0)));
        let one = colour(Color::Srgb(Rgba::new(1.0, 0.5, 0.0, 1.0)));
        let a = run_with_f32(&t, &big, 0.5, [0.3; 4], [0.6; 4]);
        let b = run_with_f32(&t, &one, 0.5, [0.3; 4], [0.6; 4]);
        assert_eq!(bits(&a), bits(&b));
        assert_eq!(a[3], 1.0);
        let neg = colour(Color::Srgb(Rgba::new(1.0, 0.5, 0.0, -0.5)));
        assert_eq!(run_with_f32(&t, &neg, 0.5, [0.3; 4], [0.6; 4]), [0.0; 4]);
        let tiny = colour(Color::Srgb(Rgba::new(1.0, 0.5, 0.0, 1.5e-3)));
        let got = run_with_f32(&t, &tiny, 0.5, [0.3; 4], [0.6; 4]);
        assert!(near(got[3], 1.5e-3, 1e-9), "{got:?}");
    }

    // ARQ-110: the same overflow in bytes is Err naming `color`, not a silent 0 or 255.
    #[test]
    fn arq_110_fade_colour_that_overflows_is_err_in_bytes_too() {
        let t = Fade::new();
        let c = Color::DisplayP3(DisplayP3Rgba::new(f32::MAX, -f32::MAX, f32::MAX, 1.0));
        let l = c.convert(gafira_color::ColorSpaceId::Srgb);
        let Color::Srgb(s) = l else { panic!("space") };
        assert!(
            !s.r.is_finite() || !s.g.is_finite() || !s.b.is_finite(),
            "the case must overflow: {s:?}"
        );
        for progress in [0.25f32, 0.5, 0.75] {
            let (f, g) = (frame_u8(1, 1), frame_u8(1, 2));
            let mut d = vec![SENTINEL_U8; 4];
            let err = t
                .run_cpu(
                    &EffectCx::whole(1.0),
                    &colour(c),
                    progress,
                    ImageView::new(1, 1, PixelData::Rgba8(&f)).unwrap(),
                    ImageView::new(1, 1, PixelData::Rgba8(&g)).unwrap(),
                    ImageViewMut::new(1, 1, PixelDataMut::Rgba8(&mut d)).unwrap(),
                )
                .unwrap_err();
            assert!(
                matches!(
                    err,
                    EffectError::Buffer(BufferError::NonFinite { field: "color", .. })
                ),
                "{err:?}"
            );
            assert!(d.iter().all(|b| *b == SENTINEL_U8));
        }
    }

    // ---- Fix round (RS3b verify round 1): every configuration, not only the defaults ----------

    /// One transition with one set of parameters.
    type Config = (Box<dyn Transition>, ParamValues, String);

    fn choice(id: &str, value: &str) -> ParamValues {
        let mut p = ParamValues::new();
        p.set(id, ParamValue::Choice(value.to_owned()));
        p
    }

    /// Every direction of Slide, Wipe (hard, soft, softest) and Zoom, and Fade in a colour.
    fn configs() -> Vec<Config> {
        let mut out: Vec<Config> = Vec::new();
        for d in ["left", "right", "up", "down"] {
            out.push((
                Box::new(Slide::new()),
                choice("direction", d),
                format!("slide {d}"),
            ));
        }
        for d in ["right", "left", "down", "up", "circular"] {
            for s in [0.0, 0.4, 1.0, 1.0e-16] {
                let mut p = choice("direction", d);
                p.set("softness", ParamValue::Float(s));
                out.push((Box::new(Wipe::new()), p, format!("wipe {d} s={s}")));
            }
        }
        for d in ["in", "out"] {
            out.push((
                Box::new(Zoom::new()),
                choice("direction", d),
                format!("zoom {d}"),
            ));
        }
        out.push((Box::new(Cut::new()), ParamValues::new(), "cut".into()));
        out.push((
            Box::new(Dissolve::new()),
            ParamValues::new(),
            "dissolve".into(),
        ));
        out.push((
            Box::new(Fade::new()),
            colour(Color::Srgb(Rgba::new(0.9, 0.2, 0.1, 0.6))),
            "fade".into(),
        ));
        out
    }

    fn run_config_u8(
        c: &Config,
        progress: f32,
        from: &[u8],
        to: &[u8],
        size: (u32, u32),
    ) -> Vec<u8> {
        let mut d = vec![SENTINEL_U8; from.len()];
        c.0.run_cpu(
            &EffectCx::whole(1.0),
            &c.1,
            progress,
            ImageView::new(size.0, size.1, PixelData::Rgba8(from)).unwrap(),
            ImageView::new(size.0, size.1, PixelData::Rgba8(to)).unwrap(),
            ImageViewMut::new(size.0, size.1, PixelDataMut::Rgba8(&mut d)).unwrap(),
        )
        .unwrap();
        d
    }

    fn run_config_f32(
        c: &Config,
        progress: f32,
        from: &[f32],
        to: &[f32],
        size: (u32, u32),
    ) -> Result<Vec<f32>, EffectError> {
        let mut d = sentinel_f32(from.len());
        c.0.run_cpu(
            &EffectCx::whole(1.0),
            &c.1,
            progress,
            ImageView::new(size.0, size.1, PixelData::RgbaF32(from)).unwrap(),
            ImageView::new(size.0, size.1, PixelData::RgbaF32(to)).unwrap(),
            ImageViewMut::new(size.0, size.1, PixelDataMut::RgbaF32(&mut d)).unwrap(),
        )?;
        Ok(d)
    }

    // ARQ-111: the same input twice gives the same bytes, for every direction and softness.
    #[test]
    fn arq_111_every_configuration_is_deterministic() {
        for c in configs() {
            for progress in [0.07f32, 0.5, 0.93] {
                let (f, g) = (frame_u8(35, 1), frame_u8(35, 2));
                let a = run_config_u8(&c, progress, &f, &g, (7, 5));
                let b = run_config_u8(&c, progress, &f, &g, (7, 5));
                assert_eq!(a, b, "{} p={progress}", c.2);
                let (f, g) = (frame_f32(35, 1.0), frame_f32(35, 2.0));
                let a = run_config_f32(&c, progress, &f, &g, (7, 5)).unwrap();
                let b = run_config_f32(&c, progress, &f, &g, (7, 5)).unwrap();
                assert_eq!(bits(&a), bits(&b), "{} p={progress}", c.2);
            }
        }
    }

    mod config_property {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            // ARQ-117: premultiplied 8-bit pixels stay premultiplied (c <= a) in every configuration.
            #[test]
            fn every_configuration_keeps_premultiplied_pixels_valid(
                seed in any::<u32>(), p in 0.0f32..=1.0, w in 1u32..6, h in 1u32..6,
            ) {
                let n = (w * h) as usize;
                let frame = |k: u32| -> Vec<u8> {
                    let mut s = seed ^ k;
                    (0..n).flat_map(|_| {
                        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                        let a = (s >> 24) as u8;
                        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                        let c = |sh: u32| crate::premultiply_u8((s >> sh) as u8, a);
                        [c(0), c(8), c(16), a]
                    }).collect()
                };
                let (f, g) = (frame(1), frame(2));
                for c in configs() {
                    let out = run_config_u8(&c, p, &f, &g, (w, h));
                    for px in out.chunks_exact(4) {
                        prop_assert!(px[..3].iter().all(|v| *v <= px[3]), "{} {px:?}", c.2);
                    }
                }
            }

            // ARQ-110: finite input in the domain gives finite output through `run_cpu`, in every
            // configuration, for large, tiny and mixed-sign pixels.
            #[test]
            fn every_configuration_gives_finite_output_for_finite_input(
                a in prop::collection::vec(prop_oneof![-1.0e30f32..1.0e30, -1.0e-30f32..1.0e-30, -2.0f32..2.0], 12),
                b in prop::collection::vec(prop_oneof![-1.0e30f32..1.0e30, -1.0e-30f32..1.0e-30, -2.0f32..2.0], 12),
                p in 0.0f32..=1.0,
            ) {
                for c in configs() {
                    let out = run_config_f32(&c, p, &a, &b, (3, 1)).unwrap();
                    prop_assert!(out.iter().all(|v| v.is_finite()), "{} {out:?}", c.2);
                }
            }
        }
    }
}
