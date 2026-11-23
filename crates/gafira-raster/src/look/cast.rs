//! The shadow of an image (S85-04), around it: the image keeps its pixels and
//! its size, and the picture grows by the margin the shadow asks for.

use gafira_doc::{ImageShadow, ShadowKind};

use super::unsupported;
use crate::effects::blur_plane;
use crate::transfer::Transfer;
use crate::{BufferError, EffectError, ImageBuf, PixelBuf};

/// The blur is worked out at the size where its radius is about this.
const BLUR_AT: f64 = 16.0;

/// Coverage, 0 to 1, row by row.
struct Plane {
    w: usize,
    h: usize,
    data: Vec<f32>,
}

impl Plane {
    /// The coverage around `(x, y)`, between its four neighbours; none outside.
    fn at(&self, x: f64, y: f64) -> f32 {
        let (fx, fy) = (x.floor(), y.floor());
        let (tx, ty) = ((x - fx) as f32, (y - fy) as f32);
        let one = |x: f64, y: f64| {
            let inside = x >= 0.0 && y >= 0.0 && x < self.w as f64 && y < self.h as f64;
            if inside {
                self.data[y as usize * self.w + x as usize]
            } else {
                0.0
            }
        };
        let top = one(fx, fy) * (1.0 - tx) + one(fx + 1.0, fy) * tx;
        let low = one(fx, fy + 1.0) * (1.0 - tx) + one(fx + 1.0, fy + 1.0) * tx;
        top * (1.0 - ty) + low * ty
    }

    fn map(&self, f: impl Fn(f64, f64) -> f32) -> Self {
        let data = (0..self.h)
            .flat_map(|y| (0..self.w).map(move |x| (x, y)))
            .map(|(x, y)| f(x as f64, y as f64))
            .collect();
        Self { data, ..*self }
    }

    /// One `k` times smaller, each of its pixels the mean of those it covers.
    fn reduced(&self, k: usize) -> Self {
        let (w, h) = (self.w.div_ceil(k), self.h.div_ceil(k));
        let mut data = vec![0.0; w * h];
        for (i, v) in self.data.iter().enumerate() {
            data[i / self.w / k * w + i % self.w / k] += v;
        }
        let area = (k * k) as f32;
        data.iter_mut().for_each(|v| *v /= area);
        Self { w, h, data }
    }
}

/// `image` (8-bit) over its shadow.
///
/// # Errors
///
/// [`EffectError`] for another format or a size that does not fit.
pub fn cast(image: &ImageBuf, shadow: &ImageShadow) -> Result<ImageBuf, EffectError> {
    let PixelBuf::Rgba8(bytes) = image.data() else {
        return Err(unsupported(image));
    };
    let (w, h) = (image.width(), image.height());
    let side = f64::from(w.min(h));
    let margin = shadow.margin(side) as u32;
    let overflow = BufferError::SizeOverflow {
        width: w,
        height: h,
    };
    let grown = |v: u32| margin.checked_mul(2).and_then(|m| v.checked_add(m));
    let (Some(out_w), Some(out_h)) = (grown(w), grown(h)) else {
        return Err(overflow.into());
    };
    let (ow, oh) = (out_w as usize, out_h as usize);
    let length = ow.checked_mul(oh).and_then(|n| n.checked_mul(4));
    let Some(length) = length else {
        return Err(overflow.into());
    };
    if w == 0 || h == 0 {
        return Ok(image.clone());
    }
    let (row, start) = (w as usize * 4, margin as usize);
    let mut alpha = Plane {
        w: ow,
        h: oh,
        data: vec![0.0; ow * oh],
    };
    for (y, line) in bytes.chunks_exact(row).enumerate() {
        let to = &mut alpha.data[(y + start) * ow + start..][..w as usize];
        for (to, px) in to.iter_mut().zip(line.chunks_exact(4)) {
            *to = f32::from(px[3]) / 255.0;
        }
    }
    let shape = shape(&alpha, shadow, side, f64::from(margin));
    let soft = softened(shape, 2.0 * shadow.blur * side);

    let color = shadow.color.to_linear_srgb();
    let [r, g, b] = Transfer::new()
        .encode([color.r, color.g, color.b].map(|c| f64::from(c).clamp(0.0, 1.0)))
        .map(f32::from);
    // Both within 0 to 1: the cast is enough.
    let strength = shadow.opacity as f32 * shadow.color.alpha();
    let mut out = vec![0_u8; length];
    for (px, cover) in out.chunks_exact_mut(4).zip(&soft.data) {
        let a = (cover * strength).clamp(0.0, 1.0);
        let top = (a * 255.0).round();
        for (to, c) in px.iter_mut().zip([r, g, b, 255.0]) {
            *to = (c * a).round().min(top) as u8;
        }
    }
    // The image over it, premultiplied.
    for (y, line) in bytes.chunks_exact(row).enumerate() {
        let to = &mut out[((y + start) * ow + start) * 4..][..row];
        for (to, px) in to.chunks_exact_mut(4).zip(line.chunks_exact(4)) {
            let rest = 255 - u32::from(px[3]);
            for (to, c) in to.iter_mut().zip(px) {
                let sum = u32::from(*c) + (u32::from(*to) * rest + 127) / 255;
                *to = u8::try_from(sum).unwrap_or(u8::MAX);
            }
        }
    }
    Ok(ImageBuf::new(out_w, out_h, PixelBuf::Rgba8(out))?)
}

