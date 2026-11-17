//! Test support for the Gafira domain crates: a bounded document generator,
//! corruption generators and their deterministic guards.
//!
//! Used only as a dev-dependency, and only from integration tests (`tests/`),
//! where every crate has one copy of each type.

#![forbid(unsafe_code)]

pub mod fonts;
pub mod golden;
mod harness;
pub mod progress;
pub mod scenes;

/// The port doubles of Gafira (ADR 0013, ARQ-112).
///
/// Behind the `ports` feature, off by default: the default tree of this
/// crate carries neither `gafira-ports` nor `gafira-raster` (ADR 0007). The
/// list of the doubles is fixed here, so a later delivery fills a file of
/// this module instead of editing this one.
#[cfg(feature = "ports")]
pub mod ports {
    pub mod clock;
    pub mod codec;
    pub mod container;
    pub mod decoder;
    pub mod encoder;
    pub mod file_system;
    pub mod muxer;
    pub mod page_renderer;
    pub mod render_backend;
}

pub use harness::{LawViolation, prop_command, try_prop_command};

use std::sync::Arc;

use gafira_color::{Color, DisplayP3Rgba, LinearRgba, Rgba};
use gafira_doc::{
    BlendMode, DocMut, Document, Dpi, EditToken, ElementBase, ExtensionNode, Flip, GridSettings,
    GridStyle, GroupNode, Guide, GuideAxis, GuideSet, LoadError, Margins, Node, NodeField, NodeId,
    NodeKind, Page, PageId, PageSize, Placement, ShapeGeometry, ShapeNode, ValidationError,
};
use gafira_geom::{Length, Unit};
use proptest::collection::vec;
use proptest::prelude::*;
use serde_json::{Map, Number, Value};

const MAX_PAGES: usize = 3;
const MAX_NODES: usize = 12;
const MAX_TREE_DEPTH: usize = 4;
const MAX_GAP: u64 = 3;
const SLOTS: usize = MAX_PAGES * (MAX_NODES + 1);

fn finite() -> impl Strategy<Value = f64> {
    use proptest::num::f64::{NEGATIVE, NORMAL, POSITIVE, SUBNORMAL, ZERO};
    POSITIVE | NEGATIVE | NORMAL | SUBNORMAL | ZERO
}

fn non_negative() -> impl Strategy<Value = f64> {
    prop_oneof![finite().prop_map(f64::abs), Just(0.0), Just(-0.0)]
}

fn skew() -> impl Strategy<Value = f64> {
    prop_oneof![-89.999..=89.999, Just(89.999), Just(-89.999)]
}

fn opacity() -> impl Strategy<Value = f32> {
    use proptest::num::f32::{POSITIVE, SUBNORMAL};
    prop_oneof![
        Just(0.0_f32),
        Just(-0.0_f32),
        Just(1.0_f32),
        POSITIVE | SUBNORMAL,
        0.0_f32..=1.0
    ]
}

fn color() -> impl Strategy<Value = Color> {
    (
        0..3,
        [-4.0_f32..=4.0, -4.0..=4.0, -4.0..=4.0],
        0.0_f32..=1.0,
    )
        .prop_map(|(space, c, a)| match space {
            0 => Color::Srgb(Rgba::new(c[0], c[1], c[2], a)),
            1 => Color::LinearSrgb(LinearRgba::new(c[0], c[1], c[2], a)),
            _ => Color::DisplayP3(DisplayP3Rgba::new(c[0], c[1], c[2], a)),
        })
}

fn placement() -> impl Strategy<Value = Placement> {
    (
        (finite(), finite(), non_negative(), non_negative(), finite()),
        skew(),
        any::<(bool, bool)>(),
    )
        .prop_map(|(v, skew, (fx, fy))| {
            let mut p = Placement::new(v.0, v.1, v.2, v.3);
            p.rotation_deg = v.4;
            p.skew = skew;
            p.flip = Flip { x: fx, y: fy };
            p
        })
}

fn any_unit() -> impl Strategy<Value = Unit> {
    proptest::sample::select(Unit::ALL.to_vec())
}

/// A length of `lo..=hi` in any unit, written as the number it holds.
fn length_in(lo: f64, hi: f64) -> impl Strategy<Value = Length> {
    (lo..=hi, any_unit()).prop_map(|(v, unit)| Length::new(v, unit))
}

/// A page bleed of 0 to 10 mm, the ends included.
pub fn bleed() -> impl Strategy<Value = Length> {
    prop_oneof![
        1 => Just(Length::new(0.0, Unit::Mm)),
        1 => Just(Length::new(10.0, Unit::Mm)),
        4 => (0.0..=10.0_f64).prop_map(|v| Length::new(v, Unit::Mm)),
    ]
}

