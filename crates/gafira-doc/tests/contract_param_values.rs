//! `ParamValues` through the public contract (ARQ-110).

use gafira_color::{Color, Rgba};
use gafira_doc::contract::{ParamValue, ParamValues};

fn red() -> Color {
    Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0))
}

// ARQ-110
#[test]
fn arq_110_set_stores_without_validating_and_returns_the_replaced_value() {
    let mut v = ParamValues::new();
    assert_eq!(v.set("radius", ParamValue::Float(2.0)), None);
    assert_eq!(v.get("radius"), Some(&ParamValue::Float(2.0)));
    assert_eq!(
        v.set("radius", ParamValue::Float(3.0)),
        Some(ParamValue::Float(2.0))
    );
    assert_eq!(v.get("radius"), Some(&ParamValue::Float(3.0)));
    assert_eq!(v.get("missing"), None);

    // Plain data: a non-finite number, an empty id and a stray kind are all kept.
    v.set("", ParamValue::Float(f64::NAN));
    assert!(matches!(v.get(""), Some(ParamValue::Float(x)) if x.is_nan()));
}

// ARQ-111
#[test]
fn arq_111_iter_follows_id_order_not_insertion_order() {
    let mut v = ParamValues::new();
    v.set("zeta", ParamValue::Bool(true));
    v.set("alpha", ParamValue::Int(1));
    v.set("mid", ParamValue::Choice("a".into()));
    let seen: Vec<(&str, &ParamValue)> = v.iter().collect();
    assert_eq!(
        seen,
        vec![
            ("alpha", &ParamValue::Int(1)),
            ("mid", &ParamValue::Choice("a".into())),
            ("zeta", &ParamValue::Bool(true)),
        ]
    );
}

// ARQ-110
#[test]
fn arq_110_accessors_return_only_their_own_kind_and_never_convert() {
    let samples = [
        ParamValue::Float(1.5),
        ParamValue::Int(7),
        ParamValue::Bool(true),
        ParamValue::Choice("fast".into()),
        ParamValue::Color(red()),
    ];
    let mut v = ParamValues::new();
    for (i, sample) in samples.iter().enumerate() {
        v.set(format!("p{i}"), sample.clone());
    }
    type Accessor = fn(&ParamValues, &str) -> Option<ParamValue>;
    let accessors: [Accessor; 5] = [
        |v, id| v.float(id).map(ParamValue::Float),
        |v, id| v.int(id).map(ParamValue::Int),
        |v, id| v.bool(id).map(ParamValue::Bool),
        |v, id| v.choice(id).map(|s| ParamValue::Choice(s.to_owned())),
        |v, id| v.color(id).map(ParamValue::Color),
    ];
    for (kind, accessor) in accessors.iter().enumerate() {
        for (i, sample) in samples.iter().enumerate() {
            let want = (i == kind).then(|| sample.clone());
            assert_eq!(
                accessor(&v, &format!("p{i}")),
                want,
                "accessor {kind} on p{i}"
            );
        }
        assert_eq!(
            accessor(&v, "absent"),
            None,
            "accessor {kind} on an absent id"
        );
    }
}
