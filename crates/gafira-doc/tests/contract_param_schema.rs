//! `ParamSchema` through the public contract (ARQ-98, ARQ-110).

use gafira_color::{Color, DisplayP3Rgba, LinearRgba, Rgba};
use gafira_doc::contract::{
    ParamChoice, ParamDef, ParamError, ParamKind, ParamSchema, ParamValue, ParamValues,
};
use gafira_doc::msg::MessageKey;

const DIRECTIONS: &[ParamChoice] = &[
    ParamChoice {
        id: "horizontal",
        label: MessageKey("param.direction_horizontal"),
    },
    ParamChoice {
        id: "vertical",
        label: MessageKey("param.direction_vertical"),
    },
];

fn red() -> Color {
    Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0))
}

fn def(id: &'static str, kind: ParamKind) -> ParamDef {
    ParamDef {
        id,
        label: MessageKey("param.label"),
        kind,
        animatable: false,
    }
}

fn float(id: &'static str, min: f64, max: f64, default: f64) -> ParamDef {
    def(id, ParamKind::Float { min, max, default })
}

fn one_of_each() -> ParamSchema {
    ParamSchema::new(vec![
        float("softness", 0.0, 1.0, 0.25),
        def(
            "steps",
            ParamKind::Int {
                min: 1,
                max: 9,
                default: 4,
            },
        ),
        def("invert", ParamKind::Bool { default: true }),
        def(
            "direction",
            ParamKind::Choice {
                options: DIRECTIONS,
                default: "vertical",
            },
        ),
        def("tint", ParamKind::Color { default: red() }),
    ])
}

// ARQ-110
#[test]
fn arq_110_new_keeps_the_definitions_in_order_and_validates_nothing() {
    assert!(ParamSchema::default().params().is_empty());
    let bad = vec![
        float("b", 5.0, 1.0, f64::INFINITY),
        float("b", 0.0, 1.0, 0.5),
        float("a", 0.0, 1.0, 0.5),
    ];
    let schema = ParamSchema::new(bad.clone());
    assert_eq!(schema.params(), bad.as_slice());
}

// ARQ-98
#[test]
fn arq_98_get_finds_the_first_definition_with_the_id() {
    let schema = ParamSchema::new(vec![
        float("a", 0.0, 1.0, 0.5),
        float("b", 0.0, 1.0, 0.1),
        float("b", 0.0, 2.0, 0.2),
    ]);
    assert_eq!(schema.get("a").map(|d| d.id), Some("a"));
    assert_eq!(
        schema.get("b").map(|d| d.kind.clone()),
        Some(ParamKind::Float {
            min: 0.0,
            max: 1.0,
            default: 0.1
        })
    );
    assert_eq!(schema.get("c"), None);
    assert_eq!(ParamSchema::default().get("a"), None);
}

// ARQ-110
#[test]
fn arq_110_defaults_gives_each_parameter_its_default_in_its_own_kind() {
    let mut want = ParamValues::new();
    want.set("softness", ParamValue::Float(0.25));
    want.set("steps", ParamValue::Int(4));
    want.set("invert", ParamValue::Bool(true));
    want.set("direction", ParamValue::Choice("vertical".into()));
    want.set("tint", ParamValue::Color(red()));
    assert_eq!(one_of_each().defaults(), want);
    assert_eq!(ParamSchema::default().defaults(), ParamValues::new());
}

const SAME_ID: &[ParamChoice] = &[
    ParamChoice {
        id: "a",
        label: MessageKey("param.one"),
    },
    ParamChoice {
        id: "a",
        label: MessageKey("param.two"),
    },
];

fn int(id: &'static str, min: i64, max: i64, default: i64) -> ParamDef {
    def(id, ParamKind::Int { min, max, default })
}

fn choice(id: &'static str, options: &'static [ParamChoice], default: &'static str) -> ParamDef {
    def(id, ParamKind::Choice { options, default })
}

fn color(id: &'static str, default: Color) -> ParamDef {
    def(id, ParamKind::Color { default })
}

fn bad(param: &'static str) -> Result<(), ParamError> {
    Err(ParamError::BadDefinition { param })
}

// ARQ-110
#[test]
fn arq_110_a_consistent_schema_is_a_valid_definition() {
    assert_eq!(one_of_each().validate_definition(), Ok(()));
    assert_eq!(ParamSchema::default().validate_definition(), Ok(()));
}

