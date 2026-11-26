//! The document root.

use std::sync::Arc;

use std::collections::BTreeMap;

use crate::timeline::{AssetEntry, AssetRef, AssetTable, Sequence};
use crate::{Dpi, IdSpaceExhausted, Page, PageId, ProjectId};

/// A whole document: a version, one id counter, a DPI, ordered pages, the
/// timelines and the asset table their clips name (02 §3.1).
///
/// Fields are private; pages are shared by `Arc` so cloning copies no page
/// or node. Reading never hands out `&mut` access; mutation needs an
/// `EditToken` and goes through `Document::edit`. Replacing a whole value
/// (`*doc = other`) is possible for whoever owns the document and is owned by
/// `EditService` in C3; it is documented, not prevented.
#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub(crate) schema_version: u32,
    pub(crate) next_id: u64,
    /// The project's identity (D-63); nil until the engine assigns one.
    pub(crate) project_id: ProjectId,
    pub(crate) dpi: Dpi,
    pub(crate) pages: Vec<Arc<Page>>,
    /// The timelines of the document, in creation order (02 §3.1).
    pub(crate) sequences: Vec<Arc<Sequence>>,
    /// The asset table the clips need (P9), keyed by the content hash.
    pub(crate) assets: AssetTable,
    /// The connected data of bulk create (spec 104), if any.
    pub(crate) data: Option<Arc<crate::DataSet>>,
}

impl Document {
    /// The sequences of the document, in the order they were created.
    ///
    /// Timeline data sits on the document root until the composition root
    /// (`Project`, 02 §3.1) exists, and the wire form carries it beside the
    /// pages.
    #[must_use]
    pub fn sequences(&self) -> &[Arc<Sequence>] {
        &self.sequences
    }

    /// The sequence with this id.
    #[must_use]
    pub fn sequence(&self, id: crate::SequenceId) -> Option<&Sequence> {
        self.sequences
            .iter()
            .find(|s| s.id == id)
            .map(|sequence| sequence.as_ref())
    }

    /// The asset table the clips name their media with (P9).
    #[must_use]
    pub fn assets(&self) -> &AssetTable {
        &self.assets
    }

    /// The entry of the asset table the reference names.
    #[must_use]
    pub fn asset(&self, reference: &AssetRef) -> Option<&AssetEntry> {
        self.assets.get(reference)
    }

    /// The connected data of bulk create (spec 104), if any.
    #[must_use]
    pub fn data(&self) -> Option<&crate::DataSet> {
        self.data.as_deref()
    }

    /// The connected data, shared.
    #[must_use]
    pub fn data_arc(&self) -> Option<&Arc<crate::DataSet>> {
        self.data.as_ref()
    }

    /// The same document with `data` as its connected data, validated.
    ///
    /// # Errors
    /// The first rule the document breaks with it.
    pub fn with_data(
        mut self,
        data: Option<Arc<crate::DataSet>>,
    ) -> Result<Self, crate::ValidationError> {
        self.data = data;
        self.validate()?;
        Ok(self)
    }

    /// The schema version this build writes and reads.
    pub const SCHEMA_VERSION: u32 = 1;

    /// The document schema version.
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// The next id the shared counter will hand out (pages, nodes and the
    /// timeline's sequences, tracks, clips and markers).
    pub fn next_id(&self) -> u64 {
        self.next_id
    }

    /// The document resolution.
    pub fn dpi(&self) -> Dpi {
        self.dpi
    }

    /// The project's identity (D-63); [`ProjectId::NIL`] until one is set.
    #[must_use]
    pub fn project_id(&self) -> ProjectId {
        self.project_id
    }

    /// The same document under the identity `id` (D-63): the engine sets it
    /// on a new project, on a file written before the field, and on the
    /// document «Guardar copia» writes. No edit changes it.
    #[must_use]
    pub fn with_project_id(mut self, id: ProjectId) -> Self {
        self.project_id = id;
        self
    }

    /// The pages, in page order.
    pub fn pages(&self) -> &[Arc<Page>] {
        &self.pages
    }

    /// The page with this id, if any.
    pub fn page(&self, id: PageId) -> Option<&Page> {
        self.pages.iter().map(Arc::as_ref).find(|p| p.id == id)
    }

