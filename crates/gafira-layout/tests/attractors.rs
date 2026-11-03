//! FD-06 (c), D-28, ADR 0009, ARQ-110 and ARQ-111 for `attractors`.

use std::sync::Arc;

use gafira_doc::{
    ElementBase, GroupNode, Node, NodeId, NodeKind, Page, PageId, PageSize, Placement,
    ShapeGeometry, ShapeNode,
};
use gafira_layout::{LayoutError, Length, Rect, Unit, attractors};

const BAD: [f64; 3] = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];

fn page() -> Page {
    let side = Length::new(1000.0, Unit::Px);
    Page::new(
        PageId::new(1),
        String::new(),
        PageSize {
            width: side,
            height: side,
        },
    )
}

fn id(n: u64) -> NodeId {
    NodeId::new(n)
}

fn add(p: &mut Page, n: u64, placement: Placement, is_group: bool, children: &[u64]) {
    let kind = if is_group {
        NodeKind::Group(GroupNode {})
    } else {
        NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect))
    };
    let mut node = Node::new(ElementBase::new(id(n), placement), kind);
    node.children = children.iter().map(|&c| id(c)).collect();
    p.nodes.insert(id(n), Arc::new(node));
}

/// A leaf at `b = [x, y, w, h]`.
fn leaf(p: &mut Page, n: u64, b: [f64; 4]) {
    add(p, n, Placement::new(b[0], b[1], b[2], b[3]), false, &[]);
}

/// A group whose own box `b` is unrelated to its children.
fn group(p: &mut Page, n: u64, b: [f64; 4], children: &[u64]) {
    add(p, n, Placement::new(b[0], b[1], b[2], b[3]), true, children);
}

fn edit(p: &mut Page, n: u64, f: impl FnOnce(&mut Node)) {
    f(Arc::make_mut(p.nodes.get_mut(&id(n)).unwrap()));
}

fn hide(p: &mut Page, n: u64) {
    edit(p, n, |node| node.base.visible = false);
}

fn lock(p: &mut Page, n: u64) {
    edit(p, n, |node| node.base.locked = true);
}

fn roots(p: &mut Page, ids: &[u64]) {
    p.roots = ids.iter().map(|&n| id(n)).collect();
}

/// The box a `[x, y, w, h]` placement covers.
fn r(b: [f64; 4]) -> Rect {
    Rect::new(b[0], b[1], b[0] + b[2], b[1] + b[3])
}

fn run(p: &Page, selection: &[u64]) -> Result<Vec<Rect>, LayoutError> {
    let sel: Vec<NodeId> = selection.iter().map(|&n| id(n)).collect();
    attractors(p, &sel)
}

fn bits(boxes: &[Rect]) -> Vec<[u64; 4]> {
    boxes
        .iter()
        .map(|b| [b.x0, b.y0, b.x1, b.y1].map(f64::to_bits))
        .collect()
}

/// A leaf that always attracts, so an empty answer is told from a broken one.
const PROBE: [f64; 4] = [900.0, 900.0, 10.0, 10.0];

/// The tree of the spec: A holds B and C, B holds D and E.
fn abcde() -> Page {
    let mut p = page();
    leaf(&mut p, 4, [200.0, 20.0, 10.0, 10.0]); // D
    leaf(&mut p, 5, [300.0, 30.0, 10.0, 10.0]); // E
    group(&mut p, 2, [-9.0, -9.0, 1.0, 1.0], &[4, 5]); // B
    leaf(&mut p, 3, [100.0, 10.0, 10.0, 10.0]); // C
    group(&mut p, 1, [-8.0, -8.0, 1.0, 1.0], &[2, 3]); // A
    roots(&mut p, &[1]);
    p
}

const C: [f64; 4] = [100.0, 10.0, 10.0, 10.0];
const E: [f64; 4] = [300.0, 30.0, 10.0, 10.0];

#[test]
fn fd_06_c_moving_d_leaves_c_and_e_but_not_a_or_b() {
    let p = abcde();
    assert_eq!(run(&p, &[4]).unwrap(), [r(E), r(C)]);
    // moving D and E: B holds only selection, A holds it too; C is left
    assert_eq!(run(&p, &[4, 5]).unwrap(), [r(C)]);
    // moving C leaves D and E; B attracts by its two children, A is the ancestor
    let b = Rect::new(200.0, 20.0, 310.0, 40.0);
    assert_eq!(
        run(&p, &[3]).unwrap(),
        [b, r([200.0, 20.0, 10.0, 10.0]), r(E)]
    );
    // an ancestor is out at any depth: moving a group takes its place too
    assert_eq!(run(&p, &[2]).unwrap(), [r(C)]);
}

