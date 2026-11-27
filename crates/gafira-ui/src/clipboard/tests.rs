use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use gafira_doc::{
    AssetStorage, ContentHash, Dpi, ElementBase, GroupNode, ImageNode, Node, PageSize, Placement,
    ShapeGeometry, ShapeNode, SpanStyle, TextNode,
};
use gafira_edit::History;
use gafira_geom::{Length, Unit};

use super::*;

const P: PageId = PageId::new(500);
const Q: PageId = PageId::new(600);

fn n(id: u64) -> NodeId {
    NodeId::new(id)
}

fn place(x: f64, y: f64, w: f64, h: f64) -> Placement {
    Placement::new(x, y, w, h)
}

fn asset() -> AssetRef {
    AssetRef::new(ContentHash([7; 32]))
}

fn entry() -> AssetEntry {
    AssetEntry {
        name: "photo.png".to_owned(),
        storage: AssetStorage::Embedded,
    }
}

fn node(id: u64, at: Placement, kind: NodeKind, children: &[u64]) -> (NodeId, Arc<Node>) {
    let mut node = Node::new(ElementBase::new(n(id), at), kind);
    node.children = children.iter().copied().map(n).collect();
    (n(id), Arc::new(node))
}

fn rect() -> NodeKind {
    NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect))
}

fn size(w: f64, h: f64) -> PageSize {
    PageSize {
        width: Length::new(w, Unit::Px),
        height: Length::new(h, Unit::Px),
    }
}

/// Page P (1000 × 800): rectangle 1 named «Logo», group 4 holding the
/// image 2 and the text 3, and rectangle 5. Page Q (200 × 200) is empty.
fn document() -> Document {
    let mut page = Page::new(P, String::new(), size(1000.0, 800.0));
    let style = SpanStyle {
        font: "Inter".to_owned(),
        size: 20.0,
        color: gafira_color::Color::Srgb(gafira_color::Rgba::new(0.0, 0.0, 0.0, 1.0)),
        bold: false,
        italic: false,
        gradient: None,
    };
    let text = TextNode::plain("Hola", &style);
    page.nodes = [
        node(1, place(10.0, 10.0, 50.0, 50.0), rect(), &[]),
        node(
            2,
            place(100.0, 100.0, 40.0, 30.0),
            NodeKind::Image(ImageNode::new(asset())),
            &[],
        ),
        node(
            3,
            place(100.0, 140.0, 80.0, 20.0),
            NodeKind::Text(text),
            &[],
        ),
        node(
            4,
            place(100.0, 100.0, 80.0, 60.0),
            NodeKind::Group(GroupNode {}),
            &[2, 3],
        ),
        node(5, place(600.0, 600.0, 300.0, 100.0), rect(), &[]),
    ]
    .into_iter()
    .collect();
    let mut logo = page.node(n(1)).expect("a node").clone();
    logo.base.name = Some("Logo".to_owned());
    page.nodes.insert(n(1), Arc::new(logo));
    page.roots = vec![n(1), n(4), n(5)];
    let other = Page::new(Q, String::new(), size(200.0, 200.0));
    let assets = [(asset(), entry())].into_iter().collect();
    Document::from_parts_with_assets(
        700,
        Dpi::new(96),
        vec![Arc::new(page), Arc::new(other)],
        assets,
    )
    .expect("a valid document")
}

fn ids(list: &[NodeId]) -> Vec<u64> {
    list.iter().map(|id| id.get()).collect()
}

fn page_of(history: &History, id: PageId) -> Page {
    history.document().page(id).expect("the page").clone()
}

// S43-08: a copy holds each subtree once, back to front, with the entries
// of its images; a node inside a listed group goes with the group.
#[test]
fn s43_08_copy_holds_subtrees_back_to_front_with_their_assets() {
    let doc = document();
    let page = doc.page(P).expect("the page");
    assert_eq!(ids(&tops(page, &[n(5), n(2), n(4), n(1)])), [1, 4, 5]);
    let clip = copy(&doc, P, &[n(5), n(4)]).expect("a clip");
    assert_eq!(clip.specs.len(), 2);
    assert!(matches!(clip.specs[0].kind, NodeKind::Group(_)));
    assert_eq!(clip.specs[0].children.len(), 2);
    assert_eq!(clip.specs[1].placement, place(600.0, 600.0, 300.0, 100.0));
    assert_eq!(clip.assets, [(asset(), entry())]);
    let shapes = copy(&doc, P, &[n(1)]).expect("a clip");
    assert!(shapes.assets.is_empty());
    assert_eq!(copy(&doc, P, &[]), None);
    assert_eq!(copy(&doc, P, &[n(99)]), None);
    assert_eq!(copy(&doc, PageId::new(9), &[n(1)]), None);
}

// S43-08: the plain text of a copy has the words of its texts, or the name
// or kind of each element, one line each.
#[test]
fn s43_08_the_plain_text_is_the_words_or_the_names() {
    let doc = document();
    let clip = copy(&doc, P, &[n(1), n(4), n(5)]).expect("a clip");
    let text = plain_text(&clip.specs, |kind| match kind {
        NodeKind::Shape(_) => "Rectángulo".to_owned(),
        _ => "Elemento".to_owned(),
    });
    assert_eq!(text, "Logo\nHola\nRectángulo");
}

