//! Node kinds and the node record stored in a page arena.

use gafira_color::Color;
use serde::{Deserialize, Serialize};

use crate::{ElementBase, ImageNode, NodeId, RichText, TextAlign, TextNode};

/// A group: it only arranges its children and carries no data of its own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupNode {}

/// The outline of a shape, drawn inside the placement box.
///
/// `Rect` and `Ellipse` are written as the strings `"rect"` and
/// `"ellipse"`; a path as `{"path": {"commands": [...]}}` (D-95), so a
/// document without paths writes the bytes it wrote before spec 054.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapeGeometry {
    /// A rectangle filling the box.
    Rect,
    /// An ellipse inscribed in the box.
    Ellipse,
    /// A vector path in the unit box, scaled to the placement (spec 054).
    Path(ShapePath),
}

/// A vector path in the unit box: `(0, 0)` is the top-left corner of the
/// placement and `(1, 1)` the bottom-right one (spec 054, D-95). Its
/// interior follows the non-zero rule.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ShapePath {
    /// The drawing orders, in order; the first one is a `MoveTo`.
    pub commands: Vec<PathCommand>,
}

impl ShapePath {
    /// The most orders a path holds.
    pub const MAX_COMMANDS: usize = 4096;
    /// The least and greatest coordinate a point may take: the path may
    /// leave the unit box by one box in each direction.
    pub const MIN_COORDINATE: f64 = -1.0;
    pub const MAX_COORDINATE: f64 = 2.0;

    /// A path of these orders.
    pub fn new(commands: Vec<PathCommand>) -> Self {
        Self { commands }
    }

    /// Every coordinate of every point, x then y, in order.
    pub fn coordinates(&self) -> impl Iterator<Item = f64> + '_ {
        self.commands.iter().flat_map(PathCommand::points).flatten()
    }
}

/// One order of a [`ShapePath`], with its points in the unit box.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathCommand {
    /// Start a new subpath at the point.
    MoveTo([f64; 2]),
    /// A straight line to the point.
    LineTo([f64; 2]),
    /// A quadratic curve through a control point to the point.
    QuadTo([f64; 2], [f64; 2]),
    /// A cubic curve through two control points to the point.
    CubicTo([f64; 2], [f64; 2], [f64; 2]),
    /// Close the subpath back to where it started.
    Close,
}

impl PathCommand {
    /// The points of the order, in order.
    pub fn points(&self) -> Vec<[f64; 2]> {
        match *self {
            Self::MoveTo(p) | Self::LineTo(p) => vec![p],
            Self::QuadTo(c, p) => vec![c, p],
            Self::CubicTo(c1, c2, p) => vec![c1, c2, p],
            Self::Close => Vec::new(),
        }
    }
}

/// The outline drawn along the edge of a shape, centred on it (spec 038,
/// FD-17): half of the width falls inside the box and half outside.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShapeStroke {
    /// The colour of the outline.
    pub color: Color,
    /// The width, in document px, `0..=MAX_WIDTH`; 0 draws nothing.
    pub width: f64,
}

impl ShapeStroke {
    /// The widest outline a document holds, in px.
    pub const MAX_WIDTH: f64 = 1000.0;

    /// An outline of `color` and `width`.
    pub const fn new(color: Color, width: f64) -> Self {
        Self { color, width }
    }
}

/// Where the lines of a shape's text sit in its box, top to bottom.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerticalAlign {
    Top,
    #[default]
    Middle,
    Bottom,
}

/// The text inside a shape (spec 055, D-96): its words and styles, and how
/// it aligns across and down the shape's box.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShapeText {
    pub content: RichText,
    #[serde(default = "centred")]
    pub align: TextAlign,
    #[serde(default)]
    pub valign: VerticalAlign,
}

fn centred() -> TextAlign {
    TextAlign::Center
}

