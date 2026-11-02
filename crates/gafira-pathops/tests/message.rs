//! ARQ-102: every `PathOpError` variant maps to its table key and argument names.
//! The test never calls `Display`: the message carries a key and arguments, no prose,
//! and the non-finite class and the geometry string stay out of it.

use gafira_doc::msg::{Message, MessageKey, MsgArg, ToMessage, is_valid_key};
use gafira_pathops::{InvalidNumber, NonFiniteValue, PathOpError};

fn message(key: &'static str, args: Vec<(&'static str, MsgArg)>) -> Message {
    Message {
        key: MessageKey(key),
        args,
    }
}

fn decimal(name: &'static str, value: f64) -> Vec<(&'static str, MsgArg)> {
    vec![(name, MsgArg::Decimal(value))]
}

/// One fixture per table row: the error and the message the design table gives.
fn fixtures() -> Vec<(PathOpError, Message)> {
    vec![
        (
            PathOpError::NonFiniteCoordinate {
                value: NonFiniteValue::Nan,
            },
            message("error.pathops.non_finite_coordinate", vec![]),
        ),
        (
            PathOpError::CoordinateOutOfRange {
                value: 2.5e9,
                limit: 1e9,
            },
            message(
                "error.pathops.coordinate_out_of_range",
                vec![
                    ("value", MsgArg::Decimal(2.5e9)),
                    ("limit", MsgArg::Decimal(1e9)),
                ],
            ),
        ),
        (
            PathOpError::OpenSubpath { subpath: 3 },
            message(
                "error.pathops.open_subpath",
                vec![("subpath", MsgArg::Int(3))],
            ),
        ),
        (
            PathOpError::InvalidStrokeWidth {
                cause: InvalidNumber::NotPositive(-1.5),
            },
            message(
                "error.pathops.stroke_width_not_positive",
                decimal("value", -1.5),
            ),
        ),
        (
            PathOpError::InvalidStrokeWidth {
                cause: InvalidNumber::NonFinite(NonFiniteValue::Infinity),
            },
            message("error.pathops.stroke_width_non_finite", vec![]),
        ),
        (
            PathOpError::InvalidTolerance {
                cause: InvalidNumber::NotPositive(0.0),
            },
            message(
                "error.pathops.tolerance_not_positive",
                decimal("value", 0.0),
            ),
        ),
        (
            PathOpError::InvalidTolerance {
                cause: InvalidNumber::NonFinite(NonFiniteValue::NegativeInfinity),
            },
            message("error.pathops.tolerance_non_finite", vec![]),
        ),
        (
            PathOpError::ToleranceTooFine { minimum: 0.25 },
            message("error.pathops.tolerance_too_fine", decimal("minimum", 0.25)),
        ),
        (
            PathOpError::Geometry("library text".to_string()),
            message("error.pathops.geometry", vec![]),
        ),
    ]
}

#[test]
fn arq_102_every_pathops_variant_gives_its_table_key_and_arguments() {
    for (error, expected) in fixtures() {
        assert_eq!(error.to_message(), expected, "{error:?}");
    }
}

#[test]
fn arq_102_pathops_keys_are_valid_and_distinct() {
    let keys: Vec<&str> = fixtures()
        .iter()
        .map(|(e, _)| e.to_message().key.0)
        .collect();
    assert_eq!(keys.len(), 9, "a fixture per table row");
    for (i, key) in keys.iter().enumerate() {
        assert!(is_valid_key(key), "{key} is not a valid key");
        assert!(!keys[..i].contains(key), "{key} is used by two rows");
    }
}

#[test]
fn arq_102_the_non_finite_class_and_the_geometry_text_stay_out_of_the_message() {
    let nan = |value| PathOpError::NonFiniteCoordinate { value };
    for kind in [
        NonFiniteValue::Nan,
        NonFiniteValue::Infinity,
        NonFiniteValue::NegativeInfinity,
    ] {
        assert_eq!(
            nan(kind).to_message(),
            nan(NonFiniteValue::Nan).to_message()
        );
    }
    for kind in [NonFiniteValue::Infinity, NonFiniteValue::NegativeInfinity] {
        let width = PathOpError::InvalidStrokeWidth {
            cause: InvalidNumber::NonFinite(kind),
        };
        let tolerance = PathOpError::InvalidTolerance {
            cause: InvalidNumber::NonFinite(kind),
        };
        assert!(width.to_message().args.is_empty());
        assert!(tolerance.to_message().args.is_empty());
    }
    let a = PathOpError::Geometry("one".to_string()).to_message();
    let b = PathOpError::Geometry("two".to_string()).to_message();
    assert_eq!(a, b);
}

#[test]
fn arq_102_an_open_subpath_index_saturates_at_i64_max() {
    // On a 32-bit target `usize::MAX` fits an `i64`; the rule is the same.
    let cases = [
        (0, 0),
        (usize::MAX, i64::try_from(usize::MAX).unwrap_or(i64::MAX)),
    ];
    for (subpath, expected) in cases {
        let message = PathOpError::OpenSubpath { subpath }.to_message();
        assert_eq!(message.args, vec![("subpath", MsgArg::Int(expected))]);
    }
}
