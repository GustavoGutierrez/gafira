//! The elements that attract the magnet (FD-06 c).

use std::collections::{BTreeMap, BTreeSet};

use gafira_doc::{NodeId, Page};
use gafira_geom::Rect;

use crate::LayoutError;
use crate::bounds::bounds_with;

/// One group on the path from a root: the node, its next child, whether it is
/// out of the attractors, and whether its subtree holds part of the selection.
struct Frame {
    id: NodeId,
    next: usize,
    blocked: bool,
    has_selection: bool,
    slot: usize,
}

enum Step {
    Enter(NodeId, bool),
    Exit,
}

/// The boxes of the elements that attract while `selection` moves, in document
/// order (roots as listed, a parent before its children).
///
/// Each box is the one [`crate::node_bounds`] gives. A node does not attract when
/// it is in the selection, under a selected node, an ancestor of a selected node
/// (so a group holding any of the selection at any depth is out, while its other
/// children stay in), hidden or locked, or under a hidden or locked ancestor.
/// Page flags are not read. A group that attracts does so by the union of all its
/// children, including those that do not attract.
///
/// The nodes read are the ones that attract and all their descendants, hidden and
/// locked ones included, as in [`crate::node_bounds`]: a NaN in a child of an
/// attracting group is an error though that child does not attract. A node in no
/// attracting subtree is not read, so a NaN there changes nothing. A selected id
/// that is not in the page is `UnknownNode`, as is a root or child missing from
/// it; a group that contains itself is `Cycle`. The tree is walked with an
/// explicit stack, so any depth is fine, and each node is measured once.
pub fn attractors(page: &Page, selection: &[NodeId]) -> Result<Vec<Rect>, LayoutError> {
    let mut selected = BTreeSet::new();
    for &id in selection {
        page.node(id).ok_or(LayoutError::UnknownNode(id))?;
        selected.insert(id);
    }
    // Every node in document order, and whether it attracts so far.
    let mut order: Vec<(NodeId, bool)> = Vec::new();
    let mut finished: BTreeMap<NodeId, bool> = BTreeMap::new();
    let mut open: BTreeSet<NodeId> = BTreeSet::new();
    let mut stack: Vec<Frame> = Vec::new();
    let mut next_root = 0;
    loop {
        let step = match stack.last_mut() {
            Some(top) => {
                let node = page.node(top.id).ok_or(LayoutError::UnknownNode(top.id))?;
                match node.children.get(top.next) {
                    Some(&child) => {
                        top.next += 1;
                        Step::Enter(child, top.blocked)
                    }
                    None => Step::Exit,
                }
            }
            None => match page.roots.get(next_root) {
                Some(&root) => {
                    next_root += 1;
                    Step::Enter(root, false)
                }
                None => break,
            },
        };
        match step {
            Step::Enter(id, parent_blocked) => {
                if open.contains(&id) {
                    return Err(LayoutError::Cycle(id));
                }
                if let Some(&holds) = finished.get(&id) {
                    // Reached twice: counted where it was first reached.
                    if let Some(top) = stack.last_mut() {
                        top.has_selection |= holds;
                    }
                    continue;
                }
                let node = page.node(id).ok_or(LayoutError::UnknownNode(id))?;
                let is_selected = selected.contains(&id);
                let blocked =
                    parent_blocked || !node.base.visible || node.base.locked || is_selected;
                open.insert(id);
                stack.push(Frame {
                    id,
                    next: 0,
                    blocked,
                    has_selection: is_selected,
                    slot: order.len(),
                });
                order.push((id, !blocked));
            }
            Step::Exit => {
                let Some(frame) = stack.pop() else { break };
                open.remove(&frame.id);
                finished.insert(frame.id, frame.has_selection);
                if frame.has_selection {
                    order[frame.slot].1 = false;
                }
                if let Some(parent) = stack.last_mut() {
                    parent.has_selection |= frame.has_selection;
                }
            }
        }
    }
    let mut done = BTreeMap::new();
    let mut boxes = Vec::new();
    for (id, attracts) in order {
        if attracts {
            boxes.push(bounds_with(page, id, &mut done)?);
        }
    }
    Ok(boxes)
}