impl ShapeText {
    /// `text` in one style, centred across and down.
    #[must_use]
    pub fn plain(text: &str, style: &crate::SpanStyle) -> Self {
        Self {
            content: RichText::plain(text, style),
            align: TextAlign::Center,
            valign: VerticalAlign::Middle,
        }
    }
}

/// A geometric shape with an optional fill and outline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShapeNode {
    /// The outline.
    pub geometry: ShapeGeometry,
    /// The fill color; `None` draws nothing.
    #[serde(default)]
    pub fill: Option<Color>,
    /// The outline drawn along the edge; `None` draws none (spec 038).
    ///
    /// Written only when set, so a shape without one keeps the bytes it had
    /// before spec 038, and a shape saved then loads with none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<ShapeStroke>,
    /// The corner radius of a rectangle in px, `>= 0` (spec 038, FD-17).
    ///
    /// A radius above half the shorter side is drawn at that half (FD-36 a);
    /// the stored value is kept. An ellipse ignores it. Written only when it
    /// is not `+0.0`.
    #[serde(default, skip_serializing_if = "is_positive_zero")]
    pub corner_radius: f64,
    /// The text written inside the shape, if any (spec 055, D-96).
    ///
    /// Written only when set, so a shape without one keeps the bytes it had
    /// before spec 055.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<ShapeText>,
    /// A gradient that fills the shape instead of `fill` (spec 063, D-101).
    /// Written only when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient: Option<crate::Gradient>,
    /// A light drawn instead of the shape (spec 065, D-103). Written only
    /// when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<crate::ShapeEffect>,
}

/// Whether `value` is `+0.0`, the default radius.
fn is_positive_zero(value: &f64) -> bool {
    value.to_bits() == 0
}

impl ShapeNode {
    /// A shape without a fill, an outline, rounded corners or text.
    pub const fn new(geometry: ShapeGeometry) -> Self {
        Self {
            geometry,
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            text: None,
            gradient: None,
            effect: None,
        }
    }
}

/// A node owned by a plugin or a future feature, kept as opaque data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExtensionNode {
    /// Identifies the extension that owns the payload.
    pub type_id: String,
    /// The extension's own schema version, unrelated to the document's.
    pub schema_version: u32,
    /// The extension data.
    pub payload: serde_json::Value,
}

/// Deepest allowed nesting of an extension payload: a scalar is 0, `[]` and
/// `{}` are 1, `[[1]]` is 2.
///
/// A saved document must reopen, and `serde_json` cannot parse a whole
/// document whose payload reaches about 122 levels.
pub const MAX_EXTENSION_DEPTH: usize = 64;

/// What a node is. Closed on purpose: adding a kind makes the compiler flag
/// every `match`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NodeKind {
    /// A container of other nodes.
    Group(GroupNode),
    /// A geometric shape.
    Shape(ShapeNode),
    /// A text element (spec 027).
    Text(TextNode),
    /// An image element (spec 029).
    Image(ImageNode),
    /// A line between two ends, which may stick to shapes (spec 058).
    Line(crate::LineNode),
    /// A photo cut out by a shape (spec 066, FD-18).
    Frame(crate::FrameNode),
    /// An opaque extension node.
    Extension(ExtensionNode),
}

/// One element of a page: shared header, kind and ordered children.
///
/// Nothing is validated here; tree rules are checked by `Document::validate`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// The shared element header.
    pub base: ElementBase,
    /// What the node is.
    pub kind: NodeKind,
    /// Child ids, back to front, resolved inside the page's own arena.
    #[serde(default)]
    pub children: Vec<NodeId>,
}

