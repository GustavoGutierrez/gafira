//! Straight-alpha `f32` RGBA value types, one per supported color space.

use serde::{Deserialize, Serialize};

use crate::ColorSpaceId;
use crate::math::convert_rgb;

/// Defines one RGBA value type.
///
/// The types hold no invariant: construction never validates and conversions
/// never clamp. Equality is IEEE (`NaN != NaN`, `-0.0 == 0.0`); compare
/// `to_bits()` when identity is meant.
macro_rules! rgba_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
        pub struct $name {
            /// Red channel.
            pub r: f32,
            /// Green channel.
            pub g: f32,
            /// Blue channel.
            pub b: f32,
            /// Alpha channel (straight, never converted).
            pub a: f32,
        }

        impl $name {
            /// Builds a value without validation; every input is stored unchanged.
            pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
                Self { r, g, b, a }
            }

            /// The color channels without alpha.
            pub(crate) const fn rgb(self) -> [f32; 3] {
                [self.r, self.g, self.b]
            }

            /// True only when all four channels are finite.
            pub fn is_finite(self) -> bool {
                self.r.is_finite() && self.g.is_finite() && self.b.is_finite() && self.a.is_finite()
            }
        }
    };
}

rgba_type! {
    /// Encoded (gamma-compressed) sRGB color.
    Rgba
}

rgba_type! {
    /// Linear-light color on the sRGB primaries.
    LinearRgba
}

rgba_type! {
    /// Encoded (gamma-compressed) Display-P3 color.
    DisplayP3Rgba
}

impl Rgba {
    /// Decodes to linear light with the sRGB transfer curve. Alpha is untouched.
    pub fn to_linear(self) -> LinearRgba {
        let [r, g, b] = convert_rgb(ColorSpaceId::Srgb, ColorSpaceId::LinearSrgb, self.rgb());
        LinearRgba::new(r, g, b, self.a)
    }
}

impl LinearRgba {
    /// Encodes with the sRGB transfer curve. Alpha is untouched.
    pub fn to_srgb(self) -> Rgba {
        let [r, g, b] = convert_rgb(ColorSpaceId::LinearSrgb, ColorSpaceId::Srgb, self.rgb());
        Rgba::new(r, g, b, self.a)
    }

    /// Re-expresses the color in encoded Display-P3. Out-of-gamut results are
    /// kept (never clamped). Alpha is untouched.
    pub fn to_display_p3(self) -> DisplayP3Rgba {
        let [r, g, b] = convert_rgb(
            ColorSpaceId::LinearSrgb,
            ColorSpaceId::DisplayP3,
            self.rgb(),
        );
        DisplayP3Rgba::new(r, g, b, self.a)
    }
}

impl DisplayP3Rgba {
    /// Decodes to linear light on the sRGB primaries. Out-of-gamut results are
    /// kept (never clamped). Alpha is untouched.
    pub fn to_linear_srgb(self) -> LinearRgba {
        let [r, g, b] = convert_rgb(
            ColorSpaceId::DisplayP3,
            ColorSpaceId::LinearSrgb,
            self.rgb(),
        );
        LinearRgba::new(r, g, b, self.a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generates the same type-level checks for each channel type.
    macro_rules! channel_type_tests {
        ($($module:ident => $ty:ident),+ $(,)?) => {$(
            mod $module {
                use super::*;

                const NON_FINITE: [f32; 3] = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY];

                fn bits(v: $ty) -> [u32; 4] {
                    [v.r.to_bits(), v.g.to_bits(), v.b.to_bits(), v.a.to_bits()]
                }

                /// `values` placed into channel `slot`, with 0.25 elsewhere.
                fn with_channel(slot: usize, value: f32) -> $ty {
                    let mut c = [0.25_f32; 4];
                    c[slot] = value;
                    $ty::new(c[0], c[1], c[2], c[3])
                }

                #[test]
                fn values_are_retained_bit_for_bit() {
                    let v = $ty::new(0.25, 0.5, 0.75, 1.0);
                    assert_eq!(
                        bits(v),
                        [0.25_f32.to_bits(), 0.5_f32.to_bits(), 0.75_f32.to_bits(), 1.0_f32.to_bits()]
                    );
                }

                #[test]
                fn construction_does_not_validate() {
                    for value in [-0.5_f32, 1.5, f32::NAN, f32::INFINITY, -0.0, 2.0, -1.0] {
                        for slot in 0..4 {
                            let v = with_channel(slot, value);
                            let read = [v.r, v.g, v.b, v.a];
                            assert_eq!(read[slot].to_bits(), value.to_bits());
                            assert_eq!(v.is_finite(), value.is_finite());
                        }
                    }
                }

                #[test]
                fn new_is_const() {
                    const V: $ty = $ty::new(-1.0, 2.0, 0.0, 1.0);
                    assert_eq!(bits(V)[0], (-1.0_f32).to_bits());
                    assert_eq!(bits(V)[1], 2.0_f32.to_bits());
                }

                #[test]
                fn is_finite_is_false_for_non_finite_in_any_channel() {
                    for value in NON_FINITE {
                        for slot in 0..4 {
                            assert!(
                                !with_channel(slot, value).is_finite(),
                                "slot {slot} value {value}"
                            );
                        }
                    }
                    assert!($ty::new(0.0, -0.0, f32::MAX, f32::MIN_POSITIVE / 2.0).is_finite());
                }

                #[test]
                fn equality_follows_ieee() {
                    let positive = $ty::new(0.0, 0.5, 0.5, 1.0);
                    let negative = $ty::new(-0.0, 0.5, 0.5, 1.0);
                    assert_eq!(positive, negative);
                    assert_ne!(positive.r.to_bits(), negative.r.to_bits());

                    let nan = $ty::new(f32::NAN, 0.5, 0.5, 1.0);
                    assert_ne!(nan, nan);
                    assert_ne!(positive, $ty::new(0.0, 0.5, 0.5, 0.5));
                }
            }
        )+};
    }

    channel_type_tests!(srgb => Rgba, linear => LinearRgba, display_p3 => DisplayP3Rgba);
}

#[cfg(test)]
mod conversion_tests {
    use super::*;

