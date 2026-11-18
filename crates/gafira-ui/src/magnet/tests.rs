use std::sync::Arc;

use gafira_doc::{
    ElementBase, Guide, GuideAxis, GuideSet, Node, NodeKind, PageId, PageSize, ShapeGeometry,
    ShapeNode,
};
use gafira_geom::{Length, Unit};

use super::*;

const P: PageId = PageId::new(900);

fn n(id: u64) -> NodeId {
    NodeId::new(id)
}

/// A 1000 × 800 page with rectangles at these placements (ids 1, 2...) and
/// a vertical guide at 600 px and a horizontal one at 10 mm.
fn page(rects: &[Placement]) -> Page {
    let size = PageSize {
        width: Length::new(1000.0, Unit::Px),
        height: Length::new(800.0, Unit::Px),
    };
    let mut page = Page::new(P, String::new(), size);
    for (i, at) in rects.iter().enumerate() {
        let id = n(i as u64 + 1);
        let node = Node::new(
            ElementBase::new(id, *at),
            NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect)),
        );
        page.nodes.insert(id, Arc::new(node));
        page.roots.push(id);
    }
    page.guides = GuideSet(vec![
        Guide {
            axis: GuideAxis::Vertical,
            position: Length::new(600.0, Unit::Px),
            locked: false,
        },
        Guide {
            axis: GuideAxis::Horizontal,
            position: Length::new(10.0, Unit::Mm),
            locked: false,
        },
    ]);
    page
}

fn field<'a>(page: &'a Page, zoom: f64, moving: &'a [NodeId]) -> Field<'a> {
    Field {
        page,
        width: 1000.0,
        height: 800.0,
        dpi: 96.0,
        zoom,
        guides: true,
        active: true,
        moving,
    }
}

fn place(x: f64, y: f64, w: f64, h: f64) -> Placement {
    Placement::new(x, y, w, h)
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
    Rect::new(x, y, x + w, y + h)
}

/// 10 mm at 96 dpi, in px.
const TEN_MM: f64 = 10.0 / 25.4 * 96.0;

// S44-03: the nearest target within 6 screen px wins, per axis: the left
// edge of a box 3 px right of another's right edge goes onto it; at 7 px
// nothing moves.
#[test]
fn s44_03_the_nearest_edge_within_the_threshold_wins() {
    let page = page(&[place(100.0, 100.0, 200.0, 100.0), place(0.0, 0.0, 1.0, 1.0)]);
    let moving = [n(2)];
    let f = field(&page, 1.0, &moving);
    let snap = snap_box(&f, rect(303.0, 433.0, 50.0, 50.0));
    assert_eq!(snap.dx, -3.0);
    assert_eq!(snap.x, Some(300.0));
    let far = snap_box(&f, rect(307.0, 433.0, 50.0, 50.0));
    assert_eq!((far.dx, far.x), (0.0, None));
    // Its own box does not attract: the moving node is out.
    let own = snap_box(&f, rect(2.0, 433.0, 1.0, 1.0));
    assert_eq!(own.x, Some(0.0), "the page edge, not node 2 at 0..1");
}

// S44-04: x and y are independent: x to the vertical guide while y goes
// to the page centre in the same move; the horizontal guide in mm
// attracts in px.
#[test]
fn s44_04_each_axis_snaps_on_its_own() {
    let page = page(&[place(0.0, 0.0, 1.0, 1.0)]);
    let moving = [n(1)];
    let f = field(&page, 1.0, &moving);
    let snap = snap_box(&f, rect(595.0, 378.0, 40.0, 40.0));
    assert_eq!(
        (snap.dx, snap.x),
        (5.0, Some(600.0)),
        "left edge to the guide"
    );
    assert_eq!(
        (snap.dy, snap.y),
        (2.0, Some(400.0)),
        "centre to the page centre"
    );
    let mm = snap_box(&f, rect(700.0, TEN_MM + 4.0, 40.0, 40.0));
    assert!((mm.dy + 4.0).abs() < 1e-9 && mm.y == Some(TEN_MM), "{mm:?}");
}

// S44-05: the threshold is in screen px: at 200 % it is 3 px of page, so
// 3 snaps and 3.5 does not; at 50 % 10 px of page snap.
#[test]
fn s44_05_the_threshold_is_the_same_on_screen_at_any_zoom() {
    let page = page(&[place(100.0, 100.0, 200.0, 100.0), place(0.0, 0.0, 1.0, 1.0)]);
    let moving = [n(2)];
    let at = |zoom: f64, x: f64| snap_box(&field(&page, zoom, &moving), rect(x, 433.0, 50.0, 50.0));
    assert_eq!(at(2.0, 303.0).x, Some(300.0));
    assert_eq!(at(2.0, 303.5).x, None);
    assert_eq!(at(0.5, 310.0).x, Some(300.0));
    assert_eq!(at(0.5, 312.5).x, None);
}

