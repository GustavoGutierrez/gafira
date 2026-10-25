//! In-place pixel-buffer conversion between color spaces.

use std::fmt;

use crate::ColorSpaceId;
use crate::math::convert_rgb;

/// Interleaved `f32` pixel layout. Premultiplied and planar layouts are out of scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PixelLayout {
    /// Three channels per pixel.
    Rgb,
    /// Four channels per pixel, straight (non-premultiplied) alpha.
    Rgba,
}

impl PixelLayout {
    /// Number of `f32` channels per pixel.
    pub const fn channels(self) -> usize {
        match self {
            PixelLayout::Rgb => 3,
            PixelLayout::Rgba => 4,
        }
    }
}

/// A pixel buffer that does not hold a whole number of pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PixelBufferError {
    /// `len` is not a multiple of `channels`.
    LengthNotMultiple {
        /// Length of the rejected buffer.
        len: usize,
        /// Channels per pixel of the requested layout.
        channels: usize,
    },
}

impl fmt::Display for PixelBufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PixelBufferError::LengthNotMultiple { len, channels } => write!(
                f,
                "pixel buffer length {len} is not a multiple of {channels} channels"
            ),
        }
    }
}

impl std::error::Error for PixelBufferError {}

/// Converts interleaved pixels in place from [`ColorTransform::src`] to [`ColorTransform::dst`].
///
/// Documented deviation from 02 §4.10, which returns `()`: a malformed buffer is an
/// error value, never a panic.
pub trait ColorTransform: Send + Sync {
    /// Source color space.
    fn src(&self) -> ColorSpaceId;

    /// Destination color space.
    fn dst(&self) -> ColorSpaceId;

    /// Converts `px` in place. Alpha is untouched for [`PixelLayout::Rgba`].
    ///
    /// # Errors
    ///
    /// [`PixelBufferError::LengthNotMultiple`] when `px.len()` is not a multiple of
    /// `layout.channels()`; the buffer is then left unmodified.
    fn apply_f32(&self, px: &mut [f32], layout: PixelLayout) -> Result<(), PixelBufferError>;
}

/// Closed-form transform between any two of the three supported spaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnalyticTransform {
    src: ColorSpaceId,
    dst: ColorSpaceId,
}

impl AnalyticTransform {
    /// Every ordered pair is supported, so construction cannot fail.
    pub const fn new(src: ColorSpaceId, dst: ColorSpaceId) -> Self {
        Self { src, dst }
    }
}

impl ColorTransform for AnalyticTransform {
    fn src(&self) -> ColorSpaceId {
        self.src
    }

    fn dst(&self) -> ColorSpaceId {
        self.dst
    }

