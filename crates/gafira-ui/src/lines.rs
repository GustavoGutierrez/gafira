//! Lines on the canvas (spec 058, D-98): where a dragged end sticks, the
//! line with one end moved, and its length and angle. Pure, so it is
//! tested without a window.

use gafira_doc::{Anchor, Attachment, LineNode, NodeId, NodeKind, Page, Placement, ShapeGeometry};
use gafira_geom::Point;

/// How near, in screen px, a dragged end must come to a shape's outline or
/// one of its five points to stick to it.
pub(crate) const SNAP_PX: f64 = 12.0;

/// How near, in screen px, a dragged end must come to a shape to show its
/// five points.
pub(crate) const REACH_PX: f64 = 40.0;

/// Where a dragged end sticks: the shape, the point in its unit box and
/// the point on the page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Stick {
    pub(crate) node: NodeId,
    pub(crate) at: [f64; 2],
    pub(crate) point: Point,
}

/// The elements a line end may stick to: everything drawn on `page` but
/// lines, groups and `exclude`, front first.
fn targets(page: &Page, exclude: NodeId) -> Vec<NodeId> {
    crate::canvas::shapes(page)
        .into_iter()
        .rev()
        .filter(|shape| shape.node != exclude && shape.line.is_none())
        .map(|shape| shape.node)
        .collect()
}

/// The box of `node` on the page, upright around its turned box.
fn page_box(page: &Page, node: NodeId) -> Option<kurbo::Rect> {
    let p = page.node(node)?.base.placement;
    Some(
        p.to_transform()
            .transform_rect_bbox(kurbo::Rect::new(0.0, 0.0, p.w, p.h)),
    )
}

/// The elements whose box comes within `reach` page px of `p`, front
/// first: those that show their five points.
pub(crate) fn near(page: &Page, p: Point, reach: f64, exclude: NodeId) -> Vec<NodeId> {
    targets(page, exclude)
        .into_iter()
        .filter(|id| {
            page_box(page, *id).is_some_and(|b| {
                let dx = (b.x0 - p.x).max(p.x - b.x1).max(0.0);
                let dy = (b.y0 - p.y).max(p.y - b.y1).max(0.0);
                dx.hypot(dy) <= reach
            })
        })
        .collect()
}

/// Where an end dragged to `p` sticks, within `snap` page px: one of the
/// five points of a shape first, else the nearest point of an outline.
/// `None` when nothing is that near.
pub(crate) fn stick(page: &Page, p: Point, snap: f64, exclude: NodeId) -> Option<Stick> {
    let targets = targets(page, exclude);
    // The five points win over the rest of the outline.
    let anchor = targets
        .iter()
        .flat_map(|id| {
            Anchor::ALL.into_iter().filter_map(move |anchor| {
                let point = LineNode::anchor_point(page, *id, anchor.unit())?;
                Some(Stick {
                    node: *id,
                    at: anchor.unit(),
                    point,
                })
            })
        })
        .filter(|s| (s.point - p).hypot() <= snap)
        .min_by(|a, b| (a.point - p).hypot().total_cmp(&(b.point - p).hypot()));
    if anchor.is_some() {
        return anchor;
    }
    targets
        .iter()
        .filter_map(|id| {
            let (at, point) = outline_point(page, *id, p)?;
            Some(Stick {
                node: *id,
                at,
                point,
            })
        })
        .filter(|s| (s.point - p).hypot() <= snap)
        .min_by(|a, b| (a.point - p).hypot().total_cmp(&(b.point - p).hypot()))
}