#[test]
fn fd_06_c_a_node_reached_by_two_parents_takes_both_out_when_it_holds_the_selection() {
    // X (4) holds the selected leaf (5) and hangs under both P1 (2) and P2 (3): each is an
    // ancestor of a selected node, so neither attracts.
    let mut p = page();
    leaf(&mut p, 1, PROBE);
    leaf(&mut p, 5, [20.0, 20.0, 5.0, 5.0]);
    group(&mut p, 4, [0.0; 4], &[5]);
    group(&mut p, 2, [0.0; 4], &[4]);
    group(&mut p, 3, [0.0; 4], &[4]);
    roots(&mut p, &[1, 2, 3]);
    assert_eq!(run(&p, &[5]).unwrap(), [r(PROBE)]);
}

#[test]
fn fd_06_c_the_selection_and_its_descendants_do_not_attract() {
    let mut p = page();
    leaf(&mut p, 1, PROBE);
    leaf(&mut p, 2, [10.0, 10.0, 5.0, 5.0]);
    leaf(&mut p, 4, [20.0, 20.0, 5.0, 5.0]);
    group(&mut p, 3, [0.0; 4], &[4]);
    roots(&mut p, &[1, 2, 3]);
    let below = r([20.0, 20.0, 5.0, 5.0]);
    assert_eq!(run(&p, &[2]).unwrap(), [r(PROBE), below, below]);
    // a selected group takes its child, and its own union, away
    assert_eq!(
        run(&p, &[3]).unwrap(),
        [r(PROBE), r([10.0, 10.0, 5.0, 5.0])]
    );
    // a selection that names the same node twice, or a parent and its child, changes nothing
    assert_eq!(run(&p, &[3, 3, 4]).unwrap(), run(&p, &[3]).unwrap());
}

#[test]
fn fd_06_c_hidden_and_locked_nodes_do_not_attract() {
    let mut p = page();
    leaf(&mut p, 1, PROBE);
    leaf(&mut p, 2, [10.0, 10.0, 5.0, 5.0]);
    hide(&mut p, 2);
    leaf(&mut p, 3, [20.0, 20.0, 5.0, 5.0]);
    lock(&mut p, 3);
    roots(&mut p, &[1, 2, 3]);
    assert_eq!(run(&p, &[]).unwrap(), [r(PROBE)]);
}

#[test]
fn fd_06_c_a_node_under_a_hidden_or_locked_ancestor_does_not_attract() {
    // a hidden group, a locked group, and a visible, free node below each, two levels down
    let mut p = page();
    leaf(&mut p, 1, PROBE);
    leaf(&mut p, 10, [10.0, 10.0, 5.0, 5.0]);
    group(&mut p, 11, [0.0; 4], &[10]);
    group(&mut p, 12, [0.0; 4], &[11]);
    hide(&mut p, 12);
    leaf(&mut p, 20, [30.0, 30.0, 5.0, 5.0]);
    group(&mut p, 21, [0.0; 4], &[20]);
    group(&mut p, 22, [0.0; 4], &[21]);
    lock(&mut p, 22);
    roots(&mut p, &[1, 12, 22]);
    assert_eq!(run(&p, &[]).unwrap(), [r(PROBE)]);
    // un-hiding the top node brings back the three nodes below it
    edit(&mut p, 12, |n| n.base.visible = true);
    let want = r([10.0, 10.0, 5.0, 5.0]);
    assert_eq!(run(&p, &[]).unwrap(), [r(PROBE), want, want, want]);
}

#[test]
fn fd_06_c_a_node_in_a_group_attracts_and_the_group_attracts_by_the_union_of_its_children() {
    let mut p = page();
    leaf(&mut p, 1, [100.0, 100.0, 50.0, 50.0]);
    leaf(&mut p, 2, [-40.5, 300.0, 10.0, 10.0]);
    // the group's own box is far from both children and must not matter
    group(&mut p, 3, [5000.0, 5000.0, 1.0, 1.0], &[1, 2]);
    roots(&mut p, &[3]);
    let union = Rect::new(-40.5, 100.0, 150.0, 310.0);
    assert_eq!(
        run(&p, &[]).unwrap(),
        [
            union,
            r([100.0, 100.0, 50.0, 50.0]),
            r([-40.5, 300.0, 10.0, 10.0])
        ]
    );
    // a hidden child is out of the attractors but still part of the group's union
    hide(&mut p, 2);
    assert_eq!(
        run(&p, &[]).unwrap(),
        [union, r([100.0, 100.0, 50.0, 50.0])]
    );
}

