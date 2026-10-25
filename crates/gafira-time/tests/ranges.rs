//! Public-API tests for [`TimeRange`].

use gafira_time::{Flicks, TimeError, TimeRange};
use proptest::prelude::*;

fn range(start: i64, duration: i64) -> TimeRange {
    TimeRange::new(Flicks::from_ticks(start), Flicks::from_ticks(duration)).unwrap()
}

fn bounds(start: i64, end: i64) -> TimeRange {
    TimeRange::from_bounds(Flicks::from_ticks(start), Flicks::from_ticks(end)).unwrap()
}

const SECOND: i64 = 705_600_000;

#[test]
fn construction_exposes_start_duration_and_end() {
    let r = range(SECOND, SECOND / 2);
    assert_eq!(r.start().ticks(), SECOND);
    assert_eq!(r.duration().ticks(), SECOND / 2);
    assert_eq!(r.end().ticks(), SECOND + SECOND / 2);
}

#[test]
fn from_bounds_computes_the_duration() {
    let r = bounds(SECOND, 2 * SECOND);
    assert_eq!(r.start().ticks(), SECOND);
    assert_eq!(r.duration().ticks(), SECOND);
    assert_eq!(r.end().ticks(), 2 * SECOND);
}

#[test]
fn an_empty_range_contains_nothing_and_never_intersects() {
    for r in [range(0, 0), range(SECOND, 0), range(-SECOND, 0)] {
        assert!(r.is_empty());
        assert!(!r.contains(r.start()));
        assert!(!r.contains(r.end()));
        assert!(!r.intersects(r));
        assert_eq!(r.intersection(r), None);
    }
}

#[test]
fn inverted_bounds_are_rejected() {
    assert_eq!(
        TimeRange::from_bounds(Flicks::from_seconds(1).unwrap(), Flicks::ZERO),
        Err(TimeError::InvertedRange {
            start: Flicks::from_seconds(1).unwrap(),
            end: Flicks::ZERO,
        })
    );
}

#[test]
fn a_negative_duration_is_rejected() {
    assert_eq!(
        TimeRange::new(Flicks::ZERO, Flicks::from_ticks(-1)),
        Err(TimeError::NegativeDuration {
            flicks: Flicks::from_ticks(-1),
        })
    );
}

#[test]
fn an_overflowing_end_is_rejected() {
    assert_eq!(
        TimeRange::new(Flicks::from_ticks(i64::MAX), Flicks::from_ticks(1)),
        Err(TimeError::Overflow)
    );
    assert_eq!(
        TimeRange::from_bounds(Flicks::from_ticks(i64::MIN), Flicks::from_ticks(i64::MAX)),
        Err(TimeError::Overflow)
    );
}

#[test]
fn a_negative_start_is_allowed() {
    let r = range(-SECOND, 2 * SECOND);
    assert!(!r.is_empty());
    assert_eq!(r.start().ticks(), -SECOND);
    assert_eq!(r.end().ticks(), SECOND);
    assert_eq!(r.duration().ticks(), 2 * SECOND);
    assert!(r.contains(Flicks::ZERO));
    assert!(r.contains(Flicks::from_ticks(-SECOND)));
    assert!(!r.contains(Flicks::from_ticks(SECOND)));
}

#[test]
fn containment_is_half_open() {
    let r = bounds(SECOND, 2 * SECOND);
    assert!(r.contains(Flicks::from_ticks(SECOND)));
    assert!(r.contains(Flicks::from_ticks(SECOND + SECOND / 2)));
    assert!(!r.contains(Flicks::from_ticks(2 * SECOND)));
    assert!(!r.contains(Flicks::from_ticks(SECOND - 1)));
}

#[test]
fn adjacent_ranges_do_not_overlap() {
    let left = bounds(SECOND, 2 * SECOND);
    let right = bounds(2 * SECOND, 3 * SECOND);
    assert!(!left.intersects(right));
    assert_eq!(left.intersection(right), None);
    assert_eq!(right.intersection(left), None);
}

