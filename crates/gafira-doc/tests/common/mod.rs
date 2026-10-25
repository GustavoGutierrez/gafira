//! Bounded document generator and its deterministic coverage guard.

use std::sync::Arc;

use gafira_color::{Color, DisplayP3Rgba, LinearRgba, Rgba};
use gafira_doc::{
    BlendMode, Document, Dpi, EditToken, ElementBase, ExtensionNode, Flip, GroupNode, Node, NodeId,
    NodeKind, Page, PageId, PageSize, Placement, ShapeGeometry, ShapeNode,
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
fn json_value() -> impl Strategy<Value = Value> {
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
    nodes: Vec<NodeSpec>,
}

fn page_spec() -> impl Strategy<Value = PageSpec> {
    let axis = (0..5_u8, 0.0..=1.0_f64).prop_map(|(mode, frac)| AxisSpec { mode, frac });
    (
        text(),
        any::<(bool, bool)>(),
        proptest::sample::select(Unit::ALL.to_vec()),
        [axis.clone(), axis],
        vec(node_spec(), 0..=MAX_NODES),
    )
        .prop_map(|(name, flags, unit, axes, nodes)| PageSpec {
            name,
            flags,
            unit,
            axes,
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
                            assert!(node.children.is_empty());
                            assert!(value_depth(&e.payload) <= 3);
                            assert!(payload_in_bounds(&e.payload));
                        }
                        NodeKind::Shape(_) => assert!(node.children.is_empty()),
                    }
                }
            }
            assert!(doc.next_id() - 1 - max_id <= MAX_GAP);
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
        ];
        let missing: Vec<_> = wanted.iter().filter(|f| !seen.contains(*f)).collect();
        assert!(missing.is_empty(), "never reached: {missing:?}");
    }
}