#[test]
fn fd_06_c_with_an_empty_selection_every_visible_free_node_attracts() {
    let p = abcde();
    let b = Rect::new(200.0, 20.0, 310.0, 40.0);
    let a = Rect::new(100.0, 10.0, 310.0, 40.0);
    assert_eq!(
        run(&p, &[]).unwrap(),
        [a, b, r([200.0, 20.0, 10.0, 10.0]), r(E), r(C)]
    );
}

#[test]
fn fd_06_c_a_rotated_node_attracts_by_its_box_aligned_to_the_axes() {
    let mut p = page();
    let mut pl = Placement::new(100.0, 100.0, 100.0, 100.0);
    pl.rotation_deg = 45.0;
    add(&mut p, 1, pl, false, &[]);
    roots(&mut p, &[1]);
    let got = run(&p, &[]).unwrap();
    assert_eq!(got.len(), 1);
    let half = 100.0 * 2f64.sqrt() / 2.0;
    for (g, w) in [
        (got[0].x0, 150.0 - half),
        (got[0].y0, 150.0 - half),
        (got[0].x1, 150.0 + half),
        (got[0].y1, 150.0 + half),
    ] {
        assert!((g - w).abs() <= 1e-9);
    }
}

#[test]
fn fd_06_c_boxes_away_from_the_origin_keep_their_exact_values() {
    let mut p = page();
    leaf(&mut p, 1, [-12345.5, 3.0, 20.25, 7.0]);
    leaf(&mut p, 2, [1e6, 2e6, 0.0, 0.0]);
    roots(&mut p, &[1, 2]);
    assert_eq!(
        bits(&run(&p, &[]).unwrap()),
        bits(&[r([-12345.5, 3.0, 20.25, 7.0]), r([1e6, 2e6, 0.0, 0.0])])
    );
}

#[test]
fn fd_06_c_a_selection_id_that_is_not_in_the_page_is_an_error() {
    let p = abcde();
    assert_eq!(run(&p, &[4, 99]), Err(LayoutError::UnknownNode(id(99))));
    assert_eq!(run(&p, &[98, 99]), Err(LayoutError::UnknownNode(id(98))));
    assert_eq!(run(&page(), &[1]), Err(LayoutError::UnknownNode(id(1))));
    // a root that is not in the arena is the same error
    let mut p = page();
    roots(&mut p, &[7]);
    assert_eq!(run(&p, &[]), Err(LayoutError::UnknownNode(id(7))));
    // and so is a child that is not
    let mut p = page();
    group(&mut p, 1, [0.0; 4], &[2]);
    roots(&mut p, &[1]);
    assert_eq!(run(&p, &[]), Err(LayoutError::UnknownNode(id(2))));
}

#[test]
fn fd_06_c_a_node_that_is_not_reachable_from_a_root_is_not_returned() {
    let mut p = page();
    leaf(&mut p, 1, PROBE);
    leaf(&mut p, 2, [10.0, 10.0, 5.0, 5.0]);
    roots(&mut p, &[1]);
    assert_eq!(run(&p, &[]).unwrap(), [r(PROBE)]);
    // naming it in the selection is allowed: it is in the page
    assert_eq!(run(&p, &[2]).unwrap(), [r(PROBE)]);
}

// ---------------------------------------------------------------------------
// ARQ-110: what is an input and what is not

