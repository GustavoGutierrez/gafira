//! Lines that connect to shapes (spec 058, D-98): the model, its JSON, its
//! validation and where its ends lie.

use std::sync::Arc;

use gafira_color::{Color, Rgba};
use gafira_doc::{
    Anchor, Attachment, Document, Dpi, ElementBase, LineCap, LineEnd, LineNode, Node, NodeField,
    NodeId, NodeKind, Page, PageId, PageSize, Placement, ShapeGeometry, ShapeNode, ValidationError,
};
use gafira_geom::{Length, Point, Unit};

const SHAPE: NodeId = NodeId::new(2);
const LINE: NodeId = NodeId::new(3);

fn black() -> Color {
    Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0))
}

fn line(width: f64, attached: Option<Attachment>) -> LineNode {
    let mut end = LineEnd::new([1.0, 1.0], LineCap::Arrow);
    end.attached = attached;
    LineNode {
        start: LineEnd::new([0.0, 0.0], LineCap::Circle),
        end,
        color: black(),
        width,
    }
}

/// A 200 × 100 shape at (100, 50), turned `turn` degrees, and a line in the
/// box (10, 10)–(60, 40).
fn page(turn: f64, line: LineNode) -> Page {
    let size = PageSize {
        width: Length::new(400.0, Unit::Px),
        height: Length::new(400.0, Unit::Px),
    };
    let mut page = Page::new(PageId::new(1), String::new(), size);
    let mut at = Placement::new(100.0, 50.0, 200.0, 100.0);
    at.rotation_deg = turn;
    let shape = Node::new(
        ElementBase::new(SHAPE, at),
        NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect)),
    );
    let line = Node::new(
        ElementBase::new(LINE, Placement::new(10.0, 10.0, 50.0, 30.0)),
        NodeKind::Line(line),
    );
    page.nodes.insert(SHAPE, Arc::new(shape));
    page.nodes.insert(LINE, Arc::new(line));
    page.roots = vec![SHAPE, LINE];
    page
}

fn document(page: Page) -> Result<Document, ValidationError> {
    Document::from_parts(4, Dpi::new(96), vec![Arc::new(page)])
}

fn near(a: Point, b: Point) -> bool {
    (a - b).hypot() < 1e-9
}

// S58-01: a line with a connected end round-trips through JSON; a loose
// end writes no `attached` key.
#[test]
fn s58_01_line_round_trips() {
    let attached = Attachment {
        node: SHAPE,
        anchor: Anchor::Left,
    };
    let doc = document(page(0.0, line(3.0, Some(attached)))).expect("valid");
    let json = doc.to_json().expect("json");
    assert!(
        json.contains(r#""type":"line""#) || json.contains(r#""line""#),
        "{json}"
    );
    assert!(json.contains(r#""cap":"arrow""#), "{json}");
    assert!(json.contains(r#""anchor":"left""#), "{json}");
    assert_eq!(json.matches(r#""attached""#).count(), 1, "{json}");
    assert_eq!(Document::from_json(json.as_bytes()).expect("loads"), doc);
}

// S58-01: the width is checked like an outline; a connection to a node
// that is not there is not an error.
#[test]
fn s58_01_line_validation() {
    for (width, valid) in [(0.0, true), (1000.0, true), (-1.0, false), (1000.5, false)] {
        assert_eq!(
            document(page(0.0, line(width, None))).is_ok(),
            valid,
            "{width}"
        );
    }
    assert_eq!(
        document(page(0.0, line(f64::NAN, None))).expect_err("refused"),
        ValidationError::NonFinite {
            node: LINE,
            field: NodeField::StrokeWidth
        }
    );
    let gone = Attachment {
        node: NodeId::new(99),
        anchor: Anchor::Top,
    };
    assert!(document(page(0.0, line(1.0, Some(gone)))).is_ok());
}

// S58-02: a loose end lies at its point in the box; a connected one at the
// point of its shape, also turned; with the shape gone, back in the box.
#[test]
fn s58_02_line_ends() {
    let box_ = Placement::new(10.0, 10.0, 50.0, 30.0);
    let loose = page(0.0, line(1.0, None));
    let LineNode { .. } = line(1.0, None);
    assert_eq!(
        line(1.0, None).ends(&loose, &box_),
        [Point::new(10.0, 10.0), Point::new(60.0, 40.0)]
    );
    let points = [
        (Anchor::Top, Point::new(200.0, 50.0)),
        (Anchor::Right, Point::new(300.0, 100.0)),
        (Anchor::Bottom, Point::new(200.0, 150.0)),
        (Anchor::Left, Point::new(100.0, 100.0)),
        (Anchor::Center, Point::new(200.0, 100.0)),
    ];
    for (anchor, at) in points {
        let connected = line(
            1.0,
            Some(Attachment {
                node: SHAPE,
                anchor,
            }),
        );
        let ends = connected.ends(&page(0.0, connected.clone()), &box_);
        assert!(near(ends[1], at), "{anchor:?}: {:?}", ends[1]);
        assert!(near(ends[0], Point::new(10.0, 10.0)));
    }
    // Turned a quarter, the right point of the shape is below its centre.
    let right = line(
        1.0,
        Some(Attachment {
            node: SHAPE,
            anchor: Anchor::Right,
        }),
    );
    let ends = right.ends(&page(90.0, right.clone()), &box_);
    assert!(near(ends[1], Point::new(200.0, 200.0)), "{:?}", ends[1]);
    let gone = line(
        1.0,
        Some(Attachment {
            node: NodeId::new(99),
            anchor: Anchor::Top,
        }),
    );
    assert_eq!(gone.ends(&loose, &box_)[1], Point::new(60.0, 40.0));
    assert_eq!(
        LineNode::anchor_point(&loose, LINE, Anchor::Top),
        None,
        "not to a line"
    );
}
