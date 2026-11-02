//! `ParamError` as a message (ARQ-102, ADR 0012). Never calls `Display`.

use gafira_doc::contract::ParamError;
use gafira_doc::msg::{Message, MessageKey, MsgArg, ToMessage, is_valid_key};

fn fixtures() -> Vec<(ParamError, &'static str)> {
    vec![
        (
            ParamError::NonFinite { param: "radius" },
            "error.param.non_finite",
        ),
        (
            ParamError::OutOfRange { param: "radius" },
            "error.param.out_of_range",
        ),
        (
            ParamError::WrongType { param: "radius" },
            "error.param.wrong_type",
        ),
        (
            ParamError::BadDefinition { param: "radius" },
            "error.param.bad_definition",
        ),
    ]
}

// ARQ-102
#[test]
fn arq_102_each_param_error_gives_its_key_and_the_param_argument() {
    for (error, key) in fixtures() {
        assert_eq!(
            error.to_message(),
            Message {
                key: MessageKey(key),
                args: vec![("param", MsgArg::Text("radius".into()))],
            },
            "{error:?}"
        );
    }
}

// ARQ-102
#[test]
fn arq_102_param_error_keys_are_valid_and_distinct() {
    let keys: Vec<&str> = fixtures().iter().map(|(_, k)| *k).collect();
    for (i, k) in keys.iter().enumerate() {
        assert!(is_valid_key(k), "{k}");
        assert!(!keys[..i].contains(k), "{k} repeated");
    }
    let produced: Vec<MessageKey> = fixtures().iter().map(|(e, _)| e.to_message().key).collect();
    for (i, k) in produced.iter().enumerate() {
        assert!(is_valid_key(k.0), "{k:?}");
        assert!(!produced[..i].contains(k), "{k:?} repeated");
    }
}

// ARQ-102
#[test]
fn arq_102_the_param_argument_follows_the_parameter_id() {
    let m = ParamError::OutOfRange { param: "softness" }.to_message();
    assert_eq!(m.args, vec![("param", MsgArg::Text("softness".into()))]);
}