// ARQ-110
#[test]
fn arq_110_a_repeated_id_is_a_bad_definition() {
    let schemas = [
        ParamSchema::new(vec![float("a", 0.0, 1.0, 0.5), float("a", 0.0, 1.0, 0.5)]),
        ParamSchema::new(vec![
            float("a", 0.0, 1.0, 0.5),
            def("b", ParamKind::Bool { default: false }),
            def("a", ParamKind::Bool { default: false }),
        ]),
    ];
    for schema in schemas {
        assert_eq!(schema.validate_definition(), bad("a"));
    }
}

// ARQ-110
#[test]
fn arq_110_a_non_finite_bound_or_default_is_a_bad_definition() {
    let nan = f64::NAN;
    let inf = f64::INFINITY;
    let rows = [
        ("min NaN", float("p", nan, 1.0, 0.5)),
        ("min -inf", float("p", -inf, 1.0, 0.0)),
        ("min +inf", float("p", inf, inf, inf)),
        ("max NaN", float("p", 0.0, nan, 0.5)),
        ("max +inf", float("p", 0.0, inf, 0.5)),
        ("max -inf", float("p", -inf, -inf, -inf)),
        ("default NaN", float("p", 0.0, 1.0, nan)),
        ("default +inf", float("p", 0.0, 1.0, inf)),
        ("default -inf", float("p", 0.0, 1.0, -inf)),
        ("default NaN, no range", float("p", -inf, inf, nan)),
    ];
    for (name, d) in rows {
        assert_eq!(
            ParamSchema::new(vec![d]).validate_definition(),
            bad("p"),
            "{name}"
        );
    }
}

// ARQ-110
#[test]
fn arq_110_min_above_max_is_bad_and_min_equal_to_max_is_valid() {
    let rejected = [
        float("p", 2.0, 1.0, 1.5),
        float("p", 1.0_f64.next_up(), 1.0, 1.0),
        int("p", 2, 1, 1),
        int("p", i64::MAX, i64::MIN, 0),
        int("p", 4, 3, 3),
    ];
    for d in rejected {
        assert_eq!(
            ParamSchema::new(vec![d.clone()]).validate_definition(),
            bad("p"),
            "{d:?}"
        );
    }
    let accepted = [
        float("p", 1.0, 1.0, 1.0),
        int("p", 3, 3, 3),
        int("p", i64::MIN, i64::MIN, i64::MIN),
        int("p", i64::MAX, i64::MAX, i64::MAX),
    ];
    for d in accepted {
        assert_eq!(
            ParamSchema::new(vec![d.clone()]).validate_definition(),
            Ok(()),
            "{d:?}"
        );
    }
}

// ARQ-110
#[test]
fn arq_110_a_default_outside_the_range_is_bad_and_a_bound_is_inside() {
    let rejected = [
        float("p", 0.0, 1.0, 0.0_f64.next_down()),
        float("p", 0.0, 1.0, 1.0_f64.next_up()),
        float("p", -2.0, -1.0, 0.0),
        int("p", 0, 5, -1),
        int("p", 0, 5, 6),
        int("p", i64::MIN + 1, i64::MAX, i64::MIN),
        int("p", i64::MIN, i64::MAX - 1, i64::MAX),
    ];
    for d in rejected {
        assert_eq!(
            ParamSchema::new(vec![d.clone()]).validate_definition(),
            bad("p"),
            "{d:?}"
        );
    }
    let accepted = [
        float("p", 0.0, 1.0, 0.0),
        float("p", 0.0, 1.0, 1.0),
        float("p", -1.0, 1.0, 0.0),
        int("p", 0, 5, 0),
        int("p", 0, 5, 5),
        int("p", i64::MIN, i64::MAX, i64::MIN),
        int("p", i64::MIN, i64::MAX, i64::MAX),
    ];
    for d in accepted {
        assert_eq!(
            ParamSchema::new(vec![d.clone()]).validate_definition(),
            Ok(()),
            "{d:?}"
        );
    }
}

