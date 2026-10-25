//! The memory budget: the per-entry estimate and compaction (Q7).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use gafira_doc::msg::{CommandLabel, MessageKey};
use gafira_doc::{
    DocMut, Document, EditToken, ElementBase, GroupNode, Node, NodeId, NodeKind, Page, PageId,
    PageSize, Placement,
};
use gafira_geom::{Length, Unit};

use super::*;
use crate::{DirtySet, MergeKey, Transaction};

/// How often a `Move` ran `apply` and `revert`.
#[derive(Debug, Default)]
struct Counts {
    applies: AtomicUsize,
    reverts: AtomicUsize,
    /// Makes `apply` fail while set.
    fail_apply: AtomicBool,
}

/// Moves nodes of one page to `x`; merges with another `Move` in a gesture.
#[derive(Debug)]
struct Move {
    page: PageId,
    nodes: Vec<NodeId>,
    x: f64,
    gesture: Option<GestureId>,
    previous: Vec<(NodeId, f64)>,
    fail_revert: bool,
    counts: Arc<Counts>,
}

impl Move {
    fn new(page: PageId, nodes: Vec<NodeId>, x: f64) -> Self {
        Self {
            page,
            nodes,
            x,
            gesture: None,
            previous: Vec::new(),
            fail_revert: false,
            counts: Arc::default(),
        }
    }

    fn boxed(page: PageId, nodes: Vec<NodeId>, x: f64) -> Box<dyn Command> {
        Box::new(Self::new(page, nodes, x))
    }

    fn counted(page: PageId, node: NodeId, x: f64, counts: &Arc<Counts>) -> Box<dyn Command> {
        Box::new(Self {
            counts: Arc::clone(counts),
            ..Self::new(page, vec![node], x)
        })
    }
}

impl Command for Move {
    fn label(&self) -> CommandLabel {
        MessageKey("test.move")
    }

    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        self.counts.applies.fetch_add(1, Ordering::SeqCst);
        if self.counts.fail_apply.load(Ordering::SeqCst) {
            return Err(CommandError::InvalidValue("apply"));
        }
        self.previous.clear();
        for id in &self.nodes {
            let node = doc
                .node_mut(self.page, *id)
                .ok_or(CommandError::MissingNode(*id))?;
            self.previous.push((*id, node.base.placement.x));
            node.base.placement.x = self.x;
        }
        let moved = self.previous.iter().any(|(_, old)| *old != self.x);
        Ok(if moved {
            Applied::Changed
        } else {
            Applied::Unchanged
        })
    }

    fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError> {
        self.counts.reverts.fetch_add(1, Ordering::SeqCst);
        if self.fail_revert {
            return Err(CommandError::InvalidValue("revert"));
        }
        for (id, old) in self.previous.iter().rev() {
            let node = doc
                .node_mut(self.page, *id)
                .ok_or(CommandError::MissingNode(*id))?;
            node.base.placement.x = *old;
        }
        Ok(())
    }

    fn merge_key(&self) -> Option<MergeKey> {
        Some(MergeKey {
            kind: "test.move",
            page: Some(self.page),
            nodes: Vec::new(),
            gesture: self.gesture,
        })
    }

    fn merge(&mut self, next: Box<dyn Command>) -> Result<(), Box<dyn Command>> {
        if !(&*next as &dyn std::any::Any).is::<Move>() {
            return Err(next);
        }
        let any: Box<dyn std::any::Any + Send + Sync> = next;
        let Ok(other) = any.downcast::<Move>() else {
            unreachable!("probed above")
        };
        self.nodes.extend(other.nodes);
        self.previous.extend(other.previous);
        self.x = other.x;
        Ok(())
    }

    fn touches(&self) -> DirtySet {
        DirtySet::default()
    }
}

