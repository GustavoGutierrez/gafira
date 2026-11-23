//! The selection of the editor (03 §9.1, FD-10; spec 040): elements of one
//! page, in the order they were added. The last one added is the primary:
//! the Inspector shows its values and a single selection is just a primary
//! alone. Pure, so it is tested without a window.

use gafira_doc::{NodeId, PageId};

/// The selected elements: nodes of one page, without repeats, oldest
/// first. The page is `None` exactly when there are no nodes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Selection {
    page: Option<PageId>,
    nodes: Vec<NodeId>,
}

impl Selection {
    /// Just `node` of `page`.
    pub(crate) fn one(page: PageId, node: NodeId) -> Self {
        Self {
            page: Some(page),
            nodes: vec![node],
        }
    }

    /// `nodes` of `page` in order, each once (the first time it appears);
    /// nothing when `nodes` is empty.
    pub(crate) fn of(page: PageId, nodes: impl IntoIterator<Item = NodeId>) -> Self {
        let mut selection = Self::default();
        selection.add(page, nodes);
        selection
    }

    /// The page of the selected nodes.
    pub(crate) fn page(&self) -> Option<PageId> {
        self.page
    }

    /// The selected nodes, oldest first.
    pub(crate) fn nodes(&self) -> &[NodeId] {
        &self.nodes
    }

    pub(crate) fn len(&self) -> usize {
        self.nodes.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// The node added last, whose values the Inspector shows.
    pub(crate) fn primary(&self) -> Option<(PageId, NodeId)> {
        Some((self.page?, *self.nodes.last()?))
    }

    /// The node, when it is the only one selected.
    pub(crate) fn single(&self) -> Option<(PageId, NodeId)> {
        (self.nodes.len() == 1).then(|| self.primary()).flatten()
    }

    pub(crate) fn contains(&self, page: PageId, node: NodeId) -> bool {
        self.page == Some(page) && self.nodes.contains(&node)
    }

    /// Shift+click: takes `node` out when it is selected and adds it as the
    /// primary otherwise. On another page it starts over with `node` alone.
    pub(crate) fn toggle(&mut self, page: PageId, node: NodeId) {
        if self.page != Some(page) {
            *self = Self::one(page, node);
        } else if self.nodes.contains(&node) {
            self.retain(|selected| selected != node);
        } else {
            self.nodes.push(node);
        }
    }

    /// Shift+marquee: adds the `nodes` not selected yet, in order; the last
    /// one becomes the primary. On another page it starts over with them;
    /// adding nothing changes nothing.
    pub(crate) fn add(&mut self, page: PageId, nodes: impl IntoIterator<Item = NodeId>) {
        let nodes: Vec<NodeId> = nodes.into_iter().collect();
        if nodes.is_empty() {
            return;
        }
        if self.page != Some(page) {
            self.clear();
        }
        for node in nodes {
            if !self.nodes.contains(&node) {
                self.nodes.push(node);
            }
        }
        self.page = Some(page);
    }

    /// Keeps the nodes `keep` accepts, in order (an undo may have taken the
    /// others away).
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(NodeId) -> bool) {
        self.nodes.retain(|node| keep(*node));
        if self.nodes.is_empty() {
            self.page = None;
        }
    }

    /// Esc: nothing selected.
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: PageId = PageId::new(1);
    const Q: PageId = PageId::new(9);

    fn n(id: u64) -> NodeId {
        NodeId::new(id)
    }

    fn ids(selection: &Selection) -> Vec<u64> {
        selection.nodes().iter().map(|node| node.get()).collect()
    }

    // S40-01: nothing selected has no page and no primary.
    #[test]
    fn s40_01_an_empty_selection_has_no_page_nodes_or_primary() {
        let none = Selection::default();
        assert!(none.is_empty());
        assert_eq!(none.len(), 0);
        assert_eq!(none.page(), None);
        assert_eq!(none.primary(), None);
        assert_eq!(none.single(), None);
        assert!(!none.contains(P, n(2)));
    }

    // S40-01: one element is the primary and the single selection.
    #[test]
    fn s40_01_one_element_is_primary_and_single() {
        let one = Selection::one(P, n(2));
        assert_eq!(one.page(), Some(P));
        assert_eq!(ids(&one), [2]);
        assert_eq!(one.primary(), Some((P, n(2))));
        assert_eq!(one.single(), Some((P, n(2))));
        assert!(one.contains(P, n(2)));
        assert!(!one.contains(Q, n(2)), "the same id on another page");
    }

    // S40-01: `of` keeps the first of each repeat; the last is the primary;
    // an empty list selects nothing.
    #[test]
    fn s40_01_of_drops_repeats_and_the_last_is_primary() {
        let many = Selection::of(P, [n(2), n(3), n(2), n(4)]);
        assert_eq!(ids(&many), [2, 3, 4]);
        assert_eq!(many.len(), 3);
        assert_eq!(many.primary(), Some((P, n(4))));
        assert_eq!(many.single(), None, "more than one is not single");
        assert_eq!(Selection::of(P, []), Selection::default());
    }

    // S40-01: Shift+click toggles: in, out, and the page empties with the
    // last node.
    #[test]
    fn s40_01_toggle_adds_removes_and_empties() {
        let mut selection = Selection::one(P, n(2));
        selection.toggle(P, n(3));
        assert_eq!(ids(&selection), [2, 3]);
        assert_eq!(selection.primary(), Some((P, n(3))));
        selection.toggle(P, n(2));
        assert_eq!(ids(&selection), [3]);
        selection.toggle(P, n(3));
        assert_eq!(selection, Selection::default());
        selection.toggle(P, n(5));
        assert_eq!(selection, Selection::one(P, n(5)));
    }

    // S40-01: a toggle or an add on another page starts over there.
    #[test]
    fn s40_01_another_page_starts_a_new_selection() {
        let mut selection = Selection::of(P, [n(2), n(3)]);
        selection.toggle(Q, n(2));
        assert_eq!(selection, Selection::one(Q, n(2)));
        selection.add(P, [n(7), n(8)]);
        assert_eq!(selection, Selection::of(P, [n(7), n(8)]));
    }

    // S40-01: Shift+marquee adds without repeats; the newest added is the
    // primary; adding nothing changes nothing.
    #[test]
    fn s40_01_add_joins_without_repeats() {
        let mut selection = Selection::of(P, [n(2), n(3)]);
        selection.add(P, [n(3), n(4)]);
        assert_eq!(ids(&selection), [2, 3, 4]);
        assert_eq!(selection.primary(), Some((P, n(4))));
        selection.add(P, []);
        assert_eq!(ids(&selection), [2, 3, 4]);
        let mut none = Selection::default();
        none.add(Q, []);
        assert_eq!(none, Selection::default(), "nothing added, no page");
        selection.add(Q, []);
        assert_eq!(ids(&selection), [2, 3, 4], "nothing on another page");
        assert_eq!(selection.page(), Some(P));
    }

    // S40-01 / S40-07: pruning keeps the order and empties the page with
    // the last node.
    #[test]
    fn s40_01_retain_keeps_the_order_and_can_empty() {
        let mut selection = Selection::of(P, [n(2), n(3), n(4)]);
        selection.retain(|node| node != n(3));
        assert_eq!(ids(&selection), [2, 4]);
        assert_eq!(selection.page(), Some(P));
        selection.retain(|_| false);
        assert_eq!(selection, Selection::default());
    }

    // S40-01: Esc clears.
    #[test]
    fn s40_01_clear_selects_nothing() {
        let mut selection = Selection::of(P, [n(2), n(3)]);
        selection.clear();
        assert_eq!(selection, Selection::default());
    }
}
