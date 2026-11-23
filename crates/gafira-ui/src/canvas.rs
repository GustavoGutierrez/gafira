//! What the canvas draws and how a pointer maps onto it (03 §9.1): the
//! shapes of a page, back to front, in page space; hit testing, moving and
//! the first box of a new piece. Pure functions, so they are tested without
//! a window.

use egui::{Color32, Pos2, Rect, Vec2};
use gafira_color::{Color, ColorSpaceId, Rgba};
use gafira_doc::{
    NodeId, NodeKind, Page, PageId, Placement, ShapeGeometry, ShapeNode, ShapeStroke, SpanStyle,
    TextNode,
};
use gafira_edit::NodeSpec;
use gafira_geom::{Affine, Point};

use crate::rotation;

/// A drawable shape of a page.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Shape {
    pub(crate) page: PageId,
    pub(crate) node: NodeId,
    pub(crate) placement: Placement,
    pub(crate) geometry: ShapeGeometry,
    pub(crate) fill: Option<Color>,
    /// The outline of a shape (spec 038); `None` for other elements.
    pub(crate) stroke: Option<ShapeStroke>,
    /// The stored corner radius of a shape, in px; 0 for other elements.
    pub(crate) corner_radius: f64,
    /// The gradient that fills a shape instead of `fill` (spec 063).
    pub(crate) gradient: Option<gafira_doc::Gradient>,
    /// A light drawn instead of the shape (spec 065).
    pub(crate) effect: Option<gafira_doc::ShapeEffect>,
    /// A frame: its photo is drawn cut out by the shape (spec 066).
    pub(crate) frame: Option<gafira_doc::FrameNode>,
    /// A QR code: it is drawn from its picture (spec 080).
    pub(crate) qr: Option<Box<gafira_doc::QrNode>>,
    /// A table: it is drawn from its picture (spec 081).
    pub(crate) table: Option<Box<gafira_doc::TableNode>>,
    /// A chart: it is drawn from its picture (spec 083).
    pub(crate) chart: Option<Box<gafira_doc::ChartNode>>,
    /// The element's opacity, 0 to 1.
    pub(crate) opacity: f32,
    /// The opacity of the groups above it, multiplied (spec 043); the
    /// canvas fades the element by both. 1 for a root.
    pub(crate) inherited: f32,
    /// A group's box standing in for the group (spec 043): the editor lists
    /// a selected group as one, with its box as the placement. The canvas
    /// never draws one.
    pub(crate) group: bool,
    /// The text of a text element; its box hits and moves like a rectangle
    /// (spec 027).
    pub(crate) text: Option<TextNode>,
    /// The text inside a shape (spec 055); `None` for other elements.
    pub(crate) inner: Option<gafira_doc::ShapeText>,
    /// A line and its two ends in page px (spec 058); `None` for other
    /// elements.
    pub(crate) line: Option<(gafira_doc::LineNode, [gafira_geom::Point; 2])>,
    /// The asset an image element shows (spec 029).
    pub(crate) image: Option<gafira_doc::AssetRef>,
    /// Locked itself or below a locked group (D-91): drawn, but a click or
    /// a marquee goes through it.
    pub(crate) locked: bool,
}

/// The visible shapes of `page`, back to front: roots in order, each node
/// before its children (02 §3.1). Groups draw nothing themselves; their
/// opacity fades what is below them (spec 043).
pub(crate) fn shapes(page: &Page) -> Vec<Shape> {
    fn visit(
        page: &Page,
        id: NodeId,
        (inherited, held): (f32, bool),
        depth: usize,
        out: &mut Vec<Shape>,
    ) {
        let Some(node) = page.node(id) else { return };
        // Deeper than any valid document, so a broken tree cannot recurse
        // for ever.
        if !node.base.visible || depth > 4096 {
            return;
        }
        let rect = ShapeNode::new(ShapeGeometry::Rect);
        let locked = held || node.base.locked;
        let drawn = match &node.kind {
            NodeKind::Shape(shape) => Some((shape.clone(), None, None)),
            NodeKind::Text(text) => Some((rect.clone(), Some(text.clone()), None)),
            NodeKind::Image(image) => Some((rect.clone(), None, Some(image.asset))),
            NodeKind::Line(_) => Some((rect.clone(), None, None)),
            // A frame is drawn as its outline over its photo; an empty one
            // shows the sample photo on the canvas (spec 066).
            NodeKind::Frame(frame) => {
                let mut shape = ShapeNode::new(frame.clip_shape.clone());
                shape.corner_radius = frame.corner_radius;
                shape.stroke = frame.stroke;
                Some((shape, None, None))
            }
            NodeKind::Qr(_) | NodeKind::Table(_) | NodeKind::Chart(_) => {
                Some((rect.clone(), None, None))
            }
            NodeKind::Group(_) | NodeKind::Extension(_) => None,
        };
        if let Some((shape, text, image)) = drawn {
            let inner_text = shape.text.clone();
            out.push(Shape {
                page: page.id,
                node: id,
                placement: node.base.placement,
                geometry: shape.geometry,
                fill: shape.fill,
                stroke: shape.stroke,
                corner_radius: shape.corner_radius,
                gradient: shape.gradient,
                effect: shape.effect,
                opacity: node.base.opacity,
                inherited,
                group: false,
                text,
                image,
                locked,
                inner: inner_text,
                line: match &node.kind {
                    NodeKind::Line(line) => {
                        Some((line.clone(), line.ends(page, &node.base.placement)))
                    }
                    _ => None,
                },
                chart: match &node.kind {
                    NodeKind::Chart(chart) => Some(chart.clone()),
                    _ => None,
                },
                table: match &node.kind {
                    NodeKind::Table(table) => Some(table.clone()),
                    _ => None,
                },
                qr: match &node.kind {
                    NodeKind::Qr(qr) => Some(qr.clone()),
                    _ => None,
                },
                frame: match &node.kind {
                    NodeKind::Frame(frame) => Some(frame.clone()),
                    _ => None,
                },
            });
        }
        let below = inherited * node.base.opacity.clamp(0.0, 1.0);
        for child in &node.children {
            visit(page, *child, (below, locked), depth + 1, out);
        }
    }
    let mut out = Vec::new();
    for root in &page.roots {
        // A locked page locks everything on it (spec 056).
        visit(page, *root, (1.0, page.locked), 0, &mut out);
    }
    out
}

impl Shape {
    /// How opaque the canvas draws it: its own opacity by the one of the
    /// groups above it, 0 to 1.
    pub(crate) fn alpha(&self) -> f32 {
        self.opacity.clamp(0.0, 1.0) * self.inherited.clamp(0.0, 1.0)
    }
}

/// One row of the Capas tab: an element of the page, front first, the
/// children of an open group below it (spec 043).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Layer {
    pub(crate) node: NodeId,
    /// The element's own name, if it has one.
    pub(crate) name: Option<String>,
    /// The catalog key of its kind (`pieces.rect`, `pieces.ellipse`, `layers.group`, ...).
    pub(crate) kind: &'static str,
    /// Its index among its siblings (0 is the back).
    pub(crate) index: usize,
    /// How many siblings it has, itself included.
    pub(crate) siblings: usize,
    /// How many groups it is in: 0 for a root.
    pub(crate) depth: usize,
    /// The group it is in, `None` for a root.
    pub(crate) parent: Option<NodeId>,
    /// A group with children: the row opens and closes.
    pub(crate) opens: bool,
}

/// The elements of `page`, front first, as the Capas tab lists them: the
/// roots, and below each group in `open` its children, one step in.
pub(crate) fn layers(page: &Page, open: &std::collections::BTreeSet<NodeId>) -> Vec<Layer> {
    struct Walk<'a> {
        page: &'a Page,
        open: &'a std::collections::BTreeSet<NodeId>,
        seen: std::collections::BTreeSet<NodeId>,
        out: Vec<Layer>,
    }
    fn list(walk: &mut Walk<'_>, parent: Option<NodeId>, ids: &[NodeId], depth: usize) {
        for (index, id) in ids.iter().enumerate().rev() {
            let Some(node) = walk.page.node(*id) else {
                continue;
            };
            // Deeper than any valid document, so a broken tree cannot
            // recurse for ever.
            if depth > 4096 || !walk.seen.insert(*id) {
                continue;
            }
            let opens = matches!(node.kind, NodeKind::Group(_)) && !node.children.is_empty();
            let row = layer(walk.page, *id, index, ids.len(), depth, parent, opens);
            walk.out.push(row);
            if opens && walk.open.contains(id) {
                list(walk, Some(*id), &node.children, depth + 1);
            }
        }
    }
    let mut walk = Walk {
        page,
        open,
        seen: std::collections::BTreeSet::new(),
        out: Vec::new(),
    };
    list(&mut walk, None, &page.roots, 0);
    walk.out
}

/// The row of `id`.
fn layer(
    page: &Page,
    id: NodeId,
    index: usize,
    siblings: usize,
    depth: usize,
    parent: Option<NodeId>,
    opens: bool,
) -> Layer {
    let node = page.node(id);
    let kind = node.map_or("layers.element", |node| kind_key(&node.kind));
    Layer {
        node: id,
        name: node.and_then(|node| node.base.name.clone()),
        kind,
        index,
        siblings,
        depth,
        parent,
        opens,
    }
}

/// The catalog key that names a kind of element: `pieces.rect`,
/// `layers.group`, ...
pub(crate) fn kind_key(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Shape(ShapeNode {
            geometry: ShapeGeometry::Rect,
            ..
        }) => "pieces.rect",
        NodeKind::Shape(ShapeNode {
            geometry: ShapeGeometry::Ellipse,
            ..
        }) => "pieces.ellipse",
        NodeKind::Group(_) => "layers.group",
        NodeKind::Text(_) => "layers.text",
        NodeKind::Image(_) => "layers.image",
        NodeKind::Line(_) => "layers.line",
        NodeKind::Frame(_) => "layers.frame",
        NodeKind::Qr(_) => "layers.qr",
        NodeKind::Table(_) => "layers.table",
        NodeKind::Chart(_) => "layers.chart",
        _ => "layers.element",
    }
}

/// The screen rectangle of `placement` on a page drawn at `page_rect` with
/// `zoom`, before its rotation, shear and flip: where an upright element is
/// drawn, and the box a turned one is laid out in before it is turned.
pub(crate) fn on_screen(page_rect: Rect, zoom: f32, placement: &Placement) -> Rect {
    Rect::from_min_size(
        page_rect.min + Vec2::new(placement.x as f32, placement.y as f32) * zoom,
        Vec2::new(placement.w as f32, placement.h as f32) * zoom,
    )
}

/// The topmost shape under `pointer`; an ellipse is hit only inside its
/// outline. The pointer is taken back to each shape's local box, so a
/// turned or flipped shape is hit where it is drawn (spec 041).
pub(crate) fn hit(shapes: &[Shape], page_rect: Rect, zoom: f32, pointer: Pos2) -> Option<NodeId> {
    // What is locked is not picked: the click goes through it (D-91).
    shapes.iter().rev().filter(|s| !s.locked).find_map(|shape| {
        // A line takes a click within half its width, or 5 screen px, of
        // its segment (spec 058).
        if let Some((line, ends)) = &shape.line {
            let [a, b] = ends.map(|p| point_on_screen(page_rect, zoom, p));
            let q = crate::lines::nearest_on_segment(
                gafira_geom::Point::new(f64::from(pointer.x), f64::from(pointer.y)),
                gafira_geom::Point::new(f64::from(a.x), f64::from(a.y)),
                gafira_geom::Point::new(f64::from(b.x), f64::from(b.y)),
            );
            let d =
                (q - gafira_geom::Point::new(f64::from(pointer.x), f64::from(pointer.y))).hypot();
            let reach = (line.width * f64::from(zoom) / 2.0).max(5.0);
            return (d <= reach).then_some(shape.node);
        }
        let (w, h) = (shape.placement.w, shape.placement.h);
        let transform = rotation::to_screen(page_rect, zoom, &shape.placement);
        let local = rotation::local(transform, pointer)?;
        let inside = match &shape.geometry {
            ShapeGeometry::Rect => (0.0..=w).contains(&local.x) && (0.0..=h).contains(&local.y),
            ShapeGeometry::Ellipse => {
                let (rx, ry) = (w / 2.0, h / 2.0);
                rx > 0.0
                    && ry > 0.0
                    && ((local.x - rx) / rx).powi(2) + ((local.y - ry) / ry).powi(2) <= 1.0
            }
            // A path takes what its non-zero interior covers (S54-04).
            ShapeGeometry::Path(path) => {
                use kurbo::Shape as _;
                crate::shapes::bez(path, w, h).contains(kurbo::Point::new(local.x, local.y))
            }
        };
        inside.then_some(shape.node)
    })
}

/// The shapes whose screen box touches `marquee`, back to front (spec 040).
/// An ellipse counts by its box, like a resize handle sits on it, and a
/// turned shape by the axis-aligned box of its turned box (D-28, spec 041).
pub(crate) fn in_marquee(
    shapes: &[Shape],
    page_rect: Rect,
    zoom: f32,
    marquee: Rect,
) -> Vec<NodeId> {
    shapes
        .iter()
        .filter(|shape| !shape.locked)
        .filter(|shape| screen_box_of(shape, page_rect, zoom).intersects(marquee))
        .map(|shape| shape.node)
        .collect()
}

/// Where page point `p` (page px) is drawn on a page at `page_rect`.
pub(crate) fn point_on_screen(page_rect: Rect, zoom: f32, p: gafira_geom::Point) -> Pos2 {
    page_rect.min + Vec2::new(p.x as f32, p.y as f32) * zoom
}

/// The upright screen box of a shape: the one around its turned box, or
/// around the two ends of a line (spec 058).
pub(crate) fn screen_box_of(shape: &Shape, page_rect: Rect, zoom: f32) -> Rect {
    match &shape.line {
        Some((_, [a, b])) => Rect::from_two_pos(
            point_on_screen(page_rect, zoom, *a),
            point_on_screen(page_rect, zoom, *b),
        ),
        None => rotation::screen_box(page_rect, zoom, &shape.placement),
    }
}

/// What the canvas draws for `line` from `a` to `b` (screen px) at `zoom`
/// (spec 058): its stroke, cut back where a hollow cap sits, and its caps,
/// as the render draws them.
pub(crate) fn line_shapes(
    line: &gafira_doc::LineNode,
    a: Pos2,
    b: Pos2,
    zoom: f32,
    color: Color32,
) -> Vec<egui::Shape> {
    use gafira_doc::LineCap;
    let width = line.width as f32 * zoom;
    let length = a.distance(b);
    if width <= 0.0 || length <= f32::EPSILON {
        return Vec::new();
    }
    let stroke = egui::Stroke::new(width, color);
    let dir = (b - a) / length;
    let size = (line.width as f32 * 2.5).max(5.0) * zoom;
    let reach = |cap: LineCap| match cap {
        LineCap::None | LineCap::Arrow => 0.0,
        _ => size,
    };
    let (ra, rb) = (reach(line.start.cap), reach(line.end.cap));
    let mut out = Vec::new();
    if ra + rb < length {
        out.push(egui::Shape::line_segment(
            [a + dir * ra, b - dir * rb],
            stroke,
        ));
    }
    for (at, cap, out_dir) in [(a, line.start.cap, -dir), (b, line.end.cap, dir)] {
        let normal = Vec2::new(-out_dir.y, out_dir.x);
        let centre = at - out_dir * size;
        match cap {
            LineCap::None => {}
            LineCap::Arrow => {
                let back = at - out_dir * size * 1.6;
                out.push(egui::Shape::line(
                    vec![back + normal * size, at, back - normal * size],
                    stroke,
                ));
            }
            LineCap::Circle => {
                out.push(egui::Shape::circle_stroke(centre, size, stroke));
            }
            LineCap::Diamond | LineCap::DiamondFilled => {
                let points = vec![
                    at,
                    centre + normal * size,
                    centre - out_dir * size,
                    centre - normal * size,
                ];
                let fill = if cap == LineCap::DiamondFilled {
                    color
                } else {
                    Color32::TRANSPARENT
                };
                out.push(egui::Shape::convex_polygon(points, fill, stroke));
            }
            LineCap::Square => {
                let half = size * 0.8;
                let points = [(1.0, 1.0), (1.0, -1.0), (-1.0, -1.0), (-1.0, 1.0)]
                    .into_iter()
                    .map(|(u, v)| centre + out_dir * (half * u) + normal * (half * v))
                    .collect();
                out.push(egui::Shape::closed_line(points, stroke));
            }
        }
    }
    out
}

