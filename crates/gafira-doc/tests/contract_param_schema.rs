//! `ParamSchema` through the public contract (ARQ-98, ARQ-110).

use gafira_color::{Color, Rgba};
use gafira_doc::contract::{
    ParamChoice, ParamDef, ParamKind, ParamSchema, ParamValue, ParamValues,
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
