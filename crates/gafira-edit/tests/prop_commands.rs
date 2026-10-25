//! Every core command runs the six 02 §10 laws (ARQ-81).

use gafira_color::{Color, Rgba};
use gafira_doc::{BlendMode, Document, NodeId, PageId, Placement, ShapeGeometry};
use gafira_edit::{GestureId, Property, SetProperty};
use gafira_testkit::prop_command;
use proptest::prelude::*;

fn placement() -> impl Strategy<Value = Placement> {
    (
        -1.0e6..1.0e6_f64,
        -1.0e6..1.0e6_f64,
        0.0..1.0e6_f64,
        0.0..1.0e6_f64,
        -720.0..720.0_f64,
        -89.9..89.9_f64,
    )
        .prop_map(|(x, y, w, h, rotation_deg, skew)| Placement {
            rotation_deg,
            skew,
            ..Placement::new(x, y, w, h)
        })
}

fn fill() -> impl Strategy<Value = Option<Color>> {
    prop_oneof![
        Just(None),
        ([-4.0..4.0_f32, -4.0..4.0, -4.0..4.0], 0.0..=1.0_f32)
            .prop_map(|(c, a)| Some(Color::Srgb(Rgba::new(c[0], c[1], c[2], a)))),
    ]
}

/// Mostly valid values, plus a few the command must refuse.
fn property() -> impl Strategy<Value = Property> {
    prop_oneof![
        prop_oneof![Just(None), "[a-z]{0,6}".prop_map(Some)].prop_map(Property::Name),
        placement().prop_map(Property::Placement),
        prop_oneof![Just(0.0_f32), Just(1.0), Just(0.5), 0.0..=1.0_f32].prop_map(Property::Opacity),
        prop_oneof![Just(f32::NAN), Just(1.5_f32), Just(-0.5)].prop_map(Property::Opacity),
        prop_oneof![Just(BlendMode::Normal), Just(BlendMode::Multiply)].prop_map(Property::Blend),
        any::<bool>().prop_map(Property::Visible),
        any::<bool>().prop_map(Property::Locked),
        fill().prop_map(Property::Fill),
        prop_oneof![Just(ShapeGeometry::Rect), Just(ShapeGeometry::Ellipse)]
            .prop_map(Property::Geometry),
    ]
}

/// Targets one page of `doc` (sometimes a missing one) and a few of its
/// nodes (sometimes a missing one). Small choices make merges likely.
fn set_property(doc: &Document) -> BoxedStrategy<SetProperty> {
    let pages: Vec<PageId> = doc.pages().iter().map(|p| p.id).collect();
    let ids: Vec<Vec<NodeId>> = doc
        .pages()
        .iter()
        .map(|p| p.nodes.keys().copied().collect())
        .collect();
    let target = (0..=pages.len()).prop_flat_map(move |i| {
        let page = pages.get(i).copied().unwrap_or(PageId::new(u64::MAX));
        let mut pool = ids.get(i).cloned().unwrap_or_default();
        pool.push(NodeId::new(u64::MAX));
        let nodes = prop::collection::vec(prop::sample::select(pool), 1..=2);
        (Just(page), nodes)
    });
    let gesture = prop_oneof![Just(None), (0..2_u64).prop_map(|g| Some(GestureId(g)))];
    (target, property(), gesture)
        .prop_map(|((page, nodes), value, gesture)| SetProperty::new(page, nodes, value, gesture))
        .boxed()
}

#[test]
fn set_property_obeys_the_command_laws() {
    prop_command(set_property);
}

/// Always the same page, node and gesture, so every pair of commands can
/// merge and law 6 runs on almost every case.
fn drag(doc: &Document) -> BoxedStrategy<SetProperty> {
    let target = doc
        .pages()
        .iter()
        .find_map(|p| p.nodes.keys().next().map(|n| (p.id, *n)));
    let Some((page, node)) = target else {
        return Just(SetProperty::new(
            PageId::new(1),
            vec![],
            Property::Visible(true),
            None,
        ))
        .boxed();
    };
    prop_oneof![
        placement().prop_map(Property::Placement),
        (0.0..=1.0_f32).prop_map(Property::Opacity)
    ]
    .prop_map(move |value| SetProperty::new(page, vec![node], value, Some(GestureId(0))))
    .boxed()
}

#[test]
fn merged_set_property_drags_obey_the_command_laws() {
    prop_command(drag);
}
