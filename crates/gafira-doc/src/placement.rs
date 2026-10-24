//! Position, size and orientation of an element in page space.

use gafira_geom::Affine;
use serde::{Deserialize, Serialize};

/// Mirroring of the local box about its center.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Flip {
    /// Mirror left to right.
    #[serde(default)]
    pub x: bool,
    /// Mirror top to bottom.
    #[serde(default)]
    pub y: bool,
}

/// Where an element sits on its page, in logical px.
///
/// The local box is `[0,w] x [0,h]`; `w` and `h` size the box and never scale
/// it. Values are stored unchanged: nothing is clamped or normalized.
///
/// `x`, `y`, `w` and `h` are required on the wire and there is no
/// container-level `#[serde(default)]`, so a keyed animation object fails.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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
    #[serde(default)]
    pub rotation_deg: f64,
    /// X shear angle in degrees.
    #[serde(default)]
    pub skew: f64,
    /// Mirroring about the box center.
    #[serde(default)]
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
    fn combined_rotation_skew_and_flip() {
        let mut p = Placement::new(0.0, 0.0, 2.0, 2.0);
        p.rotation_deg = 90.0;
        p.skew = 45.0;
        p.flip.x = true;
        // centered (1,1) -> mirror (-1,1) -> shear (0,1) -> rotate (-1,0) -> center (0,1)
        assert_near(apply(&p, 2.0, 2.0), 0.0, 1.0);
    }

    #[test]
    fn non_finite_gives_non_finite_coefficients() {
        let a = Placement::new(f64::NAN, 0.0, 1.0, 1.0);
        assert!(a.to_transform().as_coeffs().iter().any(|c| !c.is_finite()));
        let mut b = Placement::new(0.0, 0.0, 1.0, 1.0);
        b.rotation_deg = f64::INFINITY;
        assert!(b.to_transform().as_coeffs().iter().any(|c| !c.is_finite()));
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
            rot in -1e4f64..1e4, skew in (-90.0f64).next_up()..90.0,
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

    #[test]
    fn wire_form_is_flat_with_every_field() {
        let mut p = Placement::new(1.0, 2.0, 3.0, 4.0);
        p.flip.y = true;
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(
            json,
            r#"{"x":1.0,"y":2.0,"w":3.0,"h":4.0,"rotation_deg":0.0,"skew":0.0,"flip":{"x":false,"y":true}}"#
        );
        assert_eq!(serde_json::from_str::<Placement>(&json).unwrap(), p);
    }

    #[test]
    fn optional_fields_default_and_integers_are_accepted() {
        let p: Placement = serde_json::from_str(r#"{"x":3,"y":0,"w":1,"h":1}"#).unwrap();
        assert_eq!(p, Placement::new(3.0, 0.0, 1.0, 1.0));
    }

    #[test]
    fn partial_flip_defaults_missing_axes_to_false() {
        let load = |flip: &str| -> Flip {
            let json = format!(r#"{{"x":0,"y":0,"w":1,"h":1,"flip":{flip}}}"#);
            serde_json::from_str::<Placement>(&json).unwrap().flip
        };
        assert_eq!(load("{}"), Flip { x: false, y: false });
        assert_eq!(load(r#"{"x":true}"#), Flip { x: true, y: false });
        assert_eq!(load(r#"{"y":true}"#), Flip { x: false, y: true });
    }

    #[test]
    fn required_fields_and_keyed_objects() {
        for bad in [
            r#"{"y":0,"w":1,"h":1}"#,
            r#"{"x":0,"w":1,"h":1}"#,
            r#"{"x":0,"y":0,"h":1}"#,
            r#"{"x":0,"y":0,"w":1}"#,
            r#"{"keys":[]}"#,
            "{}",
        ] {
            assert!(serde_json::from_str::<Placement>(bad).is_err(), "{bad}");
        }
        assert!(serde_json::from_str::<gafira_color::Color>(r#"{"keys":[]}"#).is_err());
        assert!(serde_json::from_str::<f32>(r#"{"keys":[]}"#).is_err());
        assert_eq!(serde_json::from_str::<f32>("0.5").unwrap(), 0.5);
    }

    #[test]
    fn numbers_keep_their_bits() {
        let mut p = Placement::new(-0.0, 0.0, f64::MAX, 5e-324);
        p.rotation_deg = 5e-324;
        let back: Placement = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert!(back.x.is_sign_negative());
        for (a, b) in [
            (p.x, back.x),
            (p.w, back.w),
            (p.h, back.h),
            (p.rotation_deg, back.rotation_deg),
        ] {
            assert_eq!(a.to_bits(), b.to_bits());
        }
    }
}