impl Node {
    /// A node with no children.
    pub fn new(base: ElementBase, kind: NodeKind) -> Self {
        Self {
            base,
            kind,
            children: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use gafira_color::{Color, Rgba};

    use super::*;
    use crate::{ElementBase, NodeId, Placement};

    fn base(id: u64) -> ElementBase {
        ElementBase::new(NodeId::new(id), Placement::new(0.0, 0.0, 10.0, 10.0))
    }

    #[test]
    fn variant_sets_are_exhaustive() {
        let kind_name = |k: &NodeKind| match k {
            NodeKind::Group(GroupNode {}) => "group",
            NodeKind::Shape(_) => "shape",
            NodeKind::Text(_) => "text",
            NodeKind::Image(_) => "image",
            NodeKind::Line(_) => "line",
            NodeKind::Frame(_) => "frame",
            NodeKind::Extension(_) => "extension",
        };
        let geometry_name = |g: ShapeGeometry| match g {
            ShapeGeometry::Rect => "rect",
            ShapeGeometry::Ellipse => "ellipse",
            ShapeGeometry::Path(_) => "path",
        };
        assert_eq!(kind_name(&NodeKind::Group(GroupNode {})), "group");
        assert_eq!(
            kind_name(&NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect))),
            "shape"
        );
        let ext = ExtensionNode {
            type_id: "x".to_string(),
            schema_version: 1,
            payload: serde_json::Value::Null,
        };
        assert_eq!(kind_name(&NodeKind::Extension(ext)), "extension");
        assert_eq!(geometry_name(ShapeGeometry::Rect), "rect");
        assert_eq!(geometry_name(ShapeGeometry::Ellipse), "ellipse");
    }