    fn close(actual: f32, expected: f64, tol: f64) -> bool {
        (f64::from(actual) - expected).abs() <= tol * expected.abs().max(1.0)
    }

    const ALPHAS: [f32; 6] = [0.0, -0.0, 0.3, 1.5, f32::NAN, 1e-45];

    #[test]
    fn srgb_to_linear_matches_the_reference_curve() {
        // Independent f64 evaluation: decode(0.5) = 0.214041140.
        let linear = Rgba::new(0.5, 0.0, 1.0, 0.3).to_linear();
        assert!(close(linear.r, 0.214_041_14, 2e-6), "{linear:?}");
        assert!(close(linear.g, 0.0, 2e-6));
        assert!(close(linear.b, 1.0, 2e-6));
        assert_eq!(linear.a.to_bits(), 0.3_f32.to_bits());
    }

    #[test]
    fn linear_to_srgb_matches_the_reference_curve() {
        let encoded = LinearRgba::new(0.214_041_14, 0.0, 1.0, 0.7).to_srgb();
        assert!(close(encoded.r, 0.5, 2e-6), "{encoded:?}");
        assert!(close(encoded.g, 0.0, 2e-6));
        assert!(close(encoded.b, 1.0, 2e-6));
        assert_eq!(encoded.a.to_bits(), 0.7_f32.to_bits());
    }

    #[test]
    fn linear_srgb_red_in_display_p3() {
        // Independent f64 evaluation: (0.917488, 0.200287, 0.138561).
        let p3 = LinearRgba::new(1.0, 0.0, 0.0, 1.0).to_display_p3();
        assert!(close(p3.r, 0.917_488, 1e-4), "{p3:?}");
        assert!(close(p3.g, 0.200_287, 1e-4));
        assert!(close(p3.b, 0.138_561, 1e-4));
    }

    #[test]
    fn display_p3_red_in_linear_srgb_is_out_of_range() {
        // Independent f64 evaluation: (1.224940, -0.042057, -0.019638).
        let linear = DisplayP3Rgba::new(1.0, 0.0, 0.0, 1.0).to_linear_srgb();
        assert!(close(linear.r, 1.224_940, 1e-4), "{linear:?}");
        assert!(close(linear.g, -0.042_057, 1e-4));
        assert!(close(linear.b, -0.019_638, 1e-4));
    }

    #[test]
    fn alpha_is_bit_identical_after_every_typed_conversion() {
        for a in ALPHAS {
            assert_eq!(
                Rgba::new(0.2, 0.4, 0.6, a).to_linear().a.to_bits(),
                a.to_bits()
            );
            let linear = LinearRgba::new(0.2, 0.4, 0.6, a);
            assert_eq!(linear.to_srgb().a.to_bits(), a.to_bits());
            assert_eq!(linear.to_display_p3().a.to_bits(), a.to_bits());
            let p3 = DisplayP3Rgba::new(0.2, 0.4, 0.6, a);
            assert_eq!(p3.to_linear_srgb().a.to_bits(), a.to_bits());
        }
    }

    #[test]
    fn out_of_gamut_input_is_conserved_not_clamped() {
        let linear = Rgba::new(-0.25, 1.75, 0.5, 1.0).to_linear();
        assert!(linear.is_finite());
        assert!(linear.r < 0.0 && linear.g > 1.0, "{linear:?}");
        let p3 = linear.to_display_p3();
        assert!(p3.is_finite(), "{p3:?}");
        assert_eq!(linear.a.to_bits(), 1.0_f32.to_bits());
    }

    #[test]
    fn non_finite_channels_never_panic_and_keep_alpha() {
        let specials = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY];
        let mut converted = 0_u32;
        for s in specials {
            for slot in 0..3 {
                let mut c = [0.5_f32; 3];
                c[slot] = s;
                let a = 0.25_f32;
                let srgb = Rgba::new(c[0], c[1], c[2], a).to_linear();
                let linear = LinearRgba::new(c[0], c[1], c[2], a);
                let p3 = DisplayP3Rgba::new(c[0], c[1], c[2], a).to_linear_srgb();
                for alpha in [srgb.a, linear.to_srgb().a, linear.to_display_p3().a, p3.a] {
                    assert_eq!(alpha.to_bits(), a.to_bits());
                    converted += 1;
                }
            }
        }
        assert_eq!(converted, 36);
    }

    #[test]
    fn grays_stay_gray_through_display_p3() {
        let p3 = Rgba::new(0.5, 0.5, 0.5, 1.0).to_linear().to_display_p3();
        assert!(
            (p3.r - p3.g).abs() < 1e-5 && (p3.g - p3.b).abs() < 1e-5,
            "{p3:?}"
        );
        assert!(close(p3.r, 0.5, 1e-5));
    }
}
