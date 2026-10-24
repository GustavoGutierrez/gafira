//! Position, size and orientation of an element in page space.

use gafira_geom::Affine;

/// Mirroring of the local box about its center.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flip {
    /// Mirror left to right.
    pub x: bool,
    /// Mirror top to bottom.
    pub y: bool,
}

/// Where an element sits on its page, in logical px.
///
/// The local box is `[0,w] x [0,h]`; `w` and `h` size the box and never scale
/// it. Values are stored unchanged: nothing is clamped or normalized.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    /// Page-space x of the box's top-left corner.
    pub x: f64,
    /// Page-space y of the box's top-left corner.
    pub y: f64,
    /// Box width.
    pub w: f64,
    /// Box height.
    pub h: f64,
    /// Clockwise rotation in degrees (on a y-down page).
    pub rotation_deg: f64,
    /// X shear angle in degrees.
    pub skew: f64,
    /// Mirroring about the box center.
    pub flip: Flip,
}

impl Placement {
    /// An unrotated, unskewed, unflipped placement.
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self {
            x,
            y,
            w,
            h,
            rotation_deg: 0.0,
            skew: 0.0,
            flip: Flip { x: false, y: false },
        }
    }

    /// Maps local box coordinates to page space.
    ///
    /// Right to left: translate by minus the half size, mirror, shear x by
    /// `tan(skew)` per unit of y, rotate, translate to the box center. Total:
    /// non-finite fields give non-finite coefficients.
    pub fn to_transform(&self) -> Affine {
        let (hw, hh) = (self.w / 2.0, self.h / 2.0);
        let mirror = Affine::scale_non_uniform(
            if self.flip.x { -1.0 } else { 1.0 },
            if self.flip.y { -1.0 } else { 1.0 },
        );
        let shear = Affine::new([1.0, 0.0, self.skew.to_radians().tan(), 1.0, 0.0, 0.0]);
        Affine::translate((self.x + hw, self.y + hh))
            * Affine::rotate(self.rotation_deg.to_radians())
            * shear
            * mirror
            * Affine::translate((-hw, -hh))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gafira_geom::Point;
    use proptest::prelude::*;

    fn apply(p: &Placement, x: f64, y: f64) -> Point {
        p.to_transform() * Point::new(x, y)
    }

    fn assert_near(got: Point, x: f64, y: f64) {
        assert!(
            (got.x - x).abs() < 1e-12 && (got.y - y).abs() < 1e-12,
            "got {got:?}, want ({x}, {y})"
        );
    }

    #[test]
    fn new_sets_defaults() {
        let p = Placement::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!((p.x, p.y, p.w, p.h), (1.0, 2.0, 3.0, 4.0));
        assert_eq!(p.rotation_deg.to_bits(), 0.0f64.to_bits());
        assert_eq!(p.skew.to_bits(), 0.0f64.to_bits());
        assert_eq!(p.flip, Flip { x: false, y: false });
    }

    #[test]
    fn plain_placement() {
        let p = Placement::new(10.0, 20.0, 30.0, 40.0);
        assert_near(apply(&p, 0.0, 0.0), 10.0, 20.0);
        assert_near(apply(&p, 30.0, 40.0), 40.0, 60.0);
    }

    #[test]
    fn quarter_turn_clockwise() {
        let mut p = Placement::new(0.0, 0.0, 2.0, 2.0);
        p.rotation_deg = 90.0;
        assert_near(apply(&p, 2.0, 1.0), 1.0, 2.0);
    }

    #[test]
    fn flip() {
        let mut p = Placement::new(10.0, 0.0, 10.0, 10.0);
        p.flip.x = true;
        assert_near(apply(&p, 0.0, 0.0), 20.0, 0.0);
        p.flip = Flip { x: false, y: true };
        assert_near(apply(&p, 0.0, 0.0), 10.0, 10.0);
    }

    #[test]
    fn skew_45_degrees() {
        let mut p = Placement::new(0.0, 0.0, 2.0, 2.0);
        p.skew = 45.0;
        assert_near(apply(&p, 1.0, 2.0), 2.0, 2.0);
    }

    #[test]
    fn non_finite_does_not_panic() {
        let mut a = Placement::new(f64::NAN, 0.0, 1.0, 1.0);
        let _ = a.to_transform();
        a = Placement::new(0.0, 0.0, 1.0, 1.0);
        a.rotation_deg = f64::INFINITY;
        let _ = a.to_transform();
    }

    #[test]
    fn values_are_retained() {
        let mut p = Placement::new(-0.0, 0.0, 1.0, 1.0);
        p.rotation_deg = 720.0;
        assert_eq!(p.rotation_deg.to_bits(), 720.0f64.to_bits());
        assert_eq!(p.x.to_bits(), (-0.0f64).to_bits());
    }

    #[test]
    fn ieee_equality() {
        let a = Placement::new(-0.0, 0.0, 1.0, 1.0);
        let b = Placement::new(0.0, 0.0, 1.0, 1.0);
        assert_eq!(a, b);
        assert_ne!(a.x.to_bits(), b.x.to_bits());
        let n = Placement::new(f64::NAN, 0.0, 1.0, 1.0);
        assert_ne!(n, n);
    }

    proptest! {
        #[test]
        fn center_is_a_fixed_point(
            x in -1e4f64..1e4, y in -1e4f64..1e4,
            w in 0.0f64..1e4, h in 0.0f64..1e4,
            rot in -1e4f64..1e4, skew in -89.0f64..89.0,
            fx: bool, fy: bool,
        ) {
            let mut p = Placement::new(x, y, w, h);
            p.rotation_deg = rot;
            p.skew = skew;
            p.flip = Flip { x: fx, y: fy };
            let got = apply(&p, w / 2.0, h / 2.0);
            let (wx, wy) = (x + w / 2.0, y + h / 2.0);
            let tol = |want: f64| 1e-9 * want.abs().max(1.0);
            prop_assert!((got.x - wx).abs() <= tol(wx));
            prop_assert!((got.y - wy).abs() <= tol(wy));
        }
    }
}
