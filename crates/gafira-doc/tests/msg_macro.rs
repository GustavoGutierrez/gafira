//! `msg!` from a consumer crate (ARQ-102, ARQ-107).

use gafira_doc::msg;
use gafira_doc::msg::{Message, MessageKey, MsgArg, Unit};

#[test]
fn arq_102_msg_with_no_arguments_has_the_key_and_no_args() {
    let m = msg!("error.layout.no_area");
    assert_eq!(
        m,
        Message {
            key: MessageKey("error.layout.no_area"),
            args: vec![]
        }
    );
}

#[test]
fn arq_102_msg_keeps_argument_names_and_values_in_order() {
    let one = msg!("export.pages_selected", count = Count(3));
    assert_eq!(one.key, MessageKey("export.pages_selected"));
    assert_eq!(one.args, vec![("count", MsgArg::Count(3))]);

    let two = msg!("a.b", first = Int(-1), second = Quantity(2.5, Unit::Mm));
    assert_eq!(
        two.args,
        vec![
            ("first", MsgArg::Int(-1)),
            ("second", MsgArg::Quantity(2.5, Unit::Mm))
        ]
    );
}

#[test]
fn arq_102_msg_accepts_trailing_commas() {
    let inner = msg!("a.b", x = Text("v".to_string()),);
    assert_eq!(inner.args, vec![("x", MsgArg::Text("v".to_string()))]);
    let list = msg!("a.b", x = Int(1), y = Int(2),);
    assert_eq!(
        list.args,
        vec![("x", MsgArg::Int(1)), ("y", MsgArg::Int(2))]
    );
    let in_variant = msg!("a.b", q = Quantity(1.5, Unit::Px,));
    assert_eq!(
        in_variant.args,
        vec![("q", MsgArg::Quantity(1.5, Unit::Px))]
    );
    let key_only = msg!("a.b",);
    assert!(key_only.args.is_empty());
}
