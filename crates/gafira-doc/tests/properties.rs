//! Property tests over generated documents (Q1 to Q5).

mod common;

use common::{Corruption, corrupted, document, json_value};
use gafira_doc::{Document, LoadError, Node, NodeKind};
use proptest::prelude::*;
use serde_json::Value;

const GOLDENS: [&str; 3] = [
    include_str!("golden/minimal.json"),
    include_str!("golden/full.json"),
    include_str!("golden/extension.json"),
];

/// Floats inside a payload match by bits; everything else by value.
fn same_payload(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) if x.is_f64() && y.is_f64() => {
            x.as_f64().map(f64::to_bits) == y.as_f64().map(f64::to_bits)
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| same_payload(a, b))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .zip(y)
                    .all(|((ka, a), (kb, b))| ka == kb && same_payload(a, b))
        }
        _ => a == b,
    }
}

fn same_node(a: &Node, b: &Node) -> bool {
    let (p, q) = (&a.base.placement, &b.base.placement);
    let floats = [
        (p.x, q.x),
        (p.y, q.y),
        (p.w, q.w),
        (p.h, q.h),
        (p.rotation_deg, q.rotation_deg),
        (p.skew, q.skew),
    ];
    let payload = match (&a.kind, &b.kind) {
        (NodeKind::Extension(x), NodeKind::Extension(y)) => same_payload(&x.payload, &y.payload),
        _ => true,
    };
    floats.iter().all(|(x, y)| x.to_bits() == y.to_bits())
        && a.base.opacity.to_bits() == b.base.opacity.to_bits()
        && payload
}

/// Equal by `==`, with every float of a node compared by bits. The debug
/// text prints the sign and the shortest round-trip digits of every other float.
fn bit_identical(a: &Document, b: &Document) -> bool {
    let nodes = |d: &Document| {
        d.pages()
            .iter()
            .flat_map(|p| p.nodes.values().cloned().collect::<Vec<_>>())
            .collect::<Vec<_>>()
    };
    let (x, y) = (nodes(a), nodes(b));
    a == b
        && format!("{a:?}") == format!("{b:?}")
        && x.len() == y.len()
        && x.iter().zip(&y).all(|(m, n)| same_node(m, n))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]

    /// Q1: saving, loading and saving again changes nothing.
    #[test]
    fn round_trip_is_idempotent_and_bit_identical(doc in document()) {
        let first = doc.to_json().expect("serializes");
        let loaded = Document::from_json(first.as_bytes()).expect("a valid document reloads");
        prop_assert_eq!(loaded.to_json().expect("serializes"), first);
        prop_assert!(bit_identical(&doc, &loaded));
    }
}

proptest! {
    /// Q2: every generated document validates.
    #[test]
    fn generated_documents_validate(doc in document()) {
        prop_assert_eq!(doc.validate(), Ok(()));
    }

    /// Q3: a corruption yields exactly its variant, and the same variant
    /// comes back through the wire when the corruption is wire-visible.
    #[test]
    fn corruption_is_detected_exactly_and_survives_the_wire(c in corrupted()) {
        let error = c.observed().expect("a corruption is detected");
        prop_assert!(c.explained_by(&error), "{:?} produced {:?}", c.kind, error);
        // The arena key and a non-finite float are not visible on the wire.
        if !matches!(c.kind, Corruption::ArenaKeyMismatch | Corruption::NonFinite) {
            match Document::from_json(c.json.as_bytes()) {
                Err(LoadError::Invalid(e)) => {
                    prop_assert!(c.explained_by(&e), "{:?} reloaded as {:?}", c.kind, e);
                }
                other => prop_assert!(false, "{:?} reloaded as {:?}", c.kind, other),
            }
        }
    }

    /// Determinism: validating twice gives the same result.
    #[test]
    fn validation_is_deterministic(c in corrupted()) {
        prop_assert_eq!(c.doc.validate(), c.doc.validate());
    }

    /// Q4: a payload keeps its canonical text through a parse.
    #[test]
    fn payload_text_is_canonical(v in json_value()) {
        let text = serde_json::to_string(&v).expect("serializes");
        let again: Value = serde_json::from_str(&text).expect("parses");
        prop_assert_eq!(serde_json::to_string(&again).expect("serializes"), text);
    }

    /// Q5: one flipped, inserted or removed byte never panics and never
    /// yields an invalid document.
    #[test]
    fn one_byte_of_damage_is_an_error_or_a_valid_document(
        golden in 0..GOLDENS.len(),
        edit in 0..3_u8,
        at in any::<prop::sample::Index>(),
        byte in any::<u8>(),
    ) {
        let mut bytes = GOLDENS[golden].as_bytes().to_vec();
        let i = at.index(bytes.len());
        match edit {
            0 => bytes[i] = byte,
            1 => bytes.insert(i, byte),
            _ => {
                bytes.remove(i);
            }
        }
        if let Ok(doc) = Document::from_json(&bytes) {
            prop_assert_eq!(doc.validate(), Ok(()));
        }
    }
}
