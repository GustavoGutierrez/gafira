//! Node kinds and the node record stored in a page arena.

use gafira_color::Color;

use crate::{ElementBase, NodeId};

/// A group: it only arranges its children and carries no data of its own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GroupNode {}

/// The outline of a shape, drawn inside the placement box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeGeometry {
    /// A rectangle filling the box.
    Rect,
    /// An ellipse inscribed in the box.
    Ellipse,
}

/// A filled geometric shape (no stroke yet).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeNode {
    /// The outline.
    pub geometry: ShapeGeometry,
    /// The fill color; `None` draws nothing.
    pub fill: Option<Color>,
}

impl ShapeNode {
    /// A shape without a fill.
    pub const fn new(geometry: ShapeGeometry) -> Self {
        Self {
            geometry,
            fill: None,
        }
    }
}

/// A node owned by a plugin or a future feature, kept as opaque data.
#[derive(Clone, Debug, PartialEq)]
pub struct ExtensionNode {
    /// Identifies the extension that owns the payload.
    pub type_id: String,
    /// The extension's own schema version, unrelated to the document's.
    pub schema_version: u32,
    /// The extension data.
    pub payload: serde_json::Value,
}

/// What a node is. Closed on purpose: adding a kind makes the compiler flag
/// every `match`.
#[derive(Clone, Debug, PartialEq)]
pub enum NodeKind {
    /// A container of other nodes.
    Group(GroupNode),
    /// A geometric shape.
    Shape(ShapeNode),
    /// An opaque extension node.
    Extension(ExtensionNode),
}

/// One element of a page: shared header, kind and ordered children.
///
/// Nothing is validated here; tree rules are checked by `Document::validate`.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// The shared element header.
    pub base: ElementBase,
    /// What the node is.
    pub kind: NodeKind,
    /// Child ids, back to front, resolved inside the page's own arena.
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
            NodeKind::Extension(_) => "extension",
        };
        let geometry_name = |g: ShapeGeometry| match g {
            ShapeGeometry::Rect => "rect",
            ShapeGeometry::Ellipse => "ellipse",
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
}