/// One page of `count` group nodes; returns the document, page and ids.
fn document(count: usize) -> (Document, PageId, Vec<NodeId>) {
    let token = EditToken::__new_for_gafira_edit_only();
    let mut doc = Document::default();
    let mut edit = doc.edit(&token);
    let id = edit.alloc_page_id().expect("ids are available");
    let size = PageSize {
        width: Length::new(100.0, Unit::Px),
        height: Length::new(100.0, Unit::Px),
    };
    let mut page = Page::new(id, String::new(), size);
    for _ in 0..count {
        let node = edit.alloc_node_id().expect("ids are available");
        let base = ElementBase::new(node, Placement::new(0.0, 0.0, 1.0, 1.0));
        let node_kind = NodeKind::Group(GroupNode {});
        page.nodes
            .insert(node, Arc::new(Node::new(base, node_kind)));
        page.roots.push(node);
    }
    let ids = page.nodes.keys().copied().collect();
    edit.pages_mut().push(Arc::new(page));
    (doc, id, ids)
}

fn history(count: usize, budget: usize) -> (History, PageId, Vec<NodeId>) {
    let (doc, page, ids) = document(count);
    let history = History::with_budget(doc, budget).expect("the document is valid");
    (history, page, ids)
}

/// Which undo entries have dropped their snapshots, oldest first.
fn compacted(history: &History) -> Vec<bool> {
    history.undo.iter().map(|e| e.snaps.is_none()).collect()
}

/// The estimate of one entry that moves one node of a 100-node page.
fn one_entry() -> usize {
    let (mut probe, page, ids) = history(100, usize::MAX);
    probe
        .execute(Move::boxed(page, vec![ids[0]], 1.0), at(0))
        .unwrap();
    probe.estimated_bytes()
}

fn at(ms: u64) -> Duration {
    Duration::from_millis(ms)
}

#[test]
fn the_default_budget_is_512_mib() {
    let (doc, _, _) = document(1);
    assert_eq!(History::new(doc).unwrap().budget, 536_870_912);
}

#[test]
fn shared_data_is_not_double_counted() {
    let (mut whole, page, ids) = history(100, usize::MAX);
    whole
        .execute(Move::boxed(page, ids.clone(), 1.0), at(0))
        .unwrap();
    let full = whole.estimated_bytes();

    let (mut ten, page, ids) = history(100, usize::MAX);
    for (i, id) in ids.iter().take(10).enumerate() {
        ten.execute(Move::boxed(page, vec![*id], 1.0), at(i as u64 * 1_000))
            .unwrap();
    }
    assert_eq!(ten.len(), 10);
    assert!(ten.estimated_bytes() > 0);
    assert!(
        ten.estimated_bytes() < 2 * full,
        "{} vs {full}",
        ten.estimated_bytes()
    );
}

#[test]
fn undone_entries_keep_their_estimate() {
    let (mut history, page, ids) = history(10, usize::MAX);
    history
        .execute(Move::boxed(page, ids.clone(), 1.0), at(0))
        .unwrap();
    let held = history.estimated_bytes();
    assert!(held > 0);
    history.undo().unwrap();
    assert_eq!(history.estimated_bytes(), held);
    history.redo().unwrap();
    assert_eq!(history.estimated_bytes(), held);
}

#[test]
fn a_merged_entry_is_estimated_from_its_first_before_to_its_last_after() {
    let (mut merged, page, ids) = history(10, usize::MAX);
    let gesture = merged.begin_gesture();
    for id in ids.iter().take(2) {
        let mut step = Move::new(page, vec![*id], 1.0);
        step.gesture = Some(gesture);
        merged.execute(Box::new(step), at(0)).unwrap();
    }
    assert_eq!(merged.len(), 1);

    let (mut single, page, ids) = history(10, usize::MAX);
    single
        .execute(Move::boxed(page, ids[..2].to_vec(), 1.0), at(0))
        .unwrap();
    assert_eq!(merged.estimated_bytes(), single.estimated_bytes());
}

/// Adds `count` group nodes to a page.
#[derive(Debug)]
struct Grow {
    page: PageId,
    count: usize,
}

impl Command for Grow {
    fn label(&self) -> CommandLabel {
        MessageKey("test.grow")
    }

    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        for _ in 0..self.count {
            let id = doc.alloc_node_id().expect("ids are available");
            let base = ElementBase::new(id, Placement::new(0.0, 0.0, 1.0, 1.0));
            let node = Node::new(base, NodeKind::Group(GroupNode {}));
            let page = doc.page_mut(self.page).expect("the page exists");
            page.nodes.insert(id, Arc::new(node));
            page.roots.push(id);
        }
        Ok(Applied::Changed)
    }

    fn revert(&mut self, _: &mut DocMut<'_>) -> Result<(), CommandError> {
        Ok(())
    }

    fn touches(&self) -> DirtySet {
        DirtySet::default()
    }
}