/// Margins of 0 to 500 px per side; `linked` is free and independent of the
/// sides, which are stored as given.
pub fn margins() -> impl Strategy<Value = Margins> {
    let side = || prop_oneof![Just(Length::new(0.0, Unit::Px)), length_in(0.0, 500.0)];
    (side(), side(), side(), side(), any::<bool>()).prop_map(
        |(top, right, bottom, left, linked)| Margins {
            top,
            right,
            bottom,
            left,
            linked,
        },
    )
}

/// One guide at any finite position, including negative and far outside a page.
pub fn guide() -> impl Strategy<Value = Guide> {
    (
        any::<bool>(),
        prop_oneof![
            finite().prop_map(|v| Length::new(v, Unit::Px)),
            length_in(-1e6, 1e6)
        ],
        any::<bool>(),
    )
        .prop_map(|(vertical, position, locked)| Guide {
            axis: if vertical {
                GuideAxis::Vertical
            } else {
                GuideAxis::Horizontal
            },
            position,
            locked,
        })
}

/// Up to 6 guides, duplicates and any order included.
pub fn guide_set() -> impl Strategy<Value = GuideSet> {
    prop_oneof![
        1 => Just(GuideSet::default()),
        4 => vec(guide(), 1..=6).prop_map(GuideSet),
        1 => guide().prop_map(|g| GuideSet(vec![g, g])),
    ]
}

/// Grid settings inside the ranges the spec allows: step 1 to 500, 1 to 10
/// subdivisions.
pub fn grid_settings() -> impl Strategy<Value = GridSettings> {
    (
        any::<bool>(),
        length_in(1.0, 500.0),
        1..=10_u8,
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(|(visible, step, subdivisions, dots, snap)| GridSettings {
            visible,
            step,
            subdivisions,
            style: if dots {
                GridStyle::Dots
            } else {
                GridStyle::Lines
            },
            snap,
        })
}

/// Up to 8 characters, including non-ASCII, quote, backslash and controls.
fn text() -> impl Strategy<Value = String> {
    let special = proptest::sample::select(vec!['"', '\\', '\u{0}', '\n', '\u{1f}', 'é', '☃']);
    let ch = prop_oneof![
        proptest::char::range('a', 'z'),
        special,
        proptest::char::any()
    ];
    vec(ch, 0..=8).prop_map(|v| v.into_iter().collect())
}

/// Payloads nest at most 3 containers deep, with at most 4 entries each.
pub fn json_value() -> impl Strategy<Value = Value> {
    let number = prop_oneof![Just(-0.0_f64), finite()]
        .prop_map(|f| Value::Number(Number::from_f64(f).expect("finite")));
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        prop_oneof![Just(i64::MIN), Just(i64::MAX), any::<i64>()].prop_map(Value::from),
        prop_oneof![Just(u64::MAX), any::<u64>()].prop_map(Value::from),
        number,
        text().prop_map(Value::String),
    ];
    leaf.prop_recursive(3, 24, 4, |inner| {
        prop_oneof![
            vec(inner.clone(), 0..=4).prop_map(Value::Array),
            vec((text(), inner), 0..=4)
                .prop_map(|e| Value::Object(e.into_iter().collect::<Map<_, _>>())),
        ]
    })
}

#[derive(Clone, Debug)]
enum KindSpec {
    Group,
    Shape(ShapeGeometry, Option<Color>),
    Extension(String, u32, Value),
}

fn kind_spec() -> impl Strategy<Value = KindSpec> {
    let geometry = proptest::sample::select(vec![ShapeGeometry::Rect, ShapeGeometry::Ellipse]);
    prop_oneof![
        3 => Just(KindSpec::Group),
        3 => (geometry, proptest::option::of(color())).prop_map(|(g, c)| KindSpec::Shape(g, c)),
        2 => ("[a-z][a-z.]{0,7}", any::<u32>(), json_value())
            .prop_map(|(t, v, p)| KindSpec::Extension(t, v, p)),
    ]
}

#[derive(Clone, Debug)]
struct NodeSpec {
    pick: usize,
    kind: KindSpec,
    name: Option<String>,
    placement: Placement,
    opacity: f32,
    blend: BlendMode,
    flags: (bool, bool),
}

