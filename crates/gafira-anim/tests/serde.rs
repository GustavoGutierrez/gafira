use gafira_anim::{Animatable, Easing, KeyTrack, Keyframe};
use gafira_geom::Vec2;
use gafira_time::Flicks;
use proptest::prelude::*;

fn read(json: &str) -> Result<Animatable<f32>, String> {
    serde_json::from_str(json).map_err(|e| e.to_string())
}

// ARQ-18 golden: Static(1.5) is written bare, and a bare value reads as Static.
#[test]
fn arq_18_static_is_written_bare_and_read_back() {
    assert_eq!(
        serde_json::to_string(&Animatable::Static(1.5_f32)).unwrap(),
        "1.5"
    );
    assert_eq!(read("1.5").unwrap(), Animatable::Static(1.5));
    let v: Animatable<Vec2> = serde_json::from_str(r#"{"x":1.0,"y":2.0}"#).unwrap();
    assert_eq!(v, Animatable::Static(Vec2::new(1.0, 2.0)));
}

// ARQ-18 golden: the Keyed wire format and the Easing representation.
#[test]
fn arq_18_keyed_wire_format() {
    let a = Animatable::Keyed(
        KeyTrack::new(vec![
            Keyframe {
                at: Flicks::from_ticks(0),
                value: 1.0_f32,
                out: Easing::Linear,
            },
            Keyframe {
                at: Flicks::from_ticks(5),
                value: 2.0,
                out: Easing::CubicBezier {
                    x1: 0.5,
                    y1: 0.0,
                    x2: 1.0,
                    y2: 2.0,
                },
            },
            Keyframe {
                at: Flicks::from_ticks(9),
                value: 3.0,
                out: Easing::Hold,
            },
        ])
        .unwrap(),
    );
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        concat!(
            r#"{"keys":[{"at":0,"value":1.0,"out":"Linear"},"#,
            r#"{"at":5,"value":2.0,"out":{"CubicBezier":{"x1":0.5,"y1":0.0,"x2":1.0,"y2":2.0}}},"#,
            r#"{"at":9,"value":3.0,"out":"Hold"}]}"#
        )
    );
}

fn keyed(keys: &str) -> String {
    format!(r#"{{"keys":[{keys}]}}"#)
}

fn k(at: i64, out: &str) -> String {
    format!(r#"{{"at":{at},"value":1.0,"out":{out}}}"#)
}

fn bad_bezier(x1: f32, x2: f32) -> String {
    format!(r#"{{"CubicBezier":{{"x1":{x1},"y1":0.0,"x2":{x2},"y2":1.0}}}}"#)
}

// ARQ-18: an invalid track read as Animatable<T> (and as KeyTrack) names the rule and the index.
#[test]
fn arq_18_invalid_tracks_fail_naming_rule_and_index() {
    let lin = r#""Linear""#;
    let cases = [
        ("empty", keyed(""), "at least one key", None),
        (
            "equal at",
            keyed(&format!("{},{}", k(0, lin), k(0, lin))),
            "at strictly increasing",
            Some(1),
        ),
        (
            "decreasing at",
            keyed(&format!("{},{},{}", k(0, lin), k(5, lin), k(3, lin))),
            "at strictly increasing",
            Some(2),
        ),
        (
            "x1 above",
            keyed(&format!("{},{}", k(0, lin), k(1, &bad_bezier(2.0, 0.5)))),
            "valid easing",
            Some(1),
        ),
        (
            "x2 below",
            keyed(&k(0, &bad_bezier(0.5, -0.5))),
            "valid easing",
            Some(0),
        ),
    ];
    for (name, json, rule, index) in cases {
        let want_index = index.map(|i| format!("key index {i}"));
        let from_animatable = read(&json).expect_err(name);
        let from_track = serde_json::from_str::<KeyTrack<f32>>(&json)
            .expect_err(name)
            .to_string();
        for msg in [&from_animatable, &from_track] {
            assert!(msg.contains(rule), "{name}: {msg}");
            if let Some(i) = &want_index {
                assert!(msg.contains(i.as_str()), "{name}: {msg}");
            }
        }
    }
}

// ARQ-18: a track of the wrong shape still fails (no silent repair).
#[test]
fn arq_18_wrong_shape_fails() {
    assert!(read(r#"{"keys":"x"}"#).is_err());
    assert!(read(r#""text""#).is_err());
}

// ARQ-18 residual ambiguity: T fields plus `keys` read as Static and `keys` is ignored.
#[test]
fn arq_18_hand_written_object_with_keys_reads_as_static() {
    let v: Animatable<Vec2> = serde_json::from_str(r#"{"x":1,"y":2,"keys":[]}"#).unwrap();
    assert_eq!(v, Animatable::Static(Vec2::new(1.0, 2.0)));
}

fn easing() -> impl Strategy<Value = Easing> {
    prop_oneof![
        Just(Easing::Hold),
        Just(Easing::Linear),
        Just(Easing::EaseIn),
        Just(Easing::EaseOut),
        Just(Easing::EaseInOut),
        (0.0_f32..=1.0, -2.0_f32..3.0, 0.0_f32..=1.0, -2.0_f32..3.0)
            .prop_map(|(x1, y1, x2, y2)| Easing::CubicBezier { x1, y1, x2, y2 }),
    ]
}

fn animatable<T: gafira_anim::Interpolate + std::fmt::Debug>(
    value: BoxedStrategy<T>,
) -> impl Strategy<Value = Animatable<T>> {
    prop_oneof![
        value.clone().prop_map(Animatable::Static),
        (
            proptest::collection::btree_set(any::<i64>(), 1..6),
            proptest::collection::vec((value, easing()), 6)
        )
            .prop_map(|(ats, vals)| {
                let keys = ats
                    .into_iter()
                    .zip(vals)
                    .map(|(at, (value, out))| Keyframe {
                        at: Flicks::from_ticks(at),
                        value,
                        out,
                    })
                    .collect();
                Animatable::Keyed(KeyTrack::new(keys).unwrap())
            }),
    ]
}

fn f32_value() -> BoxedStrategy<f32> {
    prop_oneof![
        -1.0e30_f32..1.0e30,
        Just(0.0),
        Just(f32::MIN_POSITIVE),
        Just(1.0e-40)
    ]
    .boxed()
}

fn vec2_value() -> BoxedStrategy<Vec2> {
    (-1.0e300_f64..1.0e300, -1.0e300_f64..1.0e300)
        .prop_map(|(x, y)| Vec2::new(x, y))
        .boxed()
}

fn round_trips<T>(a: &Animatable<T>) -> Result<(), TestCaseError>
where
    T: gafira_anim::Interpolate + std::fmt::Debug + serde::Serialize + serde::de::DeserializeOwned,
{
    let json = serde_json::to_string(a).unwrap();
    let back: Animatable<T> = serde_json::from_str(&json).unwrap();
    prop_assert_eq!(std::mem::discriminant(a), std::mem::discriminant(&back));
    prop_assert_eq!(a, &back);
    Ok(())
}

proptest! {
    // ARQ-18: deserialize(serialize(a)) == a and keeps the variant (f32 and Vec2 values).
    #[test]
    fn arq_18_f32_round_trips(a in animatable(f32_value())) {
        round_trips(&a)?;
    }

    #[test]
    fn arq_18_vec2_round_trips(a in animatable(vec2_value())) {
        round_trips(&a)?;
    }
}
