//! Transfer curve, CSS Color 4 matrices and the one conversion pipeline.
//!
//! Everything here is crate-internal. Values are widened to `f64`, converted,
//! and narrowed to `f32` once.

use crate::ColorSpaceId;

/// A row-major 3x3 matrix.
pub(crate) type Mat3 = [[f64; 3]; 3];

/// Encoded value at or below which the sRGB curve is linear.
const DECODE_THRESHOLD: f64 = 0.04045;
/// Linear value at or below which the sRGB curve is linear.
const ENCODE_THRESHOLD: f64 = 0.0031308;
/// Slope of the linear segment.
const LINEAR_SLOPE: f64 = 12.92;
/// Offset of the power segment.
const OFFSET: f64 = 0.055;
/// Scale of the power segment.
const SCALE: f64 = 1.055;
/// Exponent of the power segment.
const GAMMA: f64 = 2.4;

/// Linear sRGB to CIE XYZ (D65), from the CSS Color 4 sample code.
pub(crate) const SRGB_TO_XYZ: Mat3 = [
    [506752.0 / 1228815.0, 87881.0 / 245763.0, 12673.0 / 70218.0],
    [87098.0 / 409605.0, 175762.0 / 245763.0, 12673.0 / 175545.0],
    [7918.0 / 409605.0, 87881.0 / 737289.0, 1001167.0 / 1053270.0],
];

/// CIE XYZ (D65) to linear sRGB, from the CSS Color 4 sample code.
pub(crate) const XYZ_TO_SRGB: Mat3 = [
    [12831.0 / 3959.0, -329.0 / 214.0, -1974.0 / 3959.0],
    [
        -851781.0 / 878810.0,
        1648619.0 / 878810.0,
        36519.0 / 878810.0,
    ],
    [705.0 / 12673.0, -2585.0 / 12673.0, 705.0 / 667.0],
];

/// Linear Display-P3 to CIE XYZ (D65), from the CSS Color 4 sample code.
pub(crate) const P3_TO_XYZ: Mat3 = [
    [
        608311.0 / 1250200.0,
        189793.0 / 714400.0,
        198249.0 / 1000160.0,
    ],
    [
        35783.0 / 156275.0,
        247089.0 / 357200.0,
        198249.0 / 2500400.0,
    ],
    [0.0, 32229.0 / 714400.0, 5220557.0 / 5000800.0],
];

/// CIE XYZ (D65) to linear Display-P3, from the CSS Color 4 sample code.
pub(crate) const XYZ_TO_P3: Mat3 = [
    [
        446124.0 / 178915.0,
        -333277.0 / 357830.0,
        -72051.0 / 178915.0,
    ],
    [-14852.0 / 17905.0, 63121.0 / 35810.0, 423.0 / 17905.0],
    [11844.0 / 330415.0, -50337.0 / 660830.0, 316169.0 / 330415.0],
];

/// Linear sRGB to linear Display-P3.
pub(crate) const SRGB_TO_P3: Mat3 = mul(&XYZ_TO_P3, &SRGB_TO_XYZ);

/// Linear Display-P3 to linear sRGB.
pub(crate) const P3_TO_SRGB: Mat3 = mul(&XYZ_TO_SRGB, &P3_TO_XYZ);

