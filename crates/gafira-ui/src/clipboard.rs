//! Duplicate, copy, cut and paste (spec 043; FD-13, 03 §9.1, §9.2). Pure,
//! so it is tested without a window: the editor turns the results into
//! commands and keeps the clipboard.
//!
//! The clipboard holds node subtrees without ids, as `InsertNodes` takes
//! them, and the asset table entries of the images in them. Image bytes
//! are not copied: they live once in the content store of the session,
//! by their hash, so a paste in another document of the same session only
//! needs the entry.

use std::collections::BTreeSet;

use gafira_doc::msg::MessageKey;
use gafira_doc::{AssetEntry, AssetRef, Document, NodeId, NodeKind, Page, PageId};
use gafira_edit::{Command, InsertAsset, InsertNodes, NodeSpec, Transaction};
use gafira_layout::Rect;

use crate::group;

/// How far a duplicate, or a paste over its own original, moves from it,
/// in page px, right and down (spec 043).
pub(crate) const COPY_OFFSET: f64 = 10.0;

/// What Copiar or Cortar took: subtrees back to front, the entries of the
/// images in them, and the plain text put on the system clipboard.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Clip {
    pub(crate) specs: Vec<NodeSpec>,
    pub(crate) assets: Vec<(AssetRef, AssetEntry)>,
    /// What went to the system clipboard; a paste whose text is this one
    /// pastes the clip (spec 043).
    pub(crate) text: String,
}

/// `nodes` of `page` in document order, each subtree once: a node below
/// another listed one travels with it.
pub(crate) fn tops(page: &Page, nodes: &[NodeId]) -> Vec<NodeId> {
    let listed: BTreeSet<NodeId> = nodes.iter().copied().collect();
    let parents = group::parents(page);
    let above = |id: NodeId| {
        let mut at = parents.get(&id).copied().flatten();
        // At most one step per node, so a broken tree cannot loop.
        for _ in 0..parents.len() {
            let Some(parent) = at else { return false };
            if listed.contains(&parent) {
                return true;
            }
            at = parents.get(&parent).copied().flatten();
        }
        false
    };
    let mut seen = BTreeSet::new();
    let mut order = Vec::new();
    let mut stack: Vec<NodeId> = page.roots.iter().rev().copied().collect();
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        if listed.contains(&id) && !above(id) {
            order.push(id);
        }
        stack.extend(page.node(id).iter().flat_map(|n| n.children.iter().rev()));
    }
    order
}

/// The clip of `nodes` of `page` in `document`; `None` with nothing to copy.
pub(crate) fn copy(document: &Document, page: PageId, nodes: &[NodeId]) -> Option<Clip> {
    let page = document.page(page)?;
    let specs: Vec<NodeSpec> = tops(page, nodes)
        .into_iter()
        .filter_map(|id| group::spec_of(page, id))
        .collect();
    if specs.is_empty() {
        return None;
    }
    let assets = group::assets(&specs)
        .into_iter()
        .filter_map(|asset| Some((asset, document.asset(&asset)?.clone())))
        .collect();
    Some(Clip {
        specs,
        assets,
        text: String::new(),
    })
}

/// The words of the texts in `spec`, one per line.
fn words(spec: &NodeSpec, out: &mut Vec<String>) {
    if let NodeKind::Text(text) = &spec.kind {
        out.push(text.content.plain_text());
    }
    for child in &spec.children {
        words(child, out);
    }
}

/// The plain text of `specs` for the system clipboard: the words of each
/// text, or else the element's name or the `label` of its kind, one line
/// per element.
pub(crate) fn plain_text(specs: &[NodeSpec], label: impl Fn(&NodeKind) -> String) -> String {
    let lines: Vec<String> = specs
        .iter()
        .map(|spec| {
            let mut found = Vec::new();
            words(spec, &mut found);
            if found.is_empty() {
                spec.name.clone().unwrap_or_else(|| label(&spec.kind))
            } else {
                found.join("\n")
            }
        })
        .collect();
    lines.join("\n")
}

/// The list `parent` owns on `page` (`None` for the roots).
fn list(page: &Page, parent: Option<NodeId>) -> Option<&Vec<NodeId>> {
    match parent {
        None => Some(&page.roots),
        Some(group) => page.node(group).map(|node| &node.children),
    }
}