/// The smallest box that holds every box of `rects`: the box drawn around
/// a multi-selection (spec 040). `None` for no box.
pub(crate) fn bounds(rects: impl IntoIterator<Item = Rect>) -> Option<Rect> {
    rects.into_iter().reduce(|all, rect| all.union(rect))
}

/// `origin` moved by `offset` screen px at `zoom`, in page space.
pub(crate) fn moved(origin: &Placement, offset: Vec2, zoom: f32) -> Placement {
    let mut placement = *origin;
    placement.x += f64::from(offset.x / zoom);
    placement.y += f64::from(offset.y / zoom);
    placement
}

/// One of the eight resize handles of a selection (03 §9.1), by the edges
/// it moves: `-1` the left or top edge, `1` the right or bottom one, `0`
/// neither.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Handle {
    pub(crate) x: i8,
    pub(crate) y: i8,
}

/// The handles in drawing order: corners, then edge midpoints.
pub(crate) const HANDLES: [Handle; 8] = [
    Handle { x: -1, y: -1 },
    Handle { x: 1, y: -1 },
    Handle { x: 1, y: 1 },
    Handle { x: -1, y: 1 },
    Handle { x: 0, y: -1 },
    Handle { x: 1, y: 0 },
    Handle { x: 0, y: 1 },
    Handle { x: -1, y: 0 },
];

/// Half the side of a handle's drawn square, in screen px.
pub(crate) const HANDLE_HALF: f32 = 5.0;

/// Half the side of a handle's hit area, in screen px: the 24 px target of
/// 03 (`size.target.min`), wider than the square that is drawn.
pub(crate) const HANDLE_REACH: f32 = 12.0;

/// Where `handle` sits on the screen rectangle `rect`.
pub(crate) fn handle_at(rect: Rect, handle: Handle) -> Pos2 {
    let along = |min: f32, max: f32, side: i8| match side {
        -1 => min,
        1 => max,
        _ => (min + max) / 2.0,
    };
    Pos2::new(
        along(rect.min.x, rect.max.x, handle.x),
        along(rect.min.y, rect.max.y, handle.y),
    )
}

/// What the pointer holds on a single selection (spec 041): a resize
/// handle or the rotate handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Grip {
    Resize(Handle),
    Rotate,
}

/// Where `handle` of `placement` sits on the screen, on its turned box.
pub(crate) fn handle_point(transform: Affine, placement: &Placement, handle: Handle) -> Pos2 {
    let local = Rect::from_min_size(
        Pos2::ZERO,
        Vec2::new(placement.w as f32, placement.h as f32),
    );
    let at = handle_at(local, handle);
    rotation::screen(transform, f64::from(at.x), f64::from(at.y))
}

/// The grip of `placement` under `pointer`, if any: the nearest of the
/// eight handles and the rotate handle whose 24 px hit area holds it.
pub(crate) fn grip_hit(
    transform: Affine,
    placement: &Placement,
    pointer: Pos2,
    text: bool,
) -> Option<Grip> {
    HANDLES
        .into_iter()
        // A text box has no handles at the middle of its top and bottom.
        .filter(|handle| !text || crate::textbox::shown_on_text(*handle))
        .map(|handle| {
            (
                Grip::Resize(handle),
                handle_point(transform, placement, handle),
            )
        })
        .chain([(Grip::Rotate, rotation::rotate_handle(transform, placement))])
        .filter(|(_, at)| {
            (pointer.x - at.x).abs() <= HANDLE_REACH && (pointer.y - at.y).abs() <= HANDLE_REACH
        })
        .min_by(|(_, a), (_, b)| a.distance_sq(pointer).total_cmp(&b.distance_sq(pointer)))
        .map(|(grip, _)| grip)
}

/// The smallest side a resize leaves, in px of page.
pub(crate) const MIN_SIDE: f64 = 1.0;

/// `origin` resized by dragging `handle` `offset` screen px at `zoom`. The
/// opposite edge stays put; a side never gets smaller than 1 px.
///
/// A turned, sheared or flipped element resizes in its own axes (spec
/// 041): the offset is taken into the local box, the box grows there, and
/// the new placement keeps the rotation and puts the opposite edge back
/// where it was on the page.
pub(crate) fn resized(origin: &Placement, handle: Handle, offset: Vec2, zoom: f32) -> Placement {
    let dx = f64::from(offset.x / zoom);
    let dy = f64::from(offset.y / zoom);
    if !rotation::upright(origin) {
        return resized_turned(origin, handle, dx, dy);
    }
    let mut placement = *origin;
    let (mut left, mut right) = (origin.x, origin.x + origin.w);
    let (mut top, mut bottom) = (origin.y, origin.y + origin.h);
    match handle.x {
        -1 => left = (left + dx).min(right - 1.0),
        1 => right = (right + dx).max(left + 1.0),
        _ => {}
    }
    match handle.y {
        -1 => top = (top + dy).min(bottom - 1.0),
        1 => bottom = (bottom + dy).max(top + 1.0),
        _ => {}
    }
    placement.x = left;
    placement.y = top;
    placement.w = right - left;
    placement.h = bottom - top;
    placement
}

/// [`resized`] for an element that is not upright, with the page offset
/// `(dx, dy)`.
fn resized_turned(origin: &Placement, handle: Handle, dx: f64, dy: f64) -> Placement {
    let transform = origin.to_transform();
    let [a, b, c, d, ..] = transform.as_coeffs();
    let linear = Affine::new([a, b, c, d, 0.0, 0.0]);
    if !linear.determinant().is_finite() || linear.determinant().abs() < 1e-12 {
        return *origin;
    }
    let delta = linear.inverse() * Point::new(dx, dy);
    let (mut left, mut right) = (0.0, origin.w);
    let (mut top, mut bottom) = (0.0, origin.h);
    match handle.x {
        -1 => left = delta.x.min(right - 1.0),
        1 => right = (right + delta.x).max(left + 1.0),
        _ => {}
    }
    match handle.y {
        -1 => top = delta.y.min(bottom - 1.0),
        1 => bottom = (bottom + delta.y).max(top + 1.0),
        _ => {}
    }
    // The centre of the new local box, on the page; the turn is about it,
    // so every point of that box stays where the old transform put it.
    let center = transform * Point::new((left + right) / 2.0, (top + bottom) / 2.0);
    let mut placement = *origin;
    placement.w = right - left;
    placement.h = bottom - top;
    placement.x = center.x - placement.w / 2.0;
    placement.y = center.y - placement.h / 2.0;
    placement
}

pub(crate) use crate::shapes::{PIECE_SECTIONS, Piece};

/// `piece` as `new_piece` makes it, with its corner radius.
pub(crate) fn new_shape(page_px: Vec2, piece: Piece, fill: Color) -> NodeSpec {
    // A line: across a third of the shorter side, in the middle, dark
    // (spec 058).
    if let Some((start, end)) = piece.line_caps() {
        let side = f64::from(page_px.x.min(page_px.y)) / 3.0;
        let line = gafira_doc::LineNode {
            start: gafira_doc::LineEnd::new([0.0, 0.5], start),
            end: gafira_doc::LineEnd::new([1.0, 0.5], end),
            color: Color::Srgb(Rgba::new(0.1, 0.1, 0.1, 1.0)),
            width: 4.0,
        };
        let placement = Placement::new(
            (f64::from(page_px.x) - side) / 2.0,
            f64::from(page_px.y) / 2.0,
            side,
            0.0,
        );
        return NodeSpec::new(NodeKind::Line(line), placement);
    }
    let mut spec = new_piece(page_px, piece.geometry(), fill);
    // A shape wider than high keeps the side as its width, and the other
    // way round (spec 054), still in the middle of the page.
    let aspect = piece.aspect();
    let side = spec.placement.w;
    let (w, h) = if aspect >= 1.0 {
        (side, side / aspect)
    } else {
        (side * aspect, side)
    };
    spec.placement.x += (side - w) / 2.0;
    spec.placement.y += (side - h) / 2.0;
    spec.placement.w = w;
    spec.placement.h = h;
    if let NodeKind::Shape(shape) = &mut spec.kind {
        shape.corner_radius = piece.corner_radius(spec.placement.w);
    }
    spec
}

/// A new piece in the middle of a page of `page_px`, a third of its
/// shorter side across.
pub(crate) fn new_piece(page_px: Vec2, geometry: ShapeGeometry, fill: Color) -> NodeSpec {
    let side = f64::from(page_px.x.min(page_px.y)) / 3.0;
    let placement = Placement::new(
        (f64::from(page_px.x) - side) / 2.0,
        (f64::from(page_px.y) - side) / 2.0,
        side,
        side,
    );
    NodeSpec::new(
        NodeKind::Shape(ShapeNode {
            geometry,
            fill: Some(fill),
            stroke: None,
            corner_radius: 0.0,
            text: None,
            gradient: None,
            effect: None,
            drawn_with: None,
        }),
        placement,
    )
}

/// The piece `new_piece` makes on a page of `page_px`, centred on `at`
/// in px of page instead of the page's middle (S52-03).
/// A new light (spec 065): a circle of `share` of the
/// shorter side of the page, in the middle, carrying `effect`. Its fill is
/// the one of a new shape, shown again if the effect is taken off.
pub(crate) fn new_effect(page_px: Vec2, effect: gafira_doc::ShapeEffect, share: f64) -> NodeSpec {
    let side = f64::from(page_px.x.min(page_px.y)) * share;
    let placement = Placement::new(
        (f64::from(page_px.x) - side) / 2.0,
        (f64::from(page_px.y) - side) / 2.0,
        side,
        side,
    );
    let mut shape = ShapeNode::new(ShapeGeometry::Ellipse);
    shape.fill = Some(default_fill());
    shape.effect = Some(effect);
    NodeSpec::new(NodeKind::Shape(shape), placement)
}

/// A new empty frame of `piece`'s outline (spec 066): half the shorter side
/// of the page (a quarter of its area), in the middle, its proportions those
/// of the piece.
pub(crate) fn new_frame(page_px: Vec2, piece: Piece) -> NodeSpec {
    let side = f64::from(page_px.x.min(page_px.y)) / 2.0;
    let aspect = piece.aspect();
    let (w, h) = if aspect >= 1.0 {
        (side, side / aspect)
    } else {
        (side * aspect, side)
    };
    let placement = Placement::new(
        (f64::from(page_px.x) - w) / 2.0,
        (f64::from(page_px.y) - h) / 2.0,
        w,
        h,
    );
    let mut frame = gafira_doc::FrameNode::new(piece.geometry());
    frame.corner_radius = piece.corner_radius(w.min(h));
    NodeSpec::new(NodeKind::Frame(frame), placement)
}

/// The grey silhouette of `outline` in a box of `side` px, its proportions
/// kept: the preview of a design in Biblioteca (spec 068).
pub(crate) fn outline_preview(
    outline: &gafira_io_vector::Outline,
    side: f64,
) -> Option<egui::ColorImage> {
    let (w, h) = if outline.aspect >= 1.0 {
        (side, side / outline.aspect)
    } else {
        (side * outline.aspect, side)
    };
    let path = crate::shapes::bez(&outline.path, w, h);
    let mut builder = tiny_skia::PathBuilder::new();
    let to = |p: kurbo::Point| (p.x as f32, p.y as f32);
    for el in path.elements() {
        match *el {
            kurbo::PathEl::MoveTo(p) => {
                let (x, y) = to(p);
                builder.move_to(x, y);
            }
            kurbo::PathEl::LineTo(p) => {
                let (x, y) = to(p);
                builder.line_to(x, y);
            }
            kurbo::PathEl::QuadTo(c, p) => {
                let ((cx, cy), (x, y)) = (to(c), to(p));
                builder.quad_to(cx, cy, x, y);
            }
            kurbo::PathEl::CurveTo(c1, c2, p) => {
                let ((ax, ay), (bx, by), (x, y)) = (to(c1), to(c2), to(p));
                builder.cubic_to(ax, ay, bx, by, x, y);
            }
            kurbo::PathEl::ClosePath => builder.close(),
        }
    }
    let shape = builder.finish()?;
    // Within 1..=side: the casts are exact.
    let mut pixmap = tiny_skia::Pixmap::new(w.round().max(1.0) as u32, h.round().max(1.0) as u32)?;
    let mut paint = tiny_skia::Paint::default();
    paint.set_color_rgba8(204, 204, 204, 255);
    paint.anti_alias = true;
    pixmap.fill_path(
        &shape,
        &paint,
        tiny_skia::FillRule::Winding,
        tiny_skia::Transform::identity(),
        None,
    );
    Some(egui::ColorImage::from_rgba_premultiplied(
        [pixmap.width() as usize, pixmap.height() as usize],
        pixmap.data(),
    ))
}

/// A new empty frame of a custom `outline` (spec 066, S66-10): its
/// proportions kept, half the shorter side of the page, in the middle.
pub(crate) fn new_custom_frame(page_px: Vec2, outline: gafira_io_vector::Outline) -> NodeSpec {
    let side = f64::from(page_px.x.min(page_px.y)) / 2.0;
    let (w, h) = if outline.aspect >= 1.0 {
        (side, side / outline.aspect)
    } else {
        (side * outline.aspect, side)
    };
    let placement = Placement::new(
        (f64::from(page_px.x) - w) / 2.0,
        (f64::from(page_px.y) - h) / 2.0,
        w,
        h,
    );
    let frame = gafira_doc::FrameNode::new(ShapeGeometry::Path(outline.path));
    NodeSpec::new(NodeKind::Frame(frame), placement)
}

/// A new frame centred at `at`, in px of page (D-105).
pub(crate) fn frame_at(page_px: Vec2, piece: Piece, at: (f64, f64)) -> NodeSpec {
    let mut spec = new_frame(page_px, piece);
    spec.placement.x = at.0 - spec.placement.w / 2.0;
    spec.placement.y = at.1 - spec.placement.h / 2.0;
    spec
}

/// The longest side of a frame texture, in px.
const FRAME_TEXTURE_MAX: f64 = 2048.0;

