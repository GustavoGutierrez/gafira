//! ARQ-102: every `LayoutError` variant maps to its table key and argument names.
//! The test never calls `Display`: the message carries a key and arguments, no prose.

use gafira_doc::NodeId;
use gafira_doc::msg::{Message, MessageKey, MsgArg, ToMessage, is_valid_key};
use gafira_layout::LayoutError;

fn message(key: &'static str, args: Vec<(&'static str, MsgArg)>) -> Message {
    Message {
        key: MessageKey(key),
        args,
    }
}

fn text(value: &str) -> MsgArg {
    MsgArg::Text(value.to_string())
}

/// One fixture per variant: the error and the message the design table gives.
fn fixtures() -> Vec<(LayoutError, Message)> {
    vec![
        (
            LayoutError::NonFinite { field: "bleed" },
            message("error.layout.non_finite", vec![("field", text("bleed"))]),
        ),
        (
            LayoutError::InvalidBox { field: "margin" },
            message("error.layout.invalid_box", vec![("field", text("margin"))]),
        ),
        (
            LayoutError::OutOfRange { field: "gutter" },
            message("error.layout.out_of_range", vec![("field", text("gutter"))]),
        ),
        (LayoutError::NoArea, message("error.layout.no_area", vec![])),
        (
            LayoutError::UnknownNode(NodeId::new(7)),
            message("error.layout.unknown_node", vec![("node", text("7"))]),
        ),
        (
            LayoutError::Cycle(NodeId::new(42)),
            message("error.layout.cycle", vec![("node", text("42"))]),
        ),
    ]
}

#[test]
fn arq_102_every_layout_variant_gives_its_table_key_and_arguments() {
    for (error, expected) in fixtures() {
        assert_eq!(error.to_message(), expected, "{error:?}");
    }
}

#[test]
fn arq_102_layout_keys_are_valid_and_distinct() {
    let keys: Vec<&str> = fixtures()
        .iter()
        .map(|(e, _)| e.to_message().key.0)
        .collect();
    assert_eq!(keys.len(), 6, "a fixture per variant");
    for (i, key) in keys.iter().enumerate() {
        assert!(is_valid_key(key), "{key} is not a valid key");
        assert!(!keys[..i].contains(key), "{key} is used by two variants");
    }
}
