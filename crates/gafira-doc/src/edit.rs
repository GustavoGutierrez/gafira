//! The edit capability: the only way to get `&mut` access inside a [`Document`].
//!
//! Reading goes through `&Document` or `Arc<Document>`; mutation needs an
//! [`EditToken`] and goes through [`DocMut`]. This is the interim ARQ-30
//! enforcement until `trybuild` arrives; the evidence is the CI step V13 and
//! review, not the `compile_fail` doctests, which are regression aids.

use std::sync::Arc;

use crate::{
    AssetEntry, AssetRef, AssetTable, Clip, ClipId, Document, Dpi, IdSpaceExhausted, MarkerId,
    Node, NodeId, Page, PageId, Sequence, SequenceId, Track, TrackId,
};

/// Proof that the holder may edit documents.
///
/// It has a private field and implements none of `Clone`, `Copy`, `Default`
/// or `Debug`, so it cannot be duplicated or built by a struct literal. One
/// token serves any number of edits. Only the editing crates may name it.
///
/// ```compile_fail
/// // Not `Clone` (E0599 expected; rustdoc does not verify the code).
/// use gafira_doc::EditToken;
/// # let token = EditToken::__new_for_gafira_edit_only();
/// let _copy = token.clone();
/// ```
///
/// ```compile_fail
/// // Not `Copy`: the second use is a use after move (E0382 expected).
/// use gafira_doc::EditToken;
/// # let token = EditToken::__new_for_gafira_edit_only();
/// fn take(_: EditToken) {}
/// take(token);
/// take(token);
/// ```
///
/// ```compile_fail
/// // No struct literal: the field is private (E0451 expected).
/// use gafira_doc::EditToken;
/// let _token = EditToken { _private: () };
/// ```
pub struct EditToken {
    _private: (),
}

impl EditToken {
    #[doc(hidden)]
    pub const fn __new_for_gafira_edit_only() -> EditToken {
        EditToken { _private: () }
    }
}

impl Document {
    /// Borrows the document mutably for editing; needs an [`EditToken`].
    ///
    /// This is the only way to get `&mut` access to anything inside a
    /// document. Replacing a whole value (`*doc = other`) by whoever owns a
    /// `Document`, or reaches one with `Arc::make_mut`, is possible and is not
    /// prevented; the session document is owned by `EditService` in C3.
    ///
    /// ```compile_fail
    /// // The field is private (E0616 expected).
    /// use gafira_doc::{Document, Dpi};
    /// let mut doc = Document::default();
    /// doc.dpi = Dpi::new(300);
    /// ```
    ///
    /// ```compile_fail
    /// // `pages_mut` lives on `DocMut`, not on `Document` (E0599 expected).
    /// use gafira_doc::Document;
    /// let mut doc = Document::default();
    /// doc.pages_mut();
    /// ```
    ///
    /// ```compile_fail
    /// // `edit` needs a token (E0061 expected).
    /// use gafira_doc::Document;
    /// let mut doc = Document::default();
    /// doc.edit();
    /// ```
    ///
    /// ```compile_fail
    /// // A shared `Arc<Document>` has no way to a mutable page.
    /// use std::sync::Arc;
    /// use gafira_doc::Document;
    /// let mut doc = Arc::new(Document::default());
    /// Arc::get_mut(&mut doc).unwrap().pages.push(todo!());
    /// ```
    ///
    /// ```compile_fail
    /// // `&Document` hands out `&Page`, never `&mut Page`.
    /// use gafira_doc::{Document, Page, PageId};
    /// let doc = Document::default();
    /// let _page: &mut Page = doc.page(PageId::new(1)).unwrap();
    /// ```
    pub fn edit(&mut self, token: &EditToken) -> DocMut<'_> {
        let _ = token;
        DocMut { doc: self }
    }
}

/// An exclusive editing view of a [`Document`].
///
/// Nothing is validated implicitly: after any sequence of operations the
/// document may be invalid, and [`Document::validate`] is the check.
pub struct DocMut<'a> {
    doc: &'a mut Document,
}

