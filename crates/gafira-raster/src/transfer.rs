//! sRGB transfer curve through the layer-0 transform (ARQ-121, P15 item 8).

use std::sync::OnceLock;

use gafira_color::{AnalyticTransform, ColorSpaceId, ColorTransform, PixelLayout};

/// sRGB to linear light and back, through the layer-0 transform.
pub(crate) struct Transfer {
    /// Linear value of each 8-bit encoded value.
    pub(crate) decode: [f64; 256],
    to_srgb: AnalyticTransform,
    to_linear: AnalyticTransform,
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
            to_linear,
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

    /// Linear light to the encoded value, any `f32` (no clamp).
    pub(crate) fn encode_f32(&self, linear: [f32; 3]) -> [f32; 3] {
        let mut px = [linear[0], linear[1], linear[2], 1.0];
        let _ = self.to_srgb.apply_f32(&mut px, PixelLayout::Rgba);
        [px[0], px[1], px[2]]
    }

    /// Encoded value to linear light, any `f32` (no clamp).
    pub(crate) fn decode_f32(&self, encoded: [f32; 3]) -> [f32; 3] {
        let mut px = [encoded[0], encoded[1], encoded[2], 1.0];
        let _ = self.to_linear.apply_f32(&mut px, PixelLayout::Rgba);
        [px[0], px[1], px[2]]
    }
}

/// The shared tables, built once.
pub(crate) fn transfer() -> &'static Transfer {
    static TRANSFER: OnceLock<Transfer> = OnceLock::new();
    TRANSFER.get_or_init(Transfer::new)
}