#[test]
fn an_entry_counts_the_snapshot_it_replaces_not_what_the_command_added() {
    let (mut whole, page, ids) = history(100, usize::MAX);
    whole.execute(Move::boxed(page, ids, 1.0), at(0)).unwrap();

    let (mut grown, page, _) = history(0, usize::MAX);
    grown
        .execute(Box::new(Grow { page, count: 100 }), at(0))
        .unwrap();
    assert!(grown.estimated_bytes() * 10 < whole.estimated_bytes());
}

#[test]
fn a_small_budget_compacts_the_oldest_entries_first() {
    let entry = one_entry();
    let budget = 2 * entry + entry / 2;
    let (mut history, page, ids) = history(100, budget);
    for (i, id) in ids.iter().take(10).enumerate() {
        let step = Move::boxed(page, vec![*id], 1.0);
        history.execute(step, at(i as u64 * 1_000)).unwrap();
        if i == 2 {
            // Only the oldest had to go to fit; no more.
            assert_eq!(compacted(&history), [true, false, false]);
        }
    }
    assert_eq!(history.len(), 10);
    assert!(history.estimated_bytes() <= budget);
    assert_eq!(history.estimated_bytes(), 2 * entry);
    let mut expected = vec![true; 8];
    expected.extend([false; 2]);
    assert_eq!(compacted(&history), expected);
}

#[test]
fn a_sum_equal_to_the_budget_fits() {
    let entry = one_entry();
    let (mut history, page, ids) = history(100, 2 * entry);
    for (i, id) in ids.iter().take(2).enumerate() {
        let step = Move::boxed(page, vec![*id], 1.0);
        history.execute(step, at(i as u64 * 1_000)).unwrap();
    }
    assert_eq!(compacted(&history), [false, false]);
}

#[test]
fn the_newest_entry_keeps_its_snapshots_even_over_the_budget() {
    let (mut history, page, ids) = history(100, 0);
    for (i, id) in ids.iter().take(3).enumerate() {
        let step = Move::boxed(page, vec![*id], 1.0);
        history.execute(step, at(i as u64 * 1_000)).unwrap();
    }
    assert_eq!(compacted(&history), [true, true, false]);
    assert_eq!(history.estimated_bytes(), one_entry());
}

#[test]
fn undo_of_a_compacted_entry_reverts_and_redo_applies() {
    let counts = Arc::new(Counts::default());
    let (mut history, page, ids) = history(10, 0);
    let mut states = vec![(**history.document()).clone()];
    for (i, x) in [1.0, 2.0, 3.0].into_iter().enumerate() {
        let step = Move::counted(page, ids[0], x, &counts);
        history.execute(step, at(i as u64 * 1_000)).unwrap();
        states.push((**history.document()).clone());
    }
    assert_eq!(compacted(&history), [true, true, false]);

    for expected in states.iter().rev().skip(1) {
        assert!(history.undo().unwrap().is_some());
        assert_eq!(&**history.document(), expected);
    }
    assert_eq!(counts.reverts.load(Ordering::SeqCst), 2);
    for expected in states.iter().skip(1) {
        assert!(history.redo().unwrap().is_some());
        assert_eq!(&**history.document(), expected);
    }
    assert_eq!(counts.applies.load(Ordering::SeqCst), 3 + 2);
    assert_eq!(counts.reverts.load(Ordering::SeqCst), 2);
}

#[test]
fn a_failed_revert_changes_nothing() {
    let (mut history, page, ids) = history(10, 0);
    let mut failing = Move::new(page, vec![ids[0]], 1.0);
    failing.fail_revert = true;
    history.execute(Box::new(failing), at(0)).unwrap();
    history
        .execute(Move::boxed(page, vec![ids[1]], 1.0), at(1_000))
        .unwrap();
    history.undo().unwrap();
    let before = Arc::clone(history.document());

    assert_eq!(history.undo(), Err(CommandError::InvalidValue("revert")));
    assert!(Arc::ptr_eq(&before, history.document()));
    assert_eq!((history.len(), history.can_redo()), (1, true));
    assert_eq!(history.undo(), Err(CommandError::InvalidValue("revert")));
}

