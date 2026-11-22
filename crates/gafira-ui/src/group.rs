//! Groups and the tree of a page (spec 043; FD-13, D-35, 03 §9.1). Pure,
//! so it is tested without a window: the editor turns the results into
//! commands.
//!
//! Every placement is in page space, the one of a group's children too
//! (D-35): a group's own placement does not move them, and its box is the
//! box around its children. So grouping and ungrouping move no element:
//! they only rewrite the sibling lists. Moving, resizing, turning or
//! flipping a group rewrites the placement of each node below it, and the
//! group's own placement (and the one of every group inside it) is set
//! back to the box around its children.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use gafira_doc::msg::MessageKey;
use gafira_doc::{GroupNode, NodeId, NodeKind, Page, Placement};
use gafira_edit::{Command, InsertNodes, NodeSpec, RemoveNodes, Reparent, Transaction};
use gafira_geom::Point;

use crate::rotation::{self, Axis};

/// The parent of every node reachable from the roots: `None` for a root,
/// `Some(group)` for a child of `group`.
pub(crate) fn parents(page: &Page) -> BTreeMap<NodeId, Option<NodeId>> {
    let mut found = BTreeMap::new();
    let mut stack: Vec<(NodeId, Option<NodeId>)> =
        page.roots.iter().rev().map(|id| (*id, None)).collect();
    while let Some((id, parent)) = stack.pop() {
        if found.contains_key(&id) {
            continue;
        }
        found.insert(id, parent);
        if let Some(node) = page.node(id) {
            stack.extend(node.children.iter().rev().map(|c| (*c, Some(id))));
        }
    }
    found
}

/// Every node of the page, parents before children, siblings back to front.
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

/// Whether `id` is a group of `page`.
pub(crate) fn is_group(page: &Page, id: NodeId) -> bool {
    page.node(id)
        .is_some_and(|node| matches!(node.kind, NodeKind::Group(GroupNode {})))
}

/// What a click on the drawn element `leaf` selects when the canvas is in
/// `scope` (the page roots for `None`, or the group entered with a double
/// click): the ancestor of `leaf`, or `leaf` itself, whose parent is
/// `scope`. `None` when `leaf` is not inside `scope`.
pub(crate) fn pick(page: &Page, leaf: NodeId, scope: Option<NodeId>) -> Option<NodeId> {
    let parents = parents(page);
    let mut at = leaf;
    // At most one step per node, so a broken tree cannot loop.
    for _ in 0..=parents.len() {
        let parent = *parents.get(&at)?;
        if parent == scope {
            return Some(at);
        }
        at = parent?;
    }
    None
}

/// What a click on the drawn element `leaf` selects with the canvas in
/// `scope`, and the scope after it (03 §9.1): the child of the scope that
/// holds `leaf`; outside the scope, the root that holds it, and the scope
/// is left. A double click on a group enters it and selects its child
/// under the pointer.
pub(crate) fn click(
    page: &Page,
    leaf: NodeId,
    scope: Option<NodeId>,
    double: bool,
) -> Option<(NodeId, Option<NodeId>)> {
    let (item, scope) = match pick(page, leaf, scope) {
        Some(item) => (item, scope),
        None => (pick(page, leaf, None)?, None),
    };
    if double
        && is_group(page, item)
        && let Some(child) = pick(page, leaf, Some(item))
    {
        return Some((child, Some(item)));
    }
    Some((item, scope))
}

/// Whether something under `id` (itself included) is drawn: a visible
/// element that is not a group, below visible groups only.
pub(crate) fn draws(page: &Page, id: NodeId) -> bool {
    let mut seen = BTreeSet::new();
    let mut stack = vec![id];
    while let Some(id) = stack.pop() {
        let Some(node) = page.node(id) else { continue };
        if !node.base.visible || !seen.insert(id) {
            continue;
        }
        match node.kind {
            NodeKind::Group(_) => stack.extend(node.children.iter().copied()),
            NodeKind::Extension(_) => {}
            _ => return true,
        }
    }
    false
}

