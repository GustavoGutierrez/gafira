//! A compacted history reaches the same states as one that keeps every
//! snapshot (ADR-03, Q7).

use std::time::Duration;

use gafira_doc::{Document, NodeId, PageId, Placement};
use gafira_edit::{Applied, History, Property, SetProperty};
use gafira_testkit::document;
use proptest::prelude::*;

/// Every `(page, node)` pair of the document.
fn targets(doc: &Document) -> Vec<(PageId, NodeId)> {
    doc.pages()
        .iter()
        .flat_map(|p| p.nodes.keys().map(|n| (p.id, *n)))
        .collect()
}

fn property() -> impl Strategy<Value = Property> {
    prop_oneof![
        (-1.0e3..1.0e3_f64, 0.0..100.0_f64)
            .prop_map(|(x, w)| Property::Placement(Placement::new(x, x, w, w))),
        any::<bool>().prop_map(Property::Visible),
        (0.0..=1.0_f32).prop_map(Property::Opacity),
        prop_oneof![Just(None), "[a-z]{0,6}".prop_map(Some)].prop_map(Property::Name),
    ]
}

/// Runs `steps` one second apart (so none merge) and returns the history
/// and the document after every recorded step, the start included.
fn run(doc: &Document, budget: usize, steps: &[(usize, Property)]) -> (History, Vec<Document>) {
    let pairs = targets(doc);
    let mut history = History::with_budget(doc.clone(), budget).unwrap();
    let mut states = vec![doc.clone()];
    for (i, (pick, value)) in steps.iter().enumerate() {
        let (page, node) = pairs[pick % pairs.len()];
        let command = SetProperty::new(page, vec![node], value.clone(), None);
        let at = Duration::from_secs(i as u64);
        if history.execute(Box::new(command), at) == Ok(Applied::Changed) {
            states.push((**history.document()).clone());
        }
    }
    (history, states)
}

proptest! {
    #![proptest_config(ProptestConfig { failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn undo_and_redo_through_revert_reach_the_same_states(
        doc in document().prop_filter("needs a node", |d| !targets(d).is_empty()),
        steps in prop::collection::vec((any::<usize>(), property()), 1..12),
    ) {
        let (kept, recorded) = run(&doc, usize::MAX, &steps);
        let (mut squeezed, states) = run(&doc, 0, &steps);
        prop_assert_eq!(&states, &recorded);
        prop_assert_eq!(squeezed.len(), kept.len());
        if squeezed.len() > 1 {
            prop_assert!(squeezed.estimated_bytes() < kept.estimated_bytes());
        }

        for expected in recorded.iter().rev().skip(1) {
            prop_assert!(squeezed.undo().unwrap().is_some());
            prop_assert_eq!(&**squeezed.document(), expected);
        }
        prop_assert!(squeezed.undo().unwrap().is_none());
        for expected in recorded.iter().skip(1) {
            prop_assert!(squeezed.redo().unwrap().is_some());
            prop_assert_eq!(&**squeezed.document(), expected);
        }
    }
}
