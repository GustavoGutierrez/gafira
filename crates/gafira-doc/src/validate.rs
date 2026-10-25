//! `Document::validate`: the structural rules, one pass at a time.

use std::collections::BTreeSet;

use gafira_color::Color;
use serde_json::Value;

use crate::node::MAX_EXTENSION_DEPTH;
use crate::{Document, Node, NodeField, NodeKind, Page, ValidationError};

impl Document {
    /// Returns the first rule the document breaks, or `Ok(())`.
    ///
    /// Passes run in a fixed order. In P1 to P3 each check runs over all
    /// pages (in `pages` order) before the next check, in the order of the
    /// [`ValidationError`] variants; P4 goes node by node. Nothing is altered
    /// or clamped.
    pub fn validate(&self) -> Result<(), ValidationError> {
        self.check_header()?;
        self.check_ids()?;
        self.check_pages()?;
        self.check_nodes()
    }

    /// P1: schema version, then DPI.
    fn check_header(&self) -> Result<(), ValidationError> {
        if self.schema_version != Self::SCHEMA_VERSION {
            return Err(ValidationError::UnsupportedSchemaVersion {
                found: self.schema_version,
            });
        }
        if !self.dpi.is_valid() {
            return Err(ValidationError::InvalidDpi {
                dpi: self.dpi.get(),
            });
        }
        Ok(())
    }

    /// P2: distinct ids, arena keys, then the `next_id` bound.
    fn check_ids(&self) -> Result<(), ValidationError> {
        let ids = || {
            self.pages.iter().flat_map(|p| {
                let nodes = p.nodes.values().map(|n| n.base.id.get());
                std::iter::once(p.id.get()).chain(nodes)
            })
        };
        let mut seen = BTreeSet::new();
        if let Some(id) = ids().find(|&id| !seen.insert(id)) {
            return Err(ValidationError::DuplicateId { id });
        }
        for page in &self.pages {
            if let Some((&key, node)) = page.nodes.iter().find(|(k, n)| **k != n.base.id) {
                let (page, id) = (page.id, node.base.id);
                return Err(ValidationError::ArenaKeyMismatch { page, key, id });
            }
        }
        match ids().max() {
            Some(max_id) if self.next_id <= max_id => Err(ValidationError::NextIdNotAboveMax {
                next_id: self.next_id,
                max_id,
            }),
            _ => Ok(()),
        }
    }

    /// P3: page sizes, then roots that do not exist, then repeated roots.
    fn check_pages(&self) -> Result<(), ValidationError> {
        for page in &self.pages {
            let size = page.size.check(self.dpi);
            size.map_err(|error| ValidationError::PageSize {
                page: page.id,
                error,
            })?;
        }
        for page in &self.pages {
            if let Some(&node) = page.roots.iter().find(|r| !page.nodes.contains_key(r)) {
                return Err(ValidationError::DanglingRoot {
                    page: page.id,
                    node,
                });
            }
        }
        for page in &self.pages {
            let mut seen = BTreeSet::new();
            if let Some(&node) = page.roots.iter().find(|&&r| !seen.insert(r)) {
                return Err(ValidationError::DuplicateRoot {
                    page: page.id,
                    node,
                });
            }
        }
        Ok(())
    }

    /// P4: node by node (ascending id), each node's checks in order.
    fn check_nodes(&self) -> Result<(), ValidationError> {
        for page in &self.pages {
            for node in page.nodes.values() {
                check_node(page, node)?;
            }
        }
        Ok(())
    }
}

type Rule = fn(f64) -> bool;

fn any(_: f64) -> bool {
    true
}

fn non_negative(v: f64) -> bool {
    v >= 0.0
}

fn below_quarter_turn(v: f64) -> bool {
    v > -90.0 && v < 90.0
}

fn unit_interval(v: f64) -> bool {
    (0.0..=1.0).contains(&v)
}

