//! ARQ-102: every `TextError` variant maps to its key and argument names.

use gafira_doc::msg::{Message, MessageKey, MsgArg, ToMessage, is_valid_key};
use gafira_text::TextError;

fn message(key: &'static str, args: Vec<(&'static str, MsgArg)>) -> Message {
    Message {
        key: MessageKey(key),
        args,
    }
}

fn text(value: &str) -> MsgArg {
    MsgArg::Text(value.to_string())
}

fn fixtures() -> Vec<(TextError, Message)> {
    vec![
        (
            TextError::NonFinite { field: "dpi" },
            message("error.text.non_finite", vec![("field", text("dpi"))]),
        ),
        (
            TextError::OutOfRange {
                field: "style.size_pt",
            },
            message(
                "error.text.out_of_range",
                vec![("field", text("style.size_pt"))],
            ),
        ),
        (
            TextError::InvalidFont { index: 2 },
            message("error.text.invalid_font", vec![("index", MsgArg::Int(2))]),
        ),
        (TextError::NoFont, message("error.text.no_font", vec![])),
    ]
}

#[test]
fn arq_102_every_text_variant_gives_its_key_and_arguments() {
    for (error, expected) in fixtures() {
        assert_eq!(error.to_message(), expected, "{error:?}");
    }
}

#[test]
fn arq_102_text_keys_are_valid_and_distinct() {
    let keys: Vec<&str> = fixtures()
        .iter()
        .map(|(e, _)| e.to_message().key.0)
        .collect();
    assert_eq!(keys.len(), 4, "a fixture per variant");
    for (i, key) in keys.iter().enumerate() {
        assert!(is_valid_key(key), "{key}");
        assert!(!keys[..i].contains(key), "{key} repeats");
    }
}