/// The photo of a frame cut out by its shape at `zoom` (spec 066), in the
/// box of the frame: the same fit and cut as the export. `photo` holds the
/// decoded pixels of the asset, at most 2048 px a side.
pub(crate) fn frame_image(
    shape: &Shape,
    photo: &egui::ColorImage,
    zoom: f32,
) -> Option<egui::ColorImage> {
    let frame = shape.frame.as_ref()?;
    let content = frame.content?;
    let (w, h) = (shape.placement.w, shape.placement.h);
    let zoom = f64::from(zoom);
    if !(w > 0.0 && h > 0.0 && zoom.is_finite() && zoom > 0.0) {
        return None;
    }
    let at = frame.content_transform(w, h)?;
    let scale = zoom.min(FRAME_TEXTURE_MAX / w.max(h));
    // Within 1..=2048: the casts are exact.
    let (pw, ph) = (
        (w * scale).round().max(1.0) as u32,
        (h * scale).round().max(1.0) as u32,
    );
    let [iw, ih] = photo.size;
    let mut bytes = Vec::with_capacity(iw * ih * 4);
    for pixel in &photo.pixels {
        bytes.extend(pixel.to_array());
    }
    let source = tiny_skia::Pixmap::from_vec(
        bytes,
        tiny_skia::IntSize::from_wh(u32::try_from(iw).ok()?, u32::try_from(ih).ok()?)?,
    )?;
    // Photo px → natural px → box px → texture px.
    let natural = kurbo::Affine::scale_non_uniform(
        f64::from(content.natural.0) / iw as f64,
        f64::from(content.natural.1) / ih as f64,
    );
    let [a, b, c, d, e, f] = (kurbo::Affine::scale(scale) * at * natural).as_coeffs();
    let shader = tiny_skia::Pattern::new(
        source.as_ref(),
        tiny_skia::SpreadMode::Pad,
        tiny_skia::FilterQuality::Bilinear,
        1.0,
        tiny_skia::Transform::from_row(a as f32, b as f32, c as f32, d as f32, e as f32, f as f32),
    );
    let mut builder = tiny_skia::PathBuilder::new();
    let to = |p: kurbo::Point| ((p.x * scale) as f32, (p.y * scale) as f32);
    for el in outline_of(shape).elements() {
        match *el {
            kurbo::PathEl::MoveTo(p) => {
                let (x, y) = to(p);
                builder.move_to(x, y);
            }
            kurbo::PathEl::LineTo(p) => {
                let (x, y) = to(p);
                builder.line_to(x, y);
            }
            kurbo::PathEl::QuadTo(c, p) => {
                let ((cx, cy), (x, y)) = (to(c), to(p));
                builder.quad_to(cx, cy, x, y);
            }
            kurbo::PathEl::CurveTo(c1, c2, p) => {
                let ((ax, ay), (bx, by), (x, y)) = (to(c1), to(c2), to(p));
                builder.cubic_to(ax, ay, bx, by, x, y);
            }
            kurbo::PathEl::ClosePath => builder.close(),
        }
    }
    let outline = builder.finish()?;
    let mut pixmap = tiny_skia::Pixmap::new(pw, ph)?;
    let paint = tiny_skia::Paint {
        shader,
        anti_alias: true,
        ..tiny_skia::Paint::default()
    };
    pixmap.fill_path(
        &outline,
        &paint,
        tiny_skia::FillRule::Winding,
        tiny_skia::Transform::identity(),
        None,
    );
    Some(egui::ColorImage::from_rgba_premultiplied(
        [pw as usize, ph as usize],
        pixmap.data(),
    ))
}

pub(crate) fn piece_at(page_px: Vec2, piece: Piece, fill: Color, at: (f64, f64)) -> NodeSpec {
    let mut spec = new_shape(page_px, piece, fill);
    spec.placement.x = at.0 - spec.placement.w / 2.0;
    spec.placement.y = at.1 - spec.placement.h / 2.0;
    spec
}

/// Where a shape dragged from the panel lands (S52-02): the page under
/// `pointer` and the point there in px of page, with the pages drawn at
/// their screen boxes with `zoom`. `None` outside every page.
pub(crate) fn drop_point(
    pages: &[(PageId, Rect)],
    zoom: f32,
    pointer: Pos2,
) -> Option<(PageId, (f64, f64))> {
    if !(zoom.is_finite() && zoom > 0.0) {
        return None;
    }
    let (page, rect) = pages.iter().find(|(_, rect)| rect.contains(pointer))?;
    let at = (pointer - rect.min) / zoom;
    Some((*page, (f64::from(at.x), f64::from(at.y))))
}

/// The three text presets of the Texto tab (03 §2.3): a title, a subtitle
/// and body text, by font size in px.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextPreset {
    Title,
    Subtitle,
    Body,
}

impl TextPreset {
    /// The font size, in px, and whether the display family is used.
    pub(crate) fn size_and_display(self) -> (f64, bool) {
        match self {
            Self::Title => (72.0, true),
            Self::Subtitle => (44.0, true),
            Self::Body => (28.0, false),
        }
    }
}

/// A new text element of `preset` saying `text`, centred on a page of
/// `page_px`, 60 % of its width across and one line high.
pub(crate) fn new_text(page_px: Vec2, text: &str, style: &SpanStyle) -> NodeSpec {
    // As wide as its words, and it grows sideways with them (spec 095).
    let mut node = TextNode::plain(text, style);
    node.sizing = gafira_doc::TextSizing::AutoWidth;
    let w = natural_width(&node).unwrap_or(f64::from(page_px.x) * 0.6);
    let h = style.size * 1.4;
    let placement = Placement::new(
        (f64::from(page_px.x) - w) / 2.0,
        (f64::from(page_px.y) - h) / 2.0,
        w,
        h,
    );
    NodeSpec::new(NodeKind::Text(node), placement)
}

/// The width a text needs to lay each paragraph on one line (spec 095):
/// the narrowest at which the export lays out its widest line whole.
/// `None` when it draws nothing (no words).
pub(crate) fn natural_width(text: &TextNode) -> Option<f64> {
    use kurbo::Shape as _;
    let size = text.content.first_style()?.size;
    let mut flat = text.clone();
    flat.align = gafira_doc::TextAlign::Left;
    flat.curve = 0.0;
    let lay = |w: f64| gafira_render::text_outline(&flat, (w, size * 2.0), text_fonts());
    let (path, _, one_line) = lay(NATURAL_ROOM)?;
    let right = path.bounding_box().x1;
    if !right.is_finite() {
        return None;
    }
    // The letters end before their advance: widen from the ink until the
    // export lays no paragraph on two lines.
    let step = (size * 0.02).max(1.0);
    let mut w = (right.max(0.0) + 1.0).ceil();
    for _ in 0..40 {
        match lay(w) {
            Some((_, _, lines)) if lines > one_line + 0.5 => w += step,
            _ => break,
        }
    }
    Some(w.ceil())
}

/// The width a text is laid out in to measure its lines.
const NATURAL_ROOM: f64 = 100_000.0;

/// The box of a text as wide as its lines, `natural` px (spec 095): it
/// keeps the side its lines are aligned to (left edge, centre or right
/// edge). `None` when it changes by half a px or less.
pub(crate) fn text_width_fit(
    align: gafira_doc::TextAlign,
    placement: &Placement,
    natural: f64,
) -> Option<Placement> {
    if !natural.is_finite() || natural <= 0.0 || (placement.w - natural).abs() <= 0.5 {
        return None;
    }
    let mut fitted = *placement;
    fitted.x = match align {
        gafira_doc::TextAlign::Left | gafira_doc::TextAlign::Justify => placement.x,
        gafira_doc::TextAlign::Center => placement.x + (placement.w - natural) / 2.0,
        gafira_doc::TextAlign::Right => placement.x + placement.w - natural,
    };
    fitted.w = natural;
    Some(fitted)
}

/// The box of a curved text that holds all its letters (spec 095): as
/// high as its arc, and widened the same on both sides when the ends of
/// the arc go past it, so its centre stays. `None` when it already fits.
pub(crate) fn curved_fit(text: &TextNode, placement: &Placement) -> Option<Placement> {
    use kurbo::Shape as _;
    if text.curve == 0.0 {
        return None;
    }
    let size = text.content.first_style()?.size;
    let mut fitted = *placement;
    fitted.w = fitted.w.abs().max(1.0);
    fitted.h = fitted.h.abs().max(1.0);
    for _ in 0..8 {
        let (path, _, _) = gafira_render::text_outline(text, (fitted.w, fitted.h), text_fonts())?;
        let b = path.bounding_box();
        if !(b.x0.is_finite() && b.x1.is_finite() && b.y1.is_finite()) {
            return None;
        }
        let over = (-b.x0).max(b.x1 - fitted.w).max(0.0);
        let h = (b.y1 + size * 0.25).max(1.0);
        if over <= 0.5 && (fitted.h - h).abs() <= 0.5 {
            break;
        }
        if over > 0.5 {
            let grow = over.ceil() + 1.0;
            fitted.x -= grow;
            fitted.w += 2.0 * grow;
        }
        fitted.h = h;
    }
    let moved = (fitted.x - placement.x).abs() > 1e-9
        || (fitted.w - placement.w).abs() > 0.5
        || (fitted.h - placement.h).abs() > 0.5;
    moved.then_some(fitted)
}

/// The box a text of `sizing` placed at `placement` takes when its text,
/// laid out at its width, is `laid` px of page high (S61-02): its height
/// becomes that of the text when it grows down with it. `None` when nothing
/// changes, by half a px or less.
pub(crate) fn text_fit(
    sizing: gafira_doc::TextSizing,
    placement: &Placement,
    laid: f64,
) -> Option<Placement> {
    if sizing == gafira_doc::TextSizing::Fixed
        || !laid.is_finite()
        || (placement.h - laid).abs() <= 0.5
    {
        return None;
    }
    let mut fitted = *placement;
    fitted.h = laid.max(1.0);
    Some(fitted)
}

/// A new text element of `text` in `style` centred on `at` of a page of
/// `page_px`, as `new_text` makes it in the middle (S61-03).
pub(crate) fn text_at(page_px: Vec2, text: &str, style: &SpanStyle, at: (f64, f64)) -> NodeSpec {
    let mut spec = new_text(page_px, text, style);
    spec.placement.x = at.0 - spec.placement.w / 2.0;
    spec.placement.y = at.1 - spec.placement.h / 2.0;
    spec
}

/// A new image element of `pixels` (width, height) centred on a page of
/// `page_px`, scaled to fit 60 % of the page and never enlarged.
pub(crate) fn new_image(
    page_px: Vec2,
    pixels: (u32, u32),
    asset: gafira_doc::AssetRef,
) -> NodeSpec {
    let (w, h) = (f64::from(pixels.0.max(1)), f64::from(pixels.1.max(1)));
    let room_w = f64::from(page_px.x) * 0.6;
    let room_h = f64::from(page_px.y) * 0.6;
    let scale = (room_w / w).min(room_h / h).min(1.0);
    let (w, h) = (w * scale, h * scale);
    let placement = Placement::new(
        (f64::from(page_px.x) - w) / 2.0,
        (f64::from(page_px.y) - h) / 2.0,
        w,
        h,
    );
    NodeSpec::new(
        NodeKind::Image(gafira_doc::ImageNode::new(asset)),
        placement,
    )
}

/// A document colour on screen (sRGB, 8 bits).
pub(crate) fn screen_color(color: Color) -> Color32 {
    let Color::Srgb(Rgba { r, g, b, a }) = color.convert(ColorSpaceId::Srgb) else {
        return Color32::TRANSPARENT;
    };
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgba_unmultiplied(byte(r), byte(g), byte(b), byte(a))
}

/// What a shape paints: its fill with its opacity applied, or nothing.
pub(crate) fn paint(shape: &Shape) -> Color32 {
    shape.fill.map_or(Color32::TRANSPARENT, |fill| {
        screen_color(fill).gamma_multiply(shape.alpha())
    })
}

/// The width, in px, of an outline the Inspector adds to a shape that had
/// none.
pub(crate) const NEW_OUTLINE_WIDTH: f64 = 2.0;

/// What the canvas paints for `shape` in its screen box `at` at `zoom`: the
/// fill, the outline centred on the edge (spec 038) and, for a rectangle,
/// the corners rounded at most to half the shorter side, as the render
/// draws them (FD-36 a). egui keeps corner radii in whole screen px up to
/// 255, so a larger one is drawn at 255.
pub(crate) fn painted(shape: &Shape, at: Rect, zoom: f32) -> egui::Shape {
    let opacity = shape.alpha();
    let stroke =
        shape
            .stroke
            .filter(|stroke| stroke.width > 0.0)
            .map_or(egui::Stroke::NONE, |stroke| {
                egui::Stroke::new(
                    stroke.width as f32 * zoom,
                    screen_color(stroke.color).gamma_multiply(opacity),
                )
            });
    if textured(shape) {
        // Drawn from a texture (specs 054 and 063): see `path_image`.
        return egui::Shape::Noop;
    }
    match &shape.geometry {
        ShapeGeometry::Path(_) => egui::Shape::Noop,
        ShapeGeometry::Rect => {
            let half = f64::from(at.width().min(at.height()).max(0.0)) / 2.0;
            let radius = (shape.corner_radius.max(0.0) * f64::from(zoom))
                .min(half)
                .min(255.0)
                .round() as u8;
            egui::Shape::Rect(egui::epaint::RectShape::new(
                at,
                egui::CornerRadius::same(radius),
                paint(shape),
                stroke,
                egui::StrokeKind::Middle,
            ))
        }
        ShapeGeometry::Ellipse => egui::Shape::Ellipse(egui::epaint::EllipseShape {
            center: at.center(),
            radius: at.size() / 2.0,
            fill: paint(shape),
            stroke,
        }),
    }
}

/// The outline of a shape of `w` × `h` local px with corners rounded by
/// `radius` (a rectangle) or as an ellipse, clockwise from the top. Each
/// quarter of a curve gets `segments` steps.
fn outline(geometry: &ShapeGeometry, w: f64, h: f64, radius: f64, segments: usize) -> Vec<Point> {
    let quarter = std::f64::consts::FRAC_PI_2;
    // Each corner: its arc centre, its radii and the angle where it starts.
    let (corners, rx, ry) = match geometry {
        ShapeGeometry::Ellipse => {
            let (rx, ry) = (w / 2.0, h / 2.0);
            ([(rx, ry); 4], rx, ry)
        }
        ShapeGeometry::Rect | ShapeGeometry::Path(_) => {
            let r = radius.max(0.0).min(w.min(h).max(0.0) / 2.0);
            if r == 0.0 || matches!(geometry, ShapeGeometry::Path(_)) {
                return vec![
                    Point::new(0.0, 0.0),
                    Point::new(w, 0.0),
                    Point::new(w, h),
                    Point::new(0.0, h),
                ];
            }
            ([(w - r, r), (w - r, h - r), (r, h - r), (r, r)], r, r)
        }
    };
    let starts = [-quarter, 0.0, quarter, 2.0 * quarter];
    let mut points = Vec::with_capacity(4 * (segments + 1));
    for ((cx, cy), start) in corners.into_iter().zip(starts) {
        for step in 0..=segments {
            let a = start + quarter * step as f64 / segments as f64;
            points.push(Point::new(cx + rx * a.cos(), cy + ry * a.sin()));
        }
    }
    points.dedup_by(|a, b| (a.x - b.x).abs() < 1e-12 && (a.y - b.y).abs() < 1e-12);
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    points
}

/// What the canvas paints for a turned, sheared or flipped `shape` (spec
/// 041): a convex polygon on its outline mapped by `transform`, filled and
/// outlined like [`painted`] (centred outline scaled by `zoom`, the radius
/// at most half the shorter side).
pub(crate) fn painted_turned(shape: &Shape, transform: Affine, zoom: f32) -> egui::Shape {
    let opacity = shape.alpha();
    let stroke =
        shape
            .stroke
            .filter(|stroke| stroke.width > 0.0)
            .map_or(egui::Stroke::NONE, |stroke| {
                egui::Stroke::new(
                    stroke.width as f32 * zoom,
                    screen_color(stroke.color).gamma_multiply(opacity),
                )
            });
    let (w, h) = (shape.placement.w, shape.placement.h);
    if textured(shape) {
        // Drawn from a texture (specs 054 and 063): see `path_image`.
        return egui::Shape::Noop;
    }
    let curve = match shape.geometry {
        ShapeGeometry::Path(_) => return egui::Shape::Noop,
        ShapeGeometry::Rect => shape.corner_radius,
        ShapeGeometry::Ellipse => w.max(h) / 2.0,
    };
    // About one step per 2 screen px of the curve, from 4 to 64.
    let segments = ((curve * f64::from(zoom)).abs() / 2.0)
        .ceil()
        .clamp(4.0, 64.0) as usize;
    let points = outline(&shape.geometry, w, h, shape.corner_radius, segments)
        .into_iter()
        .map(|p| rotation::screen(transform, p.x, p.y))
        .collect();
    egui::Shape::convex_polygon(points, paint(shape), stroke)
}

/// The box the text of a shape is laid out in (spec 055): the shape's box
/// less a tenth of its shorter side on each side, as the render lays it.
pub(crate) fn text_box(at: Rect) -> Rect {
    let pad = at.width().min(at.height()) / 10.0;
    at.shrink(pad)
}

/// Where the first line of a text `text_h` high starts in `inner`, down
/// the box by `valign` (spec 055).
pub(crate) fn text_top(inner: Rect, text_h: f32, valign: gafira_doc::VerticalAlign) -> f32 {
    let share = match valign {
        gafira_doc::VerticalAlign::Top => 0.0,
        gafira_doc::VerticalAlign::Middle => 0.5,
        gafira_doc::VerticalAlign::Bottom => 1.0,
    };
    inner.min.y + (inner.height() - text_h) * share
}

