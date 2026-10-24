//! A page and its node arena.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::{Node, NodeId, PageId, PageSize};

/// One page: its size, flags, root order and the arena that owns its nodes.
///
/// Ids in `roots` and in node `children` resolve only inside this page's own
/// `nodes`. Structural violations are representable; `Document::validate`
/// rejects them. The page flags never change a node's own `locked`/`visible`.
#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    /// The page id, unique in the document.
    pub id: PageId,
    /// Display name.
    pub name: String,
    /// Dimensions in any unit.
    pub size: PageSize,
    /// Whether the whole page is protected from edits.
    pub locked: bool,
    /// Whether the whole page is hidden.
    pub hidden: bool,
    /// Top-level node ids, back to front.
    pub roots: Vec<NodeId>,
    /// The arena, iterating in ascending [`NodeId`] order.
    pub nodes: BTreeMap<NodeId, Arc<Node>>,
}

impl Page {
    /// An unlocked, visible page with no nodes.
    pub fn new(id: PageId, name: String, size: PageSize) -> Self {
        Self {
            id,
            name,
            size,
            locked: false,
            hidden: false,
            roots: Vec::new(),
            nodes: BTreeMap::new(),
        }
    }

    /// The node with this id in this page's arena, if any.
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id).map(Arc::as_ref)
    }
}
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gafira_geom::{Length, Unit};

    use super::*;
    use crate::{ElementBase, GroupNode, NodeKind, Placement};

    fn size() -> PageSize {
        PageSize {
            width: Length::new(100.0, Unit::Px),
            height: Length::new(50.0, Unit::Px),
        }
    }

    fn node(id: u64) -> Arc<Node> {
        Arc::new(Node::new(
            ElementBase::new(NodeId::new(id), Placement::new(0.0, 0.0, 1.0, 1.0)),
            NodeKind::Group(GroupNode {}),
        ))
    }

    #[test]
    fn page_defaults() {
        let p = Page::new(PageId::new(1), "Home".to_string(), size());
        assert_eq!(p.id, PageId::new(1));
        assert_eq!(p.name, "Home");
        assert_eq!(p.size, size());
        assert!(!p.locked);
        assert!(!p.hidden);
        assert!(p.roots.is_empty());
        assert!(p.nodes.is_empty());
    }

    #[test]
    fn iteration_is_in_ascending_id_order() {
        let mut p = Page::new(PageId::new(1), String::new(), size());
        for id in [10, 2, 7] {
            p.nodes.insert(NodeId::new(id), node(id));
        }
        let order: Vec<u64> = p.nodes.keys().map(|k| k.get()).collect();
        assert_eq!(order, [2, 7, 10]);
    }

    #[test]
    fn lookup_stays_inside_the_page() {
        let first = Page::new(PageId::new(1), String::new(), size());
        let mut second = Page::new(PageId::new(2), String::new(), size());
        second.nodes.insert(NodeId::new(9), node(9));
        assert!(first.node(NodeId::new(9)).is_none());
        let found = second.node(NodeId::new(9)).unwrap();
        assert_eq!(found.base.id, NodeId::new(9));
    }

    #[test]
    fn page_flags_do_not_cascade() {
        let mut p = Page::new(PageId::new(1), String::new(), size());
        p.nodes.insert(NodeId::new(4), node(4));
        p.locked = true;
        p.hidden = true;
        let n = p.node(NodeId::new(4)).unwrap();
        assert!(n.base.visible);
        assert!(!n.base.locked);
    }

    #[test]
    fn cloning_a_page_shares_its_nodes() {
        let mut p = Page::new(PageId::new(1), String::new(), size());
        p.nodes.insert(NodeId::new(4), node(4));
        let c = p.clone();
        assert!(Arc::ptr_eq(
            &p.nodes[&NodeId::new(4)],
            &c.nodes[&NodeId::new(4)]
        ));
    }
}