#[test]
fn a_transaction_that_fails_midway_through_revert_leaves_no_partial_change() {
    let (mut history, page, ids) = history(10, 0);
    let mut failing = Move::new(page, vec![ids[0]], 1.0);
    failing.fail_revert = true;
    let pair: Vec<Box<dyn Command>> = vec![Box::new(failing), Move::boxed(page, vec![ids[1]], 1.0)];
    let both = Transaction::new(MessageKey("test.pair"), pair);
    history.execute(Box::new(both), at(0)).unwrap();
    history
        .execute(Move::boxed(page, vec![ids[2]], 1.0), at(1_000))
        .unwrap();
    history.undo().unwrap();
    let before = Arc::clone(history.document());

    // The second child reverts on the copy before the first one fails.
    assert!(history.undo().is_err());
    assert!(Arc::ptr_eq(&before, history.document()));
    let moved = history.document().page(page).and_then(|p| p.node(ids[1]));
    assert_eq!(moved.map(|n| n.base.placement.x), Some(1.0));
}

#[test]
fn a_failed_apply_on_a_compacted_redo_changes_nothing() {
    let counts = Arc::new(Counts::default());
    let (mut history, page, ids) = history(10, 0);
    history
        .execute(Move::counted(page, ids[0], 1.0, &counts), at(0))
        .unwrap();
    history
        .execute(Move::boxed(page, vec![ids[1]], 1.0), at(1_000))
        .unwrap();
    history.undo().unwrap();
    history.undo().unwrap();
    let before = Arc::clone(history.document());

    counts.fail_apply.store(true, Ordering::SeqCst);
    assert_eq!(history.redo(), Err(CommandError::InvalidValue("apply")));
    assert!(Arc::ptr_eq(&before, history.document()));
    assert_eq!((history.len(), history.can_redo()), (0, true));

    // The entry is still there: both can be redone once apply works again.
    counts.fail_apply.store(false, Ordering::SeqCst);
    assert!(history.redo().unwrap().is_some());
    assert!(history.redo().unwrap().is_some());
    assert_eq!((history.len(), history.can_redo()), (2, false));
}

#[test]
fn redo_of_a_compacted_entry_succeeds_when_apply_reports_unchanged() {
    let (mut history, page, ids) = history(10, 0);
    // A drag that ends where it started: one entry whose before equals its after.
    let gesture = history.begin_gesture();
    for x in [1.0, 0.0] {
        let mut step = Move::new(page, vec![ids[0]], x);
        step.gesture = Some(gesture);
        history.execute(Box::new(step), at(0)).unwrap();
    }
    history.end_gesture();
    history
        .execute(Move::boxed(page, vec![ids[1]], 1.0), at(1_000))
        .unwrap();
    assert_eq!(compacted(&history), [true, false]);

    history.undo().unwrap();
    history.undo().unwrap();
    assert_eq!(history.len(), 0);
    assert_eq!(history.redo(), Ok(Some(MessageKey("test.move"))));
    assert_eq!((history.len(), history.can_redo()), (1, true));
    assert!(history.redo().unwrap().is_some());
    assert_eq!(history.len(), 2);
}

#[test]
fn an_entry_that_grows_by_merging_can_push_older_ones_out() {
    let entry = one_entry();
    let (mut history, page, ids) = history(100, 2 * entry);
    history
        .execute(Move::boxed(page, vec![ids[0]], 1.0), at(0))
        .unwrap();
    let gesture = history.begin_gesture();
    for id in &ids[1..40] {
        let mut step = Move::new(page, vec![*id], 1.0);
        step.gesture = Some(gesture);
        history.execute(Box::new(step), at(1_000)).unwrap();
    }
    assert_eq!(history.len(), 2);
    assert_eq!(compacted(&history), [true, false]);
}