    /// Builds a document from parts and validates it.
    pub fn from_parts(
        next_id: u64,
        dpi: Dpi,
        pages: Vec<Arc<Page>>,
    ) -> Result<Self, crate::ValidationError> {
        let doc = Self {
            schema_version: Self::SCHEMA_VERSION,
            next_id,
            project_id: ProjectId::NIL,
            dpi,
            pages,
            sequences: Vec::new(),
            assets: BTreeMap::new(),
            data: None,
        };
        doc.validate()?;
        Ok(doc)
    }

    /// [`Document::from_parts`] with an asset table, validated with it: a
    /// command checks new image elements against the table they will live
    /// with (spec 029).
    pub fn from_parts_with_assets(
        next_id: u64,
        dpi: Dpi,
        pages: Vec<Arc<Page>>,
        assets: AssetTable,
    ) -> Result<Self, crate::ValidationError> {
        let doc = Self {
            schema_version: Self::SCHEMA_VERSION,
            next_id,
            project_id: ProjectId::NIL,
            dpi,
            pages,
            sequences: Vec::new(),
            assets,
            data: None,
        };
        doc.validate()?;
        Ok(doc)
    }

    /// Returns the current counter value and advances it; fails at `u64::MAX`
    /// leaving the counter unchanged.
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
            project_id: ProjectId::NIL,
            dpi: Dpi::default(),
            pages: Vec::new(),
            sequences: Vec::new(),
            assets: BTreeMap::new(),
            data: None,
        }
    }
}

/// Builds a document from parts without validating it (tests only).
#[cfg(test)]
pub(crate) fn test_document(next_id: u64, dpi: Dpi, pages: Vec<Arc<Page>>) -> Document {
    Document {
        schema_version: Document::SCHEMA_VERSION,
        next_id,
        project_id: ProjectId::NIL,
        dpi,
        pages,
        sequences: Vec::new(),
        assets: BTreeMap::new(),
        data: None,
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
    fn document_is_clone_debug_and_partial_eq() {
        fn assert_traits<T: Clone + std::fmt::Debug + PartialEq>() {}
        assert_traits::<Document>();
        let d = test_document(3, Dpi::new(300), vec![page(1, &[2])]);
        assert_eq!(d, d.clone());
        assert_ne!(d, Document::default());
        assert!(format!("{d:?}").starts_with("Document"));
    }

    #[test]
    fn dpi_is_read_back_from_the_document() {
        let d = test_document(1, Dpi::new(300), Vec::new());
        assert_eq!(d.dpi().get(), 300);
        assert_eq!(d.dpi(), Dpi::new(300));
        assert_ne!(d.dpi(), Dpi::default());
    }

    #[test]
    fn from_parts_keeps_page_order_and_looks_pages_up() {
        let d = Document::from_parts(
            10,
            Dpi::default(),
            vec![page(3, &[]), page(1, &[]), page(2, &[])],
        )
        .unwrap();
        let order: Vec<u64> = d.pages().iter().map(|p| p.id.get()).collect();
        assert_eq!(order, [3, 1, 2]);
        assert_eq!(d.page(PageId::new(1)).unwrap().id, PageId::new(1));
        assert!(d.page(PageId::new(9)).is_none());
    }

    #[test]
    fn from_parts_validates() {
        let err = Document::from_parts(1, Dpi::default(), vec![page(5, &[])]).unwrap_err();
        assert_eq!(
            err,
            crate::ValidationError::NextIdNotAboveMax {
                next_id: 1,
                max_id: 5
            }
        );
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

    #[test]
    fn equality_compares_dpi_and_page_contents_not_pointers() {
        let a = test_document(3, Dpi::new(96), vec![page(1, &[2])]);
        let rebuilt = test_document(3, Dpi::new(96), vec![page(1, &[2])]);
        assert!(!Arc::ptr_eq(&a.pages()[0], &rebuilt.pages()[0]));
        assert_eq!(a, rebuilt);
        assert_ne!(a, test_document(3, Dpi::new(300), vec![page(1, &[2])]));
        assert_ne!(a, test_document(3, Dpi::new(96), vec![page(1, &[4])]));
        assert_ne!(a, test_document(4, Dpi::new(96), vec![page(1, &[2])]));
    }
}
