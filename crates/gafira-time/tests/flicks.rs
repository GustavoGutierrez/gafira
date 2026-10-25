//! Public-API tests for [`Flicks`].

use gafira_time::{FLICKS_PER_MILLISECOND, FLICKS_PER_SECOND, Flicks, TimeError};
use proptest::prelude::*;

fn ticks(value: i64) -> Flicks {
    Flicks::from_ticks(value)
}

#[test]
fn constants_match_the_spec() {
    assert_eq!(FLICKS_PER_SECOND, 705_600_000);
    assert_eq!(FLICKS_PER_MILLISECOND, 705_600);
}

#[test]
fn the_tick_divides_every_required_rate() {
    for divisor in [24, 25, 30, 48, 50, 60, 120, 44_100, 48_000, 96_000] {
        assert_eq!(
            FLICKS_PER_SECOND % divisor,
            0,
            "705,600,000 must divide {divisor} exactly"
        );
    }
}

#[test]
fn negative_ticks_are_preserved() {
    assert_eq!(ticks(-1).ticks(), -1);
    assert_eq!(ticks(-1).checked_add(ticks(1)).unwrap().ticks(), 0);
}

#[test]
fn zero_is_the_default_and_an_identity() {
    assert_eq!(Flicks::default(), Flicks::ZERO);
    assert_eq!(ticks(0), Flicks::ZERO);
    assert_eq!(ticks(42).checked_add(Flicks::ZERO).unwrap(), ticks(42));
    assert_eq!(ticks(42).checked_sub(Flicks::ZERO).unwrap(), ticks(42));
    assert_eq!(ticks(42).checked_sub(ticks(42)).unwrap(), Flicks::ZERO);
}

#[test]
fn seconds_are_exact() {
    assert_eq!(Flicks::from_seconds(1).unwrap().ticks(), 705_600_000);
    assert_eq!(Flicks::from_seconds(0).unwrap().ticks(), 0);
    assert_eq!(Flicks::from_seconds(-1).unwrap().ticks(), -705_600_000);
}

#[test]
fn milliseconds_are_exact() {
    assert_eq!(Flicks::from_millis(1).unwrap().ticks(), 705_600);
    assert_eq!(Flicks::from_millis(0).unwrap().ticks(), 0);
    assert_eq!(Flicks::from_millis(1_234).unwrap().ticks(), 870_710_400);
    assert_eq!(Flicks::from_millis(-1).unwrap().ticks(), -705_600);
}

#[test]
fn seconds_and_milliseconds_overflow_instead_of_wrapping() {
    let max_seconds = i64::MAX / FLICKS_PER_SECOND;
    assert!(Flicks::from_seconds(max_seconds).is_ok());
    assert_eq!(
        Flicks::from_seconds(max_seconds + 1),
        Err(TimeError::Overflow)
    );
    assert_eq!(Flicks::from_seconds(i64::MAX), Err(TimeError::Overflow));
    assert_eq!(Flicks::from_seconds(i64::MIN), Err(TimeError::Overflow));
    assert_eq!(Flicks::from_millis(i64::MAX), Err(TimeError::Overflow));
    assert_eq!(Flicks::from_millis(i64::MIN), Err(TimeError::Overflow));
}

#[test]
fn addition_overflow_is_an_error() {
    assert_eq!(
        ticks(i64::MAX).checked_add(ticks(1)),
        Err(TimeError::Overflow)
    );
    assert_eq!(
        ticks(i64::MIN).checked_add(ticks(-1)),
        Err(TimeError::Overflow)
    );
    assert_eq!(ticks(i64::MAX).checked_add(ticks(0)), Ok(ticks(i64::MAX)));
}

#[test]
fn subtraction_overflow_is_an_error() {
    assert_eq!(
        ticks(i64::MAX).checked_sub(ticks(-1)),
        Err(TimeError::Overflow)
    );
    assert_eq!(
        ticks(i64::MIN).checked_sub(ticks(1)),
        Err(TimeError::Overflow)
    );
}

