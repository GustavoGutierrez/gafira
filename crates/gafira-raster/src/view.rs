//! Borrowed and owned pixel buffers. Length is an invariant (ARQ-117).

use crate::BufferError;

/// Pixel storage kind. RGBA buffers are premultiplied (ARQ-117).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelFormat {
    /// 8 bits per channel, sRGB-encoded, premultiplied, 4 channels.
    Rgba8,
    /// `f32` per channel, linear light, premultiplied, 4 channels.
    RgbaF32,
    /// 8-bit coverage, 1 channel.
    Alpha8,
}

impl PixelFormat {
    /// Elements per pixel.
    #[must_use]
    pub const fn channels(self) -> usize {
        match self {
            Self::Rgba8 | Self::RgbaF32 => 4,
            Self::Alpha8 => 1,
        }
    }
}

/// Borrowed pixels.
#[derive(Clone, Copy, Debug)]
pub enum PixelData<'a> {
    /// See [`PixelFormat::Rgba8`].
    Rgba8(&'a [u8]),
    /// See [`PixelFormat::RgbaF32`].
    RgbaF32(&'a [f32]),
    /// See [`PixelFormat::Alpha8`].
    Alpha8(&'a [u8]),
}

impl PixelData<'_> {
    /// The storage kind.
    #[must_use]
    pub const fn format(&self) -> PixelFormat {
        match self {
            Self::Rgba8(_) => PixelFormat::Rgba8,
            Self::RgbaF32(_) => PixelFormat::RgbaF32,
            Self::Alpha8(_) => PixelFormat::Alpha8,
        }
    }

    const fn len(&self) -> usize {
        match self {
            Self::Rgba8(d) | Self::Alpha8(d) => d.len(),
            Self::RgbaF32(d) => d.len(),
        }
    }
}

/// Mutably borrowed pixels.
#[derive(Debug)]
pub enum PixelDataMut<'a> {
    /// See [`PixelFormat::Rgba8`].
    Rgba8(&'a mut [u8]),
    /// See [`PixelFormat::RgbaF32`].
    RgbaF32(&'a mut [f32]),
    /// See [`PixelFormat::Alpha8`].
    Alpha8(&'a mut [u8]),
}

/// Checks `len == width × height × channels` without overflow.
fn check_len(width: u32, height: u32, channels: usize, actual: usize) -> Result<(), BufferError> {
    let overflow = BufferError::SizeOverflow { width, height };
    let pixels = usize::try_from(width)
        .ok()
        .and_then(|w| w.checked_mul(usize::try_from(height).ok()?))
        .ok_or(overflow)?;
    let expected = pixels.checked_mul(channels).ok_or(overflow)?;
    if expected == actual {
        Ok(())
    } else {
        Err(BufferError::LengthMismatch { expected, actual })
    }
}

/// A read-only image. The constructor checks length only, not values.
#[derive(Clone, Copy, Debug)]
pub struct ImageView<'a> {
    width: u32,
    height: u32,
    data: PixelData<'a>,
}

impl<'a> ImageView<'a> {
    /// # Errors
    ///
    /// [`BufferError::SizeOverflow`] or [`BufferError::LengthMismatch`].
    pub fn new(width: u32, height: u32, data: PixelData<'a>) -> Result<Self, BufferError> {
        check_len(width, height, data.format().channels(), data.len())?;
        Ok(Self {
            width,
            height,
            data,
        })
    }

    /// Width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// The pixels.
    #[must_use]
    pub const fn data(&self) -> PixelData<'a> {
        self.data
    }
}

/// A writable image. The constructor checks length only, not values.
#[derive(Debug)]
pub struct ImageViewMut<'a> {
    width: u32,
    height: u32,
    data: PixelDataMut<'a>,
}

impl<'a> ImageViewMut<'a> {
    /// # Errors
    ///
    /// [`BufferError::SizeOverflow`] or [`BufferError::LengthMismatch`].
    pub fn new(width: u32, height: u32, data: PixelDataMut<'a>) -> Result<Self, BufferError> {
        let (channels, len) = match &data {
            PixelDataMut::Rgba8(d) => (4, d.len()),
            PixelDataMut::RgbaF32(d) => (4, d.len()),
            PixelDataMut::Alpha8(d) => (1, d.len()),
        };
        check_len(width, height, channels, len)?;
        Ok(Self {
            width,
            height,
            data,
        })
    }

    /// Width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// The pixels, read-only.
    #[must_use]
    pub fn data(&self) -> PixelData<'_> {
        match &self.data {
            PixelDataMut::Rgba8(d) => PixelData::Rgba8(d),
            PixelDataMut::RgbaF32(d) => PixelData::RgbaF32(d),
            PixelDataMut::Alpha8(d) => PixelData::Alpha8(d),
        }
    }

    /// The pixels, writable. The length cannot change.
    pub fn data_mut(&mut self) -> PixelDataMut<'_> {
        match &mut self.data {
            PixelDataMut::Rgba8(d) => PixelDataMut::Rgba8(d),
            PixelDataMut::RgbaF32(d) => PixelDataMut::RgbaF32(d),
            PixelDataMut::Alpha8(d) => PixelDataMut::Alpha8(d),
        }
    }
}

