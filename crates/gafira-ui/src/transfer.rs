//! Moving elements from one page to another (spec 051, D-92): which page a
//! drag is dropped on, and the one-step command that takes the dragged
//! subtrees out of their page and puts them on top of the other one. Pure,
//! so it is tested without a window.

use egui::{Pos2, Rect};
use gafira_doc::msg::MessageKey;
use gafira_doc::{Document, NodeId, PageId};
use gafira_edit::{Command, InsertNodes, NodeSpec, RemoveNodes, Transaction};

use crate::{clipboard, group};

/// The page a move dropped at `at` goes to (S51-04): the page under the
/// pointer when it is not `from`; `None` on `from` or outside every page.
pub(crate) fn drop_target(pages: &[(PageId, Rect)], from: PageId, at: Pos2) -> Option<PageId> {
    pages
        .iter()
        .find(|(_, rect)| rect.contains(at))
        .map(|(page, _)| *page)
        .filter(|page| *page != from)
}

/// The one-step command that moves `nodes` of page `from` to the top of
/// page `to`, each shifted by `(dx, dy)` page px (S51-05), with the ids of
/// the moved elements in their new page. `None` with nothing to move or a
/// missing page.
pub(crate) fn transfer_command(
    document: &Document,
    from: PageId,
    nodes: &[NodeId],
    to: PageId,
    (dx, dy): (f64, f64),
) -> Option<(Box<dyn Command>, Vec<NodeId>)> {
    if from == to {
        return None;
    }
    let source = document.page(from)?;
    let target = document.page(to)?;
    let tops = clipboard::tops(source, nodes);
    let specs: Vec<NodeSpec> = tops
        .iter()
        .map(|id| Some(group::translated(&group::spec_of(source, *id)?, dx, dy)))
        .collect::<Option<_>>()?;
    if specs.is_empty() {
        return None;
    }
    // Removing frees no id, so the insert takes the ids from the counter
    // as it is now.
    let ids = group::top_ids(document.next_id(), &specs);
    let commands: Vec<Box<dyn Command>> = vec![
        Box::new(RemoveNodes::new(from, tops)),
        Box::new(InsertNodes::new(to, None, target.roots.len(), specs)),
    ];
    Some((
        Box::new(Transaction::new(MessageKey("edit.move_to_page"), commands)),
        ids,
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use egui::Vec2;
    use gafira_doc::{
        Dpi, ElementBase, GroupNode, Node, NodeKind, Page, PageSize, Placement, ShapeGeometry,
        ShapeNode,
    };
    use gafira_edit::History;
    use gafira_geom::{Length, Unit};

    use super::*;

    const ONE: PageId = PageId::new(50);
    const TWO: PageId = PageId::new(51);

    fn n(id: u64) -> NodeId {
        NodeId::new(id)
    }

    fn node(id: u64, at: Placement, kind: NodeKind, children: &[u64]) -> (NodeId, Arc<Node>) {
        let mut node = Node::new(ElementBase::new(n(id), at), kind);
        node.children = children.iter().copied().map(n).collect();
        (n(id), Arc::new(node))
    }

    fn rect() -> NodeKind {
        NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect))
    }

    fn size() -> PageSize {
        PageSize {
            width: Length::new(400.0, Unit::Px),
            height: Length::new(300.0, Unit::Px),
        }
    }

    /// Page 1: rectangle 1, group 4 holding rectangles 2 and 3, rectangle
    /// 5. Page 2: rectangle 6.
    fn document() -> Document {
        let mut one = Page::new(ONE, String::new(), size());
        one.nodes = [
            node(1, Placement::new(10.0, 10.0, 50.0, 50.0), rect(), &[]),
            node(2, Placement::new(100.0, 100.0, 20.0, 20.0), rect(), &[]),
            node(3, Placement::new(130.0, 100.0, 20.0, 20.0), rect(), &[]),
            node(
                4,
                Placement::new(100.0, 100.0, 50.0, 20.0),
                NodeKind::Group(GroupNode {}),
                &[2, 3],
            ),
            node(5, Placement::new(200.0, 250.0, 100.0, 100.0), rect(), &[]),
        ]
        .into_iter()
        .collect();
        one.roots = vec![n(1), n(4), n(5)];
        let mut two = Page::new(TWO, String::new(), size());
        two.nodes = [node(6, Placement::new(0.0, 0.0, 10.0, 10.0), rect(), &[])]
            .into_iter()
            .collect();
        two.roots = vec![n(6)];
        Document::from_parts(100, Dpi::new(96), vec![Arc::new(one), Arc::new(two)])
            .expect("a valid document")
    }

    // S51-04: a move dropped on another page goes there; on its own page,
    // in the gap or outside every page it stays.
    #[test]
    fn s51_04_drop_target() {
        let pages = [
            (
                ONE,
                Rect::from_min_size(Pos2::new(100.0, 50.0), Vec2::new(400.0, 300.0)),
            ),
            (
                TWO,
                Rect::from_min_size(Pos2::new(100.0, 400.0), Vec2::new(400.0, 300.0)),
            ),
        ];
        assert_eq!(drop_target(&pages, ONE, Pos2::new(300.0, 500.0)), Some(TWO));
        assert_eq!(drop_target(&pages, TWO, Pos2::new(300.0, 100.0)), Some(ONE));
        assert_eq!(drop_target(&pages, ONE, Pos2::new(300.0, 100.0)), None);
        assert_eq!(
            drop_target(&pages, ONE, Pos2::new(300.0, 375.0)),
            None,
            "gap"
        );
        assert_eq!(
            drop_target(&pages, ONE, Pos2::new(20.0, 500.0)),
            None,
            "outside"
        );
    }

    // S51-05: rectangle 5 and group 4 (with its children) leave page 1 and
    // go on top of page 2, shifted, in their order, with the ids
    // `top_ids` gives; one undo step brings everything back.
    #[test]
    fn s51_05_transfer() {
        let doc = document();
        let (command, moved) =
            transfer_command(&doc, ONE, &[n(5), n(4)], TWO, (5.0, -350.0)).expect("a command");
        assert_eq!(moved, [n(100), n(103)], "the group takes 3 ids");
        let mut history = History::new(doc.clone()).expect("a history");
        history
            .execute(command, Duration::ZERO)
            .expect("it transfers");
        let after = history.document();
        let one = after.page(ONE).expect("page 1");
        assert_eq!(one.roots, [n(1)]);
        assert!(one.node(n(2)).is_none() && one.node(n(5)).is_none());
        let two = after.page(TWO).expect("page 2");
        assert_eq!(two.roots, [n(6), n(100), n(103)]);
        let group = two.node(n(100)).expect("the group");
        assert!(matches!(group.kind, NodeKind::Group(_)));
        assert_eq!(group.children.len(), 2);
        let child = two.node(group.children[0]).expect("a child");
        assert_eq!(
            child.base.placement,
            Placement::new(105.0, -250.0, 20.0, 20.0)
        );
        let five = two.node(n(103)).expect("rectangle 5");
        assert_eq!(
            five.base.placement,
            Placement::new(205.0, -100.0, 100.0, 100.0)
        );
        history.undo().expect("an undo");
        assert_eq!(**history.document(), doc);
        assert!(transfer_command(&doc, ONE, &[], TWO, (0.0, 0.0)).is_none());
        assert!(transfer_command(&doc, ONE, &[n(1)], PageId::new(9), (0.0, 0.0)).is_none());
        assert!(transfer_command(&doc, ONE, &[n(1)], ONE, (0.0, 0.0)).is_none());
    }
}
