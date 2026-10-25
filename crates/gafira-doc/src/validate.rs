//! `Document::validate`: the structural rules, one pass at a time.

use std::collections::BTreeSet;

use crate::{Document, ValidationError};

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
        self.check_pages()
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
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gafira_geom::{Length, Unit};
    use serde_json::json;

    use crate::ValidationError;
    use crate::document::test_document;
    use crate::{
        Axis, Document, Dpi, ElementBase, ExtensionNode, GroupNode, Node, NodeId, NodeKind, Page,
        PageId, PageSize, PageSizeError, Placement, ShapeGeometry, ShapeNode,
    };

    type Edit = fn(&mut Document);

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
}
