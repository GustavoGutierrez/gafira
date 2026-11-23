//! Align, distribute and layer order of the selection (spec 042; FD-11,
//! FD-12, D-17, D-28, D-29). Pure, so it is tested without a window: the
//! editor turns the results into commands.
//!
//! Align and distribute read every element by its axis-aligned page box
//! (`gafira_layout::node_bounds`, so a turned element counts by the box
//! around its turned outline, D-28) and leave the maths to `gafira-layout`.
//! Moving a placement by an offset moves that box by the same offset, so
//! the new placement is the old one with `x` and `y` shifted.

use gafira_doc::{NodeId, Page, Placement};
use gafira_layout::{AlignAction, DistributeAxis, DistributeMode, Item, Offset, Rect};

use crate::group;

/// The four layer-order actions of FD-12, over one sibling list of a page
/// (index 0 is the back): the roots, or the children of a group (spec 043).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Order {
    /// «Al frente»: the selection on top of everything.
    Front,
    /// «Traer adelante»: one step towards the front.
    Forward,
    /// «Enviar atrás»: one step towards the back.
    Backward,
    /// «Al fondo»: the selection below everything.
    Back,
}

/// The items `gafira-layout` reads for `nodes`: each page box and whether
/// the element is locked. `None` when a node is missing or its box is
/// invalid.
fn items(page: &Page, nodes: &[NodeId]) -> Option<Vec<Item>> {
    nodes
        .iter()
        .map(|id| {
            let node = page.node(*id)?;
            let rect = gafira_layout::node_bounds(page, *id).ok()?;
            Some(Item {
                rect,
                locked: node.base.locked,
            })
        })
        .collect()
}

/// The placements that change when `offsets` move `nodes`; a node that
/// stays is left out. A group moves with every node below it (spec 043).
fn shifted(page: &Page, nodes: &[NodeId], offsets: &[Offset]) -> Vec<(NodeId, Placement)> {
    nodes
        .iter()
        .zip(offsets)
        .filter(|(_, offset)| offset.dx != 0.0 || offset.dy != 0.0)
        .flat_map(|(id, offset)| {
            if group::is_group(page, *id) {
                let change = group::Change::Move {
                    dx: offset.dx,
                    dy: offset.dy,
                };
                return group::changed(page, *id, change);
            }
            let Some(node) = page.node(*id) else {
                return Vec::new();
            };
            let mut placement = node.base.placement;
            placement.x += offset.dx;
            placement.y += offset.dy;
            vec![(*id, placement)]
        })
        .collect()
}

/// The new placements that align `nodes` of `page` by `action` (FD-11):
/// one element to the page box `0, 0, width, height`, two or more to the
/// box around all of them, locked ones included (D-29); locked elements do
/// not move. Only the elements that move are returned, so nothing to do is
/// an empty list; so is a node that is missing or a box that is not valid.
pub(crate) fn aligned(
    page: &Page,
    size: (f64, f64),
    nodes: &[NodeId],
    action: AlignAction,
) -> Vec<(NodeId, Placement)> {
    let Some(items) = items(page, nodes) else {
        return Vec::new();
    };
    let bounds = Rect::new(0.0, 0.0, size.0, size.1);
    match gafira_layout::align(action, bounds, &items) {
        Ok(offsets) => shifted(page, nodes, &offsets),
        Err(_) => Vec::new(),
    }
}

/// The new placements that spread the free elements of `nodes` along
/// `axis` with equal gaps (FD-11, D-17): the first and the last stay, and
/// with fewer than three free elements nothing moves.
pub(crate) fn distributed(
    page: &Page,
    nodes: &[NodeId],
    axis: DistributeAxis,
) -> Vec<(NodeId, Placement)> {
    let Some(items) = items(page, nodes) else {
        return Vec::new();
    };
    match gafira_layout::distribute(axis, DistributeMode::EqualGap, &items) {
        Ok(offsets) => shifted(page, nodes, &offsets),
        Err(_) => Vec::new(),
    }
}

