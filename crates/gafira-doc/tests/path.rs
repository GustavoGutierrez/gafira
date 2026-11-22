//! Shape paths in the document (spec 054, D-95): the model, its JSON and
//! its validation.

use std::sync::Arc;

use gafira_doc::{
    Document, Dpi, ElementBase, Node, NodeField, NodeId, NodeKind, Page, PageId, PageSize,
    PathCommand, PathProblem, Placement, ShapeGeometry, ShapeNode, ShapePath, ValidationError,
};
use gafira_geom::{Length, Unit};

const PAGE: PageId = PageId::new(1);
const NODE: NodeId = NodeId::new(2);

fn triangle() -> ShapePath {
    ShapePath::new(vec![
        PathCommand::MoveTo([0.5, 0.0]),
        PathCommand::LineTo([1.0, 1.0]),
        PathCommand::QuadTo([0.5, 1.2], [0.0, 1.0]),
        PathCommand::CubicTo([0.0, 0.5], [0.2, 0.2], [0.5, 0.0]),
        PathCommand::Close,
    ])
}

fn document(geometry: ShapeGeometry) -> Result<Document, ValidationError> {
    let size = PageSize {
        width: Length::new(100.0, Unit::Px),
        height: Length::new(100.0, Unit::Px),
    };
    let mut page = Page::new(PAGE, String::new(), size);
    let node = Node::new(
        ElementBase::new(NODE, Placement::new(10.0, 10.0, 50.0, 40.0)),
        NodeKind::Shape(ShapeNode::new(geometry)),
    );
    page.nodes.insert(NODE, Arc::new(node));
    page.roots = vec![NODE];
    Document::from_parts(3, Dpi::new(96), vec![Arc::new(page)])
}

// S54-01: a document with a path round-trips through JSON, with the path
// written as `{"path": {"commands": [...]}}`.
#[test]
fn s54_01_path_round_trips() {
    let doc = document(ShapeGeometry::Path(triangle())).expect("valid");
    let json = doc.to_json().expect("json");
    assert!(
        json.contains(r#""geometry":{"path":{"commands":[{"move_to":[0.5,0.0]}"#),
        "{json}"
    );
    assert!(json.contains(r#""close""#));
    let back = Document::from_json(json.as_bytes()).expect("loads");
    assert_eq!(back, doc);
}

// S54-01: a document without paths writes the same geometry strings as
// before spec 054.
#[test]
fn s54_01_old_json_is_unchanged() {
    for (geometry, text) in [
        (ShapeGeometry::Rect, r#""geometry":"rect""#),
        (ShapeGeometry::Ellipse, r#""geometry":"ellipse""#),
    ] {
        let json = document(geometry).expect("valid").to_json().expect("json");
        assert!(json.contains(text), "{json}");
    }
}

// S54-02: an empty path, one that does not start with move_to, one with
// too many orders, and a coordinate that is not finite or leaves -1..=2
// are refused, naming the node and what is wrong.
#[test]
fn s54_02_path_validation() {
    let invalid = |commands: Vec<PathCommand>| {
        document(ShapeGeometry::Path(ShapePath::new(commands))).expect_err("refused")
    };
    assert_eq!(
        invalid(Vec::new()),
        ValidationError::InvalidPath {
            node: NODE,
            problem: PathProblem::Empty
        }
    );
    assert_eq!(
        invalid(vec![PathCommand::LineTo([0.0, 0.0]), PathCommand::Close]),
        ValidationError::InvalidPath {
            node: NODE,
            problem: PathProblem::NoStart
        }
    );
    let mut long = vec![PathCommand::MoveTo([0.0, 0.0])];
    long.extend(std::iter::repeat_n(
        PathCommand::LineTo([1.0, 1.0]),
        ShapePath::MAX_COMMANDS,
    ));
    assert_eq!(
        invalid(long),
        ValidationError::InvalidPath {
            node: NODE,
            problem: PathProblem::TooLong {
                len: ShapePath::MAX_COMMANDS + 1
            }
        }
    );
    assert_eq!(
        invalid(vec![PathCommand::MoveTo([f64::NAN, 0.0])]),
        ValidationError::NonFinite {
            node: NODE,
            field: NodeField::PathCoordinate
        }
    );
    assert_eq!(
        invalid(vec![
            PathCommand::MoveTo([0.0, 0.0]),
            PathCommand::LineTo([2.5, 0.0])
        ]),
        ValidationError::OutOfRange {
            node: NODE,
            field: NodeField::PathCoordinate,
            value: 2.5
        }
    );
    let mut edge = vec![PathCommand::MoveTo([-1.0, 2.0])];
    edge.extend(std::iter::repeat_n(
        PathCommand::Close,
        ShapePath::MAX_COMMANDS - 1,
    ));
    assert!(
        document(ShapeGeometry::Path(ShapePath::new(edge))).is_ok(),
        "the limits are valid"
    );
}