/// The numeric fields of a node, in reporting order, with their range rule.
fn numbers(node: &Node) -> Vec<(NodeField, f64, Rule)> {
    let p = &node.base.placement;
    let mut out: Vec<(NodeField, f64, Rule)> = vec![
        (NodeField::X, p.x, any),
        (NodeField::Y, p.y, any),
        (NodeField::W, p.w, non_negative),
        (NodeField::H, p.h, non_negative),
        (NodeField::RotationDeg, p.rotation_deg, any),
        (NodeField::Skew, p.skew, below_quarter_turn),
        (
            NodeField::Opacity,
            f64::from(node.base.opacity),
            unit_interval,
        ),
    ];
    if let NodeKind::Shape(shape) = &node.kind
        && let Some(fill) = shape.fill
    {
        let [r, g, b, a] = match fill {
            Color::Srgb(c) => [c.r, c.g, c.b, c.a],
            Color::LinearSrgb(c) => [c.r, c.g, c.b, c.a],
            Color::DisplayP3(c) => [c.r, c.g, c.b, c.a],
        };
        for channel in [r, g, b] {
            out.push((NodeField::FillChannel, f64::from(channel), any));
        }
        out.push((NodeField::FillAlpha, f64::from(a), unit_interval));
    }
    out
}

/// The checks 9 to 14 of one node; the first failure wins.
fn check_node(page: &Page, node: &Node) -> Result<(), ValidationError> {
    let id = node.base.id;
    let numbers = numbers(node);
    if let Some(&(field, _, _)) = numbers.iter().find(|(_, v, _)| !v.is_finite()) {
        return Err(ValidationError::NonFinite { node: id, field });
    }
    if let Some(&(field, value, _)) = numbers.iter().find(|(_, v, ok)| !ok(*v)) {
        return Err(ValidationError::OutOfRange {
            node: id,
            field,
            value,
        });
    }
    if let NodeKind::Extension(ext) = &node.kind {
        if ext.type_id.is_empty() {
            return Err(ValidationError::EmptyExtensionTypeId { node: id });
        }
        let depth = depth(&ext.payload);
        if depth > MAX_EXTENSION_DEPTH {
            return Err(ValidationError::ExtensionTooDeep { node: id, depth });
        }
    }
    if !matches!(node.kind, NodeKind::Group(_)) && !node.children.is_empty() {
        return Err(ValidationError::ChildrenOnLeaf { node: id });
    }
    match node.children.iter().find(|c| !page.nodes.contains_key(c)) {
        Some(&child) => Err(ValidationError::DanglingChild { parent: id, child }),
        None => Ok(()),
    }
}

