//! Guided-filter upscale of the coarse mask to the full image (P12): radius 8,
//! luma guide, serial row bands of 250 (XP11).

use gafira_doc::contract::Cancelled;
use rayon::prelude::*;

use crate::meter::Meter;
use crate::work::Source;

/// Window radius in pixels (02 §4.7).
const RADIUS: usize = 8;
/// Rows per band (XP11).
const BAND_ROWS: usize = 250;
/// Regularisation of the local linear model, on luma in `[0, 1]`.
const EPSILON: f32 = 1e-3;

/// Planned work units of [`upscale`].
pub(crate) fn planned(w: usize, h: usize) -> u64 {
    (w * h) as u64 * 4
}

/// Bilinear weights of one output column against the coarse grid.
struct Tap {
    lo: usize,
    hi: usize,
    t: f32,
}

fn taps(out: usize, coarse: usize) -> Vec<Tap> {
    (0..out)
        .map(|i| {
            let f = ((i as f32 + 0.5) * coarse as f32 / out as f32 - 0.5)
                .clamp(0.0, (coarse - 1) as f32);
            let lo = f as usize;
            Tap {
                lo,
                hi: (lo + 1).min(coarse - 1),
                t: f - lo as f32,
            }
        })
        .collect()
}

/// Mean over the `(2r+1)` square window clipped to the buffer, via `tmp`.
fn box_mean(src: &[f32], dst: &mut [f32], tmp: &mut [f32], w: usize, rows: usize) {
    let r = RADIUS;
    for y in 0..rows {
        let row = &src[y * w..(y + 1) * w];
        let out = &mut tmp[y * w..(y + 1) * w];
        let mut sum: f64 = row[..r.min(w - 1) + 1].iter().map(|&v| f64::from(v)).sum();
        for x in 0..w {
            if x > 0 {
                if x + r < w {
                    sum += f64::from(row[x + r]);
                }
                if x > r {
                    sum -= f64::from(row[x - r - 1]);
                }
            }
            let (lo, hi) = (x.saturating_sub(r), (x + r).min(w - 1));
            out[x] = (sum / (hi - lo + 1) as f64) as f32;
        }
    }
    let mut acc = vec![0f64; w];
    for y in 0..r.min(rows - 1) + 1 {
        for (a, &v) in acc.iter_mut().zip(&tmp[y * w..(y + 1) * w]) {
            *a += f64::from(v);
        }
    }
    for y in 0..rows {
        if y > 0 {
            if y + r < rows {
                for (a, &v) in acc.iter_mut().zip(&tmp[(y + r) * w..(y + r + 1) * w]) {
                    *a += f64::from(v);
                }
            }
            if y > r {
                for (a, &v) in acc.iter_mut().zip(&tmp[(y - r - 1) * w..(y - r) * w]) {
                    *a -= f64::from(v);
                }
            }
        }
        let (lo, hi) = (y.saturating_sub(r), (y + r).min(rows - 1));
        let n = (hi - lo + 1) as f64;
        for (d, &a) in dst[y * w..(y + 1) * w].iter_mut().zip(&acc) {
            *d = (a / n) as f32;
        }
    }
}

/// Scales `coarse` (`cw` x `ch`, 0 or 255) to the source size, snapping its
/// edges to the colour edges of the source.
pub(crate) fn upscale(
    src: &Source<'_>,
    coarse: &[u8],
    (cw, ch): (usize, usize),
    pool: &rayon::ThreadPool,
    meter: &Meter<'_>,
) -> Result<Vec<u8>, Cancelled> {
    let (w, h) = (src.width, src.height);
    let xs = taps(w, cw);
    let ys = taps(h, ch);
    let mut mask = vec![0u8; w * h];
    pool.install(|| {
        mask.par_chunks_mut(BAND_ROWS * w)
            .enumerate()
            .try_for_each(|(band, out)| {
                meter.step((out.len() * 4) as u64)?;
                let y0 = band * BAND_ROWS;
                let y1 = y0 + out.len() / w;
                let (ya, yb) = (y0.saturating_sub(2 * RADIUS), (y1 + 2 * RADIUS).min(h));
                let rows = yb - ya;
                let n = rows * w;
                let (mut guide, mut p) = (vec![0f32; n], vec![0f32; n]);
                for (j, y) in (ya..yb).enumerate() {
                    let (top, bottom) = (&ys[y], ys[y].hi);
                    for x in 0..w {
                        guide[j * w + x] = src.luma(y * w + x);
                        let at = |yy: usize| {
                            let row = &coarse[yy * cw..(yy + 1) * cw];
                            let (a, b) = (f32::from(row[xs[x].lo]), f32::from(row[xs[x].hi]));
                            (a + (b - a) * xs[x].t) / 255.0
                        };
                        let (u, v) = (at(top.lo), at(bottom));
                        p[j * w + x] = u + (v - u) * top.t;
                    }
                }
                let (mut tmp, mut prod) = (vec![0f32; n], vec![0f32; n]);
                let (mut m_i, mut m_p, mut m_ip, mut m_ii) =
                    (vec![0f32; n], vec![0f32; n], vec![0f32; n], vec![0f32; n]);
                box_mean(&guide, &mut m_i, &mut tmp, w, rows);
                box_mean(&p, &mut m_p, &mut tmp, w, rows);
                for ((d, &g), &q) in prod.iter_mut().zip(&guide).zip(&p) {
                    *d = g * q;
                }
                box_mean(&prod, &mut m_ip, &mut tmp, w, rows);
                for (d, &g) in prod.iter_mut().zip(&guide) {
                    *d = g * g;
                }
                box_mean(&prod, &mut m_ii, &mut tmp, w, rows);
                for i in 0..n {
                    let a = (m_ip[i] - m_i[i] * m_p[i]) / (m_ii[i] - m_i[i] * m_i[i] + EPSILON);
                    m_ip[i] = a;
                    m_p[i] -= a * m_i[i];
                }
                box_mean(&m_ip, &mut prod, &mut tmp, w, rows);
                box_mean(&m_p, &mut m_ii, &mut tmp, w, rows);
                let skip = (y0 - ya) * w;
                for (i, o) in out.iter_mut().enumerate() {
                    let q = prod[skip + i] * guide[skip + i] + m_ii[skip + i];
                    *o = (q.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                }
                Ok::<(), Cancelled>(())
            })
    })?;
    Ok(mask)
}