/// The longest side, in px, of the texture a path shape is drawn from.
const PATH_TEXTURE_MAX: f64 = 2048.0;

/// The box the texture of a path shape covers (spec 054): its placement
/// grown by half its outline and one screen px, so neither the outline nor
/// the anti-aliased edge is cut. It turns and flips about the same centre.
pub(crate) fn path_cover(shape: &Shape, zoom: f32) -> Placement {
    let stroke = shape.stroke.filter(|stroke| stroke.width > 0.0);
    let margin = stroke.map_or(0.0, |stroke| stroke.width / 2.0) + 1.0 / f64::from(zoom);
    let mut covered = shape.placement;
    covered.x -= margin;
    covered.y -= margin;
    covered.w += 2.0 * margin;
    covered.h += 2.0 * margin;
    covered
}

/// What the canvas draws for a path shape (spec 054): its fill and outline
/// rasterised on a texture as large as the shape on screen, and the box the
/// texture covers, the placement grown by half the outline and one screen
/// px so neither is cut. `None` for a shape that is not a path or has no
/// size.
pub(crate) fn path_image(shape: &Shape, zoom: f32) -> Option<(egui::ColorImage, Placement)> {
    if !textured(shape) {
        return None;
    }
    let (w, h) = (shape.placement.w, shape.placement.h);
    let zoom = f64::from(zoom);
    if !(w > 0.0 && h > 0.0 && zoom.is_finite() && zoom > 0.0) {
        return None;
    }
    let stroke = shape.stroke.filter(|stroke| stroke.width > 0.0);
    let covered = path_cover(shape, zoom as f32);
    let margin = shape.placement.x - covered.x;
    let scale = zoom.min(PATH_TEXTURE_MAX / covered.w.max(covered.h));
    let side = |v: f64| (v * scale).round().max(1.0) as u32;
    let (pw, ph) = (side(covered.w), side(covered.h));
    let mut pixmap = tiny_skia::Pixmap::new(pw, ph)?;
    let mut builder = tiny_skia::PathBuilder::new();
    let at = |p: kurbo::Point| {
        (
            ((p.x + margin) * scale) as f32,
            ((p.y + margin) * scale) as f32,
        )
    };
    for el in outline_of(shape).elements() {
        match *el {
            kurbo::PathEl::MoveTo(p) => {
                let (x, y) = at(p);
                builder.move_to(x, y);
            }
            kurbo::PathEl::LineTo(p) => {
                let (x, y) = at(p);
                builder.line_to(x, y);
            }
            kurbo::PathEl::QuadTo(c, p) => {
                let ((cx, cy), (x, y)) = (at(c), at(p));
                builder.quad_to(cx, cy, x, y);
            }
            kurbo::PathEl::CurveTo(c1, c2, p) => {
                let ((ax, ay), (bx, by), (x, y)) = (at(c1), at(c2), at(p));
                builder.cubic_to(ax, ay, bx, by, x, y);
            }
            kurbo::PathEl::ClosePath => builder.close(),
        }
    }
    let outline = builder.finish()?;
    let paint = |color: Color32| {
        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(color.r(), color.g(), color.b(), color.a());
        paint.anti_alias = true;
        paint
    };
    if let Some(gradient) = &shape.gradient {
        if let Some(shader) = gradient_shader(gradient, (w, h), (margin, scale)) {
            let paint = tiny_skia::Paint {
                shader,
                anti_alias: true,
                ..tiny_skia::Paint::default()
            };
            pixmap.fill_path(
                &outline,
                &paint,
                tiny_skia::FillRule::Winding,
                tiny_skia::Transform::identity(),
                None,
            );
        }
    } else if let Some(fill) = shape.fill {
        pixmap.fill_path(
            &outline,
            &paint(screen_color(fill)),
            tiny_skia::FillRule::Winding,
            tiny_skia::Transform::identity(),
            None,
        );
    }
    if let Some(stroke) = stroke {
        // Centred on the edge with mitred corners, as the render draws it.
        let style = tiny_skia::Stroke {
            width: (stroke.width * scale) as f32,
            line_join: tiny_skia::LineJoin::Miter,
            line_cap: tiny_skia::LineCap::Butt,
            ..tiny_skia::Stroke::default()
        };
        pixmap.stroke_path(
            &outline,
            &paint(screen_color(stroke.color)),
            &style,
            tiny_skia::Transform::identity(),
            None,
        );
    }
    let image =
        egui::ColorImage::from_rgba_premultiplied([pw as usize, ph as usize], pixmap.data());
    Some((image, covered))
}

/// The longest side of a light texture, in px: the light is soft, so a
/// larger one adds time and no detail (spec 065).
const LIGHT_TEXTURE_MAX: f64 = 1024.0;

/// The texture of a light (spec 065) at `zoom` and the box it covers in
/// page px: the lights that span the page cover the page of `page_px`, the
/// others the box of the shape. The same pixels as the export (gafira-raster).
pub(crate) fn light_image(
    shape: &Shape,
    page_px: (f64, f64),
    zoom: f32,
) -> Option<(egui::ColorImage, Placement)> {
    let effect = shape.effect.as_ref()?;
    let (w, h) = (shape.placement.w, shape.placement.h);
    let zoom = f64::from(zoom);
    if !(w > 0.0 && h > 0.0 && zoom.is_finite() && zoom > 0.0) {
        return None;
    }
    let fit = |w: f64, h: f64| {
        let k = zoom.min(LIGHT_TEXTURE_MAX / w.max(h));
        // Within 1..=1024: the casts are exact.
        (
            (w * k).round().max(1.0) as u32,
            (h * k).round().max(1.0) as u32,
        )
    };
    let (image, cover) = if gafira_raster::spans_page(effect) {
        let (pw, ph) = page_px;
        if !(pw > 0.0 && ph > 0.0) {
            return None;
        }
        let (iw, ih) = fit(pw, ph);
        let (kx, ky) = (f64::from(iw) / pw, f64::from(ih) / ph);
        let centre = shape.placement.to_transform() * kurbo::Point::new(w / 2.0, h / 2.0);
        let spot = gafira_raster::LightSpot {
            centre: (centre.x * kx, centre.y * ky),
            radius: w.min(h) / 2.0 * kx.min(ky),
        };
        let cover = Placement::new(0.0, 0.0, pw, ph);
        (gafira_raster::light_picture(effect, iw, ih, spot), cover)
    } else {
        let (iw, ih) = fit(w, h);
        let spot = gafira_raster::LightSpot {
            centre: (f64::from(iw) / 2.0, f64::from(ih) / 2.0),
            radius: f64::from(iw.min(ih)) / 2.0,
        };
        (
            gafira_raster::light_picture(effect, iw, ih, spot),
            shape.placement,
        )
    };
    let gafira_raster::PixelBuf::Rgba8(data) = image.data() else {
        return None;
    };
    let size = [image.width() as usize, image.height() as usize];
    Some((egui::ColorImage::from_rgba_premultiplied(size, data), cover))
}

/// The box a light texture covers, without drawing it (spec 065).
pub(crate) fn light_image_cover(shape: &Shape, page_px: (f64, f64)) -> Option<Placement> {
    let effect = shape.effect.as_ref()?;
    Some(if gafira_raster::spans_page(effect) {
        Placement::new(0.0, 0.0, page_px.0, page_px.1)
    } else {
        shape.placement
    })
}

/// The sample photo of the frames (spec 066, S66-12): what an empty frame
/// shows on the canvas and in Elementos → Marcos, so the shape reads as a
/// place for a photo. Not exported (FD-18 c). Decoded once, at most 1024 px
/// a side.
pub(crate) fn sample_photo() -> Option<std::sync::Arc<egui::ColorImage>> {
    static SAMPLE: std::sync::OnceLock<Option<std::sync::Arc<egui::ColorImage>>> =
        std::sync::OnceLock::new();
    SAMPLE
        .get_or_init(|| {
            let bytes = include_bytes!("../../../assets/samples/frame-sample.png");
            let decoded = image::load_from_memory(bytes).ok()?.thumbnail(1024, 1024);
            let rgba = decoded.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            Some(std::sync::Arc::new(
                egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()),
            ))
        })
        .clone()
}

/// `shape` as an empty frame shows it: with `sample` as its photo.
pub(crate) fn with_sample(shape: &Shape, sample: &egui::ColorImage) -> Shape {
    let mut shown = shape.clone();
    if let Some(frame) = shown.frame.as_mut()
        && frame.content.is_none()
    {
        frame.content = Some(gafira_doc::FrameContent {
            asset: gafira_doc::timeline::AssetRef::new(gafira_doc::ContentHash([0; 32])),
            natural: (sample.size[0] as u32, sample.size[1] as u32),
        });
    }
    shown
}

/// True when the canvas draws `shape` from a texture: a path (spec 054) or
/// a shape filled with a gradient (spec 063).
pub(crate) fn textured(shape: &Shape) -> bool {
    shape.line.is_none()
        && shape.text.is_none()
        && shape.image.is_none()
        && !shape.group
        && (matches!(shape.geometry, ShapeGeometry::Path(_)) || shape.gradient.is_some())
}

/// The outline of a shape in its local box, as the render draws it: the
/// path, the ellipse, or the rectangle with its corners rounded at most to
/// half the shorter side (FD-36 a).
fn outline_of(shape: &Shape) -> kurbo::BezPath {
    use kurbo::Shape as _;
    let (w, h) = (shape.placement.w, shape.placement.h);
    match &shape.geometry {
        ShapeGeometry::Path(path) => crate::shapes::bez(path, w, h),
        ShapeGeometry::Ellipse => {
            kurbo::Ellipse::from_rect(kurbo::Rect::new(0.0, 0.0, w, h)).to_path(0.1)
        }
        ShapeGeometry::Rect => {
            let r = shape.corner_radius.max(0.0).min(w.min(h) / 2.0);
            kurbo::RoundedRect::new(0.0, 0.0, w, h, r).to_path(0.1)
        }
    }
}

/// The tiny-skia shader of `gradient` in a `w` × `h` box drawn at `scale`
/// with `margin` page px around it: the same ends as
/// The longest side of a page background texture, in px.
const BACKGROUND_MAX: f64 = 2048.0;

/// The texture of the background of a page (spec 069) of `page_px` at
/// `zoom`: a gradient, a pattern or an image (`photo`, its decoded pixels);
/// `None` for a plain colour, painted directly. The same drawing as the
/// export.
pub(crate) fn background_image(
    background: &gafira_doc::PageBackground,
    (pw, ph): (f64, f64),
    zoom: f32,
    photo: Option<&egui::ColorImage>,
) -> Option<egui::ColorImage> {
    use gafira_doc::{BackgroundFit, PageBackground};
    if !(pw > 0.0 && ph > 0.0) {
        return None;
    }
    let k = f64::from(zoom).min(BACKGROUND_MAX / pw.max(ph));
    // Within 1..=2048: the casts are exact.
    let (iw, ih) = (
        (pw * k).round().max(1.0) as u32,
        (ph * k).round().max(1.0) as u32,
    );
    let to_image = |pixmap: tiny_skia::Pixmap| {
        egui::ColorImage::from_rgba_premultiplied([iw as usize, ih as usize], pixmap.data())
    };
    let rect = tiny_skia::Rect::from_xywh(0.0, 0.0, iw as f32, ih as f32)?;
    match background {
        PageBackground::Solid(_) => None,
        PageBackground::Gradient(gradient) => {
            let shader = gradient_shader(gradient, (pw, ph), (0.0, k))?;
            let mut pixmap = tiny_skia::Pixmap::new(iw, ih)?;
            let paint = tiny_skia::Paint {
                shader,
                ..tiny_skia::Paint::default()
            };
            pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
            Some(to_image(pixmap))
        }
        PageBackground::Pattern { kind, color, back } => {
            // The same vector shapes as the page, drawn small (spec 069).
            let mut pixmap = tiny_skia::Pixmap::new(iw, ih)?;
            let paint = |c: Color| {
                let c = screen_color(c);
                let mut paint = tiny_skia::Paint::default();
                paint.set_color_rgba8(c.r(), c.g(), c.b(), c.a());
                paint.anti_alias = true;
                paint
            };
            pixmap.fill_rect(rect, &paint(*back), tiny_skia::Transform::identity(), None);
            let path = gafira_raster::pattern_path(*kind, pw, ph, gafira_raster::PATTERN_CELL);
            let mut builder = tiny_skia::PathBuilder::new();
            for el in path.elements() {
                match *el {
                    kurbo::PathEl::MoveTo(p) => builder.move_to(p.x as f32, p.y as f32),
                    kurbo::PathEl::LineTo(p) => builder.line_to(p.x as f32, p.y as f32),
                    kurbo::PathEl::QuadTo(c, p) => {
                        builder.quad_to(c.x as f32, c.y as f32, p.x as f32, p.y as f32);
                    }
                    kurbo::PathEl::CurveTo(a, b, p) => builder.cubic_to(
                        a.x as f32, a.y as f32, b.x as f32, b.y as f32, p.x as f32, p.y as f32,
                    ),
                    kurbo::PathEl::ClosePath => builder.close(),
                }
            }
            if let Some(shapes) = builder.finish() {
                pixmap.fill_path(
                    &shapes,
                    &paint(*color),
                    tiny_skia::FillRule::Winding,
                    tiny_skia::Transform::from_scale(k as f32, k as f32),
                    None,
                );
            }
            Some(to_image(pixmap))
        }
        PageBackground::Image { natural, fit, .. } => {
            let photo = photo?;
            let [sw, sh] = photo.size;
            let mut bytes = Vec::with_capacity(sw * sh * 4);
            for pixel in &photo.pixels {
                bytes.extend(pixel.to_array());
            }
            let source = tiny_skia::Pixmap::from_vec(
                bytes,
                tiny_skia::IntSize::from_wh(u32::try_from(sw).ok()?, u32::try_from(sh).ok()?)?,
            )?;
            let (nw, nh) = (f64::from(natural.0), f64::from(natural.1));
            let (scale, x, y, spread) = match fit {
                BackgroundFit::Fill | BackgroundFit::Fit => {
                    let s = if *fit == BackgroundFit::Fill {
                        (pw / nw).max(ph / nh)
                    } else {
                        (pw / nw).min(ph / nh)
                    };
                    (
                        s,
                        (pw - nw * s) / 2.0,
                        (ph - nh * s) / 2.0,
                        tiny_skia::SpreadMode::Pad,
                    )
                }
                BackgroundFit::Tile => (1.0, 0.0, 0.0, tiny_skia::SpreadMode::Repeat),
            };
            // Photo px → natural px → page px → texture px.
            let (sx, sy) = (nw / sw as f64 * scale * k, nh / sh as f64 * scale * k);
            let shader = tiny_skia::Pattern::new(
                source.as_ref(),
                spread,
                tiny_skia::FilterQuality::Bilinear,
                1.0,
                tiny_skia::Transform::from_row(
                    sx as f32,
                    0.0,
                    0.0,
                    sy as f32,
                    (x * k) as f32,
                    (y * k) as f32,
                ),
            );
            let mut pixmap = tiny_skia::Pixmap::new(iw, ih)?;
            let paint = tiny_skia::Paint {
                shader,
                ..tiny_skia::Paint::default()
            };
            // «Encajar» leaves the rest of the page as it is.
            let covered = if *fit == BackgroundFit::Fit {
                tiny_skia::Rect::from_xywh(
                    (x * k) as f32,
                    (y * k) as f32,
                    (nw * scale * k) as f32,
                    (nh * scale * k) as f32,
                )?
            } else {
                rect
            };
            pixmap.fill_rect(covered, &paint, tiny_skia::Transform::identity(), None);
            Some(to_image(pixmap))
        }
    }
}

