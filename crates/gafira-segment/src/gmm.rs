//! A colour-only Gaussian mixture (D-45): samples are Lab colours and nothing
//! else, so position cannot enter the cost (ARQ-130).

/// Components per model (P12).
const COMPONENTS: usize = 5;
/// Variance added to every axis of every component, in Lab units squared.
/// ARQ-133 keeps it from 20 to 30; tuned on the 16 gate scenes only.
pub(crate) const REGULARISATION: f64 = 25.0;
const LLOYD_STEPS: usize = 5;

type V3 = [f64; 3];

struct Component {
    /// `ln(weight) - ½ ln det`: the part of the log-density that ignores the sample.
    base: f64,
    mean: V3,
    /// Inverse covariance, `[xx, xy, xz, yy, yz, zz]`.
    inv: [f64; 6],
}

pub(crate) struct Gmm {
    parts: Vec<Component>,
}

fn dist2(a: V3, b: V3) -> f64 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

impl Gmm {
    /// Fits the mixture to colours with pixel-count weights. `None` with no sample.
    pub(crate) fn fit(samples: &[V3], weights: &[f64]) -> Option<Gmm> {
        let n = samples.len();
        if n == 0 {
            return None;
        }
        let k = COMPONENTS.min(n);
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| {
            samples[a][0]
                .total_cmp(&samples[b][0])
                .then(samples[a][1].total_cmp(&samples[b][1]))
                .then(samples[a][2].total_cmp(&samples[b][2]))
                .then(a.cmp(&b))
        });
        let mut centres: Vec<V3> = (0..k)
            .map(|j| samples[order[(2 * j + 1) * n / (2 * k)]])
            .collect();
        let mut owner = vec![0usize; n];
        for step in 0..=LLOYD_STEPS {
            for (i, s) in samples.iter().enumerate() {
                let mut best = (f64::INFINITY, 0);
                for (j, c) in centres.iter().enumerate() {
                    let d = dist2(*s, *c);
                    if d < best.0 {
                        best = (d, j);
                    }
                }
                owner[i] = best.1;
            }
            if step == LLOYD_STEPS {
                break;
            }
            let mut acc = vec![([0.0; 3], 0.0); k];
            for (i, s) in samples.iter().enumerate() {
                let (sum, w) = &mut acc[owner[i]];
                for a in 0..3 {
                    sum[a] += weights[i] * s[a];
                }
                *w += weights[i];
            }
            for (c, (sum, w)) in centres.iter_mut().zip(&acc) {
                if *w > 0.0 {
                    *c = sum.map(|v| v / w);
                }
            }
        }
        let total: f64 = weights.iter().sum();
        let mut parts = Vec::with_capacity(k);
        for (j, c) in centres.iter().enumerate() {
            let mut w = 0.0;
            let mut cov = [0.0; 6];
            for (i, s) in samples.iter().enumerate().filter(|(i, _)| owner[*i] == j) {
                let d = [s[0] - c[0], s[1] - c[1], s[2] - c[2]];
                w += weights[i];
                for (slot, (a, b)) in
                    cov.iter_mut()
                        .zip([(0, 0), (0, 1), (0, 2), (1, 1), (1, 2), (2, 2)])
                {
                    *slot += weights[i] * d[a] * d[b];
                }
            }
            if w <= 0.0 {
                continue;
            }
            let mut cov = cov.map(|v| v / w);
            for diagonal in [0, 3, 5] {
                cov[diagonal] += REGULARISATION;
            }
            let (inv, det) = invert(cov);
            parts.push(Component {
                base: (w / total).ln() - 0.5 * det.ln(),
                mean: *c,
                inv,
            });
        }
        Some(Gmm { parts })
    }

    /// `-ln` of the mixture density at `x`, without the constant shared by all models.
    pub(crate) fn cost(&self, x: V3) -> f64 {
        let logs: Vec<f64> = self
            .parts
            .iter()
            .map(|p| {
                let d = [x[0] - p.mean[0], x[1] - p.mean[1], x[2] - p.mean[2]];
                let q = p.inv[0] * d[0] * d[0]
                    + p.inv[3] * d[1] * d[1]
                    + p.inv[5] * d[2] * d[2]
                    + 2.0
                        * (p.inv[1] * d[0] * d[1]
                            + p.inv[2] * d[0] * d[2]
                            + p.inv[4] * d[1] * d[2]);
                p.base - 0.5 * q
            })
            .collect();
        let top = logs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let sum: f64 = logs.iter().map(|l| (l - top).exp()).sum();
        -(top + sum.ln())
    }
}

/// Inverse and determinant of a symmetric positive-definite 3x3 matrix.
fn invert([xx, xy, xz, yy, yz, zz]: [f64; 6]) -> ([f64; 6], f64) {
    let c00 = yy * zz - yz * yz;
    let c01 = xz * yz - xy * zz;
    let c02 = xy * yz - xz * yy;
    let det = xx * c00 + xy * c01 + xz * c02;
    let c11 = xx * zz - xz * xz;
    let c12 = xy * xz - xx * yz;
    let c22 = xx * yy - xy * xy;
    (
        [
            c00 / det,
            c01 / det,
            c02 / det,
            c11 / det,
            c12 / det,
            c22 / det,
        ],
        det,
    )
}
