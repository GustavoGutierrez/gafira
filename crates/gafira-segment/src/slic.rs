//! Own SLIC superpixels on the working copy (P12): about 1530 cells, a few
//! assignment passes, then 4-connected labels.

use gafira_doc::contract::Cancelled;
use rayon::prelude::*;

use crate::meter::{CHUNK_ROWS, Meter};
use crate::table::PairTable;
use crate::work::WorkImage;

/// Superpixels aimed for (P12).
const TARGET: f64 = 1530.0;
/// Assignment passes.
const PASSES: usize = 4;
/// Weight of position against colour (sRGB byte scale).
const COMPACTNESS: f32 = 25.0;

#[derive(Clone, Copy)]
struct Centre {
    x: f32,
    y: f32,
    c: [f32; 3],
}

/// Labels of every pixel; each label is one 4-connected region.
pub(crate) struct Superpixels {
    pub(crate) labels: Vec<u32>,
    pub(crate) count: usize,
}

/// The grid of initial cells: `(columns, rows)`.
fn grid(w: usize, h: usize) -> (usize, usize) {
    let side = ((w * h) as f64 / TARGET).sqrt().max(1.0);
    let cols = ((w as f64 / side).round() as usize).clamp(1, w);
    let rows = ((h as f64 / side).round() as usize).clamp(1, h);
    (cols, rows)
}

/// Planned work units of [`slic`], for the progress model.
pub(crate) fn planned(w: usize, h: usize) -> u64 {
    let px = (w * h) as u64;
    px * (PASSES as u64 * 4 + 1)
}

pub(crate) fn slic(
    img: &WorkImage,
    pool: &rayon::ThreadPool,
    meter: &Meter<'_>,
) -> Result<Superpixels, Cancelled> {
    let (w, h) = (img.width, img.height);
    let (cols, rows) = grid(w, h);
    let k = cols * rows;
    let cell_area = (w as f32 / cols as f32) * (h as f32 / rows as f32);
    let weight = COMPACTNESS * COMPACTNESS / cell_area;
    let mut centres: Vec<Centre> = (0..k)
        .map(|i| {
            let x = ((i % cols) as f32 + 0.5) * w as f32 / cols as f32;
            let y = ((i / cols) as f32 + 0.5) * h as f32 / rows as f32;
            let p = img.rgb[(y as usize).min(h - 1) * w + (x as usize).min(w - 1)];
            Centre {
                x,
                y,
                c: p.map(f32::from),
            }
        })
        .collect();
    let mut labels = vec![0u32; w * h];
    for _ in 0..PASSES {
        let snapshot = &centres;
        pool.install(|| {
            labels
                .par_chunks_mut(CHUNK_ROWS * w)
                .enumerate()
                .try_for_each(|(chunk, out)| {
                    meter.step((out.len() * 3) as u64)?;
                    let y_base = chunk * CHUNK_ROWS;
                    for (j, slot) in out.iter_mut().enumerate() {
                        let (x, y) = (j % w, y_base + j / w);
                        *slot = nearest(img, snapshot, (cols, rows), (x, y), weight);
                    }
                    Ok::<(), Cancelled>(())
                })
        })?;
        centres = recentre(img, &labels, &centres, meter)?;
    }
    connect(&labels, (w, h), (w * h) / k / 4, meter)
}

/// The label of the closest centre among the 3x3 cells around `(x, y)`.
fn nearest(
    img: &WorkImage,
    centres: &[Centre],
    (cols, rows): (usize, usize),
    (x, y): (usize, usize),
    weight: f32,
) -> u32 {
    let (w, h) = (img.width, img.height);
    let (cx, cy) = (x * cols / w, y * rows / h);
    let p = img.rgb[y * w + x].map(f32::from);
    let mut best = (f32::INFINITY, 0u32);
    for ny in cy.saturating_sub(1)..=(cy + 1).min(rows - 1) {
        for nx in cx.saturating_sub(1)..=(cx + 1).min(cols - 1) {
            let id = ny * cols + nx;
            let c = centres[id];
            let dc = (p[0] - c.c[0]).powi(2) + (p[1] - c.c[1]).powi(2) + (p[2] - c.c[2]).powi(2);
            let ds = (x as f32 - c.x).powi(2) + (y as f32 - c.y).powi(2);
            let d = dc + weight * ds;
            if d < best.0 {
                best = (d, id as u32);
            }
        }
    }
    best.1
}