#[test]
fn arq_110_a_nan_outside_the_attracting_nodes_changes_nothing() {
    let want = run(&abcde(), &[4]).unwrap();
    assert_eq!(want.len(), 2);
    let bad_placement = |f: fn(&mut Placement, f64), v: f64| {
        let mut pl = Placement::new(0.0, 0.0, 1.0, 1.0);
        f(&mut pl, v);
        pl
    };
    let setters: [fn(&mut Placement, f64); 6] = [
        |p, v| p.x = v,
        |p, v| p.y = v,
        |p, v| p.w = v,
        |p, v| p.h = v,
        |p, v| p.rotation_deg = v,
        |p, v| p.skew = v,
    ];
    for set in setters {
        for v in BAD {
            // D is the selection; A is its ancestor (the placement of a group)
            let mut p = abcde();
            edit(&mut p, 4, |n| n.base.placement = bad_placement(set, v));
            assert_eq!(run(&p, &[4]).unwrap(), want);
            let mut p = abcde();
            edit(&mut p, 1, |n| n.base.placement = bad_placement(set, v));
            assert_eq!(run(&p, &[4]).unwrap(), want);
            // B, the other ancestor
            let mut p = abcde();
            edit(&mut p, 2, |n| n.base.placement = bad_placement(set, v));
            assert_eq!(run(&p, &[4]).unwrap(), want);
            // a hidden node, a locked one, and nodes below them
            let mut p = abcde();
            add(&mut p, 23, bad_placement(set, v), false, &[]);
            add(&mut p, 24, bad_placement(set, v), false, &[]);
            add(&mut p, 25, bad_placement(set, v), false, &[]);
            group(&mut p, 26, [0.0; 4], &[25]);
            hide(&mut p, 23);
            lock(&mut p, 24);
            hide(&mut p, 26);
            edit(&mut p, 1, |n| n.children.extend([id(23), id(24), id(26)]));
            // the page now has hidden and locked nodes with bad numbers inside A, which is an
            // ancestor of the selection, so none of them is read
            assert_eq!(run(&p, &[4]).unwrap(), want);
            // the same nodes at the root of the page, where no ancestor excludes them
            let mut p = abcde();
            add(&mut p, 23, bad_placement(set, v), false, &[]);
            add(&mut p, 24, bad_placement(set, v), false, &[]);
            add(&mut p, 25, bad_placement(set, v), false, &[]);
            group(&mut p, 26, [0.0; 4], &[25]);
            hide(&mut p, 23);
            lock(&mut p, 24);
            hide(&mut p, 26);
            p.roots.extend([id(23), id(24), id(26)]);
            assert_eq!(run(&p, &[4]).unwrap(), want);
        }
    }
}

#[test]
fn arq_110_a_hidden_or_locked_child_of_an_attracting_group_is_read_and_one_outside_is_not() {
    // Row "Elementos que atraen": E (in B, in A) is hidden or locked and has a NaN `x`.
    let nan_x = |p: &mut Page| edit(p, 5, |n| n.base.placement.x = f64::NAN);
    let err = Err(LayoutError::NonFinite { field: "x" });
    for flag in [hide as fn(&mut Page, u64), lock] {
        let mut p = abcde();
        flag(&mut p, 5);
        nan_x(&mut p);
        // moving C: B attracts and holds E, which is read though it does not attract
        assert_eq!(run(&p, &[3]), err);
        // nothing moves: A and B attract
        assert_eq!(run(&p, &[]), err);
        // moving D: A and B are ancestors, E is in no attracting subtree, so only C is left
        assert_eq!(run(&p, &[4]), Ok(vec![r(C)]));
        // moving B or A takes E's whole branch out
        assert_eq!(run(&p, &[2]), Ok(vec![r(C)]));
        assert_eq!(run(&p, &[1]), Ok(vec![]));
    }
}

#[test]
fn arq_110_a_bad_number_in_an_attracting_node_names_its_field() {
    let names = ["x", "y", "w", "h", "rotation_deg", "skew"];
    let setters: [fn(&mut Placement, f64); 6] = [
        |p, v| p.x = v,
        |p, v| p.y = v,
        |p, v| p.w = v,
        |p, v| p.h = v,
        |p, v| p.rotation_deg = v,
        |p, v| p.skew = v,
    ];
    for (set, name) in setters.iter().zip(names) {
        for v in BAD {
            let want = Err(LayoutError::NonFinite { field: name });
            let mut pl = Placement::new(0.0, 0.0, 1.0, 1.0);
            set(&mut pl, v);
            // a leaf of the page; a leaf inside a group; the placement of a group with children
            let mut p = page();
            add(&mut p, 1, pl, false, &[]);
            roots(&mut p, &[1]);
            assert_eq!(run(&p, &[]), want, "{name} {v}");
            let mut p = page();
            add(&mut p, 1, pl, false, &[]);
            group(&mut p, 2, [0.0; 4], &[1]);
            roots(&mut p, &[2]);
            assert_eq!(run(&p, &[]), want, "{name} {v} in a group");
            let mut p = page();
            leaf(&mut p, 1, [0.0, 0.0, 1.0, 1.0]);
            add(&mut p, 2, pl, true, &[1]);
            roots(&mut p, &[2]);
            assert_eq!(run(&p, &[]), want, "{name} {v} on a group");
            // a hidden child of an attracting group is part of its union, so it is read
            let mut p = page();
            add(&mut p, 1, pl, false, &[]);
            hide(&mut p, 1);
            leaf(&mut p, 3, [5.0, 5.0, 1.0, 1.0]);
            group(&mut p, 2, [0.0; 4], &[3, 1]);
            roots(&mut p, &[2]);
            assert_eq!(run(&p, &[]), want, "{name} {v} hidden under an attractor");
        }
    }
    // a box with a negative size
    let mut p = page();
    leaf(&mut p, 1, [0.0, 0.0, -1.0, 1.0]);
    roots(&mut p, &[1]);
    assert_eq!(
        run(&p, &[]),
        Err(LayoutError::InvalidBox { field: "placement" })
    );
}

