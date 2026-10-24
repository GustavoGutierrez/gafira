//! Straight-alpha `f32` RGBA value types, one per supported color space.

use serde::{Deserialize, Serialize};

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