/// Whether a click can pick something under `id` (itself included): a
/// visible, free element that is not a group, below visible, free groups
/// only (D-91).
fn pickable(page: &Page, id: NodeId) -> bool {
    let mut seen = BTreeSet::new();
    let mut stack = vec![id];
    while let Some(id) = stack.pop() {
        let Some(node) = page.node(id) else { continue };
        if !node.base.visible || node.base.locked || !seen.insert(id) {
            continue;
        }
        match node.kind {
            NodeKind::Group(_) => stack.extend(node.children.iter().copied()),
            NodeKind::Extension(_) => {}
            _ => return true,
        }
    }
    false
}

/// The nodes of `scope` the canvas draws and a click can pick, back to
/// front: its children with something visible and free below them. What
/// Ctrl+A selects; nothing inside a hidden or locked scope (D-91).
pub(crate) fn items(page: &Page, scope: Option<NodeId>) -> Vec<NodeId> {
    if page.locked {
        return Vec::new();
    }
    let list = match scope {
        None => &page.roots,
        Some(group) if crate::layers::locked(page, group) => return Vec::new(),
        Some(group) => match page.node(group) {
            Some(node) if node.base.visible => &node.children,
            _ => return Vec::new(),
        },
    };
    list.iter()
        .copied()
        .filter(|id| pickable(page, *id))
        .collect()
}

/// The box of `id`: its own placement for an element, the upright box
/// around its children for a group (D-35). `None` for a missing node or a
/// box that is not valid.
pub(crate) fn item_box(page: &Page, id: NodeId) -> Option<Placement> {
    let node = page.node(id)?;
    if !is_group(page, id) {
        return Some(node.base.placement);
    }
    let b = gafira_layout::node_bounds(page, id).ok()?;
    Some(Placement::new(b.x0, b.y0, b.width(), b.height()))
}

/// The sibling list `nodes` share, with its owner (`None` for the roots);
/// `None` when they have different parents or one is missing.
pub(crate) fn siblings(page: &Page, nodes: &[NodeId]) -> Option<(Option<NodeId>, Vec<NodeId>)> {
    let parents = parents(page);
    let first = *parents.get(nodes.first()?)?;
    if nodes.iter().any(|id| parents.get(id) != Some(&first)) {
        return None;
    }
    let list = match first {
        None => page.roots.clone(),
        Some(group) => page.node(group)?.children.clone(),
    };
    Some((first, list))
}

/// How to group `nodes`: a new group in the parent of the frontmost of
/// them, just above it, and then the nodes moved into it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GroupPlan {
    /// The list the group goes into (`None` for the roots).
    pub(crate) parent: Option<NodeId>,
    /// Its index there, before the nodes leave that list.
    pub(crate) index: usize,
    /// The nodes, back to front: their order in the group.
    pub(crate) nodes: Vec<NodeId>,
    /// The group's own placement: the box around them.
    pub(crate) placement: Placement,
}

/// The plan that groups `nodes` of `page`; `None` with no node, a missing
/// one or a box that is not valid.
pub(crate) fn group_plan(page: &Page, nodes: &[NodeId]) -> Option<GroupPlan> {
    let wanted: BTreeSet<NodeId> = nodes.iter().copied().collect();
    let parents = parents(page);
    if wanted.is_empty() || wanted.iter().any(|id| !parents.contains_key(id)) {
        return None;
    }
    let ordered: Vec<NodeId> = document_order(page)
        .into_iter()
        .filter(|id| wanted.contains(id))
        .collect();
    // The frontmost is the last in document order whose ancestors are not
    // listed: the one drawn last.
    let front = *ordered.last()?;
    let parent = *parents.get(&front)?;
    let list = match parent {
        None => &page.roots,
        Some(group) => &page.node(group)?.children,
    };
    let index = list.iter().position(|id| *id == front)? + 1;
    let mut around: Option<gafira_layout::Rect> = None;
    for id in &ordered {
        let b = gafira_layout::node_bounds(page, *id).ok()?;
        around = Some(around.map_or(b, |all| all.union(b)));
    }
    let b = around?;
    Some(GroupPlan {
        parent,
        index,
        nodes: ordered,
        placement: Placement::new(b.x0, b.y0, b.width(), b.height()),
    })
}

