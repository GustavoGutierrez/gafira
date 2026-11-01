//! Page-space bounds of a node (D-28, ADR 0009).

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
    walk(page, id, &mut Vec::new())
}

fn walk(page: &Page, id: NodeId, path: &mut Vec<NodeId>) -> Result<Rect, LayoutError> {
    if path.contains(&id) {
        return Err(LayoutError::Cycle(id));
    }
    let node = page.node(id).ok_or(LayoutError::UnknownNode(id))?;
    let is_group = matches!(node.kind, NodeKind::Group(GroupNode {}));
    if !is_group || node.children.is_empty() {
        return own_box(&node.base.placement);
    }
    path.push(id);
    let mut union: Option<Rect> = None;
    for &child in &node.children {
        let b = walk(page, child, path)?;
        union = Some(union.map_or(b, |u| u.union(b)));
    }
    path.pop();
    union.ok_or(LayoutError::UnknownNode(id))
}

fn own_box(p: &Placement) -> Result<Rect, LayoutError> {
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
    if p.w < 0.0 || p.h < 0.0 {
        return Err(LayoutError::InvalidBox { field: "placement" });
    }
    let local = Rect::new(0.0, 0.0, p.w, p.h);
    check_rect("placement", p.to_transform().transform_rect_bbox(local))
}
