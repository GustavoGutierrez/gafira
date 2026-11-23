//! Gradient fills (spec 063, D-101).

use std::sync::Arc;

use gafira_color::{Color, Rgba};
use gafira_doc::{
    Document, Dpi, ElementBase, Gradient, GradientKind, GradientProblem, GradientStop, Node,
    NodeId, NodeKind, Page, PageId, PageSize, Placement, ShapeGeometry, ShapeNode, ValidationError,
};
use gafira_geom::{Length, Unit};

const NODE: NodeId = NodeId::new(2);

fn stop(offset: f32, r: f32, g: f32, b: f32) -> GradientStop {
    GradientStop {
        offset,
        color: Color::Srgb(Rgba::new(r, g, b, 1.0)),
    }
}

fn red_blue(kind: GradientKind) -> Gradient {
    Gradient {
        kind,
        stops: vec![stop(0.0, 1.0, 0.0, 0.0), stop(1.0, 0.0, 0.0, 1.0)],
    }
}

fn document(gradient: Option<Gradient>) -> Result<Document, ValidationError> {
    let size = PageSize {
        width: Length::new(100.0, Unit::Px),
        height: Length::new(100.0, Unit::Px),
    };
    let mut page = Page::new(PageId::new(1), String::new(), size);
    let mut shape = ShapeNode::new(ShapeGeometry::Rect);
    shape.gradient = gradient;
    let node = Node::new(
        ElementBase::new(NODE, Placement::new(0.0, 0.0, 50.0, 40.0)),
        NodeKind::Shape(shape),
    );
    page.nodes.insert(NODE, Arc::new(node));
    page.roots = vec![NODE];
    Document::from_parts(3, Dpi::new(96), vec![Arc::new(page)])
}

fn near(a: Rgba, r: f32, g: f32, b: f32) -> bool {
    (a.r - r).abs() < 0.02 && (a.g - g).abs() < 0.02 && (a.b - b).abs() < 0.02
}

// S63-01: a shape with a gradient round-trips; one without writes no
// `gradient`, so its bytes are the ones it had before spec 063.
#[test]
fn s63_01_gradient_round_trip() {
    let doc = document(Some(red_blue(GradientKind::Linear { angle_deg: 90.0 }))).expect("valid");
    let json = doc.to_json().expect("json");
    assert!(
        json.contains(r#""gradient":{"kind":{"linear":{"angle_deg":90.0}}"#),
        "{json}"
    );
    assert_eq!(Document::from_json(json.as_bytes()).expect("loads"), doc);
    let radial = document(Some(red_blue(GradientKind::Radial))).expect("valid");
    let json = radial.to_json().expect("json");
    assert!(json.contains(r#""kind":"radial""#), "{json}");
    let plain = document(None).expect("valid").to_json().expect("json");
    assert!(!plain.contains("gradient"), "{plain}");
}

// S63-01: two to sixteen stops, in order, within 0-1, finite.
#[test]
fn s63_01_gradient_validation() {
    let refused = |gradient: Gradient, problem| {
        assert_eq!(
            document(Some(gradient)).expect_err("refused"),
            ValidationError::InvalidGradient {
                node: NODE,
                problem
            }
        );
    };
    let mut one = red_blue(GradientKind::Radial);
    one.stops.truncate(1);
    refused(one, GradientProblem::StopCount { count: 1 });
    let mut many = red_blue(GradientKind::Radial);
    many.stops = (0..17)
        .map(|i| stop(i as f32 / 16.0, 0.0, 0.0, 0.0))
        .collect();
    refused(many, GradientProblem::StopCount { count: 17 });
    let mut order = red_blue(GradientKind::Radial);
    order.stops[1].offset = -0.1;
    refused(order, GradientProblem::StopOffset { index: 1 });
    let mut out = red_blue(GradientKind::Radial);
    out.stops[1].offset = 1.5;
    refused(out, GradientProblem::StopOffset { index: 1 });
    refused(
        red_blue(GradientKind::Linear {
            angle_deg: f64::NAN,
        }),
        GradientProblem::NotFinite,
    );
    let mut color = red_blue(GradientKind::Radial);
    color.stops[0].color = Color::Srgb(Rgba::new(f32::NAN, 0.0, 0.0, 1.0));
    refused(color, GradientProblem::NotFinite);
}

// S63-02: the colour at a point: 0° runs left to right, 90° top to bottom,
// 180° right to left; radial runs from the centre to the corners.
#[test]
fn s63_02_color_at() {
    let (w, h) = (200.0, 100.0);
    let across = red_blue(GradientKind::Linear { angle_deg: 0.0 });
    assert!(near(across.color_at(0.0, 50.0, w, h), 1.0, 0.0, 0.0));
    assert!(near(across.color_at(200.0, 50.0, w, h), 0.0, 0.0, 1.0));
    assert!(near(across.color_at(100.0, 0.0, w, h), 0.5, 0.0, 0.5));
    let down = red_blue(GradientKind::Linear { angle_deg: 90.0 });
    assert!(near(down.color_at(100.0, 0.0, w, h), 1.0, 0.0, 0.0));
    assert!(near(down.color_at(100.0, 100.0, w, h), 0.0, 0.0, 1.0));
    let back = red_blue(GradientKind::Linear { angle_deg: 180.0 });
    assert!(near(back.color_at(0.0, 50.0, w, h), 0.0, 0.0, 1.0));
    let round = red_blue(GradientKind::Radial);
    assert!(near(round.color_at(100.0, 50.0, w, h), 1.0, 0.0, 0.0));
    assert!(near(round.color_at(0.0, 0.0, w, h), 0.0, 0.0, 1.0));
    assert!((round.position(200.0, 100.0, w, h) - 1.0).abs() < 1e-9);
    let mut three = red_blue(GradientKind::Linear { angle_deg: 0.0 });
    three.stops.insert(1, stop(0.5, 0.0, 1.0, 0.0));
    assert!(
        near(three.color_at(100.0, 50.0, w, h), 0.0, 1.0, 0.0),
        "the middle stop"
    );
    assert!(
        near(three.color_at(-50.0, 50.0, w, h), 1.0, 0.0, 0.0),
        "before the start"
    );
}
