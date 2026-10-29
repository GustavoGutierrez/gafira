use gafira_color::{Color, LinearRgba};
use gafira_geom::{Transform2D, Vec2};

/// A value that can be blended between two keys (ARQ-19).
pub trait Interpolate: Clone + PartialEq + Send + Sync + 'static {
    /// Blends `self` towards `to` by `t`. `t = 0` returns `self` and `t = 1`
    /// returns `to`, both exactly. `t` outside `[0, 1]` extrapolates.
    fn interpolate(&self, to: &Self, t: f32) -> Self;

    /// `true` when every component is finite. A key whose value is not finite
    /// makes the track invalid (ARQ-18).
    fn is_finite(&self) -> bool;
}

impl Interpolate for f32 {
    fn interpolate(&self, to: &Self, t: f32) -> Self {
        if t == 0.0 {
            return *self;
        }
        if t == 1.0 {
            return *to;
        }
        self + (to - self) * t
    }

    fn is_finite(&self) -> bool {
        f32::is_finite(*self)
    }
}

impl Interpolate for Vec2 {
    fn interpolate(&self, to: &Self, t: f32) -> Self {
        if t == 0.0 {
            return *self;
        }
        if t == 1.0 {
            return *to;
        }
        let t = f64::from(t);
        Vec2::new(self.x + (to.x - self.x) * t, self.y + (to.y - self.y) * t)
    }

    fn is_finite(&self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

/// Linear-light blend with premultiplied alpha (Q-27, CSS Color 4). The result
/// keeps the space both ends share and is linear sRGB otherwise. When the
/// resulting alpha is not positive the channels blend straight (Q-31). Nothing
/// is clamped.
impl Interpolate for Color {
    fn interpolate(&self, to: &Self, t: f32) -> Self {
        if t == 0.0 {
            return *self;
        }
        if t == 1.0 {
            return *to;
        }
        let (a, b) = (self.to_linear_srgb(), to.to_linear_srgb());
        let alpha = a.a.interpolate(&b.a, t);
        let t = f64::from(t);
        let (wa, wb, wr) = (f64::from(a.a), f64::from(b.a), f64::from(alpha));
        let blend = |ca: f32, cb: f32| {
            let (ca, cb) = (f64::from(ca), f64::from(cb));
            let value = if wr <= 0.0 {
                ca + (cb - ca) * t
            } else {
                let (pa, pb) = (ca * wa, cb * wb);
                (pa + (pb - pa) * t) / wr
            };
            value as f32
        };
        let linear = Color::LinearSrgb(LinearRgba::new(
            blend(a.r, b.r),
            blend(a.g, b.g),
            blend(a.b, b.b),
            alpha,
        ));
        if self.space() == to.space() {
            linear.convert(self.space())
        } else {
            linear
        }
    }

    fn is_finite(&self) -> bool {
        Color::is_finite(*self)
    }
}

/// Blends the decomposition of both ends (§3.4): translation, `sx > 0`, `sy`
/// signed by the determinant, shear and rotation by the shortest path. A
/// degenerate end borrows the rotation and shear of the other (Q-28, Q-30).
impl Interpolate for Transform2D {
    fn interpolate(&self, to: &Self, t: f32) -> Self {
        if t == 0.0 {
            return *self;
        }
        if t == 1.0 {
            return *to;
        }
        let (a, b) = (self.coeffs(), to.coeffs());
        let t = f64::from(t);
        let parts = match (Parts::decompose(a), Parts::decompose(b)) {
            (Some(pa), Some(pb)) => Some((pa, pb)),
            (None, Some(pb)) if is_finite6(&a) => Some((Parts::borrowing(a, &pb), pb)),
            (Some(pa), None) if is_finite6(&b) => Some((pa, Parts::borrowing(b, &pa))),
            _ => None,
        };
        match parts {
            Some((pa, pb)) => pa.blend(&pb, t).recompose(),
            None => {
                let mut out = [0.0; 6];
                for (o, (x, y)) in out.iter_mut().zip(a.iter().zip(&b)) {
                    *o = lerp(*x, *y, t);
                }
                Transform2D::from(out)
            }
        }
    }

    fn is_finite(&self) -> bool {
        is_finite6(&self.coeffs())
    }
}

fn is_finite6(c: &[f64; 6]) -> bool {
    c.iter().all(|v| v.is_finite())
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Translation, scales, shear and rotation of an affine transform.
#[derive(Clone, Copy)]
struct Parts {
    tx: f64,
    ty: f64,
    sx: f64,
    sy: f64,
    k: f64,
    theta: f64,
}

impl Parts {
    /// `None` for a degenerate end (Q-33): non-finite coefficients, a
    /// non-finite `det` or `|c1|·|c2|`, or `|det| <= 1e-12·|c1|·|c2|` (the sine
    /// of the angle between the columns), all in `f64`.
    fn decompose(c: [f64; 6]) -> Option<Self> {
        let [a, b, c_, d, tx, ty] = c;
        let det = a * d - b * c_;
        let area = a.hypot(b) * c_.hypot(d);
        if !is_finite6(&c) || !det.is_finite() || !area.is_finite() || det.abs() <= 1e-12 * area {
            return None;
        }
        let sx = a.hypot(b);
        let (ux, uy) = (a / sx, b / sx);
        let shear = ux * c_ + uy * d;
        let (px, py) = (c_ - shear * ux, d - shear * uy);
        let sy = px.hypot(py).copysign(det);
        let parts = Self {
            tx,
            ty,
            sx,
            sy,
            k: shear / sy,
            theta: uy.atan2(ux),
        };
        let all = [parts.sx, parts.sy, parts.k, parts.theta];
        all.iter().all(|v| v.is_finite()).then_some(parts)
    }

    /// A degenerate end in the frame (rotation, shear) of `frame`: its scales
    /// are the diagonal of its linear part in that frame.
    fn borrowing(c: [f64; 6], frame: &Self) -> Self {
        let [a, b, c_, d, tx, ty] = c;
        let (s, co) = frame.theta.sin_cos();
        let k = frame.k;
        Self {
            tx,
            ty,
            sx: a * (k * s + co) - b * (k * co - s),
            sy: d * co - c_ * s,
            k,
            theta: frame.theta,
        }
    }

    fn blend(&self, to: &Self, t: f64) -> Self {
        let mut dtheta = to.theta - self.theta;
        if dtheta > std::f64::consts::PI {
            dtheta -= std::f64::consts::TAU;
        } else if dtheta < -std::f64::consts::PI {
            dtheta += std::f64::consts::TAU;
        }
        Self {
            tx: lerp(self.tx, to.tx, t),
            ty: lerp(self.ty, to.ty, t),
            sx: lerp(self.sx, to.sx, t),
            sy: lerp(self.sy, to.sy, t),
            k: lerp(self.k, to.k, t),
            theta: self.theta + dtheta * t,
        }
    }

    fn recompose(&self) -> Transform2D {
        let (s, c) = self.theta.sin_cos();
        Transform2D::from([
            self.sx * c,
            self.sx * s,
            self.sy * (self.k * c - s),
            self.sy * (self.k * s + c),
            self.tx,
            self.ty,
        ])
    }
}
