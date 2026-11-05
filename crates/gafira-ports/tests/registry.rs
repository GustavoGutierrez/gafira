//! `Registry<T>` (design P8): built by value, ids unique, registration order.

use std::sync::Arc;

use gafira_doc::contract::{Describe, Descriptor};
use gafira_doc::msg::{MessageKey, MsgArg, ToMessage, is_valid_key};
use gafira_ports::registry::{Registry, RegistryError};

trait Widget: Describe + Send + Sync {
    fn tag(&self) -> &'static str;
}

struct Item {
    descriptor: Descriptor,
    tag: &'static str,
}

impl Describe for Item {
    fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }
}

impl Widget for Item {
    fn tag(&self) -> &'static str {
        self.tag
    }
}

fn item(id: &'static str, tag: &'static str) -> Arc<dyn Widget> {
    Arc::new(Item {
        descriptor: Descriptor::new(id, MessageKey("widget.name"), 1),
        tag,
    })
}

fn ids(reg: &Registry<dyn Widget>) -> Vec<&'static str> {
    reg.iter().map(|w| w.descriptor().id).collect()
}

// ARQ-31: a registry is built by value, and needs no `Describe` to be empty.
#[test]
fn arq_31_new_registry_is_empty_without_describe() {
    struct Plain;
    let reg: Registry<Plain> = Registry::new();
    assert!(reg.is_empty());
    assert_eq!(reg.len(), 0);
    let dflt: Registry<dyn Widget> = Registry::default();
    assert!(dflt.is_empty());
}

// ARQ-31: two registries are independent values.
#[test]
fn arq_31_registries_do_not_share_state() {
    let mut a: Registry<dyn Widget> = Registry::new();
    let b: Registry<dyn Widget> = Registry::new();
    a.register(item("one", "x")).unwrap();
    assert_eq!(a.len(), 1);
    assert!(b.is_empty());
}

// P8: iteration is in registration order, not in id order.
#[test]
fn p8_iter_is_in_registration_order() {
    let mut reg: Registry<dyn Widget> = Registry::new();
    for id in ["zeta", "alpha", "mid"] {
        reg.register(item(id, "x")).unwrap();
    }
    assert_eq!(ids(&reg), ["zeta", "alpha", "mid"]);
    assert_eq!(reg.len(), 3);
    assert!(!reg.is_empty());
}

// P8: get finds the registered item by id and nothing else.
#[test]
fn p8_get_by_id_and_unknown_id() {
    let mut reg: Registry<dyn Widget> = Registry::new();
    reg.register(item("first", "a")).unwrap();
    reg.register(item("second", "b")).unwrap();
    assert_eq!(reg.get("second").map(|w| w.tag()), Some("b"));
    assert_eq!(reg.get("first").map(|w| w.tag()), Some("a"));
    assert!(reg.get("third").is_none());
    assert!(Registry::<dyn Widget>::new().get("first").is_none());
}

// P8: a repeated id is an error and leaves the registry as it was.
#[test]
fn p8_duplicate_id_is_rejected_and_changes_nothing() {
    let mut reg: Registry<dyn Widget> = Registry::new();
    reg.register(item("a", "old")).unwrap();
    reg.register(item("b", "b")).unwrap();
    let err = reg.register(item("a", "new")).unwrap_err();
    assert_eq!(err, RegistryError::DuplicateId { id: "a" });
    assert_eq!(reg.len(), 2);
    assert_eq!(ids(&reg), ["a", "b"]);
    assert_eq!(reg.get("a").map(|w| w.tag()), Some("old"));
    // The registry still accepts new ids afterwards, and get stays aligned.
    reg.register(item("c", "c")).unwrap();
    assert_eq!(reg.get("c").map(|w| w.tag()), Some("c"));
}

// ARQ-102 (ADR 0012): the error is a key with named arguments.
#[test]
fn arq_102_registry_error_message() {
    let m = RegistryError::DuplicateId { id: "png" }.to_message();
    assert!(is_valid_key(m.key.0));
    assert_eq!(m.key, MessageKey("error.registry.duplicate_id"));
    assert_eq!(m.args, vec![("id", MsgArg::Text("png".into()))]);
}
