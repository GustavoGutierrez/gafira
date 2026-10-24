//! Public API as seen from outside the crate: re-exports and wire names.

use gafira_geom::{Affine, Length, Point, Rect, Size, Transform2D, Unit, Vec2};

#[test]
fn point_translated_by_vec2_matches_kurbo() {
    let ours = Point::new(1.0, 2.0) + Vec2::new(0.5, -4.0);
    let theirs = kurbo::Point::new(1.0, 2.0) + kurbo::Vec2::new(0.5, -4.0);
    assert_eq!(ours.x.to_bits(), theirs.x.to_bits());
    assert_eq!(ours.y.to_bits(), theirs.y.to_bits());
    assert_eq!(ours, Point::new(1.5, -2.0));
}

#[test]
fn affine_applied_to_a_rect_corner_matches_kurbo() {
    let coeffs = [2.0, 0.0, 0.0, 3.0, 10.0, -1.0];
    let rect = Rect::new(1.0, 2.0, 5.0, 8.0);
    let ours = Affine::new(coeffs) * Point::new(rect.x1, rect.y1);
    let theirs = kurbo::Affine::new(coeffs) * kurbo::Point::new(rect.x1, rect.y1);
    assert_eq!(ours.x.to_bits(), theirs.x.to_bits());
    assert_eq!(ours.y.to_bits(), theirs.y.to_bits());
    assert_eq!(ours, Point::new(20.0, 23.0));
}

#[test]
fn reexports_are_the_kurbo_types() {
    // Same types, not look-alikes: values flow in both directions unchanged.
    let _: kurbo::Point = Point::new(0.0, 0.0);
    let _: kurbo::Vec2 = Vec2::new(0.0, 0.0);
    let _: kurbo::Rect = Rect::new(0.0, 0.0, 1.0, 1.0);
    let _: kurbo::Affine = Affine::IDENTITY;
}

#[test]
fn transform2d_interoperates_with_the_reexported_affine() {
    let affine = Affine::translate(Vec2::new(2.0, 3.0));
    let t = Transform2D::from(affine);
    assert_eq!(Affine::from(t), affine);
}

#[test]
fn unit_in_serializes_as_lowercase_name_and_is_accepted() {
    assert_eq!(serde_json::to_string(&Unit::In).unwrap(), r#""in""#);
    assert_eq!(serde_json::from_str::<Unit>(r#""in""#).unwrap(), Unit::In);
    for unit in Unit::ALL {
        let json = serde_json::to_string(&unit).unwrap();
        assert_eq!(serde_json::from_str::<Unit>(&json).unwrap(), unit);
    }
}

#[test]
fn unknown_unit_names_are_rejected() {
    assert!(serde_json::from_str::<Unit>(r#""km""#).is_err());
    assert!(serde_json::from_str::<Unit>(r#""IN""#).is_err());
    assert!(serde_json::from_str::<Length>(r#"{"value":1.0,"unit":"km"}"#).is_err());
}

#[test]
fn size_and_length_wire_formats_are_stable() {
    assert_eq!(
        serde_json::to_string(&Size::new(1.0, 2.0)).unwrap(),
        r#"{"width":1.0,"height":2.0}"#
    );
    assert_eq!(
        serde_json::to_string(&Length::new(10.0, Unit::Mm)).unwrap(),
        r#"{"value":10.0,"unit":"mm"}"#
    );
}
