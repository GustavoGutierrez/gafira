//! `InsertNodes`: new node subtrees into one page (ARQ-81, ADR-03).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use gafira_doc::msg::{CommandLabel, MessageKey};
use gafira_doc::{
    BlendMode, DocMut, Document, ElementBase, Node, NodeId, NodeKind, Page, PageId, Placement,
};

use crate::{Applied, Command, CommandError, DirtySet};

/// A node to insert, described without ids; ids are allocated on `apply`.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeSpec {
    pub name: Option<String>,
    pub placement: Placement,
    pub opacity: f32,
    pub blend: BlendMode,
    pub visible: bool,
    pub locked: bool,
    pub kind: NodeKind,
    /// Subtrees below this node; only a `Group` may have them.
    pub children: Vec<NodeSpec>,
}

impl NodeSpec {
    /// An opaque, visible, unlocked, unnamed node without children.
    pub fn new(kind: NodeKind, placement: Placement) -> Self {
        Self {
            name: None,
            placement,
            opacity: 1.0,
            blend: BlendMode::Normal,
            visible: true,
            locked: false,
            kind,
            children: Vec::new(),
        }
    }

    /// The same node with these subtrees below it.
    #[must_use]
    pub fn with_children(mut self, children: Vec<NodeSpec>) -> Self {
        self.children = children;
        self
    }
}

/// What one successful `apply` did, for `revert` and `touches`.
#[derive(Clone, Debug)]
struct Record {
    /// The id counter before the ids were allocated.
    counter: u64,
    /// Every new id, parents before children.
    ids: Vec<NodeId>,
    /// The new ids of the inserted subtree roots, in order.
    tops: Vec<NodeId>,
}

/// Inserts node subtrees as page roots or as children of a `Group`.
///
/// Every `apply` allocates its ids from the counter and records them. After
/// a `revert` the counter is back where it was, so a redo gets the same ids
/// and rebuilds an identical document.
#[derive(Clone, Debug)]
pub struct InsertNodes {
    page: PageId,
    parent: Option<NodeId>,
    index: usize,
    specs: Vec<NodeSpec>,
    record: Option<Record>,
}

impl InsertNodes {
    /// Inserts `specs`, in order, at `index` of the page roots (`parent` is
    /// `None`) or of the children of `parent`.
    pub fn new(page: PageId, parent: Option<NodeId>, index: usize, specs: Vec<NodeSpec>) -> Self {
        Self {
            page,
            parent,
            index,
            specs,
            record: None,
        }
    }

    /// Checks the page, the parent and the index.
    fn check_target(&self, doc: &Document) -> Result<(), CommandError> {
        let page = doc
            .page(self.page)
            .ok_or(CommandError::MissingPage(self.page))?;
        let len = match self.parent {
            None => page.roots.len(),
            Some(id) => {
                let parent = page.node(id).ok_or(CommandError::MissingNode(id))?;
                if !matches!(parent.kind, NodeKind::Group(_)) {
                    return Err(CommandError::NotAContainer(id));
                }
                parent.children.len()
            }
        };
        if self.index > len {
            return Err(CommandError::IndexOutOfRange {
                index: self.index,
                len,
            });
        }
        Ok(())
    }

    /// Builds the nodes with ids `first`, `first + 1`, ... in preorder, and
    /// the ids of the subtree roots.
    fn build(&self, first: u64) -> Result<(Vec<Node>, Vec<NodeId>), CommandError> {
        let mut nodes: Vec<Node> = Vec::new();
        let mut tops = Vec::new();
        let mut stack: Vec<(&NodeSpec, Option<usize>)> =
            self.specs.iter().rev().map(|spec| (spec, None)).collect();
        while let Some((spec, parent)) = stack.pop() {
            if !spec.children.is_empty() && !matches!(spec.kind, NodeKind::Group(_)) {
                return Err(CommandError::InvalidValue(
                    "children on a node that is not a group",
                ));
            }
            let id = u64::try_from(nodes.len())
                .ok()
                .and_then(|offset| first.checked_add(offset))
                .filter(|id| *id < u64::MAX)
                .map(NodeId::new)
                .ok_or(CommandError::IdSpaceExhausted)?;
            let base = ElementBase {
                id,
                name: spec.name.clone(),
                placement: spec.placement,
                opacity: spec.opacity,
                blend: spec.blend,
                visible: spec.visible,
                locked: spec.locked,
            };
            match parent {
                Some(at) => nodes[at].children.push(id),
                None => tops.push(id),
            }
            stack.extend(spec.children.iter().rev().map(|c| (c, Some(nodes.len()))));
            nodes.push(Node::new(base, spec.kind.clone()));
        }
        Ok((nodes, tops))
    }
}