/// Where the shadow is, before its blur.
fn shape(alpha: &Plane, shadow: &ImageShadow, side: f64, margin: f64) -> Plane {
    let turn = shadow.angle_deg.to_radians();
    let (dx, dy) = (turn.cos(), turn.sin());
    let (distance, size) = (shadow.distance * side, shadow.size * side);
    let (w, h) = (alpha.w as f64 - 2.0 * margin, alpha.h as f64 - 2.0 * margin);
    let (cx, cy) = ((alpha.w as f64 - 1.0) / 2.0, (alpha.h as f64 - 1.0) / 2.0);
    match shadow.kind {
        ShadowKind::Glow | ShadowKind::Outline => grown(alpha, size),
        ShadowKind::Drop => alpha.map(|x, y| alpha.at(x - dx * distance, y - dy * distance)),
        // Farther at both ends than in the middle, as of a sheet that bends.
        ShadowKind::Curved => {
            let half = ((dy * w).abs() + (dx * h).abs()) / 2.0;
            alpha.map(|x, y| {
                let across = (((y - cy) * dx - (x - cx) * dy) / half.max(1.0)).clamp(-1.0, 1.0);
                let far = distance * across * across;
                alpha.at(x - dx * far, y - dy * far)
            })
        }
        // Smaller than the image, and away from it.
        ShadowKind::Floating => {
            let (kx, ky) = (w / (w - 2.0 * size), h / (h - 2.0 * size));
            alpha.map(|x, y| {
                let (x, y) = (x - dx * distance - cx, y - dy * distance - cy);
                alpha.at(cx + x * kx, cy + y * ky)
            })
        }
        // Farther towards where it falls.
        ShadowKind::Angled => {
            let half = ((dx * w).abs() + (dy * h).abs()) / 2.0;
            alpha.map(|x, y| {
                let along = ((x - cx) * dx + (y - cy) * dy) / half.max(1.0);
                let far = distance + size * ((along + 1.0) / 2.0).clamp(0.0, 1.0);
                alpha.at(x - dx * far, y - dy * far)
            })
        }
        // Lying on the floor in front of the image, its feet on the edge.
        ShadowKind::Ground => {
            let floor = margin + h;
            alpha.map(|x, y| {
                let v = (y + 0.5 - floor) / size.max(1e-9);
                if !(0.0..=1.0).contains(&v) || size <= 0.0 {
                    return 0.0;
                }
                let cover = alpha.at(x - dx * distance * v, floor - v * h - 0.5);
                // Fainter as it goes away.
                cover * (1.0 - 0.6 * v) as f32
            })
        }
    }
}

/// `plane` grown by `by` pixels all around.
fn grown(plane: &Plane, by: f64) -> Plane {
    if by <= 0.0 {
        return plane.map(|x, y| plane.at(x, y));
    }
    let far = squared_distances(plane);
    let data = plane
        .data
        .iter()
        .zip(far)
        .map(|(v, d)| v.max((by + 1.0 - f64::from(d).sqrt()).clamp(0.0, 1.0) as f32))
        .collect();
    Plane { data, ..*plane }
}

/// How far each pixel is from the nearest one at least half covered, squared
/// (the lower envelope of parabolas, along the rows and then the columns).
fn squared_distances(plane: &Plane) -> Vec<f32> {
    const FAR: f32 = 1e12;
    let (w, h) = (plane.w, plane.h);
    let mut out: Vec<f32> = plane
        .data
        .iter()
        .map(|v| if *v >= 0.5 { 0.0 } else { FAR })
        .collect();
    let mut line = Vec::new();
    for row in out.chunks_exact_mut(w) {
        envelope(row, &mut line);
        row.copy_from_slice(&line);
    }
    let mut column = vec![0.0; h];
    for x in 0..w {
        for (y, v) in column.iter_mut().enumerate() {
            *v = out[y * w + x];
        }
        envelope(&column, &mut line);
        for (y, v) in line.iter().enumerate() {
            out[y * w + x] = *v;
        }
    }
    out
}

/// The lowest of the parabolas that stand on each value of `f`, at each place.
fn envelope(f: &[f32], out: &mut Vec<f32>) {
    let at = |q: usize| f64::from(f[q]) + (q * q) as f64;
    // Which parabola is lowest, and from where.
    let mut low: Vec<(usize, f64)> = Vec::with_capacity(f.len());
    for q in 0..f.len() {
        let mut from = f64::NEG_INFINITY;
        while let Some((p, start)) = low.last().copied() {
            from = (at(q) - at(p)) / (2 * (q - p)) as f64;
            if from > start {
                break;
            }
            low.pop();
            from = f64::NEG_INFINITY;
        }
        low.push((q, from));
    }
    out.clear();
    let mut k = 0;
    for x in 0..f.len() {
        while low.get(k + 1).is_some_and(|(_, from)| *from < x as f64) {
            k += 1;
        }
        let p = low[k].0;
        let d = x.abs_diff(p) as f64;
        out.push((d * d + f64::from(f[p])) as f32);
    }
}

/// `plane` blurred by `radius`: smaller first when it is wide, as nothing fine
/// is left of it.
fn softened(plane: Plane, radius: f64) -> Plane {
    let k = (radius / BLUR_AT).floor() as usize;
    if k <= 1 {
        let data = blur_plane(&plane.data, plane.w, plane.h, radius);
        return Plane { data, ..plane };
    }
    let small = plane.reduced(k);
    let data = blur_plane(&small.data, small.w, small.h, radius / k as f64);
    let soft = Plane { data, ..small };
    let place = |v: f64| (v + 0.5) / k as f64 - 0.5;
    plane.map(|x, y| soft.at(place(x), place(y)))
}