fn node_spec() -> impl Strategy<Value = NodeSpec> {
    (
        0..64_usize,
        kind_spec(),
        proptest::option::of(text()),
        placement(),
        opacity(),
        proptest::sample::select(BlendMode::ALL.to_vec()),
        any::<(bool, bool)>(),
    )
        .prop_map(
            |(pick, kind, name, placement, opacity, blend, flags)| NodeSpec {
                pick,
                kind,
                name,
                placement,
                opacity,
                blend,
                flags,
            },
        )
}

/// One axis of a page size: `mode` picks 1 px, 16 384 px, just inside either
/// limit, or a random value in range.
#[derive(Clone, Copy, Debug)]
struct AxisSpec {
    mode: u8,
    frac: f64,
}

#[derive(Clone, Debug)]
struct PageSpec {
    name: String,
    flags: (bool, bool),
    unit: Unit,
    axes: [AxisSpec; 2],
    layout: (Length, Margins, GuideSet, GridSettings),
    nodes: Vec<NodeSpec>,
}

fn page_spec() -> impl Strategy<Value = PageSpec> {
    let axis = (0..5_u8, 0.0..=1.0_f64).prop_map(|(mode, frac)| AxisSpec { mode, frac });
    (
        text(),
        any::<(bool, bool)>(),
        proptest::sample::select(Unit::ALL.to_vec()),
        [axis.clone(), axis],
        (bleed(), margins(), guide_set(), grid_settings()),
        vec(node_spec(), 0..=MAX_NODES),
    )
        .prop_map(|(name, flags, unit, axes, layout, nodes)| PageSpec {
            name,
            flags,
            unit,
            axes,
            layout,
            nodes,
        })
}

#[derive(Clone, Debug)]
struct DocSpec {
    dpi: u32,
    gap: u64,
    keys: Vec<u32>,
    pages: Vec<PageSpec>,
}

fn doc_spec() -> impl Strategy<Value = DocSpec> {
    let dpi = prop_oneof![
        proptest::sample::select(vec![1_u32, 72, 96, 300, 600, 10_000]),
        1..=10_000_u32
    ];
    (
        dpi,
        0..=MAX_GAP,
        vec(any::<u32>(), SLOTS),
        vec(page_spec(), 1..=MAX_PAGES),
    )
        .prop_map(|(dpi, gap, keys, pages)| DocSpec {
            dpi,
            gap,
            keys,
            pages,
        })
}

fn axis_length(axis: AxisSpec, unit: Unit, dpi: u32) -> Length {
    let px = match axis.mode {
        0 => 1.0,
        1 => 16_384.0,
        2 => 1.0 + axis.frac,
        3 => 16_384.0 - axis.frac,
        _ => 1.0 + axis.frac * 16_383.0,
    };
    // Conversion rounds, so exact limits are kept only in px.
    let inside = if unit == Unit::Px {
        px
    } else {
        px.clamp(1.001, 16_383.999)
    };
    Length::new(inside, Unit::Px)
        .convert(unit, f64::from(dpi))
        .unwrap_or_else(|_| Length::new(px, Unit::Px))
}

/// A size valid at `dpi`, falling back to px if a conversion left the range.
fn page_size(spec: &PageSpec, dpi: u32) -> PageSize {
    let size = PageSize {
        width: axis_length(spec.axes[0], spec.unit, dpi),
        height: axis_length(spec.axes[1], spec.unit, dpi),
    };
    if size.check(Dpi::new(dpi)).is_ok() {
        return size;
    }
    PageSize {
        width: Length::new(100.0, Unit::Px),
        height: Length::new(100.0, Unit::Px),
    }
}

fn token() -> EditToken {
    EditToken::__new_for_gafira_edit_only()
}

fn build_node(id: NodeId, spec: &NodeSpec) -> Node {
    let mut base = ElementBase::new(id, spec.placement);
    base.name = spec.name.clone();
    base.opacity = spec.opacity;
    base.blend = spec.blend;
    (base.visible, base.locked) = spec.flags;
    let kind = match &spec.kind {
        KindSpec::Group => NodeKind::Group(GroupNode {}),
        KindSpec::Shape(geometry, fill) => NodeKind::Shape(ShapeNode {
            geometry: *geometry,
            fill: *fill,
            stroke: None,
            corner_radius: 0.0,
        }),
        KindSpec::Extension(type_id, schema_version, payload) => {
            NodeKind::Extension(ExtensionNode {
                type_id: type_id.clone(),
                schema_version: *schema_version,
                payload: payload.clone(),
            })
        }
    };
    Node::new(base, kind)
}

