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