    fn apply_f32(&self, px: &mut [f32], layout: PixelLayout) -> Result<(), PixelBufferError> {
        let channels = layout.channels();
        if !px.len().is_multiple_of(channels) {
            return Err(PixelBufferError::LengthNotMultiple {
                len: px.len(),
                channels,
            });
        }
        for pixel in px.chunks_exact_mut(channels) {
            let [r, g, b] = convert_rgb(self.src, self.dst, [pixel[0], pixel[1], pixel[2]]);
            pixel[..3].copy_from_slice(&[r, g, b]);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, DisplayP3Rgba, LinearRgba, Rgba};

    const RGBA: [f32; 12] = [
        0.0, 0.25, 1.0, 0.5, //
        0.5, 0.04045, 0.003, 0.25, //
        -0.5, 1.75, 0.9, 1.0,
    ];

    fn bits(v: &[f32]) -> Vec<u32> {
        v.iter().map(|c| c.to_bits()).collect()
    }

    fn build(space: ColorSpaceId, [r, g, b, a]: [f32; 4]) -> Color {
        match space {
            ColorSpaceId::Srgb => Rgba::new(r, g, b, a).into(),
            ColorSpaceId::LinearSrgb => LinearRgba::new(r, g, b, a).into(),
            ColorSpaceId::DisplayP3 => DisplayP3Rgba::new(r, g, b, a).into(),
        }
    }

    fn channels_of(c: Color) -> [f32; 4] {
        match c {
            Color::Srgb(v) => [v.r, v.g, v.b, v.a],
            Color::LinearSrgb(v) => [v.r, v.g, v.b, v.a],
            Color::DisplayP3(v) => [v.r, v.g, v.b, v.a],
        }
    }

    /// Per-pixel typed conversion of an interleaved buffer.
    fn typed(src: ColorSpaceId, dst: ColorSpaceId, px: &[f32], layout: PixelLayout) -> Vec<f32> {
        px.chunks_exact(layout.channels())
            .flat_map(|p| {
                let a = p.get(3).copied().unwrap_or(1.0);
                let out = channels_of(build(src, [p[0], p[1], p[2], a]).convert(dst));
                out[..layout.channels()].to_vec()
            })
            .collect()
    }

    #[test]
    fn layouts_report_their_channel_counts() {
        assert_eq!(PixelLayout::Rgb.channels(), 3);
        assert_eq!(PixelLayout::Rgba.channels(), 4);
    }

    #[test]
    fn boxed_transform_is_object_safe_send_and_sync() {
        // Threads are forbidden in this layer (ARQ-02), so the bounds are proven at compile time.
        fn send_sync<T: Send + Sync>(_: &T) {}
        let t: Box<dyn ColorTransform> = Box::new(AnalyticTransform::new(
            ColorSpaceId::Srgb,
            ColorSpaceId::LinearSrgb,
        ));
        send_sync(&t);
        let mut px = [0.5_f32, 0.5, 0.5, 1.0];
        assert_eq!(t.apply_f32(&mut px, PixelLayout::Rgba), Ok(()));
        assert!((px[0] - 0.214_041_1).abs() < 2e-6);
    }

    #[test]
    fn all_nine_pairs_match_color_convert_on_rgba() {
        for src in ColorSpaceId::ALL {
            for dst in ColorSpaceId::ALL {
                let t = AnalyticTransform::new(src, dst);
                assert_eq!((t.src(), t.dst()), (src, dst));
                let mut px = RGBA;
                assert_eq!(t.apply_f32(&mut px, PixelLayout::Rgba), Ok(()));
                let expected = typed(src, dst, &RGBA, PixelLayout::Rgba);
                assert_eq!(bits(&px), bits(&expected), "{src:?} -> {dst:?}");
                for i in 0..3 {
                    assert_eq!(px[i * 4 + 3].to_bits(), RGBA[i * 4 + 3].to_bits());
                }
            }
        }
    }

    #[test]
    fn rgb_layout_converts_triples_without_an_alpha_channel() {
        let t = AnalyticTransform::new(ColorSpaceId::Srgb, ColorSpaceId::DisplayP3);
        let mut px = [1.0_f32, 0.0, 0.0, 0.0, 1.0, 0.0];
        let original = px;
        assert_eq!(t.apply_f32(&mut px, PixelLayout::Rgb), Ok(()));
        let expected = typed(
            ColorSpaceId::Srgb,
            ColorSpaceId::DisplayP3,
            &original,
            PixelLayout::Rgb,
        );
        assert_eq!(bits(&px), bits(&expected));
        assert_ne!(bits(&px), bits(&original));
    }

    #[test]
    fn wrong_length_is_an_error_and_leaves_the_buffer_untouched() {
        let t = AnalyticTransform::new(ColorSpaceId::Srgb, ColorSpaceId::LinearSrgb);
        for (len, layout, channels) in [(10, PixelLayout::Rgba, 4), (7, PixelLayout::Rgb, 3)] {
            let mut px: Vec<f32> = (0..len).map(|i| 0.1 + i as f32 / 20.0).collect();
            let original = bits(&px);
            assert_eq!(
                t.apply_f32(&mut px, layout),
                Err(PixelBufferError::LengthNotMultiple { len, channels })
            );
            assert_eq!(bits(&px), original);
        }
    }

    #[test]
    fn empty_buffer_succeeds_for_both_layouts() {
        let t = AnalyticTransform::new(ColorSpaceId::Srgb, ColorSpaceId::LinearSrgb);
        for layout in [PixelLayout::Rgb, PixelLayout::Rgba] {
            let mut px: [f32; 0] = [];
            assert_eq!(t.apply_f32(&mut px, layout), Ok(()));
        }
    }

    #[test]
    fn identity_transform_keeps_every_bit() {
        let mut px = [f32::NAN, -0.0, 2.0, f32::INFINITY, 1e-40, -3.0, 0.5, 0.25];
        let original = bits(&px);
        for space in ColorSpaceId::ALL {
            let t = AnalyticTransform::new(space, space);
            assert_eq!(t.apply_f32(&mut px, PixelLayout::Rgba), Ok(()));
            assert_eq!(bits(&px), original);
        }
    }

    #[test]
    fn error_names_length_and_channels_and_is_a_std_error() {
        let err = PixelBufferError::LengthNotMultiple {
            len: 10,
            channels: 4,
        };
        let dynamic: &dyn std::error::Error = &err;
        let text = dynamic.to_string();
        assert!(text.contains("10") && text.contains('4'), "{text}");
    }
}
