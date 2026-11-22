//! The part of an image that shows, cut to its shape (S88-02).

use gafira_doc::{CropShape, ImageCrop};

use crate::{AlphaMask, ImageBuf, PixelBuf};

/// The first pixel and the count of a share of a side of `len` pixels: one
/// at least, inside it.
fn span(from: f64, share: f64, len: u32) -> (usize, usize) {
    let side = f64::from(len);
    // Shares of a side: within it.
    let first = ((from * side).round() as u32).min(len.saturating_sub(1));
    let last = (((from + share) * side).round() as u32).clamp(first + 1, len.max(first + 1));
    (first as usize, (last - first) as usize)
}

/// The rows of a part of `data`, `n` bytes a pixel.
fn part(data: &[u8], width: u32, n: usize, x: (usize, usize), y: (usize, usize)) -> Vec<u8> {
    let row = width as usize * n;
    data.chunks_exact(row)
        .skip(y.0)
        .take(y.1)
        .flat_map(|line| &line[x.0 * n..(x.0 + x.1) * n])
        .copied()
        .collect()
}

/// The part `crop` of `image` (8-bit), clear out of its shape.
#[must_use]
pub fn cropped(image: &ImageBuf, crop: &ImageCrop) -> ImageBuf {
    let PixelBuf::Rgba8(bytes) = image.data() else {
        return image.clone();
    };
    if image.width() == 0 || image.height() == 0 || crop.is_whole() {
        return image.clone();
    }
    let x = span(crop.x, crop.w, image.width());
    let y = span(crop.y, crop.h, image.height());
    let mut out = part(bytes, image.width(), 4, x, y);
    // Parts of the sides of the image: the casts are exact.
    let (w, h) = (x.1 as u32, y.1 as u32);
    if crop.shape != CropShape::Rect {
        let cover = coverage(&crop.shape, w, h);
        for (px, cover) in out.chunks_exact_mut(4).zip(cover) {
            for c in px {
                *c = crate::premultiply_u8(*c, cover);
            }
        }
    }
    ImageBuf::new(w, h, PixelBuf::Rgba8(out)).unwrap_or_else(|_| image.clone())
}

/// The part `crop` of a mask of the whole image: what is out of its shape
/// does not show, so it does not matter.
#[must_use]
pub fn cropped_mask(mask: &AlphaMask, crop: &ImageCrop) -> AlphaMask {
    if mask.width() == 0 || mask.height() == 0 || crop.is_whole() {
        return mask.clone();
    }
    let x = span(crop.x, crop.w, mask.width());
    let y = span(crop.y, crop.h, mask.height());
    let out = part(mask.data(), mask.width(), 1, x, y);
    AlphaMask::new(x.1 as u32, y.1 as u32, out).unwrap_or_else(|_| mask.clone())
}

/// The samples of a pixel, across and down.
const SAMPLES: usize = 4;

/// How much `shape` covers of each pixel of `w` × `h`.
#[must_use]
pub fn coverage(shape: &CropShape, w: u32, h: u32) -> Vec<u8> {
    let (width, height) = (f64::from(w), f64::from(h));
    let near = |distance: f64| {
        // Clamped: the cast drops the fraction alone.
        ((0.5 - distance).clamp(0.0, 1.0) * 255.0).round() as u8
    };
    let each = |f: &dyn Fn(f64, f64) -> u8| -> Vec<u8> {
        (0..h)
            .flat_map(|y| (0..w).map(move |x| (f64::from(x) + 0.5, f64::from(y) + 0.5)))
            .map(|(x, y)| f(x, y))
            .collect()
    };
    match shape {
        CropShape::Rect => vec![255; w as usize * h as usize],
        CropShape::Ellipse => {
            let (a, b) = (width / 2.0, height / 2.0);
            each(&|x, y| {
                // The distance to an ellipse, to the first order.
                let (u, v) = ((x - a) / a, (y - b) / b);
                let value = u * u + v * v - 1.0;
                let slope = 2.0 * (u / a).hypot(v / b);
                if slope <= f64::EPSILON {
                    return 255;
                }
                near(value / slope)
            })
        }
        CropShape::Rounded(share) => {
            let r = share * width.min(height);
            each(&|x, y| {
                let (dx, dy) = (
                    (x - width / 2.0).abs() - (width / 2.0 - r),
                    (y - height / 2.0).abs() - (height / 2.0 - r),
                );
                if dx <= 0.0 || dy <= 0.0 {
                    return 255;
                }
                near(dx.hypot(dy) - r)
            })
        }
        CropShape::Path(points) => path(points, (w as usize, h as usize)),
    }
}

/// The inside of a closed line (even and odd), by how many samples of each
/// pixel it has.
fn path(points: &[[f64; 2]], (w, h): (usize, usize)) -> Vec<u8> {
    let scale = |v: f64, side: usize| v * (side * SAMPLES) as f64;
    let points: Vec<(f64, f64)> = points
        .iter()
        .map(|p| (scale(p[0], w), scale(p[1], h)))
        .collect();
    let mut counts = vec![0_u32; w * h];
    let mut crossings = Vec::new();
    for row in 0..h * SAMPLES {
        let y = row as f64 + 0.5;
        crossings.clear();
        for (i, a) in points.iter().enumerate() {
            let b = points[(i + 1) % points.len()];
            if (a.1 <= y) != (b.1 <= y) {
                crossings.push(a.0 + (y - a.1) / (b.1 - a.1) * (b.0 - a.0));
            }
        }
        crossings.sort_by(f64::total_cmp);
        let line = &mut counts[row / SAMPLES * w..][..w];
        for pair in crossings.chunks_exact(2) {
            // From the first sample in to the last one, inside the row.
            let sample = |v: f64| ((v - 0.5).ceil().max(0.0) as usize).min(w * SAMPLES);
            for x in sample(pair[0])..sample(pair[1]) {
                line[x / SAMPLES] += 1;
            }
        }
    }
    // At most all of them: the cast is exact.
    let all = (SAMPLES * SAMPLES) as u32;
    counts
        .into_iter()
        .map(|n| ((n * 255 + all / 2) / all).min(255) as u8)
        .collect()
}
