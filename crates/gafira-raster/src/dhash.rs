//! The perceptual difference hash (ARQ-122, FD-28).

use crate::transfer::Transfer;
use crate::{BufferError, EffectError, ImageView, PixelData, PixelFormat};

/// Two hashes this far apart or less are of similar images (ARQ-122, PROPUESTA).
pub const SIMILAR_DISTANCE: u32 = 10;

/// Cells across and down: nine columns give eight comparisons per row.
const COLS: usize = 9;
const ROWS: usize = 8;

/// The pixels `[lo, hi)` of cell `i` of `cells` over a side of `n` pixels. Cells at the same
/// pace share at most the pixels on their border, and each has one pixel at least (the end of
/// a cell is past its start), so an image smaller than the grid still fills every cell.
fn span(i: usize, cells: usize, n: usize) -> (usize, usize) {
    let lo = i * n / cells;
    let hi = ((i + 1) * n).div_ceil(cells);
    (lo, hi)
}

/// The sum of the luminance of every cell, `luma` giving one pixel's value as an integer.
fn cell_sums<T>(data: &[T], w: usize, h: usize, luma: impl Fn(&[T]) -> u64) -> [[u64; COLS]; ROWS] {
    let mut sums = [[0u64; COLS]; ROWS];
    for (r, row_sums) in sums.iter_mut().enumerate() {
        let (y0, y1) = span(r, ROWS, h);
        for y in y0..y1 {
            let line = &data[y * w * 4..(y + 1) * w * 4];
            for (c, cell) in row_sums.iter_mut().enumerate() {
                let (x0, x1) = span(c, COLS, w);
                *cell += line[x0 * 4..x1 * 4].chunks_exact(4).map(&luma).sum::<u64>();
            }
        }
    }
    sums
}

/// The 64-bit dHash of an image: the image is read as nine by eight cells of mean luminance
/// (of the premultiplied sRGB values, so transparent is black), and bit `8 r + c` is set when
/// cell `c` of row `r` is brighter than cell `c + 1`. An image of size 0 hashes to 0.
///
/// # Errors
///
/// [`EffectError::UnsupportedFormat`] for `Alpha8`, [`EffectError::Buffer`] for a non-finite
/// `f32` pixel.
pub fn dhash(image: ImageView<'_>) -> Result<u64, EffectError> {
    let format = image.data().format();
    if format == PixelFormat::Alpha8 {
        return Err(EffectError::UnsupportedFormat { format });
    }
    let (w, h) = (image.width() as usize, image.height() as usize);
    let sums = match image.data() {
        PixelData::Rgba8(s) => {
            if w == 0 || h == 0 {
                return Ok(0);
            }
            cell_sums(s, w, h, |p| {
                // Alpha 0 is black, whatever bytes the pixel holds (ARQ-117).
                if p[3] == 0 {
                    return 0;
                }
                u64::from(2126 * u32::from(p[0]) + 7152 * u32::from(p[1]) + 722 * u32::from(p[2]))
            })
        }
        PixelData::RgbaF32(s) => {
            if let Some(index) = s.iter().position(|v| !v.is_finite()) {
                return Err(BufferError::NonFinite {
                    field: "src",
                    index,
                }
                .into());
            }
            if w == 0 || h == 0 {
                return Ok(0);
            }
            let t = Transfer::new();
            cell_sums(s, w, h, |p| {
                if p[3] == 0.0 {
                    return 0;
                }
                let e = t.encode_unit([f64::from(p[0]), f64::from(p[1]), f64::from(p[2])]);
                // The same scale as the 8-bit sum: 10 000 * 255 for white. Whole numbers, so
                // a flat image has equal cells.
                (2126.0 * 255.0 * e[0] + 7152.0 * 255.0 * e[1] + 722.0 * 255.0 * e[2]).round()
                    as u64
            })
        }
        PixelData::Alpha8(_) => return Ok(0),
    };
    let mut hash = 0u64;
    for (r, row) in sums.iter().enumerate() {
        let (y0, y1) = span(r, ROWS, h);
        for c in 0..COLS - 1 {
            let (xl0, xl1) = span(c, COLS, w);
            let (xr0, xr1) = span(c + 1, COLS, w);
            let left_area = ((y1 - y0) * (xl1 - xl0)) as u128;
            let right_area = ((y1 - y0) * (xr1 - xr0)) as u128;
            // mean(left) > mean(right), without dividing.
            if u128::from(row[c]) * right_area > u128::from(row[c + 1]) * left_area {
                hash |= 1 << (8 * r + c);
            }
        }
    }
    Ok(hash)
}