// S43-09: a duplicate is a copy with new ids just above the frontmost
// original, moved by the copy offset, its named top elements named «… copia»,
// selected afterwards, in one undo step (FD-13 (c)).
#[test]
fn s43_09_duplicate_makes_new_ids_above_the_originals_in_one_undo() {
    let doc = document();
    let start = doc.page(P).expect("the page").clone();
    let mut history = History::new(doc).expect("a history");
    let next = history.document().next_id();
    let (command, copies) =
        duplicate_command(&start, &[n(4), n(1)], next, |name| format!("{name} copia"))
            .expect("a command");
    assert_eq!(
        ids(&copies),
        [next, next + 1],
        "1 first: it is further back"
    );
    history
        .execute(command, Duration::ZERO)
        .expect("it duplicates");
    let page = page_of(&history, P);
    assert_eq!(ids(&page.roots), [1, 4, next, next + 1, 5]);
    let old: BTreeSet<NodeId> = start.nodes.keys().copied().collect();
    let new: BTreeSet<NodeId> = page
        .nodes
        .keys()
        .copied()
        .filter(|id| !old.contains(id))
        .collect();
    assert_eq!(
        new.len(),
        4,
        "the rectangle, the group and its two children"
    );
    let logo = page.node(copies[0]).expect("the copy");
    assert_eq!(logo.base.name.as_deref(), Some("Logo copia"));
    assert_eq!(logo.base.placement, place(20.0, 20.0, 50.0, 50.0));
    let group = page.node(copies[1]).expect("the copy");
    assert_eq!(group.base.name, None, "unnamed stays unnamed");
    let child = page.node(group.children[0]).expect("a child");
    assert_eq!(child.base.placement, place(110.0, 110.0, 40.0, 30.0));
    assert!(!old.contains(&group.children[0]));
    assert_eq!(history.len(), 1);
    history.undo().expect("an undo");
    assert_eq!(page_of(&history, P), start);
    assert!(duplicate_command(&start, &[], next, |n| n.to_owned()).is_none());
}

fn clip_of(doc: &Document, nodes: &[NodeId]) -> Clip {
    copy(doc, P, nodes).expect("a clip")
}

// S43-10: a paste over its own original moves by the offset, once more per
// paste; onto a free place it stays; outside the page it goes to the middle.
#[test]
fn s43_10_paste_lands_free_and_visible() {
    let doc = document();
    let page = doc.page(P).expect("the page");
    let one = clip_of(&doc, &[n(1)]);
    assert_eq!(
        paste_offset(&one, page, (1000.0, 800.0), None),
        (10.0, 10.0)
    );
    let other = doc.page(Q).expect("the page");
    assert_eq!(paste_offset(&one, other, (200.0, 200.0), None), (0.0, 0.0));
    // Once a copy sits at +10, the next paste goes to +20.
    let mut history = History::new(doc.clone()).expect("a history");
    let (command, _) = paste_command(&doc, P, None, &one).expect("a command");
    history.execute(command, Duration::ZERO).expect("it pastes");
    let after = page_of(&history, P);
    assert_eq!(
        paste_offset(&one, &after, (1000.0, 800.0), None),
        (20.0, 20.0)
    );
    // Rectangle 5 (600..900, 600..700) is off the 200 px page: its box
    // goes to the middle, (−50, 50) to (250, 150).
    let far = clip_of(&doc, &[n(5)]);
    assert_eq!(
        paste_offset(&far, other, (200.0, 200.0), None),
        (-650.0, -550.0)
    );
}

// S43-10: a paste inserts on top with new ids and is one undo step; in
// another document it brings the entries of its images, which the undo
// takes away again.
#[test]
fn s43_10_paste_into_another_document_brings_its_assets_in_one_undo() {
    let doc = document();
    let clip = clip_of(&doc, &[n(4)]);
    let target = Document::from_parts(
        50,
        Dpi::new(96),
        vec![Arc::new(Page::new(
            PageId::new(1),
            String::new(),
            size(1000.0, 800.0),
        ))],
    )
    .expect("a document");
    let mut history = History::new(target.clone()).expect("a history");
    let (command, pasted) = paste_command(&target, PageId::new(1), None, &clip).expect("a command");
    assert_eq!(ids(&pasted), [50]);
    history.execute(command, Duration::ZERO).expect("it pastes");
    let now = history.document().clone();
    assert_eq!(now.asset(&asset()), Some(&entry()));
    let page = now.page(PageId::new(1)).expect("the page");
    assert_eq!(ids(&page.roots), [50]);
    assert_eq!(page.node(n(50)).expect("the group").children.len(), 2);
    assert_eq!(history.len(), 1);
    history.undo().expect("an undo");
    assert_eq!(**history.document(), target);
    // In its own document the entry is already there: nothing to add.
    let mut same = History::new(doc.clone()).expect("a history");
    let (command, pasted) = paste_command(&doc, P, None, &clip).expect("a command");
    // Only the insert: an `InsertAsset` that changes nothing could not be
    // reverted once the history drops its snapshot.
    assert_eq!(command.label(), MessageKey("edit.insert_nodes"));
    same.execute(command, Duration::ZERO).expect("it pastes");
    let page = page_of(&same, P);
    assert_eq!(page.roots.last(), pasted.first());
    let group = page.node(pasted[0]).expect("the group");
    assert_eq!(
        page.node(group.children[0])
            .expect("a child")
            .base
            .placement,
        place(110.0, 110.0, 40.0, 30.0),
        "over its original: moved by the offset"
    );
    same.undo().expect("an undo");
    assert_eq!(**same.document(), doc);
    assert!(paste_command(&doc, PageId::new(9), None, &clip).is_none());
}
