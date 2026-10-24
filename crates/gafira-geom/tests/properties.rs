//! Property tests: conversion round trips, transform inverses and serde round trips.

use gafira_geom::{Affine, Length, Point, Rect, Size, Transform2D, Unit, Vec2};
use proptest::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Any finite `f64`, including `-0.0` and subnormals.
fn finite() -> impl Strategy<Value = f64> {
    use proptest::num::f64::{NEGATIVE, NORMAL, POSITIVE, SUBNORMAL, ZERO};
    POSITIVE | NEGATIVE | NORMAL | SUBNORMAL | ZERO
}

fn unit() -> impl Strategy<Value = Unit> {
    proptest::sample::select(Unit::ALL.to_vec())
}

/// A non-zero scale factor with magnitude in [0.1, 10].
fn scale_factor() -> impl Strategy<Value = f64> {
    (0.1_f64..=10.0, any::<bool>()).prop_map(|(s, negative)| if negative { -s } else { s })
}

fn transform() -> impl Strategy<Value = Transform2D> {
    let rotation = (-7.0_f64..=7.0).prop_map(Transform2D::rotation);
    let translation = (-1e3_f64..=1e3, -1e3_f64..=1e3)
        .prop_map(|(x, y)| Transform2D::translation(Vec2::new(x, y)));
    let scale = (scale_factor(), scale_factor()).prop_map(|(sx, sy)| Transform2D::scale_xy(sx, sy));
    let step = prop_oneof![rotation, translation, scale];
    proptest::collection::vec(step, 1..=4).prop_map(|steps| {
        steps
            .into_iter()
            .fold(Transform2D::IDENTITY, Transform2D::then)
    })
}

fn bits6(coeffs: [f64; 6]) -> [u64; 6] {
    coeffs.map(f64::to_bits)
}

fn round_trip<T: Serialize + DeserializeOwned>(value: &T) -> T {
    let json = serde_json::to_string(value).expect("finite values serialize");
    serde_json::from_str(&json).expect("serialized value deserializes")
}

proptest! {
    #[test]
    fn conversion_round_trip_stays_within_tolerance(
        value in -1e9_f64..=1e9,
        from in unit(),
        to in unit(),
        dpi in 1.0_f64..=1e4,
    ) {
        let original = Length::new(value, from);
        let there = original.convert(to, dpi).unwrap();
        prop_assert_eq!(there.unit(), to);
        let back = there.convert(from, dpi).unwrap();
        prop_assert_eq!(back.unit(), from);
        let error = (back.value() - value).abs();
        prop_assert!(
            error <= 1e-9 * value.abs() + 1e-12,
            "{value} {from:?} -> {to:?} @ {dpi}: back {} (error {error})",
            back.value()
        );
    }

    #[test]
    fn inverse_of_composed_transforms_recovers_the_point(
        t in transform(),
        x in -1e3_f64..=1e3,
        y in -1e3_f64..=1e3,
    ) {
        let p = Point::new(x, y);
        let back = t.inverse().unwrap().apply(t.apply(p));
        prop_assert!((back.x - x).abs() <= 1e-9 * x.abs().max(1.0), "{back:?} vs {p:?}");
        prop_assert!((back.y - y).abs() <= 1e-9 * y.abs().max(1.0), "{back:?} vs {p:?}");
    }

    #[test]
    fn length_json_round_trip_is_bit_identical(value in finite(), unit in unit()) {
        let back = round_trip(&Length::new(value, unit));
        prop_assert_eq!(back.value().to_bits(), value.to_bits());
        prop_assert_eq!(back.unit(), unit);
    }

    #[test]
    fn unit_json_round_trip(unit in unit()) {
        prop_assert_eq!(round_trip(&unit), unit);
    }

    #[test]
    fn size_json_round_trip_is_bit_identical(w in finite(), h in finite()) {
        let back = round_trip(&Size::new(w, h));
        prop_assert_eq!(back.width().to_bits(), w.to_bits());
        prop_assert_eq!(back.height().to_bits(), h.to_bits());
    }

    #[test]
    fn transform2d_json_round_trip_is_bit_identical(coeffs in proptest::array::uniform6(finite())) {
        let back = round_trip(&Transform2D::from(coeffs));
        prop_assert_eq!(bits6(back.coeffs()), bits6(coeffs));
    }

    #[test]
    fn point_json_round_trip_is_bit_identical(x in finite(), y in finite()) {
        let back = round_trip(&Point::new(x, y));
        prop_assert_eq!(back.x.to_bits(), x.to_bits());
        prop_assert_eq!(back.y.to_bits(), y.to_bits());
    }

    #[test]
    fn vec2_json_round_trip_is_bit_identical(x in finite(), y in finite()) {
        let back = round_trip(&Vec2::new(x, y));
        prop_assert_eq!(back.x.to_bits(), x.to_bits());
        prop_assert_eq!(back.y.to_bits(), y.to_bits());
    }

    #[test]
    fn rect_json_round_trip_is_bit_identical(
        x0 in finite(), y0 in finite(), x1 in finite(), y1 in finite(),
    ) {
        let back = round_trip(&Rect::new(x0, y0, x1, y1));
        prop_assert_eq!(
            [back.x0.to_bits(), back.y0.to_bits(), back.x1.to_bits(), back.y1.to_bits()],
            [x0.to_bits(), y0.to_bits(), x1.to_bits(), y1.to_bits()]
        );
    }

    #[test]
    fn affine_json_round_trip_is_bit_identical(coeffs in proptest::array::uniform6(finite())) {
        let back = round_trip(&Affine::new(coeffs));
        prop_assert_eq!(bits6(back.as_coeffs()), bits6(coeffs));
    }
}

#[test]
fn strategies_reach_negative_zero_and_subnormals() {
    // Guards the generators: without them the bit-identity claims would be vacuous.
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;
    let mut runner = TestRunner::deterministic();
    let strategy = finite();
    let (mut neg_zero, mut subnormal) = (false, false);
    for _ in 0..20_000 {
        let v = strategy.new_tree(&mut runner).unwrap().current();
        assert!(v.is_finite());
        neg_zero |= v.to_bits() == (-0.0_f64).to_bits();
        subnormal |= v != 0.0 && !v.is_normal();
    }
    assert!(neg_zero, "-0.0 never generated");
    assert!(subnormal, "subnormal never generated");
}
