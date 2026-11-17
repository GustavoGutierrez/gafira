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
    /// The element's opacity, 0 to 1.
    pub(crate) opacity: f32,
    /// The text of a text element; its box hits and moves like a rectangle
    /// (spec 027).
    pub(crate) text: Option<TextNode>,
    /// The asset an image element shows (spec 029).
    pub(crate) image: Option<gafira_doc::AssetRef>,
}

/// The visible shapes of `page`, back to front: roots in order, each node
/// before its children (02 §3.1). Groups draw nothing themselves.
pub(crate) fn shapes(page: &Page) -> Vec<Shape> {
    fn visit(page: &Page, id: NodeId, out: &mut Vec<Shape>) {
        let Some(node) = page.node(id) else { return };
        if !node.base.visible {
            return;
        }
        let rect = ShapeNode::new(ShapeGeometry::Rect);
        let drawn = match &node.kind {
            NodeKind::Shape(shape) => Some((*shape, None, None)),
            NodeKind::Text(text) => Some((rect, Some(text.clone()), None)),
            NodeKind::Image(image) => Some((rect, None, Some(image.asset))),
            NodeKind::Group(_) | NodeKind::Extension(_) => None,
        };
        if let Some((shape, text, image)) = drawn {
            out.push(Shape {
                page: page.id,
                node: id,
                placement: node.base.placement,
                geometry: shape.geometry,
                fill: shape.fill,
                stroke: shape.stroke,
                corner_radius: shape.corner_radius,
                opacity: node.base.opacity,
                text,
                image,
            });
        }
        for child in &node.children {
            visit(page, *child, out);
        }
    }
    let mut out = Vec::new();
    for root in &page.roots {
        visit(page, *root, &mut out);
    }
    out
}

/// One row of the Capas tab: a root element of the page, front first.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Layer {
    pub(crate) node: NodeId,
    /// The element's own name, if it has one.
    pub(crate) name: Option<String>,
    /// The catalog key of its kind (`pieces.rect`, `pieces.ellipse`, `layers.group`, ...).
    pub(crate) kind: &'static str,
    /// Its index among the roots (0 is the back).
    pub(crate) index: usize,
}