/// An 8-bit coverage mask.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlphaMask {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl AlphaMask {
    /// # Errors
    ///
    /// [`BufferError::SizeOverflow`] or [`BufferError::LengthMismatch`].
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Result<Self, BufferError> {
        check_len(width, height, 1, data.len())?;
        Ok(Self {
            width,
            height,
            data,
        })
    }

    /// Width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// The coverage values, row-major.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

/// Owned pixels.
#[derive(Clone, Debug, PartialEq)]
pub enum PixelBuf {
    /// See [`PixelFormat::Rgba8`].
    Rgba8(Vec<u8>),
    /// See [`PixelFormat::RgbaF32`].
    RgbaF32(Vec<f32>),
    /// See [`PixelFormat::Alpha8`].
    Alpha8(Vec<u8>),
}

/// An owned image, such as a mip level.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageBuf {
    width: u32,
    height: u32,
    data: PixelBuf,
}

impl ImageBuf {
    /// # Errors
    ///
    /// [`BufferError::SizeOverflow`] or [`BufferError::LengthMismatch`].
    pub fn new(width: u32, height: u32, data: PixelBuf) -> Result<Self, BufferError> {
        let (channels, len) = match &data {
            PixelBuf::Rgba8(d) => (4, d.len()),
            PixelBuf::RgbaF32(d) => (4, d.len()),
            PixelBuf::Alpha8(d) => (1, d.len()),
        };
        check_len(width, height, channels, len)?;
        Ok(Self {
            width,
            height,
            data,
        })
    }

    /// Width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// The pixels.
    #[must_use]
    pub const fn data(&self) -> &PixelBuf {
        &self.data
    }

    /// A read-only view of the same pixels.
    #[must_use]
    pub fn view(&self) -> ImageView<'_> {
        let data = match &self.data {
            PixelBuf::Rgba8(d) => PixelData::Rgba8(d),
            PixelBuf::RgbaF32(d) => PixelData::RgbaF32(d),
            PixelBuf::Alpha8(d) => PixelData::Alpha8(d),
        };
        ImageView {
            width: self.width,
            height: self.height,
            data,
        }
    }
}

/// Where `src` and `dst` sit in the full image, in the pixels of `src`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileWindow {
    /// Full image width.
    pub image_width: u32,
    /// Full image height.
    pub image_height: u32,
    /// Column of the first pixel of `src` in the full image.
    pub x: u32,
    /// Row of the first pixel of `src` in the full image.
    pub y: u32,
}

/// Context handed to an effect. Plain data: it does not validate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffectCx {
    /// Logical-to-physical pixel ratio (ARQ-120).
    pub scale: f32,
    /// Set when `src` is a cut of a larger image. `None`: `src` is the whole image.
    pub tile: Option<TileWindow>,
}

