//! What `Reorder` and `Reparent` share: planning a move as new sibling lists,
//! writing them, and undoing them only over the state `apply` left.

use std::collections::{BTreeMap, BTreeSet};
use std::iter::successors;

use gafira_doc::{DocMut, NodeId, NodeKind, Page, PageId};

use crate::{CommandError, DirtySet};

/// Sibling lists by owner: `None` is the page roots, `Some(g)` the children
/// of group `g`.
type Lists = BTreeMap<Option<NodeId>, Vec<NodeId>>;

/// The lists a successful move rewrote, as they were and as they became.
/// Only lists that differ are kept; both maps have the same keys.
#[derive(Clone, Debug, Default)]
pub(crate) struct Record {
    before: Lists,
    after: Lists,
    moved: Vec<NodeId>,
}

/// Parent and index of every node reachable from the roots or a `children`
/// list of the page.
pub(crate) fn places(page: &Page) -> BTreeMap<NodeId, (Option<NodeId>, usize)> {
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

/// The sibling list owned by `parent`.
pub(crate) fn list(page: &Page, parent: Option<NodeId>) -> Option<&Vec<NodeId>> {
    match parent {
        None => Some(&page.roots),
        Some(id) => page.node(id).map(|n| &n.children),
    }
}

/// Every node of the page, parents before children, siblings in list order.
fn document_order(page: &Page) -> Vec<NodeId> {
    let mut seen = BTreeSet::new();
    let mut order = Vec::new();
    let mut stack: Vec<NodeId> = page.roots.iter().rev().copied().collect();
    while let Some(id) = stack.pop() {
        if seen.insert(id) {
            order.push(id);
            stack.extend(page.node(id).iter().flat_map(|n| n.children.iter().rev()));
        }
    }
    order
}

/// Plans moving `nodes` under `parent` at `index` of that list once the
/// moved nodes are out of it. `Ok(None)` means the lists would not change.
/// Nothing is written: every check of ARQ-13 happens here.
pub(crate) fn plan(
    page: &Page,
    nodes: &[NodeId],
    parent: Option<NodeId>,
    index: usize,
) -> Result<Option<Record>, CommandError> {
    if nodes.is_empty() {
        return Err(CommandError::InvalidValue("an empty list of nodes"));
    }
    if let Some(id) = nodes.iter().find(|id| page.node(**id).is_none()) {
        return Err(CommandError::MissingNode(*id));
    }
    if let Some(id) = parent {
        let target = page.node(id).ok_or(CommandError::MissingNode(id))?;
        if !matches!(target.kind, NodeKind::Group(_)) {
            return Err(CommandError::NotAContainer(id));
        }
    }
    let at = places(page);
    let above = |id: NodeId| {
        successors(at.get(&id).and_then(|p| p.0), |c| {
            at.get(c).and_then(|p| p.0)
        })
        .take(at.len())
    };
    // A node listed with one of its ancestors travels with that ancestor.
    let listed: BTreeSet<NodeId> = nodes.iter().copied().collect();
    let tops: BTreeSet<NodeId> = listed
        .iter()
        .copied()
        .filter(|id| !above(*id).any(|a| listed.contains(&a)))
        .collect();
    if let Some(id) = parent
        && let Some(top) = successors(Some(id), |c| above(*c).next())
            .take(at.len())
            .find(|c| tops.contains(c))
    {
        return Err(CommandError::WouldCreateCycle(top));
    }
    let moved: Vec<NodeId> = document_order(page)
        .into_iter()
        .filter(|id| tops.contains(id))
        .collect();

    let owners: BTreeSet<Option<NodeId>> = moved
        .iter()
        .filter_map(|id| at.get(id).map(|p| p.0))
        .chain([parent])
        .collect();
    let mut before: Lists = owners
        .into_iter()
        .filter_map(|owner| list(page, owner).map(|ids| (owner, ids.clone())))
        .collect();
    let mut after = before.clone();
    for ids in after.values_mut() {
        ids.retain(|id| !tops.contains(id));
    }
    let target = after.entry(parent).or_default();
    if index > target.len() {
        return Err(CommandError::IndexOutOfRange {
            index,
            len: target.len(),
        });
    }
    target.splice(index..index, moved.iter().copied());

    before.retain(|owner, ids| after.get(owner) != Some(ids));
    after.retain(|owner, _| before.contains_key(owner));
    if after.is_empty() {
        return Ok(None);
    }
    Ok(Some(Record {
        before,
        after,
        moved,
    }))
}

impl Record {
    /// Writes the new lists. `plan` already proved the page and every owner
    /// exist, so nothing is half done.
    pub(crate) fn write(&self, doc: &mut DocMut<'_>, page: PageId) -> Result<(), CommandError> {
        write(doc, page, &self.after)
    }

    /// Puts the old lists back, but only over the lists `apply` left; any
    /// other state fails before a single write.
    pub(crate) fn revert(&self, doc: &mut DocMut<'_>, page: PageId) -> Result<(), CommandError> {
        if self.after.is_empty() {
            return Ok(());
        }
        let current = doc
            .as_doc()
            .page(page)
            .ok_or(CommandError::MissingPage(page))?;
        for (owner, expected) in &self.after {
            let now = match owner {
                None => &current.roots,
                Some(id) => {
                    &current
                        .node(*id)
                        .ok_or(CommandError::MissingNode(*id))?
                        .children
                }
            };
            if now != expected {
                return Err(CommandError::InvalidValue("the lists changed since apply"));
            }
        }
        write(doc, page, &self.before)
    }

    /// The owners of the changed lists (the page for the roots) and the
    /// nodes that moved.
    pub(crate) fn touches(&self, page: PageId) -> DirtySet {
        let mut dirty = DirtySet::default();
        for owner in self.after.keys() {
            match owner {
                None => dirty.pages.insert(page),
                Some(id) => dirty.nodes.insert(*id),
            };
        }
        dirty.nodes.extend(self.moved.iter().copied());
        dirty
    }
}

fn write(doc: &mut DocMut<'_>, page: PageId, lists: &Lists) -> Result<(), CommandError> {
    for (owner, ids) in lists {
        match owner {
            None => {
                let target = doc.page_mut(page).ok_or(CommandError::MissingPage(page))?;
                target.roots.clone_from(ids);
            }
            Some(id) => {
                let node = doc
                    .node_mut(page, *id)
                    .ok_or(CommandError::MissingNode(*id))?;
                node.children.clone_from(ids);
            }
        }
    }
    Ok(())
}