/// Moves each centre to the mean of its pixels, with integer sums so the
/// result does not depend on the thread count.
fn recentre(
    img: &WorkImage,
    labels: &[u32],
    old: &[Centre],
    meter: &Meter<'_>,
) -> Result<Vec<Centre>, Cancelled> {
    let w = img.width;
    let mut sums = vec![[0u64; 6]; old.len()];
    for y0 in (0..img.height).step_by(CHUNK_ROWS) {
        let y1 = (y0 + CHUNK_ROWS).min(img.height);
        meter.step(((y1 - y0) * w) as u64)?;
        for y in y0..y1 {
            for x in 0..w {
                let i = y * w + x;
                let s = &mut sums[labels[i] as usize];
                let [r, g, b] = img.rgb[i];
                s[0] += 1;
                s[1] += x as u64;
                s[2] += y as u64;
                s[3] += u64::from(r);
                s[4] += u64::from(g);
                s[5] += u64::from(b);
            }
        }
    }
    Ok(sums
        .iter()
        .zip(old)
        .map(|(s, c)| {
            if s[0] == 0 {
                return *c;
            }
            let n = s[0] as f32;
            Centre {
                x: s[1] as f32 / n + 0.5,
                y: s[2] as f32 / n + 0.5,
                c: [s[3] as f32 / n, s[4] as f32 / n, s[5] as f32 / n],
            }
        })
        .collect())
}

/// Relabels so each region is 4-connected: a piece smaller than `min_size`
/// joins the region seen just before it.
fn connect(
    labels: &[u32],
    (w, h): (usize, usize),
    min_size: usize,
    meter: &Meter<'_>,
) -> Result<Superpixels, Cancelled> {
    let mut out = vec![u32::MAX; w * h];
    let mut members: Vec<u32> = Vec::new();
    let mut count = 0u32;
    for y in 0..h {
        if y % CHUNK_ROWS == 0 {
            meter.step((CHUNK_ROWS.min(h - y) * w) as u64)?;
        }
        for x in 0..w {
            let start = y * w + x;
            if out[start] != u32::MAX {
                continue;
            }
            let previous = if x > 0 {
                Some(out[start - 1])
            } else if y > 0 {
                Some(out[start - w])
            } else {
                None
            };
            let old = labels[start];
            members.clear();
            members.push(start as u32);
            out[start] = u32::MAX - 1;
            let mut head = 0;
            while head < members.len() {
                let i = members[head] as usize;
                head += 1;
                let (px, py) = (i % w, i / w);
                let around = [
                    (px > 0).then(|| i - 1),
                    (px + 1 < w).then(|| i + 1),
                    (py > 0).then(|| i - w),
                    (py + 1 < h).then(|| i + w),
                ];
                for n in around.into_iter().flatten() {
                    if out[n] == u32::MAX && labels[n] == old {
                        out[n] = u32::MAX - 1;
                        members.push(n as u32);
                    }
                }
            }
            let id = match previous {
                Some(p) if members.len() < min_size => p,
                _ => {
                    count += 1;
                    count - 1
                }
            };
            for &m in &members {
                out[m as usize] = id;
            }
        }
    }
    Ok(Superpixels {
        labels: out,
        count: count as usize,
    })
}

/// Distinct pairs of touching superpixels with the length of their shared border,
/// the number of 4-neighbour pixel pairs with a pixel in each (ARQ-133), sorted.
/// A planar graph of `k` connected regions has at most `3k` edges, which sizes
/// the table (XP11).
pub(crate) fn adjacency(
    sp: &Superpixels,
    (w, h): (usize, usize),
    meter: &Meter<'_>,
) -> Result<Vec<(u32, u32, u32)>, Cancelled> {
    let mut table = PairTable::with_bound(3 * sp.count);
    for y0 in (0..h).step_by(CHUNK_ROWS) {
        let y1 = (y0 + CHUNK_ROWS).min(h);
        meter.step(((y1 - y0) * w) as u64)?;
        for y in y0..y1 {
            for x in 0..w {
                let i = y * w + x;
                let l = sp.labels[i];
                if x + 1 < w && sp.labels[i + 1] != l {
                    table.insert(l, sp.labels[i + 1]);
                }
                if y + 1 < h && sp.labels[i + w] != l {
                    table.insert(l, sp.labels[i + w]);
                }
            }
        }
    }
    Ok(table.drain_sorted())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gafira_doc::contract::{CancelToken, NoProgress};

    // ARQ-133: the length of a shared border is the number of 4-neighbour pixel
    // pairs with a pixel in each superpixel.
    #[test]
    fn arq_133_the_shared_border_counts_pixel_pairs() {
        // 0 0 1
        // 0 2 1
        // 2 2 1
        let sp = Superpixels {
            labels: vec![0, 0, 1, 0, 2, 1, 2, 2, 1],
            count: 3,
        };
        let (progress, cancel) = (NoProgress, CancelToken::new());
        let meter = Meter::new(&progress, &cancel, 1 << 30);
        let edges = adjacency(&sp, (3, 3), &meter).unwrap();
        assert_eq!(edges, [(0, 1, 1), (0, 2, 3), (1, 2, 2)]);
    }
}
