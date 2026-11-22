//! Giving an image a better resolution, piece by piece: each one is worked
//! on with some of what is around it, so no seam shows.

use std::sync::{Arc, Mutex, PoisonError};

use gafira_doc::contract::{CancelToken, Progress};
use gafira_ports::vision::{Pixels, Upscaler, VisionError};
use image::imageops::FilterType;
use image::{GrayImage, RgbImage, imageops};
use rayon::prelude::*;

use crate::runtime::Session;

/// What is seen around a piece, in px.
const AROUND: usize = 10;

/// The most pixels of an image to work on, and of a result.
pub(crate) const MOST_IN: u64 = 6_000_000;
const MOST_OUT: u64 = 100_000_000;

pub(crate) struct Enlarger {
    pub session: Arc<dyn Session>,
    pub tile: usize,
    pub scale: u32,
}

/// A piece: its place, and its size without what is around it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Piece {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

/// The pieces of an image of `w` × `h`, each within `inner`.
fn pieces(w: usize, h: usize, inner: usize) -> Vec<Piece> {
    let mut out = Vec::new();
    for y in (0..h).step_by(inner) {
        for x in (0..w).step_by(inner) {
            out.push(Piece {
                x,
                y,
                w: inner.min(w - x),
                h: inner.min(h - y),
            });
        }
    }
    out
}

impl Enlarger {
    /// What goes in for `piece`: it and what is around it; the edges of the
    /// image go on outside.
    fn input(&self, image: &Pixels, piece: Piece) -> Vec<f32> {
        let (w, h) = (image.size().0 as usize, image.size().1 as usize);
        let place =
            |start: usize, i: usize, last: usize| (start + i).saturating_sub(AROUND).min(last);
        // Before the start, the first one again.
        let before = |start: usize, i: usize| start + i < AROUND;
        let mut out = Vec::with_capacity(3 * self.tile * self.tile);
        for c in 0..3 {
            for y in 0..self.tile {
                let row = if before(piece.y, y) {
                    0
                } else {
                    place(piece.y, y, h - 1)
                };
                for x in 0..self.tile {
                    let col = if before(piece.x, x) {
                        0
                    } else {
                        place(piece.x, x, w - 1)
                    };
                    out.push(f32::from(image.rgba()[(row * w + col) * 4 + c]) / 255.0);
                }
            }
        }
        out
    }

    /// The pixels of `piece` in `out`, what the model gave, without what is
    /// around it: row by row, three bytes each.
    fn taken(&self, out: &[f32], piece: Piece) -> Result<Vec<u8>, VisionError> {
        let k = self.scale as usize;
        let side = self.tile * k;
        if out.len() != 3 * side * side {
            return Err(VisionError::Model("an odd output".to_owned()));
        }
        let mut bytes = Vec::with_capacity(piece.w * piece.h * k * k * 3);
        for y in 0..piece.h * k {
            for x in 0..piece.w * k {
                let at = (y + AROUND * k) * side + x + AROUND * k;
                for c in 0..3 {
                    let v = out[c * side * side + at].clamp(0.0, 1.0);
                    // Within 0..=255: the cast is exact.
                    bytes.push((v * 255.0).round() as u8);
                }
            }
        }
        Ok(bytes)
    }
}

impl Upscaler for Enlarger {
    fn most(&self) -> u32 {
        self.scale
    }

    fn upscale(
        &self,
        image: &Pixels,
        factor: u32,
        progress: &dyn Progress,
        cancel: &CancelToken,
    ) -> Result<Pixels, VisionError> {
        let cancelled = || cancel.check().map_err(|_| VisionError::Cancelled);
        cancelled()?;
        let (w, h) = image.size();
        let pixels = u64::from(w) * u64::from(h);
        let k = u64::from(self.scale);
        if factor == 0 || factor > self.scale || pixels > MOST_IN || pixels * k * k > MOST_OUT {
            return Err(VisionError::TooLarge);
        }
        let pieces = pieces(w as usize, h as usize, self.tile - 2 * AROUND);
        // Told in order, though many are done at once.
        let done = Mutex::new(0);
        let one = |piece: &Piece| -> Result<Vec<u8>, VisionError> {
            cancelled()?;
            let out = self.session.run(self.input(image, *piece))?;
            let bytes = self.taken(&out, *piece)?;
            let mut count = done.lock().unwrap_or_else(PoisonError::into_inner);
            *count += 1;
            // Few pieces: the casts are exact. The last tenth puts them
            // together.
            progress.report(0.9 * *count as f32 / pieces.len() as f32);
            Ok(bytes)
        };
        let at_once = self.session.at_once().min(pieces.len());
        let parts: Vec<Vec<u8>> = if at_once > 1 {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(at_once)
                .build()
                .map_err(|e| VisionError::Model(e.to_string()))?;
            pool.install(|| pieces.par_iter().map(one).collect::<Result<_, _>>())?
        } else {
            pieces.iter().map(one).collect::<Result<_, _>>()?
        };
        let k = self.scale as usize;
        let wide = w as usize * k;
        let mut rgb = vec![0; wide * h as usize * k * 3];
        for (piece, bytes) in pieces.iter().zip(&parts) {
            let line = piece.w * k * 3;
            for (row, bytes) in bytes.chunks_exact(line).enumerate() {
                let at = ((piece.y * k + row) * wide + piece.x * k) * 3;
                rgb[at..at + line].copy_from_slice(bytes);
            }
        }
        drop(parts);
        cancelled()?;
        let odd = || VisionError::Model("an odd output".to_owned());
        let mut whole = RgbImage::from_raw(w * self.scale, h * self.scale, rgb).ok_or_else(odd)?;
        let (out_w, out_h) = (w * factor, h * factor);
        if factor != self.scale {
            whole = imageops::resize(&whole, out_w, out_h, FilterType::CatmullRom);
        }
        let alpha = opacity(image, out_w, out_h).ok_or_else(odd)?;
        let rgba = whole
            .pixels()
            .zip(alpha.pixels())
            .flat_map(|(p, a)| [p.0[0], p.0[1], p.0[2], a.0[0]])
            .collect();
        progress.report(1.0);
        Pixels::new(out_w, out_h, rgba).ok_or_else(odd)
    }
}

