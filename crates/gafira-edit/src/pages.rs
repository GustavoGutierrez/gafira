//! `InsertPage` and `RemovePage`: pages in and out of the page list (Q4);
//! `ReorderPage` and `DuplicatePage` (spec 056, FD-02).

use std::sync::Arc;

use gafira_doc::msg::{CommandLabel, MessageKey};
use gafira_doc::{DocMut, Page, PageId, PageSize, ValidationError};

use crate::{Applied, Command, CommandError, DirtySet};

/// What one successful `InsertPage::apply` did.
#[derive(Clone, Debug)]
struct Inserted {
    /// The id counter before the page id was allocated.
    counter: u64,
    page: Arc<Page>,
}

/// Inserts one empty page.
///
/// The id is allocated by `apply` and given back by `revert`, so a redo gets
/// the same id and rebuilds an identical document.
#[derive(Clone, Debug)]
pub struct InsertPage {
    index: usize,
    name: String,
    size: PageSize,
    record: Option<Inserted>,
}

impl InsertPage {
    /// Inserts an empty page called `name` of `size` at `index` of the page
    /// list (`0..=len`); its id comes from the document counter.
    pub fn new(index: usize, name: String, size: PageSize) -> Self {
        Self {
            index,
            name,
            size,
            record: None,
        }
    }
}

impl Command for InsertPage {
    fn label(&self) -> CommandLabel {
        MessageKey("edit.insert_page")
    }

    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        let view = doc.as_doc();
        let len = view.pages().len();
        if self.index > len {
            return Err(CommandError::IndexOutOfRange {
                index: self.index,
                len,
            });
        }
        let counter = view.next_id();
        self.size.check(view.dpi()).map_err(|error| {
            let page = PageId::new(counter);
            CommandError::InvalidDocument(ValidationError::PageSize { page, error })
        })?;
        let id = doc
            .alloc_page_id()
            .map_err(|_| CommandError::IdSpaceExhausted)?;
        let page = Arc::new(Page::new(id, self.name.clone(), self.size));
        doc.pages_mut().insert(self.index, Arc::clone(&page));
        self.record = Some(Inserted { counter, page });
        Ok(Applied::Changed)
    }

    fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError> {
        let record = self.record.as_ref().ok_or(CommandError::NotApplied)?;
        let id = record.page.id;
        let at = doc
            .as_doc()
            .pages()
            .iter()
            .position(|p| p.id == id)
            .ok_or(CommandError::MissingPage(id))?;
        if *doc.as_doc().pages()[at] != *record.page {
            return Err(CommandError::InvalidValue("the inserted page was changed"));
        }
        // Restoring the counter must not hand out an id a later command took.
        if doc.as_doc().next_id().checked_sub(1) != Some(record.counter) {
            return Err(CommandError::InvalidValue(
                "ids were allocated after the page was inserted",
            ));
        }
        doc.pages_mut().remove(at);
        doc.restore_next_id(record.counter);
        Ok(())
    }

    fn touches(&self) -> DirtySet {
        let mut dirty = DirtySet::default();
        if let Some(record) = &self.record {
            dirty.pages.insert(record.page.id);
        }
        dirty
    }
}

/// Removes one page with its whole arena.
///
/// `revert` puts the very same page back at the index it had.
#[derive(Clone, Debug)]
pub struct RemovePage {
    page: PageId,
    record: Option<(usize, Arc<Page>)>,
}

impl RemovePage {
    /// Removes `page`; `revert` puts it back at the same index.
    pub fn new(page: PageId) -> Self {
        Self { page, record: None }
    }
}

impl Command for RemovePage {
    fn label(&self) -> CommandLabel {
        MessageKey("edit.remove_page")
    }

    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        let at = doc
            .as_doc()
            .pages()
            .iter()
            .position(|p| p.id == self.page)
            .ok_or(CommandError::MissingPage(self.page))?;
        let removed = doc.pages_mut().remove(at);
        self.record = Some((at, removed));
        Ok(Applied::Changed)
    }

    fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError> {
        let (index, page) = self.record.as_ref().ok_or(CommandError::NotApplied)?;
        let pages = doc.as_doc().pages();
        if pages.iter().any(|p| p.id == page.id) {
            return Err(CommandError::InvalidValue(
                "the removed page is already back",
            ));
        }
        if *index > pages.len() {
            return Err(CommandError::IndexOutOfRange {
                index: *index,
                len: pages.len(),
            });
        }
        doc.pages_mut().insert(*index, Arc::clone(page));
        Ok(())
    }

    fn touches(&self) -> DirtySet {
        let mut dirty = DirtySet::default();
        dirty.pages.insert(self.page);
        if let Some((_, page)) = &self.record {
            dirty.nodes.extend(page.nodes.keys().copied());
        }
        dirty
    }
}

/// Moves one page to another index of the page list (FD-02).
#[derive(Clone, Debug)]
pub struct ReorderPage {
    page: PageId,
    index: usize,
    record: Option<usize>,
}