/// How many of the 64 bits differ.
#[must_use]
pub fn hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// Whether two hashes are of similar images: a distance of [`SIMILAR_DISTANCE`] or less.
#[must_use]
pub fn is_similar(a: u64, b: u64) -> bool {
    hamming_distance(a, b) <= SIMILAR_DISTANCE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_mips;

    /// An `Rgba8` image from a function of the pixel.
    fn img(w: usize, h: usize, mut f: impl FnMut(usize, usize) -> [u8; 4]) -> Vec<u8> {
        let mut v = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
                v.extend(f(x, y));
            }
        }
        v
    }

    fn hash8(bytes: &[u8], w: usize, h: usize) -> u64 {
        dhash(ImageView::new(w as u32, h as u32, PixelData::Rgba8(bytes)).unwrap()).unwrap()
    }

    fn grey(v: u8) -> [u8; 4] {
        [v, v, v, 255]
    }

    // ARQ-122: bit `8 r + c` is set when cell (c, r) is brighter than cell (c + 1, r), on a grid
    // of 9 by 8 cells. A ramp that falls to the right sets all 64; one that rises sets none.
    #[test]
    fn arq_122_dhash_ramps_set_all_or_no_bits() {
        let (w, h) = (90, 80);
        let falling = img(w, h, |x, _| grey(250 - (x * 2) as u8));
        let rising = img(w, h, |x, _| grey(10 + (x * 2) as u8));
        assert_eq!(hash8(&falling, w, h), u64::MAX);
        assert_eq!(hash8(&rising, w, h), 0);
        // Only the horizontal change counts.
        let down = img(w, h, |_, y| grey(250 - (y * 3) as u8));
        assert_eq!(hash8(&down, w, h), 0);
        let flat = img(w, h, |_, _| grey(77));
        assert_eq!(hash8(&flat, w, h), 0);
    }

    // ARQ-122: a falling ramp in one band of rows sets that byte of the hash.
    #[test]
    fn arq_122_dhash_bit_rows_follow_the_bands_of_the_image() {
        let (w, h) = (90, 80);
        for r in 0..8usize {
            let bytes = img(w, h, |x, y| {
                if y / 10 == r {
                    grey(250 - (x * 2) as u8)
                } else {
                    grey(10 + (x * 2) as u8)
                }
            });
            assert_eq!(hash8(&bytes, w, h), 0xFFu64 << (8 * r), "row {r}");
        }
    }

    // ARQ-122: a bright column of cells sets the bit of its own column (the one it begins),
    // in every row.
    #[test]
    fn arq_122_dhash_bit_columns_follow_the_cells() {
        let (w, h) = (90, 80);
        for c in 0..8usize {
            let bytes = img(w, h, |x, _| if x / 10 == c { grey(220) } else { grey(30) });
            let column: u64 = (0..8).map(|r| 1u64 << (8 * r + c)).sum();
            assert_eq!(hash8(&bytes, w, h), column, "column {c}");
        }
        // The last column has no right neighbour: bright there sets nothing.
        let bytes = img(w, h, |x, _| if x / 10 == 8 { grey(220) } else { grey(30) });
        assert_eq!(hash8(&bytes, w, h), 0);
    }

    // ARQ-122: an image smaller than the grid still gives every cell a pixel.
    #[test]
    fn arq_122_dhash_tiny_images_cover_every_cell() {
        let ramp = img(3, 2, |x, _| grey(200 - (x * 80) as u8));
        // Cells 0-2 read pixel 0, 3-5 pixel 1, 6-8 pixel 2; rows 0-3 read row 0, 4-7 row 1.
        assert_eq!(hash8(&ramp, 3, 2), 0x2424_2424_2424_2424);
        assert_eq!(hash8(&[10, 20, 30, 255], 1, 1), 0);
        let col = img(1, 5, |_, y| grey((y * 40) as u8));
        assert_eq!(hash8(&col, 1, 5), 0);
        let row = img(20, 1, |x, _| grey(250 - (x * 10) as u8));
        assert_eq!(hash8(&row, 20, 1), u64::MAX);
    }

    // ARQ-122, 4.17.4: a size that is not a multiple of the grid still reads every pixel.
    #[test]
    fn arq_122_dhash_odd_sizes_split_the_image_evenly() {
        // 13 x 11: the last pixel, alone, is in the last cell only, which has no cell after it.
        let (w, h) = (13, 11);
        let base = img(w, h, |_, _| grey(40));
        assert_eq!(hash8(&base, w, h), 0);
        let mut lit = base.clone();
        lit[((h - 1) * w + w - 1) * 4..][..4].copy_from_slice(&grey(255));
        let a = hash8(&lit, w, h);
        assert_eq!(a, 0);
        let mut lit = base.clone();
        let x = 7;
        for y in 0..h {
            lit[(y * w + x) * 4..][..4].copy_from_slice(&grey(255));
        }
        // Column 7 of 13 lies in cells 4 and 5 (ranges 5..7 and 7..9 of a 9 wide split).
        assert_eq!(hash8(&lit, w, h), 0x2020_2020_2020_2020);
    }

    // ARQ-122: the colour is read as luminance of the premultiplied sRGB values (over black).
    #[test]
    fn arq_122_dhash_reads_premultiplied_luminance() {
        let (w, h) = (90, 80);
        // White at half alpha is premultiplied (128, 128, 128): darker than opaque 200.
        let a = img(w, h, |x, _| {
            if x < 50 {
                [128, 128, 128, 128]
            } else {
                grey(200)
            }
        });
        assert_eq!(hash8(&a, w, h), 0);
        let b = img(w, h, |x, _| {
            if x < 50 {
                grey(200)
            } else {
                [128, 128, 128, 128]
            }
        });
        let col4: u64 = (0..8).map(|r| 1u64 << (8 * r + 4)).sum();
        assert_eq!(hash8(&b, w, h), col4);
        // Fully transparent is black whatever colour it holds.
        let c = img(w, h, |x, _| if x < 50 { [0, 0, 0, 0] } else { grey(30) });
        assert_eq!(hash8(&c, w, h), 0);
        let d = img(w, h, |x, _| if x < 50 { [90, 90, 90, 0] } else { grey(30) });
        assert_eq!(hash8(&d, w, h), hash8(&c, w, h));
        // Green weighs more than blue: equal steps of each give opposite bits.
        let green = img(w, h, |x, _| {
            if x < 50 {
                [0, 160, 0, 255]
            } else {
                [0, 0, 160, 255]
            }
        });
        assert_eq!(hash8(&green, w, h), col4);
        let blue = img(w, h, |x, _| {
            if x < 50 {
                [0, 0, 160, 255]
            } else {
                [0, 160, 0, 255]
            }
        });
        assert_eq!(hash8(&blue, w, h), 0);
    }

    /// Premultiplied linear `f32` pixels of opaque bytes.
    fn to_f32(bytes: &[u8]) -> Vec<f32> {
        let lin = |b: u8| {
            let v = f64::from(b) / 255.0;
            (if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }) as f32
        };
        bytes
            .chunks(4)
            .flat_map(|p| [lin(p[0]), lin(p[1]), lin(p[2]), f32::from(p[3]) / 255.0])
            .collect()
    }

    // ARQ-122: luminance weights 0.2126, 0.7152 and 0.0722 on the encoded values, in both
    // formats: each pair is (left colour, right colour, whether the left is brighter).
    #[test]
    fn arq_122_dhash_channel_weights() {
        let (w, h) = (90, 80);
        let cases: [([u8; 3], [u8; 3], bool); 8] = [
            ([200, 0, 0], [40, 40, 40], true),
            ([160, 0, 0], [0, 0, 160], true),
            ([0, 0, 160], [160, 0, 0], false),
            ([0, 100, 0], [60, 60, 60], true),
            ([0, 0, 200], [10, 10, 10], true),
            ([0, 0, 200], [20, 20, 20], false),
            ([0, 0, 0], [0, 0, 0], false),
            ([255, 255, 255], [254, 254, 254], true),
        ];
        let col4: u64 = (0..8).map(|r| 1u64 << (8 * r + 4)).sum();
        for (l, r, left_brighter) in cases {
            let bytes = img(w, h, |x, _| {
                if x < 50 {
                    [l[0], l[1], l[2], 255]
                } else {
                    [r[0], r[1], r[2], 255]
                }
            });
            let want = if left_brighter { col4 } else { 0 };
            assert_eq!(hash8(&bytes, w, h), want, "{l:?} vs {r:?}");
            let floats = to_f32(&bytes);
            let f = dhash(ImageView::new(w as u32, h as u32, PixelData::RgbaF32(&floats)).unwrap())
                .unwrap();
            assert_eq!(f, want, "f32 {l:?} vs {r:?}");
        }
    }

    // ARQ-122: `RgbaF32` (linear, premultiplied) gives the hash of the same picture.
    #[test]
    fn arq_122_dhash_rgba_f32_matches_rgba8() {
        let (w, h) = (36, 32);
        let bytes = img(w, h, |x, y| {
            let v = 128.0 + 100.0 * ((x as f64 / 5.0).sin() * (y as f64 / 7.0).cos());
            [v as u8, (v * 0.8) as u8, (v * 0.6) as u8, 255]
        });
        let lin = |b: u8| {
            let v = f64::from(b) / 255.0;
            (if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }) as f32
        };
        let floats: Vec<f32> = bytes
            .chunks(4)
            .flat_map(|p| [lin(p[0]), lin(p[1]), lin(p[2]), 1.0])
            .collect();
        let f = dhash(ImageView::new(w as u32, h as u32, PixelData::RgbaF32(&floats)).unwrap())
            .unwrap();
        assert_ne!(f, 0);
        assert!(hamming_distance(f, hash8(&bytes, w, h)) <= 2);
        let flat = vec![0.3f32; w * h * 4];
        let flat_hash =
            dhash(ImageView::new(w as u32, h as u32, PixelData::RgbaF32(&flat)).unwrap()).unwrap();
        assert_eq!(flat_hash, 0);
    }

    // ARQ-122, ARQ-117: in `f32` too, alpha 0 is black whatever colour the pixel holds.
    #[test]
    fn arq_122_dhash_f32_alpha_zero_is_black() {
        let (w, h) = (18usize, 16usize);
        let make = |ghost: f32| -> Vec<f32> {
            (0..w * h)
                .flat_map(|i| {
                    if i % w < 9 {
                        [ghost, ghost, ghost, 0.0]
                    } else {
                        [0.2, 0.2, 0.2, 1.0]
                    }
                })
                .collect()
        };
        let hash = |px: &[f32]| {
            dhash(ImageView::new(w as u32, h as u32, PixelData::RgbaF32(px)).unwrap()).unwrap()
        };
        assert_eq!(hash(&make(0.9)), hash(&make(0.0)));
        assert_eq!(hash(&make(0.9)), 0);
    }

    fn scene(w: usize, h: usize) -> Vec<u8> {
        img(w, h, |x, y| {
            let (u, v) = (x as f64 / w as f64, y as f64 / h as f64);
            let g = 128.0
                + 50.0 * (u * 9.0 + 0.4).sin() * (v * 6.0).cos()
                + 40.0 * ((u - 0.5).hypot(v - 0.4) * 11.0).cos()
                + 30.0 * (u * 3.0 - v * 5.0).sin();
            let g = g.clamp(0.0, 255.0);
            [g as u8, (g * 0.9) as u8, (g * 0.7) as u8, 255]
        })
    }

    fn noise(w: usize, h: usize, seed: u32) -> Vec<u8> {
        let mut s = seed;
        img(w, h, |_, _| {
            s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let g = (s >> 24) as u8;
            [g, (s >> 16) as u8, (s >> 8) as u8, 255]
        })
    }

    // ARQ-122: an image and its version at 50 % are at distance <= 10 (similar).
    #[test]
    fn arq_122_dhash_an_image_and_its_half_are_similar() {
        for (w, h) in [(128, 96), (200, 150), (256, 192)] {
            let big = scene(w, h);
            let view = ImageView::new(w as u32, h as u32, PixelData::Rgba8(&big)).unwrap();
            let half = build_mips(view).unwrap().remove(1);
            let (a, b) = (hash8(&big, w, h), dhash(half.view()).unwrap());
            let d = hamming_distance(a, b);
            assert!(d <= 10 && is_similar(a, b), "{w}x{h}: distance {d}");
            assert_ne!(a, 0);
        }
    }

    // ARQ-122: two noise images with different seeds are further apart than 10.
    #[test]
    fn arq_122_dhash_noise_with_different_seeds_is_not_similar() {
        let (w, h) = (96, 80);
        let base = noise(w, h, 1);
        assert_eq!(hash8(&base, w, h), hash8(&noise(w, h, 1), w, h));
        for seed in [2, 3, 4, 99, 12345] {
            let other = noise(w, h, seed);
            let (a, b) = (hash8(&base, w, h), hash8(&other, w, h));
            let d = hamming_distance(a, b);
            assert!(d > 10 && !is_similar(a, b), "seed {seed}: distance {d}");
        }
    }

    // ARQ-122 (PROPUESTA): similar means a distance of at most 10.
    #[test]
    fn arq_122_hamming_distance_and_the_threshold() {
        assert_eq!(SIMILAR_DISTANCE, 10);
        assert_eq!(hamming_distance(0, 0), 0);
        assert_eq!(hamming_distance(0, u64::MAX), 64);
        assert_eq!(hamming_distance(0b1011, 0b0001), 2);
        assert_eq!(hamming_distance(u64::MAX, 1 << 63), 63);
        let base = 0xDEAD_BEEF_0123_4567u64;
        let flip = |n: u32| base ^ ((1u64 << n) - 1);
        assert!(is_similar(base, base));
        assert!(is_similar(base, flip(9)));
        assert!(is_similar(base, flip(10)));
        assert!(!is_similar(base, flip(11)));
        assert!(!is_similar(base, !base));
        assert!(is_similar(flip(10), base));
    }

    // ARQ-110 and formats; size 0.
    #[test]
    fn arq_110_dhash_rejects_non_finite_pixels_and_alpha8() {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let px = [0.1f32, 0.2, 0.3, 1.0, 0.1, bad, 0.3, 1.0];
            assert_eq!(
                dhash(ImageView::new(2, 1, PixelData::RgbaF32(&px)).unwrap()),
                Err(EffectError::Buffer(BufferError::NonFinite {
                    field: "src",
                    index: 5
                }))
            );
        }
        let a8 = [1u8, 2, 3, 4];
        assert_eq!(
            dhash(ImageView::new(2, 2, PixelData::Alpha8(&a8)).unwrap()),
            Err(EffectError::UnsupportedFormat {
                format: PixelFormat::Alpha8
            })
        );
        assert_eq!(
            dhash(ImageView::new(0, 0, PixelData::Rgba8(&[])).unwrap()),
            Ok(0)
        );
        assert_eq!(
            dhash(ImageView::new(4, 0, PixelData::RgbaF32(&[])).unwrap()),
            Ok(0)
        );
    }

    // ARQ-111
    #[test]
    fn arq_111_dhash_is_deterministic() {
        let big = scene(64, 48);
        let h = hash8(&big, 64, 48);
        assert_eq!(h, hash8(&big, 64, 48));
        assert_ne!(h, 0);
    }
}
