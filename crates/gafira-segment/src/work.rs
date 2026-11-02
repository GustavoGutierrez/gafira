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

/// Longest side of the working copy (D-45).
pub(crate) const WORK_SIDE: usize = 2048;

/// Working size: a longer side above [`WORK_SIDE`] is reduced to it, keeping the
/// aspect; anything at or below is kept (never enlarged).
pub(crate) fn working_size(w: usize, h: usize) -> (usize, usize) {
    let long = w.max(h);
    if long <= WORK_SIDE {
        return (w, h);
    }
    let scaled = |side: usize| ((side * WORK_SIDE * 2 + long) / (2 * long)).max(1);
    (scaled(w), scaled(h))
}

/// The working copy: sRGB bytes at the working size.
pub(crate) struct WorkImage {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) rgb: Vec<[u8; 3]>,
}

impl WorkImage {
    /// The working copy of `src`: each pixel is the mean of its block of source
    /// pixels (a block is one pixel when nothing is reduced). Serial chunks of
    /// work rows, the token checked in each.
    pub(crate) fn build(src: &Source<'_>, meter: &Meter<'_>) -> Result<Self, Cancelled> {
        let (width, height) = working_size(src.width, src.height);
        let mut rgb = Vec::with_capacity(width * height);
        for y0 in (0..height).step_by(CHUNK_ROWS) {
            let y1 = (y0 + CHUNK_ROWS).min(height);
            let rows = (y1 * src.height / height) - (y0 * src.height / height);
            meter.step((rows.max(1) * src.width) as u64)?;
            for y in y0..y1 {
                let (sy0, sy1) = block(y, height, src.height);
                for x in 0..width {
                    let (sx0, sx1) = block(x, width, src.width);
                    let mut sum = [0u32; 3];
                    for sy in sy0..sy1 {
                        for sx in sx0..sx1 {
                            let p = src.rgb(sy * src.width + sx);
                            for (s, v) in sum.iter_mut().zip(p) {
                                *s += u32::from(v);
                            }
                        }
                    }
                    let n = ((sy1 - sy0) * (sx1 - sx0)) as u32;
                    rgb.push(sum.map(|v| ((v * 2 + n) / (2 * n)) as u8));
                }
            }
        }
        Ok(Self { width, height, rgb })
    }
}

/// The source range `[start, end)` that work index `i` averages; never empty.
fn block(i: usize, work: usize, source: usize) -> (usize, usize) {
    let start = i * source / work;
    (start, ((i + 1) * source / work).max(start + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    // FD-38: the reduced copy is the area average of the source blocks.
    #[test]
    fn fd_38_reduction_averages_source_blocks() {
        use gafira_doc::contract::{CancelToken, NoProgress};
        use gafira_raster::{ImageView, PixelData};
        // 4096x2: columns alternate 0 and 255; a 2048x1 copy averages pairs and rows.
        let (w, h) = (4096usize, 2usize);
        let mut px = Vec::new();
        for _ in 0..h {
            for x in 0..w {
                let v = if x % 4 < 2 { 0 } else { 255 };
                px.extend_from_slice(&[v, v, v, 255]);
            }
        }
        let view = ImageView::new(w as u32, h as u32, PixelData::Rgba8(&px)).unwrap();
        let (progress, cancel) = (NoProgress, CancelToken::new());
        let meter = Meter::new(&progress, &cancel, 1 << 30);
        let work = WorkImage::build(&Source::new(&view), &meter).unwrap();
        assert_eq!((work.width, work.height), (2048, 1));
        // Pairs (0,1) are black and pairs (2,3) white: work pixels alternate 0, 255.
        assert_eq!(work.rgb[0], [0, 0, 0]);
        assert_eq!(work.rgb[1], [255, 255, 255]);
    }

    // FD-38 / ARQ-130: a long side above 2048 is reduced to it; 2048 or less is kept.
    #[test]
    fn fd_38_working_size_reduces_only_above_2048() {
        let cases: [((usize, usize), (usize, usize)); 14] = [
            ((4000, 3000), (2048, 1536)),
            ((3000, 4000), (1536, 2048)),
            ((2049, 1536), (2048, 1535)),
            ((1536, 2049), (1535, 2048)),
            ((2049, 2049), (2048, 2048)),
            ((2048, 1536), (2048, 1536)),
            ((1536, 2048), (1536, 2048)),
            ((2048, 2048), (2048, 2048)),
            ((2047, 2047), (2047, 2047)),
            ((800, 600), (800, 600)),
            ((1, 1), (1, 1)),
            ((1, 9000), (1, 2048)),
            ((9000, 1), (2048, 1)),
            ((100, 50), (100, 50)),
        ];
        for ((w, h), want) in cases {
            assert_eq!(working_size(w, h), want, "{w}x{h}");
        }
    }
}
