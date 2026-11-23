//! `ReorderPage` and `DuplicatePage` (spec 056, FD-02).

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use gafira_doc::{
    Document, Dpi, ElementBase, GroupNode, Guide, GuideAxis, GuideSet, Node, NodeId, NodeKind,
    Page, PageId, PageSize, Placement, ShapeGeometry, ShapeNode,
};
use gafira_edit::{CommandError, DuplicatePage, History, ReorderPage};
use gafira_geom::{Length, Unit};

const P1: PageId = PageId::new(1);
const P2: PageId = PageId::new(2);
const P3: PageId = PageId::new(3);

fn size() -> PageSize {
    PageSize {
        width: Length::new(100.0, Unit::Px),
        height: Length::new(100.0, Unit::Px),
    }
}

/// Three pages; page 1 holds rectangle 4 and group 5 with rectangle 6, a
/// guide and a name.
fn document() -> Document {
    let mut one = Page::new(P1, "Portada".to_owned(), size());
    let rect = |id: u64| {
        Node::new(
            ElementBase::new(NodeId::new(id), Placement::new(id as f64, 0.0, 10.0, 10.0)),
            NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect)),
        )
    };
    let mut group = Node::new(
        ElementBase::new(NodeId::new(5), Placement::new(6.0, 0.0, 10.0, 10.0)),
        NodeKind::Group(GroupNode {}),
    );
    group.children = vec![NodeId::new(6)];
    one.nodes.insert(NodeId::new(4), Arc::new(rect(4)));
    one.nodes.insert(NodeId::new(5), Arc::new(group));
    one.nodes.insert(NodeId::new(6), Arc::new(rect(6)));
    one.roots = vec![NodeId::new(4), NodeId::new(5)];
    one.guides = GuideSet(vec![Guide {
        axis: GuideAxis::Vertical,
        position: Length::new(20.0, Unit::Px),
        locked: false,
    }]);
    let pages = vec![
        Arc::new(one),
        Arc::new(Page::new(P2, String::new(), size())),
        Arc::new(Page::new(P3, String::new(), size())),
    ];
    Document::from_parts(7, Dpi::new(96), pages).expect("a valid document")
}

fn order(history: &History) -> Vec<PageId> {
    history
        .document()
        .pages()
        .iter()
        .map(|page| page.id)
        .collect()
}

// S56-01: a page moves to the index asked for; out of range or a missing
// page is an error; an undo puts it back.
#[test]
fn s56_01_reorder_page() {
    let mut history = History::new(document()).expect("a history");
    history
        .execute(Box::new(ReorderPage::new(P1, 2)), Duration::ZERO)
        .expect("it moves");
    assert_eq!(order(&history), [P2, P3, P1]);
    history
        .execute(Box::new(ReorderPage::new(P3, 0)), Duration::from_secs(1))
        .expect("it moves");
    assert_eq!(order(&history), [P3, P2, P1]);
    history.undo().expect("an undo");
    history.undo().expect("an undo");
    assert_eq!(*history.document(), Arc::new(document()));
    assert!(matches!(
        history.execute(Box::new(ReorderPage::new(P1, 3)), Duration::ZERO),
        Err(CommandError::IndexOutOfRange { index: 3, len: 3 })
    ));
    assert!(matches!(
        history.execute(
            Box::new(ReorderPage::new(PageId::new(99), 0)),
            Duration::ZERO
        ),
        Err(CommandError::MissingPage(_))
    ));
}

// S56-02: the copy lands right after its page with a new page id and new
// node ids, the same content, tree, guides and name; the document stays
// valid; an undo takes it away and gives the ids back, so a redo rebuilds
// the same document.
#[test]
fn s56_02_duplicate_page() {
    let original = document();
    let mut history = History::new(original.clone()).expect("a history");
    history
        .execute(Box::new(DuplicatePage::new(P1)), Duration::ZERO)
        .expect("it copies");
    let after = history.document();
    assert_eq!(after.pages().len(), 4);
    let copy = &after.pages()[1];
    assert_eq!(copy.id, PageId::new(7), "the next id");
    assert_eq!(copy.name, "Portada");
    assert_eq!(copy.guides, original.pages()[0].guides);
    let source = &after.pages()[0];
    let old: BTreeSet<NodeId> = source.nodes.keys().copied().collect();
    let new: BTreeSet<NodeId> = copy.nodes.keys().copied().collect();
    assert_eq!(new.len(), 3);
    assert!(old.is_disjoint(&new), "no id is shared");
    assert_eq!(copy.roots.len(), 2);
    let group = copy.node(copy.roots[1]).expect("the group copy");
    assert!(matches!(group.kind, NodeKind::Group(_)));
    let child = copy.node(group.children[0]).expect("its child");
    assert_eq!(
        child.base.placement,
        source.node(NodeId::new(6)).unwrap().base.placement
    );
    assert_eq!(child.base.id, group.children[0]);
    assert_eq!(after.validate(), Ok(()));
    assert_eq!(after.next_id(), 11);
    history.undo().expect("an undo");
    assert_eq!(**history.document(), original);
    history.redo().expect("a redo");
    assert_eq!(history.document().pages()[1].id, PageId::new(7));
    assert!(matches!(
        History::new(original).expect("a history").execute(
            Box::new(DuplicatePage::new(PageId::new(99))),
            Duration::ZERO
        ),
        Err(CommandError::MissingPage(_))
    ));
}
