//! Black-box tests of the edit capability.

use std::sync::Arc;

use gafira_doc::{
    DocMut, Document, Dpi, EditToken, ElementBase, GroupNode, IdSpaceExhausted, Node, NodeId,
    NodeKind, Page, PageId, PageSize, Placement, ValidationError,
};
use gafira_geom::{Length, Unit};

fn token() -> EditToken {
    EditToken::__new_for_gafira_edit_only()
}

fn size(inches: f64) -> PageSize {
    PageSize {
        width: Length::new(inches, Unit::In),
        height: Length::new(inches, Unit::In),
    }
}

fn group(id: NodeId) -> Node {
    Node::new(
        ElementBase::new(id, Placement::new(0.0, 0.0, 1.0, 1.0)),
        NodeKind::Group(GroupNode {}),
    )
}

/// Adds a page with one root group per call and returns the ids.
fn add_page(edit: &mut DocMut<'_>) -> (PageId, NodeId) {
    let page_id = edit.alloc_page_id().unwrap();
    let node_id = edit.alloc_node_id().unwrap();
    let mut page = Page::new(page_id, String::new(), size(2.0));
    page.roots.push(node_id);
    page.nodes.insert(node_id, Arc::new(group(node_id)));
    edit.pages_mut().push(Arc::new(page));
    (page_id, node_id)
}

/// A document with `pages` pages, each with two root groups.
fn sample(pages: u64) -> Document {
    let t = token();
    let mut doc = Document::default();
    let mut edit = doc.edit(&t);
    for _ in 0..pages {
        let (page_id, first) = add_page(&mut edit);
        let second = edit.alloc_node_id().unwrap();
        let page = edit.page_mut(page_id).unwrap();
        page.roots.push(second);
        page.nodes.insert(second, Arc::new(group(second)));
        assert_ne!(first, second);
    }
    doc
}

#[test]
fn edit_builds_a_valid_document() {
    let mut doc = Document::default();
    let t = token();
    let mut edit = doc.edit(&t);
    let (page_id, node_id) = add_page(&mut edit);
    assert_eq!((page_id.get(), node_id.get()), (1, 2));
    assert_eq!(edit.as_doc().pages().len(), 1);
    assert_eq!(doc.next_id(), 3);
    assert_eq!(doc.page(page_id).unwrap().roots, [node_id]);
    assert!(doc.page(page_id).unwrap().node(node_id).is_some());
    assert_eq!(doc.validate(), Ok(()));
}

#[test]
fn one_token_serves_many_edits() {
    let mut doc = Document::default();
    let t = token();
    doc.edit(&t).set_dpi(Dpi::new(150));
    doc.edit(&t).set_dpi(Dpi::new(200));
    let (page, node) = add_page(&mut doc.edit(&t));
    assert_eq!(doc.dpi(), Dpi::new(200));
    assert!(doc.page(page).unwrap().node(node).is_some());
}

#[test]
fn an_invalid_document_is_allowed_until_validated() {
    let mut doc = sample(1);
    let t = token();
    let mut edit = doc.edit(&t);
    let page_id = edit.as_doc().pages()[0].id;
    let stray = edit.alloc_node_id().unwrap();
    edit.page_mut(page_id)
        .unwrap()
        .nodes
        .insert(stray, Arc::new(group(stray)));
    assert_eq!(doc.validate(), Err(ValidationError::Orphan { node: stray }));
}

#[test]
fn ids_come_from_one_counter() {
    let mut doc = Document::from_parts(5, Dpi::default(), Vec::new()).unwrap();
    let t = token();
    let mut edit = doc.edit(&t);
    assert_eq!(edit.alloc_page_id().unwrap().get(), 5);
    assert_eq!(edit.alloc_node_id().unwrap().get(), 6);
    assert_eq!(edit.alloc_node_id().unwrap().get(), 7);
    assert_eq!(doc.next_id(), 8);
}

#[test]
fn the_last_issuable_id_is_u64_max_minus_one() {
    let mut doc = Document::from_parts(u64::MAX - 1, Dpi::default(), Vec::new()).unwrap();
    let t = token();
    assert_eq!(doc.edit(&t).alloc_node_id().unwrap().get(), u64::MAX - 1);
    assert_eq!(doc.next_id(), u64::MAX);
}

#[test]
fn an_exhausted_counter_fails_and_does_not_wrap() {
    let mut doc = Document::from_parts(u64::MAX, Dpi::default(), Vec::new()).unwrap();
    let t = token();
    assert_eq!(doc.edit(&t).alloc_node_id(), Err(IdSpaceExhausted));
    assert_eq!(doc.edit(&t).alloc_page_id(), Err(IdSpaceExhausted));
    assert_eq!(doc.next_id(), u64::MAX);
}

#[test]
fn a_missing_page_or_node_is_none() {
    let mut doc = sample(1);
    let t = token();
    let mut edit = doc.edit(&t);
    let page = edit.as_doc().pages()[0].id;
    assert!(edit.page_mut(PageId::new(999)).is_none());
    assert!(edit.node_mut(PageId::new(999), NodeId::new(2)).is_none());
    assert!(edit.node_mut(page, NodeId::new(999)).is_none());
}

#[test]
fn changing_the_dpi_can_break_a_page_size() {
    let mut doc = sample(1);
    let t = token();
    assert_eq!(doc.validate(), Ok(()));
    doc.edit(&t).set_dpi(Dpi::new(10_000));
    assert_eq!(doc.dpi(), Dpi::new(10_000));
    assert!(matches!(
        doc.validate(),
        Err(ValidationError::PageSize { .. })
    ));
}

#[test]
fn an_edit_copies_only_its_path() {
    let mut doc = sample(2);
    let before = doc.clone();
    let t = token();
    let (page_id, node_id) = (doc.pages()[0].id, NodeId::new(3));
    doc.edit(&t).node_mut(page_id, node_id).unwrap().base.locked = true;

    let (old_page, new_page) = (&before.pages()[0], &doc.pages()[0]);
    assert!(!Arc::ptr_eq(old_page, new_page));
    assert!(!Arc::ptr_eq(
        &old_page.nodes[&node_id],
        &new_page.nodes[&node_id]
    ));
    assert!(new_page.node(node_id).unwrap().base.locked);
    assert!(!old_page.node(node_id).unwrap().base.locked);
    let other = NodeId::new(2);
    assert!(Arc::ptr_eq(
        &old_page.nodes[&other],
        &new_page.nodes[&other]
    ));
    assert!(Arc::ptr_eq(&before.pages()[1], &doc.pages()[1]));
}

#[test]
fn a_cloned_node_handle_does_not_make_the_document_mutable() {
    let doc = sample(1);
    let page = &doc.pages()[0];
    let id = NodeId::new(2);
    let mut handle = Arc::clone(&page.nodes[&id]);
    Arc::make_mut(&mut handle).base.locked = true;
    assert!(!doc.pages()[0].node(id).unwrap().base.locked);
    assert!(!Arc::ptr_eq(&handle, &doc.pages()[0].nodes[&id]));
}
