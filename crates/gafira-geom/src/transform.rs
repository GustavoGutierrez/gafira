//! A 2D affine transform wrapping [`kurbo::Affine`].

use std::fmt;

use kurbo::{Affine, Point, Vec2};
use serde::{Deserialize, Serialize};

/// A 2D affine transform.
///
/// Serialized as the six coefficients `[a, b, c, d, e, f]` of the wrapped
/// [`Affine`]. Composition is only available through [`Transform2D::then`].
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "[f64; 6]", into = "[f64; 6]")]
pub struct Transform2D(Affine);

impl Transform2D {
    /// The transform that leaves every point unchanged.
    pub const IDENTITY: Self = Self(Affine::IDENTITY);

    /// A translation by `offset`.
    pub fn translation(offset: Vec2) -> Self {
        Self(Affine::translate(offset))
    }

    /// A uniform scale about the origin.
    pub const fn scale(factor: f64) -> Self {
        Self(Affine::scale(factor))
    }

    /// A non-uniform scale about the origin.
    pub const fn scale_xy(sx: f64, sy: f64) -> Self {
        Self(Affine::scale_non_uniform(sx, sy))
    }

    /// A rotation about the origin by `radians`.
    pub fn rotation(radians: f64) -> Self {
        Self(Affine::rotate(radians))
    }

    /// Applies `self` first, then `next`.
    #[must_use]
    pub fn then(self, next: Self) -> Self {
        // kurbo multiplies right to left: `a * b` applies `b` first.
        Self(next.0 * self.0)
    }

    /// Maps a point through the transform.
    pub fn apply(self, p: Point) -> Point {
        self.0 * p
    }

    /// The determinant of the linear part.
    pub const fn determinant(self) -> f64 {
        self.0.determinant()
    }

    /// The inverse transform.
    ///
    /// Fails when the determinant is zero or not finite, or when any inverse
    /// coefficient is not finite. Never returns NaN coefficients.
    pub fn inverse(self) -> Result<Self, SingularTransform> {
        let det = self.determinant();
        if det == 0.0 || !det.is_finite() {
            return Err(SingularTransform);
        }
        let inverse = self.0.inverse();
        if inverse.as_coeffs().iter().all(|c| c.is_finite()) {
            Ok(Self(inverse))
        } else {
            Err(SingularTransform)
        }
    }

    /// The coefficients `[a, b, c, d, e, f]`.
    pub const fn coeffs(self) -> [f64; 6] {
        self.0.as_coeffs()
    }
}

impl From<Affine> for Transform2D {
    fn from(affine: Affine) -> Self {
        Self(affine)
    }
}

impl From<Transform2D> for Affine {
    fn from(transform: Transform2D) -> Self {
        transform.0
    }
}

impl From<[f64; 6]> for Transform2D {
    fn from(coeffs: [f64; 6]) -> Self {
        Self(Affine::new(coeffs))
    }
}

impl From<Transform2D> for [f64; 6] {
    fn from(transform: Transform2D) -> Self {
        transform.coeffs()
    }
}

/// The transform has no inverse.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SingularTransform;

impl fmt::Display for SingularTransform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("transform is singular and cannot be inverted")
    }
}

impl std::error::Error for SingularTransform {}

#[cfg(test)]
mod tests {
    use super::*;

