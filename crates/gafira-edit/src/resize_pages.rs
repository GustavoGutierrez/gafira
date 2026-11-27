//! `ResizePages`: pages of the document replaced by resized ones of the
//! same id, their content already fitted (spec 108, «Redimensionar el
//! diseño»). One undo step; the node ids stay.

use std::sync::Arc;

use gafira_doc::msg::{CommandLabel, MessageKey};
use gafira_doc::{DocMut, Page};

use crate::{Applied, Command, CommandError, DirtySet};

/// Replaces each page of the document whose id is in `pages` with that
/// page: its new size and its content fitted to it.
#[derive(Clone, Debug)]
pub struct ResizePages {
    pages: Vec<Page>,
    /// The pages `apply` replaced, with their index, for `revert`.
    old: Vec<(usize, Arc<Page>)>,
}

impl ResizePages {
    /// Puts `pages` in place of the pages with their ids.
    pub fn new(pages: Vec<Page>) -> Self {
        Self {
            pages,
            old: Vec::new(),
        }
    }
}

impl Command for ResizePages {
    fn label(&self) -> CommandLabel {
        MessageKey("edit.resize_design")
    }

    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        if self.pages.is_empty() {
            return Err(CommandError::InvalidValue("pages to resize"));
        }
        let view = doc.as_doc();
        let dpi = view.dpi();
        let mut places = Vec::with_capacity(self.pages.len());
        for page in &self.pages {
            let at = view
                .pages()
                .iter()
                .position(|p| p.id == page.id)
                .ok_or(CommandError::MissingPage(page.id))?;
            page.size
                .check(dpi)
                .map_err(|_| CommandError::InvalidValue("page size"))?;
            page.check_layout(dpi)
                .map_err(|_| CommandError::InvalidValue("page layout"))?;
            places.push(at);
        }
        let old: Vec<(usize, Arc<Page>)> = places
            .iter()
            .map(|&at| (at, Arc::clone(&view.pages()[at])))
            .collect();
        for (page, &at) in self.pages.iter().zip(&places) {
            doc.pages_mut()[at] = Arc::new(page.clone());
        }
        self.old = old;
        Ok(Applied::Changed)
    }

    fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError> {
        if self.old.is_empty() {
            return Err(CommandError::NotApplied);
        }
        for (at, page) in std::mem::take(&mut self.old).into_iter().rev() {
            doc.pages_mut()[at] = page;
        }
        Ok(())
    }

    /// Every page it replaced and every node of them, before and after.
    fn touches(&self) -> DirtySet {
        DirtySet {
            pages: self.pages.iter().map(|page| page.id).collect(),
            nodes: self
                .pages
                .iter()
                .flat_map(|page| page.nodes.keys().copied())
                .chain(self.old.iter().flat_map(|(_, p)| p.nodes.keys().copied()))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use gafira_doc::{
        Document, Dpi, ElementBase, GroupNode, Node, NodeId, NodeKind, PageId, PageSize, Placement,
    };
    use gafira_geom::{Length, Unit};

    use super::*;
    use crate::History;

    fn size(w: f64, h: f64) -> PageSize {
        PageSize {
            width: Length::new(w, Unit::Px),
            height: Length::new(h, Unit::Px),
        }
    }

    /// A page `id` of 100 × 100 px holding one group `node` at 10, 10.
    fn page(id: u64, node: u64) -> Page {
        let mut page = Page::new(PageId::new(id), format!("p{id}"), size(100.0, 100.0));
        let group = Node::new(
            ElementBase::new(NodeId::new(node), Placement::new(10.0, 10.0, 20.0, 20.0)),
            NodeKind::Group(GroupNode {}),
        );
        page.nodes.insert(NodeId::new(node), Arc::new(group));
        page.roots.push(NodeId::new(node));
        page
    }

    fn document() -> Document {
        Document::from_parts(
            10,
            Dpi::new(96),
            vec![Arc::new(page(1, 2)), Arc::new(page(3, 4))],
        )
        .expect("a valid document")
    }

    /// Page `id` of [`document`] at 200 × 100, its group moved and grown.
    fn resized(id: u64, node: u64) -> Page {
        let mut page = page(id, node);
        page.size = size(200.0, 100.0);
        let group = Arc::make_mut(page.nodes.get_mut(&NodeId::new(node)).expect("group"));
        group.base.placement = Placement::new(60.0, 10.0, 20.0, 20.0);
        page
    }

    // S108-05: the pages are replaced by id, in place, the others and the
    // node ids kept; undo gives the exact document back, redo again.
    #[test]
    fn s108_05_pages_are_resized_in_one_undo_step() {
        let before = document();
        let mut history = History::new(before.clone()).expect("valid");
        let applied = history
            .execute(
                Box::new(ResizePages::new(vec![resized(3, 4)])),
                Default::default(),
            )
            .expect("applies");
        assert_eq!(applied, Applied::Changed);
        let after = history.document().clone();
        assert_eq!(after.pages().len(), 2);
        assert_eq!(*after.pages()[0], *before.pages()[0], "page 1 untouched");
        assert_eq!(*after.pages()[1], resized(3, 4));
        assert_eq!(after.next_id(), before.next_id());
        history.undo().expect("undo");
        assert_eq!(history.document().pages(), before.pages());
        history.redo().expect("redo");
        assert_eq!(history.document().pages(), after.pages());
    }

    // S108-05: every page of the list in one step.
    #[test]
    fn s108_05_every_page_at_once() {
        let mut history = History::new(document()).expect("valid");
        history
            .execute(
                Box::new(ResizePages::new(vec![resized(1, 2), resized(3, 4)])),
                Default::default(),
            )
            .expect("applies");
        assert!(
            history
                .document()
                .pages()
                .iter()
                .all(|p| p.size == size(200.0, 100.0))
        );
        history.undo().expect("undo");
        assert_eq!(history.document().pages(), document().pages());
    }

    // S108-05: a page the document lacks or a size out of the limits is
    // refused and nothing changes.
    #[test]
    fn s108_05_a_missing_page_or_a_bad_size_is_refused() {
        let mut history = History::new(document()).expect("valid");
        let missing = history.execute(
            Box::new(ResizePages::new(vec![resized(1, 2), resized(9, 4)])),
            Default::default(),
        );
        assert_eq!(missing, Err(CommandError::MissingPage(PageId::new(9))));
        let mut tiny = resized(1, 2);
        tiny.size = size(0.0, 100.0);
        let bad = history.execute(Box::new(ResizePages::new(vec![tiny])), Default::default());
        assert!(bad.is_err());
        assert_eq!(history.document().pages(), document().pages());
        let empty = history.execute(Box::new(ResizePages::new(Vec::new())), Default::default());
        assert!(empty.is_err());
    }
}
