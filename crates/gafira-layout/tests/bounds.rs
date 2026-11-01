//! D-28, ADR 0009, ARQ-110 and ARQ-111 for `node_bounds`.

use std::sync::Arc;

use gafira_doc::{
    ElementBase, GroupNode, Node, NodeId, NodeKind, Page, PageId, PageSize, Placement,
    ShapeGeometry, ShapeNode,
};
use gafira_layout::{LayoutError, Length, Rect, Unit, node_bounds};

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

fn add(page: &mut Page, id: u64, placement: Placement, group: bool, children: &[u64]) {
    let kind = if group {
        NodeKind::Group(GroupNode {})
    } else {
        NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect))
    };
    let mut node = Node::new(ElementBase::new(NodeId::new(id), placement), kind);
    node.children = children.iter().map(|&c| NodeId::new(c)).collect();
    page.nodes.insert(NodeId::new(id), Arc::new(node));
}

fn leaf(page: &mut Page, id: u64, b: [f64; 4]) {
    add(page, id, Placement::new(b[0], b[1], b[2], b[3]), false, &[]);
}

fn group(page: &mut Page, id: u64, b: [f64; 4], children: &[u64]) {
    add(
        page,
        id,
        Placement::new(b[0], b[1], b[2], b[3]),
        true,
        children,
    );
}

fn near(a: Rect, b: Rect) {
    for (x, y) in [(a.x0, b.x0), (a.y0, b.y0), (a.x1, b.x1), (a.y1, b.y1)] {
        assert!((x - y).abs() <= 1e-9, "{a:?} vs {b:?}");
    }
}

#[test]
fn d_28_a_leaf_uses_the_box_aligned_to_the_axes() {
    let mut p = page();
    leaf(&mut p, 1, [10.0, 20.0, 100.0, 50.0]);
    assert_eq!(
        node_bounds(&p, NodeId::new(1)),
        Ok(Rect::new(10.0, 20.0, 110.0, 70.0))
    );
    // FD-11(f): 100 x 100 rotated 45 degrees about its centre spans 100 * sqrt(2).
    let mut r = Placement::new(0.0, 0.0, 100.0, 100.0);
    r.rotation_deg = 45.0;
    add(&mut p, 2, r, false, &[]);
    let half = 100.0 * 2f64.sqrt() / 2.0;
    near(
        node_bounds(&p, NodeId::new(2)).unwrap(),
        Rect::new(50.0 - half, 50.0 - half, 50.0 + half, 50.0 + half),
    );
    // A 90 degree turn of a wide box swaps its extents about the centre.
    let mut t = Placement::new(0.0, 0.0, 100.0, 20.0);
    t.rotation_deg = 90.0;
    add(&mut p, 3, t, false, &[]);
    near(
        node_bounds(&p, NodeId::new(3)).unwrap(),
        Rect::new(40.0, -40.0, 60.0, 60.0),
    );
}

#[test]
fn adr_0009_a_group_is_the_union_of_all_its_children_and_ignores_its_own_box() {
    let mut p = page();
    leaf(&mut p, 1, [0.0, 0.0, 10.0, 10.0]);
    leaf(&mut p, 2, [500.0, 40.0, 10.0, 10.0]);
    leaf(&mut p, 3, [-30.0, 900.0, 10.0, 10.0]);
    group(&mut p, 9, [1.0, 1.0, 2.0, 2.0], &[1, 2, 3]);
    assert_eq!(
        node_bounds(&p, NodeId::new(9)),
        Ok(Rect::new(-30.0, 0.0, 510.0, 910.0))
    );
    // Order does not matter, nesting unions through, and rotated children count by their box.
    let mut r = Placement::new(2000.0, 0.0, 100.0, 100.0);
    r.rotation_deg = 45.0;
    add(&mut p, 4, r, false, &[]);
    group(&mut p, 8, [0.0; 4], &[4, 9]);
    let outer = node_bounds(&p, NodeId::new(8)).unwrap();
    let half = 100.0 * 2f64.sqrt() / 2.0;
    near(outer, Rect::new(-30.0, 50.0 - half, 2050.0 + half, 910.0));
    // An empty group falls back to its own box.
    group(&mut p, 7, [5.0, 6.0, 7.0, 8.0], &[]);
    assert_eq!(
        node_bounds(&p, NodeId::new(7)),
        Ok(Rect::new(5.0, 6.0, 12.0, 14.0))
    );
}

#[test]
fn arq_110_bad_placements_and_broken_trees_are_errors() {
    let mut p = page();
    let fields: [fn(&mut Placement, f64); 6] = [
        |p, v| p.x = v,
        |p, v| p.y = v,
        |p, v| p.w = v,
        |p, v| p.h = v,
        |p, v| p.rotation_deg = v,
        |p, v| p.skew = v,
    ];
    let names = ["x", "y", "w", "h", "rotation_deg", "skew"];
    for (set, name) in fields.iter().zip(names) {
        for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut pl = Placement::new(0.0, 0.0, 10.0, 10.0);
            set(&mut pl, v);
            add(&mut p, 1, pl, false, &[]);
            leaf(&mut p, 2, [0.0, 0.0, 1.0, 1.0]);
            // Also bad inside a group: the whole call fails, with no partial union.
            group(&mut p, 3, [0.0, 0.0, 1.0, 1.0], &[2, 1]);
            let nf = Err(LayoutError::NonFinite { field: name });
            assert_eq!(node_bounds(&p, NodeId::new(1)), nf, "{name} {v}");
            assert_eq!(node_bounds(&p, NodeId::new(3)), nf, "{name} {v} in a group");
        }
    }
    leaf(&mut p, 1, [f64::MAX, 0.0, f64::MAX, 1.0]);
    assert!(node_bounds(&p, NodeId::new(1)).is_err());
    leaf(&mut p, 1, [0.0, 0.0, -1.0, 1.0]);
    assert!(node_bounds(&p, NodeId::new(1)).is_err());
    assert_eq!(
        node_bounds(&p, NodeId::new(42)),
        Err(LayoutError::UnknownNode(NodeId::new(42)))
    );
    group(&mut p, 5, [0.0, 0.0, 1.0, 1.0], &[6, 77]);
    assert_eq!(
        node_bounds(&p, NodeId::new(5)),
        Err(LayoutError::UnknownNode(NodeId::new(6)))
    );
    group(&mut p, 5, [0.0, 0.0, 1.0, 1.0], &[6]);
    group(&mut p, 6, [0.0, 0.0, 1.0, 1.0], &[5]);
    assert_eq!(
        node_bounds(&p, NodeId::new(5)),
        Err(LayoutError::Cycle(NodeId::new(5)))
    );
}

#[test]
fn arq_111_the_same_input_gives_the_same_bounds() {
    let mut p = page();
    let mut r = Placement::new(3.3, 4.4, 50.5, 20.25);
    r.rotation_deg = 33.0;
    r.skew = 10.0;
    add(&mut p, 1, r, false, &[]);
    let (a, b) = (
        node_bounds(&p, NodeId::new(1)).unwrap(),
        node_bounds(&p, NodeId::new(1)).unwrap(),
    );
    assert_eq!(
        [a.x0, a.y0, a.x1, a.y1].map(f64::to_bits),
        [b.x0, b.y0, b.x1, b.y1].map(f64::to_bits)
    );
}