/// The one-step command that duplicates `nodes` of `page` (FD-13): copies
/// moved by [`COPY_OFFSET`] just above the frontmost of them, the top ones
/// named by `rename` when they have a name. With the ids of the copies;
/// `next` is the document's next id.
pub(crate) fn duplicate_command(
    page: &Page,
    nodes: &[NodeId],
    next: u64,
    rename: impl Fn(&str) -> String,
) -> Option<(Box<dyn Command>, Vec<NodeId>)> {
    let tops = tops(page, nodes);
    let front = *tops.last()?;
    let parent = *group::parents(page).get(&front)?;
    let index = list(page, parent)?.iter().position(|id| *id == front)? + 1;
    let specs: Vec<NodeSpec> = tops
        .iter()
        .filter_map(|id| group::spec_of(page, *id))
        .map(|spec| {
            let mut copy = group::translated(&spec, COPY_OFFSET, COPY_OFFSET);
            copy.name = spec.name.as_deref().map(&rename);
            copy
        })
        .collect();
    let ids = group::top_ids(next, &specs);
    Some((
        Box::new(InsertNodes::new(page.id, parent, index, specs)),
        ids,
    ))
}

/// The page box of `spec` as it would be drawn: a turned element by the
/// box around its turned outline, a group by the box around its children.
fn spec_box(spec: &NodeSpec) -> Option<Rect> {
    if matches!(spec.kind, NodeKind::Group(_)) && !spec.children.is_empty() {
        return spec
            .children
            .iter()
            .filter_map(spec_box)
            .reduce(|all, b| all.union(b));
    }
    let p = spec.placement;
    let b = p
        .to_transform()
        .transform_rect_bbox(Rect::new(0.0, 0.0, p.w, p.h));
    [b.x0, b.y0, b.x1, b.y1]
        .iter()
        .all(|v| v.is_finite())
        .then_some(b)
}

/// Whether two boxes are the same within a hundredth of a px.
fn same(a: Rect, b: Rect) -> bool {
    [a.x0 - b.x0, a.y0 - b.y0, a.x1 - b.x1, a.y1 - b.y1]
        .iter()
        .all(|d| d.abs() < 0.01)
}

/// How far to move `clip` to paste it into `parent` (`None` for the
/// roots) of `page`, whose size is `size`: nothing when it lands on the
/// page and not exactly over an element there; onto the middle of the page
/// when it would land outside it; then [`COPY_OFFSET`] more at a time while
/// one pasted element would cover one of the same box.
pub(crate) fn paste_offset(
    clip: &Clip,
    page: &Page,
    size: (f64, f64),
    parent: Option<NodeId>,
) -> (f64, f64) {
    let boxes: Vec<Rect> = clip.specs.iter().filter_map(spec_box).collect();
    let Some(all) = boxes.iter().copied().reduce(|all, b| all.union(b)) else {
        return (0.0, 0.0);
    };
    let sheet = Rect::new(0.0, 0.0, size.0, size.1);
    let (mut dx, mut dy) = if all.intersect(sheet).area() > 0.0 {
        (0.0, 0.0)
    } else {
        let (mid, at) = (sheet.center(), all.center());
        (mid.x - at.x, mid.y - at.y)
    };
    let taken: Vec<Rect> = list(page, parent)
        .map(|ids| {
            ids.iter()
                .filter_map(|id| gafira_layout::node_bounds(page, *id).ok())
                .collect()
        })
        .unwrap_or_default();
    // Bounded, so a page full of copies cannot hold the frame.
    for _ in 0..1000 {
        let covers = boxes.iter().any(|b| {
            let moved = Rect::new(b.x0 + dx, b.y0 + dy, b.x1 + dx, b.y1 + dy);
            taken.iter().any(|t| same(*t, moved))
        });
        if !covers {
            break;
        }
        dx += COPY_OFFSET;
        dy += COPY_OFFSET;
    }
    (dx, dy)
}

/// The one-step command that pastes `clip` on top of `parent` of `page`
/// in `document`, with the entries of its images the document lacks. With
/// the ids of the pasted elements.
pub(crate) fn paste_command(
    document: &Document,
    page: PageId,
    parent: Option<NodeId>,
    clip: &Clip,
) -> Option<(Box<dyn Command>, Vec<NodeId>)> {
    let target = document.page(page)?;
    let size = target.size.to_px(document.dpi()).ok()?;
    let index = list(target, parent)?.len();
    let (dx, dy) = paste_offset(clip, target, (size.width(), size.height()), parent);
    let specs: Vec<NodeSpec> = clip
        .specs
        .iter()
        .map(|spec| group::translated(spec, dx, dy))
        .collect();
    let ids = group::top_ids(document.next_id(), &specs);
    let mut commands: Vec<Box<dyn Command>> = clip
        .assets
        .iter()
        .filter(|(asset, _)| document.asset(asset).is_none())
        .map(|(asset, entry)| Box::new(InsertAsset::new(*asset, entry.clone())) as Box<dyn Command>)
        .collect();
    commands.push(Box::new(InsertNodes::new(page, parent, index, specs)));
    let command: Box<dyn Command> = if commands.len() == 1 {
        commands.pop()?
    } else {
        Box::new(Transaction::new(MessageKey("edit.paste"), commands))
    };
    Some((command, ids))
}

#[cfg(test)]
mod tests;
