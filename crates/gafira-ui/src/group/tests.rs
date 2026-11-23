use std::sync::Arc;
use std::time::Duration;

use gafira_doc::{
    Document, Dpi, ElementBase, GroupNode, Node, NodeKind, Page, PageId, PageSize, ShapeGeometry,
    ShapeNode,
};
use gafira_edit::History;
use gafira_geom::{Length, Unit};

use super::*;

const P: PageId = PageId::new(500);

fn n(id: u64) -> NodeId {
    NodeId::new(id)
}

fn place(x: f64, y: f64, w: f64, h: f64) -> Placement {
    Placement::new(x, y, w, h)
}

fn turned(mut p: Placement, degrees: f64) -> Placement {
    p.rotation_deg = degrees;
    p
}

/// A node of a test page: a rectangle, or a group with these children.
enum Make {
    Rect(u64, Placement),
    Group(u64, Placement, Vec<u64>),
}

/// A 1000 × 800 page with `nodes` and these roots, back first.
fn build(nodes: Vec<Make>, roots: &[u64]) -> Page {
    let size = PageSize {
        width: Length::new(1000.0, Unit::Px),
        height: Length::new(800.0, Unit::Px),
    };
    let mut page = Page::new(P, String::new(), size);
    for make in nodes {
        let (id, node) = match make {
            Make::Rect(id, at) => (
                id,
                Node::new(
                    ElementBase::new(n(id), at),
                    NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect)),
                ),
            ),
            Make::Group(id, at, children) => {
                let mut group =
                    Node::new(ElementBase::new(n(id), at), NodeKind::Group(GroupNode {}));
                group.children = children.into_iter().map(n).collect();
                (id, group)
            }
        };
        page.nodes.insert(n(id), Arc::new(node));
    }
    page.roots = roots.iter().copied().map(n).collect();
    page
}

fn history(page: Page) -> History {
    let next = P.get() + 1;
    let doc = Document::from_parts(next, Dpi::new(96), vec![Arc::new(page)]).expect("a valid page");
    History::new(doc).expect("a valid document")
}

fn now(history: &History) -> Page {
    history.document().page(P).expect("the page").clone()
}

fn ids(list: &[NodeId]) -> Vec<u64> {
    list.iter().map(|id| id.get()).collect()
}

fn children(page: &Page, id: u64) -> Vec<u64> {
    ids(&page.node(n(id)).expect("a node").children)
}