/// `gafira_doc::Gradient::position` (spec 063).
pub(crate) fn gradient_shader(
    gradient: &gafira_doc::Gradient,
    (w, h): (f64, f64),
    (margin, scale): (f64, f64),
) -> Option<tiny_skia::Shader<'static>> {
    let stops = gradient
        .stops
        .iter()
        .enumerate()
        .map(|(index, stop)| {
            let c = gradient.stop_rgba(index);
            let color = tiny_skia::Color::from_rgba(
                c.r.clamp(0.0, 1.0),
                c.g.clamp(0.0, 1.0),
                c.b.clamp(0.0, 1.0),
                c.a.clamp(0.0, 1.0),
            )
            .unwrap_or(tiny_skia::Color::TRANSPARENT);
            tiny_skia::GradientStop::new(stop.offset, color)
        })
        .collect();
    let at = |x: f64, y: f64| {
        tiny_skia::Point::from_xy(((x + margin) * scale) as f32, ((y + margin) * scale) as f32)
    };
    let (cx, cy) = (w / 2.0, h / 2.0);
    match gradient.kind {
        gafira_doc::GradientKind::Linear { angle_deg } => {
            let (sin, cos) = angle_deg.to_radians().sin_cos();
            let half = (cx * cos).abs() + (cy * sin).abs();
            tiny_skia::LinearGradient::new(
                at(cx - cos * half, cy - sin * half),
                at(cx + cos * half, cy + sin * half),
                stops,
                tiny_skia::SpreadMode::Pad,
                tiny_skia::Transform::identity(),
            )
        }
        gafira_doc::GradientKind::Radial => {
            let centre = at(cx, cy);
            tiny_skia::RadialGradient::new(
                centre,
                0.0,
                centre,
                (cx.hypot(cy) * scale) as f32,
                stops,
                tiny_skia::SpreadMode::Pad,
                tiny_skia::Transform::identity(),
            )
        }
    }
}

/// An image on its box mapped by `transform`: one textured quad whose
/// texture corners follow the local box, so a flip mirrors the picture
/// (spec 041).
pub(crate) fn image_mesh(
    texture: egui::TextureId,
    transform: Affine,
    placement: &Placement,
    tint: Color32,
) -> egui::Mesh {
    image_mesh_with(texture, transform, placement, (0.0, 0.0), tint)
}

/// [`image_mesh`] of the part of the texture from `from`, `size` of it, in
/// shares of its sides (spec 088).
pub(crate) fn image_mesh_part(
    texture: egui::TextureId,
    transform: Affine,
    placement: &Placement,
    (from, size): ((f64, f64), (f64, f64)),
    tint: Color32,
) -> egui::Mesh {
    let mut mesh = image_mesh(texture, transform, placement, tint);
    for vertex in &mut mesh.vertices {
        let part = |uv: f32, from: f64, size: f64| (from + f64::from(uv) * size) as f32;
        vertex.uv = Pos2::new(
            part(vertex.uv.x, from.0, size.0),
            part(vertex.uv.y, from.1, size.1),
        );
    }
    mesh
}

/// [`image_mesh`] that reaches past its box, across and down (spec 085).
pub(crate) fn image_mesh_with(
    texture: egui::TextureId,
    transform: Affine,
    placement: &Placement,
    (bx, by): (f64, f64),
    tint: Color32,
) -> egui::Mesh {
    let mut mesh = egui::Mesh::with_texture(texture);
    let (w, h) = (placement.w, placement.h);
    for (u, v) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: rotation::screen(transform, u * (w + 2.0 * bx) - bx, v * (h + 2.0 * by) - by),
            // 0 or 1: the casts are exact.
            uv: Pos2::new(u as f32, v as f32),
            color: tint,
        });
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    mesh
}

/// The fill a new shape takes (spec 053): a light grey, #CCD1D9. It is
/// document content, so it does not follow the light or dark theme.
pub(crate) fn default_fill() -> Color {
    Color::Srgb(Rgba::new(0.8, 0.82, 0.851, 1.0))
}

/// A theme colour as a document colour, for the default fill of a piece.
pub(crate) fn document_color(color: Color32) -> Color {
    let channel = |v: u8| f32::from(v) / 255.0;
    Color::Srgb(Rgba::new(
        channel(color.r()),
        channel(color.g()),
        channel(color.b()),
        1.0,
    ))
}

/// The colour a picker shows for `color`: unmultiplied sRGB bytes; no
/// colour shows as transparent.
pub(crate) fn picker_rgba(color: Option<Color>) -> [u8; 4] {
    let Some(Color::Srgb(Rgba { r, g, b, a })) = color.map(|c| c.convert(ColorSpaceId::Srgb))
    else {
        return [0, 0, 0, 0];
    };
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    [byte(r), byte(g), byte(b), byte(a)]
}

/// The colour picked as `rgba` (unmultiplied sRGB bytes) over `before`
/// (spec 063): its transparency is kept, so a colour can fade out; picked
/// over no colour, a transparent pick is made opaque, or it would not show.
pub(crate) fn picked_color(before: Option<Color>, rgba: [u8; 4]) -> Color {
    let channel = |v: u8| f32::from(v) / 255.0;
    let alpha = if before.is_none() && rgba[3] == 0 {
        1.0
    } else {
        channel(rgba[3])
    };
    Color::Srgb(Rgba::new(
        channel(rgba[0]),
        channel(rgba[1]),
        channel(rgba[2]),
        alpha,
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gafira_doc::{ElementBase, GroupNode, Node, PageSize};
    use gafira_geom::{Length, Unit};

    use super::*;

    fn place(x: f64, y: f64, w: f64, h: f64) -> Placement {
        Placement::new(x, y, w, h)
    }

    /// The resize handle of the upright screen box `rect` under `pointer`:
    /// [`grip_hit`] on a page at the origin at zoom 1.
    fn handle_hit(rect: Rect, pointer: Pos2) -> Option<Handle> {
        let p = place(
            f64::from(rect.min.x),
            f64::from(rect.min.y),
            f64::from(rect.width()),
            f64::from(rect.height()),
        );
        let page = Rect::from_min_size(Pos2::ZERO, Vec2::splat(1.0));
        match grip_hit(rotation::to_screen(page, 1.0, &p), &p, pointer, false)? {
            Grip::Resize(handle) => Some(handle),
            Grip::Rotate => None,
        }
    }

    fn page() -> Page {
        let size = PageSize {
            width: Length::new(100.0, Unit::Px),
            height: Length::new(100.0, Unit::Px),
        };
        let mut page = Page::new(PageId::new(1), String::new(), size);
        let shape = |id: u64, geometry, at| {
            let base = ElementBase::new(NodeId::new(id), at);
            Arc::new(Node::new(
                base,
                NodeKind::Shape(ShapeNode {
                    geometry,
                    fill: None,
                    stroke: None,
                    corner_radius: 0.0,
                    text: None,
                    gradient: None,
                    effect: None,
                    drawn_with: None,
                }),
            ))
        };
        let mut group = Node::new(
            ElementBase::new(NodeId::new(3), place(0.0, 0.0, 100.0, 100.0)),
            NodeKind::Group(GroupNode {}),
        );
        group.children = vec![NodeId::new(4)];
        let mut hidden = ElementBase::new(NodeId::new(5), place(0.0, 0.0, 100.0, 100.0));
        hidden.visible = false;
        page.nodes.insert(
            NodeId::new(2),
            shape(2, ShapeGeometry::Rect, place(10.0, 10.0, 50.0, 50.0)),
        );
        page.nodes.insert(NodeId::new(3), Arc::new(group));
        page.nodes.insert(
            NodeId::new(4),
            shape(4, ShapeGeometry::Ellipse, place(40.0, 40.0, 40.0, 40.0)),
        );
        page.nodes.insert(
            NodeId::new(5),
            Arc::new(Node::new(
                hidden,
                NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect)),
            )),
        );
        page.roots = vec![NodeId::new(2), NodeId::new(3), NodeId::new(5)];
        page
    }

    #[test]
    fn s20_01_shapes_come_back_to_front_without_groups_or_hidden_ones() {
        let ids: Vec<u64> = shapes(&page()).iter().map(|s| s.node.get()).collect();
        assert_eq!(ids, [2, 4]);
    }

    #[test]
    fn s20_02_the_topmost_shape_under_the_pointer_is_hit() {
        let shapes = shapes(&page());
        let page_rect = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::splat(200.0));
        let at = |x: f32, y: f32| Pos2::new(100.0 + x * 2.0, 100.0 + y * 2.0);
        assert_eq!(
            hit(&shapes, page_rect, 2.0, at(55.0, 55.0)),
            Some(NodeId::new(4)),
            "the ellipse is on top"
        );
        assert_eq!(
            hit(&shapes, page_rect, 2.0, at(15.0, 15.0)),
            Some(NodeId::new(2))
        );
        assert_eq!(
            hit(&shapes, page_rect, 2.0, at(42.0, 42.0)),
            Some(NodeId::new(2)),
            "outside the ellipse's curve"
        );
        assert_eq!(hit(&shapes, page_rect, 2.0, at(90.0, 5.0)), None);
    }

    #[test]
    fn s20_03_a_drag_moves_by_the_offset_in_page_space() {
        let origin = place(10.0, 20.0, 30.0, 40.0);
        let to = moved(&origin, Vec2::new(50.0, -20.0), 2.0);
        assert_eq!((to.x, to.y, to.w, to.h), (35.0, 10.0, 30.0, 40.0));
        let rect = on_screen(Rect::from_min_size(Pos2::ZERO, Vec2::splat(1.0)), 2.0, &to);
        assert_eq!(
            rect,
            Rect::from_min_size(Pos2::new(70.0, 20.0), Vec2::new(60.0, 80.0))
        );
    }

    #[test]
    fn s20_04_a_new_piece_is_centred_and_a_third_of_the_shorter_side() {
        let fill = document_color(Color32::from_rgb(255, 0, 0));
        let spec = new_piece(Vec2::new(1920.0, 1080.0), ShapeGeometry::Ellipse, fill);
        assert_eq!((spec.placement.w, spec.placement.h), (360.0, 360.0));
        assert_eq!((spec.placement.x, spec.placement.y), (780.0, 360.0));
        assert_eq!(screen_color(fill), Color32::from_rgb(255, 0, 0));
    }

    // The hit area is the 24 px target of 03, wider than the drawn square,
    // and the nearest handle wins on a small element.
    #[test]
    fn handles_are_hit_within_24_px_and_the_nearest_wins() {
        let rect = Rect::from_min_max(Pos2::new(100.0, 100.0), Pos2::new(300.0, 200.0));
        assert_eq!(
            handle_hit(rect, Pos2::new(311.0, 89.0)),
            Some(Handle { x: 1, y: -1 }),
            "11 px off the corner still grabs it"
        );
        assert_eq!(
            handle_hit(rect, Pos2::new(313.0, 100.0)),
            None,
            "13 px is outside"
        );
        let small = Rect::from_min_max(Pos2::new(100.0, 100.0), Pos2::new(116.0, 116.0));
        assert_eq!(
            handle_hit(small, Pos2::new(115.0, 115.0)),
            Some(Handle { x: 1, y: 1 }),
            "the corner nearest the pointer, not the first in the list"
        );
        assert_eq!(
            handle_hit(small, Pos2::new(108.0, 99.0)),
            Some(Handle { x: 0, y: -1 })
        );
    }

    #[test]
    fn s22_01_handles_sit_on_corners_and_edge_midpoints_and_are_hit() {
        let rect = Rect::from_min_max(Pos2::new(10.0, 20.0), Pos2::new(110.0, 70.0));
        assert_eq!(
            handle_at(rect, Handle { x: -1, y: -1 }),
            Pos2::new(10.0, 20.0)
        );
        assert_eq!(
            handle_at(rect, Handle { x: 1, y: 0 }),
            Pos2::new(110.0, 45.0)
        );
        assert_eq!(
            handle_at(rect, Handle { x: 0, y: 1 }),
            Pos2::new(60.0, 70.0)
        );
        assert_eq!(
            handle_hit(rect, Pos2::new(113.0, 47.0)),
            Some(Handle { x: 1, y: 0 })
        );
        assert_eq!(
            handle_hit(rect, Pos2::new(60.0, 45.0)),
            None,
            "the middle is not a handle"
        );
    }

    #[test]
    fn s22_02_resizing_keeps_the_opposite_edge_and_a_minimum_side() {
        let origin = place(10.0, 10.0, 100.0, 50.0);
        let bigger = resized(&origin, Handle { x: 1, y: 1 }, Vec2::new(20.0, 10.0), 2.0);
        assert_eq!(
            (bigger.x, bigger.y, bigger.w, bigger.h),
            (10.0, 10.0, 110.0, 55.0)
        );
        let from_left = resized(&origin, Handle { x: -1, y: 0 }, Vec2::new(20.0, 99.0), 2.0);
        assert_eq!(
            (from_left.x, from_left.y, from_left.w, from_left.h),
            (20.0, 10.0, 90.0, 50.0)
        );
        let crushed = resized(
            &origin,
            Handle { x: -1, y: -1 },
            Vec2::new(1000.0, 1000.0),
            1.0,
        );
        assert_eq!(
            (crushed.x + crushed.w, crushed.y + crushed.h),
            (110.0, 60.0),
            "opposite corner stays"
        );
        assert_eq!((crushed.w, crushed.h), (1.0, 1.0));
    }

    #[test]
    fn s23_01_the_paint_carries_fill_and_opacity() {
        let red = document_color(Color32::from_rgb(255, 0, 0));
        let mut shape = shapes(&page())[0].clone();
        assert_eq!(
            paint(&shape),
            Color32::TRANSPARENT,
            "no fill, nothing painted"
        );
        shape.fill = Some(red);
        assert_eq!(paint(&shape), Color32::from_rgb(255, 0, 0));
        shape.opacity = 0.5;
        assert_eq!(paint(&shape).a(), 128);
        shape.opacity = 0.0;
        assert_eq!(paint(&shape).a(), 0);
    }

    // S38-07: the canvas paints the outline centred on the edge, scaled by
    // the zoom and faded by the opacity, and the radius as the render does:
    // at most half the shorter side.
    #[test]
    fn s38_07_the_canvas_paints_the_outline_and_the_rounded_corners() {
        let blue = document_color(Color32::from_rgb(0, 0, 255));
        let mut shape = shapes(&page())[0].clone();
        assert_eq!(shape.stroke, None);
        assert_eq!(shape.corner_radius, 0.0);
        let at = Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(100.0, 40.0));
        let egui::Shape::Rect(plain) = painted(&shape, at, 2.0) else {
            panic!("a rectangle")
        };
        assert_eq!(plain.stroke, egui::Stroke::NONE);
        assert_eq!(plain.corner_radius, egui::CornerRadius::ZERO);
        shape.stroke = Some(gafira_doc::ShapeStroke::new(blue, 3.0));
        shape.corner_radius = 8.0;
        shape.opacity = 0.5;
        let egui::Shape::Rect(outlined) = painted(&shape, at, 2.0) else {
            panic!("a rectangle")
        };
        assert_eq!(outlined.rect, at);
        assert_eq!(outlined.stroke.width, 6.0);
        assert_eq!(outlined.stroke.color.a(), 128);
        assert_eq!(outlined.stroke_kind, egui::StrokeKind::Middle);
        // 8 px at zoom 2 is 16 screen px, under half the short side (20).
        assert_eq!(outlined.corner_radius, egui::CornerRadius::same(16));
        shape.corner_radius = 1e9;
        let egui::Shape::Rect(capped) = painted(&shape, at, 2.0) else {
            panic!("a rectangle")
        };
        assert_eq!(capped.corner_radius, egui::CornerRadius::same(20));
        // A zero width paints no outline.
        shape.stroke = Some(gafira_doc::ShapeStroke::new(blue, 0.0));
        let egui::Shape::Rect(zero) = painted(&shape, at, 2.0) else {
            panic!("a rectangle")
        };
        assert_eq!(zero.stroke.width, 0.0);
        // An ellipse carries the outline and ignores the radius.
        shape.geometry = ShapeGeometry::Ellipse;
        shape.stroke = Some(gafira_doc::ShapeStroke::new(blue, 3.0));
        let egui::Shape::Ellipse(ellipse) = painted(&shape, at, 1.0) else {
            panic!("an ellipse")
        };
        assert_eq!(ellipse.center, at.center());
        assert_eq!(ellipse.radius, Vec2::new(50.0, 20.0));
        assert_eq!(ellipse.stroke.width, 3.0);
    }

    #[test]
    fn s24_01_layers_list_the_roots_front_first_with_their_kind() {
        let rows = layers(&page(), &std::collections::BTreeSet::new());
        let summary: Vec<(u64, &str, usize)> = rows
            .iter()
            .map(|l| (l.node.get(), l.kind, l.index))
            .collect();
        assert_eq!(
            summary,
            [
                (5, "pieces.rect", 2),
                (3, "layers.group", 1),
                (2, "pieces.rect", 0)
            ]
        );
    }

    // S43-11: an open group lists its children below it, one step in, front
    // first, with their index among their siblings; a closed one does not.
    #[test]
    fn s43_11_layers_open_a_group_to_list_its_children() {
        let page = page();
        let open: std::collections::BTreeSet<NodeId> = [NodeId::new(3)].into_iter().collect();
        let rows = layers(&page, &open);
        let summary: Vec<(u64, usize, usize, Option<u64>, bool)> = rows
            .iter()
            .map(|l| {
                (
                    l.node.get(),
                    l.depth,
                    l.index,
                    l.parent.map(NodeId::get),
                    l.opens,
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                (5, 0, 2, None, false),
                (3, 0, 1, None, true),
                (4, 1, 0, Some(3), false),
                (2, 0, 0, None, false),
            ]
        );
        assert_eq!(rows[2].siblings, 1);
        assert_eq!(rows[0].siblings, 3);
        let closed = layers(&page, &std::collections::BTreeSet::new());
        assert_eq!(closed.len(), 3);
    }

    // S43-11: a group's opacity fades what is below it on the canvas; the
    // element keeps its own opacity for the Inspector.
    #[test]
    fn s43_11_a_group_fades_its_children() {
        let mut page = page();
        let mut group = page.node(NodeId::new(3)).expect("the group").clone();
        group.base.opacity = 0.5;
        page.nodes.insert(NodeId::new(3), Arc::new(group));
        let all = shapes(&page);
        let ellipse = all
            .iter()
            .find(|s| s.node == NodeId::new(4))
            .expect("drawn");
        assert_eq!(ellipse.opacity, 1.0);
        assert_eq!(ellipse.alpha(), 0.5);
        let rect = all
            .iter()
            .find(|s| s.node == NodeId::new(2))
            .expect("drawn");
        assert_eq!(rect.alpha(), 1.0);
    }

    #[test]
    fn s27_03_a_text_element_is_listed_hit_and_placed_like_a_box() {
        let style = SpanStyle {
            font: "Inter".to_owned(),
            size: 40.0,
            color: document_color(Color32::BLACK),
            bold: false,
            italic: false,
        };
        let spec = new_text(Vec2::new(1000.0, 1000.0), "Hola", &style);
        // As wide as «Hola» (spec 095), centred on the page.
        let p = spec.placement;
        assert!(p.w > 40.0 && p.w < 200.0, "{}", p.w);
        assert_eq!(p.h, 56.0);
        assert_eq!((p.x + p.w / 2.0, p.y), (500.0, 472.0));
        let mut page = page();
        let base = gafira_doc::ElementBase::new(NodeId::new(9), spec.placement);
        page.nodes
            .insert(NodeId::new(9), Arc::new(Node::new(base, spec_kind(&spec))));
        page.roots.push(NodeId::new(9));
        let all = shapes(&page);
        let text = all.last().expect("the text is drawn last");
        assert_eq!(text.node, NodeId::new(9));
        assert_eq!(
            text.text.as_ref().map(|t| t.content.plain_text()),
            Some("Hola".to_owned())
        );
        assert_eq!(
            layers(&page, &std::collections::BTreeSet::new())[0].kind,
            "layers.text"
        );
        let page_rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(1000.0));
        assert_eq!(
            hit(&all, page_rect, 1.0, Pos2::new(500.0, 500.0)),
            Some(NodeId::new(9))
        );
    }

    fn spec_kind(spec: &NodeSpec) -> NodeKind {
        spec.kind.clone()
    }

    #[test]
    fn s29_02_a_new_image_fits_the_page_keeping_its_aspect_and_never_grows() {
        let asset = gafira_doc::AssetRef::new(gafira_doc::ContentHash([1; 32]));
        let big = new_image(Vec2::new(1000.0, 1000.0), (4000, 2000), asset);
        assert_eq!((big.placement.w, big.placement.h), (600.0, 300.0));
        assert_eq!((big.placement.x, big.placement.y), (200.0, 350.0));
        let small = new_image(Vec2::new(1000.0, 1000.0), (100, 50), asset);
        assert_eq!(
            (small.placement.w, small.placement.h),
            (100.0, 50.0),
            "never enlarged"
        );
        assert!(
            matches!(small.kind, NodeKind::Image(gafira_doc::ImageNode { asset: a, .. }) if a == asset)
        );
    }

    // S40-02: the marquee selects what its box touches, back to front,
    // dragged in any direction; an ellipse counts by its box.
    #[test]
    fn s40_02_the_marquee_selects_the_shapes_it_touches() {
        let shapes = shapes(&page());
        let page_rect = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::splat(200.0));
        let at = |x: f32, y: f32| Pos2::new(100.0 + x * 2.0, 100.0 + y * 2.0);
        let ids = |from: Pos2, to: Pos2| -> Vec<u64> {
            in_marquee(&shapes, page_rect, 2.0, Rect::from_two_pos(from, to))
                .iter()
                .map(|node| node.get())
                .collect()
        };
        assert_eq!(ids(at(0.0, 0.0), at(20.0, 20.0)), [2]);
        assert_eq!(ids(at(45.0, 45.0), at(30.0, 30.0)), [2, 4], "drawn up-left");
        assert_eq!(ids(at(62.0, 0.0), at(99.0, 30.0)), [] as [u64; 0]);
        assert_eq!(
            ids(at(78.0, 78.0), at(99.0, 99.0)),
            [4],
            "the corner of the ellipse's box"
        );
        assert_eq!(
            ids(at(0.0, 0.0), at(99.0, 99.0)),
            [2, 4],
            "hidden stays out"
        );
    }

    // S40-02, S43-04: Ctrl+A takes the drawn roots of the page, back to
    // front, nothing hidden; since spec 043 a group with something drawn
    // below it counts.
    #[test]
    fn s43_04_select_all_takes_the_drawn_roots_and_groups() {
        let roots = |page: &Page| crate::group::items(page, None);
        let mut page = page();
        assert_eq!(roots(&page), [NodeId::new(2), NodeId::new(3)]);
        let size = page.size;
        let mut two = ShapeNode::new(ShapeGeometry::Ellipse);
        two.fill = None;
        page.nodes.insert(
            NodeId::new(6),
            Arc::new(Node::new(
                ElementBase::new(NodeId::new(6), place(0.0, 0.0, 5.0, 5.0)),
                NodeKind::Shape(two),
            )),
        );
        page.roots.insert(0, NodeId::new(6));
        assert_eq!(
            roots(&page),
            [NodeId::new(6), NodeId::new(2), NodeId::new(3)]
        );
        assert!(roots(&Page::new(PageId::new(7), String::new(), size)).is_empty());
    }

    // S40-03: the box of a multi-selection holds every selected box.
    #[test]
    fn s40_03_the_selection_box_is_the_union_of_its_boxes() {
        let a = Rect::from_min_max(Pos2::new(10.0, 20.0), Pos2::new(30.0, 40.0));
        let b = Rect::from_min_max(Pos2::new(-5.0, 25.0), Pos2::new(15.0, 60.0));
        assert_eq!(
            bounds([a, b]),
            Some(Rect::from_min_max(
                Pos2::new(-5.0, 20.0),
                Pos2::new(30.0, 60.0)
            ))
        );
        assert_eq!(bounds([a]), Some(a));
        assert_eq!(bounds([]), None);
    }
}