impl ReorderPage {
    /// Moves `page` so it ends at `index` (`0..len`).
    pub fn new(page: PageId, index: usize) -> Self {
        Self {
            page,
            index,
            record: None,
        }
    }
}

impl Command for ReorderPage {
    fn label(&self) -> CommandLabel {
        MessageKey("edit.reorder_page")
    }

    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        let pages = doc.as_doc().pages();
        let len = pages.len();
        let from = pages
            .iter()
            .position(|p| p.id == self.page)
            .ok_or(CommandError::MissingPage(self.page))?;
        if self.index >= len {
            return Err(CommandError::IndexOutOfRange {
                index: self.index,
                len,
            });
        }
        self.record = Some(from);
        if from == self.index {
            return Ok(Applied::Unchanged);
        }
        let page = doc.pages_mut().remove(from);
        doc.pages_mut().insert(self.index, page);
        Ok(Applied::Changed)
    }

    fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError> {
        let from = self.record.ok_or(CommandError::NotApplied)?;
        let at = doc
            .as_doc()
            .pages()
            .iter()
            .position(|p| p.id == self.page)
            .ok_or(CommandError::MissingPage(self.page))?;
        let page = doc.pages_mut().remove(at);
        doc.pages_mut().insert(from, page);
        Ok(())
    }

    fn touches(&self) -> DirtySet {
        let mut dirty = DirtySet::default();
        dirty.pages.insert(self.page);
        dirty
    }
}

/// Puts a copy of one page right after it, with new ids for the page and
/// every node (FD-02, ARQ-13).
///
/// The ids are allocated by `apply` and given back by `revert`, so a redo
/// gets the same ids and rebuilds an identical document.
#[derive(Clone, Debug)]
pub struct DuplicatePage {
    page: PageId,
    record: Option<Inserted>,
}

impl DuplicatePage {
    /// Copies `page` to the index after it.
    pub fn new(page: PageId) -> Self {
        Self { page, record: None }
    }

    /// The id the copy got, once applied.
    pub fn copy(&self) -> Option<PageId> {
        self.record.as_ref().map(|record| record.page.id)
    }
}

impl Command for DuplicatePage {
    fn label(&self) -> CommandLabel {
        MessageKey("edit.duplicate_page")
    }

    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        let view = doc.as_doc();
        let at = view
            .pages()
            .iter()
            .position(|p| p.id == self.page)
            .ok_or(CommandError::MissingPage(self.page))?;
        let source = Arc::clone(&view.pages()[at]);
        let counter = view.next_id();
        let id = doc
            .alloc_page_id()
            .map_err(|_| CommandError::IdSpaceExhausted)?;
        // Every node gets a new id, in the order of the old ones, so a redo
        // gives the same ids.
        let mut ids = std::collections::BTreeMap::new();
        for old in source.nodes.keys() {
            let new = doc
                .alloc_node_id()
                .map_err(|_| CommandError::IdSpaceExhausted)?;
            ids.insert(*old, new);
        }
        let map = |old: &gafira_doc::NodeId| ids.get(old).copied().unwrap_or(*old);
        let mut copy = (*source).clone();
        copy.id = id;
        copy.roots = source.roots.iter().map(map).collect();
        copy.nodes = source
            .nodes
            .iter()
            .map(|(old, node)| {
                let mut node = (**node).clone();
                node.base.id = map(old);
                node.children = node.children.iter().map(map).collect();
                (map(old), Arc::new(node))
            })
            .collect();
        let page = Arc::new(copy);
        doc.pages_mut().insert(at + 1, Arc::clone(&page));
        self.record = Some(Inserted { counter, page });
        Ok(Applied::Changed)
    }

    fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError> {
        let record = self.record.as_ref().ok_or(CommandError::NotApplied)?;
        let id = record.page.id;
        let at = doc
            .as_doc()
            .pages()
            .iter()
            .position(|p| p.id == id)
            .ok_or(CommandError::MissingPage(id))?;
        if *doc.as_doc().pages()[at] != *record.page {
            return Err(CommandError::InvalidValue("the copied page was changed"));
        }
        // Restoring the counter must not hand out an id a later command took.
        let taken = 1 + record.page.nodes.len() as u64;
        if doc.as_doc().next_id().checked_sub(taken) != Some(record.counter) {
            return Err(CommandError::InvalidValue(
                "ids were allocated after the page was copied",
            ));
        }
        doc.pages_mut().remove(at);
        doc.restore_next_id(record.counter);
        Ok(())
    }

    fn touches(&self) -> DirtySet {
        let mut dirty = DirtySet::default();
        dirty.pages.insert(self.page);
        if let Some(record) = &self.record {
            dirty.pages.insert(record.page.id);
            dirty.nodes.extend(record.page.nodes.keys().copied());
        }
        dirty
    }
}
