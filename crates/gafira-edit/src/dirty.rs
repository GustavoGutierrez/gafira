//! The pages and nodes a command touches.

use std::collections::BTreeSet;

use gafira_doc::{NodeId, PageId};

/// What a command changed, for cache invalidation (02 §4.3).
///
/// A page is listed when its own fields or its `roots` change; a node is
/// listed when its record or its `children` change.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DirtySet {
    pub pages: BTreeSet<PageId>,
    pub nodes: BTreeSet<NodeId>,
}

impl DirtySet {
    /// Everything in `self` or in `other`, once.
    #[must_use]
    pub fn union(&self, other: &DirtySet) -> DirtySet {
        DirtySet {
            pages: self.pages.union(&other.pages).copied().collect(),
            nodes: self.nodes.union(&other.nodes).copied().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(pages: &[u64], nodes: &[u64]) -> DirtySet {
        DirtySet {
            pages: pages.iter().copied().map(PageId::new).collect(),
            nodes: nodes.iter().copied().map(NodeId::new).collect(),
        }
    }

    #[test]
    fn union_is_ordered_and_deduplicated() {
        let joined = set(&[3, 1], &[9, 5]).union(&set(&[2, 3], &[5, 4]));
        let pages: Vec<u64> = joined.pages.iter().map(|p| p.get()).collect();
        let nodes: Vec<u64> = joined.nodes.iter().map(|n| n.get()).collect();
        assert_eq!(pages, [1, 2, 3]);
        assert_eq!(nodes, [4, 5, 9]);
    }
}
