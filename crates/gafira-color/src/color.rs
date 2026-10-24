//! The wire-level color: a value tagged with its color space.

use serde::{Deserialize, Serialize};

use crate::math::convert_rgb;
use crate::{ColorSpaceId, DisplayP3Rgba, LinearRgba, Rgba};

/// A color in one of the supported spaces.
///
/// Serialized as one flat object, `space` first, then `r`, `g`, `b`, `a`:
/// `{"space":"srgb","r":1.0,"g":0.5,"b":0.0,"a":1.0}`. An unknown or missing
/// `space` fails to deserialize; extra fields are ignored. The enum is
/// exhaustive on purpose: adding a space makes the compiler flag every `match`.
///
/// Equality is IEEE and structural, so `Srgb(x) != LinearSrgb(x)`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "space")]
pub enum Color {
    /// Encoded sRGB.
    #[serde(rename = "srgb")]
    Srgb(Rgba),
    /// Linear-light sRGB primaries.
    #[serde(rename = "srgb-linear")]
    LinearSrgb(LinearRgba),
    /// Encoded Display-P3.
    #[serde(rename = "display-p3")]
    DisplayP3(DisplayP3Rgba),
}

impl Color {
    /// The space this color is expressed in.
    pub const fn space(self) -> ColorSpaceId {
        match self {
            Color::Srgb(_) => ColorSpaceId::Srgb,
            Color::LinearSrgb(_) => ColorSpaceId::LinearSrgb,
            Color::DisplayP3(_) => ColorSpaceId::DisplayP3,
        }
    }

    /// The straight alpha channel.
    pub const fn alpha(self) -> f32 {
        match self {
            Color::Srgb(c) => c.a,
            Color::LinearSrgb(c) => c.a,
            Color::DisplayP3(c) => c.a,
        }
    }

    /// True only when all four channels are finite.
    pub fn is_finite(self) -> bool {
        match self {
            Color::Srgb(c) => c.is_finite(),
            Color::LinearSrgb(c) => c.is_finite(),
            Color::DisplayP3(c) => c.is_finite(),
        }
    }

    /// Converts to `to`. Never clamps; alpha is copied bit for bit, and
    /// converting to the same space returns a bit-identical value.
    pub fn convert(self, to: ColorSpaceId) -> Color {
        let (rgb, a) = self.parts();
        let [r, g, b] = convert_rgb(self.space(), to, rgb);
        match to {
            ColorSpaceId::Srgb => Color::Srgb(Rgba::new(r, g, b, a)),
            ColorSpaceId::LinearSrgb => Color::LinearSrgb(LinearRgba::new(r, g, b, a)),
            ColorSpaceId::DisplayP3 => Color::DisplayP3(DisplayP3Rgba::new(r, g, b, a)),
        }
    }

    /// Converts to linear-light sRGB.
    pub fn to_linear_srgb(self) -> LinearRgba {
        let (rgb, a) = self.parts();
        let [r, g, b] = convert_rgb(self.space(), ColorSpaceId::LinearSrgb, rgb);
        LinearRgba::new(r, g, b, a)
    }

    /// Color channels and alpha, whatever the space.
    fn parts(self) -> ([f32; 3], f32) {
        match self {
            Color::Srgb(c) => (c.rgb(), c.a),
            Color::LinearSrgb(c) => (c.rgb(), c.a),
            Color::DisplayP3(c) => (c.rgb(), c.a),
        }
    }
}

impl From<Rgba> for Color {
    fn from(value: Rgba) -> Self {
        Color::Srgb(value)
    }
}

impl From<LinearRgba> for Color {
    fn from(value: LinearRgba) -> Self {
        Color::LinearSrgb(value)
    }
}