/// The point of `node`'s outline nearest to `p`, as a point of its unit
/// box and on the page; the outline of a path is followed point by point.
pub(crate) fn outline_point(page: &Page, node: NodeId, p: Point) -> Option<([f64; 2], Point)> {
    let target = page.node(node)?;
    let placement = target.base.placement;
    let (w, h) = (placement.w, placement.h);
    if !(w > 0.0 || h > 0.0) {
        return None;
    }
    let transform = placement.to_transform();
    let local = transform.inverse() * p;
    let nearest = match &target.kind {
        NodeKind::Line(_) | NodeKind::Group(_) | NodeKind::Extension(_) => return None,
        NodeKind::Shape(shape) => match &shape.geometry {
            ShapeGeometry::Ellipse => {
                // Along the ray from the centre: on the ellipse.
                let (rx, ry) = (w / 2.0, h / 2.0);
                let d = local - Point::new(rx, ry);
                let scale = ((d.x / rx).powi(2) + (d.y / ry).powi(2)).sqrt();
                if scale <= f64::EPSILON {
                    Point::new(rx, 0.0)
                } else {
                    Point::new(rx, ry) + d / scale
                }
            }
            ShapeGeometry::Path(path) => {
                let mut best: Option<Point> = None;
                let mut last = Point::ZERO;
                let mut start = Point::ZERO;
                let mut consider = |a: Point, b: Point| {
                    let q = nearest_on_segment(local, a, b);
                    if best.is_none_or(|n| (n - local).hypot() > (q - local).hypot()) {
                        best = Some(q);
                    }
                };
                let outline = crate::shapes::bez(path, w, h);
                kurbo::flatten(outline.elements().iter().copied(), 0.25, |el| match el {
                    kurbo::PathEl::MoveTo(p) => {
                        last = p;
                        start = p;
                    }
                    kurbo::PathEl::LineTo(p) => {
                        consider(last, p);
                        last = p;
                    }
                    kurbo::PathEl::ClosePath => {
                        consider(last, start);
                        last = start;
                    }
                    _ => {}
                });
                best?
            }
            ShapeGeometry::Rect => box_point(local, w, h),
        },
        NodeKind::Text(_)
        | NodeKind::Image(_)
        | NodeKind::Frame(_)
        | NodeKind::Qr(_)
        | NodeKind::Table(_)
        | NodeKind::Chart(_) => box_point(local, w, h),
    };
    let unit = |v: f64, side: f64| if side > 0.0 { v / side } else { 0.5 };
    Some((
        [unit(nearest.x, w), unit(nearest.y, h)],
        transform * nearest,
    ))
}

/// The outline of `node` on the page, as closed rings of page px: what is
/// lit while a line end sticks to it.
pub(crate) fn outline(page: &Page, node: NodeId) -> Vec<Vec<Point>> {
    let Some(target) = page.node(node) else {
        return Vec::new();
    };
    let p = target.base.placement;
    let (w, h) = (p.w, p.h);
    let transform = p.to_transform();
    let path = match &target.kind {
        NodeKind::Shape(shape) => match &shape.geometry {
            ShapeGeometry::Ellipse => kurbo::Shape::to_path(
                &kurbo::Ellipse::from_rect(kurbo::Rect::new(0.0, 0.0, w, h)),
                0.1,
            ),
            ShapeGeometry::Path(path) => crate::shapes::bez(path, w, h),
            ShapeGeometry::Rect => kurbo::Shape::to_path(&kurbo::Rect::new(0.0, 0.0, w, h), 0.1),
        },
        // A frame is lit along its shape (spec 066).
        NodeKind::Frame(frame) => match &frame.clip_shape {
            ShapeGeometry::Ellipse => kurbo::Shape::to_path(
                &kurbo::Ellipse::from_rect(kurbo::Rect::new(0.0, 0.0, w, h)),
                0.1,
            ),
            ShapeGeometry::Path(path) => crate::shapes::bez(path, w, h),
            ShapeGeometry::Rect => {
                let r = frame.corner_radius.max(0.0).min(w.min(h) / 2.0);
                kurbo::Shape::to_path(&kurbo::RoundedRect::new(0.0, 0.0, w, h, r), 0.1)
            }
        },
        NodeKind::Text(_) | NodeKind::Image(_) => {
            kurbo::Shape::to_path(&kurbo::Rect::new(0.0, 0.0, w, h), 0.1)
        }
        _ => return Vec::new(),
    };
    let mut rings: Vec<Vec<Point>> = Vec::new();
    kurbo::flatten(path.elements().iter().copied(), 0.25, |el| match el {
        kurbo::PathEl::MoveTo(q) => rings.push(vec![transform * q]),
        kurbo::PathEl::LineTo(q) => {
            if let Some(ring) = rings.last_mut() {
                ring.push(transform * q);
            }
        }
        _ => {}
    });
    rings
}

