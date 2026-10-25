//! Gesture merge with the real `SetProperty` command (ARQ-33).

use std::sync::Arc;
use std::time::Duration;

use gafira_doc::{Document, NodeId, PageId, Placement};
use gafira_edit::{Applied, Command, History, Property, SetProperty};
use gafira_testkit::document;
use proptest::prelude::*;

/// The first node of the first page that has one.
fn first_node(doc: &Document) -> Option<(PageId, NodeId)> {
    doc.pages()
        .iter()
        .find_map(|p| p.nodes.keys().next().map(|n| (p.id, *n)))
}

fn placement_of(doc: &Document, page: PageId, node: NodeId) -> Placement {
    doc.page(page)
        .and_then(|p| p.node(node))
        .map(|n| n.base.placement)
        .expect("the node exists")
}

fn at(index: usize, step_ms: u64) -> Duration {
    Duration::from_millis(index as u64 * step_ms)
}

fn drag_step(
    page: PageId,
    node: NodeId,
    x: f64,
    gesture: gafira_edit::GestureId,
) -> Box<dyn Command> {
    let value = Property::Placement(Placement::new(x, 0.0, 10.0, 10.0));
    Box::new(SetProperty::new(page, vec![node], value, Some(gesture)))
}

proptest! {
    #![proptest_config(ProptestConfig { failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn a_300_move_drag_is_one_entry_that_undoes_to_the_origin(
        doc in document().prop_filter("needs a node", |d| first_node(d).is_some()),
    ) {
        let (page, node) = first_node(&doc).unwrap();
        let origin = placement_of(&doc, page, node);
        let mut history = History::new(doc).unwrap();
        let start = Arc::clone(history.document());

        let gesture = history.begin_gesture();
        for i in 0..300_usize {
            // One second apart: only the gesture can merge them.
            let step = drag_step(page, node, 2.0e6 + i as f64, gesture);
            prop_assert_eq!(history.execute(step, at(i, 1_000)), Ok(Applied::Changed));
        }
        history.end_gesture();
        let last = Arc::clone(history.document());

        prop_assert_eq!(history.len(), 1);
        prop_assert_eq!(placement_of(history.document(), page, node).x, 2.0e6 + 299.0);
        prop_assert!(history.undo().unwrap().is_some());
        prop_assert!(Arc::ptr_eq(&start, history.document()));
        prop_assert_eq!(placement_of(history.document(), page, node), origin);
        prop_assert!(history.redo().unwrap().is_some());
        prop_assert!(Arc::ptr_eq(&last, history.document()));
    }

    #[test]
    fn a_drag_copies_only_the_dragged_node(
        doc in document().prop_filter("needs a node", |d| first_node(d).is_some()),
    ) {
        let (page, node) = first_node(&doc).unwrap();
        let mut history = History::new(doc).unwrap();
        let start = Arc::clone(history.document());
        let gesture = history.begin_gesture();
        for i in 0..300_usize {
            history.execute(drag_step(page, node, i as f64, gesture), at(i, 1)).unwrap();
        }
        for before in start.pages() {
            let after = history.document().page(before.id).expect("the page is kept");
            for (id, kept) in &before.nodes {
                let same = Arc::ptr_eq(kept, &after.nodes[id]);
                prop_assert_eq!(same, !(before.id == page && *id == node));
            }
        }
    }

    #[test]
    fn merging_equals_applying_the_sequence_one_by_one(
        doc in document().prop_filter("needs a node", |d| first_node(d).is_some()),
        xs in prop::collection::vec(-1.0e6..1.0e6_f64, 1..20),
    ) {
        let (page, node) = first_node(&doc).unwrap();
        let mut history = History::new(doc.clone()).unwrap();
        let mut sequential = History::new(doc.clone()).unwrap();
        let gesture = history.begin_gesture();
        for (i, x) in xs.iter().enumerate() {
            history.execute(drag_step(page, node, *x, gesture), at(i, 1)).unwrap();
            // The control never merges: a closed key per command.
            let other = sequential.begin_gesture();
            sequential.execute(drag_step(page, node, *x, other), at(i, 1_000)).unwrap();
            sequential.end_gesture();
        }
        history.end_gesture();

        prop_assert_eq!(&**history.document(), &**sequential.document());
        prop_assert!(history.len() <= 1);
        while history.undo().unwrap().is_some() {}
        prop_assert_eq!(&**history.document(), &doc);
    }
}
