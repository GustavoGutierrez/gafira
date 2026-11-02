//! Transitions between two frames (02 §4.13, ARQ-118).

mod cut;
mod guard;

use gafira_doc::contract::{AudioCrossfade, Describe, ParamSchema, ParamValues};

pub use cut::Cut;

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
        vec![Box::new(Cut::new())]
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
            &EffectCx { scale: 1.0 },
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
            &EffectCx { scale: 1.0 },
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

    // ARQ-25, ARQ-118: p = 0 copies `from` and p = 1 copies `to`, byte for byte, for every
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
                        &EffectCx { scale: 1.0 },
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
            let ctx = EffectCx { scale: 1.0 };
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
            let ctx = EffectCx { scale: 1.0 };
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
            let ctx = EffectCx { scale: 1.0 };
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
                    &EffectCx { scale: 1.0 },
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
                    &EffectCx { scale: 1.0 },
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
}