/// Builds a valid document: ids come from the counter, handed out in the
/// order the keys dictate, and each node hangs under an earlier group or a root.
fn build(spec: &DocSpec) -> Document {
    let token = token();
    let mut doc = Document::default();
    {
        let mut edit = doc.edit(&token);
        edit.set_dpi(Dpi::new(spec.dpi));
        let slots = spec.pages.len() + spec.pages.iter().map(|p| p.nodes.len()).sum::<usize>();
        let ids: Vec<u64> = (0..slots)
            .map(|_| edit.alloc_node_id().expect("ids").get())
            .collect();
        for _ in 0..spec.gap {
            edit.alloc_node_id().expect("gap");
        }
        let mut order: Vec<usize> = (0..slots).collect();
        order.sort_by_key(|&i| (spec.keys[i], i));
        let mut next_id = order.into_iter().map(|i| ids[i]);
        for p in &spec.pages {
            let mut page = Page::new(
                PageId::new(next_id.next().expect("slot")),
                p.name.clone(),
                page_size(p, spec.dpi),
            );
            (page.locked, page.hidden) = p.flags;
            (page.bleed, page.margins, page.guides, page.grid) = p.layout.clone();
            let mut nodes: Vec<Node> = Vec::new();
            let mut depths: Vec<usize> = Vec::new();
            for n in &p.nodes {
                let id = NodeId::new(next_id.next().expect("slot"));
                let parents: Vec<usize> = (0..nodes.len())
                    .filter(|&i| {
                        matches!(nodes[i].kind, NodeKind::Group(_)) && depths[i] < MAX_TREE_DEPTH
                    })
                    .collect();
                let slot = n.pick % (parents.len() + 1);
                if slot == parents.len() {
                    page.roots.push(id);
                    depths.push(1);
                } else {
                    nodes[parents[slot]].children.push(id);
                    depths.push(depths[parents[slot]] + 1);
                }
                nodes.push(build_node(id, n));
            }
            for node in nodes {
                page.nodes.insert(node.base.id, Arc::new(node));
            }
            edit.pages_mut().push(Arc::new(page));
        }
    }
    doc
}

/// Valid documents only: 1 to 3 pages, or the empty document.
pub fn document() -> impl Strategy<Value = Document> {
    prop_oneof![
        1 => Just(Document::default()),
        9 => doc_spec().prop_map(|s| build(&s)),
    ]
}

/// One way to break a valid document, named after the rule it breaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Corruption {
    DuplicateIdAcrossPages,
    DuplicateIdPageAndNode,
    ArenaKeyMismatch,
    NextIdNotAboveMax,
    PageSizeBelow,
    PageSizeAbove,
    DanglingRoot,
    DuplicateRoot,
    NonFinite,
    OpacityRange,
    AlphaRange,
    SkewRange,
    WidthRange,
    HeightRange,
    EmptyTypeId,
    ExtensionTooDeep,
    ChildrenOnLeaf,
    DanglingChild,
    MultipleParents,
    RootHasParent,
    Orphan,
    Cycle,
    InvalidDpi,
    UnsupportedSchema,
}

impl Corruption {
    pub const ALL: [Corruption; 24] = [
        Self::DuplicateIdAcrossPages,
        Self::DuplicateIdPageAndNode,
        Self::ArenaKeyMismatch,
        Self::NextIdNotAboveMax,
        Self::PageSizeBelow,
        Self::PageSizeAbove,
        Self::DanglingRoot,
        Self::DuplicateRoot,
        Self::NonFinite,
        Self::OpacityRange,
        Self::AlphaRange,
        Self::SkewRange,
        Self::WidthRange,
        Self::HeightRange,
        Self::EmptyTypeId,
        Self::ExtensionTooDeep,
        Self::ChildrenOnLeaf,
        Self::DanglingChild,
        Self::MultipleParents,
        Self::RootHasParent,
        Self::Orphan,
        Self::Cycle,
        Self::InvalidDpi,
        Self::UnsupportedSchema,
    ];

