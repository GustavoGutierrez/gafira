use serde::{Deserialize, Serialize};

/// Progress curve applied between two keys (ARQ-17).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Easing {
    Hold,
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    CubicBezier { x1: f32, y1: f32, x2: f32, y2: f32 },
}

const EASE_IN: [f32; 4] = [0.42, 0.0, 1.0, 1.0];
const EASE_OUT: [f32; 4] = [0.0, 0.0, 0.58, 1.0];
const EASE_IN_OUT: [f32; 4] = [0.42, 0.0, 0.58, 1.0];

/// Solver stop threshold on the last step in `s`. A bound on the error in `x`
/// is not enough: near a flat inflection (`x1 = 1`, `x2 ~ 0`) `x(s) - u` is
/// cubic in the distance to the root, so a tiny `x` error hides a large `s`
/// (and `y`) error. A step of at most `S_EPSILON` bounds the error in `s` by a
/// small multiple of it (bisection halves the bracket; Newton converges at
/// least linearly at a triple root).
const S_EPSILON: f64 = 1e-12;
const MAX_ITERATIONS: usize = 64;

impl Easing {
    /// `true` when the curve satisfies its validity rule: a `CubicBezier` needs
    /// `x1` and `x2` in `[0, 1]` and finite `y1` and `y2`.
    pub fn is_valid(&self) -> bool {
        match *self {
            Self::CubicBezier { x1, y1, x2, y2 } => {
                (0.0..=1.0).contains(&x1)
                    && (0.0..=1.0).contains(&x2)
                    && y1.is_finite()
                    && y2.is_finite()
            }
            _ => true,
        }
    }

    /// Evaluates `e(u)` for progress `u`.
    ///
    /// `e(0) = 0` and `e(1) = 1` hold exactly; `u` outside `[0, 1]` (or NaN)
    /// is treated as the nearest end. `Hold` is a step that reaches 1 at `u = 1`.
    pub fn apply(&self, u: f32) -> f32 {
        if u.is_nan() || u <= 0.0 {
            return 0.0;
        }
        if u >= 1.0 {
            return 1.0;
        }
        match *self {
            Self::Hold => 0.0,
            Self::Linear => u,
            Self::EaseIn => bezier(EASE_IN, u),
            Self::EaseOut => bezier(EASE_OUT, u),
            Self::EaseInOut => bezier(EASE_IN_OUT, u),
            Self::CubicBezier { x1, y1, x2, y2 } => bezier([x1, y1, x2, y2], u),
        }
    }
}

/// One coordinate of the Bezier with end points 0 and 1.
fn coord(p1: f64, p2: f64, s: f64) -> f64 {
    let m = 1.0 - s;
    3.0 * m * m * s * p1 + 3.0 * m * s * s * p2 + s * s * s
}

fn slope(p1: f64, p2: f64, s: f64) -> f64 {
    let m = 1.0 - s;
    3.0 * m * m * p1 + 6.0 * m * s * (p2 - p1) + 3.0 * s * s * (1.0 - p2)
}

/// Solves `x(s) = u` (Newton, safeguarded by a bisection bracket) and returns `y(s)`
/// clamped to the convex-hull bounds.
fn bezier([x1, y1, x2, y2]: [f32; 4], u: f32) -> f32 {
    let (x1, y1, x2, y2, u) = (x1.into(), y1.into(), x2.into(), y2.into(), f64::from(u));
    let (mut lo, mut hi, mut s) = (0.0_f64, 1.0_f64, u);
    for _ in 0..MAX_ITERATIONS {
        let err = coord(x1, x2, s) - u;
        if err < 0.0 {
            lo = s;
        } else {
            hi = s;
        }
        let d = slope(x1, x2, s);
        let next = s - err / d;
        let next = if d.abs() > 1e-9 && next > lo && next < hi {
            next
        } else {
            0.5 * (lo + hi)
        };
        let done = (next - s).abs() <= S_EPSILON;
        s = next;
        if done {
            break;
        }
    }
    let y = coord(y1, y2, s);
    let (min, max) = (0.0_f64.min(y1).min(y2), 1.0_f64.max(y1).max(y2));
    y.clamp(min, max) as f32
}