#[cfg(test)]
mod s41_tests {
    use std::sync::Arc;

    use gafira_doc::{ElementBase, Node, PageSize};
    use gafira_geom::{Length, Point, Unit};

    use super::*;
    use crate::rotation;

    // S53-01: square, rounded square and circle: their geometry, their
    // corner radius (an eighth of the side for the rounded one) and the
    // size of a new piece.
    #[test]
    fn s53_01_basic_pieces() {
        let fill = Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0));
        let page = Vec2::new(900.0, 600.0);
        let cases = [
            (Piece::Square, ShapeGeometry::Rect, 0.0),
            (Piece::RoundedSquare, ShapeGeometry::Rect, 25.0),
            (Piece::Circle, ShapeGeometry::Ellipse, 0.0),
        ];
        for (piece, geometry, radius) in cases {
            let spec = new_shape(page, piece, fill);
            assert_eq!((spec.placement.w, spec.placement.h), (200.0, 200.0));
            let NodeKind::Shape(shape) = spec.kind else {
                panic!("a shape");
            };
            assert_eq!(shape.geometry, geometry, "{piece:?}");
            assert_eq!(shape.corner_radius, radius, "{piece:?}");
        }
    }

    // S53-04: a new shape is filled light grey, #CCD1D9.
    #[test]
    fn s53_04_default_fill_is_light_grey() {
        let Color::Srgb(rgba) = default_fill() else {
            panic!("an sRGB colour");
        };
        let byte = |v: f32| (v * 255.0).round() as u8;
        assert_eq!(
            (byte(rgba.r), byte(rgba.g), byte(rgba.b), rgba.a),
            (0xCC, 0xD1, 0xD9, 1.0)
        );
    }

    // S58-05: «Líneas» comes first with its five lines, and a line from
    // the panel is a line element across the middle of the page.
    #[test]
    fn s58_05_the_lines_come_first() {
        let (title, pieces) = PIECE_SECTIONS[0];
        assert_eq!(title, "pieces.lines");
        assert_eq!(pieces.len(), 5);
        assert!(pieces.iter().all(|p| p.line_caps().is_some()));
        let fill = default_fill();
        let spec = new_shape(Vec2::new(900.0, 600.0), Piece::LineArrow, fill);
        let NodeKind::Line(line) = &spec.kind else {
            panic!("a line");
        };
        assert_eq!(
            (line.start.cap, line.end.cap),
            (gafira_doc::LineCap::Square, gafira_doc::LineCap::Arrow)
        );
        assert_eq!(spec.placement, Placement::new(350.0, 300.0, 200.0, 0.0));
        let dropped = piece_at(
            Vec2::new(900.0, 600.0),
            Piece::LinePlain,
            fill,
            (100.0, 50.0),
        );
        assert_eq!(dropped.placement, Placement::new(0.0, 50.0, 200.0, 0.0));
    }

    // S53-02: the panel lists «Formas básicas» with its three shapes in
    // order, and no section is empty.
    #[test]
    fn s53_02_sections() {
        let (title, pieces) = PIECE_SECTIONS[1];
        assert_eq!(title, "pieces.basic");
        assert_eq!(
            pieces[..3],
            [Piece::Square, Piece::RoundedSquare, Piece::Circle],
            "spec 054 adds more after them"
        );
        assert!(PIECE_SECTIONS.iter().all(|(_, pieces)| !pieces.is_empty()));
    }

    // S55-04: the text of a shape is laid out in its box less a tenth of
    // its shorter side, and starts at the top, the middle or the bottom.
    #[test]
    fn s55_04_text_offset() {
        use gafira_doc::VerticalAlign;
        let inner = text_box(Rect::from_min_size(
            Pos2::new(10.0, 20.0),
            Vec2::new(200.0, 100.0),
        ));
        assert_eq!(
            inner,
            Rect::from_min_max(Pos2::new(20.0, 30.0), Pos2::new(200.0, 110.0))
        );
        assert_eq!(text_top(inner, 20.0, VerticalAlign::Top), 30.0);
        assert_eq!(text_top(inner, 20.0, VerticalAlign::Middle), 60.0);
        assert_eq!(text_top(inner, 20.0, VerticalAlign::Bottom), 90.0);
        assert_eq!(
            text_top(inner, 200.0, VerticalAlign::Middle),
            -30.0,
            "taller than the box"
        );
    }

    // S54-04: a path shape is drawn from a texture that fills its inside
    // and leaves the rest of its box clear; a click inside the path takes
    // it and one in the box but outside the path does not.
    #[test]
    fn s54_04_a_path_draws_and_hits_inside_only() {
        use gafira_doc::{PathCommand, ShapePath};
        let triangle = ShapePath::new(vec![
            PathCommand::MoveTo([0.5, 0.0]),
            PathCommand::LineTo([1.0, 1.0]),
            PathCommand::LineTo([0.0, 1.0]),
            PathCommand::Close,
        ]);
        let mut node = ShapeNode::new(ShapeGeometry::Path(triangle));
        node.fill = Some(Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0)));
        let shape = Shape {
            page: PageId::new(1),
            node: NodeId::new(2),
            placement: Placement::new(0.0, 0.0, 100.0, 100.0),
            geometry: node.geometry.clone(),
            fill: node.fill,
            stroke: None,
            corner_radius: 0.0,
            gradient: None,
            effect: None,
            opacity: 1.0,
            inherited: 1.0,
            group: false,
            text: None,
            image: None,
            locked: false,
            inner: None,
            line: None,
            frame: None,
            qr: None,
            table: None,
            chart: None,
        };
        let (image, covered) = path_image(&shape, 1.0).expect("a texture");
        // One screen px of margin on each side.
        assert_eq!(covered, Placement::new(-1.0, -1.0, 102.0, 102.0));
        assert_eq!(image.size, [102, 102]);
        let at = |x: usize, y: usize| image.pixels[(y + 1) * 102 + x + 1];
        assert_eq!(at(50, 90), Color32::from_rgb(255, 0, 0), "inside");
        assert_eq!(at(5, 5).a(), 0, "a corner of the box");
        let page = Rect::from_min_size(Pos2::ZERO, Vec2::splat(400.0));
        let shapes = [shape];
        assert_eq!(
            hit(&shapes, page, 1.0, Pos2::new(50.0, 90.0)),
            Some(NodeId::new(2))
        );
        assert_eq!(hit(&shapes, page, 1.0, Pos2::new(5.0, 5.0)), None);
        let mut flat = shapes[0].clone();
        flat.placement.h = 0.0;
        assert!(path_image(&flat, 1.0).is_none());
    }

    // S63-05: the picker keeps the transparency picked, so a gradient can
    // run to transparent; over no colour, a transparent pick is opaque.
    #[test]
    fn s63_05_the_picker_keeps_the_alpha() {
        let half = Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 0.5));
        assert_eq!(picker_rgba(Some(half)), [255, 0, 0, 128]);
        assert_eq!(picker_rgba(None), [0, 0, 0, 0]);
        let red = Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0));
        let clear = picked_color(Some(red), [255, 0, 0, 0]);
        assert_eq!(clear, Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 0.0)));
        let faded = picked_color(Some(red), [0, 0, 255, 51]);
        assert_eq!(faded, Color::Srgb(Rgba::new(0.0, 0.0, 1.0, 0.2)));
        assert_eq!(
            picked_color(None, [0, 0, 255, 0]),
            Color::Srgb(Rgba::new(0.0, 0.0, 1.0, 1.0))
        );
    }

    // S63-03: a shape with a gradient is drawn from a texture filled with
    // it, red on the left of a left-to-right gradient and blue on the
    // S66-05: the canvas cuts the photo of a frame to its shape: red inside
    // the circle, nothing in the corners of the box; an empty frame shows
    // the placeholder grey.
    #[test]
    fn s66_05_the_canvas_cuts_the_photo_to_the_shape() {
        use gafira_doc::timeline::AssetRef;
        use gafira_doc::{ContentHash, FrameContent, FrameNode};
        let page_with = |content: Option<FrameContent>| {
            let mut page = Page::new(
                PageId::new(1),
                String::new(),
                gafira_doc::PageSize {
                    width: gafira_geom::Length::new(400.0, gafira_geom::Unit::Px),
                    height: gafira_geom::Length::new(400.0, gafira_geom::Unit::Px),
                },
            );
            let mut frame = FrameNode::new(ShapeGeometry::Ellipse);
            frame.content = content;
            let base = gafira_doc::ElementBase::new(
                NodeId::new(2),
                Placement::new(0.0, 0.0, 100.0, 100.0),
            );
            page.nodes.insert(
                NodeId::new(2),
                std::sync::Arc::new(gafira_doc::Node::new(base, NodeKind::Frame(frame))),
            );
            page.roots = vec![NodeId::new(2)];
            page
        };
        let content = FrameContent {
            asset: AssetRef::new(ContentHash([5; 32])),
            natural: (8, 8),
        };
        let shape = shapes(&page_with(Some(content))).remove(0);
        assert!(shape.fill.is_none(), "a frame has no fill of its own");
        let photo = egui::ColorImage::new([8, 8], vec![Color32::from_rgb(255, 0, 0); 64]);
        let image = frame_image(&shape, &photo, 1.0).expect("a texture");
        assert_eq!(image.size, [100, 100]);
        assert_eq!(image.pixels[50 * 100 + 50], Color32::from_rgb(255, 0, 0));
        assert_eq!(image.pixels[2 * 100 + 2].a(), 0, "the corner is cut");
        // An empty frame shows the sample photo, cut out the same way.
        let empty = shapes(&page_with(None)).remove(0);
        assert!(empty.fill.is_none());
        assert!(
            frame_image(&empty, &photo, 1.0).is_none(),
            "no photo of its own"
        );
        let sample = sample_photo().expect("the sample decodes");
        let shown = frame_image(&with_sample(&empty, &sample), &sample, 1.0).expect("sample");
        assert!(
            shown.pixels[50 * 100 + 50].a() > 0,
            "the sample shows inside"
        );
        assert_eq!(shown.pixels[2 * 100 + 2].a(), 0, "and is cut at the corner");
        assert!(
            crate::frames::spec(
                crate::frames::Kind::Piece(Piece::Heart),
                (44.0, 44.0),
                (22.0, 22.0)
            )
            .and_then(|spec| spec_preview(&spec, 44.0, 2.0))
            .is_some(),
            "the panel tile"
        );
    }

    // right, inside the rounded corners only.
    #[test]
    fn s63_03_a_gradient_shape_draws_from_a_texture() {
        use gafira_doc::{Gradient, GradientKind, GradientStop};
        let mut shape = shapes(&{
            let mut page = Page::new(
                PageId::new(1),
                String::new(),
                gafira_doc::PageSize {
                    width: gafira_geom::Length::new(400.0, gafira_geom::Unit::Px),
                    height: gafira_geom::Length::new(400.0, gafira_geom::Unit::Px),
                },
            );
            let mut node = ShapeNode::new(ShapeGeometry::Rect);
            node.corner_radius = 20.0;
            node.gradient = Some(Gradient {
                kind: GradientKind::Linear { angle_deg: 0.0 },
                stops: vec![
                    GradientStop {
                        offset: 0.0,
                        color: Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0)),
                    },
                    GradientStop {
                        offset: 1.0,
                        color: Color::Srgb(Rgba::new(0.0, 0.0, 1.0, 1.0)),
                    },
                ],
            });
            let base =
                gafira_doc::ElementBase::new(NodeId::new(2), Placement::new(0.0, 0.0, 100.0, 50.0));
            page.nodes.insert(
                NodeId::new(2),
                std::sync::Arc::new(gafira_doc::Node::new(base, NodeKind::Shape(node))),
            );
            page.roots = vec![NodeId::new(2)];
            page
        })
        .remove(0);
        assert!(textured(&shape));
        let (image, _) = path_image(&shape, 1.0).expect("a texture");
        let at = |x: usize, y: usize| image.pixels[(y + 1) * 102 + x + 1];
        assert!(
            at(2, 25).r() > 200 && at(2, 25).b() < 50,
            "red on the left: {:?}",
            at(2, 25)
        );
        assert!(
            at(97, 25).b() > 200 && at(97, 25).r() < 50,
            "blue on the right"
        );
        assert_eq!(at(1, 1).a(), 0, "outside the rounded corner");
        assert_eq!(
            painted(
                &shape,
                Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 50.0)),
                1.0
            ),
            egui::Shape::Noop
        );
        shape.gradient = None;
        assert!(!textured(&shape));
    }

    // S61-02: a box that grows down with its text takes the text's height;
    // a fixed box does not; half a px or less changes nothing.
    #[test]
    fn s61_02_text_fit() {
        use gafira_doc::TextSizing;
        let at = Placement::new(10.0, 20.0, 300.0, 40.0);
        let grown = text_fit(TextSizing::AutoHeight, &at, 96.0).expect("it grows");
        assert_eq!(grown, Placement::new(10.0, 20.0, 300.0, 96.0));
        assert_eq!(
            text_fit(TextSizing::AutoHeight, &at, 20.0).map(|p| p.h),
            Some(20.0),
            "and shrinks"
        );
        assert_eq!(text_fit(TextSizing::AutoHeight, &at, 40.4), None);
        assert_eq!(text_fit(TextSizing::Fixed, &at, 96.0), None);
        assert_eq!(
            text_fit(TextSizing::AutoWidth, &at, 96.0).map(|p| p.h),
            Some(96.0),
            "a box as wide as its lines is as high as them too"
        );
        assert_eq!(text_fit(TextSizing::AutoHeight, &at, f64::NAN), None);
    }

    // S61-03: a dropped text is centred on the point where it was dropped.
    #[test]
    fn s61_03_text_at() {
        let style = SpanStyle {
            font: "Inter".to_owned(),
            size: 20.0,
            color: Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)),
            bold: false,
            italic: false,
        };
        let spec = text_at(Vec2::new(1000.0, 800.0), "Hola", &style, (300.0, 100.0));
        let p = spec.placement;
        assert_eq!((p.x + p.w / 2.0, p.y + p.h / 2.0), (300.0, 100.0));
        assert_eq!(p.h, 28.0, "the height of a new text");
    }

    fn style_of(size: f64) -> SpanStyle {
        SpanStyle {
            font: "Inter".to_owned(),
            size,
            color: Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)),
            bold: false,
            italic: false,
        }
    }

    // S95-02: a box as wide as its lines keeps the side its lines are
    // aligned to: the left edge, the centre or the right edge.
    #[test]
    fn s95_02_a_box_as_wide_as_its_lines_keeps_its_anchor() {
        use gafira_doc::TextAlign;
        let at = Placement::new(100.0, 20.0, 300.0, 40.0);
        let left = text_width_fit(TextAlign::Left, &at, 120.0).expect("it narrows");
        assert_eq!(left, Placement::new(100.0, 20.0, 120.0, 40.0));
        let centre = text_width_fit(TextAlign::Center, &at, 120.0).expect("it narrows");
        assert_eq!(centre, Placement::new(190.0, 20.0, 120.0, 40.0));
        let right = text_width_fit(TextAlign::Right, &at, 120.0).expect("it narrows");
        assert_eq!(right, Placement::new(280.0, 20.0, 120.0, 40.0));
        let wider = text_width_fit(TextAlign::Center, &at, 500.0).expect("it widens");
        assert_eq!(wider, Placement::new(0.0, 20.0, 500.0, 40.0));
        assert_eq!(text_width_fit(TextAlign::Left, &at, 300.4), None);
        assert_eq!(text_width_fit(TextAlign::Left, &at, f64::NAN), None);
    }

    // S95-03: the width a text needs is that of its widest line, and at
    // that width the export lays every paragraph on one line.
    #[test]
    fn s95_03_the_natural_width_is_the_widest_line_and_wraps_nothing() {
        let style = style_of(40.0);
        let width = |words: &str| natural_width(&TextNode::plain(words, &style)).expect("laid");
        assert!(width("Hola mundo") > width("Hola"));
        assert!(width("Hola\nmundo") < width("Hola mundo"));
        let title = TextNode::plain("Agregar un título de prueba", &style);
        let wide = natural_width(&title).expect("laid");
        let (_, _, one_line) =
            gafira_render::text_outline(&title, (10_000.0, 100.0), text_fonts()).expect("laid");
        let (_, _, at_width) =
            gafira_render::text_outline(&title, (wide, 100.0), text_fonts()).expect("laid");
        assert!(
            (at_width - one_line).abs() < 0.5,
            "{at_width} vs {one_line}"
        );
    }

    // S95-04: a new text grows sideways with its words.
    #[test]
    fn s95_04_a_new_text_is_as_wide_as_its_words() {
        let style = style_of(40.0);
        let spec = new_text(Vec2::new(1080.0, 1080.0), "Hola", &style);
        let NodeKind::Text(text) = &spec.kind else {
            panic!("a text");
        };
        assert_eq!(text.sizing, gafira_doc::TextSizing::AutoWidth);
        assert_eq!(Some(spec.placement.w), natural_width(text));
        let p = spec.placement;
        assert!(
            (p.x + p.w / 2.0 - 540.0).abs() < 1e-9,
            "centred on the page"
        );
    }

    // S95-05: a curved text never leaves its box: the box takes the height
    // of its arc and widens about its centre when the ends go past it.
    #[test]
    fn s95_05_a_curved_text_fits_inside_its_box() {
        use kurbo::Shape as _;
        let style = style_of(40.0);
        for curve in [-100.0, -50.0, 50.0, 100.0] {
            let mut text = TextNode::plain("Texto curvo de prueba largo", &style);
            text.curve = curve;
            let at = Placement::new(100.0, 50.0, 300.0, 60.0);
            let fitted = curved_fit(&text, &at).expect("it changes");
            assert!(
                (fitted.x + fitted.w / 2.0 - 250.0).abs() < 1e-6,
                "{curve}: the centre stays"
            );
            let (path, _, _) =
                gafira_render::text_outline(&text, (fitted.w, fitted.h), text_fonts())
                    .expect("laid");
            let b = path.bounding_box();
            assert!(
                b.x0 >= -0.5 && b.x1 <= fitted.w + 0.5,
                "{curve}: {b:?} in {fitted:?}"
            );
            assert!(
                b.y0 >= -0.5 && b.y1 <= fitted.h + 0.5,
                "{curve}: {b:?} in {fitted:?}"
            );
            assert_eq!(
                curved_fit(&text, &fitted),
                None,
                "{curve}: a fitted box stays"
            );
        }
    }

    // S52-02: a shape dropped on a page lands on the point under the
    // pointer, in px of page; in the gap between pages, nowhere.
    #[test]
    fn s52_02_drop_point() {
        let (one, two) = (PageId::new(1), PageId::new(2));
        let pages = [
            (
                one,
                Rect::from_min_size(Pos2::new(100.0, 50.0), Vec2::new(500.0, 400.0)),
            ),
            (
                two,
                Rect::from_min_size(Pos2::new(100.0, 500.0), Vec2::new(500.0, 400.0)),
            ),
        ];
        assert_eq!(
            drop_point(&pages, 0.5, Pos2::new(150.0, 100.0)),
            Some((one, (100.0, 100.0)))
        );
        assert_eq!(
            drop_point(&pages, 0.5, Pos2::new(600.0, 900.0)),
            Some((two, (1000.0, 800.0)))
        );
        assert_eq!(drop_point(&pages, 0.5, Pos2::new(300.0, 475.0)), None);
        assert_eq!(drop_point(&pages, 0.5, Pos2::new(20.0, 100.0)), None);
    }

    // S52-03: the dropped shape has the size of a new piece, centred on the
    // point where it was dropped.
    #[test]
    fn s52_03_piece_at() {
        let fill = Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0));
        let page = Vec2::new(900.0, 600.0);
        let spec = piece_at(page, Piece::Circle, fill, (100.0, 50.0));
        assert_eq!(spec.placement, Placement::new(0.0, -50.0, 200.0, 200.0));
        assert!(matches!(
            spec.kind,
            NodeKind::Shape(ShapeNode {
                geometry: ShapeGeometry::Ellipse,
                ..
            })
        ));
    }

    /// A page of 100 px with one rectangle and one ellipse, both 40 × 10
    /// centred on (50, 50) and turned by `rotation`.
    fn turned_page(rotation: f64) -> Page {
        let size = PageSize {
            width: Length::new(100.0, Unit::Px),
            height: Length::new(100.0, Unit::Px),
        };
        let mut page = Page::new(PageId::new(1), String::new(), size);
        for (id, geometry) in [(2, ShapeGeometry::Rect), (3, ShapeGeometry::Ellipse)] {
            let mut at = Placement::new(30.0, 45.0, 40.0, 10.0);
            at.rotation_deg = rotation;
            let base = ElementBase::new(NodeId::new(id), at);
            page.nodes.insert(
                NodeId::new(id),
                Arc::new(Node::new(base, NodeKind::Shape(ShapeNode::new(geometry)))),
            );
        }
        page
    }

    fn only(page: &Page, id: u64) -> Vec<Shape> {
        let mut page = page.clone();
        page.roots = vec![NodeId::new(id)];
        shapes(&page)
    }

    // S41-10: a click hits the turned shape, not its unturned box.
    #[test]
    fn s41_10_hit_testing_follows_the_rotation() {
        let page = turned_page(90.0);
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0));
        let rect = only(&page, 2);
        // Turned 90°, the 40 × 10 bar stands from y 30 to 70 at x 45..55.
        assert_eq!(
            hit(&rect, screen, 1.0, Pos2::new(50.0, 33.0)),
            Some(NodeId::new(2))
        );
        assert_eq!(
            hit(&rect, screen, 1.0, Pos2::new(33.0, 50.0)),
            None,
            "the old box"
        );
        let ellipse = only(&page, 3);
        assert_eq!(
            hit(&ellipse, screen, 1.0, Pos2::new(50.0, 35.0)),
            Some(NodeId::new(3))
        );
        assert_eq!(hit(&ellipse, screen, 1.0, Pos2::new(35.0, 50.0)), None);
        assert_eq!(
            hit(&ellipse, screen, 1.0, Pos2::new(54.0, 31.0)),
            None,
            "outside the turned curve"
        );
    }

    // S41-10: the marquee counts a turned shape by its axis-aligned box
    // (D-28).
    #[test]
    fn s41_10_the_marquee_uses_the_turned_box() {
        let page = turned_page(90.0);
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0));
        let rect = only(&page, 2);
        let pick = |a: Pos2, b: Pos2| in_marquee(&rect, screen, 1.0, Rect::from_two_pos(a, b));
        assert_eq!(
            pick(Pos2::new(48.0, 25.0), Pos2::new(52.0, 32.0)),
            [NodeId::new(2)]
        );
        assert!(pick(Pos2::new(30.0, 48.0), Pos2::new(40.0, 52.0)).is_empty());
    }

    // S41-11: the handles sit on the turned box and the rotate handle is
    // one of the grips; the nearest grip wins.
    #[test]
    fn s41_11_the_grips_follow_the_turned_box() {
        let mut p = Placement::new(30.0, 45.0, 40.0, 10.0);
        p.rotation_deg = 90.0;
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0));
        let t = rotation::to_screen(screen, 1.0, &p);
        // The local right edge faces down after a quarter turn clockwise.
        let right = Handle { x: 1, y: 0 };
        let at = handle_point(t, &p, right);
        assert!((at - Pos2::new(50.0, 70.0)).length() < 1e-3, "{at:?}");
        assert_eq!(
            grip_hit(t, &p, Pos2::new(51.0, 71.0), false),
            Some(Grip::Resize(right))
        );
        // The top edge faces right: the rotate handle is 24 px past x = 55.
        assert_eq!(
            grip_hit(t, &p, Pos2::new(79.0, 50.0), false),
            Some(Grip::Rotate)
        );
        assert_eq!(
            grip_hit(t, &p, Pos2::new(57.0, 50.0), false),
            Some(Grip::Resize(Handle { x: 0, y: -1 })),
            "the top-middle handle is nearer"
        );
        assert_eq!(
            grip_hit(t, &p, Pos2::new(50.0, 90.0), false),
            None,
            "past the ends"
        );
    }

    // S41-12: resizing a turned element works in its own axes and keeps the
    // opposite edge where it was on the page.
    #[test]
    fn s41_12_resizing_a_turned_element_keeps_the_opposite_edge() {
        let mut origin = Placement::new(30.0, 45.0, 40.0, 10.0);
        origin.rotation_deg = 90.0;
        // The local right edge faces down: dragging it 20 screen px down at
        // zoom 2 makes the element 10 px longer.
        let to = resized(&origin, Handle { x: 1, y: 0 }, Vec2::new(3.0, 20.0), 2.0);
        assert!(
            (to.w - 50.0).abs() < 1e-9 && (to.h - 10.0).abs() < 1e-9,
            "{to:?}"
        );
        assert_eq!(to.rotation_deg, 90.0);
        let left = |p: &Placement| p.to_transform() * Point::new(0.0, p.h / 2.0);
        let (a, b) = (left(&origin), left(&to));
        assert!(
            (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9,
            "{a:?} {b:?}"
        );
        // A flipped element resizes in its local axes too: its local right
        // edge is on the left of the screen.
        let mut mirrored = Placement::new(0.0, 0.0, 40.0, 10.0);
        mirrored.flip.x = true;
        let wider = resized(&mirrored, Handle { x: 1, y: 0 }, Vec2::new(-10.0, 0.0), 1.0);
        assert_eq!((wider.x, wider.w), (-10.0, 50.0));
        // The 1 px floor holds in local axes.
        let crushed = resized(&origin, Handle { x: 1, y: 0 }, Vec2::new(0.0, -1000.0), 1.0);
        assert!((crushed.w - 1.0).abs() < 1e-9);
    }

    // S41-13: a turned shape paints as a polygon on its turned outline, with
    // the outline scaled by the zoom.
    #[test]
    fn s41_13_a_turned_shape_paints_its_turned_outline() {
        let page = turned_page(90.0);
        let mut shape = only(&page, 2).remove(0);
        shape.fill = Some(document_color(Color32::from_rgb(255, 0, 0)));
        shape.stroke = Some(ShapeStroke::new(document_color(Color32::BLACK), 2.0));
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0));
        let t = rotation::to_screen(screen, 2.0, &shape.placement);
        let egui::Shape::Path(path) = painted_turned(&shape, t, 2.0) else {
            panic!("a path")
        };
        assert!(path.closed);
        assert_eq!(path.fill, Color32::from_rgb(255, 0, 0));
        assert_eq!(path.stroke.width, 4.0);
        let want = rotation::corners(t, &shape.placement);
        assert_eq!(path.points.len(), 4);
        for (got, want) in path.points.iter().zip(want) {
            assert!((*got - want).length() < 1e-3, "{got:?} {want:?}");
        }
        // An ellipse's points lie on its turned curve.
        let ellipse = only(&page, 3).remove(0);
        let egui::Shape::Path(curve) = painted_turned(&ellipse, t, 2.0) else {
            panic!("a path")
        };
        assert!(curve.points.len() >= 16);
        for point in &curve.points {
            let local = rotation::local(t, *point).expect("invertible");
            let (u, v) = ((local.x - 20.0) / 20.0, (local.y - 5.0) / 5.0);
            assert!((u * u + v * v - 1.0).abs() < 1e-3, "{local:?}");
        }
        // Rounded corners leave the corner itself out.
        shape.corner_radius = 4.0;
        let egui::Shape::Path(round) = painted_turned(&shape, t, 2.0) else {
            panic!("a path")
        };
        assert!(round.points.len() > 8);
        assert!(round.points.iter().all(|p| (*p - want[0]).length() > 0.5));
    }

    // S41-14: an image is a quad on the turned box whose texture follows the
    // flip.
    #[test]
    fn s41_14_an_image_mesh_follows_rotation_and_flip() {
        let mut p = Placement::new(0.0, 0.0, 40.0, 20.0);
        p.flip.x = true;
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0));
        let t = rotation::to_screen(screen, 1.0, &p);
        let mesh = image_mesh(egui::TextureId::default(), t, &p, Color32::WHITE);
        assert_eq!(mesh.indices.len(), 6);
        let origin = mesh
            .vertices
            .iter()
            .find(|v| v.uv == Pos2::ZERO)
            .expect("the texture's top-left");
        assert!(
            (origin.pos - Pos2::new(40.0, 0.0)).length() < 1e-3,
            "mirrored to the right"
        );
        p.flip.x = false;
        p.rotation_deg = 90.0;
        let t = rotation::to_screen(screen, 1.0, &p);
        let mesh = image_mesh(egui::TextureId::default(), t, &p, Color32::WHITE);
        let origin = mesh.vertices.iter().find(|v| v.uv == Pos2::ZERO).unwrap();
        // Centre (20, 10); the local top-left (-20, -10) turns to (30, -10).
        assert!(
            (origin.pos - Pos2::new(30.0, -10.0)).length() < 1e-3,
            "{:?}",
            origin.pos
        );
    }
}