    /// Whether `error` is the variant this corruption must produce.
    pub fn matches(self, error: &ValidationError, duplicate: Option<u64>) -> bool {
        use ValidationError as E;
        let range =
            |want: NodeField| matches!(error, E::OutOfRange { field, .. } if *field == want);
        match self {
            Self::DuplicateIdAcrossPages | Self::DuplicateIdPageAndNode => {
                matches!(error, E::DuplicateId { id } if Some(*id) == duplicate)
            }
            Self::ArenaKeyMismatch => matches!(error, E::ArenaKeyMismatch { .. }),
            Self::NextIdNotAboveMax => matches!(error, E::NextIdNotAboveMax { .. }),
            Self::PageSizeBelow | Self::PageSizeAbove => matches!(error, E::PageSize { .. }),
            Self::DanglingRoot => matches!(error, E::DanglingRoot { .. }),
            Self::DuplicateRoot => matches!(error, E::DuplicateRoot { .. }),
            Self::NonFinite => matches!(error, E::NonFinite { .. }),
            Self::OpacityRange => range(NodeField::Opacity),
            Self::AlphaRange => range(NodeField::FillAlpha),
            Self::SkewRange => range(NodeField::Skew),
            Self::WidthRange => range(NodeField::W),
            Self::HeightRange => range(NodeField::H),
            Self::EmptyTypeId => matches!(error, E::EmptyExtensionTypeId { .. }),
            Self::ExtensionTooDeep => matches!(error, E::ExtensionTooDeep { .. }),
            Self::ChildrenOnLeaf => matches!(error, E::ChildrenOnLeaf { .. }),
            Self::DanglingChild => matches!(error, E::DanglingChild { .. }),
            Self::MultipleParents => matches!(error, E::MultipleParents { .. }),
            Self::RootHasParent => matches!(error, E::RootHasParent { .. }),
            Self::Orphan => matches!(error, E::Orphan { .. }),
            Self::Cycle => matches!(error, E::Cycle { .. }),
            Self::InvalidDpi => matches!(error, E::InvalidDpi { .. }),
            Self::UnsupportedSchema => matches!(error, E::UnsupportedSchemaVersion { .. }),
        }
    }
}

/// A corrupted document and its wire form.
///
/// `UnsupportedSchema` cannot be built in memory (`schema_version` has no
/// setter), so its `doc` stays valid and only `json` carries the corruption.
#[derive(Debug)]
pub struct Corrupted {
    pub kind: Corruption,
    pub doc: Document,
    pub json: String,
    /// The id the duplicate-id kinds repeat.
    pub duplicate: Option<u64>,
}

impl Corrupted {
    /// Whether `error` is the variant (and, for duplicates, the id) expected.
    pub fn explained_by(&self, error: &ValidationError) -> bool {
        self.kind.matches(error, self.duplicate)
    }

    /// The rule broken: `validate()` on the document, or the load error of
    /// the patched JSON for the wire-only corruption.
    pub fn observed(&self) -> Option<ValidationError> {
        if self.kind == Corruption::UnsupportedSchema {
            match Document::from_json(self.json.as_bytes()) {
                Err(LoadError::Invalid(e)) => Some(e),
                _ => None,
            }
        } else {
            self.doc.validate().err()
        }
    }
}

fn px_size(w: f64, h: f64) -> PageSize {
    PageSize {
        width: Length::new(w, Unit::Px),
        height: Length::new(h, Unit::Px),
    }
}

fn push_page(edit: &mut DocMut<'_>, size: PageSize) -> PageId {
    let id = edit.alloc_page_id().expect("id");
    edit.pages_mut()
        .push(Arc::new(Page::new(id, String::new(), size)));
    id
}

/// The first page, created if the document has none.
fn first_page(edit: &mut DocMut<'_>) -> PageId {
    match edit.as_doc().pages().first() {
        Some(p) => p.id,
        None => push_page(edit, px_size(100.0, 100.0)),
    }
}

/// A node with a fresh id and a unit box.
fn fresh(edit: &mut DocMut<'_>, kind: NodeKind) -> Node {
    let id = edit.alloc_node_id().expect("id");
    Node::new(
        ElementBase::new(id, Placement::new(0.0, 0.0, 1.0, 1.0)),
        kind,
    )
}

fn group() -> NodeKind {
    NodeKind::Group(GroupNode {})
}

fn extension(type_id: &str, payload: Value) -> NodeKind {
    NodeKind::Extension(ExtensionNode {
        type_id: type_id.to_string(),
        schema_version: 1,
        payload,
    })
}

/// Stores `node` in the page's arena without listing it anywhere.
fn insert(edit: &mut DocMut<'_>, page: PageId, node: Node) {
    edit.page_mut(page)
        .expect("page")
        .nodes
        .insert(node.base.id, Arc::new(node));
}

fn add_root(edit: &mut DocMut<'_>, page: PageId, node: Node) {
    edit.page_mut(page).expect("page").roots.push(node.base.id);
    insert(edit, page, node);
}

fn nested(depth: usize) -> Value {
    (0..depth).fold(Value::Null, |v, _| Value::Array(vec![v]))
}

