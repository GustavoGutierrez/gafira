use gafira_anim::Easing;
use proptest::prelude::*;

const EASE_IN: (f32, f32, f32, f32) = (0.42, 0.0, 1.0, 1.0);
const EASE_OUT: (f32, f32, f32, f32) = (0.0, 0.0, 0.58, 1.0);
const EASE_IN_OUT: (f32, f32, f32, f32) = (0.42, 0.0, 0.58, 1.0);

fn bezier((x1, y1, x2, y2): (f32, f32, f32, f32)) -> Easing {
    Easing::CubicBezier { x1, y1, x2, y2 }
}

/// f64 reference: bisection on x(s) = u, then y(s).
fn reference(x1: f32, y1: f32, x2: f32, y2: f32, u: f32) -> f64 {
    let (x1, y1, x2, y2) = (f64::from(x1), f64::from(y1), f64::from(x2), f64::from(y2));
    let b = |p1: f64, p2: f64, s: f64| {
        let m = 1.0 - s;
        3.0 * m * m * s * p1 + 3.0 * m * s * s * p2 + s * s * s
    };
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if b(x1, x2, mid) < f64::from(u) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    b(y1, y2, 0.5 * (lo + hi))
}

fn grid() -> impl Iterator<Item = f32> {
    (0..=1000).map(|i| i as f32 / 1000.0)
}

#[test]
fn arq_17_linear_is_the_identity() {
    for u in grid() {
        assert_eq!(Easing::Linear.apply(u), u, "u = {u}");
    }
}

#[test]
fn arq_17_named_curves_equal_their_cubic_bezier_bit_for_bit() {
    for (named, params) in [
        (Easing::EaseIn, EASE_IN),
        (Easing::EaseOut, EASE_OUT),
        (Easing::EaseInOut, EASE_IN_OUT),
    ] {
        for u in grid() {
            assert_eq!(
                named.apply(u).to_bits(),
                bezier(params).apply(u).to_bits(),
                "{named:?} at u = {u}"
            );
        }
    }
}

#[test]
fn arq_17_named_curves_match_css_reference_values() {
    // CSS ease-in-out is symmetric about (0.5, 0.5).
    assert!((Easing::EaseInOut.apply(0.5) - 0.5).abs() < 1e-6);
    // Reference from bisection, independent of the solver.
    for (named, (x1, y1, x2, y2)) in [
        (Easing::EaseIn, EASE_IN),
        (Easing::EaseOut, EASE_OUT),
        (Easing::EaseInOut, EASE_IN_OUT),
    ] {
        for u in grid() {
            let want = reference(x1, y1, x2, y2, u);
            assert!(
                (f64::from(named.apply(u)) - want).abs() <= 1e-5,
                "{named:?} at {u}"
            );
        }
    }
}

#[test]
fn arq_17_every_curve_is_exact_at_the_ends() {
    for e in [
        Easing::Hold,
        Easing::Linear,
        Easing::EaseIn,
        Easing::EaseOut,
        Easing::EaseInOut,
        bezier((0.3, -2.0, 0.9, 3.0)),
    ] {
        assert_eq!(e.apply(0.0).to_bits(), 0.0_f32.to_bits(), "{e:?}");
        assert_eq!(e.apply(1.0), 1.0, "{e:?}");
    }
}

#[test]
fn arq_17_cubic_bezier_validity_rule() {
    let cases = [
        ((0.0, 0.0, 1.0, 1.0), true),
        ((0.5, -2.0, 0.5, 3.0), true),
        ((-0.01, 0.0, 0.5, 1.0), false),
        ((1.01, 0.0, 0.5, 1.0), false),
        ((0.5, 0.0, -0.01, 1.0), false),
        ((0.5, 0.0, 1.01, 1.0), false),
        ((f32::NAN, 0.0, 0.5, 1.0), false),
        ((0.5, f32::NAN, 0.5, 1.0), false),
        ((0.5, 0.0, 0.5, f32::INFINITY), false),
        ((0.5, f32::NEG_INFINITY, 0.5, 1.0), false),
    ];
    for (params, valid) in cases {
        assert_eq!(bezier(params).is_valid(), valid, "{params:?}");
    }
    for e in [
        Easing::Hold,
        Easing::Linear,
        Easing::EaseIn,
        Easing::EaseOut,
        Easing::EaseInOut,
    ] {
        assert!(e.is_valid(), "{e:?}");
    }
}

#[test]
fn arq_17_serde_names_follow_the_spec_enum() {
    assert_eq!(
        serde_json::to_string(&Easing::EaseInOut).unwrap(),
        "\"EaseInOut\""
    );
    let json = serde_json::to_string(&bezier((0.25, 0.5, 0.75, 1.5))).unwrap();
    assert_eq!(
        json,
        r#"{"CubicBezier":{"x1":0.25,"y1":0.5,"x2":0.75,"y2":1.5}}"#
    );
    assert_eq!(
        serde_json::from_str::<Easing>(&json).unwrap(),
        bezier((0.25, 0.5, 0.75, 1.5))
    );
}

proptest! {
    #[test]
    fn arq_17_bezier_ends_are_exact(
        x1 in 0.0_f32..=1.0, y1 in -2.0_f32..=3.0, x2 in 0.0_f32..=1.0, y2 in -2.0_f32..=3.0,
    ) {
        let e = Easing::CubicBezier { x1, y1, x2, y2 };
        prop_assert_eq!(e.apply(0.0).to_bits(), 0.0_f32.to_bits());
        prop_assert_eq!(e.apply(1.0), 1.0);
    }

    #[test]
    fn arq_17_bezier_matches_f64_reference_within_tolerance(
        x1 in 0.0_f32..=1.0, y1 in -2.0_f32..=3.0, x2 in 0.0_f32..=1.0, y2 in -2.0_f32..=3.0,
        u in 0.0_f32..=1.0,
    ) {
        let got = f64::from(Easing::CubicBezier { x1, y1, x2, y2 }.apply(u));
        let want = reference(x1, y1, x2, y2, u);
        let tol = 1e-5 * 1.0_f64.max(f64::from(y1).abs()).max(f64::from(y2).abs());
        prop_assert!((got - want).abs() <= tol, "got {got}, want {want}, tol {tol}");
    }

    #[test]
    fn arq_17_bezier_stays_within_bounds(
        x1 in 0.0_f32..=1.0, y1 in -2.0_f32..=3.0, x2 in 0.0_f32..=1.0, y2 in -2.0_f32..=3.0,
        u in 0.0_f32..=1.0,
    ) {
        let got = Easing::CubicBezier { x1, y1, x2, y2 }.apply(u);
        prop_assert!(got >= 0.0_f32.min(y1).min(y2) && got <= 1.0_f32.max(y1).max(y2), "got {got}");
    }
}

#[test]
fn arq_17_hold_is_a_step_that_reaches_one_only_at_the_end() {
    for u in grid().filter(|&u| u < 1.0) {
        assert_eq!(Easing::Hold.apply(u), 0.0, "u = {u}");
    }
}