/// The point of the edge of a `w` × `h` box nearest to `p`.
fn box_point(p: Point, w: f64, h: f64) -> Point {
    let corners = [
        (Point::new(0.0, 0.0), Point::new(w, 0.0)),
        (Point::new(w, 0.0), Point::new(w, h)),
        (Point::new(w, h), Point::new(0.0, h)),
        (Point::new(0.0, h), Point::new(0.0, 0.0)),
    ];
    corners
        .into_iter()
        .map(|(a, b)| nearest_on_segment(p, a, b))
        .min_by(|a, b| (*a - p).hypot().total_cmp(&(*b - p).hypot()))
        .unwrap_or(p)
}

/// The point of the segment from `a` to `b` nearest to `p`.
pub(crate) fn nearest_on_segment(p: Point, a: Point, b: Point) -> Point {
    let ab = b - a;
    let len = ab.hypot2();
    if len <= f64::EPSILON {
        return a;
    }
    let t = ((p - a).dot(ab) / len).clamp(0.0, 1.0);
    a + ab * t
}

/// `line` placed at `placement` on `page` with end `which` (0 the start,
/// 1 the end) moved to `point`, stuck to `attach` or loose: the new line
/// and the box around its two ends, upright.
pub(crate) fn with_end(
    line: &LineNode,
    placement: &Placement,
    page: &Page,
    which: usize,
    point: Point,
    attach: Option<Attachment>,
) -> (LineNode, Placement) {
    let mut ends = line.ends(page, placement);
    ends[which.min(1)] = point;
    let [a, b] = ends;
    let (x0, y0) = (a.x.min(b.x), a.y.min(b.y));
    let (w, h) = ((a.x - b.x).abs(), (a.y - b.y).abs());
    let unit = |p: Point| {
        [
            if w > 0.0 { (p.x - x0) / w } else { 0.5 },
            if h > 0.0 { (p.y - y0) / h } else { 0.5 },
        ]
    };
    let mut next = line.clone();
    next.start.at = unit(a);
    next.end.at = unit(b);
    if which == 0 {
        next.start.attached = attach;
    } else {
        next.end.attached = attach;
    }
    (next, Placement::new(x0, y0, w, h))
}