/// The one-step command that groups `nodes` of `page` (one undo step):
/// a new group inserted above the frontmost of them, which gets the id
/// `next` (the document's next id), and the nodes moved into it in their
/// order. With the new group's id.
pub(crate) fn group_command(
    page: &Page,
    nodes: &[NodeId],
    next: u64,
) -> Option<(Box<dyn Command>, NodeId)> {
    let plan = group_plan(page, nodes)?;
    let group = NodeId::new(next);
    let spec = NodeSpec::new(NodeKind::Group(GroupNode {}), plan.placement);
    let commands: Vec<Box<dyn Command>> = vec![
        Box::new(InsertNodes::new(
            page.id,
            plan.parent,
            plan.index,
            vec![spec],
        )),
        Box::new(Reparent::new(page.id, plan.nodes, Some(group), 0)),
    ];
    Some((
        Box::new(Transaction::new(MessageKey("edit.group"), commands)),
        group,
    ))
}

/// How to ungroup a group: its children go to its parent at its index,
/// in their order, and the empty group is removed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UngroupPlan {
    pub(crate) parent: Option<NodeId>,
    pub(crate) index: usize,
    pub(crate) children: Vec<NodeId>,
}

/// The plan that ungroups `group`; `None` when it is not a group or has no
/// children.
pub(crate) fn ungroup_plan(page: &Page, group: NodeId) -> Option<UngroupPlan> {
    if !is_group(page, group) {
        return None;
    }
    let children = page.node(group)?.children.clone();
    if children.is_empty() {
        return None;
    }
    let parent = *parents(page).get(&group)?;
    let list = match parent {
        None => &page.roots,
        Some(owner) => &page.node(owner)?.children,
    };
    let index = list.iter().position(|id| *id == group)?;
    Some(UngroupPlan {
        parent,
        index,
        children,
    })
}

/// The one-step command that ungroups every group of `groups` (one undo
/// step), with the nodes that were their children, in order.
pub(crate) fn ungroup_command(
    page: &Page,
    groups: &[NodeId],
) -> Option<(Box<dyn Command>, Vec<NodeId>)> {
    let mut commands: Vec<Box<dyn Command>> = Vec::new();
    let mut freed = Vec::new();
    // Each plan is read on the page as the earlier steps leave it: they
    // move other lists, and a group may sit inside another one.
    let mut now = page.clone();
    for group in groups {
        let Some(plan) = ungroup_plan(&now, *group) else {
            continue;
        };
        let mut siblings = match plan.parent {
            None => now.roots.clone(),
            Some(owner) => now.node(owner)?.children.clone(),
        };
        siblings.splice(plan.index..=plan.index, plan.children.iter().copied());
        match plan.parent {
            None => now.roots = siblings,
            Some(owner) => {
                let mut node = now.node(owner)?.clone();
                node.children = siblings;
                now.nodes.insert(owner, Arc::new(node));
            }
        }
        now.nodes.remove(group);
        freed.extend(plan.children.iter().copied());
        commands.push(Box::new(Reparent::new(
            page.id,
            plan.children,
            plan.parent,
            plan.index,
        )));
        commands.push(Box::new(RemoveNodes::new(page.id, vec![*group])));
    }
    if commands.is_empty() {
        return None;
    }
    Some((
        Box::new(Transaction::new(MessageKey("edit.ungroup"), commands)),
        freed,
    ))
}

/// What happens to a whole group.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Change {
    /// Every node moves by `dx, dy` page px.
    Move { dx: f64, dy: f64 },
    /// The upright box `from` becomes the upright box `to`: each node keeps
    /// its place relative to the box and scales with it.
    Frame { from: Placement, to: Placement },
    /// Every node turns `degrees` clockwise about `center`.
    Turn { center: Point, degrees: f64 },
    /// Every node is mirrored along `axis` through `center`.
    Flip { center: Point, axis: Axis },
}

/// `p` with its centre at `(cx, cy)`.
fn centred(mut p: Placement, cx: f64, cy: f64) -> Placement {
    p.x = cx - p.w / 2.0;
    p.y = cy - p.h / 2.0;
    p
}

