//! The memory budget: the per-entry estimate and compaction (Q7).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use gafira_doc::msg::{CommandLabel, MessageKey};
use gafira_doc::{
    DocMut, Document, EditToken, ElementBase, GroupNode, Node, NodeId, NodeKind, Page, PageId,
    PageSize, Placement,
};
use gafira_geom::{Length, Unit};

use super::*;
use crate::{DirtySet, MergeKey};

/// How often a `Move` ran `apply` and `revert`.
#[derive(Debug, Default)]
struct Counts {
    applies: AtomicUsize,
    reverts: AtomicUsize,
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
}

impl Command for Move {
    fn label(&self) -> CommandLabel {
        MessageKey("test.move")
    }

    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        self.counts.applies.fetch_add(1, Ordering::SeqCst);
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

fn history(count: usize, _budget: usize) -> (History, PageId, Vec<NodeId>) {
    let (doc, page, ids) = document(count);
    (History::new(doc).expect("the document is valid"), page, ids)
}

fn at(ms: u64) -> Duration {
    Duration::from_millis(ms)
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
