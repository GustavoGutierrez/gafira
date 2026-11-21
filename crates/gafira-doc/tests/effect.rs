//! Light effects of shapes (spec 065, D-103).

use std::sync::Arc;

use gafira_color::{Color, Rgba};
use gafira_doc::{
    Document, Dpi, EffectProblem, ElementBase, Light, Node, NodeId, NodeKind, Page, PageId,
    PageSize, Placement, ShapeEffect, ShapeGeometry, ShapeNode, ValidationError,
};
use gafira_geom::{Length, Unit};

const NODE: NodeId = NodeId::new(2);

fn document(effect: Option<ShapeEffect>) -> Result<Document, ValidationError> {
    let size = PageSize {
        width: Length::new(100.0, Unit::Px),
        height: Length::new(100.0, Unit::Px),
    };
    let mut page = Page::new(PageId::new(1), String::new(), size);
    let mut shape = ShapeNode::new(ShapeGeometry::Ellipse);
    shape.effect = effect;
    let node = Node::new(
        ElementBase::new(NODE, Placement::new(0.0, 0.0, 50.0, 50.0)),
        NodeKind::Shape(shape),
    );
    page.nodes.insert(NODE, Arc::new(node));
    page.roots = vec![NODE];
    Document::from_parts(3, Dpi::new(96), vec![Arc::new(page)])
}

// S65-01: each effect round-trips; a shape without one writes no `effect`,
// so its bytes are the ones it had before spec 065.
#[test]
fn s65_01_effect_round_trip() {
    let mut halo = Light::SPARKLE;
    halo.halo = Some(Color::Srgb(Rgba::new(1.0, 0.3, 0.2, 1.0)));
    for effect in [
        ShapeEffect::Sparkle(Light::SPARKLE),
        ShapeEffect::LensFlare(halo),
        ShapeEffect::Streak(Light::STREAK),
        ShapeEffect::SunRays(Light::SUN_RAYS),
        ShapeEffect::Bokeh(Light::BOKEH),
    ] {
        let doc = document(Some(effect.clone())).expect("valid");
        let json = doc.to_json().expect("json");
        assert!(json.contains(r#""effect":{"#), "{json}");
        assert_eq!(Document::from_json(json.as_bytes()).expect("loads"), doc);
    }
    let sparkle = document(Some(ShapeEffect::Sparkle(Light::SPARKLE)))
        .expect("valid")
        .to_json()
        .expect("json");
    assert!(
        sparkle.contains(r#""effect":{"sparkle":{"rays":"#),
        "{sparkle}"
    );
    assert!(!sparkle.contains("halo"), "no halo, no key: {sparkle}");
    let plain = document(None).expect("valid").to_json().expect("json");
    assert!(!plain.contains("effect"), "{plain}");
}

// S65-01: four to sixteen rays; width and intensity within 0-1; finite
// numbers.
#[test]
fn s65_01_effect_validation() {
    let refused = |effect: ShapeEffect, problem| {
        assert_eq!(
            document(Some(effect)).expect_err("refused"),
            ValidationError::InvalidEffect {
                node: NODE,
                problem
            }
        );
    };
    let light = |change: fn(&mut Light)| {
        let mut light = Light::SPARKLE;
        change(&mut light);
        ShapeEffect::Sparkle(light)
    };
    refused(light(|l| l.rays = 3), EffectProblem::Rays { count: 3 });
    refused(light(|l| l.rays = 17), EffectProblem::Rays { count: 17 });
    refused(light(|l| l.ray_width = 1.5), EffectProblem::OutOfRange);
    refused(light(|l| l.intensity = -0.1), EffectProblem::OutOfRange);
    refused(light(|l| l.angle_deg = f64::NAN), EffectProblem::NotFinite);
    refused(
        light(|l| l.halo = Some(Color::Srgb(Rgba::new(f32::NAN, 0.0, 0.0, 1.0)))),
        EffectProblem::NotFinite,
    );
    assert!(document(Some(ShapeEffect::LensFlare(Light::LENS_FLARE))).is_ok());
    let mut wide = Light::BOKEH;
    wide.intensity = 2.0;
    refused(ShapeEffect::Bokeh(wide), EffectProblem::OutOfRange);
}
