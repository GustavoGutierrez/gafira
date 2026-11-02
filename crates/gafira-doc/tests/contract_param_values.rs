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
    let mut v = ParamValues::new();
    v.set("f", ParamValue::Float(1.5));
    v.set("i", ParamValue::Int(7));
    v.set("b", ParamValue::Bool(true));
    v.set("c", ParamValue::Choice("fast".into()));
    v.set("k", ParamValue::Color(red()));

    assert_eq!(v.float("f"), Some(1.5));
    assert_eq!(v.int("i"), Some(7));
    assert_eq!(v.bool("b"), Some(true));
    assert_eq!(v.choice("c"), Some("fast"));
    assert_eq!(v.color("k"), Some(red()));

    // Another kind, including Int <-> Float, is None.
    assert_eq!(v.float("i"), None);
    assert_eq!(v.int("f"), None);
    assert_eq!(v.bool("f"), None);
    assert_eq!(v.choice("b"), None);
    assert_eq!(v.color("c"), None);
    assert_eq!(v.float("k"), None);
    assert_eq!(v.int("c"), None);
    assert_eq!(v.bool("k"), None);
    assert_eq!(v.choice("i"), None);
    assert_eq!(v.color("f"), None);

    // Absent.
    assert_eq!(v.float("nope"), None);
    assert_eq!(v.int("nope"), None);
    assert_eq!(v.bool("nope"), None);
    assert_eq!(v.choice("nope"), None);
    assert_eq!(v.color("nope"), None);
}
