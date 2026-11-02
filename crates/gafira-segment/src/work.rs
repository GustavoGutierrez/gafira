//! The colour-only working copy (D-45) and access to the source pixels.

use gafira_doc::contract::Cancelled;
use gafira_raster::{ImageView, PixelData};

use crate::meter::{CHUNK_ROWS, Meter};

/// The source image as sRGB bytes, whatever its storage (premultiplied: the
/// colour is read over black).
pub(crate) struct Source<'a> {
    data: PixelData<'a>,
    pub(crate) width: usize,
    pub(crate) height: usize,
}

/// Linear light to an encoded byte.
fn encode(linear: f32) -> u8 {
    let v = linear.clamp(0.0, 1.0);
    let e = if v <= 0.003_130_8 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };
    (e * 255.0 + 0.5).floor() as u8
}

impl<'a> Source<'a> {
    pub(crate) fn new(img: &ImageView<'a>) -> Self {
        Self {
            data: img.data(),
            width: img.width() as usize,
            height: img.height() as usize,
        }
    }

    /// The colour of pixel number `i` (row-major).
    pub(crate) fn rgb(&self, i: usize) -> [u8; 3] {
        match self.data {
            PixelData::Rgba8(d) => [d[i * 4], d[i * 4 + 1], d[i * 4 + 2]],
            PixelData::RgbaF32(d) => [encode(d[i * 4]), encode(d[i * 4 + 1]), encode(d[i * 4 + 2])],
            PixelData::Alpha8(_) => [0; 3],
        }
    }

    /// Luma in `[0, 1]` of pixel number `i`.
    pub(crate) fn luma(&self, i: usize) -> f32 {
        let [r, g, b] = self.rgb(i);
        (0.299 * f32::from(r) + 0.587 * f32::from(g) + 0.114 * f32::from(b)) / 255.0
    }
}

/// The working copy: sRGB bytes at the working size.
pub(crate) struct WorkImage {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) rgb: Vec<[u8; 3]>,
}

impl WorkImage {
    /// Copies the source, one serial chunk of rows at a time.
    pub(crate) fn copy(src: &Source<'_>, meter: &Meter<'_>) -> Result<Self, Cancelled> {
        let (width, height) = (src.width, src.height);
        let mut rgb = Vec::with_capacity(width * height);
        for y0 in (0..height).step_by(CHUNK_ROWS) {
            let y1 = (y0 + CHUNK_ROWS).min(height);
            meter.step(((y1 - y0) * width) as u64)?;
            rgb.extend((y0 * width..y1 * width).map(|i| src.rgb(i)));
        }
        Ok(Self { width, height, rgb })
    }
}
