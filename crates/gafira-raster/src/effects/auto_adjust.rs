//! Ajuste automático: the sliders of Balance de blancos and Luz that a one-click correction
//! gives (FD-33 (d)).

use gafira_doc::contract::{ParamError, ParamValue, ParamValues};

use super::eyedropper;
use crate::transfer::Transfer;
use crate::{BufferError, EffectError, ImageView, PixelData, PixelFormat, unpremultiply_u8};

/// The sliders the correction sets, in slider units (`[-100, 100]`), already scaled by the
/// intensity. They go to Balance de blancos and Luz and can be fine tuned there.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AutoAdjust {
    /// Temperatura.
    pub temperature: f64,
    /// Matiz.
    pub tint: f64,
    /// Brillo.
    pub brightness: f64,
    /// Blancos.
    pub whites: f64,
    /// Negros.
    pub blacks: f64,
}

impl AutoAdjust {
    /// The values for [`WhiteBalance`](super::WhiteBalance).
    #[must_use]
    pub fn white_balance_params(&self) -> ParamValues {
        let mut p = ParamValues::new();
        p.set("temperature", ParamValue::Float(self.temperature));
        p.set("tint", ParamValue::Float(self.tint));
        p
    }

    /// The values for [`Light`](super::Light).
    #[must_use]
    pub fn light_params(&self) -> ParamValues {
        let mut p = ParamValues::new();
        p.set("brightness", ParamValue::Float(self.brightness));
        p.set("whites", ParamValue::Float(self.whites));
        p.set("blacks", ParamValue::Float(self.blacks));
        p
    }
}

/// Where the median of Y goes (linear light).
const TARGET_MEDIAN: f64 = 0.18;

/// Luminance of a linear colour.
fn luma(c: [f64; 3]) -> f64 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

/// The nearest-rank percentile `p` of a sorted, non-empty list: the smallest value with at
/// least `p` percent of the values at or below it.
fn percentile(sorted: &[f64], p: f64) -> f64 {
    let rank = (p / 100.0 * sorted.len() as f64).ceil() as usize;
    sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
}