/// The fonts the canvas draws a curved text with (FD-44): those of the
/// theme and the library, as the export has them.
pub(crate) fn text_fonts() -> &'static gafira_render::FontBook {
    static FONTS: std::sync::OnceLock<gafira_render::FontBook> = std::sync::OnceLock::new();
    FONTS.get_or_init(|| {
        let book = gafira_theme::fonts()
            .iter()
            .fold(gafira_render::FontBook::new(), |book, font| {
                book.with(font.family, font.bytes)
            });
        gafira_theme::library().iter().fold(book, |book, face| {
            book.with(
                &gafira_theme::face_key(face.family, face.bold, face.italic),
                face.bytes,
            )
        })
    })
}

/// The height a text drawn from its outline needs: a curved one down to the
/// lowest letter of its arc and a quarter of its size under it (FD-44), a
/// straight one the height of its lines (FD-41); `None` when it draws
/// nothing.
pub(crate) fn curved_height(text: &TextNode, width: f64, height: f64) -> Option<f64> {
    use kurbo::Shape as _;
    let size = text.content.first_style()?.size;
    let (path, _, lines) = gafira_render::text_outline(text, (width, height), text_fonts())?;
    Some(if text.curve == 0.0 {
        lines
    } else {
        path.bounding_box().y1 + size * 0.25
    })
}