impl DocMut<'_> {
    /// Allocates a node id from the shared counter.
    pub fn alloc_node_id(&mut self) -> Result<NodeId, IdSpaceExhausted> {
        self.doc.alloc_id().map(NodeId::new)
    }

    /// Allocates a page id from the shared counter.
    pub fn alloc_page_id(&mut self) -> Result<PageId, IdSpaceExhausted> {
        self.doc.alloc_id().map(PageId::new)
    }

    /// Lowers the id counter back to `value`, a counter value read before the
    /// allocations being undone. A `value` at or above the counter is ignored,
    /// so the counter never moves up and ids stay unique in every state.
    pub fn restore_next_id(&mut self, value: u64) {
        self.doc.next_id = self.doc.next_id.min(value);
    }

    /// Raises the id counter to `value`, the counter of content being put
    /// back (a restored version, spec 031), so no id of that content is
    /// handed out again. A `value` at or below the counter is ignored.
    pub fn raise_next_id(&mut self, value: u64) {
        self.doc.next_id = self.doc.next_id.max(value);
    }

    /// Sets the connected data of bulk create (spec 104; not validated here).
    pub fn set_data(&mut self, data: Option<Arc<crate::DataSet>>) {
        self.doc.data = data;
    }

    /// Sets the document resolution (not validated here).
    pub fn set_dpi(&mut self, dpi: Dpi) {
        self.doc.dpi = dpi;
    }

    /// The page list, for adding, removing and reordering pages.
    pub fn pages_mut(&mut self) -> &mut Vec<Arc<Page>> {
        &mut self.doc.pages
    }

    /// The page with this id, copied on write along this path only.
    pub fn page_mut(&mut self, id: PageId) -> Option<&mut Page> {
        self.doc
            .pages
            .iter_mut()
            .find(|p| p.id == id)
            .map(Arc::make_mut)
    }

    /// The node with this id, copied on write along the page and node only.
    pub fn node_mut(&mut self, page: PageId, id: NodeId) -> Option<&mut Node> {
        self.page_mut(page)?.nodes.get_mut(&id).map(Arc::make_mut)
    }

    // ---- timeline of 02 §3.1 -------------------------------------------
    // The ids come from the same counter as pages and nodes (ARQ-10), and
    // every mutation below goes through `edit(&token)` (ARQ-30).

    /// Allocates a sequence id from the shared counter.
    pub fn alloc_sequence_id(&mut self) -> Result<SequenceId, IdSpaceExhausted> {
        self.doc.alloc_id().map(SequenceId::new)
    }

    /// Allocates a track id from the shared counter.
    pub fn alloc_track_id(&mut self) -> Result<TrackId, IdSpaceExhausted> {
        self.doc.alloc_id().map(TrackId::new)
    }

    /// Allocates a clip id from the shared counter.
    pub fn alloc_clip_id(&mut self) -> Result<ClipId, IdSpaceExhausted> {
        self.doc.alloc_id().map(ClipId::new)
    }

    /// Allocates a marker id from the shared counter.
    pub fn alloc_marker_id(&mut self) -> Result<MarkerId, IdSpaceExhausted> {
        self.doc.alloc_id().map(MarkerId::new)
    }

    /// The sequence list, for adding or removing whole sequences.
    pub fn sequences_mut(&mut self) -> &mut Vec<Arc<Sequence>> {
        &mut self.doc.sequences
    }

    /// The sequence with this id, copied on write along this path only.
    pub fn sequence_mut(&mut self, id: SequenceId) -> Option<&mut Sequence> {
        self.doc
            .sequences
            .iter_mut()
            .find(|sequence| sequence.id == id)
            .map(Arc::make_mut)
    }

    /// The track of one sequence with this id, copied on write along this
    /// path only.
    pub fn track_mut(&mut self, sequence: SequenceId, id: TrackId) -> Option<&mut Track> {
        self.sequence_mut(sequence)?.track_mut_by_id(id)
    }

    /// The clip of one track with this id, copied on write along the path only.
    pub fn clip_mut(
        &mut self,
        sequence: SequenceId,
        track: TrackId,
        id: ClipId,
    ) -> Option<&mut Clip> {
        self.track_mut(sequence, track)?.clip_mut_by_id(id)
    }

    /// The asset table the clips name their media with, for adding, updating
    /// or removing entries.
    pub fn assets_mut(&mut self) -> &mut AssetTable {
        &mut self.doc.assets
    }

    /// The entry of the asset table this reference names, copied on write.
    pub fn asset_mut(&mut self, reference: AssetRef) -> Option<&mut AssetEntry> {
        self.doc.assets.get_mut(&reference)
    }

    /// Read access to the document being edited.
    pub fn as_doc(&self) -> &Document {
        self.doc
    }
}
