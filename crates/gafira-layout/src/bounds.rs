//! Page-space bounds of a node (D-28, ADR 0009).

use std::collections::{BTreeMap, BTreeSet};

use gafira_doc::{GroupNode, NodeId, NodeKind, Page, Placement};
use gafira_geom::Rect;

use crate::LayoutError;
use crate::error::{check_rect, finite};

/// The axis-aligned box of a node in page space.
///
/// A leaf is the bounding box of its placed local box (rotation, skew and flip
/// included). A group is the union of its children's bounds; an empty group
/// uses its own box. Strokes and effects do not count.
pub fn node_bounds(page: &Page, id: NodeId) -> Result<Rect, LayoutError> {
    bounds_with(page, id, &mut BTreeMap::new())
}

/// [`node_bounds`] that keeps the boxes of the nodes it finishes in `done`, so
/// asking for many nodes of one page costs one walk of each node in all.
pub(crate) fn bounds_with(
    page: &Page,
    id: NodeId,
    done: &mut BTreeMap<NodeId, Rect>,
) -> Result<Rect, LayoutError> {
    // One frame per group on the path: the group, its next child, the union so far.
    // An explicit stack keeps any nesting depth off the call stack.
    let mut open: BTreeSet<NodeId> = BTreeSet::new();
    let mut stack: Vec<(NodeId, usize, Option<Rect>)> = Vec::new();
    let mut next = Some(id);
    loop {
        if let Some(id) = next.take() {
            if open.contains(&id) {
                return Err(LayoutError::Cycle(id));
            }
            if let Some(&b) = done.get(&id) {
                fold(&mut stack, b);
            } else {
                let node = page.node(id).ok_or(LayoutError::UnknownNode(id))?;
                let is_group = matches!(node.kind, NodeKind::Group(GroupNode {}));
                if is_group && !node.children.is_empty() {
                    check_placement(&node.base.placement)?;
                    open.insert(id);
                    stack.push((id, 0, None));
                } else {
                    let b = own_box(&node.base.placement)?;
                    done.insert(id, b);
                    fold(&mut stack, b);
                }
            }
        }
        let Some(&(gid, at, union)) = stack.last() else {
            return done.get(&id).copied().ok_or(LayoutError::UnknownNode(id));
        };
        let node = page.node(gid).ok_or(LayoutError::UnknownNode(gid))?;
        if let Some(&child) = node.children.get(at) {
            if let Some(top) = stack.last_mut() {
                top.1 += 1;
            }
            next = Some(child);
        } else {
            stack.pop();
            open.remove(&gid);
            let b = union.ok_or(LayoutError::UnknownNode(gid))?;
            done.insert(gid, b);
            fold(&mut stack, b);
        }
    }
}

/// Adds a finished child box to the union of the group being walked.
fn fold(stack: &mut [(NodeId, usize, Option<Rect>)], b: Rect) {
    if let Some(top) = stack.last_mut() {
        top.2 = Some(top.2.map_or(b, |u| u.union(b)));
    }
}

fn check_placement(p: &Placement) -> Result<(), LayoutError> {
    let fields = [
        ("x", p.x),
        ("y", p.y),
        ("w", p.w),
        ("h", p.h),
        ("rotation_deg", p.rotation_deg),
        ("skew", p.skew),
    ];
    for (field, v) in fields {
        finite(field, v)?;
    }
    Ok(())
}

fn own_box(p: &Placement) -> Result<Rect, LayoutError> {
    check_placement(p)?;
    if p.w < 0.0 || p.h < 0.0 {
        return Err(LayoutError::InvalidBox { field: "placement" });
    }
    let local = Rect::new(0.0, 0.0, p.w, p.h);
    check_rect("placement", p.to_transform().transform_rect_bbox(local))
}