// ARQ-110
#[test]
fn arq_110_a_choice_default_must_be_an_option_and_option_ids_must_differ() {
    let rejected = [
        choice("p", DIRECTIONS, "diagonal"),
        choice("p", DIRECTIONS, ""),
        choice("p", DIRECTIONS, "Vertical"),
        choice("p", &[], "vertical"),
        choice("p", SAME_ID, "a"),
    ];
    for d in rejected {
        assert_eq!(
            ParamSchema::new(vec![d.clone()]).validate_definition(),
            bad("p"),
            "{d:?}"
        );
    }
    for default in ["horizontal", "vertical"] {
        let schema = ParamSchema::new(vec![choice("p", DIRECTIONS, default)]);
        assert_eq!(schema.validate_definition(), Ok(()), "{default}");
    }
}

// ARQ-110
#[test]
fn arq_110_a_non_finite_color_default_is_a_bad_definition() {
    let nan = f32::NAN;
    let inf = f32::INFINITY;
    let rejected = [
        Color::Srgb(Rgba::new(nan, 0.0, 0.0, 1.0)),
        Color::Srgb(Rgba::new(0.0, inf, 0.0, 1.0)),
        Color::Srgb(Rgba::new(0.0, 0.0, -inf, 1.0)),
        Color::Srgb(Rgba::new(0.0, 0.0, 0.0, nan)),
        Color::LinearSrgb(LinearRgba::new(0.0, 0.0, 0.0, inf)),
        Color::DisplayP3(DisplayP3Rgba::new(nan, 0.0, 0.0, 1.0)),
    ];
    for c in rejected {
        assert_eq!(
            ParamSchema::new(vec![color("p", c)]).validate_definition(),
            bad("p"),
            "{c:?}"
        );
    }
    let accepted = [
        red(),
        Color::LinearSrgb(LinearRgba::new(0.5, 0.5, 0.5, 0.0)),
        Color::DisplayP3(DisplayP3Rgba::new(0.1, 0.2, 0.3, 1.0)),
    ];
    for c in accepted {
        assert_eq!(
            ParamSchema::new(vec![color("p", c)]).validate_definition(),
            Ok(()),
            "{c:?}"
        );
    }
}

// ARQ-110
#[test]
fn arq_110_validate_definition_reports_the_first_fault_in_definition_order() {
    // Two faults of different kinds: the earlier parameter wins, whatever the id order.
    let schema = ParamSchema::new(vec![
        float("z", 2.0, 1.0, 1.5),
        float("ok", 0.0, 1.0, 0.5),
        float("a", f64::NAN, 1.0, 0.5),
    ]);
    assert_eq!(schema.validate_definition(), bad("z"));

    // A repeated id comes first when its position is first.
    let schema = ParamSchema::new(vec![
        float("a", 0.0, 1.0, 0.5),
        float("a", 0.0, 1.0, 0.5),
        float("b", 2.0, 1.0, 1.5),
    ]);
    assert_eq!(schema.validate_definition(), bad("a"));

    // A bad bound comes first when its position is first.
    let schema = ParamSchema::new(vec![
        float("ok", 0.0, 1.0, 0.5),
        float("b", 2.0, 1.0, 1.5),
        float("ok", 0.0, 1.0, 0.5),
    ]);
    assert_eq!(schema.validate_definition(), bad("b"));

    // Several faults in a late parameter do not hide an earlier different one.
    let schema = ParamSchema::new(vec![
        choice("c", DIRECTIONS, "diagonal"),
        float("d", f64::NAN, f64::NAN, f64::NAN),
        float("c", 0.0, 1.0, 0.5),
    ]);
    assert_eq!(schema.validate_definition(), bad("c"));
}

fn values(entries: &[(&str, ParamValue)]) -> ParamValues {
    let mut v = ParamValues::new();
    for (id, value) in entries {
        v.set(*id, value.clone());
    }
    v
}

fn only(id: &str, value: ParamValue) -> ParamValues {
    values(&[(id, value)])
}

