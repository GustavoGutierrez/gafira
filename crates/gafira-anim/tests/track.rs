use gafira_anim::{Easing, KeyTrack, KeyTrackRule, Keyframe};
use gafira_geom::Vec2;
use gafira_time::Flicks;

fn key(at: i64, value: f32, out: Easing) -> Keyframe<f32> {
    Keyframe {
        at: Flicks::from_ticks(at),
        value,
        out,
    }
}

fn lin(at: i64, value: f32) -> Keyframe<f32> {
    key(at, value, Easing::Linear)
}

fn bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> Easing {
    Easing::CubicBezier { x1, y1, x2, y2 }
}

type Case = (
    &'static str,
    Vec<Keyframe<f32>>,
    KeyTrackRule,
    Option<usize>,
);

fn fails(keys: Vec<Keyframe<f32>>) -> (KeyTrackRule, Option<usize>) {
    let e = KeyTrack::new(keys).expect_err("must be rejected");
    (e.rule, e.index)
}

// ARQ-18: each rule is reported with its name and the index of the offending key.
#[test]
fn arq_18_construction_names_the_rule_and_index() {
    use KeyTrackRule::*;
    let nan = f32::NAN;
    let cases: Vec<Case> = vec![
        ("empty", vec![], AtLeastOneKey, None),
        (
            "equal at",
            vec![lin(0, 0.0), lin(5, 1.0), lin(5, 2.0)],
            AtStrictlyIncreasing,
            Some(2),
        ),
        (
            "decreasing at",
            vec![lin(0, 0.0), lin(5, 1.0), lin(3, 2.0)],
            AtStrictlyIncreasing,
            Some(2),
        ),
        (
            "decreasing at i64",
            vec![lin(i64::MAX, 0.0), lin(i64::MIN, 1.0)],
            AtStrictlyIncreasing,
            Some(1),
        ),
        (
            "x1 below",
            vec![lin(0, 0.0), key(1, 0.0, bezier(-0.1, 0.0, 0.5, 1.0))],
            ValidEasing,
            Some(1),
        ),
        (
            "x1 above",
            vec![key(0, 0.0, bezier(1.1, 0.0, 0.5, 1.0))],
            ValidEasing,
            Some(0),
        ),
        (
            "x2 below",
            vec![
                lin(0, 0.0),
                lin(1, 0.0),
                key(2, 0.0, bezier(0.5, 0.0, -0.1, 1.0)),
            ],
            ValidEasing,
            Some(2),
        ),
        (
            "x2 above",
            vec![key(0, 0.0, bezier(0.5, 0.0, 1.1, 1.0))],
            ValidEasing,
            Some(0),
        ),
        (
            "y1 nan",
            vec![key(0, 0.0, bezier(0.5, nan, 0.5, 1.0))],
            ValidEasing,
            Some(0),
        ),
        (
            "y2 inf",
            vec![key(0, 0.0, bezier(0.5, 0.0, 0.5, f32::INFINITY))],
            ValidEasing,
            Some(0),
        ),
        (
            "x1 nan",
            vec![key(0, 0.0, bezier(nan, 0.0, 0.5, 1.0))],
            ValidEasing,
            Some(0),
        ),
        (
            "value nan",
            vec![lin(0, 0.0), lin(1, nan)],
            FiniteValue,
            Some(1),
        ),
        (
            "value inf",
            vec![lin(0, f32::NEG_INFINITY)],
            FiniteValue,
            Some(0),
        ),
    ];
    for (name, keys, rule, index) in cases {
        assert_eq!(fails(keys), (rule, index), "{name}");
    }
}

// ARQ-18: a Vec2 value with one non-finite component is rejected.
#[test]
fn arq_18_non_finite_vec2_component_is_rejected() {
    for v in [Vec2::new(f64::NAN, 0.0), Vec2::new(0.0, f64::INFINITY)] {
        let keys = vec![Keyframe {
            at: Flicks::ZERO,
            value: v,
            out: Easing::Hold,
        }];
        let e = KeyTrack::new(keys).unwrap_err();
        assert_eq!((e.rule, e.index), (KeyTrackRule::FiniteValue, Some(0)));
    }
}

// ARQ-18: a valid track is kept as given (no sorting, no repair); boundary cases are valid.
#[test]
fn arq_18_valid_tracks_are_kept_untouched() {
    let keys = vec![
        key(i64::MIN, 0.0, bezier(0.0, -5.0, 1.0, 9.0)),
        lin(-1, 1.0),
        key(0, 2.0, Easing::Hold),
        lin(1, 3.0),
        lin(i64::MAX, 4.0),
    ];
    let track = KeyTrack::new(keys.clone()).unwrap();
    assert_eq!(track.keys(), keys.as_slice());
    assert!(KeyTrack::new(vec![lin(7, 1.0)]).is_ok());
}

// ARQ-18: the error message names the rule and the index.
#[test]
fn arq_18_error_message_names_rule_and_index() {
    let e = KeyTrack::new(vec![lin(0, 0.0), lin(0, 1.0)]).unwrap_err();
    let msg = e.to_string();
    assert!(
        msg.contains("at strictly increasing") && msg.contains("index 1"),
        "{msg}"
    );
    let msg = KeyTrack::<f32>::new(vec![]).unwrap_err().to_string();
    assert!(msg.contains("at least one key"), "{msg}");
}