/// The root elements of `page`, front first, as the Capas tab lists them.
pub(crate) fn layers(page: &Page) -> Vec<Layer> {
    page.roots
        .iter()
        .enumerate()
        .rev()
        .filter_map(|(index, id)| {
            let node = page.node(*id)?;
            let kind = match &node.kind {
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
                _ => "layers.element",
            };
            Some(Layer {
                node: *id,
                name: node.base.name.clone(),
                kind,
                index,
            })
        })
        .collect()
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
    shapes.iter().rev().find_map(|shape| {
        let (w, h) = (shape.placement.w, shape.placement.h);
        let transform = rotation::to_screen(page_rect, zoom, &shape.placement);
        let local = rotation::local(transform, pointer)?;
        let inside = match shape.geometry {
            ShapeGeometry::Rect => (0.0..=w).contains(&local.x) && (0.0..=h).contains(&local.y),
            ShapeGeometry::Ellipse => {
                let (rx, ry) = (w / 2.0, h / 2.0);
                rx > 0.0
                    && ry > 0.0
                    && ((local.x - rx) / rx).powi(2) + ((local.y - ry) / ry).powi(2) <= 1.0
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
        .filter(|shape| rotation::screen_box(page_rect, zoom, &shape.placement).intersects(marquee))
        .map(|shape| shape.node)
        .collect()
}

/// The roots of `page` the canvas draws, back to front: what Ctrl+A selects
/// (spec 040). Groups draw nothing and are not selectable yet.
pub(crate) fn roots(page: &Page) -> Vec<NodeId> {
    let drawn: std::collections::BTreeSet<NodeId> =
        shapes(page).into_iter().map(|shape| shape.node).collect();
    page.roots
        .iter()
        .copied()
        .filter(|id| drawn.contains(id))
        .collect()
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
pub(crate) fn grip_hit(transform: Affine, placement: &Placement, pointer: Pos2) -> Option<Grip> {
    HANDLES
        .into_iter()
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
        }),
        placement,
    )
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
    let w = f64::from(page_px.x) * 0.6;
    let h = style.size * 1.4;
    let placement = Placement::new(
        (f64::from(page_px.x) - w) / 2.0,
        (f64::from(page_px.y) - h) / 2.0,
        w,
        h,
    );
    NodeSpec::new(NodeKind::Text(TextNode::plain(text, style)), placement)
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
    NodeSpec::new(NodeKind::Image(gafira_doc::ImageNode { asset }), placement)
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
        screen_color(fill).gamma_multiply(shape.opacity.clamp(0.0, 1.0))
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
    let opacity = shape.opacity.clamp(0.0, 1.0);
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
    match shape.geometry {
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
fn outline(geometry: ShapeGeometry, w: f64, h: f64, radius: f64, segments: usize) -> Vec<Point> {
    let quarter = std::f64::consts::FRAC_PI_2;
    // Each corner: its arc centre, its radii and the angle where it starts.
    let (corners, rx, ry) = match geometry {
        ShapeGeometry::Ellipse => {
            let (rx, ry) = (w / 2.0, h / 2.0);
            ([(rx, ry); 4], rx, ry)
        }
        ShapeGeometry::Rect => {
            let r = radius.max(0.0).min(w.min(h).max(0.0) / 2.0);
            if r == 0.0 {
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
    let opacity = shape.opacity.clamp(0.0, 1.0);
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
    let curve = match shape.geometry {
        ShapeGeometry::Rect => shape.corner_radius,
        ShapeGeometry::Ellipse => w.max(h) / 2.0,
    };
    // About one step per 2 screen px of the curve, from 4 to 64.
    let segments = ((curve * f64::from(zoom)).abs() / 2.0)
        .ceil()
        .clamp(4.0, 64.0) as usize;
    let points = outline(shape.geometry, w, h, shape.corner_radius, segments)
        .into_iter()
        .map(|p| rotation::screen(transform, p.x, p.y))
        .collect();
    egui::Shape::convex_polygon(points, paint(shape), stroke)
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
    let mut mesh = egui::Mesh::with_texture(texture);
    let uvs = [
        Pos2::new(0.0, 0.0),
        Pos2::new(1.0, 0.0),
        Pos2::new(1.0, 1.0),
        Pos2::new(0.0, 1.0),
    ];
    for (pos, uv) in rotation::corners(transform, placement).into_iter().zip(uvs) {
        mesh.vertices.push(egui::epaint::Vertex {
            pos,
            uv,
            color: tint,
        });
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    mesh
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
        match grip_hit(rotation::to_screen(page, 1.0, &p), &p, pointer)? {
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
        let rows = layers(&page());
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

    #[test]
    fn s27_03_a_text_element_is_listed_hit_and_placed_like_a_box() {
        let style = SpanStyle {
            font: "Inter".to_owned(),
            size: 40.0,
            color: document_color(Color32::BLACK),
        };
        let spec = new_text(Vec2::new(1000.0, 1000.0), "Hola", &style);
        assert_eq!((spec.placement.w, spec.placement.h), (600.0, 56.0));
        assert_eq!((spec.placement.x, spec.placement.y), (200.0, 472.0));
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
        assert_eq!(layers(&page)[0].kind, "layers.text");
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
            matches!(small.kind, NodeKind::Image(gafira_doc::ImageNode { asset: a }) if a == asset)
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

    // S40-02: Ctrl+A takes the drawn roots of the page: no group, nothing
    // hidden, back to front.
    #[test]
    fn s40_02_select_all_takes_the_drawn_roots() {
        let mut page = page();
        assert_eq!(roots(&page), [NodeId::new(2)]);
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
        assert_eq!(roots(&page), [NodeId::new(6), NodeId::new(2)]);
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
            grip_hit(t, &p, Pos2::new(51.0, 71.0)),
            Some(Grip::Resize(right))
        );
        // The top edge faces right: the rotate handle is 24 px past x = 55.
        assert_eq!(grip_hit(t, &p, Pos2::new(79.0, 50.0)), Some(Grip::Rotate));
        assert_eq!(
            grip_hit(t, &p, Pos2::new(57.0, 50.0)),
            Some(Grip::Resize(Handle { x: 0, y: -1 })),
            "the top-middle handle is nearer"
        );
        assert_eq!(
            grip_hit(t, &p, Pos2::new(50.0, 90.0)),
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
