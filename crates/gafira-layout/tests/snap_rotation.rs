//! FD-06 (i), D-28, ARQ-110 and ARQ-111 for the rotation snap.

use gafira_layout::{LayoutError, snap_rotation};
use proptest::prelude::*;

const BAD: [f64; 3] = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];

#[test]
fn fd_06_i_the_spec_examples() {
    for (angle, want) in [
        (7.4, 0.0),
        (7.5, 15.0),
        (7.499999999999999, 0.0),
        (-7.5, 0.0),
        (-22.5, -15.0),
        (367.0, 360.0),
    ] {
        assert_eq!(snap_rotation(angle), Ok(want), "{angle}");
    }
}

#[test]
fn fd_06_i_halfway_goes_to_the_larger_value_and_the_neighbours_to_the_nearer() {
    // every halfway point from -352.5 to 352.5, and the float on each side of it
    for k in -24..=23 {
        let mid = f64::from(k) * 15.0 + 7.5;
        let lo = f64::from(k) * 15.0;
        assert_eq!(snap_rotation(mid), Ok(lo + 15.0), "{mid}");
        assert_eq!(snap_rotation(mid.next_down()), Ok(lo), "{mid} down");
        assert_eq!(snap_rotation(mid.next_up()), Ok(lo + 15.0), "{mid} up");
    }
    // on a multiple, and either side of it
    for k in -24..=24 {
        let m = f64::from(k) * 15.0;
        assert_eq!(snap_rotation(m), Ok(m));
        assert_eq!(snap_rotation(m + 7.4), Ok(m));
        assert_eq!(snap_rotation(m - 7.4), Ok(m));
        assert_eq!(snap_rotation(m + 7.6), Ok(m + 15.0));
        assert_eq!(snap_rotation(m - 7.6), Ok(m - 15.0));
    }
}

#[test]
fn fd_06_i_the_angle_is_not_normalized_to_one_turn() {
    for (angle, want) in [
        (720.0, 720.0),
        (-360.0, -360.0),
        (-367.0, -360.0),
        (1081.0, 1080.0),
        (1_000_000.0, 1_000_005.0),
    ] {
        assert_eq!(snap_rotation(angle), Ok(want), "{angle}");
    }
}

#[test]
fn fd_06_i_a_zero_result_is_positive_zero() {
    for angle in [
        0.0,
        -0.0,
        7.4,
        -7.4,
        -7.5,
        7.499999999999999,
        -7.499999999999999,
    ] {
        assert_eq!(
            snap_rotation(angle).unwrap().to_bits(),
            0.0f64.to_bits(),
            "{angle}"
        );
    }
}

#[test]
fn arq_110_the_angle_must_be_finite() {
    for v in BAD {
        assert_eq!(
            snap_rotation(v),
            Err(LayoutError::NonFinite { field: "angle" })
        );
    }
}

#[test]
fn arq_110_a_result_that_overflows_is_an_error_not_infinity() {
    // the multiple 15 * floor(MAX / 15) rounds to infinity
    for angle in [f64::MAX, -f64::MAX] {
        assert_eq!(
            snap_rotation(angle),
            Err(LayoutError::NonFinite { field: "angle" })
        );
    }
    assert!(snap_rotation(f64::MAX.next_down()).is_ok());
}

#[test]
fn arq_111_the_same_angle_gives_the_same_bytes() {
    for angle in [7.5, -7.5, 33.3, -0.0, 0.0, 1e12, -2.5e-3] {
        let first = snap_rotation(angle).map(f64::to_bits);
        for _ in 0..3 {
            assert_eq!(snap_rotation(angle).map(f64::to_bits), first);
        }
    }
}

proptest! {
    /// An angle built as `15 k + r` with `r` exact: the answer follows from `r` alone.
    #[test]
    fn fd_06_i_an_angle_off_a_multiple_goes_to_the_nearer_one(
        k in -1_000_000i32..1_000_000,
        quarter in -29i32..=29,
    ) {
        let r = f64::from(quarter) * 0.25; // -7.25 to 7.25: always nearer to 15 k
        let angle = f64::from(k) * 15.0 + r;
        prop_assert_eq!(snap_rotation(angle), Ok(f64::from(k) * 15.0));
        // halfway, the larger value
        prop_assert_eq!(snap_rotation(f64::from(k) * 15.0 + 7.5), Ok(f64::from(k) * 15.0 + 15.0));
        prop_assert_eq!(snap_rotation(f64::from(k) * 15.0 - 7.5), Ok(f64::from(k) * 15.0));
    }

    #[test]
    fn arq_110_a_finite_angle_gives_a_multiple_of_15_within_half_a_step(angle in -1.0e9f64..1.0e9) {
        let v = snap_rotation(angle).unwrap();
        prop_assert!(v.is_finite());
        prop_assert_eq!((v / 15.0).fract(), 0.0);
        prop_assert!((v - angle).abs() <= 7.5);
        prop_assert_eq!(snap_rotation(v), Ok(v));
    }

    #[test]
    fn arq_110_any_finite_angle_gives_a_finite_value_or_an_error(angle in prop::num::f64::NORMAL | prop::num::f64::SUBNORMAL | prop::num::f64::ZERO) {
        if let Ok(v) = snap_rotation(angle) {
            prop_assert!(v.is_finite());
        }
    }
}