/// Runs `validate` over the new nodes alone, in a scratch page of the same
/// size, so a bad value is refused before the document is touched.
fn validate_new(
    doc: &Document,
    page: &Page,
    nodes: &[Node],
    tops: &[NodeId],
) -> Result<(), CommandError> {
    let mut scratch = Page::new(page.id, String::new(), page.size);
    scratch.roots = tops.to_vec();
    scratch.nodes = nodes
        .iter()
        .map(|n| (n.base.id, Arc::new(n.clone())))
        .collect::<BTreeMap<_, _>>();
    let next = doc.next_id() + u64::try_from(nodes.len()).unwrap_or(0);
    // The asset table comes along, so a new image is checked against the
    // assets it will live with (spec 029).
    Document::from_parts_with_assets(
        next,
        doc.dpi(),
        vec![Arc::new(scratch)],
        doc.assets().clone(),
    )
    .map(|_| ())
    .map_err(CommandError::InvalidDocument)
}

impl Command for InsertNodes {
    fn label(&self) -> CommandLabel {
        MessageKey("edit.insert_nodes")
    }

    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        if self.specs.is_empty() {
            return Err(CommandError::InvalidValue("an empty list of nodes"));
        }
        let view = doc.as_doc();
        self.check_target(view)?;
        let counter = view.next_id();
        let (nodes, tops) = self.build(counter)?;
        let page = view
            .page(self.page)
            .ok_or(CommandError::MissingPage(self.page))?;
        validate_new(view, page, &nodes, &tops)?;

        for _ in &nodes {
            doc.alloc_node_id()
                .map_err(|_| CommandError::IdSpaceExhausted)?;
        }
        let ids: Vec<NodeId> = nodes.iter().map(|n| n.base.id).collect();
        if let Some(parent) = self.parent {
            let node = doc
                .node_mut(self.page, parent)
                .ok_or(CommandError::MissingNode(parent))?;
            node.children
                .splice(self.index..self.index, tops.iter().copied());
        }
        let page = doc
            .page_mut(self.page)
            .ok_or(CommandError::MissingPage(self.page))?;
        if self.parent.is_none() {
            page.roots
                .splice(self.index..self.index, tops.iter().copied());
        }
        page.nodes
            .extend(nodes.into_iter().map(|n| (n.base.id, Arc::new(n))));
        self.record = Some(Record { counter, ids, tops });
        Ok(Applied::Changed)
    }

    fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError> {
        let record = self.record.as_ref().ok_or(CommandError::NotApplied)?;
        let page = doc
            .as_doc()
            .page(self.page)
            .ok_or(CommandError::MissingPage(self.page))?;
        if let Some(id) = record.ids.iter().find(|id| page.node(**id).is_none()) {
            return Err(CommandError::MissingNode(*id));
        }
        // Restoring the counter must not hand out an id a later command took.
        let allocated = u64::try_from(record.ids.len()).ok();
        if doc.as_doc().next_id().checked_sub(record.counter) != allocated {
            return Err(CommandError::InvalidValue(
                "ids were allocated after the nodes were inserted",
            ));
        }
        let tops: BTreeSet<NodeId> = record.tops.iter().copied().collect();
        if let Some(parent) = self.parent {
            let node = doc
                .node_mut(self.page, parent)
                .ok_or(CommandError::MissingNode(parent))?;
            node.children.retain(|c| !tops.contains(c));
        }
        let page = doc
            .page_mut(self.page)
            .ok_or(CommandError::MissingPage(self.page))?;
        if self.parent.is_none() {
            page.roots.retain(|r| !tops.contains(r));
        }
        for id in &record.ids {
            page.nodes.remove(id);
        }
        doc.restore_next_id(record.counter);
        Ok(())
    }

    fn touches(&self) -> DirtySet {
        let mut dirty = DirtySet::default();
        match self.parent {
            None => dirty.pages.insert(self.page),
            Some(parent) => dirty.nodes.insert(parent),
        };
        if let Some(record) = &self.record {
            dirty.nodes.extend(record.ids.iter().copied());
        }
        dirty
    }
}
