//! Every core command runs the six 02 §10 laws (ARQ-81).

use gafira_color::{Color, Rgba};
use gafira_doc::{
    BlendMode, Document, GroupNode, NodeId, NodeKind, PageId, Placement, ShapeGeometry, ShapeNode,
};
use gafira_edit::{GestureId, InsertNodes, NodeSpec, Property, RemoveNodes, SetProperty};
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
    let page_index = prop_oneof![
        9 => 0..pages.len().max(1),
        1 => Just(pages.len()),
    ];
    let target = page_index.prop_flat_map(move |i| {
        let page = pages.get(i).copied().unwrap_or(PageId::new(u64::MAX));
        let present = ids.get(i).cloned().unwrap_or_default();
        // A missing node is the rare case, so most commands reach the
        // success path and law 6 gets to run.
        let node = if present.is_empty() {
            Just(NodeId::new(u64::MAX)).boxed()
        } else {
            prop_oneof![
                9 => prop::sample::select(present),
                1 => Just(NodeId::new(u64::MAX)),
            ]
            .boxed()
        };
        (Just(page), prop::collection::vec(node, 1..=2))
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

/// A page of `doc` (sometimes a missing one) with a pool of its node ids.
fn page_and_pool(doc: &Document) -> BoxedStrategy<(PageId, Vec<NodeId>)> {
    let pages: Vec<(PageId, Vec<NodeId>)> = doc
        .pages()
        .iter()
        .map(|p| (p.id, p.nodes.keys().copied().collect()))
        .collect();
    (0..=pages.len())
        .prop_map(move |i| {
            pages
                .get(i)
                .cloned()
                .unwrap_or((PageId::new(u64::MAX), Vec::new()))
        })
        .boxed()
}

/// Small trees of shapes and groups; one case in twenty has an opacity
/// `validate()` refuses.
fn node_spec() -> impl Strategy<Value = NodeSpec> {
    let leaf = (
        placement(),
        prop_oneof![19 => Just(1.0_f32), 1 => Just(1.5_f32)],
    )
        .prop_map(|(at, opacity)| NodeSpec {
            opacity,
            ..NodeSpec::new(NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect)), at)
        });
    leaf.prop_recursive(2, 6, 3, |inner| {
        (placement(), prop::collection::vec(inner, 0..=3)).prop_map(|(at, kids)| {
            NodeSpec::new(NodeKind::Group(GroupNode {}), at).with_children(kids)
        })
    })
}

/// A page id, its root count, and each node id with its child count.
type PageShape = (PageId, usize, Vec<(NodeId, usize)>);

/// A page, a parent (a root insert, a node of the page, or a missing one) and
/// an index that is mostly inside `0..=len` and sometimes past it.
fn insert_nodes(doc: &Document) -> BoxedStrategy<InsertNodes> {
    let sizes: Vec<PageShape> = doc
        .pages()
        .iter()
        .map(|p| {
            let kids = p.nodes.values().map(|n| (n.base.id, n.children.len()));
            (p.id, p.roots.len(), kids.collect())
        })
        .collect();
    let target = (0..=sizes.len(), any::<prop::sample::Index>()).prop_flat_map(move |(i, pick)| {
        let Some((page, roots, nodes)) = sizes.get(i).cloned() else {
            return Just((PageId::new(u64::MAX), None, 0)).boxed();
        };
        let mut parents: Vec<(Option<NodeId>, usize)> = vec![(None, roots)];
        parents.extend(nodes.iter().map(|(id, len)| (Some(*id), *len)));
        parents.push((Some(NodeId::new(u64::MAX)), 0));
        let (parent, len) = parents[pick.index(parents.len())];
        Just((page, parent, len)).boxed()
    });
    (
        target,
        0..=6_usize,
        prop::collection::vec(node_spec(), 0..=2),
    )
        .prop_map(|((page, parent, len), over, specs)| {
            let index = if over == 6 { len + 1 } else { over.min(len) };
            InsertNodes::new(page, parent, index, specs)
        })
        .boxed()
}

#[test]
fn insert_nodes_obeys_the_command_laws() {
    prop_command(insert_nodes);
}

/// One to three nodes of one page, sometimes a missing one.
fn remove_nodes(doc: &Document) -> BoxedStrategy<RemoveNodes> {
    page_and_pool(doc)
        .prop_flat_map(|(page, mut pool)| {
            pool.push(NodeId::new(u64::MAX));
            let nodes = prop::collection::vec(prop::sample::select(pool), 0..=3);
            (Just(page), nodes)
        })
        .prop_map(|(page, nodes)| RemoveNodes::new(page, nodes))
        .boxed()
}

#[test]
fn remove_nodes_obeys_the_command_laws() {
    prop_command(remove_nodes);
}