#[test]
fn intersection_covers_overlap_containment_and_disjoint_cases() {
    let overlapping = bounds(0, 2 * SECOND).intersection(bounds(SECOND, 3 * SECOND));
    assert_eq!(overlapping, Some(bounds(SECOND, 2 * SECOND)));
    assert!(bounds(0, 2 * SECOND).intersects(bounds(SECOND, 3 * SECOND)));

    let contained = bounds(0, 3 * SECOND).intersection(bounds(SECOND, 2 * SECOND));
    assert_eq!(contained, Some(bounds(SECOND, 2 * SECOND)));

    let disjoint = bounds(0, SECOND).intersection(bounds(2 * SECOND, 3 * SECOND));
    assert_eq!(disjoint, None);
    assert!(!bounds(0, SECOND).intersects(bounds(2 * SECOND, 3 * SECOND)));

    let identical = bounds(0, SECOND);
    assert_eq!(identical.intersection(identical), Some(identical));
}

#[test]
fn equal_bounds_are_an_empty_range() {
    let r = bounds(SECOND, SECOND);
    assert!(r.is_empty());
    assert_eq!(r.duration(), Flicks::ZERO);
    assert_eq!(r, range(SECOND, 0));
}

#[test]
fn serde_uses_start_and_duration() {
    let r = range(SECOND, 2 * SECOND);
    assert_eq!(
        serde_json::to_string(&r).unwrap(),
        r#"{"start":705600000,"duration":1411200000}"#
    );
    assert_eq!(
        serde_json::from_str::<TimeRange>(r#"{"start":705600000,"duration":1411200000}"#).unwrap(),
        r
    );
}

#[test]
fn serde_rejects_invalid_ranges() {
    for json in [
        r#"{"start":0,"duration":-1}"#,
        r#"{"start":9223372036854775807,"duration":1}"#,
    ] {
        assert!(
            serde_json::from_str::<TimeRange>(json).is_err(),
            "accepted {json}"
        );
    }
}

proptest! {
    #[test]
    fn construction_invariants(start in any::<i64>(), duration in any::<i64>()) {
        if let Ok(r) = TimeRange::new(Flicks::from_ticks(start), Flicks::from_ticks(duration)) {
            prop_assert!(r.duration().ticks() >= 0);
            prop_assert_eq!(r.end().ticks(), start + duration);
            prop_assert_eq!(r.is_empty(), duration == 0);
            prop_assert_eq!(r.contains(Flicks::from_ticks(start)), duration > 0);
            prop_assert!(!r.contains(r.end()));
        }
    }

    #[test]
    fn from_bounds_matches_new(start in any::<i64>(), end in any::<i64>()) {
        match TimeRange::from_bounds(Flicks::from_ticks(start), Flicks::from_ticks(end)) {
            Ok(r) => {
                prop_assert!(end >= start);
                prop_assert_eq!(r.start().ticks(), start);
                prop_assert_eq!(r.end().ticks(), end);
                prop_assert_eq!(r.duration().ticks(), end - start);
            }
            Err(err) => {
                if end < start {
                    prop_assert_eq!(
                        err,
                        TimeError::InvertedRange {
                            start: Flicks::from_ticks(start),
                            end: Flicks::from_ticks(end),
                        }
                    );
                } else {
                    prop_assert_eq!(err, TimeError::Overflow);
                }
            }
        }
    }

    #[test]
    fn intersection_is_symmetric_and_contained(
        a_start in any::<i64>(), a_duration in 0i64..=1_000_000_000_000,
        b_start in any::<i64>(), b_duration in 0i64..=1_000_000_000_000,
    ) {
        let a = TimeRange::new(Flicks::from_ticks(a_start), Flicks::from_ticks(a_duration));
        let b = TimeRange::new(Flicks::from_ticks(b_start), Flicks::from_ticks(b_duration));
        if let (Ok(a), Ok(b)) = (a, b) {
            prop_assert_eq!(a.intersection(b), b.intersection(a));
            prop_assert_eq!(a.intersects(b), a.intersection(b).is_some());
            if let Some(common) = a.intersection(b) {
                prop_assert!(a.contains(common.start()));
                prop_assert!(b.contains(common.start()));
                prop_assert!(common.end().ticks() <= a.end().ticks());
                prop_assert!(common.end().ticks() <= b.end().ticks());
            }
        }
    }

    #[test]
    fn serde_round_trip(start in any::<i64>(), duration in 0i64..=1_000_000_000_000) {
        if let Ok(r) = TimeRange::new(Flicks::from_ticks(start), Flicks::from_ticks(duration)) {
            let json = serde_json::to_string(&r).unwrap();
            prop_assert_eq!(serde_json::from_str::<TimeRange>(&json).unwrap(), r);
        }
    }
}
