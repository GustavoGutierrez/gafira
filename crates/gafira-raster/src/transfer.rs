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