/// Breaks `doc` in exactly one way; `pick` chooses the sub-case.
/// Fresh material is injected, so every corruption applies to every document.
pub fn corrupt(mut doc: Document, kind: Corruption, pick: usize) -> Corrupted {
    let token = token();
    let mut schema_two = false;
    let mut duplicate = None;
    {
        let mut edit = doc.edit(&token);
        let page = first_page(&mut edit);
        match kind {
            Corruption::DuplicateIdAcrossPages => {
                let node = fresh(&mut edit, group());
                duplicate = Some(node.base.id.get());
                add_root(&mut edit, page, node.clone());
                let other = push_page(&mut edit, px_size(100.0, 100.0));
                add_root(&mut edit, other, node);
            }
            Corruption::DuplicateIdPageAndNode => {
                let node = fresh(&mut edit, group());
                let same = PageId::new(node.base.id.get());
                duplicate = Some(same.get());
                add_root(&mut edit, page, node);
                edit.pages_mut().push(Arc::new(Page::new(
                    same,
                    String::new(),
                    px_size(100.0, 100.0),
                )));
            }
            Corruption::ArenaKeyMismatch => {
                let node = fresh(&mut edit, group());
                let key = edit.alloc_node_id().expect("id");
                edit.page_mut(page)
                    .expect("page")
                    .nodes
                    .insert(key, Arc::new(node));
            }
            Corruption::NextIdNotAboveMax => {
                let id = NodeId::new(edit.as_doc().next_id() + (pick % 3) as u64);
                let node = Node::new(
                    ElementBase::new(id, Placement::new(0.0, 0.0, 1.0, 1.0)),
                    group(),
                );
                add_root(&mut edit, page, node);
            }
            Corruption::PageSizeBelow | Corruption::PageSizeAbove => {
                let bad = if kind == Corruption::PageSizeBelow {
                    [0.5, 0.0, 0.999][pick % 3]
                } else {
                    [16_384.5, 20_000.0, 1e9][pick % 3]
                };
                let size = if (pick / 3).is_multiple_of(2) {
                    px_size(bad, 100.0)
                } else {
                    px_size(100.0, bad)
                };
                push_page(&mut edit, size);
            }
            Corruption::DanglingRoot => {
                let id = edit.alloc_node_id().expect("id");
                edit.page_mut(page).expect("page").roots.push(id);
            }
            Corruption::DuplicateRoot => {
                let node = fresh(&mut edit, group());
                let id = node.base.id;
                add_root(&mut edit, page, node);
                edit.page_mut(page).expect("page").roots.push(id);
            }
            Corruption::NonFinite => {
                let mut node = fresh(&mut edit, group());
                let bad = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY][(pick / 7) % 3];
                let p = &mut node.base.placement;
                match pick % 7 {
                    0 => p.x = bad,
                    1 => p.y = bad,
                    2 => p.w = bad,
                    3 => p.h = bad,
                    4 => p.rotation_deg = bad,
                    5 => p.skew = bad,
                    _ => node.base.opacity = bad as f32,
                }
                add_root(&mut edit, page, node);
            }
            Corruption::OpacityRange => {
                let mut node = fresh(&mut edit, group());
                node.base.opacity = [1.0001, -0.0001, 2.0, -1.0, f32::MAX][pick % 5];
                add_root(&mut edit, page, node);
            }
            Corruption::AlphaRange => {
                let alpha = [1.0001, -0.0001, 2.0, -1.0, f32::MAX][pick % 5];
                let fill = Color::Srgb(Rgba::new(0.0, 0.0, 0.0, alpha));
                let kind = NodeKind::Shape(ShapeNode {
                    geometry: ShapeGeometry::Rect,
                    fill: Some(fill),
                    stroke: None,
                    corner_radius: 0.0,
                });
                let node = fresh(&mut edit, kind);
                add_root(&mut edit, page, node);
            }
            Corruption::SkewRange => {
                let mut node = fresh(&mut edit, group());
                node.base.placement.skew = [90.0, -90.0, 91.0, 1e3, -1e3][pick % 5];
                add_root(&mut edit, page, node);
            }
            Corruption::WidthRange | Corruption::HeightRange => {
                let mut node = fresh(&mut edit, group());
                let bad = [-0.001, -1.0, -1e9, f64::MIN][pick % 4];
                if kind == Corruption::WidthRange {
                    node.base.placement.w = bad;
                } else {
                    node.base.placement.h = bad;
                }
                add_root(&mut edit, page, node);
            }
            Corruption::EmptyTypeId => {
                let node = fresh(&mut edit, extension("", Value::Null));
                add_root(&mut edit, page, node);
            }
            Corruption::ExtensionTooDeep => {
                let node = fresh(&mut edit, extension("deep", nested(65 + pick % 3)));
                add_root(&mut edit, page, node);
            }
            Corruption::ChildrenOnLeaf => {
                let leaf = if pick.is_multiple_of(2) {
                    NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect))
                } else {
                    extension("leaf", Value::Null)
                };
                let child = fresh(&mut edit, group());
                let mut node = fresh(&mut edit, leaf);
                node.children.push(child.base.id);
                insert(&mut edit, page, child);
                add_root(&mut edit, page, node);
            }
            Corruption::DanglingChild => {
                let missing = edit.alloc_node_id().expect("id");
                let mut node = fresh(&mut edit, group());
                node.children.push(missing);
                add_root(&mut edit, page, node);
            }
            Corruption::MultipleParents => {
                let child = fresh(&mut edit, group());
                let mut first = fresh(&mut edit, group());
                let mut second = fresh(&mut edit, group());
                first.children.push(child.base.id);
                second.children.push(child.base.id);
                insert(&mut edit, page, child);
                add_root(&mut edit, page, first);
                add_root(&mut edit, page, second);
            }
            Corruption::RootHasParent => {
                let child = fresh(&mut edit, group());
                let mut parent = fresh(&mut edit, group());
                parent.children.push(child.base.id);
                add_root(&mut edit, page, child);
                add_root(&mut edit, page, parent);
            }
            Corruption::Orphan => {
                let node = fresh(&mut edit, group());
                insert(&mut edit, page, node);
            }
            Corruption::Cycle => {
                let mut a = fresh(&mut edit, group());
                let mut b = fresh(&mut edit, group());
                if pick.is_multiple_of(2) {
                    a.children.push(b.base.id);
                    b.children.push(a.base.id);
                    insert(&mut edit, page, b);
                } else {
                    a.children.push(a.base.id);
                }
                insert(&mut edit, page, a);
            }
            Corruption::InvalidDpi => edit.set_dpi(Dpi::new([0, 10_001, u32::MAX][pick % 3])),
            Corruption::UnsupportedSchema => schema_two = true,
        }
    }
    let mut json = doc.to_json().expect("serializes");
    if schema_two {
        json = json.replacen("\"schema_version\":1", "\"schema_version\":2", 1);
    }
    Corrupted {
        kind,
        doc,
        json,
        duplicate,
    }
}

