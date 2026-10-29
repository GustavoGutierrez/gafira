use gafira_anim::Interpolate;
use gafira_geom::Vec2;
use proptest::prelude::*;

// Q-32: contract range is [-MAX/2, MAX/2], so `b - a` is finite.
const HALF32: f32 = f32::MAX / 2.0;
const HALF64: f64 = f64::MAX / 2.0;

fn f32_any() -> impl Strategy<Value = f32> {
    prop_oneof![
        -1.0e30_f32..1.0e30_f32,
        -HALF32..=HALF32,
        Just(HALF32),
        Just(-HALF32),
        Just(0.0_f32),
        Just(-0.0_f32),
        Just(f32::MIN_POSITIVE),
        Just(1.0e-40_f32),
        Just(1.0e30_f32),
        Just(-1.0e30_f32),
        -4.0_f32..4.0_f32,
    ]
}

fn f64_any() -> impl Strategy<Value = f64> {
    prop_oneof![
        -1.0e300_f64..1.0e300_f64,
        -HALF64..=HALF64,
        Just(HALF64),
        Just(-HALF64),
        Just(0.0_f64),
        Just(f64::MIN_POSITIVE),
        Just(1.0e300_f64),
        -4.0_f64..4.0_f64,
    ]
}

proptest! {
    // ARQ-19: exact endpoints for f32.
    #[test]
    fn arq_19_f32_endpoints_are_exact(a in f32_any(), b in f32_any()) {
        prop_assert_eq!(a.interpolate(&b, 0.0).to_bits(), a.to_bits());
        prop_assert_eq!(a.interpolate(&b, 1.0).to_bits(), b.to_bits());
    }

    // ARQ-19: exact endpoints for Vec2.
    #[test]
    fn arq_19_vec2_endpoints_are_exact(
        ax in f64_any(), ay in f64_any(), bx in f64_any(), by in f64_any()
    ) {
        let (a, b) = (Vec2::new(ax, ay), Vec2::new(bx, by));
        prop_assert_eq!(a.interpolate(&b, 0.0), a);
        prop_assert_eq!(a.interpolate(&b, 1.0), b);
    }

    // ARQ-19: for t in [0, 1] the f32 result is between the ends within eps * max(|a|, |b|).
    #[test]
    fn arq_19_f32_stays_between_ends(a in f32_any(), b in f32_any(), t in 0.0_f32..=1.0) {
        let r = a.interpolate(&b, t);
        let tol = f32::EPSILON * a.abs().max(b.abs());
        prop_assert!(r >= a.min(b) - tol && r <= a.max(b) + tol, "{a} {b} {t} -> {r}");
    }

    // ARQ-19: same for each Vec2 component, with the f64 epsilon.
    #[test]
    fn arq_19_vec2_stays_between_ends(
        ax in f64_any(), ay in f64_any(), bx in f64_any(), by in f64_any(), t in 0.0_f32..=1.0
    ) {
        let r = Vec2::new(ax, ay).interpolate(&Vec2::new(bx, by), t);
        for (r, a, b) in [(r.x, ax, bx), (r.y, ay, by)] {
            let tol = f64::EPSILON * a.abs().max(b.abs());
            prop_assert!(r >= a.min(b) - tol && r <= a.max(b) + tol, "{a} {b} {t} -> {r}");
        }
    }
}

// ARQ-19: the f32 rule is `a + (b - a) * t`, bit for bit, with extrapolation (no clamping).
#[test]
fn arq_19_f32_follows_the_formula_and_extrapolates() {
    let cases: [(f32, f32, f32); 7] = [
        (2.0, 6.0, 0.5),
        (2.0, 6.0, 0.25),
        (2.0, 6.0, 1.5),
        (2.0, 6.0, -0.5),
        (0.1, 0.7, 0.3),
        (-3.0, 5.0, 2.0),
        (5.0, 5.0, 0.3),
    ];
    for (a, b, t) in cases {
        assert_eq!(
            a.interpolate(&b, t).to_bits(),
            (a + (b - a) * t).to_bits(),
            "{a} {b} {t}"
        );
    }
    assert_eq!(2.0_f32.interpolate(&6.0, 1.5), 8.0);
    assert_eq!(2.0_f32.interpolate(&6.0, -0.5), 0.0);
}

// ARQ-19: Vec2 applies the same rule per component in f64, t widened without loss.
#[test]
fn arq_19_vec2_follows_the_formula_per_component() {
    let (a, b) = (Vec2::new(1.0, -4.0), Vec2::new(5.0, 8.0));
    let t = 0.3_f32;
    let r = a.interpolate(&b, t);
    let t = f64::from(t);
    assert_eq!(r.x.to_bits(), (1.0 + (5.0 - 1.0) * t).to_bits());
    assert_eq!(r.y.to_bits(), (-4.0 + (8.0 - -4.0) * t).to_bits());
    assert_eq!(a.interpolate(&b, 2.0), Vec2::new(9.0, 20.0));
    assert_eq!(a.interpolate(&b, -1.0), Vec2::new(-3.0, -16.0));
}

// ARQ-18 / ARQ-19: finiteness check used by track validation.
#[test]
fn arq_18_finiteness_covers_every_component() {
    assert!(1.0_f32.is_finite_value());
    assert!(!f32::NAN.is_finite_value());
    assert!(!f32::INFINITY.is_finite_value());
    assert!(Vec2::new(1.0, 2.0).is_finite_value());
    assert!(!Vec2::new(f64::NAN, 2.0).is_finite_value());
    assert!(!Vec2::new(1.0, f64::NEG_INFINITY).is_finite_value());
}

trait Probe {
    fn is_finite_value(&self) -> bool;
}
impl<T: Interpolate> Probe for T {
    fn is_finite_value(&self) -> bool {
        Interpolate::is_finite(self)
    }
}