/// Matrix product `a * b`.
pub(crate) const fn mul(a: &Mat3, b: &Mat3) -> Mat3 {
    let mut out = [[0.0; 3]; 3];
    let mut i = 0;
    while i < 3 {
        let mut j = 0;
        while j < 3 {
            out[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
            j += 1;
        }
        i += 1;
    }
    out
}

/// Matrix-vector product `m * v`.
pub(crate) fn mul_vec(m: &Mat3, v: [f64; 3]) -> [f64; 3] {
    m.map(|row| row[0] * v[0] + row[1] * v[1] + row[2] * v[2])
}

/// sRGB transfer curve, encoded to linear. Sign-symmetric and unclamped.
pub(crate) fn decode(c: f64) -> f64 {
    let magnitude = c.abs();
    if magnitude <= DECODE_THRESHOLD {
        c / LINEAR_SLOPE
    } else {
        ((magnitude + OFFSET) / SCALE).powf(GAMMA).copysign(c)
    }
}

/// sRGB transfer curve, linear to encoded. Sign-symmetric and unclamped.
pub(crate) fn encode(l: f64) -> f64 {
    let magnitude = l.abs();
    if magnitude <= ENCODE_THRESHOLD {
        LINEAR_SLOPE * l
    } else {
        (SCALE * magnitude.powf(1.0 / GAMMA) - OFFSET).copysign(l)
    }
}

/// Channels of `space` to linear-light sRGB primaries.
fn to_linear_srgb(space: ColorSpaceId, rgb: [f64; 3]) -> [f64; 3] {
    match space {
        ColorSpaceId::Srgb => rgb.map(decode),
        ColorSpaceId::LinearSrgb => rgb,
        ColorSpaceId::DisplayP3 => mul_vec(&P3_TO_SRGB, rgb.map(decode)),
    }
}

/// Linear-light sRGB primaries to the channels of `space`.
fn from_linear_srgb(space: ColorSpaceId, rgb: [f64; 3]) -> [f64; 3] {
    match space {
        ColorSpaceId::Srgb => rgb.map(encode),
        ColorSpaceId::LinearSrgb => rgb,
        ColorSpaceId::DisplayP3 => mul_vec(&SRGB_TO_P3, rgb).map(encode),
    }
}

/// Converts color channels (never alpha) between spaces.
///
/// Infallible and unclamped. Same-space input is returned untouched; a
/// non-finite channel yields unspecified (NaN or infinite) output, never a panic.
pub(crate) fn convert_rgb(src: ColorSpaceId, dst: ColorSpaceId, rgb: [f32; 3]) -> [f32; 3] {
    if src == dst {
        return rgb;
    }
    let linear = to_linear_srgb(src, rgb.map(f64::from));
    from_linear_srgb(dst, linear).map(|c| c as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ColorSpaceId::{DisplayP3, LinearSrgb, Srgb};

    fn close(actual: f64, expected: f64, tol: f64) -> bool {
        (actual - expected).abs() <= tol * expected.abs().max(1.0)
    }

    fn assert_close3(actual: [f32; 3], expected: [f64; 3], tol: f64) {
        for i in 0..3 {
            assert!(
                close(f64::from(actual[i]), expected[i], tol),
                "channel {i}: {actual:?} vs {expected:?}"
            );
        }
    }

    fn bits3(v: [f32; 3]) -> [u32; 3] {
        v.map(f32::to_bits)
    }

    const IDENTITY: Mat3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    const D65: [f64; 3] = [0.3127 / 0.3290, 1.0, (1.0 - 0.3127 - 0.3290) / 0.3290];

    // ---- transfer curve -------------------------------------------------

    #[test]
    fn decode_reference_values() {
        assert_eq!(decode(0.0), 0.0);
        assert_eq!(decode(1.0), 1.0);
        // Independent f64 evaluation of ((0.5 + 0.055) / 1.055)^2.4 = 0.214041140...
        assert!((decode(0.5) - 0.214_041_14).abs() < 1e-8, "{}", decode(0.5));
        // Linear segment: 0.04 / 12.92.
        assert!((decode(0.04) - 0.04 / 12.92).abs() < 1e-15);
    }

    #[test]
    fn encode_reference_values() {
        assert_eq!(encode(0.0), 0.0);
        // 1.055 - 0.055 is not exactly 1.0 in f64.
        assert!((encode(1.0) - 1.0).abs() < 1e-15, "{}", encode(1.0));
        // Inverse of decode(0.5) above.
        assert!((encode(0.214_041_14) - 0.5).abs() < 1e-8);
        // Linear segment: 12.92 * 0.003.
        assert!((encode(0.003) - 12.92 * 0.003).abs() < 1e-15);
    }

    #[test]
    fn decode_is_continuous_at_its_threshold() {
        let t = 0.04045_f64;
        for sign in [1.0, -1.0] {
            let linear_side = decode(sign * t);
            let power_side = decode(sign * t.next_up());
            assert!(
                (linear_side - power_side).abs() < 1e-8,
                "gap {}",
                (linear_side - power_side).abs()
            );
        }
        // The threshold belongs to the linear branch.
        assert_eq!(decode(t), t / 12.92);
    }

    #[test]
    fn encode_is_continuous_at_its_threshold() {
        let t = 0.0031308_f64;
        for sign in [1.0, -1.0] {
            let linear_side = encode(sign * t);
            let power_side = encode(sign * t.next_up());
            // The published constants leave a genuine gap of about 2.85e-8.
            assert!(
                (linear_side - power_side).abs() < 5e-8,
                "gap {}",
                (linear_side - power_side).abs()
            );
        }
        assert_eq!(encode(t), 12.92 * t);
    }

    #[test]
    fn curve_is_odd_symmetric_bit_for_bit() {
        for x in [
            0.0, 1e-300, 0.003, 0.0031308, 0.04, 0.04045, 0.5, 1.0, 1.75, 4.0, 1e30,
        ] {
            assert_eq!(decode(-x).to_bits(), (-decode(x)).to_bits(), "decode {x}");
            assert_eq!(encode(-x).to_bits(), (-encode(x)).to_bits(), "encode {x}");
        }
    }

    #[test]
    fn negative_zero_keeps_its_sign_through_both_curves() {
        assert!(decode(-0.0).is_sign_negative());
        assert!(encode(-0.0).is_sign_negative());
        assert!(decode(0.0).is_sign_positive());
    }

    // ---- matrices ---------------------------------------------------------

    fn product(a: &Mat3, b: &Mat3) -> Mat3 {
        mul(a, b)
    }

    fn assert_identity(m: &Mat3) {
        for i in 0..3 {
            for j in 0..3 {
                assert!(
                    (m[i][j] - IDENTITY[i][j]).abs() < 1e-12,
                    "[{i}][{j}] = {}",
                    m[i][j]
                );
            }
        }
    }

    #[test]
    fn matrices_are_published_inverses() {
        assert_identity(&product(&SRGB_TO_XYZ, &XYZ_TO_SRGB));
        assert_identity(&product(&XYZ_TO_SRGB, &SRGB_TO_XYZ));
        assert_identity(&product(&P3_TO_XYZ, &XYZ_TO_P3));
        assert_identity(&product(&XYZ_TO_P3, &P3_TO_XYZ));
    }

    #[test]
    fn composite_matrices_are_inverses_of_each_other() {
        assert_identity(&product(&SRGB_TO_P3, &P3_TO_SRGB));
        assert_identity(&product(&P3_TO_SRGB, &SRGB_TO_P3));
    }

    #[test]
    fn white_maps_to_d65_in_both_gamuts() {
        for m in [&SRGB_TO_XYZ, &P3_TO_XYZ] {
            let xyz = mul_vec(m, [1.0, 1.0, 1.0]);
            for i in 0..3 {
                assert!((xyz[i] - D65[i]).abs() < 1e-12, "{xyz:?} vs {D65:?}");
            }
        }
    }

    #[test]
    fn composite_matrices_preserve_white() {
        for m in [&SRGB_TO_P3, &P3_TO_SRGB] {
            let white = mul_vec(m, [1.0, 1.0, 1.0]);
            for channel in white {
                assert!((channel - 1.0).abs() < 1e-12, "{white:?}");
            }
        }
    }

    #[test]
    fn luminance_row_is_bt709() {
        let y = SRGB_TO_XYZ[1];
        for (got, want) in y.into_iter().zip([0.2126, 0.7152, 0.0722]) {
            assert!((got - want).abs() < 1e-4, "{y:?}");
        }
        // Display-P3 luminance weights differ, so a swapped matrix cannot pass both.
        assert!((P3_TO_XYZ[1][0] - 0.2290).abs() < 1e-4);
    }

    // ---- convert_rgb: published references --------------------------------

    #[test]
    fn srgb_primaries_in_display_p3() {
        // CSS Color 4 publishes red as color(display-p3 0.91748756 0.20028681 0.13856059).
        // Independent f64 evaluation of the CSS matrices gives the other two.
        let red = convert_rgb(Srgb, DisplayP3, [1.0, 0.0, 0.0]);
        assert_close3(red, [0.91749, 0.20029, 0.13856], 1e-4);
        let green = convert_rgb(Srgb, DisplayP3, [0.0, 1.0, 0.0]);
        assert_close3(green, [0.45840, 0.98526, 0.29829], 1e-4);
        let blue = convert_rgb(Srgb, DisplayP3, [0.0, 0.0, 1.0]);
        assert_close3(blue, [0.0, 0.0, 0.9596], 1e-4);
    }

    #[test]
    fn display_p3_red_is_out_of_the_srgb_gamut_and_unclamped() {
        // Independent f64 evaluation: (1.09307, -0.22674, -0.15013).
        let out = convert_rgb(DisplayP3, Srgb, [1.0, 0.0, 0.0]);
        assert_close3(out, [1.0931, -0.2267, -0.1501], 1e-4);
        assert!(out[0] > 1.0 && out[1] < 0.0 && out[2] < 0.0, "{out:?}");
    }

    #[test]
    fn white_black_and_grays_across_spaces() {
        for target in [LinearSrgb, DisplayP3] {
            assert_close3(convert_rgb(Srgb, target, [1.0, 1.0, 1.0]), [1.0; 3], 1e-5);
            assert_close3(convert_rgb(Srgb, target, [0.0, 0.0, 0.0]), [0.0; 3], 1e-5);
            for g in [0.1_f32, 0.5, 0.9] {
                let out = convert_rgb(Srgb, target, [g, g, g]);
                assert!((out[0] - out[1]).abs() < 1e-5 && (out[1] - out[2]).abs() < 1e-5);
            }
        }
    }

    // ---- convert_rgb: structure of the pipeline ---------------------------

    #[test]
    fn srgb_and_linear_use_exactly_the_curve() {
        for v in [
            [0.0_f32, 0.25, 1.0],
            [0.5, 0.04045, 0.003],
            [-0.5, 1.75, 0.9],
        ] {
            let linear = convert_rgb(Srgb, LinearSrgb, v);
            assert_eq!(bits3(linear), bits3(v.map(|c| decode(f64::from(c)) as f32)));
            let back = convert_rgb(LinearSrgb, Srgb, v);
            assert_eq!(bits3(back), bits3(v.map(|c| encode(f64::from(c)) as f32)));
        }
    }

    #[test]
    fn display_p3_to_linear_srgb_is_decode_then_matrix() {
        let v = [1.0_f32, 0.25, 0.5];
        let lin = v.map(|c| decode(f64::from(c)));
        let expected = mul_vec(&P3_TO_SRGB, lin).map(|c| c as f32);
        assert_eq!(
            bits3(convert_rgb(DisplayP3, LinearSrgb, v)),
            bits3(expected)
        );
    }

    #[test]
    fn linear_srgb_to_display_p3_is_matrix_then_encode() {
        let v = [0.5_f32, 0.25, 0.75];
        let lin = v.map(f64::from);
        let expected = mul_vec(&SRGB_TO_P3, lin).map(|c| encode(c) as f32);
        assert_eq!(
            bits3(convert_rgb(LinearSrgb, DisplayP3, v)),
            bits3(expected)
        );
    }

    #[test]
    fn same_space_is_bit_identical_even_for_nan_and_negative_zero() {
        let samples = [
            [f32::NAN, -0.0, 2.0],
            [-1.0, f32::INFINITY, f32::NEG_INFINITY],
            [1e-45, -1e-45, f32::MAX],
        ];
        for space in [Srgb, LinearSrgb, DisplayP3] {
            for v in samples {
                assert_eq!(bits3(convert_rgb(space, space, v)), bits3(v), "{space:?}");
            }
        }
    }

    // ---- convert_rgb: edge values -----------------------------------------

    #[test]
    fn signed_zero_and_subnormals_through_the_curve() {
        let out = convert_rgb(Srgb, LinearSrgb, [-0.0, 1e-45, -1e-45]);
        assert!(out[0].is_sign_negative() && out[0] == 0.0);
        assert!(out[1].is_finite() && out[1].abs() < 2e-6);
        assert!(out[2].is_finite() && out[2].abs() < 2e-6);
        let back = convert_rgb(LinearSrgb, Srgb, [-0.0, 1e-45, -1e-45]);
        assert!(back[0].is_sign_negative() && back[0] == 0.0);
        assert!(back[1].abs() < 2e-6 && back[2].abs() < 2e-6);
    }

    #[test]
    fn negative_input_mirrors_the_positive_one() {
        let pos = convert_rgb(Srgb, LinearSrgb, [0.5, 0.01, 2.0]);
        let neg = convert_rgb(Srgb, LinearSrgb, [-0.5, -0.01, -2.0]);
        assert_eq!(bits3(neg), bits3(pos.map(|c| -c)));
    }

    #[test]
    fn out_of_range_values_are_conserved_not_clamped() {
        let out = convert_rgb(Srgb, LinearSrgb, [2.0, -0.25, 1e30]);
        assert!(out[0].is_finite() && out[0] > 1.0, "{out:?}");
        assert!(out[1] < 0.0, "{out:?}");
        // 1e30 only has to survive without panicking; f32 may overflow to inf.
        assert!(!out[2].is_nan());
        let thresholds = convert_rgb(Srgb, LinearSrgb, [0.04045, 0.0, 0.0]);
        assert!((f64::from(thresholds[0]) - 0.04045 / 12.92).abs() < 1e-7);
    }

    #[test]
    fn non_finite_input_never_panics_for_any_pair() {
        let specials = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.5];
        let mut converted = 0_u32;
        for src in ColorSpaceId::ALL {
            for dst in ColorSpaceId::ALL {
                for r in specials {
                    for g in specials {
                        for b in specials {
                            let _ = convert_rgb(src, dst, [r, g, b]);
                            converted += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(converted, 9 * 64);
    }

    #[test]
    fn non_finite_channels_stay_non_finite_in_a_curve_only_pair() {
        let out = convert_rgb(Srgb, LinearSrgb, [f32::NAN, f32::INFINITY, 0.5]);
        assert!(out[0].is_nan());
        assert!(out[1].is_infinite());
        assert!(out[2].is_finite());
    }
}
