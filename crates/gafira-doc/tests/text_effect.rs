//! The effects of a text (spec 089, S89-01).

use std::sync::Arc;

use gafira_color::{Color, Rgba};
use gafira_doc::{
    Document, Dpi, ElementBase, Node, NodeId, NodeKind, Page, PageId, PageSize, Placement,
    SpanStyle, TextEffect, TextEffectKind, TextEffectNumber, TextNode, ValidationError,
};
use gafira_geom::{Length, Unit};

const NODE: NodeId = NodeId::new(2);

fn document(effects: Vec<TextEffect>) -> Result<Document, ValidationError> {
    let size = PageSize {
        width: Length::new(400.0, Unit::Px),
        height: Length::new(400.0, Unit::Px),
    };
    let style = SpanStyle {
        font: "Inter".into(),
        size: 40.0,
        color: Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)),
        bold: false,
        italic: false,
        gradient: None,
    };
    let mut text = TextNode::plain("Hola", &style);
    text.effects = effects;
    let mut page = Page::new(PageId::new(1), String::new(), size);
    let base = ElementBase::new(NODE, Placement::new(10.0, 10.0, 320.0, 90.0));
    page.nodes
        .insert(NODE, Arc::new(Node::new(base, NodeKind::Text(text))));
    page.roots = vec![NODE];
    Document::from_parts(3, Dpi::new(96), vec![Arc::new(page)])
}

// S89-01: each effect starts valid and round-trips; none is not written.
#[test]
fn s89_01_round_trip() {
    for kind in TextEffectKind::ALL {
        let doc = document(vec![TextEffect::new(kind)]).expect("valid");
        let json = doc.to_json().expect("json");
        assert_eq!(Document::from_json(json.as_bytes()).expect("loads"), doc);
        // What it uses is not nothing: it is seen as soon as it is put on.
        let mut effect = TextEffect::new(kind);
        let some = kind
            .numbers()
            .iter()
            .filter(|n| **n != TextEffectNumber::Angle);
        assert!(some.clone().all(|n| *effect.number(*n) > 0.0), "{kind:?}");
    }
    let plain = document(Vec::new())
        .expect("valid")
        .to_json()
        .expect("json");
    assert!(!plain.contains("effects"));
}

// S89-01: each limit, inside and outside.
#[test]
fn s89_01_validation() {
    let ok = |change: &dyn Fn(&mut TextEffect)| {
        let mut effect = TextEffect::new(TextEffectKind::Drop);
        change(&mut effect);
        let result = document(vec![effect]);
        if let Err(error) = &result {
            assert_eq!(*error, ValidationError::InvalidText { node: NODE });
        }
        result.is_ok()
    };
    use TextEffectNumber as N;
    for number in [
        N::Thickness,
        N::Distance,
        N::Blur,
        N::Opacity,
        N::Roundness,
        N::Spread,
        N::Intensity,
    ] {
        assert!(ok(&|e| *e.number(number) = 0.0) && ok(&|e| *e.number(number) = 100.0));
        for wrong in [-0.1, 100.1, f64::NAN] {
            assert!(!ok(&|e| *e.number(number) = wrong), "{number:?} {wrong}");
        }
    }
    assert!(ok(&|e| e.angle_deg = -180.0) && !ok(&|e| e.angle_deg = 180.5));
    assert!(!ok(
        &|e| e.color = Color::Srgb(Rgba::new(f32::NAN, 0.0, 0.0, 1.0))
    ));
    assert!(!ok(
        &|e| e.second = Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 2.0))
    ));
    let many = |n| document(vec![TextEffect::new(TextEffectKind::Glow); n]).is_ok();
    assert!(many(4) && !many(5));
}