/// A valid generated document with one corruption applied.
pub fn corrupted() -> impl Strategy<Value = Corrupted> {
    (
        document(),
        proptest::sample::select(Corruption::ALL.to_vec()),
        0..1000_usize,
    )
        .prop_map(|(doc, kind, pick)| corrupt(doc, kind, pick))
}

#[cfg(test)]
mod guards {
    use std::collections::BTreeSet;

    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;

    use super::*;

    const DRAWS: usize = 20_000;

    fn draws<S: Strategy>(strategy: S) -> impl Iterator<Item = S::Value> {
        let mut runner = TestRunner::deterministic();
        (0..DRAWS).map(move |_| strategy.new_tree(&mut runner).expect("tree").current())
    }

    fn value_depth(v: &Value) -> usize {
        match v {
            Value::Array(a) => 1 + a.iter().map(value_depth).max().unwrap_or(0),
            Value::Object(o) => 1 + o.values().map(value_depth).max().unwrap_or(0),
            _ => 0,
        }
    }

    /// Entries per container and string lengths are within the bounds.
    fn payload_in_bounds(v: &Value) -> bool {
        let short = |s: &String| s.chars().count() <= 8;
        match v {
            Value::String(s) => short(s),
            Value::Array(a) => a.len() <= 4 && a.iter().all(payload_in_bounds),
            Value::Object(o) => {
                o.len() <= 4 && o.iter().all(|(k, v)| short(k) && payload_in_bounds(v))
            }
            _ => true,
        }
    }

    fn tree_depth(page: &Page, id: NodeId) -> usize {
        let node = page.node(id).expect("child in arena");
        1 + node
            .children
            .iter()
            .map(|c| tree_depth(page, *c))
            .max()
            .unwrap_or(0)
    }

    /// Records the special characters a string holds.
    fn note_text(seen: &mut BTreeSet<&'static str>, s: &str) {
        for c in s.chars() {
            match c {
                '"' => seen.insert("quote"),
                '\\' => seen.insert("backslash"),
                c if c.is_control() => seen.insert("control char"),
                c if !c.is_ascii() => seen.insert("non-ASCII char"),
                _ => false,
            };
        }
    }

