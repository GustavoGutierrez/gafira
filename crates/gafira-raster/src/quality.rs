//! How good a photo is for a model to find its subject (S84-02).

/// What is wrong with an image, if anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quality {
    Good,
    /// Too few pixels.
    Small,
    /// Its edges are smeared.
    Blurry,
    /// It shows the blocks of a strong compression.
    Blocky,
}

/// The longest side under which an image is small, in px.
const SMALL: u32 = 800;

/// The longest side the sharpness is measured at: about what a model sees.
const SEEN: usize = 1024;

/// The sharpness under which an image is blurry: the share of its contrast
/// that lies between neighbours. Sharp photos measure 0.1 to 1, and one blurred by
/// three px as seen, 0.001 to 0.004.
const BLURRY: f32 = 0.006;

/// The steps between blocks against the ones inside them over which an image
/// is blocky (a JPEG of quality 10 measures about 2; one of 50, 1.2), and the longest side it is looked for at: seen smaller, the
/// blocks are gone.
const BLOCKY: f32 = 1.8;
const BLOCKS_SEEN: u32 = 2048;

/// The least contrast there is something to judge at (levels, squared).
const FLAT: f32 = 12.0;

/// The quality of an image of straight RGBA bytes, row by row.
#[must_use]
pub fn assess(width: u32, height: u32, rgba: &[u8]) -> Quality {
    let Some(luma) = luma(width, height, rgba) else {
        return Quality::Good;
    };
    let (w, h) = (width as usize, height as usize);
    if width.max(height) < SMALL {
        return Quality::Small;
    }
    if width.max(height) <= BLOCKS_SEEN && blockiness(&luma, w, h) > BLOCKY {
        return Quality::Blocky;
    }
    if sharp(&luma, w, h).is_some_and(|s| s < BLURRY) {
        return Quality::Blurry;
    }
    Quality::Good
}

/// How sharp an image of straight RGBA bytes is, from 0; none for one with
/// nothing to judge.
#[must_use]
pub fn sharpness(width: u32, height: u32, rgba: &[u8]) -> Option<f32> {
    sharp(&luma(width, height, rgba)?, width as usize, height as usize)
}

/// The light of each pixel, 0 to 255; none when `rgba` is not the image's.
fn luma(width: u32, height: u32, rgba: &[u8]) -> Option<Vec<f32>> {
    let pixels = (width as usize).checked_mul(height as usize)?;
    if pixels == 0 || pixels.checked_mul(4)? != rgba.len() {
        return None;
    }
    Some(
        rgba.chunks_exact(4)
            .map(|p| 0.299 * f32::from(p[0]) + 0.587 * f32::from(p[1]) + 0.114 * f32::from(p[2]))
            .collect(),
    )
}

/// The variance of the Laplacian over the one of the light, at the size a
/// model sees.
fn sharp(luma: &[f32], w: usize, h: usize) -> Option<f32> {
    let k = w.max(h).div_ceil(SEEN).max(1);
    let (sw, sh) = (w / k, h / k);
    if sw < 3 || sh < 3 {
        return None;
    }
    // Small counts: the casts are exact.
    let area = (k * k) as f32;
    let small: Vec<f32> = (0..sw * sh)
        .map(|i| {
            let (x, y) = (i % sw * k, i / sw * k);
            let rows = (y..y + k).map(|row| luma[row * w + x..row * w + x + k].iter().sum::<f32>());
            rows.sum::<f32>() / area
        })
        .collect();
    let contrast = variance(small.iter().copied());
    if contrast < FLAT {
        return None;
    }
    let edges = (1..sh - 1).flat_map(|y| (1..sw - 1).map(move |x| y * sw + x));
    let laplacian =
        edges.map(|i| small[i - 1] + small[i + 1] + small[i - sw] + small[i + sw] - 4.0 * small[i]);
    Some(variance(laplacian) / contrast)
}

fn variance(values: impl Iterator<Item = f32>) -> f32 {
    let (mut n, mut sum, mut squares) = (0.0_f64, 0.0_f64, 0.0_f64);
    for v in values {
        let v = f64::from(v);
        n += 1.0;
        sum += v;
        squares += v * v;
    }
    if n == 0.0 {
        return 0.0;
    }
    (squares / n - (sum / n) * (sum / n)).max(0.0) as f32
}

/// The steps between the blocks of eight of a JPEG against the ones inside.
fn blockiness(luma: &[f32], w: usize, h: usize) -> f32 {
    let (mut edge, mut edges, mut inner, mut inners) = (0.0_f64, 0_u32, 0.0_f64, 0_u32);
    for y in 0..h {
        let row = &luma[y * w..(y + 1) * w];
        for x in 1..w {
            let step = f64::from((row[x] - row[x - 1]).abs());
            if x % 8 == 0 {
                edge += step;
                edges += 1;
            } else {
                inner += step;
                inners += 1;
            }
        }
    }
    if edges == 0 || inners == 0 {
        return 0.0;
    }
    let (edge, inner) = (edge / f64::from(edges), inner / f64::from(inners));
    // Half a level of slack: a flat picture has no blocks.
    (edge / (inner + 0.5)) as f32
}
