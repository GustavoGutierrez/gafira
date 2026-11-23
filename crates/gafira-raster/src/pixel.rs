//! Premultiplied alpha (ARQ-117).

use crate::{BufferError, ImageViewMut, PixelDataMut};

/// Premultiplies one 8-bit channel: `(c × a + 127) / 255`, integer division.
#[must_use]
pub fn premultiply_u8(c: u8, a: u8) -> u8 {
    let v = (u32::from(c) * u32::from(a) + 127) / 255;
    u8::try_from(v).unwrap_or(u8::MAX)
}

/// Reverses [`premultiply_u8`]. `a = 0` gives 0; otherwise
/// `min(255, round(c × 255 / a))`.
#[must_use]
pub fn unpremultiply_u8(c: u8, a: u8) -> u8 {
    if a == 0 {
        return 0;
    }
    let a = u32::from(a);
    // round(c * 255 / a) with halves up, in integers.
    let v = (u32::from(c) * 510 + a) / (2 * a);
    u8::try_from(v.min(255)).unwrap_or(0)
}

/// Which way a pixel pass converts.
#[derive(Clone, Copy)]
enum Direction {
    Premultiply,
    Unpremultiply,
}

fn convert_f32(px: [f32; 4], dir: Direction) -> [f32; 4] {
    let [r, g, b, a] = px;
    match dir {
        Direction::Premultiply => [r * a, g * a, b * a, a],
        Direction::Unpremultiply if a == 0.0 => [0.0; 4],
        Direction::Unpremultiply => [r / a, g / a, b / a, a],
    }
}

fn convert(dst: &mut ImageViewMut<'_>, dir: Direction) -> Result<(), BufferError> {
    match dst.data_mut() {
        PixelDataMut::Alpha8(_) => Ok(()),
        PixelDataMut::Rgba8(px) => {
            for p in px.chunks_exact_mut(4) {
                let a = p[3];
                for c in &mut p[..3] {
                    *c = match dir {
                        Direction::Premultiply => premultiply_u8(*c, a),
                        Direction::Unpremultiply => unpremultiply_u8(*c, a),
                    };
                }
            }
            Ok(())
        }
        PixelDataMut::RgbaF32(px) => {
            // Check every input, then every result, before the first write.
            if let Some(index) = px.iter().position(|v| !v.is_finite()) {
                return Err(BufferError::NonFinite {
                    field: "pixels",
                    index,
                });
            }
            for (i, p) in px.chunks_exact(4).enumerate() {
                let out = convert_f32([p[0], p[1], p[2], p[3]], dir);
                if let Some(k) = out.iter().position(|v| !v.is_finite()) {
                    return Err(BufferError::NonFinite {
                        field: "pixels",
                        index: i * 4 + k,
                    });
                }
            }
            for p in px.chunks_exact_mut(4) {
                p.copy_from_slice(&convert_f32([p[0], p[1], p[2], p[3]], dir));
            }
            Ok(())
        }
    }
}

/// Premultiplies an RGBA view in place. `Alpha8` has no colour and is left as is.
///
/// # Errors
///
/// [`BufferError::NonFinite`] for an `f32` pixel that is, or becomes, NaN or
/// infinite. The buffer is then left unchanged.
pub fn premultiply(dst: &mut ImageViewMut<'_>) -> Result<(), BufferError> {
    convert(dst, Direction::Premultiply)
}