/// Computes the correction of `original` (never an image that already carries the stack of
/// adjustments) at `intensity` in `[0, 100]`.
///
/// # Errors
///
/// [`EffectError`] for an intensity that is not finite or outside `[0, 100]`, an `Alpha8`
/// image, or a non-finite `f32` pixel.
pub fn auto_adjust(original: ImageView<'_>, intensity: f64) -> Result<AutoAdjust, EffectError> {
    if !intensity.is_finite() {
        return Err(ParamError::NonFinite { param: "intensity" }.into());
    }
    if !(0.0..=100.0).contains(&intensity) {
        return Err(ParamError::OutOfRange { param: "intensity" }.into());
    }
    let format = original.data().format();
    if format == PixelFormat::Alpha8 {
        return Err(EffectError::UnsupportedFormat { format });
    }
    if let PixelData::RgbaF32(px) = original.data()
        && let Some(index) = px.iter().position(|v| !v.is_finite())
    {
        return Err(BufferError::NonFinite {
            field: "src",
            index,
        }
        .into());
    }
    // Straight linear colour of every pixel that has alpha.
    let t = Transfer::new();
    let mut colours: Vec<[f64; 3]> = Vec::new();
    match original.data() {
        PixelData::Rgba8(s) => {
            for px in s.chunks_exact(4).filter(|px| px[3] != 0) {
                colours
                    .push([0, 1, 2].map(|k| t.decode[usize::from(unpremultiply_u8(px[k], px[3]))]));
            }
        }
        PixelData::RgbaF32(s) => {
            for px in s.chunks_exact(4).filter(|px| px[3] != 0.0) {
                let a = f64::from(px[3]);
                colours.push([0, 1, 2].map(|k| f64::from(px[k]) / a));
            }
        }
        PixelData::Alpha8(_) => {}
    }
    if colours.is_empty() {
        return Ok(AutoAdjust::default());
    }
    let n = colours.len();
    let mut y: Vec<f64> = colours.iter().map(|c| luma(*c)).collect();
    let mut v: Vec<f64> = y.iter().map(|y| t.encode_f64(*y).clamp(0.0, 1.0)).collect();
    y.sort_by(f64::total_cmp);
    v.sort_by(f64::total_cmp);
    let mean = [0, 1, 2].map(|k| colours.iter().map(|c| c[k]).sum::<f64>() / n as f64);
    // Brillo takes the median of Y to 0.18; a black image has no median to move.
    let median = percentile(&y, 50.0);
    let brightness = if median > 0.0 {
        (TARGET_MEDIAN / median).log2().clamp(-1.0, 1.0)
    } else {
        0.0
    };
    // n = -0.25 k_N and b = 1 - 0.25 k_B, solved for the sliders.
    let blacks = (-4.0 * percentile(&v, 0.5)).clamp(-1.0, 1.0);
    let whites = (4.0 * (1.0 - percentile(&v, 99.5))).clamp(-1.0, 1.0);
    let tt = eyedropper([mean[0] as f32, mean[1] as f32, mean[2] as f32])?;
    let k = intensity / 100.0;
    Ok(AutoAdjust {
        temperature: tt.temperature * k,
        tint: tt.tint * k,
        brightness: brightness * 100.0 * k,
        whites: whites * 100.0 * k,
        blacks: blacks * 100.0 * k,
    })
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::super::{Effect, Light, WhiteBalance, eyedropper};
    use super::*;

    fn view(px: &[f32], w: u32, h: u32) -> ImageView<'_> {
        ImageView::new(w, h, PixelData::RgbaF32(px)).unwrap()
    }

    fn grey_f32(values: &[f64]) -> Vec<f32> {
        values
            .iter()
            .flat_map(|v| [*v as f32, *v as f32, *v as f32, 1.0])
            .collect()
    }

    fn near(a: f64, b: f64, what: &str) {
        assert!((a - b).abs() < 1e-3, "{what}: {a} vs {b}");
    }

    // FD-33 (d): Brillo takes the median of Y to 0.18 (log2 of the ratio, in slider units).
    #[test]
    fn fd_33_d_brightness_moves_the_median_luminance_to_0_18() {
        let at = |y: f64, i: f64| {
            let px = grey_f32(&[y; 4]);
            auto_adjust(view(&px, 2, 2), i).unwrap().brightness
        };
        near(at(0.09, 100.0), 100.0, "dark by one stop");
        near(at(0.36, 100.0), -100.0, "bright by one stop");
        near(at(0.18, 100.0), 0.0, "already there");
        near(
            at(0.127, 100.0),
            100.0 * (0.18f64 / 0.127).log2(),
            "between",
        );
        near(at(0.0225, 100.0), 100.0, "clipped at +100");
        near(
            at(0.9, 100.0),
            -100.0 * 1.0f64.max(0.0),
            "clipped at -100 (2.3 stops)",
        );
        // The median, not the mean: 0.05, 0.1 and 0.9 give 0.1.
        let px = grey_f32(&[0.9, 0.05, 0.1]);
        near(
            auto_adjust(view(&px, 3, 1), 100.0).unwrap().brightness,
            100.0 * (0.18f64 / 0.1).log2(),
            "median of three",
        );
        // With an even count the lower middle one counts.
        let px = grey_f32(&[0.9, 0.05, 0.1, 0.2]);
        near(
            auto_adjust(view(&px, 4, 1), 100.0).unwrap().brightness,
            100.0 * (0.18f64 / 0.1).log2(),
            "median of four",
        );
    }

    // FD-33 (d): Negros and Blancos from the 0.5 and 99.5 percentiles of v_Y.
    #[test]
    fn fd_33_d_whites_and_blacks_come_from_the_0_5_and_99_5_percentiles() {
        let v = |i: usize| 0.1 + 0.8 * i as f64 / 999.0;
        let values: Vec<f64> = (0..1000).map(|i| dec_f(v(i))).collect();
        let px = grey_f32(&values);
        let a = auto_adjust(view(&px, 1000, 1), 100.0).unwrap();
        // Nearest rank: the 5th and the 995th of 1000.
        near(a.blacks, -400.0 * v(4), "blacks");
        near(a.whites, 400.0 * (1.0 - v(994)), "whites");
        // Order of the input does not matter.
        let mut rev = values.clone();
        rev.reverse();
        let px = grey_f32(&rev);
        assert_eq!(
            auto_adjust(view(&px, 1000, 1), 100.0).unwrap().blacks,
            a.blacks
        );
        // A full range image needs neither; out of range values clip at the ends.
        let px = grey_f32(
            &(0..1000)
                .map(|i| dec_f(i as f64 / 999.0))
                .collect::<Vec<_>>(),
        );
        let full = auto_adjust(view(&px, 1000, 1), 100.0).unwrap();
        assert!(
            full.blacks.abs() < 2.5 && full.whites.abs() < 2.5,
            "{full:?}"
        );
        let px = grey_f32(&[dec_f(0.5); 8]);
        let flat = auto_adjust(view(&px, 8, 1), 100.0).unwrap();
        near(flat.blacks, -100.0, "flat blacks clip");
        near(flat.whites, 100.0, "flat whites clip");
    }

    // FD-33 (d): T and M come from the eyedropper on the mean of R, G and B, alpha 0 ignored.
    #[test]
    fn fd_33_d_temperature_and_tint_use_the_mean_colour() {
        let mut px = vec![
            0.4f32, 0.5, 0.45, 1.0, 0.4, 0.5, 0.45, 1.0, 0.4, 0.5, 0.45, 1.0,
        ];
        // Pixels with alpha 0 add no colour.
        px.extend([0.0f32, 0.0, 0.0, 0.0]);
        let a = auto_adjust(view(&px, 4, 1), 100.0).unwrap();
        let tt = eyedropper([0.4, 0.5, 0.45]).unwrap();
        near(a.temperature, tt.temperature, "temperature");
        near(a.tint, tt.tint, "tint");
        // Straight colour of a translucent pixel is the mean's input, not the premultiplied one.
        let px = [0.2f32, 0.25, 0.225, 0.5, 0.4, 0.5, 0.45, 1.0];
        let b = auto_adjust(view(&px, 2, 1), 100.0).unwrap();
        near(b.temperature, tt.temperature, "translucent temperature");
        // The mean of a mix, not the first pixel.
        let px = [0.2f32, 0.2, 0.2, 1.0, 0.6, 0.8, 1.0, 1.0];
        let m = auto_adjust(view(&px, 2, 1), 100.0).unwrap();
        let tm = eyedropper([0.4, 0.5, 0.6]).unwrap();
        near(m.temperature, tm.temperature, "mean temperature");
        near(m.tint, tm.tint, "mean tint");
    }

    // FD-33 (d): a black image leaves Brillo, T and M at 0; so does one with no colour at all.
    #[test]
    fn fd_33_d_a_black_image_keeps_brightness_temperature_and_tint_at_zero() {
        for px in [
            grey_f32(&[0.0; 6]),
            vec![0.0f32; 24],
            vec![0.3f32, 0.2, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0],
        ] {
            let n = (px.len() / 4) as u32;
            let a = auto_adjust(view(&px, n, 1), 100.0).unwrap();
            assert_eq!(
                (a.brightness, a.temperature, a.tint),
                (0.0, 0.0, 0.0),
                "{px:?}"
            );
        }
        let a = auto_adjust(view(&[], 0, 0), 100.0).unwrap();
        assert_eq!(a, AutoAdjust::default());
        let black8 = [0u8, 0, 0, 255, 0, 0, 0, 255];
        let a = auto_adjust(
            ImageView::new(2, 1, PixelData::Rgba8(&black8)).unwrap(),
            100.0,
        )
        .unwrap();
        assert_eq!((a.brightness, a.temperature, a.tint), (0.0, 0.0, 0.0));
    }

    // FD-33 (d): Brillo takes the straight colour of translucent pixels, in both formats, and
    // the median is the middle one, not the 40th percentile.
    #[test]
    fn fd_33_d_translucent_pixels_count_with_their_straight_colour() {
        // f32: premultiplied 0.09 at alpha 0.5 is straight 0.18, already at the target.
        let px = [0.09f32, 0.09, 0.09, 0.5];
        near(
            auto_adjust(view(&px, 1, 1), 100.0).unwrap().brightness,
            0.0,
            "f32 translucent",
        );
        // 8 bits: premultiplied (60, 40, 20) at alpha 128 is straight (120, 80, 40).
        let b = [60u8, 40, 20, 128];
        let a = auto_adjust(ImageView::new(1, 1, PixelData::Rgba8(&b)).unwrap(), 100.0).unwrap();
        let straight = [lin(120.0), lin(80.0), lin(40.0)];
        let tt = eyedropper(straight.map(|x| x as f32)).unwrap();
        near(a.temperature, tt.temperature, "8-bit temperature");
        near(a.tint, tt.tint, "8-bit tint");
        let y = 0.2126 * straight[0] + 0.7152 * straight[1] + 0.0722 * straight[2];
        near(
            a.brightness,
            (100.0 * (0.18f64 / y).log2()).clamp(-100.0, 100.0),
            "8-bit brightness",
        );
        // Ten values: the median is the fifth, 0.2.
        let px = grey_f32(&(1..=10).map(|i| f64::from(i) * 0.04).collect::<Vec<_>>());
        near(
            auto_adjust(view(&px, 10, 1), 100.0).unwrap().brightness,
            100.0 * (0.18f64 / 0.2).log2(),
            "median of ten",
        );
    }

    // FD-33 (d): the result is multiplied by Intensidad / 100; the ends are accepted.
    #[test]
    fn fd_33_d_intensity_scales_every_value() {
        // Dimmed so the top percentile of v_Y stays below 1 and Blancos is not 0.
        let src: Vec<f32> = img_f32(10, 8, 3)
            .chunks(4)
            .flat_map(|p| [p[0] * 0.3, p[1] * 0.4, p[2] * 0.9, p[3]])
            .collect();
        let full = auto_adjust(view(&src, 10, 8), 100.0).unwrap();
        assert!(full.brightness != 0.0 && full.temperature != 0.0 && full.whites != 0.0);
        for i in [0.0, 25.0, 60.0, 99.5] {
            let a = auto_adjust(view(&src, 10, 8), i).unwrap();
            for (x, y) in [
                (a.temperature, full.temperature),
                (a.tint, full.tint),
                (a.brightness, full.brightness),
                (a.whites, full.whites),
                (a.blacks, full.blacks),
            ] {
                assert!((x - y * i / 100.0).abs() < 1e-9, "{i}: {x} vs {y}");
            }
        }
        assert_eq!(
            auto_adjust(view(&src, 10, 8), 0.0).unwrap(),
            AutoAdjust::default()
        );
    }

    // FD-33 (h), ARQ-110: a bad intensity is Err.
    #[test]
    fn fd_33_h_intensity_outside_0_100_or_not_finite_is_err() {
        let src = img_f32(3, 3, 1);
        for bad in [-0.01, 100.01, -1e300, 1e300] {
            assert_eq!(
                auto_adjust(view(&src, 3, 3), bad),
                Err(EffectError::Param(ParamError::OutOfRange {
                    param: "intensity"
                })),
                "{bad}"
            );
        }
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                auto_adjust(view(&src, 3, 3), bad),
                Err(EffectError::Param(ParamError::NonFinite {
                    param: "intensity"
                })),
                "{bad}"
            );
        }
        auto_adjust(view(&src, 3, 3), 0.0).unwrap();
        auto_adjust(view(&src, 3, 3), 100.0).unwrap();
    }

    // ARQ-110 and formats.
    #[test]
    fn arq_110_auto_adjust_rejects_non_finite_pixels_and_alpha8() {
        let mut src = img_f32(3, 3, 1);
        src[5] = f32::INFINITY;
        assert_eq!(
            auto_adjust(view(&src, 3, 3), 60.0),
            Err(EffectError::Buffer(BufferError::NonFinite {
                field: "src",
                index: 5
            }))
        );
        let a8 = [1u8, 2, 3, 4];
        assert_eq!(
            auto_adjust(ImageView::new(2, 2, PixelData::Alpha8(&a8)).unwrap(), 60.0),
            Err(EffectError::UnsupportedFormat {
                format: PixelFormat::Alpha8
            })
        );
    }

    // FD-33 (d): the values fit the sliders they go to.
    #[test]
    fn fd_33_d_the_values_are_valid_for_white_balance_and_light() {
        let src = img_f32(12, 9, 8);
        let a = auto_adjust(view(&src, 12, 9), 100.0).unwrap();
        WhiteBalance::new()
            .params()
            .check_values(&a.white_balance_params())
            .unwrap();
        Light::new()
            .params()
            .check_values(&a.light_params())
            .unwrap();
        assert_eq!(a.white_balance_params().float("tint"), Some(a.tint));
        assert_eq!(a.light_params().float("blacks"), Some(a.blacks));
        for v in [a.temperature, a.tint, a.brightness, a.whites, a.blacks] {
            assert!((-100.0..=100.0).contains(&v), "{a:?}");
        }
    }

    fn apply(a: &AutoAdjust, src: &[u8], size: (u32, u32)) -> Vec<u8> {
        let mid = run8(
            &WhiteBalance::new(),
            &cx(1.0),
            &a.white_balance_params(),
            src,
            size,
        )
        .unwrap();
        run8(&Light::new(), &cx(1.0), &a.light_params(), &mid, size).unwrap()
    }

    // FD-33 (d): idempotent. It is computed on the original, so asking twice gives the same
    // values bit for bit, and applying them twice to the original gives the same image.
    #[test]
    fn fd_33_d_auto_adjust_is_idempotent() {
        let src = img_u8(16, 12, 5);
        let view8 = || ImageView::new(16, 12, PixelData::Rgba8(&src)).unwrap();
        let a = auto_adjust(view8(), 60.0).unwrap();
        let b = auto_adjust(view8(), 60.0).unwrap();
        assert_eq!(a, b);
        let (r1, r2) = (apply(&a, &src, (16, 12)), apply(&b, &src, (16, 12)));
        for (x, y) in r1.iter().zip(&r2) {
            assert!(x.abs_diff(*y) <= 1);
        }
        assert_ne!(r1, src);
    }
}
