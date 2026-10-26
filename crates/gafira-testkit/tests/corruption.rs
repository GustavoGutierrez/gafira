//! Oracles for the corruption generators: the guards reject a foreign error,
//! and `corrupt` sub-cases follow the pick table.
//!
//! Documents come from a deterministic runner with a fixed number of draws, so
//! the verdict does not depend on `PROPTEST_CASES`.

use gafira_doc::{Document, Node, NodeField, NodeKind, ValidationError};
use gafira_testkit::{Corruption, corrupt, document};
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;

/// `corrupted()` draws its pick from `0..1000`.
const PICKS: std::ops::Range<usize> = 0..1000;

/// The empty document first, then deterministic generated ones.
fn docs(generated: usize) -> Vec<Document> {
    let strategy = document();
    let mut runner = TestRunner::deterministic();
    let mut docs = vec![Document::default()];
    docs.extend((0..generated).map(|_| strategy.new_tree(&mut runner).expect("tree").current()));
    docs
}

/// The class of error each corruption must produce, stated apart from
/// `Corruption::matches`.
fn expected_class(kind: Corruption) -> &'static str {
    use Corruption as C;
    match kind {
        C::DuplicateIdAcrossPages | C::DuplicateIdPageAndNode => "duplicate id",
        C::ArenaKeyMismatch => "arena key",
        C::NextIdNotAboveMax => "next id",
        C::PageSizeBelow | C::PageSizeAbove => "page size",
        C::DanglingRoot => "dangling root",
        C::DuplicateRoot => "duplicate root",
        C::NonFinite => "non-finite",
        C::OpacityRange => "range opacity",
        C::AlphaRange => "range fill alpha",
        C::SkewRange => "range skew",
        C::WidthRange => "range w",
        C::HeightRange => "range h",
        C::EmptyTypeId => "empty type id",
        C::ExtensionTooDeep => "too deep",
        C::ChildrenOnLeaf => "children on leaf",
        C::DanglingChild => "dangling child",
        C::MultipleParents => "multiple parents",
        C::RootHasParent => "root has parent",
        C::Orphan => "orphan",
        C::Cycle => "cycle",
        C::InvalidDpi => "invalid dpi",
        C::UnsupportedSchema => "schema",
    }
}

fn class_of(error: &ValidationError) -> String {
    use ValidationError as E;
    let range = |field: &NodeField| match field {
        NodeField::Opacity => "range opacity",
        NodeField::FillAlpha => "range fill alpha",
        NodeField::Skew => "range skew",
        NodeField::W => "range w",
        NodeField::H => "range h",
        _ => "range other",
    };
    match error {
        E::DuplicateId { .. } => "duplicate id",
        E::ArenaKeyMismatch { .. } => "arena key",
        E::NextIdNotAboveMax { .. } => "next id",
        E::PageSize { .. } => "page size",
        E::DanglingRoot { .. } => "dangling root",
        E::DuplicateRoot { .. } => "duplicate root",
        E::NonFinite { .. } => "non-finite",
        E::OutOfRange { field, .. } => range(field),
        E::EmptyExtensionTypeId { .. } => "empty type id",
        E::ExtensionTooDeep { .. } => "too deep",
        E::ChildrenOnLeaf { .. } => "children on leaf",
        E::DanglingChild { .. } => "dangling child",
        E::MultipleParents { .. } => "multiple parents",
        E::RootHasParent { .. } => "root has parent",
        E::Orphan { .. } => "orphan",
        E::Cycle { .. } => "cycle",
        E::InvalidDpi { .. } => "invalid dpi",
        E::UnsupportedSchemaVersion { .. } => "schema",
        _ => "unknown",
    }
    .to_string()
}

#[test]
fn a_guard_rejects_the_error_of_every_other_class() {
    let mut rejected = 0;
    for doc in docs(4) {
        for pick in [0, 1, 7, 500] {
            let built: Vec<_> = Corruption::ALL
                .iter()
                .map(|k| corrupt(doc.clone(), *k, pick))
                .collect();
            for guard in &built {
                for other in &built {
                    let error = other.observed().expect("a corruption is detected");
                    let same_class = expected_class(guard.kind) == class_of(&error);
                    let same_id = !matches!(error, ValidationError::DuplicateId { .. })
                        || error
                            == ValidationError::DuplicateId {
                                id: guard.duplicate.unwrap_or(0),
                            };
                    let want = same_class && same_id;
                    assert_eq!(
                        guard.kind.matches(&error, guard.duplicate),
                        want,
                        "{:?} against the error of {:?}: {error:?}",
                        guard.kind,
                        other.kind
                    );
                    assert_eq!(guard.explained_by(&error), want);
                    rejected += usize::from(!want);
                }
            }
        }
    }
    assert!(rejected > 0, "no foreign error was ever offered");
}

#[test]
fn page_size_kinds_share_an_error_and_duplicate_id_kinds_share_it_by_id() {
    let doc = Document::default();
    let below = corrupt(doc.clone(), Corruption::PageSizeBelow, 0);
    let above = corrupt(doc.clone(), Corruption::PageSizeAbove, 0);
    let error = above.observed().expect("detected");
    assert!(below.explained_by(&error));
    assert!(above.explained_by(&below.observed().expect("detected")));

    let across = corrupt(doc.clone(), Corruption::DuplicateIdAcrossPages, 0);
    let page_node = corrupt(doc, Corruption::DuplicateIdPageAndNode, 0);
    let id = across.duplicate.expect("duplicate id");
    let same = ValidationError::DuplicateId { id };
    let other = ValidationError::DuplicateId { id: id + 1 };
    for kind in [&across, &page_node] {
        assert_eq!(kind.explained_by(&same), kind.duplicate == Some(id));
        assert!(!kind.explained_by(&other));
    }
    assert!(across.explained_by(&same));
}

