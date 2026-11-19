//! Text inside a shape (spec 055, D-96): the model, its JSON and its
//! validation.

use std::sync::Arc;

use gafira_color::{Color, Rgba};
use gafira_doc::{
    Document, Dpi, ElementBase, Node, NodeField, NodeId, NodeKind, Page, PageId, PageSize,
    Placement, ShapeGeometry, ShapeNode, ShapeText, SpanStyle, TextAlign, ValidationError,
    VerticalAlign,
};
use gafira_geom::{Length, Unit};

const NODE: NodeId = NodeId::new(2);

fn style(size: f64) -> SpanStyle {
    SpanStyle {
        font: "Inter".to_owned(),
        size,
        color: Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)),
    }
}

fn document(text: Option<ShapeText>) -> Result<Document, ValidationError> {
    let size = PageSize {
        width: Length::new(100.0, Unit::Px),
        height: Length::new(100.0, Unit::Px),
    };
    let mut page = Page::new(PageId::new(1), String::new(), size);
    let mut shape = ShapeNode::new(ShapeGeometry::Ellipse);
    shape.text = text;
    let node = Node::new(
        ElementBase::new(NODE, Placement::new(10.0, 10.0, 50.0, 40.0)),
        NodeKind::Shape(shape),
    );
    page.nodes.insert(NODE, Arc::new(node));
    page.roots = vec![NODE];
    Document::from_parts(3, Dpi::new(96), vec![Arc::new(page)])
}

// S55-01: a shape with text round-trips through JSON, centred and in the
// middle by default.
#[test]
fn s55_01_shape_text_round_trips() {
    let plain = ShapeText::plain("Hola\nmundo", &style(24.0));
    assert_eq!(
        (plain.align, plain.valign),
        (TextAlign::Center, VerticalAlign::Middle)
    );
    let mut text = plain;
    text.valign = VerticalAlign::Bottom;
    let doc = document(Some(text)).expect("valid");
    let json = doc.to_json().expect("json");
    assert!(json.contains(r#""valign":"bottom""#), "{json}");
    assert_eq!(Document::from_json(json.as_bytes()).expect("loads"), doc);
}

// S55-01: a shape without text writes no `text` key, so its bytes are the
// ones it had before spec 055; a shape text without its alignments loads
// centred and in the middle.
#[test]
fn s55_01_shape_without_text_is_unchanged() {
    let json = document(None).expect("valid").to_json().expect("json");
    assert!(!json.contains(r#""text""#), "{json}");
    let with = document(Some(ShapeText::plain("a", &style(24.0)))).expect("valid");
    let json = with.to_json().expect("json");
    let trimmed = json
        .replace(r#","align":"center""#, "")
        .replace(r#","valign":"middle""#, "");
    assert_ne!(trimmed, json);
    assert_eq!(
        Document::from_json(trimmed.as_bytes()).expect("loads"),
        with
    );
}

// S55-01: the spans of a shape's text are checked like those of a text
// element.
#[test]
fn s55_01_shape_text_validation() {
    assert_eq!(
        document(Some(ShapeText::plain("a", &style(0.0)))).expect_err("refused"),
        ValidationError::OutOfRange {
            node: NODE,
            field: NodeField::FontSize,
            value: 0.0
        }
    );
    assert_eq!(
        document(Some(ShapeText::plain("a", &style(f64::NAN)))).expect_err("refused"),
        ValidationError::NonFinite {
            node: NODE,
            field: NodeField::FontSize
        }
    );
}