/// The texture of a text drawn from its outline (curved, spaced or with a
/// gradient) at `zoom` and the box it covers:
/// its own box grown the same on every side until the letters fit, so it
/// turns about the same centre. The same outline as the export.
pub(crate) fn curved_text_image(
    text: &TextNode,
    placement: &Placement,
    zoom: f32,
) -> Option<(egui::ColorImage, Placement)> {
    let (w, h) = (placement.w.abs(), placement.h.abs());
    let zoom = f64::from(zoom);
    if !(w > 0.0 && h > 0.0 && zoom.is_finite() && zoom > 0.0) {
        return None;
    }
    let (path, color, _) = gafira_render::text_outline(text, (w, h), text_fonts())?;
    let pieces = gafira_render::text_pieces(text, (w, h), text_fonts(), zoom)?;
    let bbox = gafira_render::pieces_bounds(&pieces, &path);
    let margin = [-bbox.x0, -bbox.y0, bbox.x1 - w, bbox.y1 - h]
        .into_iter()
        .fold(0.0_f64, f64::max)
        + 2.0;
    let cover = Placement {
        x: placement.x - margin,
        y: placement.y - margin,
        w: w + 2.0 * margin,
        h: h + 2.0 * margin,
        ..*placement
    };
    let scale = zoom.min(PATH_TEXTURE_MAX / cover.w.max(cover.h));
    let side = |v: f64| (v * scale).round().max(1.0) as u32;
    let (pw, ph) = (side(cover.w), side(cover.h));
    let mut pixmap = tiny_skia::Pixmap::new(pw, ph)?;
    let place = kurbo::Affine::scale(scale) * kurbo::Affine::translate((margin, margin));
    let fill =
        |pixmap: &mut tiny_skia::Pixmap, path: &kurbo::BezPath, paint: &tiny_skia::Paint<'_>| {
            if let Some(outline) = skia_path(&(place * path.clone())) {
                let rule = tiny_skia::FillRule::Winding;
                pixmap.fill_path(
                    &outline,
                    paint,
                    rule,
                    tiny_skia::Transform::identity(),
                    None,
                );
            }
        };
    let painted = |color: Color| {
        let c = screen_color(color).to_srgba_unmultiplied();
        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(c[0], c[1], c[2], c[3]);
        paint.anti_alias = true;
        paint
    };
    for piece in &pieces {
        match piece {
            gafira_render::TextPiece::Fill { path, color } => {
                fill(&mut pixmap, path, &painted(*color));
            }
            gafira_render::TextPiece::Soft { image, at } => {
                let gafira_raster::PixelBuf::Rgba8(bytes) = image.data() else {
                    continue;
                };
                let size = tiny_skia::IntSize::from_wh(image.width(), image.height())?;
                let soft = tiny_skia::Pixmap::from_vec(bytes.clone(), size)?;
                // Sizes of a picture: the casts lose nothing seen.
                let t = (place * *at).as_coeffs().map(|v| v as f32);
                let t = tiny_skia::Transform::from_row(t[0], t[1], t[2], t[3], t[4], t[5]);
                let paint = tiny_skia::PixmapPaint {
                    quality: tiny_skia::FilterQuality::Bilinear,
                    ..tiny_skia::PixmapPaint::default()
                };
                pixmap.draw_pixmap(0, 0, soft.as_ref(), &paint, t, None);
            }
            gafira_render::TextPiece::Letters(other) => {
                let mut paint = painted(other.unwrap_or(color));
                // A gradient over the box, as the export fills it (spec 070).
                if other.is_none()
                    && let Some(shader) = text
                        .gradient
                        .as_ref()
                        .and_then(|g| gradient_shader(g, (w, h), (margin, scale)))
                {
                    paint.shader = shader;
                }
                fill(&mut pixmap, &path, &paint);
            }
        }
    }
    let image =
        egui::ColorImage::from_rgba_premultiplied([pw as usize, ph as usize], pixmap.data());
    Some((image, cover))
}

/// `path` as a tiny-skia path.
pub(crate) fn skia_path(path: &kurbo::BezPath) -> Option<tiny_skia::Path> {
    let mut builder = tiny_skia::PathBuilder::new();
    for el in path.elements() {
        match *el {
            kurbo::PathEl::MoveTo(p) => builder.move_to(p.x as f32, p.y as f32),
            kurbo::PathEl::LineTo(p) => builder.line_to(p.x as f32, p.y as f32),
            kurbo::PathEl::QuadTo(c, p) => {
                builder.quad_to(c.x as f32, c.y as f32, p.x as f32, p.y as f32);
            }
            kurbo::PathEl::CurveTo(a, b, p) => builder.cubic_to(
                a.x as f32, a.y as f32, b.x as f32, b.y as f32, p.x as f32, p.y as f32,
            ),
            kurbo::PathEl::ClosePath => builder.close(),
        }
    }
    builder.finish()
}

/// The outline of `geometry` in a `w` × `h` box from its top-left corner,
/// a rectangle's corners rounded by `radius`.
fn spec_outline(geometry: &ShapeGeometry, (w, h): (f64, f64), radius: f64) -> kurbo::BezPath {
    use kurbo::Shape as _;
    match geometry {
        ShapeGeometry::Path(path) => crate::shapes::bez(path, w, h),
        ShapeGeometry::Ellipse => {
            kurbo::Ellipse::from_rect(kurbo::Rect::new(0.0, 0.0, w, h)).to_path(0.1)
        }
        ShapeGeometry::Rect => {
            let r = radius.max(0.0).min(w.min(h) / 2.0);
            kurbo::RoundedRect::new(0.0, 0.0, w, h, r).to_path(0.1)
        }
    }
}

/// A frame, or a group of shapes and frames, drawn in a square of `side`
/// px at `scale`, fitted with its proportions kept: shapes in their colours
/// and outlines, turned as they are, frames holding the sample photo cut to
/// cover them. The preview of a tile of Marcos.
pub(crate) fn spec_preview(spec: &NodeSpec, side: f64, scale: f32) -> Option<egui::ColorImage> {
    let sample = sample_photo()?;
    let px = (side * f64::from(scale)).round().clamp(1.0, 1024.0) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(px, px)?;
    let [sw, sh] = sample.size;
    // egui's colours are premultiplied, as tiny-skia's pixmaps are.
    let data: Vec<u8> = sample.pixels.iter().flat_map(|c| c.to_array()).collect();
    let photo = tiny_skia::Pixmap::from_vec(
        data,
        tiny_skia::IntSize::from_wh(u32::try_from(sw).ok()?, u32::try_from(sh).ok()?)?,
    )?;
    let b = spec.placement;
    let k = f64::from(px) / b.w.max(b.h).max(1e-9);
    let (ox, oy) = (
        (f64::from(px) - b.w * k) / 2.0 - b.x * k,
        (f64::from(px) - b.h * k) / 2.0 - b.y * k,
    );
    let mut leaves = Vec::new();
    fn walk<'a>(spec: &'a NodeSpec, out: &mut Vec<&'a NodeSpec>) {
        if spec.children.is_empty() {
            out.push(spec);
        }
        for child in &spec.children {
            walk(child, out);
        }
    }
    walk(spec, &mut leaves);
    for leaf in leaves {
        let p = leaf.placement;
        let (cx, cy) = (p.w / 2.0, p.h / 2.0);
        let at = kurbo::Affine::translate((ox, oy))
            * kurbo::Affine::scale(k)
            * kurbo::Affine::translate((p.x + cx, p.y + cy))
            * kurbo::Affine::rotate(p.rotation_deg.to_radians())
            * kurbo::Affine::translate((-cx, -cy));
        let (geometry, radius) = match &leaf.kind {
            NodeKind::Shape(shape) => (&shape.geometry, shape.corner_radius),
            NodeKind::Frame(frame) => (&frame.clip_shape, frame.corner_radius),
            _ => continue,
        };
        let mut outline = spec_outline(geometry, (p.w, p.h), radius);
        outline.apply_affine(at);
        let Some(path) = skia_path(&outline) else {
            continue;
        };
        match &leaf.kind {
            NodeKind::Shape(shape) => {
                if let Some(fill) = shape.fill {
                    let c = screen_color(fill);
                    let mut paint = tiny_skia::Paint::default();
                    paint.set_color_rgba8(c.r(), c.g(), c.b(), c.a());
                    paint.anti_alias = true;
                    pixmap.fill_path(
                        &path,
                        &paint,
                        tiny_skia::FillRule::Winding,
                        tiny_skia::Transform::identity(),
                        None,
                    );
                }
                if let Some(stroke) = shape.stroke.filter(|s| s.width > 0.0) {
                    let c = screen_color(stroke.color);
                    let mut paint = tiny_skia::Paint::default();
                    paint.set_color_rgba8(c.r(), c.g(), c.b(), c.a());
                    paint.anti_alias = true;
                    let style = tiny_skia::Stroke {
                        width: ((stroke.width * k) as f32).max(0.75),
                        ..tiny_skia::Stroke::default()
                    };
                    pixmap.stroke_path(
                        &path,
                        &paint,
                        &style,
                        tiny_skia::Transform::identity(),
                        None,
                    );
                }
            }
            NodeKind::Frame(_) => {
                // The photo covering the frame's box, its middle kept.
                let cover = (p.w / sw as f64).max(p.h / sh as f64);
                let into =
                    at * kurbo::Affine::translate((
                        (p.w - sw as f64 * cover) / 2.0,
                        (p.h - sh as f64 * cover) / 2.0,
                    )) * kurbo::Affine::scale(cover);
                let [a, b2, c, d, e, f] = into.as_coeffs();
                let shader = tiny_skia::Pattern::new(
                    photo.as_ref(),
                    tiny_skia::SpreadMode::Pad,
                    tiny_skia::FilterQuality::Bilinear,
                    1.0,
                    tiny_skia::Transform::from_row(
                        a as f32, b2 as f32, c as f32, d as f32, e as f32, f as f32,
                    ),
                );
                let paint = tiny_skia::Paint {
                    shader,
                    anti_alias: true,
                    ..tiny_skia::Paint::default()
                };
                pixmap.fill_path(
                    &path,
                    &paint,
                    tiny_skia::FillRule::Winding,
                    tiny_skia::Transform::identity(),
                    None,
                );
            }
            _ => {}
        }
    }
    Some(egui::ColorImage::from_rgba_premultiplied(
        [px as usize, px as usize],
        pixmap.data(),
    ))
}
