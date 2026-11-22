//! The Capas tab and the lock rule of the canvas (spec 048; FD-12, FD-13,
//! D-91): what is locked, what the lock and hide shortcuts set, how a typed
//! name is stored and where a dragged row lands. Pure, so it is tested
//! without a window: the editor turns the results into commands.

use std::collections::BTreeSet;

use gafira_doc::{NodeId, Page};

use crate::group;

/// Whether `id` is locked on `page`: itself, or a group above it (D-91).
/// A locked element is not picked on the canvas and does not move.
pub(crate) fn locked(page: &Page, id: NodeId) -> bool {
    // A locked page locks everything on it (spec 056).
    let own = |id: NodeId| page.node(id).is_some_and(|node| node.base.locked);
    page.locked || own(id) || ancestors(page, id).into_iter().any(own)
}

/// Whether `id` is hidden on `page`: itself, or a group above it. Its
/// row in Capas is dimmed.
pub(crate) fn hidden(page: &Page, id: NodeId) -> bool {
    let own = |id: NodeId| page.node(id).is_some_and(|node| !node.base.visible);
    own(id) || ancestors(page, id).into_iter().any(own)
}

/// What a click selects on a locked `leaf` of `page` (D-110): the outermost
/// locked element among it and the groups above it, so it can be unlocked
/// from the bar over it. `None` when nothing there is locked, or when the
/// whole page is (it is unlocked from the page bar).
pub(crate) fn locked_pick(page: &Page, leaf: NodeId) -> Option<NodeId> {
    if page.locked {
        return None;
    }
    let own = |id: &NodeId| page.node(*id).is_some_and(|node| node.base.locked);
    std::iter::once(leaf)
        .chain(ancestors(page, leaf))
        .rfind(own)
}

/// The nodes of `nodes` that a move, a nudge or a resize may change: the
/// ones that are not locked, in their order (FD-13 b, D-91).
pub(crate) fn movable(page: &Page, nodes: &[NodeId]) -> Vec<NodeId> {
    nodes
        .iter()
        .copied()
        .filter(|id| !locked(page, *id))
        .collect()
}

/// What the hide and lock shortcuts switch (03 §11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Flag {
    Hidden,
    Locked,
}

/// The value the shortcut sets on every node of `nodes` (S48-03): `true`
/// (hidden or locked) when one of them is still visible or free, `false`
/// when all are already hidden or locked. `None` with no node of `page`.
pub(crate) fn toggled(page: &Page, nodes: &[NodeId], flag: Flag) -> Option<bool> {
    let mut set = false;
    for id in nodes {
        let base = &page.node(*id)?.base;
        set |= match flag {
            Flag::Hidden => base.visible,
            Flag::Locked => !base.locked,
        };
    }
    (!nodes.is_empty()).then_some(set)
}

/// The name a typed text stores, when it changes the current one (S48-05):
/// trimmed, and `None` (the name by kind) when nothing is left. `None` when
/// it is the name the element already has.
pub(crate) fn renamed(current: Option<&str>, typed: &str) -> Option<Option<String>> {
    let typed = typed.trim();
    let next = (!typed.is_empty()).then(|| typed.to_owned());
    (next.as_deref() != current).then_some(next)
}

/// The index a row lands on when it is dropped (S48-06). Rows list the
/// siblings front first; `from` is the row's index among its `siblings`
/// (0 is the back) and `gap` the line between rows it is dropped on, from
/// 0 (above the frontmost) to `siblings` (below the backmost). `None` when
/// it lands where it is or the input is out of range.
pub(crate) fn dropped(from: usize, siblings: usize, gap: usize) -> Option<usize> {
    if from >= siblings || gap > siblings {
        return None;
    }
    // The gap as a place in the list back to front, before the row leaves
    // it; once it has left, the places above it are one lower.
    let place = siblings - gap;
    let index = if place > from { place - 1 } else { place };
    (index != from).then_some(index)
}