/// The opacity of `image` at `w` × `h`.
fn opacity(image: &Pixels, w: u32, h: u32) -> Option<GrayImage> {
    let alpha: Vec<u8> = image.rgba().chunks_exact(4).map(|p| p[3]).collect();
    if alpha.iter().all(|a| *a == 255) {
        return Some(GrayImage::from_pixel(w, h, image::Luma([255])));
    }
    let (iw, ih) = image.size();
    let alpha = GrayImage::from_raw(iw, ih, alpha)?;
    Some(imageops::resize(&alpha, w, h, FilterType::Triangle))
}

#[cfg(test)]
mod tests {
    use gafira_doc::contract::NoProgress;
    use proptest::prelude::*;

    use super::*;

    /// Each pixel again and again.
    struct Again {
        tile: usize,
        scale: usize,
        at_once: usize,
    }

    impl Session for Again {
        fn run(&self, input: Vec<f32>) -> Result<Vec<f32>, VisionError> {
            let side = self.tile * self.scale;
            let mut out = Vec::with_capacity(3 * side * side);
            for c in 0..3 {
                for y in 0..side {
                    for x in 0..side {
                        let at = y / self.scale * self.tile + x / self.scale;
                        out.push(input[c * self.tile * self.tile + at]);
                    }
                }
            }
            Ok(out)
        }

        fn at_once(&self) -> usize {
            self.at_once
        }
    }

    fn enlarger(tile: usize, at_once: usize) -> Enlarger {
        Enlarger {
            session: Arc::new(Again {
                tile,
                scale: 4,
                at_once,
            }),
            tile,
            scale: 4,
        }
    }

    fn image(w: u32, h: u32) -> Pixels {
        let bytes = (0..w * h * 4).map(|i| (i.wrapping_mul(2_654_435_761) >> 24) as u8);
        Pixels::new(w, h, bytes.collect()).unwrap()
    }

    #[derive(Default)]
    struct Seen(Mutex<Vec<f32>>);

    impl Progress for Seen {
        fn report(&self, fraction: f32) {
            self.0.lock().unwrap().push(fraction);
        }
    }

    proptest! {
        // S84-03: every pixel lands in its place, whatever the size and the
        // pieces, one by one or many at once: no seam, none missing.
        #[test]
        fn s84_03_pieces_make_the_whole(w in 1_u32..90, h in 1_u32..90, tile in 24_usize..48, at_once in 1_usize..4) {
            let image = image(w, h);
            let out = enlarger(tile, at_once).upscale(&image, 4, &NoProgress, &CancelToken::new()).unwrap();
            prop_assert_eq!(out.size(), (w * 4, h * 4));
            for y in 0..h * 4 {
                for x in 0..w * 4 {
                    let at = ((y * w * 4 + x) * 4) as usize;
                    let of = ((y / 4 * w + x / 4) * 4) as usize;
                    prop_assert_eq!(&out.rgba()[at..at + 3], &image.rgba()[of..of + 3]);
                }
            }
        }
    }

    // S84-01: less than the most, and what is see-through stays so.
    #[test]
    fn s84_01_factors_and_opacity() {
        let mut bytes = vec![90; 20 * 10 * 4];
        for p in bytes.chunks_exact_mut(4).take(100) {
            p[3] = 0;
        }
        let image = Pixels::new(20, 10, bytes).unwrap();
        let out = enlarger(32, 1)
            .upscale(&image, 2, &NoProgress, &CancelToken::new())
            .unwrap();
        assert_eq!(out.size(), (40, 20));
        assert_eq!((out.rgba()[3], out.rgba()[out.rgba().len() - 1]), (0, 90));
        assert_eq!(out.rgba()[0], 90);
    }

    // S84-07: it tells how far it is, and stops when asked.
    #[test]
    fn s84_07_progress_and_cancel() {
        let seen = Seen::default();
        enlarger(32, 1)
            .upscale(&image(60, 60), 4, &seen, &CancelToken::new())
            .unwrap();
        let seen = seen.0.into_inner().unwrap();
        assert!(
            seen.len() > 5 && seen.windows(2).all(|p| p[0] <= p[1]) && seen.last() == Some(&1.0)
        );
        let cancel = CancelToken::new();
        cancel.cancel();
        let out = enlarger(32, 1).upscale(&image(60, 60), 4, &NoProgress, &cancel);
        assert_eq!(out, Err(VisionError::Cancelled));
    }

    #[test]
    fn s84_07_too_large() {
        let large = Pixels::new(4000, 2000, vec![0; 32_000_000]).unwrap();
        let one = enlarger(32, 1);
        assert_eq!(
            one.upscale(&large, 2, &NoProgress, &CancelToken::new()),
            Err(VisionError::TooLarge)
        );
        assert_eq!(
            one.upscale(&image(5, 5), 8, &NoProgress, &CancelToken::new()),
            Err(VisionError::TooLarge)
        );
        assert_eq!(
            one.upscale(&image(5, 5), 0, &NoProgress, &CancelToken::new()),
            Err(VisionError::TooLarge)
        );
    }
}
