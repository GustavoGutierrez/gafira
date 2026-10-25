//! `RemoveNodes`: node subtrees out of one page (ARQ-81, ADR-03).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use gafira_doc::msg::{CommandLabel, MessageKey};
use gafira_doc::{DocMut, Node, NodeId, Page, PageId};

use crate::{Applied, Command, CommandError, DirtySet};

/// Where a removed subtree root sat: its parent (`None` for a page root) and
/// its index in that list before anything was removed.
#[derive(Clone, Debug)]
struct Place {
    id: NodeId,
    parent: Option<NodeId>,
    index: usize,
}

/// What one successful `apply` took out, for `revert` and `touches`.
#[derive(Clone, Debug)]
struct Record {
    /// The removed subtree roots, ordered by parent and then index.
    places: Vec<Place>,
    /// Every removed node, kept as the very `Arc` that was in the arena.
    nodes: Vec<Arc<Node>>,
}

/// Removes nodes with all their descendants from one page.
///
/// A node listed together with one of its descendants counts once, as the
/// ancestor. `revert` puts every subtree back at its old parent and index.
#[derive(Clone, Debug)]
pub struct RemoveNodes {
    page: PageId,
    nodes: Vec<NodeId>,
    record: Option<Record>,
}

impl RemoveNodes {
    /// Removes `nodes` (and everything below them) from `page`.
    pub fn new(page: PageId, nodes: Vec<NodeId>) -> Self {
        Self {
            page,
            nodes,
            record: None,
        }
    }
}

/// The ids strictly below `roots`, following `children` inside `page`.
fn below(page: &Page, roots: &[NodeId]) -> BTreeSet<NodeId> {
    let mut found = BTreeSet::new();
    let mut stack: Vec<NodeId> = roots.to_vec();
    while let Some(id) = stack.pop() {
        for child in page.node(id).iter().flat_map(|n| &n.children) {
            if found.insert(*child) {
                stack.push(*child);
            }
        }
    }
    found
}

/// Parent and index of every node reachable from the roots or a `children`
/// list of the page.
fn places(page: &Page) -> BTreeMap<NodeId, (Option<NodeId>, usize)> {
    let mut found = BTreeMap::new();
    for (index, id) in page.roots.iter().enumerate() {
        found.insert(*id, (None, index));
    }
    for (parent, node) in &page.nodes {
        for (index, id) in node.children.iter().enumerate() {
            found.insert(*id, (Some(*parent), index));
        }
    }
    found
}

impl Command for RemoveNodes {
    fn label(&self) -> CommandLabel {
        MessageKey("edit.remove_nodes")
    }

    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        if self.nodes.is_empty() {
            return Err(CommandError::InvalidValue("an empty list of nodes"));
        }
        let page = doc
            .as_doc()
            .page(self.page)
            .ok_or(CommandError::MissingPage(self.page))?;
        if let Some(id) = self.nodes.iter().find(|id| page.node(**id).is_none()) {
            return Err(CommandError::MissingNode(*id));
        }
        let descendants = below(page, &self.nodes);
        let tops: BTreeSet<NodeId> = self
            .nodes
            .iter()
            .copied()
            .filter(|id| !descendants.contains(id))
            .collect();
        let at = places(page);
        let mut places: Vec<Place> = tops
            .iter()
            .filter_map(|id| at.get(id).map(|(parent, index)| (*id, *parent, *index)))
            .map(|(id, parent, index)| Place { id, parent, index })
            .collect();
        places.sort_by_key(|p| (p.parent, p.index));
        let gone: BTreeSet<NodeId> = tops.union(&descendants).copied().collect();
        let nodes: Vec<Arc<Node>> = page
            .nodes
            .iter()
            .filter(|(id, _)| gone.contains(id))
            .map(|(_, node)| Arc::clone(node))
            .collect();

        let parents: BTreeSet<NodeId> = places.iter().filter_map(|p| p.parent).collect();
        for parent in parents {
            let node = doc
                .node_mut(self.page, parent)
                .ok_or(CommandError::MissingNode(parent))?;
            node.children.retain(|c| !tops.contains(c));
        }
        let page = doc
            .page_mut(self.page)
            .ok_or(CommandError::MissingPage(self.page))?;
        page.roots.retain(|r| !tops.contains(r));
        page.nodes.retain(|id, _| !gone.contains(id));
        self.record = Some(Record { places, nodes });
        Ok(Applied::Changed)
    }

    fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError> {
        let record = self.record.as_ref().ok_or(CommandError::NotApplied)?;
        let page = doc
            .as_doc()
            .page(self.page)
            .ok_or(CommandError::MissingPage(self.page))?;
        if record.nodes.iter().any(|n| page.node(n.base.id).is_some()) {
            return Err(CommandError::InvalidValue("a removed node is already back"));
        }
        let mut lens: BTreeMap<Option<NodeId>, usize> = BTreeMap::new();
        for place in &record.places {
            if let Some(parent) = place.parent
                && page.node(parent).is_none()
            {
                return Err(CommandError::MissingNode(parent));
            }
            let len = lens.entry(place.parent).or_insert_with(|| {
                place
                    .parent
                    .and_then(|p| page.node(p))
                    .map_or(page.roots.len(), |p| p.children.len())
            });
            if place.index > *len {
                return Err(CommandError::IndexOutOfRange {
                    index: place.index,
                    len: *len,
                });
            }
            *len += 1;
        }

        let target = doc
            .page_mut(self.page)
            .ok_or(CommandError::MissingPage(self.page))?;
        target
            .nodes
            .extend(record.nodes.iter().map(|n| (n.base.id, Arc::clone(n))));
        // Ascending order: every lower index of a list is back before the next.
        for place in &record.places {
            match place.parent {
                None => target_roots(doc, self.page)?.insert(place.index, place.id),
                Some(parent) => doc
                    .node_mut(self.page, parent)
                    .ok_or(CommandError::MissingNode(parent))?
                    .children
                    .insert(place.index, place.id),
            }
        }
        Ok(())
    }

    fn touches(&self) -> DirtySet {
        let mut dirty = DirtySet::default();
        match &self.record {
            Some(record) => {
                dirty.nodes.extend(record.nodes.iter().map(|n| n.base.id));
                for place in &record.places {
                    match place.parent {
                        None => dirty.pages.insert(self.page),
                        Some(parent) => dirty.nodes.insert(parent),
                    };
                }
            }
            None => dirty.nodes.extend(self.nodes.iter().copied()),
        }
        dirty
    }
}

fn target_roots<'a>(
    doc: &'a mut DocMut<'_>,
    page: PageId,
) -> Result<&'a mut Vec<NodeId>, CommandError> {
    doc.page_mut(page)
        .map(|p| &mut p.roots)
        .ok_or(CommandError::MissingPage(page))
}
