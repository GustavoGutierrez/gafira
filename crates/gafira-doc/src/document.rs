//! The document root.

use std::sync::Arc;

#[cfg(test)]
use crate::IdSpaceExhausted;
use crate::{Dpi, Page, PageId};

/// A whole document: a version, one id counter, a DPI and ordered pages.
///
/// Fields are private; pages are shared by `Arc` so cloning copies no page
/// or node. Reading never hands out `&mut` access.
#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    schema_version: u32,
    next_id: u64,
    dpi: Dpi,
    pages: Vec<Arc<Page>>,
}

impl Document {
    /// The schema version this build writes and reads.
    pub const SCHEMA_VERSION: u32 = 1;

    /// The document schema version.
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// The next id the shared counter will hand out (nodes and pages).
    pub fn next_id(&self) -> u64 {
        self.next_id
    }

    /// The document resolution.
    pub fn dpi(&self) -> Dpi {
        self.dpi
    }

    /// The pages, in page order.
    pub fn pages(&self) -> &[Arc<Page>] {
        &self.pages
    }

    /// The page with this id, if any.
    pub fn page(&self, id: PageId) -> Option<&Page> {
        self.pages.iter().map(Arc::as_ref).find(|p| p.id == id)
    }

    /// Returns the current counter value and advances it; fails at `u64::MAX`
    /// leaving the counter unchanged.
    #[cfg(test)]
    pub(crate) fn alloc_id(&mut self) -> Result<u64, IdSpaceExhausted> {
        let id = self.next_id;
        self.next_id = id.checked_add(1).ok_or(IdSpaceExhausted)?;
        Ok(id)
    }
}

impl Default for Document {
    fn default() -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            next_id: 1,
            dpi: Dpi::default(),
            pages: Vec::new(),
        }
    }
}

/// Builds a document from parts without validating it (tests only).
#[cfg(test)]
pub(crate) fn test_document(next_id: u64, dpi: Dpi, pages: Vec<Arc<Page>>) -> Document {
    Document {
        schema_version: Document::SCHEMA_VERSION,
        next_id,
        dpi,
        pages,
    }
}

#[cfg(test)]
mod tests {
    use gafira_geom::{Length, Unit};

    use super::*;
    use crate::{ElementBase, GroupNode, Node, NodeId, NodeKind, Placement};

    fn page(id: u64, node_ids: &[u64]) -> Arc<Page> {
        let mut p = Page::new(
            PageId::new(id),
            String::new(),
            crate::PageSize {
                width: Length::new(100.0, Unit::Px),
                height: Length::new(100.0, Unit::Px),
            },
        );
        for &n in node_ids {
            let node = Node::new(
                ElementBase::new(NodeId::new(n), Placement::new(0.0, 0.0, 1.0, 1.0)),
                NodeKind::Group(GroupNode {}),
            );
            p.nodes.insert(NodeId::new(n), Arc::new(node));
        }
        Arc::new(p)
    }

    #[test]
    fn default_document() {
        let d = Document::default();
        assert_eq!(d.schema_version(), 1);
        assert_eq!(d.next_id(), 1);
        assert_eq!(d.dpi().get(), 96);
        assert!(d.pages().is_empty());
        assert!(d.page(PageId::new(1)).is_none());
    }

    #[test]
    fn allocator_increments_then_exhausts() {
        let mut d = test_document(5, Dpi::default(), Vec::new());
        assert_eq!(d.alloc_id(), Ok(5));
        assert_eq!(d.next_id(), 6);
        let mut full = test_document(u64::MAX, Dpi::default(), Vec::new());
        assert_eq!(full.alloc_id(), Err(IdSpaceExhausted));
        assert_eq!(full.next_id(), u64::MAX);
    }

    #[test]
    fn page_lookup_and_order() {
        let d = test_document(
            10,
            Dpi::default(),
            vec![page(3, &[]), page(1, &[]), page(2, &[])],
        );
        let order: Vec<u64> = d.pages().iter().map(|p| p.id.get()).collect();
        assert_eq!(order, [3, 1, 2]);
        assert_eq!(d.page(PageId::new(1)).unwrap().id, PageId::new(1));
        assert!(d.page(PageId::new(9)).is_none());
    }

    #[test]
    fn clone_shares_everything() {
        let d = test_document(10, Dpi::default(), vec![page(1, &[4, 5, 6]), page(2, &[])]);
        let c = d.clone();
        assert_eq!(d.pages().len(), 2);
        for (a, b) in d.pages().iter().zip(c.pages()) {
            assert!(Arc::ptr_eq(a, b));
        }
        let (first, first_clone) = (&d.pages()[0], &c.pages()[0]);
        assert_eq!(first.nodes.len(), 3);
        for (id, n) in &first.nodes {
            assert!(Arc::ptr_eq(n, &first_clone.nodes[id]));
        }
    }
}
