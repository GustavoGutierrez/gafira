//! `Document::validate`: the structural rules, one pass at a time.

use std::collections::BTreeSet;

use crate::{Document, ValidationError};

impl Document {
    /// Returns the first rule the document breaks, or `Ok(())`.
    ///
    /// Passes run in a fixed order and, inside a pass, checks run in the
    /// order of the [`ValidationError`] variants (pages in `pages` order,
    /// nodes in ascending id). Nothing is altered or clamped.
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

    use crate::ValidationError::{self, *};
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

    fn roots(d: &mut Document, ids: &[u64]) {
        first(d).roots = ids.iter().map(|&i| NodeId::new(i)).collect();
    }

    /// Applies each edit to a valid document and compares the `Debug` form of the result.
    fn run(rows: &[(Edit, &str)]) {
        for (i, (edit, want)) in rows.iter().enumerate() {
            let mut d = base();
            edit(&mut d);
            assert_eq!(format!("{:?}", d.validate()), *want, "row {i}");
        }
    }

    fn out(axis: &str, px: f64) -> String {
        let err = format!("OutOfRange {{ axis: {axis}, px: {px:?} }}");
        format!("Err(PageSize {{ page: PageId(1), error: {err} }})")
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
        fn rekey(d: &mut Document) {
            let n = first(d).nodes.remove(&NodeId::new(3)).unwrap();
            first(d).nodes.insert(NodeId::new(5), n);
        }
        run(&[
            (|d| push(d, 1, &[]), "Err(DuplicateId { id: 1 })"),
            (|d| push(d, 7, &[3]), "Err(DuplicateId { id: 3 })"),
            (|d| push(d, 2, &[]), "Err(DuplicateId { id: 2 })"),
            (
                rekey,
                "Err(ArenaKeyMismatch { page: PageId(1), key: NodeId(5), id: NodeId(3) })",
            ),
            (|d| d.next_id = 4, "Ok(())"),
            (
                |d| d.next_id = 3,
                "Err(NextIdNotAboveMax { next_id: 3, max_id: 3 })",
            ),
            (
                |d| d.next_id = 0,
                "Err(NextIdNotAboveMax { next_id: 0, max_id: 3 })",
            ),
            (|d| d.pages.clear(), "Ok(())"),
        ]);
        let mut d = base();
        push(&mut d, u64::MAX, &[]);
        d.next_id = u64::MAX;
        let (next_id, max_id) = (u64::MAX, u64::MAX);
        assert_eq!(d.validate(), Err(NextIdNotAboveMax { next_id, max_id }));
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
                &out("Width", low),
            ),
            (
                |d| first(d).size = px(5.0, 16_384.0f64.next_up()),
                &out("Height", high),
            ),
            (|d| first(d).size = px(0.0, 5.0), &out("Width", 0.0)),
            (|d| first(d).size = px(5.0, -5.0), &out("Height", -5.0)),
            (|d| a0(d, 300), "Ok(())"),
            (|d| a0(d, 350), "Ok(())"),
            (|d| a0(d, 351), &out("Height", a0_px)),
        ]);
        let mut d = base();
        first(&mut d).size = px(f64::NAN, 5.0);
        let Err(ValidationError::PageSize {
            page,
            error: PageSizeError::Conversion { axis, .. },
        }) = d.validate()
        else {
            panic!("expected a conversion error");
        };
        assert_eq!((page, axis), (PageId::new(1), Axis::Width));
    }

    #[test]
    fn root_rows() {
        fn elsewhere(d: &mut Document) {
            push(d, 7, &[9]);
            roots(d, &[2, 9]);
        }
        run(&[
            (
                |d| roots(d, &[42]),
                "Err(DanglingRoot { page: PageId(1), node: NodeId(42) })",
            ),
            (
                |d| roots(d, &[3, 3]),
                "Err(DuplicateRoot { page: PageId(1), node: NodeId(3) })",
            ),
            (
                elsewhere,
                "Err(DanglingRoot { page: PageId(1), node: NodeId(9) })",
            ),
        ]);
    }
}