/// `roots` (back first) after `order` on the `selected` ones, which keep
/// their order among themselves; ids of `selected` that are not roots are
/// ignored. `Forward` moves each selected run one place above the next
/// element that is not selected; `Backward` one place below the previous
/// one.
pub(crate) fn reordered(roots: &[NodeId], selected: &[NodeId], order: Order) -> Vec<NodeId> {
    let picked = |id: &NodeId| selected.contains(id);
    let mut list = roots.to_vec();
    match order {
        Order::Front | Order::Back => {
            let (mut chosen, rest): (Vec<NodeId>, Vec<NodeId>) = list.into_iter().partition(picked);
            if order == Order::Front {
                let mut all = rest;
                all.append(&mut chosen);
                all
            } else {
                chosen.extend(rest);
                chosen
            }
        }
        // From the front down, so a run moves as one: its front element
        // passes the neighbour first, and the next one passes it again.
        Order::Forward => {
            for i in (0..list.len().saturating_sub(1)).rev() {
                if picked(&list[i]) && !picked(&list[i + 1]) {
                    list.swap(i, i + 1);
                }
            }
            list
        }
        Order::Backward => {
            for i in 1..list.len() {
                if picked(&list[i]) && !picked(&list[i - 1]) {
                    list.swap(i, i - 1);
                }
            }
            list
        }
    }
}