#[test]
fn multiplication_overflow_is_an_error() {
    assert_eq!(ticks(0).checked_mul(i64::MAX), Ok(Flicks::ZERO));
    assert_eq!(ticks(1).checked_mul(i64::MAX), Ok(ticks(i64::MAX)));
    assert_eq!(ticks(i64::MAX).checked_mul(2), Err(TimeError::Overflow));
    assert_eq!(ticks(i64::MIN).checked_mul(-1), Err(TimeError::Overflow));
}

#[test]
fn negation_overflow_is_an_error() {
    assert_eq!(ticks(i64::MIN).checked_neg(), Err(TimeError::Overflow));
    assert_eq!(ticks(5).checked_neg(), Ok(ticks(-5)));
    assert_eq!(ticks(0).checked_neg(), Ok(Flicks::ZERO));
}

#[test]
fn whole_seconds_floor_towards_negative_infinity() {
    assert_eq!(
        Flicks::from_seconds(1)
            .unwrap()
            .checked_add(Flicks::from_millis(500).unwrap())
            .unwrap()
            .seconds_floor(),
        1
    );
    assert_eq!(Flicks::from_millis(-500).unwrap().seconds_floor(), -1);
    assert_eq!(Flicks::from_seconds(-1).unwrap().seconds_floor(), -1);
    assert_eq!(ticks(705_599_999).seconds_floor(), 0);
    assert_eq!(ticks(-705_600_000).seconds_floor(), -1);
    assert_eq!(Flicks::ZERO.seconds_floor(), 0);
}

#[test]
fn f64_seconds_are_the_lossy_display_value() {
    assert_eq!(Flicks::from_seconds(1).unwrap().as_seconds_f64(), 1.0);
    assert_eq!(Flicks::from_millis(-500).unwrap().as_seconds_f64(), -0.5);
    assert!((ticks(1).as_seconds_f64() - 1.0 / 705_600_000.0).abs() < 1e-24);
}

#[test]
fn serde_uses_a_plain_integer() {
    assert_eq!(
        serde_json::to_string(&Flicks::from_seconds(1).unwrap()).unwrap(),
        "705600000"
    );
    assert_eq!(serde_json::to_string(&ticks(-1)).unwrap(), "-1");
    assert_eq!(
        serde_json::from_str::<Flicks>("705600000").unwrap(),
        Flicks::from_seconds(1).unwrap()
    );
    assert_eq!(serde_json::from_str::<Flicks>("-1").unwrap(), ticks(-1));
}

proptest! {
    #[test]
    fn add_then_subtract_is_an_identity(a in any::<i64>(), b in any::<i64>()) {
        if let Ok(sum) = ticks(a).checked_add(ticks(b)) {
            prop_assert_eq!(sum.checked_sub(ticks(b)), Ok(ticks(a)));
            prop_assert_eq!(ticks(b).checked_add(ticks(a)), Ok(sum));
        }
    }

    #[test]
    fn seconds_round_trip(value in any::<i64>()) {
        let expected = value.checked_mul(FLICKS_PER_SECOND);
        prop_assert_eq!(
            Flicks::from_seconds(value).ok().map(Flicks::ticks),
            expected
        );
    }

    #[test]
    fn milliseconds_round_trip(value in any::<i64>()) {
        let expected = value.checked_mul(FLICKS_PER_MILLISECOND);
        prop_assert_eq!(
            Flicks::from_millis(value).ok().map(Flicks::ticks),
            expected
        );
    }

    #[test]
    fn seconds_floor_matches_euclidean_division(value in any::<i64>()) {
        prop_assert_eq!(
            ticks(value).seconds_floor(),
            value.div_euclid(FLICKS_PER_SECOND)
        );
    }

    #[test]
    fn serde_round_trip(value in any::<i64>()) {
        let flicks = ticks(value);
        let json = serde_json::to_string(&flicks).unwrap();
        prop_assert_eq!(serde_json::from_str::<Flicks>(&json).unwrap(), flicks);
    }
}