/// The nodes of the corrupted document whose id is at least `from`.
fn nodes_from(doc: &Document, from: u64) -> Vec<&Node> {
    doc.pages()
        .iter()
        .flat_map(|p| p.nodes.values())
        .map(|n| &**n)
        .filter(|n| n.base.id.get() >= from)
        .collect()
}

#[test]
fn next_id_offset_follows_pick_modulo_3() {
    for doc in docs(2) {
        for pick in PICKS {
            let c = corrupt(doc.clone(), Corruption::NextIdNotAboveMax, pick);
            let injected = nodes_from(&c.doc, c.doc.next_id());
            assert_eq!(injected.len(), 1, "pick {pick}");
            let offset = injected[0].base.id.get() - c.doc.next_id();
            assert_eq!(offset, (pick % 3) as u64, "pick {pick}");
        }
    }
}

#[test]
fn page_size_value_and_side_follow_the_pick_table() {
    let cases = [
        (Corruption::PageSizeBelow, [0.5, 0.0, 0.999]),
        (Corruption::PageSizeAbove, [16_384.5, 20_000.0, 1e9]),
    ];
    for doc in docs(2) {
        for (kind, values) in cases {
            for pick in PICKS {
                let c = corrupt(doc.clone(), kind, pick);
                let page = c.doc.pages().last().expect("the added page");
                let (w, h) = (page.size.width.value(), page.size.height.value());
                let bad = values[pick % 3];
                let want = if (pick / 3) % 2 == 0 {
                    (bad, 100.0)
                } else {
                    (100.0, bad)
                };
                assert_eq!((w, h), want, "{kind:?} pick {pick}");
            }
        }
    }
}

fn class_of_value(v: f64) -> &'static str {
    if v.is_nan() {
        "nan"
    } else if v == f64::INFINITY {
        "+inf"
    } else if v == f64::NEG_INFINITY {
        "-inf"
    } else {
        "finite"
    }
}

#[test]
fn non_finite_field_and_value_follow_the_pick_table() {
    let fields = [
        NodeField::X,
        NodeField::Y,
        NodeField::W,
        NodeField::H,
        NodeField::RotationDeg,
        NodeField::Skew,
        NodeField::Opacity,
    ];
    let values = ["nan", "+inf", "-inf"];
    for doc in docs(2) {
        let mut pairs = std::collections::BTreeSet::new();
        for pick in PICKS {
            let c = corrupt(doc.clone(), Corruption::NonFinite, pick);
            let Some(ValidationError::NonFinite { node, field }) = c.observed() else {
                panic!("pick {pick}: not a NonFinite error");
            };
            let want_field = fields[pick % 7];
            assert_eq!(field, want_field, "pick {pick}");
            let node = nodes_from(&c.doc, node.get())
                .into_iter()
                .find(|n| n.base.id == node)
                .expect("the injected node");
            let p = &node.base.placement;
            let observed = [
                p.x,
                p.y,
                p.w,
                p.h,
                p.rotation_deg,
                p.skew,
                f64::from(node.base.opacity),
            ];
            for (i, v) in observed.iter().enumerate() {
                let class = class_of_value(*v);
                if i == pick % 7 {
                    assert_eq!(class, values[(pick / 7) % 3], "pick {pick}");
                } else {
                    assert_eq!(
                        class, "finite",
                        "pick {pick}: field {i} is not the corrupted one"
                    );
                }
            }
            pairs.insert((pick % 7, (pick / 7) % 3));
        }
        assert_eq!(pairs.len(), 21, "every field and value pair appears");
    }
}

#[test]
fn skew_values_follow_the_pick_table() {
    let skews = [90.0, -90.0, 91.0, 1e3, -1e3];
    for doc in docs(2) {
        for pick in PICKS {
            let c = corrupt(doc.clone(), Corruption::SkewRange, pick);
            let Some(ValidationError::OutOfRange { node, field, .. }) = c.observed() else {
                panic!("pick {pick}: not an OutOfRange error");
            };
            assert_eq!(field, NodeField::Skew);
            let found = nodes_from(&c.doc, 0)
                .into_iter()
                .find(|n| n.base.id == node)
                .expect("the injected node");
            assert_eq!(found.base.placement.skew, skews[pick % 5], "pick {pick}");
        }
    }
}

/// Containers nested in a payload built as `[[...[null]...]]`.
fn container_depth(v: &serde_json::Value) -> usize {
    match v {
        serde_json::Value::Array(a) => 1 + a.iter().map(container_depth).max().unwrap_or(0),
        serde_json::Value::Object(o) => 1 + o.values().map(container_depth).max().unwrap_or(0),
        _ => 0,
    }
}

#[test]
fn extension_payload_depth_follows_pick_modulo_3() {
    for doc in docs(2) {
        for pick in PICKS {
            let c = corrupt(doc.clone(), Corruption::ExtensionTooDeep, pick);
            let Some(ValidationError::ExtensionTooDeep { node, .. }) = c.observed() else {
                panic!("pick {pick}: not an ExtensionTooDeep error");
            };
            let found = nodes_from(&c.doc, 0)
                .into_iter()
                .find(|n| n.base.id == node)
                .expect("the injected node");
            let NodeKind::Extension(ext) = &found.kind else {
                panic!("pick {pick}: not an extension node");
            };
            assert_eq!(container_depth(&ext.payload), 65 + pick % 3, "pick {pick}");
        }
    }
}
