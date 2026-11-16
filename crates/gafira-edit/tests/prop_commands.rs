//! Every core command runs the six 02 §10 laws (ARQ-81).

use std::sync::Arc;

use gafira_color::{Color, Rgba};
use gafira_doc::msg::{CommandLabel, MessageKey};
use gafira_doc::{
    AssetEntry, AssetRef, AssetStorage, BlendMode, ContentHash, DocMut, Document, Dpi, GroupNode,
    NodeId, NodeKind, PageId, PageSize, Placement, ShapeGeometry, ShapeNode,
};
use gafira_edit::{
    Applied, Command, CommandError, DirtySet, GestureId, InsertAsset, InsertNodes, InsertPage,
    NodeSpec, Property, RemoveNodes, RemovePage, Reorder, Reparent, RestoreVersion, SetProperty,
    Transaction,
};
use gafira_geom::{Length, Unit};
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

/// A node of `doc` (sometimes a missing one) with its sibling count; the
/// index runs one past it now and then, which the command must refuse.
fn reorder(doc: &Document) -> BoxedStrategy<Reorder> {
    let mut targets: Vec<(PageId, NodeId, usize)> = Vec::new();
    for page in doc.pages() {
        targets.extend(page.roots.iter().map(|id| (page.id, *id, page.roots.len())));
        for node in page.nodes.values() {
            let kids = node.children.len();
            targets.extend(node.children.iter().map(|id| (page.id, *id, kids)));
        }
    }
    targets.push((PageId::new(u64::MAX), NodeId::new(u64::MAX), 0));
    prop::sample::select(targets)
        .prop_flat_map(|(page, node, siblings)| (Just(page), Just(node), 0..=siblings))
        .prop_map(|(page, node, index)| Reorder::new(page, node, index))
        .boxed()
}

#[test]
fn reorder_obeys_the_command_laws() {
    prop_command(reorder);
}

/// One to three nodes of one page (sometimes a missing one) under the roots,
/// a node of the page (shape or group, so cycles and non-containers come up)
/// or a missing node, at a small index.
fn reparent(doc: &Document) -> BoxedStrategy<Reparent> {
    page_and_pool(doc)
        .prop_flat_map(|(page, mut pool)| {
            pool.push(NodeId::new(u64::MAX));
            let nodes = prop::collection::vec(prop::sample::select(pool.clone()), 1..=3);
            let mut parents: Vec<Option<NodeId>> = vec![None];
            parents.extend(pool.iter().map(|id| Some(*id)));
            (
                Just(page),
                nodes,
                prop::sample::select(parents),
                0..=3_usize,
            )
        })
        .prop_map(|(page, nodes, parent, index)| Reparent::new(page, nodes, parent, index))
        .boxed()
}

#[test]
fn reparent_obeys_the_command_laws() {
    prop_command(reparent);
}

/// A page size in any unit, mostly inside the pixel limits and sometimes not.
fn page_size() -> impl Strategy<Value = PageSize> {
    let unit = prop_oneof![
        Just(Unit::Px),
        Just(Unit::Mm),
        Just(Unit::Cm),
        Just(Unit::In),
        Just(Unit::Pt)
    ];
    let side = (
        prop_oneof![9 => 1.0..2_000.0_f64, 1 => 0.0..40_000.0_f64],
        unit,
    )
        .prop_map(|(v, u)| Length::new(v, u));
    (side.clone(), side).prop_map(|(width, height)| PageSize { width, height })
}

/// A page field or the DPI. DPI values reach past both ends of the valid
/// range, and the pages the generator builds make some of them overflow.
fn page_property() -> impl Strategy<Value = Property> {
    prop_oneof![
        "[a-z]{0,6}".prop_map(Property::PageName),
        page_size().prop_map(Property::PageSize),
        any::<bool>().prop_map(Property::PageLocked),
        any::<bool>().prop_map(Property::PageHidden),
        prop_oneof![
            8 => 1..=10_000_u32,
            1 => Just(0_u32),
            1 => Just(10_001_u32),
        ]
        .prop_map(|dpi| Property::Dpi(Dpi::new(dpi))),
    ]
}

/// A page of `doc` (sometimes a missing one), a page property and a gesture
/// out of two, so equal keys, and law 6, come up.
fn set_page_property(doc: &Document) -> BoxedStrategy<SetProperty> {
    let mut pages: Vec<PageId> = doc.pages().iter().map(|p| p.id).collect();
    pages.push(PageId::new(u64::MAX));
    let gesture = prop_oneof![Just(None), (0..2_u64).prop_map(|g| Some(GestureId(g)))];
    (prop::sample::select(pages), page_property(), gesture)
        .prop_map(|(page, value, gesture)| match value {
            Property::Dpi(_) => SetProperty::document(value, gesture),
            _ => SetProperty::page(page, value, gesture),
        })
        .boxed()
}

#[test]
fn set_property_on_pages_and_dpi_obeys_the_command_laws() {
    prop_command(set_page_property);
}

/// An index that is mostly inside `0..=len` and sometimes past it, a name and
/// a size that is sometimes outside the pixel limits.
fn insert_page(doc: &Document) -> BoxedStrategy<InsertPage> {
    let len = doc.pages().len();
    (0..=len + 1, "[a-z]{0,6}", page_size())
        .prop_map(|(index, name, size)| InsertPage::new(index, name, size))
        .boxed()
}