/// Nesting depth of a JSON value: a scalar is 0, `[]` and `{}` are 1.
/// Iterative, so a hostile payload cannot overflow the stack.
fn depth(root: &Value) -> usize {
    let mut deepest = 0;
    let mut stack = vec![(root, 0usize)];
    while let Some((value, level)) = stack.pop() {
        let children: Vec<&Value> = match value {
            Value::Array(items) => items.iter().collect(),
            Value::Object(map) => map.values().collect(),
            _ => continue,
        };
        deepest = deepest.max(level + 1);
        stack.extend(children.into_iter().map(|c| (c, level + 1)));
    }
    deepest
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gafira_color::{Color, LinearRgba, Rgba};
    use gafira_geom::{Length, Unit};
    use serde_json::{Value, json};

    use crate::ValidationError;
    use crate::document::test_document;
    use crate::{
        Axis, Document, Dpi, ElementBase, ExtensionNode, GroupNode, Node, NodeId, NodeKind, Page,
        PageId, PageSize, PageSizeError, Placement, ShapeGeometry, ShapeNode,
    };

    type Edit = fn(&mut Document);
    type EditRef<'a> = &'a dyn Fn(&mut Document);

    fn px(w: f64, h: f64) -> PageSize {
        let side = |v| Length::new(v, Unit::Px);
        PageSize {
            width: side(w),
            height: side(h),
        }
    }

    fn node(id: u64, kind: NodeKind) -> Arc<Node> {
        let base = ElementBase::new(NodeId::new(id), Placement::new(0.0, 0.0, 1.0, 1.0));
        Arc::new(Node::new(base, kind))
    }

    /// A page of 100 x 100 px whose nodes are all root groups.
    fn page(id: u64, nodes: &[u64]) -> Page {
        let mut p = Page::new(PageId::new(id), String::new(), px(100.0, 100.0));
        for &n in nodes {
            p.roots.push(NodeId::new(n));
            p.nodes
                .insert(NodeId::new(n), node(n, NodeKind::Group(GroupNode {})));
        }
        p
    }

    /// Page 1 with nodes 2 and 3, `next_id` 10, DPI 96: valid.
    fn base() -> Document {
        test_document(10, Dpi::default(), vec![Arc::new(page(1, &[2, 3]))])
    }

    fn first(d: &mut Document) -> &mut Page {
        Arc::make_mut(&mut d.pages[0])
    }

    fn push(d: &mut Document, id: u64, nodes: &[u64]) {
        d.pages.push(Arc::new(page(id, nodes)));
    }

    fn roots(p: &mut Page, ids: &[u64]) {
        p.roots = ids.iter().map(|&i| NodeId::new(i)).collect();
    }

    /// Appends page 7 (nodes 8 and 9) and edits it, so the defect is on the second page.
    fn two(d: &mut Document, edit: fn(&mut Page)) {
        push(d, 7, &[8, 9]);
        edit(Arc::make_mut(&mut d.pages[1]));
    }

    /// Moves the highest node of a page under key 5 (which must not collide with an existing id).
    fn rekey(p: &mut Page) {
        let (_, n) = p.nodes.pop_last().unwrap();
        p.nodes.insert(NodeId::new(5), n);
    }

    /// Appends page 7 holding node 10 and sets `next_id`.
    fn ten(d: &mut Document, next_id: u64) {
        push(d, 7, &[10]);
        d.next_id = next_id;
    }

    /// Appends page `page` holding `nodes` and sets `next_id` to `u64::MAX`.
    fn at_max(d: &mut Document, page: u64, nodes: &[u64]) {
        push(d, page, nodes);
        d.next_id = u64::MAX;
    }

    /// Applies each edit to a valid document and compares the `Debug` form of the result.
    fn run(rows: &[(Edit, &str)]) {
        for (i, (edit, want)) in rows.iter().enumerate() {
            let mut d = base();
            edit(&mut d);
            assert_eq!(format!("{:?}", d.validate()), *want, "row {i}");
        }
    }

    fn out(page: u64, axis: &str, px: f64) -> String {
        let err = format!("OutOfRange {{ axis: {axis}, px: {px:?} }}");
        format!("Err(PageSize {{ page: PageId({page}), error: {err} }})")
    }

    fn next(next_id: u64, max_id: u64) -> String {
        format!("Err(NextIdNotAboveMax {{ next_id: {next_id}, max_id: {max_id} }})")
    }

    fn root(kind: &str, page: u64, node: u64) -> String {
        format!("Err({kind} {{ page: PageId({page}), node: NodeId({node}) }})")
    }

    fn key(page: u64, key: u64, id: u64) -> String {
        let (page, key, id) = (format!("PageId({page})"), format!("NodeId({key})"), id);
        format!("Err(ArenaKeyMismatch {{ page: {page}, key: {key}, id: NodeId({id}) }})")
    }

    #[test]
    fn valid_documents_pass() {
        assert_eq!(Document::default().validate(), Ok(()));
        assert_eq!(base().validate(), Ok(()));
        let base = ElementBase::new(NodeId::new(2), Placement::new(0.0, 0.0, 9.0, 9.0));
        let mut group = Node::new(base, NodeKind::Group(GroupNode {}));
        group.children = vec![NodeId::new(3), NodeId::new(4)];
        let ext = ExtensionNode {
            type_id: "x".to_string(),
            schema_version: 1,
            payload: json!([1]),
        };
        let mut p = page(1, &[]);
        p.roots = vec![NodeId::new(2)];
        for n in [
            Arc::new(group),
            node(3, NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect))),
            node(4, NodeKind::Extension(ext)),
        ] {
            p.nodes.insert(n.base.id, n);
        }
        let d = test_document(5, Dpi::default(), vec![Arc::new(p)]);
        assert_eq!(d.validate(), Ok(()));
    }

    #[test]
    fn header_rows() {
        run(&[
            (
                |d| d.schema_version = 0,
                "Err(UnsupportedSchemaVersion { found: 0 })",
            ),
            (
                |d| d.schema_version = 2,
                "Err(UnsupportedSchemaVersion { found: 2 })",
            ),
            (
                |d| d.schema_version = u32::MAX,
                "Err(UnsupportedSchemaVersion { found: 4294967295 })",
            ),
            (|d| d.dpi = Dpi::new(0), "Err(InvalidDpi { dpi: 0 })"),
            (|d| d.dpi = Dpi::new(1), "Ok(())"),
            (|d| d.dpi = Dpi::new(10_000), "Ok(())"),
            (
                |d| d.dpi = Dpi::new(10_001),
                "Err(InvalidDpi { dpi: 10001 })",
            ),
            (
                |d| d.dpi = Dpi::new(u32::MAX),
                "Err(InvalidDpi { dpi: 4294967295 })",
            ),
        ]);
    }

    #[test]
    fn id_rows() {
        let max = next(u64::MAX, u64::MAX);
        run(&[
            (|d| push(d, 1, &[]), "Err(DuplicateId { id: 1 })"),
            (|d| push(d, 7, &[3]), "Err(DuplicateId { id: 3 })"),
            (|d| push(d, 2, &[]), "Err(DuplicateId { id: 2 })"),
            (|d| rekey(first(d)), &key(1, 5, 3)),
            (|d| two(d, rekey), &key(7, 5, 9)),
            (|d| ten(d, 11), "Ok(())"),
            (|d| ten(d, 10), &next(10, 10)),
            (|d| ten(d, 5), &next(5, 10)),
            (|d| ten(d, 0), &next(0, 10)),
            (
                |d| {
                    d.pages.clear();
                    d.next_id = 0;
                },
                "Ok(())",
            ),
            (|d| at_max(d, u64::MAX, &[]), &max),
            (|d| at_max(d, 7, &[u64::MAX]), &max),
        ]);
    }

    #[test]
    fn page_rows() {
        fn a0(d: &mut Document, dpi: u32) {
            let side = |v| Length::new(v, Unit::Mm);
            first(d).size = PageSize {
                width: side(841.0),
                height: side(1189.0),
            };
            d.dpi = Dpi::new(dpi);
        }
        let (low, high) = (1.0f64.next_down(), 16_384.0f64.next_up());
        let a0_px = Length::new(1189.0, Unit::Mm).to_px(351.0).unwrap();
        run(&[
            (|d| first(d).size = px(1.0, 1.0), "Ok(())"),
            (|d| first(d).size = px(16_384.0, 16_384.0), "Ok(())"),
            (
                |d| first(d).size = px(1.0f64.next_down(), 5.0),
                &out(1, "Width", low),
            ),
            (
                |d| first(d).size = px(5.0, 16_384.0f64.next_up()),
                &out(1, "Height", high),
            ),
            (|d| first(d).size = px(0.0, 5.0), &out(1, "Width", 0.0)),
            (|d| first(d).size = px(-5.0, 5.0), &out(1, "Width", -5.0)),
            (|d| a0(d, 300), "Ok(())"),
            (|d| a0(d, 350), "Ok(())"),
            (|d| a0(d, 351), &out(1, "Height", a0_px)),
            (|d| two(d, |p| p.size = px(0.0, 5.0)), &out(7, "Width", 0.0)),
        ]);
        for width in [f64::NAN, f64::INFINITY] {
            let mut d = base();
            first(&mut d).size = px(width, 5.0);
            let Err(ValidationError::PageSize {
                page,
                error: PageSizeError::Conversion { axis, .. },
            }) = d.validate()
            else {
                panic!("expected a conversion error for {width}");
            };
            assert_eq!((page, axis), (PageId::new(1), Axis::Width));
        }
    }

    #[test]
    fn root_rows() {
        fn elsewhere(d: &mut Document) {
            push(d, 7, &[9]);
            roots(first(d), &[2, 9]);
        }
        run(&[
            (|d| roots(first(d), &[42]), &root("DanglingRoot", 1, 42)),
            (
                |d| two(d, |p| roots(p, &[42])),
                &root("DanglingRoot", 7, 42),
            ),
            (|d| roots(first(d), &[3, 3]), &root("DuplicateRoot", 1, 3)),
            (
                |d| two(d, |p| roots(p, &[9, 9])),
                &root("DuplicateRoot", 7, 9),
            ),
            (
                elsewhere,
                "Err(DanglingRoot { page: PageId(1), node: NodeId(9) })",
            ),
        ]);
    }

    #[test]
    fn earlier_rules_win() {
        run(&[
            // P1 before P2.
            (
                |d| {
                    d.schema_version = 2;
                    push(d, 1, &[]);
                },
                "Err(UnsupportedSchemaVersion { found: 2 })",
            ),
            // P2 before P3.
            (
                |d| {
                    push(d, 1, &[]);
                    first(d).size = px(0.0, 5.0);
                },
                "Err(DuplicateId { id: 1 })",
            ),
            // P4 inside the pass: a lower id wins over a higher id's earlier-listed check.
            (
                |d| {
                    node_mut(d, 2, |n| n.base.opacity = 2.0);
                    node_mut(d, 3, |n| n.children = vec![NodeId::new(99)]);
                },
                &range(2, "Opacity", 2.0),
            ),
            // Inside P1: schema before DPI.
            (
                |d| {
                    d.schema_version = 2;
                    d.dpi = Dpi::new(0);
                },
                "Err(UnsupportedSchemaVersion { found: 2 })",
            ),
            // Inside P2: duplicate id before `next_id`.
            (
                |d| {
                    push(d, 1, &[]);
                    d.next_id = 0;
                },
                "Err(DuplicateId { id: 1 })",
            ),
            // Inside P2: duplicate id before arena key.
            (
                |d| {
                    push(d, 1, &[]);
                    rekey(first(d));
                },
                "Err(DuplicateId { id: 1 })",
            ),
            // Inside P3: dangling root before repeated root.
            (
                |d| roots(first(d), &[42, 42]),
                "Err(DanglingRoot { page: PageId(1), node: NodeId(42) })",
            ),
            // Inside P3, each check runs over all pages before the next one.
            (
                |d| {
                    roots(first(d), &[42]);
                    two(d, |p| p.size = px(0.0, 5.0));
                },
                &out(7, "Width", 0.0),
            ),
        ]);
    }

    fn node_mut(d: &mut Document, id: u64, edit: impl FnOnce(&mut Node)) {
        let node = first(d).nodes.get_mut(&NodeId::new(id)).unwrap();
        edit(Arc::make_mut(node));
    }

    fn non_finite(node: u64, field: &str) -> String {
        format!("Err(NonFinite {{ node: NodeId({node}), field: {field} }})")
    }

    fn range(node: u64, field: &str, value: f64) -> String {
        let (node, field) = (format!("NodeId({node})"), field);
        format!("Err(OutOfRange {{ node: {node}, field: {field}, value: {value:?} }})")
    }

    fn extension(type_id: &str, payload: Value) -> NodeKind {
        NodeKind::Extension(ExtensionNode {
            type_id: type_id.to_string(),
            schema_version: 1,
            payload,
        })
    }

    /// Nested `levels` deep: `[]` is 1, `[[]]` is 2.
    fn nested(levels: usize, object: bool) -> Value {
        (1..levels).fold(if object { json!({}) } else { json!([]) }, |inner, _| {
            if object {
                json!({ "a": inner })
            } else {
                Value::Array(vec![inner])
            }
        })
    }

    fn shape_with(fill: Color) -> NodeKind {
        let mut shape = ShapeNode::new(ShapeGeometry::Rect);
        shape.fill = Some(fill);
        NodeKind::Shape(shape)
    }

    #[test]
    fn node_value_rows() {
        type Set = fn(&mut Node, f64);
        let nan = f64::NAN;
        let rows: Vec<(Set, f64, String)> = vec![
            (|n, v| n.base.opacity = v as f32, 0.0, "Ok(())".into()),
            (|n, v| n.base.opacity = v as f32, -0.0, "Ok(())".into()),
            (|n, v| n.base.opacity = v as f32, 1.0, "Ok(())".into()),
            (|n, v| n.base.opacity = v as f32, 1e-45, "Ok(())".into()),
            (
                |n, v| n.base.opacity = v as f32,
                -0.0000001,
                range(2, "Opacity", f64::from(-0.0000001f32)),
            ),
            (
                |n, v| n.base.opacity = v as f32,
                1.0000001,
                range(2, "Opacity", f64::from(1.0000001f32)),
            ),
            (
                |n, v| n.base.opacity = v as f32,
                2.0,
                range(2, "Opacity", 2.0),
            ),
            (
                |n, v| n.base.opacity = v as f32,
                nan,
                non_finite(2, "Opacity"),
            ),
            (
                |n, v| n.base.opacity = v as f32,
                f64::INFINITY,
                non_finite(2, "Opacity"),
            ),
            (
                |n, v| n.base.opacity = v as f32,
                f64::NEG_INFINITY,
                non_finite(2, "Opacity"),
            ),
            (|n, v| n.base.placement.x = v, nan, non_finite(2, "X")),
            (
                |n, v| n.base.placement.y = v,
                f64::INFINITY,
                non_finite(2, "Y"),
            ),
            (
                |n, v| n.base.placement.rotation_deg = v,
                f64::NEG_INFINITY,
                non_finite(2, "RotationDeg"),
            ),
            (|n, v| n.base.placement.skew = v, nan, non_finite(2, "Skew")),
            (|n, v| n.base.placement.w = v, -1.0, range(2, "W", -1.0)),
            (
                |n, v| n.base.placement.h = v,
                -1e-300,
                range(2, "H", -1e-300),
            ),
            (
                |n, v| n.base.placement.skew = v,
                90.0,
                range(2, "Skew", 90.0),
            ),
            (
                |n, v| n.base.placement.skew = v,
                -90.0,
                range(2, "Skew", -90.0),
            ),
            (|n, v| n.base.placement.w = v, -0.0, "Ok(())".into()),
            (|n, v| n.base.placement.w = v, 0.0, "Ok(())".into()),
            (
                |n, v| n.base.placement.rotation_deg = v,
                720.0,
                "Ok(())".into(),
            ),
            (|n, v| n.base.placement.skew = v, 89.999, "Ok(())".into()),
            (|n, v| n.base.placement.skew = v, -89.999, "Ok(())".into()),
            (
                |n, v| n.kind = shape_with(Color::Srgb(Rgba::new(v as f32, 0.0, 0.0, 1.0))),
                nan,
                non_finite(2, "FillChannel"),
            ),
            (
                |n, v| n.kind = shape_with(Color::Srgb(Rgba::new(0.0, 0.0, 0.0, v as f32))),
                nan,
                non_finite(2, "FillAlpha"),
            ),
            (
                |n, v| n.kind = shape_with(Color::Srgb(Rgba::new(0.0, 0.0, 0.0, v as f32))),
                1.5,
                range(2, "FillAlpha", 1.5),
            ),
            (
                |n, v| n.kind = shape_with(Color::Srgb(Rgba::new(v as f32, 1.75, 0.0, 0.5))),
                -0.5,
                "Ok(())".into(),
            ),
            (
                |n, v| {
                    n.kind = shape_with(Color::LinearSrgb(LinearRgba::new(0.0, v as f32, 0.0, 1.0)))
                },
                nan,
                non_finite(2, "FillChannel"),
            ),
            (
                |n, v| {
                    n.kind = shape_with(Color::LinearSrgb(LinearRgba::new(0.0, 0.0, 0.0, v as f32)))
                },
                1.5,
                range(2, "FillAlpha", 1.5),
            ),
            (
                |n, v| n.kind = shape_with(Color::Srgb(Rgba::new(0.0, 0.0, v as f32, 1.0))),
                f32::MAX.into(),
                "Ok(())".into(),
            ),
        ];
        for (i, (set, value, want)) in rows.iter().enumerate() {
            let mut d = base();
            node_mut(&mut d, 2, |n| set(n, *value));
            assert_eq!(format!("{:?}", d.validate()), *want, "row {i}");
        }
    }

    #[test]
    fn validation_does_not_clamp() {
        let mut d = base();
        node_mut(&mut d, 2, |n| n.base.opacity = 1.5);
        assert!(d.validate().is_err());
        let node = first(&mut d).nodes[&NodeId::new(2)].clone();
        assert_eq!(node.base.opacity.to_bits(), 1.5f32.to_bits());
    }

    #[test]
    fn extension_rows() {
        fn ext(d: &mut Document, type_id: &str, payload: Value) {
            node_mut(d, 2, |n| n.kind = extension(type_id, payload));
        }
        let deep =
            |depth: usize| format!("Err(ExtensionTooDeep {{ node: NodeId(2), depth: {depth} }})");
        let rows: [(Value, String); 8] = [
            (nested(64, false), "Ok(())".into()),
            (nested(65, false), deep(65)),
            (nested(65, true), deep(65)),
            (json!(7), "Ok(())".into()),
            (json!([]), "Ok(())".into()),
            (nested(1_000, false), deep(1_000)),
            (json!({"a": [1, {"b": null}]}), "Ok(())".into()),
            (json!([[[[0]]]]), "Ok(())".into()),
        ];
        for (i, (payload, want)) in rows.into_iter().enumerate() {
            let mut d = base();
            ext(&mut d, "x", payload);
            assert_eq!(format!("{:?}", d.validate()), want, "row {i}");
        }
        let mut d = base();
        ext(&mut d, "", json!(null));
        assert_eq!(
            format!("{:?}", d.validate()),
            "Err(EmptyExtensionTypeId { node: NodeId(2) })"
        );
    }

    #[test]
    fn child_rows() {
        let leaf = |kind: NodeKind| {
            move |d: &mut Document| {
                node_mut(d, 2, |n| {
                    n.kind = kind.clone();
                    n.children = vec![NodeId::new(3)];
                });
            }
        };
        let dangling = |d: &mut Document| {
            node_mut(d, 2, |n| n.children = vec![NodeId::new(3), NodeId::new(99)]);
        };
        let elsewhere = |d: &mut Document| {
            push(d, 7, &[8]);
            node_mut(d, 2, |n| n.children = vec![NodeId::new(8)]);
        };
        let shape = NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect));
        let rows: [(EditRef, &str); 5] = [
            (&leaf(shape), "Err(ChildrenOnLeaf { node: NodeId(2) })"),
            (
                &leaf(extension("x", json!(null))),
                "Err(ChildrenOnLeaf { node: NodeId(2) })",
            ),
            (
                &dangling,
                "Err(DanglingChild { parent: NodeId(2), child: NodeId(99) })",
            ),
            (
                &elsewhere,
                "Err(DanglingChild { parent: NodeId(2), child: NodeId(8) })",
            ),
            (
                &|d| node_mut(d, 2, |n| n.children = vec![NodeId::new(3)]),
                "Ok(())",
            ),
        ];
        for (i, (edit, want)) in rows.iter().enumerate() {
            let mut d = base();
            edit(&mut d);
            assert_eq!(format!("{:?}", d.validate()), *want, "row {i}");
        }
    }

    #[test]
    fn nodes_are_checked_in_ascending_id_and_check_order() {
        fn nan_x(n: &mut Node) {
            n.base.placement.x = f64::NAN;
        }
        let with_eight = |d: &mut Document| {
            first(d)
                .nodes
                .insert(NodeId::new(8), node(8, NodeKind::Group(GroupNode {})));
            first(d).roots.push(NodeId::new(8));
        };
        let too_deep = || extension("x", nested(65, false));
        let rows: [(EditRef, String); 7] = [
            (
                &|d| {
                    with_eight(d);
                    node_mut(d, 8, nan_x);
                    node_mut(d, 3, nan_x);
                },
                non_finite(3, "X"),
            ),
            (
                &|d| {
                    node_mut(d, 2, |n| n.children = vec![NodeId::new(99)]);
                    node_mut(d, 3, nan_x);
                },
                "Err(DanglingChild { parent: NodeId(2), child: NodeId(99) })".to_string(),
            ),
            (
                &|d| {
                    node_mut(d, 2, |n| {
                        n.base.opacity = 2.0;
                        nan_x(n);
                    });
                },
                non_finite(2, "X"),
            ),
            (
                &|d| {
                    node_mut(d, 2, |n| {
                        n.kind = extension("", json!(null));
                        n.base.opacity = 2.0;
                    });
                },
                range(2, "Opacity", 2.0),
            ),
            (
                &|d| {
                    node_mut(d, 2, |n| {
                        n.kind = extension("", nested(65, false));
                    });
                },
                "Err(EmptyExtensionTypeId { node: NodeId(2) })".to_string(),
            ),
            (
                &|d| {
                    node_mut(d, 2, |n| {
                        n.kind = too_deep();
                        n.children = vec![NodeId::new(3)];
                    });
                },
                "Err(ExtensionTooDeep { node: NodeId(2), depth: 65 })".to_string(),
            ),
            (
                &|d| {
                    node_mut(d, 2, |n| {
                        n.kind = NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect));
                        n.children = vec![NodeId::new(99)];
                    });
                },
                "Err(ChildrenOnLeaf { node: NodeId(2) })".to_string(),
            ),
        ];
        for (i, (edit, want)) in rows.iter().enumerate() {
            let mut d = base();
            edit(&mut d);
            assert_eq!(format!("{:?}", d.validate()), *want, "row {i}");
        }
    }

    #[test]
    fn a_self_parenting_group_still_passes_until_d5() {
        let mut d = base();
        node_mut(&mut d, 2, |n| n.children = vec![NodeId::new(2)]);
        assert_eq!(d.validate(), Ok(()));
    }
}