/// The letters a corner resize of group `id` by `scale` scales (owner
/// rule): the words of every text element and every shape below it, all by
/// the same factor, so a large and a small letter keep their proportion.
/// The factor is kept so no letter goes below the smallest size nor past
/// the largest; it is returned with the new words, `scale` when nothing
/// below has words.
pub(crate) fn scaled_letters(
    page: &Page,
    id: NodeId,
    scale: f64,
) -> (f64, Vec<(NodeId, ScaledWords)>) {
    let words: Vec<(NodeId, ScaledWords)> = subtree(page, id)
        .into_iter()
        .filter_map(|node| match &page.node(node)?.kind {
            NodeKind::Text(text) => Some((node, ScaledWords::Text(text.content.clone()))),
            NodeKind::Shape(shape) => shape
                .text
                .clone()
                .map(|text| (node, ScaledWords::Shape(text))),
            _ => None,
        })
        .collect();
    let k = words.iter().fold(scale, |k, (_, words)| {
        crate::textbox::corner_factor(words.content(), k)
    });
    let scaled = words
        .into_iter()
        .map(|(node, words)| (node, words.scaled(k)))
        .collect();
    (k, scaled)
}

/// The words of a text element or of a shape, for [`scaled_letters`].
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ScaledWords {
    Text(gafira_doc::RichText),
    Shape(gafira_doc::ShapeText),
}

impl ScaledWords {
    fn content(&self) -> &gafira_doc::RichText {
        match self {
            Self::Text(text) => text,
            Self::Shape(text) => &text.content,
        }
    }

    fn scaled(self, k: f64) -> Self {
        match self {
            Self::Text(text) => Self::Text(crate::textbox::scaled(&text, k)),
            Self::Shape(mut text) => {
                text.content = crate::textbox::scaled(&text.content, k);
                Self::Shape(text)
            }
        }
    }
}

/// The placement of one element after `change`.
fn mapped(p: &Placement, change: Change) -> Placement {
    let (cx, cy) = (p.x + p.w / 2.0, p.y + p.h / 2.0);
    match change {
        Change::Move { dx, dy } => {
            let mut out = *p;
            out.x += dx;
            out.y += dy;
            out
        }
        Change::Frame { from, to } => {
            let scale = |a: f64, b: f64| if a > 0.0 { b / a } else { 1.0 };
            let (sx, sy) = (scale(from.w, to.w), scale(from.h, to.h));
            // Each side scales by how much the box stretches along it: exact
            // for an upright element, a quarter turn or an even scale; a
            // turned element under an uneven scale keeps its turn.
            let (sin, cos) = p.rotation_deg.to_radians().sin_cos();
            let mut out = *p;
            out.w = p.w * (cos * sx).hypot(sin * sy);
            out.h = p.h * (sin * sx).hypot(cos * sy);
            centred(out, to.x + (cx - from.x) * sx, to.y + (cy - from.y) * sy)
        }
        Change::Turn { center, degrees } => {
            let (sin, cos) = degrees.to_radians().sin_cos();
            let (dx, dy) = (cx - center.x, cy - center.y);
            let out = rotation::turned_by(p, degrees);
            centred(
                out,
                center.x + dx * cos - dy * sin,
                center.y + dx * sin + dy * cos,
            )
        }
        Change::Flip { center, axis } => {
            let out = rotation::flipped(p, axis);
            match axis {
                Axis::Horizontal => centred(out, 2.0 * center.x - cx, cy),
                Axis::Vertical => centred(out, cx, 2.0 * center.y - cy),
            }
        }
    }
}

/// The subtree of `id`, parents before children.
fn subtree(page: &Page, id: NodeId) -> Vec<NodeId> {
    let mut seen = BTreeSet::new();
    let mut order = Vec::new();
    let mut stack = vec![id];
    while let Some(id) = stack.pop() {
        if page.node(id).is_some() && seen.insert(id) {
            order.push(id);
            stack.extend(page.node(id).iter().flat_map(|n| n.children.iter().rev()));
        }
    }
    order
}