impl From<DisplayP3Rgba> for Color {
    fn from(value: DisplayP3Rgba) -> Self {
        Color::DisplayP3(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColorSpaceId, DisplayP3Rgba, LinearRgba, Rgba};

    fn channels(c: Color) -> [f32; 4] {
        match c {
            Color::Srgb(v) => [v.r, v.g, v.b, v.a],
            Color::LinearSrgb(v) => [v.r, v.g, v.b, v.a],
            Color::DisplayP3(v) => [v.r, v.g, v.b, v.a],
        }
    }

    fn bits(c: Color) -> [u32; 4] {
        channels(c).map(f32::to_bits)
    }

    fn build(space: ColorSpaceId, c: [f32; 4]) -> Color {
        match space {
            ColorSpaceId::Srgb => Rgba::new(c[0], c[1], c[2], c[3]).into(),
            ColorSpaceId::LinearSrgb => LinearRgba::new(c[0], c[1], c[2], c[3]).into(),
            ColorSpaceId::DisplayP3 => DisplayP3Rgba::new(c[0], c[1], c[2], c[3]).into(),
        }
    }

    fn close_all(actual: Color, expected: [f64; 3], tol: f64) {
        let got = channels(actual);
        for i in 0..3 {
            assert!(
                (f64::from(got[i]) - expected[i]).abs() <= tol * expected[i].abs().max(1.0),
                "{actual:?} vs {expected:?}"
            );
        }
    }

    #[test]
    fn from_impls_pick_the_matching_variant() {
        let c = [0.1, 0.2, 0.3, 0.4];
        let srgb = Color::from(Rgba::new(c[0], c[1], c[2], c[3]));
        assert!(matches!(srgb, Color::Srgb(_)));
        assert_eq!(bits(srgb), c.map(f32::to_bits));
        let linear = Color::from(LinearRgba::new(c[0], c[1], c[2], c[3]));
        assert!(matches!(linear, Color::LinearSrgb(_)));
        assert_eq!(bits(linear), c.map(f32::to_bits));
        let p3 = Color::from(DisplayP3Rgba::new(c[0], c[1], c[2], c[3]));
        assert!(matches!(p3, Color::DisplayP3(_)));
        assert_eq!(bits(p3), c.map(f32::to_bits));
    }

    #[test]
    fn space_and_alpha_report_the_variant() {
        for space in ColorSpaceId::ALL {
            let color = build(space, [0.1, 0.2, 0.3, 0.4]);
            assert_eq!(color.space(), space);
            assert_eq!(color.alpha().to_bits(), 0.4_f32.to_bits());
        }
        const C: Color = Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 0.5));
        const SPACE: ColorSpaceId = C.space();
        assert_eq!(SPACE, ColorSpaceId::Srgb);
    }

    #[test]
    fn is_finite_looks_at_every_channel_of_every_variant() {
        for space in ColorSpaceId::ALL {
            assert!(build(space, [0.0, -1.0, 2.0, 1.0]).is_finite());
            for slot in 0..4 {
                for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                    let mut c = [0.5_f32; 4];
                    c[slot] = bad;
                    assert!(!build(space, c).is_finite(), "{space:?} slot {slot}");
                }
            }
        }
    }

    #[test]
    fn variants_are_never_equal_for_identical_channels() {
        let c = [0.25, 0.5, 0.75, 1.0];
        let srgb = build(ColorSpaceId::Srgb, c);
        let linear = build(ColorSpaceId::LinearSrgb, c);
        let p3 = build(ColorSpaceId::DisplayP3, c);
        assert_ne!(srgb, linear);
        assert_ne!(linear, p3);
        assert_ne!(srgb, p3);
        assert_eq!(srgb, build(ColorSpaceId::Srgb, c));
    }

    #[test]
    fn exhaustive_match_over_color_has_no_wildcard() {
        fn tag(c: Color) -> ColorSpaceId {
            match c {
                Color::Srgb(_) => ColorSpaceId::Srgb,
                Color::LinearSrgb(_) => ColorSpaceId::LinearSrgb,
                Color::DisplayP3(_) => ColorSpaceId::DisplayP3,
            }
        }
        for space in ColorSpaceId::ALL {
            assert_eq!(tag(build(space, [0.0; 4])), space);
        }
    }

    #[test]
    fn convert_reaches_every_target_with_the_right_variant() {
        let mut checked = 0;
        for src in ColorSpaceId::ALL {
            for dst in ColorSpaceId::ALL {
                let out = build(src, [0.2, 0.5, 0.8, 0.6]).convert(dst);
                assert_eq!(out.space(), dst, "{src:?} -> {dst:?}");
                assert_eq!(out.alpha().to_bits(), 0.6_f32.to_bits());
                checked += 1;
            }
        }
        assert_eq!(checked, 9);
    }

    #[test]
    fn all_nine_pairs_round_trip_within_tolerance() {
        for a in ColorSpaceId::ALL {
            for b in ColorSpaceId::ALL {
                let original = build(a, [0.2, 0.5, 0.8, 1.0]);
                let back = original.convert(b).convert(a);
                close_all(back, [0.2, 0.5, 0.8], 1e-5);
            }
        }
    }

    #[test]
    fn convert_matches_the_published_references() {
        let red = build(ColorSpaceId::Srgb, [1.0, 0.0, 0.0, 1.0]);
        close_all(
            red.convert(ColorSpaceId::DisplayP3),
            [0.91749, 0.20029, 0.13856],
            1e-4,
        );
        let p3_red = build(ColorSpaceId::DisplayP3, [1.0, 0.0, 0.0, 1.0]);
        close_all(
            p3_red.convert(ColorSpaceId::Srgb),
            [1.0931, -0.2267, -0.1501],
            1e-4,
        );
        let out = p3_red.convert(ColorSpaceId::Srgb);
        assert!(channels(out)[0] > 1.0 && channels(out)[1] < 0.0, "{out:?}");
    }

    #[test]
    fn same_space_convert_is_bit_identical() {
        let samples = [
            [f32::NAN, -0.0, 2.0, f32::NAN],
            [-1.0, f32::INFINITY, f32::NEG_INFINITY, -0.0],
            [1e-45, -1e-45, f32::MAX, 1.5],
        ];
        for space in ColorSpaceId::ALL {
            for c in samples {
                let color = build(space, c);
                assert_eq!(bits(color.convert(space)), bits(color), "{space:?}");
            }
        }
    }

    #[test]
    fn non_finite_channels_convert_without_panic_and_keep_alpha() {
        let specials = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.5];
        let mut converted = 0_u32;
        for src in ColorSpaceId::ALL {
            for dst in ColorSpaceId::ALL {
                for r in specials {
                    for g in specials {
                        let alpha = f32::NAN;
                        let out = build(src, [r, g, 0.5, alpha]).convert(dst);
                        assert_eq!(out.alpha().to_bits(), alpha.to_bits());
                        converted += 1;
                    }
                }
            }
        }
        assert_eq!(converted, 9 * 16);
    }

    #[test]
    fn to_linear_srgb_agrees_with_the_typed_conversions() {
        fn linear_bits(v: LinearRgba) -> [u32; 4] {
            [v.r, v.g, v.b, v.a].map(f32::to_bits)
        }
        let c = [0.3, 0.6, 0.9, 0.5];
        let srgb = Rgba::new(c[0], c[1], c[2], c[3]);
        assert_eq!(
            linear_bits(Color::from(srgb).to_linear_srgb()),
            linear_bits(srgb.to_linear())
        );
        let linear = LinearRgba::new(c[0], c[1], c[2], c[3]);
        assert_eq!(
            linear_bits(Color::from(linear).to_linear_srgb()),
            linear_bits(linear)
        );
        let p3 = DisplayP3Rgba::new(c[0], c[1], c[2], c[3]);
        assert_eq!(
            linear_bits(Color::from(p3).to_linear_srgb()),
            linear_bits(p3.to_linear_srgb())
        );
    }

    #[test]
    fn out_of_gamut_color_converts_to_finite_unclamped_values() {
        let out =
            build(ColorSpaceId::Srgb, [-0.25, 1.75, 0.5, 1.0]).convert(ColorSpaceId::DisplayP3);
        assert!(out.is_finite());
        assert!(
            channels(out).iter().any(|v| *v < 0.0 || *v > 1.0),
            "{out:?}"
        );
    }
}