    fn note_payload(seen: &mut BTreeSet<&'static str>, v: &Value) {
        match v {
            Value::String(s) => note_text(seen, s),
            Value::Array(a) => a.iter().for_each(|v| note_payload(seen, v)),
            Value::Object(o) => o.iter().for_each(|(k, v)| {
                note_text(seen, k);
                note_payload(seen, v);
            }),
            _ => {}
        }
    }

    fn note_f64(seen: &mut BTreeSet<&'static str>, v: f64) {
        if v == 0.0 && v.is_sign_negative() {
            seen.insert("-0.0");
        }
        if v.is_subnormal() {
            seen.insert("subnormal");
        }
    }

    #[test]
    fn generated_documents_reach_every_feature_within_the_bounds() {
        let mut seen = BTreeSet::new();
        for doc in draws(document()) {
            assert_eq!(doc.validate(), Ok(()));
            assert!(doc.pages().len() <= MAX_PAGES);
            if doc.pages().is_empty() {
                seen.insert("empty document");
            }
            if doc.pages().len() > 1 {
                seen.insert("several pages");
            }
            let mut max_id = 0;
            for page in doc.pages() {
                max_id = max_id.max(page.id.get());
                note_text(&mut seen, &page.name);
                assert!(page.nodes.len() <= MAX_NODES);
                if page.nodes.is_empty() {
                    seen.insert("empty page");
                }
                let px = page.size.to_px(doc.dpi()).expect("valid size");
                for (value, name) in [(1.0, "px 1.0"), (16_384.0, "px 16384.0")] {
                    if px.width() == value || px.height() == value {
                        seen.insert(name);
                    }
                }
                for root in &page.roots {
                    let depth = tree_depth(page, *root);
                    assert!(depth <= MAX_TREE_DEPTH);
                    if depth >= 3 {
                        seen.insert("depth 3");
                    }
                }
                for node in page.nodes.values() {
                    max_id = max_id.max(node.base.id.get());
                    if node.base.id.get() < page.id.get() {
                        seen.insert("ids out of order");
                    }
                    note_text(&mut seen, node.base.name.as_deref().unwrap_or(""));
                    let p = &node.base.placement;
                    for v in [p.x, p.y, p.w, p.h, p.rotation_deg, p.skew] {
                        note_f64(&mut seen, v);
                    }
                    let o = node.base.opacity;
                    if o == 0.0 {
                        seen.insert(if o.is_sign_negative() {
                            "opacity -0.0"
                        } else {
                            "opacity 0.0"
                        });
                    }
                    if o == 1.0 {
                        seen.insert("opacity 1.0");
                    }
                    match &node.kind {
                        NodeKind::Group(_) => {
                            if !node.children.is_empty() {
                                seen.insert("group with children");
                            }
                        }
                        NodeKind::Extension(e) => {
                            seen.insert("extension");
                            note_text(&mut seen, &e.type_id);
                            note_payload(&mut seen, &e.payload);
                            assert!(node.children.is_empty());
                            assert!(value_depth(&e.payload) <= 3);
                            assert!(payload_in_bounds(&e.payload));
                        }
                        NodeKind::Shape(_) | NodeKind::Text(_) | NodeKind::Image(_) => {
                            assert!(node.children.is_empty());
                        }
                    }
                }
            }
            let gap = doc.next_id() - 1 - max_id;
            assert!(gap <= MAX_GAP);
            if gap > 0 {
                seen.insert("id gap");
            }
        }
        let wanted = [
            "-0.0",
            "subnormal",
            "px 1.0",
            "px 16384.0",
            "opacity 0.0",
            "opacity -0.0",
            "opacity 1.0",
            "depth 3",
            "extension",
            "group with children",
            "several pages",
            "empty page",
            "empty document",
            "ids out of order",
            "id gap",
            "quote",
            "backslash",
            "control char",
            "non-ASCII char",
        ];
        let missing: Vec<_> = wanted.iter().filter(|f| !seen.contains(*f)).collect();
        assert!(missing.is_empty(), "never reached: {missing:?}");
    }

    #[test]
    fn every_corruption_is_produced_and_breaks_its_own_rule() {
        let mut seen = BTreeSet::new();
        for c in draws(corrupted()) {
            let error = c.observed().expect("a corruption is detected");
            assert!(c.explained_by(&error), "{:?} produced {error:?}", c.kind);
            seen.insert(c.kind);
        }
        let missing: Vec<_> = Corruption::ALL
            .iter()
            .filter(|k| !seen.contains(k))
            .collect();
        assert!(missing.is_empty(), "never produced: {missing:?}");
    }
}