fn bits(p: &Placement) -> [u64; 6] {
    [p.x, p.y, p.w, p.h, p.rotation_deg, p.skew].map(f64::to_bits)
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn three() -> Page {
    build(
        vec![
            Make::Rect(1, place(10.0, 10.0, 20.0, 20.0)),
            Make::Rect(2, place(100.0, 50.0, 40.0, 10.0)),
            Make::Rect(3, turned(place(10.0, 20.0, 40.0, 10.0), 90.0)),
        ],
        &[1, 2, 3],
    )
}

// S43-01: the group goes just above the frontmost node, holds the nodes
// back to front, and its box is the one around them, a turned one by the
// box around its turned outline (D-28).
#[test]
fn s43_01_a_group_plan_goes_above_the_frontmost_with_the_box_around() {
    let page = three();
    let plan = group_plan(&page, &[n(3), n(1)]).expect("a plan");
    assert_eq!(plan.parent, None);
    assert_eq!(plan.index, 3, "above node 3, the frontmost");
    assert_eq!(ids(&plan.nodes), [1, 3], "back to front, not as selected");
    // Node 3 turned 90° about (30, 25) covers x 25..35, y 5..45.
    let b = plan.placement;
    assert!(near(b.x, 10.0) && near(b.y, 5.0), "{b:?}");
    assert!(near(b.x + b.w, 35.0) && near(b.y + b.h, 45.0), "{b:?}");
    assert!(rotation::upright(&b));
    assert_eq!(group_plan(&page, &[]), None);
    assert_eq!(group_plan(&page, &[n(99)]), None);
    // Inside a group, the new group stays in it.
    let nested = build(
        vec![
            Make::Rect(1, place(0.0, 0.0, 10.0, 10.0)),
            Make::Rect(2, place(20.0, 0.0, 10.0, 10.0)),
            Make::Rect(3, place(40.0, 0.0, 10.0, 10.0)),
            Make::Group(4, place(0.0, 0.0, 50.0, 10.0), vec![1, 2, 3]),
        ],
        &[4],
    );
    let plan = group_plan(&nested, &[n(1), n(2)]).expect("a plan");
    assert_eq!((plan.parent, plan.index), (Some(n(4)), 2));
}

// S43-02: grouping keeps every placement bit for bit and the relative
// order, and it is one undo step.
#[test]
fn s43_02_group_keeps_places_and_order_in_one_undo() {
    let mut history = history(three());
    let before = now(&history);
    let next = history.document().next_id();
    let (command, group) = group_command(&before, &[n(3), n(1)], next).expect("a command");
    assert_eq!(group, n(next));
    history.execute(command, Duration::ZERO).expect("it groups");
    let after = now(&history);
    assert_eq!(ids(&after.roots), [2, next]);
    assert_eq!(children(&after, next), [1, 3]);
    for id in [1, 2, 3] {
        let was = before.node(n(id)).expect("a node").base.placement;
        let is = after.node(n(id)).expect("a node").base.placement;
        assert_eq!(bits(&was), bits(&is), "node {id} did not move");
    }
    assert_eq!(history.len(), 1, "one undo step");
    history.undo().expect("an undo");
    assert_eq!(now(&history), before);
}

// S43-03: ungrouping puts the children where the group was, in their order
// and at their places, removes the group, and is one undo step; several
// groups ungroup in the same step.
#[test]
fn s43_03_ungroup_restores_the_children_in_place_in_one_undo() {
    let start = build(
        vec![
            Make::Rect(1, place(0.0, 0.0, 10.0, 10.0)),
            Make::Rect(2, turned(place(20.0, 0.0, 10.0, 30.0), 45.0)),
            Make::Rect(3, place(40.0, 0.0, 10.0, 10.0)),
            Make::Group(4, place(0.0, 0.0, 50.0, 30.0), vec![2, 3]),
            Make::Rect(5, place(0.0, 90.0, 10.0, 10.0)),
            Make::Rect(6, place(0.0, 60.0, 10.0, 10.0)),
            Make::Group(7, place(0.0, 60.0, 10.0, 40.0), vec![6, 5]),
        ],
        &[1, 4, 7],
    );
    let mut history = history(start.clone());
    let (command, freed) = ungroup_command(&start, &[n(4), n(7)]).expect("a command");
    assert_eq!(ids(&freed), [2, 3, 6, 5]);
    history
        .execute(command, Duration::ZERO)
        .expect("it ungroups");
    let after = now(&history);
    assert_eq!(ids(&after.roots), [1, 2, 3, 6, 5]);
    assert!(after.node(n(4)).is_none() && after.node(n(7)).is_none());
    for id in [2, 3, 5, 6] {
        let was = start.node(n(id)).expect("a node").base.placement;
        let is = after.node(n(id)).expect("a node").base.placement;
        assert_eq!(bits(&was), bits(&is), "node {id} did not move");
    }
    assert_eq!(history.len(), 1);
    history.undo().expect("an undo");
    assert_eq!(now(&history), start);
    // A nested group ungroups into its parent group.
    let nested = build(
        vec![
            Make::Rect(1, place(0.0, 0.0, 10.0, 10.0)),
            Make::Rect(2, place(20.0, 0.0, 10.0, 10.0)),
            Make::Group(3, place(0.0, 0.0, 10.0, 10.0), vec![1]),
            Make::Group(4, place(0.0, 0.0, 30.0, 10.0), vec![2, 3]),
        ],
        &[4],
    );
    let plan = ungroup_plan(&nested, n(3)).expect("a plan");
    assert_eq!(
        plan,
        UngroupPlan {
            parent: Some(n(4)),
            index: 1,
            children: vec![n(1)]
        }
    );
    assert_eq!(ungroup_plan(&nested, n(1)), None, "not a group");
    assert_eq!(ungroup_command(&nested, &[n(1)]).map(|(_, ids)| ids), None);
}

/// Roots `[9]`; group 9 holds group 8 and rectangle 1; group 8 holds
/// rectangle 2, hidden rectangle 3 and the empty group 7.
fn nested() -> Page {
    let mut page = build(
        vec![
            Make::Rect(1, place(0.0, 0.0, 10.0, 10.0)),
            Make::Rect(2, place(50.0, 50.0, 10.0, 10.0)),
            Make::Rect(3, place(500.0, 500.0, 10.0, 10.0)),
            Make::Group(7, place(0.0, 0.0, 1.0, 1.0), vec![]),
            Make::Group(8, place(0.0, 0.0, 1.0, 1.0), vec![2, 3, 7]),
            Make::Group(9, place(0.0, 0.0, 1.0, 1.0), vec![8, 1]),
        ],
        &[9],
    );
    let mut hidden = page.node(n(3)).expect("a node").clone();
    hidden.base.visible = false;
    page.nodes.insert(n(3), Arc::new(hidden));
    page
}

// S43-04: a click picks the child of the scope that holds the drawn
// element, at any depth; outside the scope it picks nothing.
#[test]
fn s43_04_pick_climbs_to_the_child_of_the_scope() {
    let page = nested();
    assert_eq!(pick(&page, n(2), None), Some(n(9)));
    assert_eq!(pick(&page, n(2), Some(n(9))), Some(n(8)));
    assert_eq!(pick(&page, n(2), Some(n(8))), Some(n(2)));
    assert_eq!(pick(&page, n(1), Some(n(9))), Some(n(1)));
    assert_eq!(pick(&page, n(1), Some(n(8))), None, "outside the scope");
    assert_eq!(pick(&page, n(99), None), None);
    let parents = parents(&page);
    assert_eq!(parents.get(&n(9)), Some(&None));
    assert_eq!(parents.get(&n(2)), Some(&Some(n(8))));
}

// S43-04: a double click on a group enters it, one level at a time, and
// selects its child under the pointer; a click outside the scope leaves it
// for the root that holds the element.
#[test]
fn s43_04_double_click_enters_and_a_click_outside_leaves() {
    let page = nested();
    assert_eq!(click(&page, n(2), None, false), Some((n(9), None)));
    assert_eq!(click(&page, n(2), None, true), Some((n(8), Some(n(9)))));
    assert_eq!(
        click(&page, n(2), Some(n(9)), false),
        Some((n(8), Some(n(9))))
    );
    assert_eq!(
        click(&page, n(2), Some(n(9)), true),
        Some((n(2), Some(n(8))))
    );
    assert_eq!(
        click(&page, n(2), Some(n(8)), true),
        Some((n(2), Some(n(8)))),
        "a leaf"
    );
    assert_eq!(
        click(&page, n(1), Some(n(8)), false),
        Some((n(9), None)),
        "outside"
    );
    assert_eq!(click(&page, n(99), None, true), None);
}

// S43-04: Ctrl+A and the marquee take the children of the scope that draw
// something: groups now count; hidden and empty ones do not.
#[test]
fn s43_04_items_are_the_drawn_children_of_the_scope() {
    let page = nested();
    assert_eq!(ids(&items(&page, None)), [9]);
    assert_eq!(ids(&items(&page, Some(n(9)))), [8, 1]);
    assert_eq!(
        ids(&items(&page, Some(n(8)))),
        [2],
        "hidden and empty are out"
    );
    let mut hidden = page.clone();
    let mut nine = hidden.node(n(9)).expect("a node").clone();
    nine.base.visible = false;
    hidden.nodes.insert(n(9), Arc::new(nine));
    assert!(items(&hidden, None).is_empty());
}

// S43-05: the box of a group is the box around its children, whatever its
// own placement says; an element's box is its placement.
#[test]
fn s43_05_the_box_of_a_group_is_around_its_children() {
    let page = nested();
    let b = item_box(&page, n(9)).expect("a box");
    // Rectangles 1 (0..10) and 2 (50..60); hidden 3 still counts in the
    // layout box (node_bounds), so the box reaches 510.
    assert_eq!((b.x, b.y, b.x + b.w, b.y + b.h), (0.0, 0.0, 510.0, 510.0));
    let t = three();
    assert_eq!(
        item_box(&t, n(3)),
        Some(t.node(n(3)).expect("a node").base.placement)
    );
    assert_eq!(item_box(&t, n(99)), None);
    assert_eq!(
        siblings(&page, &[n(8), n(1)]),
        Some((Some(n(9)), vec![n(8), n(1)]))
    );
    assert_eq!(siblings(&page, &[n(8), n(2)]), None, "two parents");
    assert_eq!(siblings(&t, &[n(1)]), Some((None, vec![n(1), n(2), n(3)])));
}

/// A group 9 with rectangle 1 at (0, 0, 20, 10), rectangle 2 turned 90°
/// with its centre at (50, 50), and group 8 holding rectangle 3.
fn frame_page() -> Page {
    build(
        vec![
            Make::Rect(1, place(0.0, 0.0, 20.0, 10.0)),
            Make::Rect(2, turned(place(30.0, 45.0, 40.0, 10.0), 90.0)),
            Make::Rect(3, place(80.0, 90.0, 20.0, 10.0)),
            Make::Group(8, place(0.0, 0.0, 1.0, 1.0), vec![3]),
            Make::Group(9, place(0.0, 0.0, 1.0, 1.0), vec![1, 2, 8]),
        ],
        &[9],
    )
}

fn get(list: &[(NodeId, Placement)], id: u64) -> Placement {
    list.iter()
        .find(|(node, _)| *node == n(id))
        .map(|(_, p)| *p)
        .expect("listed")
}

fn center(p: &Placement) -> (f64, f64) {
    (p.x + p.w / 2.0, p.y + p.h / 2.0)
}

// S43-06: moving a group moves every node below it; the groups' own
// placements follow the box around their children; every node is listed,
// parents first.
#[test]
fn s43_06_a_group_moves_with_everything_below_it() {
    let page = frame_page();
    let out = changed(&page, n(9), Change::Move { dx: 5.0, dy: -2.0 });
    assert_eq!(
        ids(&out.iter().map(|(id, _)| *id).collect::<Vec<_>>()),
        [9, 1, 2, 8, 3]
    );
    assert_eq!(get(&out, 1), place(5.0, -2.0, 20.0, 10.0));
    assert_eq!(get(&out, 2), turned(place(35.0, 43.0, 40.0, 10.0), 90.0));
    assert_eq!(get(&out, 3), place(85.0, 88.0, 20.0, 10.0));
    assert_eq!(
        get(&out, 8),
        place(85.0, 88.0, 20.0, 10.0),
        "refit to its child"
    );
    let g = get(&out, 9);
    assert_eq!((g.x, g.y, g.x + g.w, g.y + g.h), (5.0, -2.0, 105.0, 98.0));
    assert!(changed(&page, n(99), Change::Move { dx: 1.0, dy: 1.0 }).is_empty());
}

// S43-06: resizing a group scales every node's place in the box; a turned
// child scales along its own axes and keeps its turn.
#[test]
fn s43_06_a_group_frame_scales_its_children_in_their_axes() {
    let page = frame_page();
    let from = item_box(&page, n(9)).expect("a box");
    assert_eq!((from.x, from.y, from.w, from.h), (0.0, 0.0, 100.0, 100.0));
    let to = place(0.0, 0.0, 200.0, 100.0);
    let out = changed(&page, n(9), Change::Frame { from, to });
    assert_eq!(get(&out, 1), place(0.0, 0.0, 40.0, 10.0));
    let two = get(&out, 2);
    // Its local x runs down the page (scale 1), its local y across (2).
    assert!(near(two.w, 40.0) && near(two.h, 20.0), "{two:?}");
    assert_eq!(center(&two), (100.0, 50.0));
    assert_eq!(two.rotation_deg, 90.0);
    let g = get(&out, 9);
    assert!(near(g.w, 200.0) && near(g.h, 100.0), "{g:?}");
}

// S43-06: turning a group turns every node about the group's centre.
#[test]
fn s43_06_a_group_turns_its_children_about_its_centre() {
    let page = frame_page();
    let out = changed(
        &page,
        n(9),
        Change::Turn {
            center: Point::new(50.0, 50.0),
            degrees: 90.0,
        },
    );
    let one = get(&out, 1);
    // The centre (10, 5) goes to (95, 10) clockwise about (50, 50).
    let (cx, cy) = center(&one);
    assert!(near(cx, 95.0) && near(cy, 10.0), "{one:?}");
    assert_eq!(one.rotation_deg, 90.0);
    assert_eq!((one.w, one.h), (20.0, 10.0));
    assert_eq!(get(&out, 2).rotation_deg, 180.0);
    let back = changed(
        &page,
        n(9),
        Change::Turn {
            center: Point::new(50.0, 50.0),
            degrees: -90.0,
        },
    );
    assert_eq!(get(&back, 1).rotation_deg, 270.0, "normalised");
}

// S43-06: flipping a group mirrors every node through the group's centre.
#[test]
fn s43_06_a_group_flips_its_children_through_its_centre() {
    let page = frame_page();
    let out = changed(
        &page,
        n(9),
        Change::Flip {
            center: Point::new(50.0, 50.0),
            axis: Axis::Horizontal,
        },
    );
    let one = get(&out, 1);
    assert_eq!(center(&one), (90.0, 5.0));
    assert!(one.flip.x && !one.flip.y);
    let three = get(&out, 3);
    assert_eq!(center(&three), (10.0, 95.0));
    let v = changed(
        &page,
        n(9),
        Change::Flip {
            center: Point::new(50.0, 50.0),
            axis: Axis::Vertical,
        },
    );
    assert_eq!(center(&get(&v, 1)), (10.0, 95.0));
    assert!(get(&v, 1).flip.y);
}

// S43-07: a subtree becomes a node to insert with the same values; moved
// copies move every placement; InsertNodes numbers in preorder.
#[test]
fn s43_07_specs_copy_the_subtree_and_number_in_preorder() {
    let page = frame_page();
    let spec = spec_of(&page, n(9)).expect("a spec");
    assert!(matches!(spec.kind, NodeKind::Group(_)));
    assert_eq!(spec.children.len(), 3);
    assert_eq!(
        spec.children[1].placement,
        page.node(n(2)).expect("a node").base.placement
    );
    assert_eq!(spec.children[2].children.len(), 1, "the nested group");
    let moved = translated(&spec, 10.0, 10.0);
    assert_eq!(moved.children[0].placement, place(10.0, 10.0, 20.0, 10.0));
    assert_eq!(
        moved.children[2].children[0].placement,
        place(90.0, 100.0, 20.0, 10.0)
    );
    let leaf = spec_of(&page, n(1)).expect("a spec");
    assert_eq!(ids(&top_ids(10, &[spec.clone(), leaf.clone()])), [10, 15]);
    assert_eq!(ids(&top_ids(3, &[leaf.clone(), leaf])), [3, 4]);
    assert_eq!(spec_of(&page, n(99)), None);
}
