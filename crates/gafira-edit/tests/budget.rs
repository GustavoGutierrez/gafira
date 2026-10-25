//! A compacted history reaches the same states as one that keeps every
//! snapshot (ADR-03, Q7).

use std::time::Duration;

use gafira_doc::{
    Document, GroupNode, NodeId, NodeKind, PageId, Placement, ShapeGeometry, ShapeNode,
};
use gafira_edit::{
    Applied, Command, History, InsertNodes, NodeSpec, Property, RemoveNodes, SetProperty,
};
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

/// One recorded step: a property change, a root insert or a removal. The
/// numbers pick a target among those the document has at that moment.
#[derive(Clone, Debug)]
enum Step {
    Set(usize, Property),
    Insert {
        page: usize,
        index: usize,
        group: bool,
    },
    Remove(usize),
}

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        (any::<usize>(), property()).prop_map(|(pick, value)| Step::Set(pick, value)),
        (any::<usize>(), any::<usize>(), any::<bool>())
            .prop_map(|(page, index, group)| Step::Insert { page, index, group }),
        any::<usize>().prop_map(Step::Remove),
    ]
}

fn at() -> Placement {
    Placement::new(0.0, 0.0, 10.0, 10.0)
}

/// The command a step stands for on `doc`, or `None` when it has no target.
fn command(doc: &Document, step: &Step) -> Option<Box<dyn Command>> {
    let pairs = targets(doc);
    let shape = || NodeSpec::new(NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect)), at());
    Some(match step {
        Step::Set(pick, value) => {
            let (page, node) = *pairs.get(pick.checked_rem(pairs.len())?)?;
            Box::new(SetProperty::new(page, vec![node], value.clone(), None))
        }
        Step::Insert { page, index, group } => {
            let target = doc.pages().get(page.checked_rem(doc.pages().len())?)?;
            let spec = if *group {
                NodeSpec::new(NodeKind::Group(GroupNode {}), at()).with_children(vec![shape()])
            } else {
                shape()
            };
            let index = index % (target.roots.len() + 1);
            Box::new(InsertNodes::new(target.id, None, index, vec![spec]))
        }
        Step::Remove(pick) => {
            let (page, node) = *pairs.get(pick.checked_rem(pairs.len())?)?;
            Box::new(RemoveNodes::new(page, vec![node]))
        }
    })
}

/// Runs `steps` one second apart (so none merge) and returns the history
/// and the document after every recorded step, the start included.
fn run(doc: &Document, budget: usize, steps: &[Step]) -> (History, Vec<Document>) {
    let mut history = History::with_budget(doc.clone(), budget).unwrap();
    let mut states = vec![doc.clone()];
    for (i, step) in steps.iter().enumerate() {
        let Some(command) = command(history.document(), step) else {
            continue;
        };
        let at = Duration::from_secs(i as u64);
        if history.execute(command, at) == Ok(Applied::Changed) {
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
        steps in prop::collection::vec(step(), 1..12),
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