impl EffectCx {
    /// A whole-image run.
    #[must_use]
    pub fn whole(scale: f32) -> Self {
        Self { scale, tile: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ARQ-117: length must equal width x height x channels, for every format.
    #[test]
    fn arq_117_length_mismatch_is_err_for_each_format() {
        let cases: [(u32, u32, usize, usize); 6] = [
            (2, 3, 4, 24),
            (2, 3, 4, 23),
            (2, 3, 4, 25),
            (0, 5, 4, 0),
            (5, 0, 4, 0),
            (1, 1, 4, 0),
        ];
        for (w, h, ch, len) in cases {
            let ok = len == w as usize * h as usize * ch;
            let rgba8 = vec![0u8; len];
            let f32s = vec![0f32; len];
            let alpha = vec![0u8; len / 4];
            let alpha_ok = alpha.len() == w as usize * h as usize;
            assert_eq!(
                ImageView::new(w, h, PixelData::Rgba8(&rgba8)).is_ok(),
                ok,
                "rgba8 {w}x{h} len {len}"
            );
            assert_eq!(
                ImageView::new(w, h, PixelData::RgbaF32(&f32s)).is_ok(),
                ok,
                "f32 {w}x{h} len {len}"
            );
            assert_eq!(
                ImageView::new(w, h, PixelData::Alpha8(&alpha)).is_ok(),
                alpha_ok,
                "alpha8 {w}x{h} len {}",
                alpha.len()
            );
        }
        let one = [0u8; 1];
        assert_eq!(
            ImageView::new(2, 3, PixelData::Alpha8(&one)).unwrap_err(),
            BufferError::LengthMismatch {
                expected: 6,
                actual: 1
            }
        );
    }

    // ARQ-117: an overflowing product is Err and allocates nothing.
    #[test]
    fn arq_117_size_overflow_is_err_without_allocation() {
        let empty: [u8; 0] = [];
        let err = ImageView::new(u32::MAX, u32::MAX, PixelData::Rgba8(&empty)).unwrap_err();
        assert_eq!(
            err,
            BufferError::SizeOverflow {
                width: u32::MAX,
                height: u32::MAX
            }
        );
        // The pixel count fits in 64 bits but the element count does not.
        let err = ImageView::new(u32::MAX, u32::MAX / 2, PixelData::RgbaF32(&[])).unwrap_err();
        assert!(matches!(err, BufferError::SizeOverflow { .. }), "{err:?}");
        let mut none: [u8; 0] = [];
        assert!(matches!(
            ImageViewMut::new(u32::MAX, u32::MAX, PixelDataMut::Rgba8(&mut none)),
            Err(BufferError::SizeOverflow { .. })
        ));
        // One channel still fits in 64 bits: a plain mismatch, no panic.
        assert!(matches!(
            AlphaMask::new(u32::MAX, u32::MAX, Vec::new()),
            Err(BufferError::LengthMismatch { .. })
        ));
        // A large but representable size is a plain mismatch.
        assert_eq!(
            ImageView::new(u32::MAX, 1, PixelData::Alpha8(&[])).unwrap_err(),
            BufferError::LengthMismatch {
                expected: u32::MAX as usize,
                actual: 0
            }
        );
    }

    // ARQ-117: the mutable view and the mask apply the same check.
    #[test]
    fn arq_117_mutable_view_mask_and_buf_check_length() {
        let mut ok = [0u8; 8];
        assert!(ImageViewMut::new(2, 1, PixelDataMut::Rgba8(&mut ok)).is_ok());
        let mut bad = [0f32; 7];
        assert!(ImageViewMut::new(2, 1, PixelDataMut::RgbaF32(&mut bad)).is_err());
        assert!(AlphaMask::new(2, 2, vec![0; 4]).is_ok());
        assert!(AlphaMask::new(2, 2, vec![0; 5]).is_err());
        assert!(ImageBuf::new(1, 1, PixelBuf::Rgba8(vec![0; 3])).is_err());
        assert!(ImageBuf::new(1, 1, PixelBuf::Rgba8(vec![0; 4])).is_ok());
    }

    // ARQ-110: constructors and plain data do not validate numbers.
    #[test]
    fn arq_110_constructors_accept_non_finite_values() {
        let px = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0];
        let view = ImageView::new(1, 1, PixelData::RgbaF32(&px)).unwrap();
        assert_eq!((view.width(), view.height()), (1, 1));
        let cx = EffectCx::whole(f32::NAN);
        assert!(cx.scale.is_nan());
    }

    // ARQ-117: the accessors give back what was stored.
    #[test]
    fn arq_117_view_round_trips_dimensions_and_pixels() {
        let data = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let view = ImageView::new(2, 1, PixelData::Rgba8(&data)).unwrap();
        assert!(matches!(view.data(), PixelData::Rgba8(d) if d == data));
        assert_eq!(view.data().format(), PixelFormat::Rgba8);
        let mut copy = data;
        let mut m = ImageViewMut::new(1, 2, PixelDataMut::Rgba8(&mut copy)).unwrap();
        assert_eq!((m.width(), m.height()), (1, 2));
        if let PixelDataMut::Rgba8(d) = m.data_mut() {
            d[0] = 99;
        }
        assert!(matches!(m.data(), PixelData::Rgba8(d) if d[0] == 99));
        let mask = AlphaMask::new(3, 1, vec![7, 8, 9]).unwrap();
        assert_eq!(
            (mask.width(), mask.height(), mask.data()),
            (3, 1, &[7u8, 8, 9][..])
        );
        let buf = ImageBuf::new(2, 1, PixelBuf::Rgba8(data.to_vec())).unwrap();
        assert!(matches!(buf.view().data(), PixelData::Rgba8(d) if d == data));
    }

    // ARQ-102 / ADR 0012: each variant has a valid key with named arguments.
    #[test]
    fn buffer_error_maps_to_message_keys() {
        use gafira_doc::msg::ToMessage;
        let keys = [
            (
                BufferError::SizeOverflow {
                    width: 1,
                    height: 2,
                },
                "error.raster.size_overflow",
            ),
            (
                BufferError::LengthMismatch {
                    expected: 1,
                    actual: 2,
                },
                "error.raster.length_mismatch",
            ),
            (
                BufferError::NonFinite {
                    field: "pixels",
                    index: 3,
                },
                "error.raster.non_finite",
            ),
        ];
        for (e, key) in keys {
            assert_eq!(e.to_message().key.0, key);
        }
    }
}