/// Unpremultiplies an RGBA view in place; `a = 0` gives `(0, 0, 0, 0)`.
///
/// # Errors
///
/// As [`premultiply`].
pub fn unpremultiply(dst: &mut ImageViewMut<'_>) -> Result<(), BufferError> {
    convert(dst, Direction::Unpremultiply)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bits(v: &[f32]) -> Vec<u32> {
        v.iter().map(|x| x.to_bits()).collect()
    }

    // ARQ-117: all 65 536 pairs, premultiplied <= a and the exact rounding.
    #[test]
    fn arq_117_premultiply_is_exact_rounding_and_at_most_alpha() {
        for c in 0..=255u32 {
            for a in 0..=255u32 {
                let got = u32::from(premultiply_u8(c as u8, a as u8));
                let exact = (f64::from(c) * f64::from(a) / 255.0 + 0.5).floor();
                assert_eq!(f64::from(got), exact, "c={c} a={a}");
                assert!(got <= a, "c={c} a={a} got={got}");
            }
        }
    }

    // ARQ-117: unpremultiply rounds, clamps and maps a = 0 to 0.
    #[test]
    fn arq_117_unpremultiply_rounds_clamps_and_zero_alpha_is_zero() {
        for c in 0..=255u32 {
            assert_eq!(unpremultiply_u8(c as u8, 0), 0, "c={c} a=0");
            for a in 1..=255u32 {
                let exact = (f64::from(c) * 255.0 / f64::from(a) + 0.5)
                    .floor()
                    .min(255.0);
                assert_eq!(
                    f64::from(unpremultiply_u8(c as u8, a as u8)),
                    exact,
                    "c={c} a={a}"
                );
            }
        }
        // Boundaries: clamp, truncation versus rounding.
        assert_eq!(unpremultiply_u8(255, 1), 255);
        assert_eq!(unpremultiply_u8(2, 3), 170);
        assert_eq!(unpremultiply_u8(1, 3), 85);
        assert_eq!(unpremultiply_u8(1, 2), 128);
    }

    // ARQ-117: a = 255 is the identity both ways; a = 0 gives 0.
    #[test]
    fn arq_117_opaque_is_identity_and_transparent_is_zero() {
        for c in 0..=255u8 {
            assert_eq!(premultiply_u8(c, 255), c);
            assert_eq!(unpremultiply_u8(c, 255), c);
            assert_eq!(premultiply_u8(c, 0), 0);
        }
    }

    // ARQ-117: whole-view operation keeps alpha, handles a = 0, ignores Alpha8.
    #[test]
    fn arq_117_view_premultiply_and_unpremultiply_8bit() {
        let mut px = [200u8, 100, 50, 128, 9, 8, 7, 0, 1, 2, 3, 255];
        let mut v = ImageViewMut::new(3, 1, PixelDataMut::Rgba8(&mut px)).unwrap();
        premultiply(&mut v).unwrap();
        assert_eq!(px, [100, 50, 25, 128, 0, 0, 0, 0, 1, 2, 3, 255]);
        let mut v = ImageViewMut::new(3, 1, PixelDataMut::Rgba8(&mut px)).unwrap();
        unpremultiply(&mut v).unwrap();
        assert_eq!(px, [199, 100, 50, 128, 0, 0, 0, 0, 1, 2, 3, 255]);
        let mut mask = [10u8, 20];
        let mut v = ImageViewMut::new(2, 1, PixelDataMut::Alpha8(&mut mask)).unwrap();
        premultiply(&mut v).unwrap();
        unpremultiply(&mut v).unwrap();
        assert_eq!(mask, [10, 20]);
    }

    // ARQ-117: f32 premultiply and unpremultiply; a = 0 gives zeros, even -0.0.
    #[test]
    fn arq_117_view_f32_values_and_zero_alpha() {
        let mut px = [
            0.5f32, 1.0, -0.25, 0.5, 0.3, 0.2, 0.1, 0.0, 0.3, 0.2, 0.1, -0.0,
        ];
        let mut v = ImageViewMut::new(3, 1, PixelDataMut::RgbaF32(&mut px)).unwrap();
        premultiply(&mut v).unwrap();
        assert_eq!(px[..4], [0.25, 0.5, -0.125, 0.5]);
        assert_eq!(px[4..8], [0.0, 0.0, 0.0, 0.0]);
        let mut v = ImageViewMut::new(3, 1, PixelDataMut::RgbaF32(&mut px)).unwrap();
        unpremultiply(&mut v).unwrap();
        assert_eq!(px[..4], [0.5, 1.0, -0.25, 0.5]);
        assert_eq!(bits(&px[4..8]), bits(&[0.0; 4]));
        assert_eq!(bits(&px[8..12]), bits(&[0.0; 4]));
        // A negative alpha divides with its sign.
        let mut neg = [0.25f32, 0.0, 0.0, -0.5];
        let mut v = ImageViewMut::new(1, 1, PixelDataMut::RgbaF32(&mut neg)).unwrap();
        unpremultiply(&mut v).unwrap();
        assert_eq!(neg, [-0.5, 0.0, 0.0, -0.5]);
    }

    // ARQ-110: NaN or infinity in any channel is Err and the buffer is untouched.
    #[test]
    fn arq_110_non_finite_pixel_is_err_and_buffer_unchanged() {
        let bad = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY];
        for ch in 0..4 {
            for b in bad {
                for op in [premultiply, unpremultiply] {
                    let mut px = [0.5f32, 0.5, 0.5, 0.5, 0.25, 0.25, 0.25, 0.5];
                    px[4 + ch] = b;
                    let before = bits(&px);
                    let mut v = ImageViewMut::new(2, 1, PixelDataMut::RgbaF32(&mut px)).unwrap();
                    let err = op(&mut v).unwrap_err();
                    assert_eq!(
                        err,
                        BufferError::NonFinite {
                            field: "pixels",
                            index: 4 + ch
                        }
                    );
                    assert_eq!(bits(&px), before, "ch={ch} b={b}");
                }
            }
        }
    }

    // ARQ-110: a computed value that is not finite is Err, even when inputs are finite.
    #[test]
    fn arq_110_overflowing_result_is_err_and_buffer_unchanged() {
        let mut px = [0.5f32, 0.5, 0.5, 0.5, f32::MAX, 0.0, 0.0, 2.0];
        let before = bits(&px);
        let mut v = ImageViewMut::new(2, 1, PixelDataMut::RgbaF32(&mut px)).unwrap();
        assert!(premultiply(&mut v).is_err());
        assert_eq!(bits(&px), before);
        let mut px = [0.5f32, 0.5, 0.5, 0.5, f32::MAX, 0.0, 0.0, 1e-30];
        let before = bits(&px);
        let mut v = ImageViewMut::new(2, 1, PixelDataMut::RgbaF32(&mut px)).unwrap();
        assert!(unpremultiply(&mut v).is_err());
        assert_eq!(bits(&px), before);
    }

    // S13-18, ARQ-110: the error names the element that became non-finite, `4 i + k` for
    // channel `k` of pixel `i`: here channel 2 (blue) of the second pixel.
    #[test]
    fn arq_110_overflowing_result_names_its_element() {
        let nan_at = |r: Result<(), BufferError>| match r {
            Err(BufferError::NonFinite { field, index }) => (field, index),
            other => panic!("expected NonFinite, got {other:?}"),
        };
        let mut px = [0.5f32, 0.5, 0.5, 0.5, 0.0, 0.0, f32::MAX, 2.0];
        let mut v = ImageViewMut::new(2, 1, PixelDataMut::RgbaF32(&mut px)).unwrap();
        assert_eq!(nan_at(premultiply(&mut v)), ("pixels", 6));
        let mut px = [0.5f32, 0.5, 0.5, 0.5, 0.0, 0.0, f32::MAX, 1e-30];
        let mut v = ImageViewMut::new(2, 1, PixelDataMut::RgbaF32(&mut px)).unwrap();
        assert_eq!(nan_at(unpremultiply(&mut v)), ("pixels", 6));
    }

    // ARQ-111: the same input gives the same bytes.
    #[test]
    fn arq_111_premultiply_is_deterministic() {
        let src: Vec<f32> = (0..64).map(|i| (i as f32) * 0.013).collect();
        let run = || {
            let mut px = src.clone();
            let mut v = ImageViewMut::new(4, 4, PixelDataMut::RgbaF32(&mut px)).unwrap();
            premultiply(&mut v).unwrap();
            bits(&px)
        };
        assert_eq!(run(), run());
    }

    // ARQ-110: a non-finite colour in an a = 0 pixel is Err, not silently zeroed.
    #[test]
    fn arq_110_non_finite_colour_in_transparent_pixel_is_err() {
        for b in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for ch in 0..3 {
                for op in [premultiply, unpremultiply] {
                    for alpha in [0.0f32, -0.0] {
                        let mut px = [0.5f32, 0.5, 0.5, 0.5, 0.1, 0.1, 0.1, alpha];
                        px[4 + ch] = b;
                        let before = bits(&px);
                        let mut v =
                            ImageViewMut::new(2, 1, PixelDataMut::RgbaF32(&mut px)).unwrap();
                        assert_eq!(
                            op(&mut v).unwrap_err(),
                            BufferError::NonFinite {
                                field: "pixels",
                                index: 4 + ch
                            }
                        );
                        assert_eq!(bits(&px), before);
                    }
                }
            }
        }
    }

    // ARQ-111: every conversion, in both formats, run twice gives the same bytes.
    #[test]
    fn arq_111_every_conversion_is_deterministic() {
        let f: Vec<f32> = (0..64)
            .map(|i| ((i * 7) % 11) as f32 * 0.09 + 0.01)
            .collect();
        let b: Vec<u8> = (0..64).map(|i| (i * 37 % 256) as u8).collect();
        for op in [premultiply, unpremultiply] {
            let run_f = || {
                let mut px = f.clone();
                let mut v = ImageViewMut::new(4, 4, PixelDataMut::RgbaF32(&mut px)).unwrap();
                op(&mut v).unwrap();
                bits(&px)
            };
            let run_b = || {
                let mut px = b.clone();
                let mut v = ImageViewMut::new(4, 4, PixelDataMut::Rgba8(&mut px)).unwrap();
                op(&mut v).unwrap();
                px
            };
            assert_eq!(run_f(), run_f());
            assert_eq!(run_b(), run_b());
            assert_ne!(run_b(), b, "the conversion must do something");
        }
    }

    mod property {
        use super::*;
        use proptest::prelude::*;

        fn pixels() -> impl Strategy<Value = Vec<f32>> {
            prop::collection::vec(-1.0e6f32..1.0e6, 0..8).prop_map(|mut v| {
                v.truncate(v.len() / 4 * 4);
                v
            })
        }

        proptest! {
            // ARQ-110: finite input in the domain gives finite output, or Err and no change.
            #[test]
            fn arq_110_finite_input_gives_finite_output(px in pixels()) {
                for op in [premultiply, unpremultiply] {
                    let mut buf = px.clone();
                    let n = (buf.len() / 4) as u32;
                    let mut v = ImageViewMut::new(n, 1, PixelDataMut::RgbaF32(&mut buf)).unwrap();
                    match op(&mut v) {
                        Ok(()) => prop_assert!(buf.iter().all(|x| x.is_finite())),
                        Err(_) => prop_assert_eq!(bits(&buf), bits(&px)),
                    }
                }
            }

            // ARQ-110: alpha in [0, 1] and colour in [0, 1] always premultiply to finite values.
            #[test]
            fn arq_110_unit_range_premultiply_is_ok_and_finite(
                px in prop::collection::vec(0.0f32..=1.0, 0..8)
            ) {
                let mut buf = px.clone();
                buf.truncate(buf.len() / 4 * 4);
                let n = (buf.len() / 4) as u32;
                let mut v = ImageViewMut::new(n, 1, PixelDataMut::RgbaF32(&mut buf)).unwrap();
                prop_assert!(premultiply(&mut v).is_ok());
                prop_assert!(buf.iter().all(|x| x.is_finite()));
            }

            // ARQ-110: mip levels of finite input are finite.
            #[test]
            fn arq_110_finite_mips_are_finite(
                w in 1u32..6, h in 1u32..6,
                seed in prop::collection::vec(-1.0e6f32..1.0e6, 100)
            ) {
                let px: Vec<f32> = seed.iter().copied().cycle().take((w * h * 4) as usize).collect();
                let view = crate::ImageView::new(w, h, crate::PixelData::RgbaF32(&px)).unwrap();
                for level in crate::build_mips(view).unwrap() {
                    match level.data() {
                        crate::PixelBuf::RgbaF32(d) => prop_assert!(d.iter().all(|x| x.is_finite())),
                        _ => prop_assert!(false),
                    }
                }
            }
        }
    }
}