/// The ancestors of `id`, nearest first, for a tree that may be broken.
fn ancestors(page: &Page, id: NodeId) -> Vec<NodeId> {
    let parents = group::parents(page);
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    let mut at = id;
    while let Some(Some(parent)) = parents.get(&at) {
        if !seen.insert(*parent) {
            break;
        }
        out.push(*parent);
        at = *parent;
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gafira_doc::{
        ElementBase, GroupNode, Node, NodeKind, PageId, PageSize, Placement, ShapeGeometry,
        ShapeNode,
    };
    use gafira_geom::{Length, Unit};

    use super::*;
    use crate::canvas;

    fn n(id: u64) -> NodeId {
        NodeId::new(id)
    }

    /// A 400 × 400 page: rectangles 1 and 2 at the same place (2 in front),
    /// and group 9 holding rectangle 3, away from them.
    fn page() -> Page {
        let size = PageSize {
            width: Length::new(400.0, Unit::Px),
            height: Length::new(400.0, Unit::Px),
        };
        let mut page = Page::new(PageId::new(1), String::new(), size);
        let rect = |id: u64, x: f64| {
            Node::new(
                ElementBase::new(n(id), Placement::new(x, 10.0, 100.0, 100.0)),
                NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect)),
            )
        };
        page.nodes.insert(n(1), Arc::new(rect(1, 10.0)));
        page.nodes.insert(n(2), Arc::new(rect(2, 10.0)));
        page.nodes.insert(n(3), Arc::new(rect(3, 200.0)));
        let mut group = Node::new(
            ElementBase::new(n(9), Placement::new(200.0, 10.0, 100.0, 100.0)),
            NodeKind::Group(GroupNode {}),
        );
        group.children = vec![n(3)];
        page.nodes.insert(n(9), Arc::new(group));
        page.roots = vec![n(1), n(2), n(9)];
        page
    }

    fn set(page: &mut Page, id: u64, change: impl FnOnce(&mut ElementBase)) {
        let mut node = page.node(n(id)).expect("a node").clone();
        change(&mut node.base);
        page.nodes.insert(n(id), Arc::new(node));
    }

    fn lock(page: &mut Page, id: u64) {
        set(page, id, |base| base.locked = true);
    }

    fn at(page: &Page) -> egui::Rect {
        let _ = page;
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::splat(400.0))
    }

    // S48-04 (D-91): a click on a locked element that covers another picks
    // the one below; with nothing below it picks nothing; a child of a
    // locked group is not picked either.
    #[test]
    fn s48_04_locked_is_not_picked() {
        let mut page = page();
        let pointer = egui::Pos2::new(50.0, 50.0);
        let shapes = canvas::shapes(&page);
        assert_eq!(canvas::hit(&shapes, at(&page), 1.0, pointer), Some(n(2)));
        lock(&mut page, 2);
        let shapes = canvas::shapes(&page);
        assert_eq!(
            canvas::hit(&shapes, at(&page), 1.0, pointer),
            Some(n(1)),
            "the click goes through to what is below"
        );
        lock(&mut page, 1);
        let shapes = canvas::shapes(&page);
        assert_eq!(canvas::hit(&shapes, at(&page), 1.0, pointer), None);
        let inside = egui::Pos2::new(250.0, 50.0);
        assert_eq!(canvas::hit(&shapes, at(&page), 1.0, inside), Some(n(3)));
        lock(&mut page, 9);
        let shapes = canvas::shapes(&page);
        assert_eq!(
            canvas::hit(&shapes, at(&page), 1.0, inside),
            None,
            "under a locked group"
        );
        assert_eq!(shapes.len(), 3, "locked elements are still drawn");
    }

    // D-110: a click on a locked element selects it (or the locked group
    // above it), for the bar to unlock it; nothing locked, nothing picked.
    #[test]
    fn a_click_selects_what_is_locked() {
        let mut page = page();
        assert_eq!(locked_pick(&page, n(2)), None);
        lock(&mut page, 2);
        assert_eq!(locked_pick(&page, n(2)), Some(n(2)));
        lock(&mut page, 9);
        assert_eq!(locked_pick(&page, n(3)), Some(n(9)), "the locked group");
    }

    // S48-04 (D-91): the marquee and Ctrl+A leave out what is locked and
    // what is below a locked group.
    #[test]
    fn s48_04_marquee_and_select_all_skip_locked() {
        let mut page = page();
        let all = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::splat(400.0));
        lock(&mut page, 1);
        let shapes = canvas::shapes(&page);
        assert_eq!(
            canvas::in_marquee(&shapes, at(&page), 1.0, all),
            [n(2), n(3)]
        );
        assert_eq!(group::items(&page, None), [n(2), n(9)]);
        lock(&mut page, 9);
        let shapes = canvas::shapes(&page);
        assert_eq!(canvas::in_marquee(&shapes, at(&page), 1.0, all), [n(2)]);
        assert_eq!(group::items(&page, None), [n(2)]);
        assert!(group::items(&page, Some(n(9))).is_empty());
    }

    // S56-05: on a locked page every element is locked: not picked, not
    // in Ctrl+A, not moved.
    #[test]
    fn s56_05_a_locked_page_locks_its_elements() {
        let mut page = page();
        page.locked = true;
        assert!(locked(&page, n(2)) && locked(&page, n(3)));
        assert!(movable(&page, &[n(1), n(2)]).is_empty());
        let shapes = canvas::shapes(&page);
        assert_eq!(
            canvas::hit(&shapes, at(&page), 1.0, egui::Pos2::new(50.0, 50.0)),
            None
        );
        assert!(group::items(&page, None).is_empty());
    }

    // S48-04 (D-91): a selection that mixes locked and free elements only
    // moves the free ones; one under a locked group is locked too.
    #[test]
    fn s48_04_locked_does_not_move() {
        let mut page = page();
        lock(&mut page, 1);
        assert!(locked(&page, n(1)));
        assert!(!locked(&page, n(2)));
        assert_eq!(movable(&page, &[n(1), n(2), n(3)]), [n(2), n(3)]);
        lock(&mut page, 9);
        assert!(locked(&page, n(3)), "below a locked group");
        assert_eq!(movable(&page, &[n(1), n(3)]), Vec::<NodeId>::new());
    }

    // S48-07: a row is dimmed when it, or a group above it, is hidden or
    // locked.
    #[test]
    fn s48_07_dimmed_rows() {
        let mut page = page();
        assert!(!hidden(&page, n(3)) && !locked(&page, n(3)));
        set(&mut page, 9, |base| base.visible = false);
        assert!(hidden(&page, n(9)) && hidden(&page, n(3)));
        assert!(!hidden(&page, n(1)));
        assert!(!locked(&page, n(3)), "hidden is not locked");
    }

    // S48-03: with a mixed selection the shortcut hides or locks them all;
    // with all hidden or locked it shows or frees them all.
    #[test]
    fn s48_03_toggle_rule() {
        let mut page = page();
        let both = [n(1), n(2)];
        assert_eq!(toggled(&page, &both, Flag::Hidden), Some(true));
        assert_eq!(toggled(&page, &both, Flag::Locked), Some(true));
        lock(&mut page, 1);
        set(&mut page, 1, |base| base.visible = false);
        assert_eq!(toggled(&page, &both, Flag::Hidden), Some(true), "mixed");
        assert_eq!(toggled(&page, &both, Flag::Locked), Some(true), "mixed");
        lock(&mut page, 2);
        set(&mut page, 2, |base| base.visible = false);
        assert_eq!(toggled(&page, &both, Flag::Hidden), Some(false));
        assert_eq!(toggled(&page, &both, Flag::Locked), Some(false));
        assert_eq!(toggled(&page, &[], Flag::Locked), None);
        assert_eq!(toggled(&page, &[n(77)], Flag::Hidden), None);
    }

    // S48-05: a typed name is trimmed; empty or only spaces stores `None`;
    // the same name gives no change.
    #[test]
    fn s48_05_name_normalised() {
        assert_eq!(renamed(None, "  Logo "), Some(Some("Logo".to_owned())));
        assert_eq!(renamed(Some("Logo"), ""), Some(None));
        assert_eq!(renamed(Some("Logo"), "   "), Some(None));
        assert_eq!(renamed(Some("Logo"), " Logo"), None);
        assert_eq!(renamed(None, " "), None);
        assert_eq!(
            renamed(Some("Logo"), "Marca"),
            Some(Some("Marca".to_owned()))
        );
    }

    // S48-06: rows list four siblings front first; the back one (index 0)
    // dropped above the frontmost lands on 3 and below it on 2; dropped
    // next to itself it does not move; nothing lands out of range.
    #[test]
    fn s48_06_drop_index() {
        assert_eq!(dropped(0, 4, 0), Some(3));
        assert_eq!(dropped(0, 4, 1), Some(2));
        assert_eq!(dropped(0, 4, 3), None, "its own gap above it");
        assert_eq!(dropped(0, 4, 4), None, "its own gap below it");
        assert_eq!(dropped(3, 4, 4), Some(0));
        assert_eq!(dropped(3, 4, 0), None);
        assert_eq!(dropped(3, 4, 1), None);
        assert_eq!(dropped(2, 4, 0), Some(3));
        assert_eq!(dropped(1, 4, 3), None);
        assert_eq!(dropped(1, 4, 4), Some(0));
        assert_eq!(dropped(4, 4, 0), None, "no such row");
        assert_eq!(dropped(0, 4, 5), None, "no such gap");
        for from in 0..4 {
            for gap in 0..=4 {
                if let Some(index) = dropped(from, 4, gap) {
                    assert!(index < 4);
                }
            }
        }
    }
}
