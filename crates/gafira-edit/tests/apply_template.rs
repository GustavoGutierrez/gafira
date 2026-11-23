//! `ApplyTemplate` (spec 091): the first page of a template takes the
//! place of a page's content, its other pages follow it, with new ids.

use std::sync::Arc;
use std::time::Duration;

use gafira_color::{Color, Rgba};
use gafira_doc::{
    Attachment, Document, Dpi, ElementBase, GroupNode, LineCap, LineEnd, LineNode, Node, NodeId,
    NodeKind, Page, PageBackground, PageId, PageSize, Placement, ShapeGeometry, ShapeNode,
};
use gafira_edit::{ApplyTemplate, CommandError, History};
use gafira_geom::{Length, Unit};

const P1: PageId = PageId::new(1);
const P2: PageId = PageId::new(2);

fn size(w: f64, h: f64) -> PageSize {
    PageSize {
        width: Length::new(w, Unit::Px),
        height: Length::new(h, Unit::Px),
    }
}

fn rect(id: u64, x: f64) -> Node {
    Node::new(
        ElementBase::new(NodeId::new(id), Placement::new(x, 0.0, 10.0, 10.0)),
        NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect)),
    )
}

fn red() -> PageBackground {
    PageBackground::Solid(Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0)))
}

/// The document: page 1 with rectangle 3 and a guide, page 2 empty.
fn document() -> Document {
    let mut one = Page::new(P1, "Mía".to_owned(), size(200.0, 100.0));
    one.guides = gafira_doc::GuideSet(vec![gafira_doc::Guide {
        axis: gafira_doc::GuideAxis::Vertical,
        position: Length::new(66.0, Unit::Px),
        locked: true,
    }]);
    one.bleed = Length::new(3.0, Unit::Mm);
    one.nodes.insert(NodeId::new(3), Arc::new(rect(3, 1.0)));
    one.roots = vec![NodeId::new(3)];
    let two = Page::new(P2, String::new(), size(200.0, 100.0));
    Document::from_parts(4, Dpi::new(96), vec![Arc::new(one), Arc::new(two)])
        .expect("a valid document")
}

/// A template of two pages whose ids clash with the document's: page 1
/// has group 3 with rectangle 4 and a line stuck to 4; page 2 rectangle 3.
fn template() -> Vec<Page> {
    let mut first = Page::new(P1, "Portada".to_owned(), size(200.0, 100.0));
    let mut group = Node::new(
        ElementBase::new(NodeId::new(3), Placement::new(0.0, 0.0, 50.0, 50.0)),
        NodeKind::Group(GroupNode {}),
    );
    group.children = vec![NodeId::new(4)];
    let mut start = LineEnd::new([0.0, 0.0], LineCap::default());
    start.attached = Some(Attachment {
        node: NodeId::new(4),
        at: [0.5, 0.5],
    });
    let line = Node::new(
        ElementBase::new(NodeId::new(5), Placement::new(0.0, 0.0, 40.0, 40.0)),
        NodeKind::Line(LineNode {
            start,
            end: LineEnd::new([1.0, 1.0], LineCap::default()),
            color: Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)),
            width: 2.0,
            dash: Default::default(),
        }),
    );
    first.nodes.insert(NodeId::new(3), Arc::new(group));
    first.nodes.insert(NodeId::new(4), Arc::new(rect(4, 5.0)));
    first.nodes.insert(NodeId::new(5), Arc::new(line));
    first.roots = vec![NodeId::new(3), NodeId::new(5)];
    first.background = Some(red());
    let mut second = Page::new(P2, "Segunda".to_owned(), size(200.0, 100.0));
    second.nodes.insert(NodeId::new(3), Arc::new(rect(3, 9.0)));
    second.roots = vec![NodeId::new(3)];
    vec![first, second]
}

#[test]
fn s91_10_the_page_takes_the_template_and_the_rest_follow_it() {
    let before = document();
    let mut history = History::new(before.clone()).expect("a history");
    history
        .execute(Box::new(ApplyTemplate::new(P1, template())), Duration::ZERO)
        .expect("it applies");
    let doc = history.document();
    let ids: Vec<PageId> = doc.pages().iter().map(|p| p.id).collect();
    assert_eq!(ids.len(), 3, "a page more");
    assert_eq!((ids[0], ids[2]), (P1, P2), "the new page goes right after");
    let page = doc.page(P1).expect("page 1");
    assert_eq!(page.name, "Mía", "the page keeps its own name and size");
    assert_eq!(page.size, size(200.0, 100.0));
    assert_eq!(page.background, Some(red()), "the template's background");
    assert_eq!(page.nodes.len(), 3, "the old rectangle is gone");
    assert!(!page.nodes.contains_key(&NodeId::new(3)), "new ids");
    // The group keeps its child and the line its shape, under the new ids.
    let group = page.node(page.roots[0]).expect("the group");
    let child = group.children[0];
    assert!(matches!(
        page.node(child).expect("child").kind,
        NodeKind::Shape(_)
    ));
    let NodeKind::Line(line) = &page.node(page.roots[1]).expect("the line").kind else {
        panic!("a line");
    };
    assert_eq!(line.start.attached.map(|a| a.node), Some(child));
    let added = doc.page(ids[1]).expect("the added page");
    assert_eq!(added.size, size(200.0, 100.0), "the size of the page");
    assert_eq!(added.guides, page.guides, "its folds and margins");
    assert_eq!(added.bleed, page.bleed, "and its bleed");
    assert_eq!(added.nodes.len(), 1);
    // Every id is fresh: none is used twice in the document.
    let mut all: Vec<u64> = doc
        .pages()
        .iter()
        .flat_map(|p| std::iter::once(p.id.get()).chain(p.nodes.keys().map(|n| n.get())))
        .collect();
    all.sort_unstable();
    let len = all.len();
    all.dedup();
    assert_eq!(all.len(), len);
    // One undo brings everything back; a redo gives the same document.
    let applied = doc.clone();
    history.undo().expect("an undo");
    assert_eq!(**history.document(), before);
    history.redo().expect("a redo");
    assert_eq!(**history.document(), *applied);
}

#[test]
fn s91_10_a_missing_page_or_no_template_page_is_refused() {
    let mut history = History::new(document()).expect("a history");
    let missing = history.execute(
        Box::new(ApplyTemplate::new(PageId::new(99), template())),
        Duration::ZERO,
    );
    assert!(matches!(missing, Err(CommandError::MissingPage(_))));
    let empty = history.execute(Box::new(ApplyTemplate::new(P1, Vec::new())), Duration::ZERO);
    assert!(matches!(empty, Err(CommandError::InvalidValue(_))));
    assert_eq!(**history.document(), document(), "nothing changed");
}