    #[test]
    fn shape_without_fill() {
        let s = ShapeNode::new(ShapeGeometry::Rect);
        assert_eq!(s.geometry, ShapeGeometry::Rect);
        assert_eq!(s.fill, None);
        let filled = ShapeNode {
            fill: Some(Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0))),
            ..s
        };
        assert!(filled.fill.is_some());
    }

    #[test]
    fn new_node_has_no_children() {
        let n = Node::new(base(3), NodeKind::Group(GroupNode {}));
        assert_eq!(n.base.id, NodeId::new(3));
        assert!(n.children.is_empty());
    }

    #[test]
    fn extension_keeps_its_own_version_and_payload() {
        let payload = serde_json::json!({"k": [1, 2]});
        let e = ExtensionNode {
            type_id: "vendor.thing".to_string(),
            schema_version: 7,
            payload: payload.clone(),
        };
        assert_eq!(e.schema_version, 7);
        assert_eq!(e.payload, payload);
    }

    #[test]
    fn nan_placement_node_is_not_equal_to_itself() {
        let mut b = base(1);
        b.placement = Placement::new(f64::NAN, 0.0, 1.0, 1.0);
        let n = Node::new(b, NodeKind::Group(GroupNode {}));
        let copy = n.clone();
        assert_ne!(n, copy);
        assert_eq!(
            n.base.placement.x.to_bits(),
            copy.base.placement.x.to_bits()
        );
    }

    fn node_json(node: &Node) -> String {
        serde_json::to_string(node).unwrap()
    }

    #[test]
    fn kinds_have_their_wire_shapes() {
        let kind = |n: &Node| serde_json::to_value(n).unwrap()["kind"].to_string();
        let group = Node::new(base(1), NodeKind::Group(GroupNode {}));
        assert_eq!(kind(&group), r#"{"type":"group"}"#);
        assert!(node_json(&group).ends_with(r#""kind":{"type":"group"},"children":[]}"#));
        let bare = Node::new(
            base(2),
            NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect)),
        );
        assert_eq!(
            kind(&bare),
            r#"{"fill":null,"geometry":"rect","type":"shape"}"#
        );
        assert!(
            node_json(&bare).contains(r#""kind":{"type":"shape","geometry":"rect","fill":null}"#)
        );
        let mut shape = ShapeNode::new(ShapeGeometry::Ellipse);
        shape.fill = Some(Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0)));
        let s = node_json(&Node::new(base(3), NodeKind::Shape(shape)));
        assert!(s.contains(
            r#""kind":{"type":"shape","geometry":"ellipse","fill":{"space":"srgb","r":1.0,"g":0.0,"b":0.0,"a":1.0}}"#
        ));
        let ext = ExtensionNode {
            type_id: "com.example.sticker".to_string(),
            schema_version: 2,
            payload: serde_json::json!({"b": 1, "a": null}),
        };
        let e = Node::new(base(4), NodeKind::Extension(ext));
        assert!(node_json(&e).contains(
            r#""kind":{"type":"extension","type_id":"com.example.sticker","schema_version":2,"payload":{"a":null,"b":1}},"children":[]}"#
        ));
        for n in [group, bare, e] {
            assert_eq!(serde_json::from_str::<Node>(&node_json(&n)).unwrap(), n);
        }
    }

    // S38-01: an outline and a corner radius are written only when set, so a
    // shape without them keeps its earlier bytes, and they round-trip.
    #[test]
    fn s38_01_outline_and_radius_are_additive_on_the_wire() {
        let kind = |n: &Node| serde_json::to_value(n).unwrap()["kind"].to_string();
        let mut shape = ShapeNode::new(ShapeGeometry::Rect);
        shape.stroke = Some(ShapeStroke::new(
            Color::Srgb(Rgba::new(0.0, 0.0, 1.0, 1.0)),
            2.5,
        ));
        shape.corner_radius = 12.0;
        let n = Node::new(base(1), NodeKind::Shape(shape.clone()));
        assert_eq!(
            kind(&n),
            r#"{"corner_radius":12.0,"fill":null,"geometry":"rect","stroke":{"color":{"a":1.0,"b":1.0,"g":0.0,"r":0.0,"space":"srgb"},"width":2.5},"type":"shape"}"#
        );
        assert_eq!(serde_json::from_str::<Node>(&node_json(&n)).unwrap(), n);
        // A negative zero radius is a set value and survives.
        shape.corner_radius = -0.0;
        shape.stroke = None;
        let n = Node::new(base(1), NodeKind::Shape(shape));
        let back: Node = serde_json::from_str(&node_json(&n)).unwrap();
        let NodeKind::Shape(back) = back.kind else {
            panic!("not a shape")
        };
        assert!(back.corner_radius.is_sign_negative());
        // Only a positive zero is left out.
        let plain = Node::new(
            base(1),
            NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect)),
        );
        assert!(!node_json(&plain).contains("corner_radius"));
        assert!(!node_json(&plain).contains("stroke"));
    }

    // S38-01: a shape saved before spec 038 loads with no outline and radius 0.
    #[test]
    fn s38_01_an_old_shape_loads_with_the_defaults() {
        let n = parse_kind(r#"{"type":"shape","geometry":"rect","fill":null}"#).unwrap();
        let NodeKind::Shape(shape) = n.kind else {
            panic!("not a shape")
        };
        assert_eq!(shape.stroke, None);
        assert_eq!(shape.corner_radius.to_bits(), 0f64.to_bits());
        assert_eq!(shape, ShapeNode::new(ShapeGeometry::Rect));
        for bad in [
            r#"{"type":"shape","geometry":"rect","stroke":{"width":1}}"#,
            r#"{"type":"shape","geometry":"rect","stroke":{"color":{"space":"srgb","r":0,"g":0,"b":0,"a":1}}}"#,
            r#"{"type":"shape","geometry":"rect","corner_radius":"big"}"#,
        ] {
            assert!(parse_kind(bad).is_err(), "{bad}");
        }
        let null = parse_kind(r#"{"type":"shape","geometry":"rect","stroke":null}"#).unwrap();
        assert!(matches!(
            null.kind,
            NodeKind::Shape(ShapeNode { stroke: None, .. })
        ));
    }

    fn parse_kind(kind: &str) -> Result<Node, serde_json::Error> {
        let b = r#"{"id":1,"placement":{"x":0,"y":0,"w":1,"h":1}}"#;
        serde_json::from_str(&format!(r#"{{"base":{b},"kind":{kind},"extra":1}}"#))
    }

    #[test]
    fn kind_defaults_required_fields_and_vocabulary() {
        let n = parse_kind(r#"{"type":"group","extra":1}"#).unwrap();
        assert_eq!(n, Node::new(base_for(1), NodeKind::Group(GroupNode {})));
        let s = parse_kind(r#"{"type":"shape","geometry":"rect"}"#).unwrap();
        assert!(matches!(
            s.kind,
            NodeKind::Shape(ShapeNode { fill: None, .. })
        ));
        for bad in [
            r#"{"type":"text"}"#,
            r#"{"type":"Group"}"#,
            r#"{}"#,
            r#"{"type":"shape"}"#,
            r#"{"type":"shape","geometry":"star"}"#,
            r#"{"type":"shape","geometry":"rect","geometry":"ellipse"}"#,
            r#"{"type":"shape","geometry":"rect","fill":{"space":"rec2020"}}"#,
            r#"{"type":"extension","schema_version":1,"payload":null}"#,
            r#"{"type":"extension","type_id":"x","payload":null}"#,
            r#"{"type":"extension","type_id":"x","schema_version":1}"#,
        ] {
            assert!(parse_kind(bad).is_err(), "{bad}");
        }
        assert!(serde_json::from_str::<Node>(r#"{"kind":{"type":"group"}}"#).is_err());
    }

    fn base_for(id: u64) -> ElementBase {
        ElementBase::new(NodeId::new(id), Placement::new(0.0, 0.0, 1.0, 1.0))
    }

    #[test]
    fn extension_payload_null_and_f64_extremes_survive() {
        let ext = ExtensionNode {
            type_id: "x".to_string(),
            schema_version: 3,
            payload: serde_json::Value::Null,
        };
        let mut b = base(1);
        b.placement = Placement::new(f64::MAX, -0.0, 5e-324, f64::MIN_POSITIVE);
        let n = Node::new(b, NodeKind::Extension(ext));
        let back: Node = serde_json::from_str(&node_json(&n)).unwrap();
        assert_eq!(back, n);
        assert!(back.base.placement.y.is_sign_negative());
    }

    #[test]
    fn floats_inside_the_tagged_kind_survive() {
        let mut shape = ShapeNode::new(ShapeGeometry::Rect);
        shape.fill = Some(Color::Srgb(Rgba::new(f32::MAX, 1e-45, -0.0, 1.0)));
        let n = Node::new(base(1), NodeKind::Shape(shape));
        let back: Node = serde_json::from_str(&node_json(&n)).unwrap();
        let NodeKind::Shape(ShapeNode {
            fill: Some(Color::Srgb(c)),
            ..
        }) = &back.kind
        else {
            panic!("not an sRGB shape")
        };
        assert_eq!(c.r.to_bits(), f32::MAX.to_bits());
        assert_eq!(c.g.to_bits(), 1e-45f32.to_bits());
        assert!(c.b.is_sign_negative());
        assert_eq!(back, n);
    }

    #[test]
    fn f64_payload_inside_the_tagged_kind_survives() {
        let ext = ExtensionNode {
            type_id: "x".to_string(),
            schema_version: 1,
            payload: serde_json::json!({"max": f64::MAX, "tiny": 5e-324, "neg": -0.0}),
        };
        let n = Node::new(base(1), NodeKind::Extension(ext));
        let json = node_json(&n);
        assert!(
            json.contains(r#""payload":{"max":1.7976931348623157e+308,"neg":-0.0,"tiny":5e-324}"#)
        );
        let back: Node = serde_json::from_str(&json).unwrap();
        assert_eq!(back, n);
        let NodeKind::Extension(e) = &back.kind else {
            panic!("not an extension")
        };
        assert_eq!(
            e.payload["max"].as_f64().unwrap().to_bits(),
            f64::MAX.to_bits()
        );
        assert_eq!(
            e.payload["tiny"].as_f64().unwrap().to_bits(),
            5e-324f64.to_bits()
        );
        assert!(e.payload["neg"].as_f64().unwrap().is_sign_negative());
    }
}
