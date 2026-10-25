//! The edit capability: the only way to get `&mut` access inside a [`Document`].
//!
//! Reading goes through `&Document` or `Arc<Document>`; mutation needs an
//! [`EditToken`] and goes through [`DocMut`]. This is the interim ARQ-30
//! enforcement until `trybuild` arrives; the evidence is the CI step V13 and
//! review, not the `compile_fail` doctests, which are regression aids.

use std::sync::Arc;

use crate::{Document, Dpi, IdSpaceExhausted, Node, NodeId, Page, PageId};

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

    /// Read access to the document being edited.
    pub fn as_doc(&self) -> &Document {
        self.doc
    }
}