#[test]
fn arq_111_the_same_input_gives_the_same_boxes_in_the_same_order() {
    let p = abcde();
    let mut with_zero = abcde();
    leaf(&mut with_zero, 30, [-0.0, 0.0, 0.0, 0.0]);
    with_zero.roots.push(id(30));
    for (page, sel) in [(&p, vec![4]), (&p, vec![]), (&with_zero, vec![3])] {
        let first = bits(&run(page, &sel).unwrap());
        for _ in 0..3 {
            assert_eq!(bits(&run(page, &sel).unwrap()), first);
        }
    }
}

// ---------------------------------------------------------------------------
// Depth

/// A chain of `n - 1` groups, each holding the next, with a leaf at the bottom.
fn chain(n: u64) -> Page {
    let mut p = page();
    leaf(&mut p, n, [3.0, 4.0, 5.0, 6.0]);
    for k in 1..n {
        group(&mut p, k, [0.0; 4], &[k + 1]);
    }
    roots(&mut p, &[1]);
    p
}

const DEPTH: u64 = 100_000;

#[test]
fn arq_110_a_very_deep_chain_gives_a_result_not_a_stack_overflow() {
    let p = chain(DEPTH);
    // everything attracts, each group by the box of the leaf below it
    let all = run(&p, &[]).unwrap();
    assert_eq!(all.len(), DEPTH as usize);
    assert!(all.iter().all(|b| *b == r([3.0, 4.0, 5.0, 6.0])));
    // the leaf selected: every group above it is an ancestor
    assert_eq!(run(&p, &[DEPTH]).unwrap(), []);
    // the top selected: it and everything below it are out
    assert_eq!(run(&p, &[1]).unwrap(), []);
    // a node in the middle selected: the groups above are ancestors, the ones below descendants
    assert_eq!(run(&p, &[DEPTH / 2]).unwrap(), []);
}

#[test]
fn arq_110_a_deep_chain_with_a_hidden_node_stops_there() {
    let mut p = chain(DEPTH);
    hide(&mut p, DEPTH / 2);
    // the groups above the hidden one still attract, with the union that holds it
    assert_eq!(run(&p, &[]).unwrap().len(), (DEPTH / 2 - 1) as usize);
    let mut p = chain(DEPTH);
    lock(&mut p, 10);
    assert_eq!(run(&p, &[]).unwrap().len(), 9);
}

#[test]
fn arq_110_a_deep_chain_closed_into_a_loop_is_an_error_not_an_abort() {
    let mut p = chain(DEPTH);
    group(&mut p, DEPTH, [0.0; 4], &[1]);
    assert_eq!(run(&p, &[]), Err(LayoutError::Cycle(id(1))));
    assert_eq!(run(&p, &[5]), Err(LayoutError::Cycle(id(1))));
}

#[test]
fn arq_110_a_wide_group_is_not_walked_twice_per_child() {
    // 20 000 leaves in one group: a quadratic walk would take minutes
    let mut p = page();
    let mut kids = Vec::new();
    for k in 1..=20_000u64 {
        leaf(&mut p, k, [k as f64, 0.0, 1.0, 1.0]);
        kids.push(k);
    }
    group(&mut p, 30_000, [0.0; 4], &kids);
    roots(&mut p, &[30_000]);
    assert_eq!(run(&p, &[]).unwrap().len(), 20_001);
    assert_eq!(run(&p, &[7]).unwrap().len(), 19_999);
}