    const IDENTITY_COEFFS: [f64; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

    fn bits(coeffs: [f64; 6]) -> [u64; 6] {
        coeffs.map(f64::to_bits)
    }

    fn close(actual: Point, expected: Point) -> bool {
        (actual.x - expected.x).abs() <= 1e-9 * expected.x.abs().max(1.0)
            && (actual.y - expected.y).abs() <= 1e-9 * expected.y.abs().max(1.0)
    }

    #[test]
    fn identity_has_identity_coefficients_and_fixes_points() {
        assert_eq!(bits(Transform2D::IDENTITY.coeffs()), bits(IDENTITY_COEFFS));
        let p = Point::new(3.5, -7.25);
        let mapped = Transform2D::IDENTITY.apply(p);
        assert_eq!(mapped.x.to_bits(), p.x.to_bits());
        assert_eq!(mapped.y.to_bits(), p.y.to_bits());
    }

    #[test]
    fn translate_then_scale_maps_one_one_to_six_eight() {
        let t = Transform2D::translation(Vec2::new(2.0, 3.0)).then(Transform2D::scale(2.0));
        assert_eq!(t.apply(Point::new(1.0, 1.0)), Point::new(6.0, 8.0));
    }

    #[test]
    fn composition_order_is_apply_first_then_next() {
        let t = Transform2D::scale(2.0).then(Transform2D::translation(Vec2::new(2.0, 3.0)));
        assert_eq!(t.apply(Point::new(1.0, 1.0)), Point::new(4.0, 5.0));
        let with_identity = Transform2D::scale(2.0).then(Transform2D::IDENTITY);
        assert_eq!(
            bits(with_identity.coeffs()),
            bits(Transform2D::scale(2.0).coeffs())
        );
    }

    #[test]
    fn primitive_constructors_map_points() {
        assert_eq!(
            Transform2D::translation(Vec2::new(-1.0, 4.0)).apply(Point::new(1.0, 1.0)),
            Point::new(0.0, 5.0)
        );
        assert_eq!(
            Transform2D::scale_xy(2.0, -3.0).apply(Point::new(1.0, 1.0)),
            Point::new(2.0, -3.0)
        );
        let rotated =
            Transform2D::rotation(std::f64::consts::FRAC_PI_2).apply(Point::new(1.0, 0.0));
        assert!(close(rotated, Point::new(0.0, 1.0)), "{rotated:?}");
    }

    #[test]
    fn determinant_is_the_area_scale() {
        assert_eq!(Transform2D::scale_xy(2.0, 3.0).determinant(), 6.0);
        assert_eq!(Transform2D::IDENTITY.determinant(), 1.0);
        assert_eq!(Transform2D::scale(0.0).determinant(), 0.0);
    }

    #[test]
    fn inverse_undoes_the_transform() {
        let t = Transform2D::rotation(0.7)
            .then(Transform2D::scale_xy(2.0, 0.5))
            .then(Transform2D::translation(Vec2::new(10.0, -4.0)));
        let p = Point::new(3.0, 9.0);
        let back = t.inverse().unwrap().apply(t.apply(p));
        assert!(close(back, p), "{back:?}");
    }

    #[test]
    fn singular_inverse_is_an_error_never_nan() {
        assert_eq!(Transform2D::scale(0.0).inverse(), Err(SingularTransform));
        assert_eq!(
            Transform2D::scale_xy(1.0, 0.0).inverse(),
            Err(SingularTransform)
        );
    }

    #[test]
    fn non_finite_transforms_have_no_inverse() {
        assert_eq!(
            Transform2D::scale_xy(f64::NAN, 1.0).inverse(),
            Err(SingularTransform)
        );
        assert_eq!(
            Transform2D::scale_xy(f64::INFINITY, 1.0).inverse(),
            Err(SingularTransform)
        );
        // Non-zero determinant, but the inverse coefficient overflows.
        assert_eq!(
            Transform2D::scale_xy(5e-324, 1.0).inverse(),
            Err(SingularTransform)
        );
    }

    #[test]
    fn infinite_determinant_has_no_inverse() {
        // Finite coefficients, but the determinant overflows to +inf while
        // kurbo's inverse coefficients come out as finite zeros.
        assert_eq!(Transform2D::scale(1e200).inverse(), Err(SingularTransform));
    }

    #[test]
    fn affine_round_trip_is_bit_equal() {
        let coeffs = [-0.0, 1.5, 5e-324, -2.25, 1e300, f64::MIN_POSITIVE];
        let affine = Affine::new(coeffs);
        let back: Affine = Transform2D::from(affine).into();
        assert_eq!(bits(back.as_coeffs()), bits(coeffs));
        assert_eq!(bits(Transform2D::from(affine).coeffs()), bits(coeffs));
    }

    #[test]
    fn array_round_trip_is_bit_equal() {
        let coeffs = [0.1, -0.0, 3.0, 5e-324, -7.5, 1e-300];
        let t = Transform2D::from(coeffs);
        let back: [f64; 6] = t.into();
        assert_eq!(bits(back), bits(coeffs));
    }

    #[test]
    fn serializes_as_a_six_element_array() {
        let t = Transform2D::translation(Vec2::new(2.0, 3.0));
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, "[1.0,0.0,0.0,1.0,2.0,3.0]");
        let back: Transform2D = serde_json::from_str(&json).unwrap();
        assert_eq!(bits(back.coeffs()), bits(t.coeffs()));
        assert!(serde_json::from_str::<Transform2D>("[1.0,0.0,0.0,1.0,2.0]").is_err());
    }

    #[test]
    fn singular_transform_is_a_displayable_error() {
        assert!(SingularTransform.to_string().contains("singular"));
        let _: &dyn std::error::Error = &SingularTransform;
    }
}