/// The `Reorder` steps, `(node, index)` each, that turn `from` into `to`,
/// a permutation of it: applied in order, each moves `node` to `index`
/// among the roots as they are after the steps before it.
pub(crate) fn steps(from: &[NodeId], to: &[NodeId]) -> Vec<(NodeId, usize)> {
    let mut list = from.to_vec();
    let mut out = Vec::new();
    for (index, node) in to.iter().enumerate() {
        let Some(at) = list.iter().position(|id| id == node) else {
            continue;
        };
        if at != index {
            list.remove(at);
            list.insert(index, *node);
            out.push((*node, index));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gafira_doc::{ElementBase, Node, NodeKind, PageId, PageSize, ShapeGeometry, ShapeNode};
    use gafira_geom::{Length, Unit};

    use super::*;

    fn n(id: u64) -> NodeId {
        NodeId::new(id)
    }

    /// A 1000 × 800 page with one rectangle per placement, ids from 1, in
    /// root order; the ids in `locked` are locked.
    fn page(placements: &[Placement], locked: &[u64]) -> Page {
        let size = PageSize {
            width: Length::new(1000.0, Unit::Px),
            height: Length::new(800.0, Unit::Px),
        };
        let mut page = Page::new(PageId::new(1), String::new(), size);
        for (i, at) in placements.iter().enumerate() {
            let id = i as u64 + 1;
            let mut base = ElementBase::new(n(id), *at);
            base.locked = locked.contains(&id);
            page.nodes.insert(
                n(id),
                Arc::new(Node::new(
                    base,
                    NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect)),
                )),
            );
            page.roots.push(n(id));
        }
        page
    }

    fn place(x: f64, y: f64, w: f64, h: f64) -> Placement {
        Placement::new(x, y, w, h)
    }

    const SIZE: (f64, f64) = (1000.0, 800.0);

    /// The page box of `node` after `moves`.
    fn box_after(page: &Page, moves: &[(NodeId, Placement)], node: NodeId) -> Rect {
        let mut page = page.clone();
        for (id, to) in moves {
            let mut changed = page.node(*id).expect("a node").clone();
            changed.base.placement = *to;
            page.nodes.insert(*id, Arc::new(changed));
        }
        gafira_layout::node_bounds(&page, node).expect("a box")
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.01
    }

    // S42-01: two or more align to the box around the selection; only one
    // axis changes; an element already in place is not returned.
    #[test]
    fn s42_01_align_to_the_selection_box_on_one_axis() {
        let page = page(
            &[
                place(100.0, 50.0, 40.0, 30.0),
                place(300.0, 200.0, 60.0, 20.0),
                place(200.0, 400.0, 10.0, 10.0),
            ],
            &[],
        );
        let all = [n(1), n(2), n(3)];
        let left = aligned(&page, SIZE, &all, AlignAction::Left);
        assert_eq!(
            left.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            [n(2), n(3)],
            "the leftmost does not move"
        );
        for id in all {
            let b = box_after(&page, &left, id);
            assert!(close(b.x0, 100.0), "{id:?} {b:?}");
        }
        for (id, to) in &left {
            let from = page.node(*id).expect("a node").base.placement;
            assert_eq!(to.y, from.y, "align left keeps y");
            assert_eq!((to.w, to.h), (from.w, from.h));
        }
        // The box of the three is x 100..360, y 50..410.
        type Edge = fn(Rect) -> f64;
        let checks: [(AlignAction, Edge, f64); 5] = [
            (AlignAction::Center, |b| (b.x0 + b.x1) / 2.0, 230.0),
            (AlignAction::Right, |b| b.x1, 360.0),
            (AlignAction::Top, |b| b.y0, 50.0),
            (AlignAction::Middle, |b| (b.y0 + b.y1) / 2.0, 230.0),
            (AlignAction::Bottom, |b| b.y1, 410.0),
        ];
        for (action, edge, want) in checks {
            let moves = aligned(&page, SIZE, &all, action);
            for id in all {
                let got = edge(box_after(&page, &moves, id));
                assert!(close(got, want), "{action:?} {id:?}: {got}");
            }
            for (id, to) in &moves {
                let from = page.node(*id).expect("a node").base.placement;
                let horizontal = matches!(
                    action,
                    AlignAction::Left | AlignAction::Center | AlignAction::Right
                );
                if horizontal {
                    assert_eq!(to.y, from.y, "{action:?}");
                } else {
                    assert_eq!(to.x, from.x, "{action:?}");
                }
            }
        }
    }

    // S42-02: one element aligns to the page.
    // S43-12: aligning a group moves it with every node below it, by its
    // box around them.
    #[test]
    fn s43_12_a_group_aligns_with_its_children() {
        let mut page = page(
            &[
                place(100.0, 50.0, 40.0, 30.0),
                place(300.0, 200.0, 60.0, 20.0),
                place(500.0, 400.0, 10.0, 10.0),
            ],
            &[],
        );
        let mut group = Node::new(
            ElementBase::new(n(4), place(0.0, 0.0, 1.0, 1.0)),
            NodeKind::Group(gafira_doc::GroupNode {}),
        );
        group.children = vec![n(2), n(3)];
        page.nodes.insert(n(4), Arc::new(group));
        page.roots = vec![n(1), n(4)];
        let moves = aligned(&page, SIZE, &[n(1), n(4)], AlignAction::Left);
        let moved: Vec<NodeId> = moves.iter().map(|(id, _)| *id).collect();
        assert_eq!(moved, [n(4), n(2), n(3)]);
        for id in [n(2), n(3)] {
            let from = page.node(id).expect("a node").base.placement;
            let to = moves.iter().find(|(m, _)| *m == id).expect("moved").1;
            assert!(close(to.x - from.x, -200.0), "{id:?}");
            assert_eq!(to.y, from.y);
        }
        assert!(close(box_after(&page, &moves, n(4)).x0, 100.0));
    }

    #[test]
    fn s42_02_one_element_aligns_to_the_page() {
        let page = page(&[place(100.0, 50.0, 40.0, 30.0)], &[]);
        let one = [n(1)];
        let checks: [(AlignAction, f64, f64); 6] = [
            (AlignAction::Left, 0.0, 50.0),
            (AlignAction::Center, 480.0, 50.0),
            (AlignAction::Right, 960.0, 50.0),
            (AlignAction::Top, 100.0, 0.0),
            (AlignAction::Middle, 100.0, 385.0),
            (AlignAction::Bottom, 100.0, 770.0),
        ];
        for (action, x, y) in checks {
            let moves = aligned(&page, SIZE, &one, action);
            assert_eq!(moves.len(), 1, "{action:?}");
            assert_eq!((moves[0].1.x, moves[0].1.y), (x, y), "{action:?}");
        }
    }

    // S42-03: a turned element aligns by the axis-aligned box of its turned
    // outline (D-28), and keeps its rotation.
    #[test]
    fn s42_03_a_turned_element_aligns_by_its_axis_aligned_box() {
        let mut turned = place(200.0, 200.0, 100.0, 100.0);
        turned.rotation_deg = 45.0;
        let page = page(&[place(50.0, 0.0, 10.0, 10.0), turned], &[]);
        let moves = aligned(&page, SIZE, &[n(1), n(2)], AlignAction::Left);
        assert_eq!(moves.len(), 1, "the upright one is the leftmost");
        let (id, to) = moves[0];
        assert_eq!(id, n(2));
        assert_eq!(to.rotation_deg, 45.0);
        let b = box_after(&page, &moves, n(2));
        assert!(close(b.x0, 50.0), "{b:?}");
        // The turned square reaches 50·√2 out of its centre: its box starts
        // 20.71 px left of its own x.
        assert!(close(to.x, 50.0 + 50.0 * 2f64.sqrt() - 50.0), "{to:?}");

        // On its own, against the page.
        let alone = aligned(&page, SIZE, &[n(2)], AlignAction::Right);
        let b = box_after(&page, &alone, n(2));
        assert!(close(b.x1, 1000.0), "{b:?}");
    }

    // S42-04: locked elements count in the selection box but do not move
    // (D-29); a single locked element, or nothing selected, changes nothing.
    #[test]
    fn s42_04_locked_elements_count_but_stay() {
        let page = page(
            &[place(10.0, 0.0, 10.0, 10.0), place(300.0, 0.0, 10.0, 10.0)],
            &[1],
        );
        let moves = aligned(&page, SIZE, &[n(1), n(2)], AlignAction::Left);
        assert_eq!(moves, [(n(2), place(10.0, 0.0, 10.0, 10.0))]);
        assert!(aligned(&page, SIZE, &[n(1)], AlignAction::Right).is_empty());
        assert!(aligned(&page, SIZE, &[], AlignAction::Left).is_empty());
        assert!(
            aligned(&page, SIZE, &[n(1), n(99)], AlignAction::Left).is_empty(),
            "a missing node aligns nothing"
        );
    }

    // S42-05: distribute leaves equal gaps between the boxes; the first and
    // the last stay; the order is by position, not by selection.
    #[test]
    fn s42_05_distribute_leaves_equal_gaps() {
        let page = page(
            &[
                place(0.0, 0.0, 100.0, 10.0),
                place(500.0, 30.0, 50.0, 10.0),
                place(150.0, 60.0, 20.0, 10.0),
                place(600.0, 90.0, 100.0, 10.0),
            ],
            &[],
        );
        let all = [n(4), n(2), n(1), n(3)];
        let moves = distributed(&page, &all, DistributeAxis::Horizontal);
        assert!(moves.iter().all(|(id, _)| *id != n(1) && *id != n(4)));
        let mut boxes: Vec<Rect> = all.iter().map(|id| box_after(&page, &moves, *id)).collect();
        boxes.sort_by(|a, b| a.x0.total_cmp(&b.x0));
        // Span 0..700, sizes 270: three gaps of 430 / 3.
        let gaps: Vec<f64> = boxes.windows(2).map(|w| w[1].x0 - w[0].x1).collect();
        for gap in &gaps {
            assert!(close(*gap, 430.0 / 3.0), "{gaps:?}");
        }
        for (id, to) in &moves {
            let from = page.node(*id).expect("a node").base.placement;
            assert_eq!(to.y, from.y, "horizontal keeps y");
        }

        let vertical = distributed(&page, &all, DistributeAxis::Vertical);
        assert!(vertical.is_empty(), "already 20 px apart");
    }

    // S42-06: fewer than three free elements distribute nothing; a locked
    // one stays and is left out (D-17); a tight span overlaps evenly.
    #[test]
    fn s42_06_distribute_needs_three_free_elements() {
        let row = [
            place(0.0, 0.0, 100.0, 10.0),
            place(400.0, 0.0, 100.0, 10.0),
            place(500.0, 0.0, 100.0, 10.0),
            place(900.0, 0.0, 100.0, 10.0),
        ];
        let three_and_locked = page(&row, &[2]);
        let all = [n(1), n(2), n(3), n(4)];
        let moves = distributed(&three_and_locked, &all, DistributeAxis::Horizontal);
        assert_eq!(moves.len(), 1, "only the middle free one moves");
        assert_eq!(moves[0].0, n(3));
        assert_eq!(moves[0].1.x, 450.0);
        let two_free = page(&row[..3], &[2]);
        assert!(distributed(&two_free, &[n(1), n(2), n(3)], DistributeAxis::Horizontal).is_empty());
        let free = page(&row, &[]);
        assert!(distributed(&free, &[n(1), n(2)], DistributeAxis::Horizontal).is_empty());
        // FD-11 (e): three boxes of 100 over a 250 span overlap by 25.
        let tight = page(
            &[
                place(0.0, 0.0, 100.0, 10.0),
                place(10.0, 0.0, 100.0, 10.0),
                place(150.0, 0.0, 100.0, 10.0),
            ],
            &[],
        );
        let moves = distributed(&tight, &[n(1), n(2), n(3)], DistributeAxis::Horizontal);
        assert_eq!(moves, [(n(2), place(75.0, 0.0, 100.0, 10.0))]);
    }

    // S42-07: a turned element distributes by its axis-aligned box (D-28).
    #[test]
    fn s42_07_a_turned_element_distributes_by_its_box() {
        let mut turned = place(200.0, 0.0, 100.0, 20.0);
        turned.rotation_deg = 90.0;
        let page = page(
            &[
                place(0.0, 0.0, 100.0, 10.0),
                turned,
                place(600.0, 0.0, 100.0, 10.0),
            ],
            &[],
        );
        let moves = distributed(&page, &[n(1), n(2), n(3)], DistributeAxis::Horizontal);
        let b = box_after(&page, &moves, n(2));
        // Turned 90°, it is 20 px wide: 480 px of gap split in two.
        assert!(close(b.x1 - b.x0, 20.0), "{b:?}");
        assert!(close(b.x0, 340.0), "{b:?}");
        assert_eq!(moves[0].1.rotation_deg, 90.0);
    }

    // S42-08: Al frente and Al fondo move the whole selection, in its order.
    #[test]
    fn s42_08_front_and_back_keep_the_relative_order() {
        let roots = [n(1), n(2), n(3), n(4), n(5)];
        // The selection order does not matter: the layers keep theirs.
        let selected = [n(4), n(2)];
        assert_eq!(
            reordered(&roots, &selected, Order::Front),
            [n(1), n(3), n(5), n(2), n(4)]
        );
        assert_eq!(
            reordered(&roots, &selected, Order::Back),
            [n(2), n(4), n(1), n(3), n(5)]
        );
        assert_eq!(reordered(&roots, &[], Order::Front), roots);
        assert_eq!(reordered(&roots, &[n(9)], Order::Back), roots, "not a root");
    }

    // S42-09: Traer adelante and Enviar atrás move every selected run one
    // step past its unselected neighbour; at the edge a run stays.
    #[test]
    fn s42_09_forward_and_backward_move_one_step() {
        let roots = [n(1), n(2), n(3), n(4), n(5)];
        assert_eq!(
            reordered(&roots, &[n(2)], Order::Forward),
            [n(1), n(3), n(2), n(4), n(5)]
        );
        assert_eq!(
            reordered(&roots, &[n(1), n(2)], Order::Forward),
            [n(3), n(1), n(2), n(4), n(5)],
            "a run moves as one"
        );
        assert_eq!(
            reordered(&roots, &[n(2), n(4)], Order::Forward),
            [n(1), n(3), n(2), n(5), n(4)]
        );
        assert_eq!(
            reordered(&roots, &[n(4), n(5)], Order::Forward),
            roots,
            "already in front"
        );
        assert_eq!(
            reordered(&roots, &[n(3), n(5)], Order::Forward),
            [n(1), n(2), n(4), n(3), n(5)],
            "the front one stays, the other passes its neighbour"
        );
        assert_eq!(
            reordered(&roots, &[n(4)], Order::Backward),
            [n(1), n(2), n(4), n(3), n(5)]
        );
        assert_eq!(
            reordered(&roots, &[n(4), n(5)], Order::Backward),
            [n(1), n(2), n(4), n(5), n(3)]
        );
        assert_eq!(
            reordered(&roots, &[n(1), n(3)], Order::Backward),
            [n(1), n(3), n(2), n(4), n(5)]
        );
        assert_eq!(reordered(&roots, &[n(1)], Order::Backward), roots);
        assert_eq!(
            reordered(&roots, &[n(1), n(2)], Order::Backward),
            roots,
            "a run at the back stays"
        );
    }

    // S42-10: the steps, applied one after the other as `Reorder` does,
    // turn the roots into the new order; no steps for no change.
    #[test]
    fn s42_10_steps_rebuild_the_order() {
        let roots: Vec<NodeId> = (1..=6).map(n).collect();
        let replay = |from: &[NodeId], steps: &[(NodeId, usize)]| {
            let mut list = from.to_vec();
            for (node, index) in steps {
                let at = list.iter().position(|id| id == node).expect("a root");
                list.remove(at);
                list.insert(*index, *node);
            }
            list
        };
        for order in [Order::Front, Order::Forward, Order::Backward, Order::Back] {
            for selected in [vec![n(2)], vec![n(1), n(6)], vec![n(5), n(2), n(3)]] {
                let to = reordered(&roots, &selected, order);
                let steps = steps(&roots, &to);
                assert_eq!(replay(&roots, &steps), to, "{order:?} {selected:?}");
                assert!(steps.len() <= roots.len());
            }
        }
        assert!(steps(&roots, &roots).is_empty());
        let swapped = [n(2), n(1), n(3), n(4), n(5), n(6)];
        assert_eq!(steps(&roots, &swapped).len(), 1);
    }
}