// S44-06: the magnet off (or Alt held) and hidden guides attract nothing;
// a hidden guide leaves the other targets.
#[test]
fn s44_06_off_snaps_nothing_and_hidden_guides_do_not_attract() {
    let page = page(&[place(0.0, 0.0, 1.0, 1.0)]);
    let moving = [n(1)];
    let mut f = field(&page, 1.0, &moving);
    f.active = false;
    assert_eq!(
        snap_box(&f, rect(595.0, 378.0, 40.0, 40.0)),
        Snap::default()
    );
    assert_eq!(snap_edges(&f, Some(498.0), Some(401.0)), Snap::default());
    f.active = true;
    f.guides = false;
    let snap = snap_box(&f, rect(595.0, 378.0, 40.0, 40.0));
    assert_eq!(snap.x, None, "the guide is hidden");
    assert_eq!(snap.y, Some(400.0), "the page still attracts");
}

// S44-07: a resize snaps only the edges its handle drags: the right edge
// of an upright box to the guide, the opposite edge fixed; an edge handle
// leaves the other axis alone; a turned element does not snap (spec 041).
#[test]
fn s44_07_a_resize_snaps_the_dragged_edges() {
    let page = page(&[place(100.0, 100.0, 200.0, 100.0)]);
    let moving = [n(1)];
    let f = field(&page, 1.0, &moving);
    let origin = place(100.0, 100.0, 200.0, 100.0);
    let corner = Handle { x: 1, y: 1 };
    let (to, snap) = snap_resize(&f, &origin, corner, place(100.0, 100.0, 396.0, 297.0));
    assert_eq!((to.x, to.y), (100.0, 100.0), "the opposite corner stays");
    assert_eq!(to.w, 400.0, "right edge on the page centre at 500");
    assert_eq!(to.h, 300.0, "bottom edge on the page centre at 400");
    assert_eq!((snap.x, snap.y), (Some(500.0), Some(400.0)));
    let left = Handle { x: -1, y: 0 };
    let (to, snap) = snap_resize(&f, &origin, left, place(4.0, 100.0, 296.0, 103.0));
    assert_eq!((to.x, to.w), (0.0, 300.0), "left edge on the page edge");
    assert_eq!((to.y, to.h), (100.0, 103.0), "y is not the handle's");
    assert_eq!(snap.y, None);
    let mut turned = origin;
    turned.rotation_deg = 30.0;
    let mut out = turned;
    out.w = 396.0;
    assert_eq!(
        snap_resize(&f, &turned, corner, out),
        (out, Snap::default())
    );
    let edges = snap_edges(&f, None, Some(397.0));
    assert_eq!(
        (edges.dx, edges.x, edges.dy, edges.y),
        (0.0, None, 3.0, Some(400.0))
    );
}

// S44-03: the box of a selection is the union of the axis-aligned boxes,
// a turned element by its turned outline (D-28).
#[test]
fn s44_03_the_selection_box_is_the_union_of_aligned_boxes() {
    let mut square = place(300.0, 300.0, 100.0, 100.0);
    square.rotation_deg = 45.0;
    let page = page(&[place(100.0, 100.0, 50.0, 50.0), square]);
    let all = selection_box(&page, &[n(1), n(2)]).expect("a box");
    let half = 50.0 * 2f64.sqrt();
    assert!((all.x0 - 100.0).abs() < 1e-9 && (all.y0 - 100.0).abs() < 1e-9);
    assert!((all.x1 - (350.0 + half)).abs() < 1e-9, "{all:?}");
    assert_eq!(selection_box(&page, &[]), None);
    assert_eq!(selection_box(&page, &[n(9)]), None);
}

// S44-02: arrows nudge 1 px, with Shift 10 px; other keys do not.
#[test]
fn s44_02_arrows_nudge_one_or_ten_px() {
    assert_eq!(nudge(Key::ArrowLeft, false), Some((-1.0, 0.0)));
    assert_eq!(nudge(Key::ArrowRight, false), Some((1.0, 0.0)));
    assert_eq!(nudge(Key::ArrowUp, false), Some((0.0, -1.0)));
    assert_eq!(nudge(Key::ArrowDown, false), Some((0.0, 1.0)));
    assert_eq!(nudge(Key::ArrowLeft, true), Some((-10.0, 0.0)));
    assert_eq!(nudge(Key::ArrowDown, true), Some((0.0, 10.0)));
    assert_eq!(nudge(Key::A, false), None);
}
