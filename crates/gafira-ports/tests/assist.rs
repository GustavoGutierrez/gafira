//! `TextAssistant`, `TextAssistantRegistry` and `AssistError` (design P4, P6, P8).

use std::sync::Arc;

use gafira_doc::contract::{CancelToken, Cancelled, Describe, Descriptor, NoProgress, Progress};
use gafira_doc::msg::{MessageKey, ToMessage, is_valid_key};
use gafira_ports::assist::{
    AssistError, AssistReq, RewriteRequest, Suggestion, TextAssistant, TextAssistantRegistry,
};

struct Upper {
    descriptor: Descriptor,
}

impl Upper {
    fn arc() -> Arc<dyn TextAssistant> {
        Arc::new(Self {
            descriptor: Descriptor::new("test.upper", MessageKey("assist.test.upper"), 1),
        })
    }
}

impl Describe for Upper {
    fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }
}

impl TextAssistant for Upper {
    fn requirements(&self) -> AssistReq {
        AssistReq {
            ram_bytes: 1 << 30,
            ..AssistReq::default()
        }
    }
    fn rewrite(
        &self,
        text: &str,
        _req: &RewriteRequest,
        progress: &dyn Progress,
        cancel: &CancelToken,
    ) -> Result<Vec<Suggestion>, AssistError> {
        cancel.check().map_err(AssistError::Cancelled)?;
        progress.report(1.0);
        Ok(vec![Suggestion {
            text: text.to_uppercase(),
        }])
    }
}

// ARQ-26: the MVP registry is empty; a test double registers and runs through `dyn`.
#[test]
fn arq_26_default_registry_is_empty_and_a_double_registers() {
    let mut registry = TextAssistantRegistry::default();
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
    assert!(registry.get("test.upper").is_none());

    registry.register(Upper::arc()).unwrap();
    assert_eq!(registry.len(), 1);
    let assistant = registry.get("test.upper").unwrap();
    assert_eq!(assistant.requirements().ram_bytes, 1 << 30);
    let out = assistant
        .rewrite(
            "hi",
            &RewriteRequest::default(),
            &NoProgress,
            &CancelToken::new(),
        )
        .unwrap();
    assert_eq!(
        out,
        vec![Suggestion {
            text: "HI".to_string()
        }]
    );
    assert!(registry.register(Upper::arc()).is_err());
}

// FD-64: the trait lives here and honours the cancel token.
#[test]
fn fd_64_rewrite_reports_cancellation() {
    let token = CancelToken::new();
    token.cancel();
    let err = Upper::arc()
        .rewrite("x", &RewriteRequest::default(), &NoProgress, &token)
        .unwrap_err();
    assert_eq!(err, AssistError::Cancelled(Cancelled));
}

// ARQ-102 (ADR 0012): one fixture per variant; `Cancelled` delegates.
#[test]
fn arq_102_assist_error_messages() {
    let missing = AssistError::ModelMissing.to_message();
    assert!(is_valid_key(missing.key.0), "{:?}", missing.key);
    assert_eq!(missing.key, MessageKey("error.assist.model_missing"));
    assert!(missing.args.is_empty());

    let cancelled = AssistError::Cancelled(Cancelled).to_message();
    assert_eq!(cancelled, Cancelled.to_message());
}