fn err(kind: fn(&'static str) -> ParamError, param: &'static str) -> Result<(), ParamError> {
    Err(kind(param))
}

fn non_finite(param: &'static str) -> ParamError {
    ParamError::NonFinite { param }
}

fn out_of_range(param: &'static str) -> ParamError {
    ParamError::OutOfRange { param }
}

fn wrong_type(param: &'static str) -> ParamError {
    ParamError::WrongType { param }
}

// ARQ-110
#[test]
fn arq_110_check_values_accepts_values_inside_the_schema_and_bounds_are_inclusive() {
    let schema = one_of_each();
    let ok = [
        ("softness", ParamValue::Float(0.0)),
        ("softness", ParamValue::Float(1.0)),
        ("softness", ParamValue::Float(-0.0)),
        ("softness", ParamValue::Float(0.5)),
        ("steps", ParamValue::Int(1)),
        ("steps", ParamValue::Int(9)),
        ("steps", ParamValue::Int(5)),
        ("invert", ParamValue::Bool(false)),
        ("invert", ParamValue::Bool(true)),
        ("direction", ParamValue::Choice("horizontal".into())),
        ("direction", ParamValue::Choice("vertical".into())),
        ("tint", ParamValue::Color(red())),
    ];
    for (id, v) in ok {
        assert_eq!(
            schema.check_values(&only(id, v.clone())),
            Ok(()),
            "{id} = {v:?}"
        );
    }
    assert_eq!(schema.check_values(&schema.defaults()), Ok(()));

    let extremes = ParamSchema::new(vec![int("n", i64::MIN, i64::MAX, 0)]);
    for v in [i64::MIN, i64::MAX] {
        assert_eq!(
            extremes.check_values(&only("n", ParamValue::Int(v))),
            Ok(())
        );
    }
}

// ARQ-110
#[test]
fn arq_110_check_values_rejects_a_number_just_outside_the_range() {
    let schema = ParamSchema::new(vec![float("f", 0.0, 10.0, 5.0), int("i", -5, 5, 0)]);
    let rows = [
        ("f", ParamValue::Float(0.0_f64.next_down())),
        ("f", ParamValue::Float(10.0_f64.next_up())),
        ("f", ParamValue::Float(-1.0)),
        ("f", ParamValue::Float(11.0)),
        ("i", ParamValue::Int(-6)),
        ("i", ParamValue::Int(6)),
        ("i", ParamValue::Int(i64::MIN)),
        ("i", ParamValue::Int(i64::MAX)),
    ];
    for (id, v) in rows {
        let want = Err(ParamError::OutOfRange {
            param: if id == "f" { "f" } else { "i" },
        });
        assert_eq!(
            schema.check_values(&only(id, v.clone())),
            want,
            "{id} = {v:?}"
        );
    }
}

// ARQ-110
#[test]
fn arq_110_check_values_reports_non_finite_before_the_range() {
    let schema = ParamSchema::new(vec![float("f", 0.0, 10.0, 5.0), color("k", red())]);
    for x in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            schema.check_values(&only("f", ParamValue::Float(x))),
            err(non_finite, "f"),
            "{x}"
        );
    }
    let nan = f32::NAN;
    let inf = f32::INFINITY;
    let colors = [
        Color::Srgb(Rgba::new(nan, 0.0, 0.0, 1.0)),
        Color::Srgb(Rgba::new(0.0, inf, 0.0, 1.0)),
        Color::Srgb(Rgba::new(0.0, 0.0, 0.0, nan)),
        Color::LinearSrgb(LinearRgba::new(0.0, 0.0, -inf, 1.0)),
        Color::DisplayP3(DisplayP3Rgba::new(0.0, nan, 0.0, 1.0)),
    ];
    for c in colors {
        assert_eq!(
            schema.check_values(&only("k", ParamValue::Color(c))),
            err(non_finite, "k"),
            "{c:?}"
        );
    }
}

// ARQ-110
#[test]
fn arq_110_check_values_rejects_a_choice_that_is_not_an_option() {
    let schema = one_of_each();
    for id in ["diagonal", "", "Vertical", "vertical "] {
        assert_eq!(
            schema.check_values(&only("direction", ParamValue::Choice(id.into()))),
            err(out_of_range, "direction"),
            "{id:?}"
        );
    }
}

// ARQ-110
#[test]
fn arq_110_check_values_rejects_every_other_kind_without_converting() {
    let schema = one_of_each();
    let samples = [
        ("softness", ParamValue::Float(0.5)),
        ("steps", ParamValue::Int(3)),
        ("invert", ParamValue::Bool(true)),
        ("direction", ParamValue::Choice("vertical".into())),
        ("tint", ParamValue::Color(red())),
    ];
    for (id, own) in &samples {
        for (_, other) in &samples {
            let v = only(id, other.clone());
            let got = schema.check_values(&v);
            let param = schema.get(id).map(|d| d.id).unwrap();
            if std::mem::discriminant(own) == std::mem::discriminant(other) {
                assert_eq!(got, Ok(()), "{id} = {other:?}");
            } else {
                assert_eq!(got, err(wrong_type, param), "{id} = {other:?}");
            }
        }
    }
    // A conversion would put these in or out of range; the kind is wrong first.
    let schema = ParamSchema::new(vec![float("f", 0.0, 1.0, 0.5), int("i", 0, 9, 1)]);
    assert_eq!(
        schema.check_values(&only("f", ParamValue::Int(100))),
        err(wrong_type, "f")
    );
    assert_eq!(
        schema.check_values(&only("i", ParamValue::Float(100.0))),
        err(wrong_type, "i")
    );
    assert_eq!(
        schema.check_values(&only("f", ParamValue::Int(0))),
        err(wrong_type, "f")
    );
    assert_eq!(
        schema.check_values(&only("i", ParamValue::Float(1.0))),
        err(wrong_type, "i")
    );
}

