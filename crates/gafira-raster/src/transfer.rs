//! sRGB transfer curve through the layer-0 transform (ARQ-121, P15 item 8).

use gafira_color::{AnalyticTransform, ColorSpaceId, ColorTransform, PixelLayout};

/// sRGB to linear light and back, through the layer-0 transform.
pub(crate) struct Transfer {
    /// Linear value of each 8-bit encoded value.
    pub(crate) decode: [f64; 256],
    to_srgb: AnalyticTransform,
}

impl Transfer {
    pub(crate) fn new() -> Self {
        let to_linear = AnalyticTransform::new(ColorSpaceId::Srgb, ColorSpaceId::LinearSrgb);
        let mut decode = [0.0; 256];
        for (v, slot) in (0u8..=255).zip(&mut decode) {
            let c = f32::from(v) / 255.0;
            let mut px = [c, c, c, 1.0];
            // A four-channel buffer is always whole, so this cannot fail.
            let _ = to_linear.apply_f32(&mut px, PixelLayout::Rgba);
            *slot = f64::from(px[0]);
        }
        Self {
            decode,
            to_srgb: AnalyticTransform::new(ColorSpaceId::LinearSrgb, ColorSpaceId::Srgb),
        }
    }

    /// Linear light to the nearest encoded byte.
    pub(crate) fn encode(&self, linear: [f64; 3]) -> [u8; 3] {
        let e = self.encode_unit(linear);
        let q = |v: f64| (v * 255.0 + 0.5).floor() as u8;
        [q(e[0]), q(e[1]), q(e[2])]
    }

    /// Linear light to the encoded value in `[0, 1]`, not rounded.
    pub(crate) fn encode_unit(&self, linear: [f64; 3]) -> [f64; 3] {
        let mut px = [linear[0] as f32, linear[1] as f32, linear[2] as f32, 1.0];
        let _ = self.to_srgb.apply_f32(&mut px, PixelLayout::Rgba);
        [0, 1, 2].map(|i| f64::from(px[i]).clamp(0.0, 1.0))
    }

    /// Linear light to the encoded value in `f64`, any finite input, odd about zero (CSS Color 4).
    pub(crate) fn encode_f64(&self, linear: f64) -> f64 {
        let a = linear.abs();
        let e = if a <= 0.003_130_8 {
            12.92 * a
        } else {
            1.055 * a.powf(1.0 / 2.4) - 0.055
        };
        e.copysign(linear)
    }

    /// Encoded value to linear light in `f64`, any finite input, odd about zero (CSS Color 4).
    pub(crate) fn decode_f64(&self, encoded: f64) -> f64 {
        let a = encoded.abs();
        let l = if a <= 0.040_45 {
            a / 12.92
        } else {
            ((a + 0.055) / 1.055).powf(2.4)
        };
        l.copysign(encoded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // W6: the `f64` curves agree with the layer-0 transform over a dense sweep, both ways, and are
    // odd about zero, so a change of a constant in either is caught.
    #[test]
    fn the_f64_curves_agree_with_the_analytic_transform() {
        let t = Transfer::new();
        let to_linear = AnalyticTransform::new(ColorSpaceId::Srgb, ColorSpaceId::LinearSrgb);
        let to_srgb = AnalyticTransform::new(ColorSpaceId::LinearSrgb, ColorSpaceId::Srgb);
        let mut checked = 0;
        for i in -2000..=3000 {
            let v = f64::from(i) / 1000.0; // -2 to 3, with the joins at 0.0031308 and 0.04045 inside
            let mut px = [v as f32, v as f32, v as f32, 1.0];
            let _ = to_srgb.apply_f32(&mut px, PixelLayout::Rgba);
            assert!(
                (t.encode_f64(v) - f64::from(px[0])).abs() < 1e-6,
                "encode {v}: {} vs {}",
                t.encode_f64(v),
                px[0]
            );
            let mut px = [v as f32, v as f32, v as f32, 1.0];
            let _ = to_linear.apply_f32(&mut px, PixelLayout::Rgba);
            assert!(
                (t.decode_f64(v) - f64::from(px[0])).abs() < 1e-5 * (1.0 + v.abs().powf(2.4)),
                "decode {v}: {} vs {}",
                t.decode_f64(v),
                px[0]
            );
            checked += 1;
        }
        assert_eq!(checked, 5001);
        // The joins of both curves, just either side, and the fixed points.
        for v in [0.003_130_7, 0.003_130_9, 0.040_44, 0.040_46, 0.0, 1.0] {
            let mut px = [v as f32, 0.0, 0.0, 1.0];
            let _ = to_srgb.apply_f32(&mut px, PixelLayout::Rgba);
            assert!(
                (t.encode_f64(v) - f64::from(px[0])).abs() < 1e-6,
                "encode join {v}"
            );
            let mut px = [v as f32, 0.0, 0.0, 1.0];
            let _ = to_linear.apply_f32(&mut px, PixelLayout::Rgba);
            assert!(
                (t.decode_f64(v) - f64::from(px[0])).abs() < 1e-6,
                "decode join {v}"
            );
        }
        // Round trip, and odd about zero.
        for v in [0.1, 0.5, 2.5, 1.0e6] {
            assert!((t.decode_f64(t.encode_f64(v)) - v).abs() < 1e-9 * v.max(1.0));
            assert_eq!(t.encode_f64(-v), -t.encode_f64(v));
            assert_eq!(t.decode_f64(-v), -t.decode_f64(v));
        }
    }
}