#[test]
fn insert_page_obeys_the_command_laws() {
    prop_command(insert_page);
}

/// A page of `doc`, or a missing one now and then.
fn remove_page(doc: &Document) -> BoxedStrategy<RemovePage> {
    let mut pages: Vec<PageId> = doc.pages().iter().map(|p| p.id).collect();
    pages.push(PageId::new(u64::MAX));
    prop::sample::select(pages)
        .prop_map(RemovePage::new)
        .boxed()
}

#[test]
fn remove_page_obeys_the_command_laws() {
    prop_command(remove_page);
}

/// A reference the document already has now and then, so the `Unchanged`
/// path runs too, and otherwise a new one; either storage (spec 011).
fn insert_asset(doc: &Document) -> BoxedStrategy<InsertAsset> {
    let mut known: Vec<AssetRef> = doc.assets().keys().copied().collect();
    known.push(AssetRef::new(ContentHash([7; 32])));
    let reference = prop_oneof![
        prop::sample::select(known),
        any::<[u8; 32]>().prop_map(|bytes| AssetRef::new(ContentHash(bytes))),
    ];
    let storage = prop_oneof![Just(AssetStorage::Embedded), Just(AssetStorage::Linked)];
    (reference, "[a-z]{0,8}", storage)
        .prop_map(|(reference, name, storage)| {
            InsertAsset::new(reference, AssetEntry { name, storage })
        })
        .boxed()
}

#[test]
fn insert_asset_obeys_the_command_laws() {
    prop_command(insert_asset);
}

/// A version of `doc` (spec 031): now and then the same content, so the
/// `Unchanged` path runs, otherwise a subset of its pages under a counter
/// at or above its own.
fn restore_version(doc: &Document) -> BoxedStrategy<RestoreVersion> {
    let same = RestoreVersion::new(doc);
    let pages = doc.pages().to_vec();
    let (next_id, dpi) = (doc.next_id(), doc.dpi());
    let subset = (prop::collection::vec(any::<bool>(), pages.len()), 0..3_u64).prop_map(
        move |(keep, extra)| {
            let kept = pages
                .iter()
                .zip(&keep)
                .filter(|(_, keep)| **keep)
                .map(|(page, _)| Arc::clone(page))
                .collect();
            let version = Document::from_parts(next_id + extra, dpi, kept)
                .expect("a subset of valid pages is a valid document");
            RestoreVersion::new(&version)
        },
    );
    prop_oneof![1 => Just(same), 4 => subset].boxed()
}

#[test]
fn restore_version_obeys_the_command_laws() {
    prop_command(restore_version);
}

/// One child of a generated transaction. Every step is valid on any
/// document the transaction can start from, whatever ran before it: a
/// failing child would roll back to equal but not pointer-equal pages, which
/// law 5 cannot accept (the rollback is covered by the `Transaction` unit tests).
#[derive(Clone, Debug)]
enum Step {
    Set(SetProperty),
    Insert(InsertPage),
}

fn step(first: Option<PageId>) -> BoxedStrategy<Step> {
    let size = |w: f64| PageSize {
        width: Length::new(w, Unit::Px),
        height: Length::new(50.0, Unit::Px),
    };
    let insert = ("[a-z]{0,6}", 100.0..=800.0_f64)
        .prop_map(move |(name, w)| Step::Insert(InsertPage::new(0, name, size(w))));
    let Some(page) = first else {
        return insert.boxed();
    };
    let rename = "[a-z]{0,6}"
        .prop_map(move |n| Step::Set(SetProperty::page(page, Property::PageName(n), None)));
    let hide = any::<bool>()
        .prop_map(move |h| Step::Set(SetProperty::page(page, Property::PageHidden(h), None)));
    prop_oneof![insert, rename, hide].boxed()
}

/// A `Transaction` that the harness can clone: it keeps the clonable steps
/// and rebuilds the transaction from them (a clone is always taken before the
/// first apply).
#[derive(Debug)]
struct Batch {
    steps: Vec<Step>,
    tx: Transaction,
}

impl Batch {
    fn new(steps: Vec<Step>) -> Self {
        let boxed = steps
            .iter()
            .map(|s| match s {
                Step::Set(c) => Box::new(c.clone()) as Box<dyn Command>,
                Step::Insert(c) => Box::new(c.clone()),
            })
            .collect();
        let tx = Transaction::new(MessageKey("test.batch"), boxed);
        Self { steps, tx }
    }
}

impl Clone for Batch {
    fn clone(&self) -> Self {
        Self::new(self.steps.clone())
    }
}

impl Command for Batch {
    fn label(&self) -> CommandLabel {
        self.tx.label()
    }
    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        self.tx.apply(doc)
    }
    fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError> {
        self.tx.revert(doc)
    }
    fn touches(&self) -> DirtySet {
        self.tx.touches()
    }
}

/// Up to three steps; the empty transaction is one of the cases.
fn batch(doc: &Document) -> BoxedStrategy<Batch> {
    let first = doc.pages().first().map(|p| p.id);
    prop::collection::vec(step(first), 0..=3)
        .prop_map(Batch::new)
        .boxed()
}

#[test]
fn transaction_obeys_the_command_laws() {
    prop_command(batch);
}
