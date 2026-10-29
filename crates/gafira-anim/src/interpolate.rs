use gafira_color::{Color, LinearRgba};
use gafira_geom::Vec2;

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