/// The new placement of `id` and of every node below it after `change`,
/// parents before children. Every node is listed, also one that does not
/// move, so the frames of one drag are the same commands (spec 040).
pub(crate) fn changed(page: &Page, id: NodeId, change: Change) -> Vec<(NodeId, Placement)> {
    let order = subtree(page, id);
    let mut scratch = page.clone();
    let holds = |page: &Page, id: NodeId| {
        is_group(page, id) && !page.node(id).is_some_and(|n| n.children.is_empty())
    };
    // Elements and empty groups first, then each group around its new
    // children, the deepest first.
    for node in &order {
        if holds(page, *node) {
            continue;
        }
        let Some(mut moved) = scratch.node(*node).cloned() else {
            continue;
        };
        moved.base.placement = mapped(&moved.base.placement, change);
        scratch.nodes.insert(*node, Arc::new(moved));
    }
    for node in order.iter().rev() {
        if !holds(page, *node) {
            continue;
        }
        let Some(b) = item_box(&scratch, *node) else {
            continue;
        };
        let Some(mut refit) = scratch.node(*node).cloned() else {
            continue;
        };
        refit.base.placement = Placement::new(b.x, b.y, b.w, b.h);
        scratch.nodes.insert(*node, Arc::new(refit));
    }
    order
        .into_iter()
        .filter_map(|node| Some((node, scratch.node(node)?.base.placement)))
        .collect()
}

/// The page after `placements` are written.
pub(crate) fn with_placements(page: &Page, placements: &[(NodeId, Placement)]) -> Page {
    let mut out = page.clone();
    for (id, placement) in placements {
        if let Some(mut node) = out.node(*id).cloned() {
            node.base.placement = *placement;
            out.nodes.insert(*id, Arc::new(node));
        }
    }
    out
}

/// The subtree of `id` as a node to insert: the same values, no ids.
pub(crate) fn spec_of(page: &Page, id: NodeId) -> Option<NodeSpec> {
    fn build(page: &Page, id: NodeId, depth: usize) -> Option<NodeSpec> {
        let node = page.node(id)?;
        // Deeper than any valid document (validate caps the depth), so a
        // broken tree cannot recurse for ever.
        if depth > 4096 {
            return None;
        }
        Some(NodeSpec {
            name: node.base.name.clone(),
            placement: node.base.placement,
            opacity: node.base.opacity,
            blend: node.base.blend,
            visible: node.base.visible,
            locked: node.base.locked,
            kind: node.kind.clone(),
            children: node
                .children
                .iter()
                .filter_map(|child| build(page, *child, depth + 1))
                .collect(),
        })
    }
    build(page, id, 0)
}

/// `spec` with every placement in it moved by `dx, dy`.
pub(crate) fn translated(spec: &NodeSpec, dx: f64, dy: f64) -> NodeSpec {
    let mut out = spec.clone();
    out.placement.x += dx;
    out.placement.y += dy;
    out.children = spec
        .children
        .iter()
        .map(|child| translated(child, dx, dy))
        .collect();
    out
}

/// How many nodes `spec` holds, itself included.
fn size(spec: &NodeSpec) -> u64 {
    1 + spec.children.iter().map(size).sum::<u64>()
}

/// The ids `InsertNodes` gives the roots of `specs` when its first id is
/// `first`: it numbers every node in preorder.
pub(crate) fn top_ids(first: u64, specs: &[NodeSpec]) -> Vec<NodeId> {
    let mut next = first;
    specs
        .iter()
        .map(|spec| {
            let id = NodeId::new(next);
            next += size(spec);
            id
        })
        .collect()
}

/// Every image asset `specs` show, each once, in order.
pub(crate) fn assets(specs: &[NodeSpec]) -> Vec<gafira_doc::AssetRef> {
    fn visit(spec: &NodeSpec, out: &mut Vec<gafira_doc::AssetRef>) {
        if let NodeKind::Image(image) = &spec.kind {
            // What it can go back to comes too (spec 084).
            for asset in image.assets() {
                if !out.contains(&asset) {
                    out.push(asset);
                }
            }
        }
        for child in &spec.children {
            visit(child, out);
        }
    }
    let mut out = Vec::new();
    for spec in specs {
        visit(spec, &mut out);
    }
    out
}

#[cfg(test)]
mod tests;