/// The length of the segment from `a` to `b` and its angle in degrees,
/// clockwise from the right, in `[0, 360)`.
pub(crate) fn length_and_angle(a: Point, b: Point) -> (f64, f64) {
    let d = b - a;
    let angle = d.y.atan2(d.x).to_degrees();
    (d.hypot(), if angle < 0.0 { angle + 360.0 } else { angle })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gafira_color::{Color, Rgba};
    use gafira_doc::{ElementBase, LineCap, LineEnd, Node, PageId, PageSize, ShapeNode};
    use gafira_geom::{Length, Unit};

    use super::*;

    const RECT: NodeId = NodeId::new(2);
    const CIRCLE: NodeId = NodeId::new(3);
    const LINE: NodeId = NodeId::new(4);

    fn line() -> LineNode {
        LineNode {
            start: LineEnd::new([0.0, 0.0], LineCap::None),
            end: LineEnd::new([1.0, 1.0], LineCap::Arrow),
            color: Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)),
            width: 2.0,
            dash: Default::default(),
        }
    }

    /// A 200 × 100 rectangle at (100, 50), a circle of 100 at (400, 50)
    /// and a line in the box (10, 10)–(60, 40).
    fn page() -> Page {
        let size = PageSize {
            width: Length::new(600.0, Unit::Px),
            height: Length::new(400.0, Unit::Px),
        };
        let mut page = Page::new(PageId::new(1), String::new(), size);
        let shape = |id, at, geometry| {
            Node::new(
                ElementBase::new(id, at),
                NodeKind::Shape(ShapeNode::new(geometry)),
            )
        };
        page.nodes.insert(
            RECT,
            Arc::new(shape(
                RECT,
                Placement::new(100.0, 50.0, 200.0, 100.0),
                ShapeGeometry::Rect,
            )),
        );
        page.nodes.insert(
            CIRCLE,
            Arc::new(shape(
                CIRCLE,
                Placement::new(400.0, 50.0, 100.0, 100.0),
                ShapeGeometry::Ellipse,
            )),
        );
        page.nodes.insert(
            LINE,
            Arc::new(Node::new(
                ElementBase::new(LINE, Placement::new(10.0, 10.0, 50.0, 30.0)),
                NodeKind::Line(line()),
            )),
        );
        page.roots = vec![RECT, CIRCLE, LINE];
        page
    }

    fn near_point(a: Point, b: Point) -> bool {
        (a - b).hypot() < 1e-6
    }

    // S58-04: near one of the five points the end sticks to it; near the
    // outline elsewhere, to the nearest point of the outline (a circle's
    // too); far away, to nothing; never to the line itself.
    #[test]
    fn s58_04_nearest_anchor() {
        let page = page();
        let right = stick(&page, Point::new(305.0, 104.0), 12.0, LINE).expect("sticks");
        assert_eq!((right.node, right.at), (RECT, Anchor::Right.unit()));
        assert!(near_point(right.point, Point::new(300.0, 100.0)));
        let edge = stick(&page, Point::new(150.0, 45.0), 12.0, LINE).expect("sticks");
        assert_eq!(edge.node, RECT);
        assert!(
            near_point(edge.point, Point::new(150.0, 50.0)),
            "{:?}",
            edge.point
        );
        assert!((edge.at[0] - 0.25).abs() < 1e-9 && edge.at[1].abs() < 1e-9);
        let round =
            stick(&page, Point::new(450.0 + 36.0, 100.0 + 36.0), 12.0, LINE).expect("sticks");
        assert_eq!(round.node, CIRCLE);
        let on = round.point - Point::new(450.0, 100.0);
        assert!((on.hypot() - 50.0).abs() < 1.0, "on the circle: {on:?}");
        assert_eq!(stick(&page, Point::new(350.0, 250.0), 12.0, LINE), None);
        assert_eq!(near(&page, Point::new(330.0, 100.0), 40.0, LINE), [RECT]);
        assert!(near(&page, Point::new(20.0, 300.0), 40.0, LINE).is_empty());
        assert_eq!(
            outline_point(&page, LINE, Point::new(0.0, 0.0)),
            None,
            "not a line"
        );
    }

    // S58-04: moving one end keeps the other where it is, and the box is
    // the upright box of both ends; a stuck end keeps its connection.
    #[test]
    fn s58_04_with_end() {
        let page = page();
        let at = Placement::new(10.0, 10.0, 50.0, 30.0);
        let (moved, placement) = with_end(&line(), &at, &page, 1, Point::new(110.0, 0.0), None);
        assert_eq!(placement, Placement::new(10.0, 0.0, 100.0, 10.0));
        let ends = moved.ends(&page, &placement);
        assert!(
            near_point(ends[0], Point::new(10.0, 10.0))
                && near_point(ends[1], Point::new(110.0, 0.0))
        );
        let stuck = Attachment::anchor(RECT, Anchor::Left);
        let (moved, placement) = with_end(
            &line(),
            &at,
            &page,
            1,
            Point::new(100.0, 100.0),
            Some(stuck),
        );
        assert_eq!(moved.end.attached, Some(stuck));
        assert_eq!(moved.start.attached, None);
        assert!(near_point(
            moved.ends(&page, &placement)[1],
            Point::new(100.0, 100.0)
        ));
        assert_eq!(moved.end.cap, LineCap::Arrow, "the caps stay");
    }

    // S58-04: the label shows the length and the angle, clockwise.
    #[test]
    fn s58_04_length_and_angle() {
        let (len, angle) = length_and_angle(Point::new(0.0, 0.0), Point::new(30.0, 40.0));
        assert!((len - 50.0).abs() < 1e-9);
        assert!((angle - 53.130_102_354).abs() < 1e-6);
        let (_, up) = length_and_angle(Point::new(0.0, 0.0), Point::new(0.0, -10.0));
        assert!((up - 270.0).abs() < 1e-9);
    }
}