// ARQ-110
#[test]
fn arq_110_a_missing_id_passes_and_an_unknown_id_is_ignored() {
    let schema = one_of_each();
    assert_eq!(schema.check_values(&ParamValues::new()), Ok(()));
    assert_eq!(
        schema.check_values(&only("softness", ParamValue::Float(0.5))),
        Ok(())
    );
    // Unknown ids are not checked, whatever they hold.
    let stray = values(&[
        ("from_a_newer_build", ParamValue::Float(f64::NAN)),
        ("softness", ParamValue::Float(0.5)),
        ("zzz", ParamValue::Choice("anything".into())),
    ]);
    assert_eq!(schema.check_values(&stray), Ok(()));
}

// ARQ-110
#[test]
fn arq_110_check_values_reports_the_first_fault_in_definition_order() {
    // Schema order is z, a; id order is a, z.
    let schema = ParamSchema::new(vec![float("z", 0.0, 1.0, 0.5), float("a", 0.0, 1.0, 0.5)]);
    let both_bad = values(&[("a", ParamValue::Float(9.0)), ("z", ParamValue::Float(9.0))]);
    assert_eq!(schema.check_values(&both_bad), err(out_of_range, "z"));

    let kinds = values(&[
        ("a", ParamValue::Float(f64::NAN)),
        ("z", ParamValue::Bool(true)),
    ]);
    assert_eq!(schema.check_values(&kinds), err(wrong_type, "z"));

    let only_a = values(&[
        ("a", ParamValue::Float(f64::NAN)),
        ("z", ParamValue::Float(0.5)),
    ]);
    assert_eq!(schema.check_values(&only_a), err(non_finite, "a"));
}

// ARQ-110
#[test]
fn arq_110_resolve_fills_missing_ids_and_keeps_given_and_unknown_values() {
    let schema = one_of_each();
    let given = values(&[
        ("softness", ParamValue::Float(0.75)),
        ("invert", ParamValue::Bool(false)),
        ("from_a_newer_build", ParamValue::Int(42)),
    ]);
    let got = schema.resolve(&given).unwrap();
    let want = values(&[
        ("softness", ParamValue::Float(0.75)),
        ("steps", ParamValue::Int(4)),
        ("invert", ParamValue::Bool(false)),
        ("direction", ParamValue::Choice("vertical".into())),
        ("tint", ParamValue::Color(red())),
        ("from_a_newer_build", ParamValue::Int(42)),
    ]);
    assert_eq!(got, want);

    assert_eq!(
        schema.resolve(&ParamValues::new()).unwrap(),
        schema.defaults()
    );
    assert_eq!(
        schema.resolve(&schema.defaults()).unwrap(),
        schema.defaults()
    );
}

// ARQ-110
#[test]
fn arq_110_resolve_returns_the_error_of_check_values() {
    let schema = one_of_each();
    let rows = [
        only("softness", ParamValue::Float(f64::NAN)),
        only("softness", ParamValue::Float(2.0)),
        only("steps", ParamValue::Float(2.0)),
        only("direction", ParamValue::Choice("diagonal".into())),
    ];
    for v in rows {
        let want = schema.check_values(&v).unwrap_err();
        assert_eq!(schema.resolve(&v), Err(want), "{v:?}");
    }
}

// ARQ-110
#[test]
fn arq_110_resolve_checks_the_given_values_before_filling_defaults() {
    // A default is not looked at: only what the caller gave is checked first.
    let schema = ParamSchema::new(vec![float("p", 0.0, 1.0, 5.0)]);
    let got = schema.resolve(&ParamValues::new()).unwrap();
    assert_eq!(got.float("p"), Some(5.0));
}
