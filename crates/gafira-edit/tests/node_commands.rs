//! `InsertNodes` and `RemoveNodes` through their public contract.

use std::sync::Arc;

use gafira_doc::msg::MessageKey;
use gafira_doc::{
    Document, Dpi, EditToken, GroupNode, NodeId, NodeKind, Page, PageId, PageSize, Placement,
    ShapeGeometry, ShapeNode, ValidationError,
};
use gafira_edit::{Applied, Command, CommandError, InsertNodes, NodeSpec, Transaction};
use gafira_geom::{Length, Unit};

const PAGE: PageId = PageId::new(1);

fn token() -> EditToken {
    EditToken::__new_for_gafira_edit_only()
}

fn size() -> PageSize {
    PageSize {
        width: Length::new(800.0, Unit::Px),
        height: Length::new(600.0, Unit::Px),
    }
}

fn at() -> Placement {
    Placement::new(0.0, 0.0, 10.0, 10.0)
}

fn shape() -> NodeSpec {
    NodeSpec::new(NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect)), at())
}

fn group(children: Vec<NodeSpec>) -> NodeSpec {
    NodeSpec::new(NodeKind::Group(GroupNode {}), at()).with_children(children)
}

/// One empty page (id 1) in a document whose counter stands at `next_id`.
fn empty(next_id: u64) -> Document {
    let page = Arc::new(Page::new(PAGE, "p".into(), size()));
    Document::from_parts(next_id, Dpi::default(), vec![page]).expect("valid")
}

fn apply(command: &mut impl Command, doc: &mut Document) -> Result<Applied, CommandError> {
    command.apply(&mut doc.edit(&token()))
}

fn revert(command: &mut impl Command, doc: &mut Document) -> Result<(), CommandError> {
    command.revert(&mut doc.edit(&token()))
}

fn ids(list: &[u64]) -> Vec<NodeId> {
    list.iter().copied().map(NodeId::new).collect()
}

fn page(doc: &Document) -> &Page {
    doc.page(PAGE).expect("page")
}

/// Fails unless `command` refused with `error` and left `doc` pointer-equal.
fn refused(mut command: impl Command, doc: &Document, error: CommandError) {
    let mut copy = doc.clone();
    assert_eq!(apply(&mut command, &mut copy), Err(error));
    assert_eq!(copy, *doc);
    assert!(Arc::ptr_eq(&copy.pages()[0], &doc.pages()[0]));
}

#[test]
fn insert_then_redo_reuses_ids() {
    let original = empty(10);
    let mut doc = original.clone();
    let mut insert = InsertNodes::new(PAGE, None, 0, vec![group(vec![shape(), shape()])]);
    assert_eq!(apply(&mut insert, &mut doc), Ok(Applied::Changed));
    assert_eq!(page(&doc).roots, ids(&[10]));
    let top = page(&doc).node(NodeId::new(10)).expect("group");
    assert_eq!(top.children, ids(&[11, 12]));
    assert_eq!(doc.next_id(), 13);
    assert_eq!(doc.validate(), Ok(()));

    revert(&mut insert, &mut doc).unwrap();
    assert_eq!(doc, original);
    assert_eq!(apply(&mut insert, &mut doc), Ok(Applied::Changed));
    let first = {
        let mut once = original.clone();
        apply(
            &mut InsertNodes::new(PAGE, None, 0, vec![group(vec![shape(), shape()])]),
            &mut once,
        )
        .unwrap();
        once
    };
    assert_eq!(doc, first);
}

#[test]
fn insert_into_a_group_keeps_sibling_order_and_touches_the_parent() {
    let mut doc = empty(2);
    let mut seed = InsertNodes::new(PAGE, None, 0, vec![group(vec![shape(), shape()])]);
    apply(&mut seed, &mut doc).unwrap();
    // Group is 2, its children 3 and 4.
    let before = doc.clone();
    let mut insert = InsertNodes::new(PAGE, Some(NodeId::new(2)), 1, vec![shape(), shape()]);
    apply(&mut insert, &mut doc).unwrap();
    let parent = page(&doc).node(NodeId::new(2)).unwrap();
    assert_eq!(parent.children, ids(&[3, 5, 6, 4]));
    let touched = insert.touches();
    assert!(touched.pages.is_empty());
    assert_eq!(touched.nodes, ids(&[2, 5, 6]).into_iter().collect());
    revert(&mut insert, &mut doc).unwrap();
    assert_eq!(doc, before);
}

#[test]
fn insert_as_a_root_touches_the_page_and_the_new_nodes() {
    let mut doc = empty(5);
    let mut insert = InsertNodes::new(PAGE, None, 0, vec![shape()]);
    apply(&mut insert, &mut doc).unwrap();
    let touched = insert.touches();
    assert_eq!(touched.pages, [PAGE].into_iter().collect());
    assert_eq!(touched.nodes, ids(&[5]).into_iter().collect());
}

#[test]
fn an_index_above_the_length_is_refused() {
    let mut doc = empty(2);
    apply(
        &mut InsertNodes::new(PAGE, None, 0, vec![shape(), shape()]),
        &mut doc,
    )
    .unwrap();
    let bad = InsertNodes::new(PAGE, None, 3, vec![shape()]);
    refused(
        bad,
        &doc,
        CommandError::IndexOutOfRange { index: 3, len: 2 },
    );
}

#[test]
fn the_end_of_the_list_is_a_valid_index() {
    let mut doc = empty(2);
    apply(
        &mut InsertNodes::new(PAGE, None, 0, vec![shape(), shape()]),
        &mut doc,
    )
    .unwrap();
    apply(
        &mut InsertNodes::new(PAGE, None, 2, vec![shape()]),
        &mut doc,
    )
    .unwrap();
    assert_eq!(page(&doc).roots, ids(&[2, 3, 4]));
}

#[test]
fn a_shape_cannot_be_a_parent() {
    let mut doc = empty(2);
    apply(
        &mut InsertNodes::new(PAGE, None, 0, vec![shape()]),
        &mut doc,
    )
    .unwrap();
    let bad = InsertNodes::new(PAGE, Some(NodeId::new(2)), 0, vec![shape()]);
    refused(bad, &doc, CommandError::NotAContainer(NodeId::new(2)));
}

#[test]
fn a_missing_page_or_parent_is_refused() {
    let doc = empty(2);
    let gone = PageId::new(99);
    refused(
        InsertNodes::new(gone, None, 0, vec![shape()]),
        &doc,
        CommandError::MissingPage(gone),
    );
    let ghost = NodeId::new(77);
    refused(
        InsertNodes::new(PAGE, Some(ghost), 0, vec![shape()]),
        &doc,
        CommandError::MissingNode(ghost),
    );
}

#[test]
fn bad_specs_are_refused_without_a_change() {
    let doc = empty(2);
    refused(
        InsertNodes::new(PAGE, None, 0, vec![]),
        &doc,
        CommandError::InvalidValue("an empty list of nodes"),
    );
    refused(
        InsertNodes::new(PAGE, None, 0, vec![shape().with_children(vec![shape()])]),
        &doc,
        CommandError::InvalidValue("children on a node that is not a group"),
    );
    let mut loud = shape();
    loud.opacity = 1.5;
    let mut copy = doc.clone();
    let result = apply(
        &mut InsertNodes::new(PAGE, None, 0, vec![group(vec![loud])]),
        &mut copy,
    );
    assert!(matches!(
        result,
        Err(CommandError::InvalidDocument(
            ValidationError::OutOfRange { .. }
        ))
    ));
    assert_eq!(copy, doc);
}

#[test]
fn an_exhausted_id_space_changes_nothing() {
    // Only id u64::MAX - 1 can still be issued.
    let doc = empty(u64::MAX - 1);
    refused(
        InsertNodes::new(PAGE, None, 0, vec![shape(), shape()]),
        &doc,
        CommandError::IdSpaceExhausted,
    );
    let mut one = doc.clone();
    assert_eq!(
        apply(
            &mut InsertNodes::new(PAGE, None, 0, vec![shape()]),
            &mut one
        ),
        Ok(Applied::Changed)
    );
    assert_eq!(page(&one).roots, ids(&[u64::MAX - 1]));
}

#[test]
fn a_command_is_reusable_after_a_failed_apply() {
    let mut insert = InsertNodes::new(PAGE, Some(NodeId::new(2)), 0, vec![shape()]);
    let mut doc = empty(2);
    assert_eq!(
        apply(&mut insert, &mut doc),
        Err(CommandError::MissingNode(NodeId::new(2)))
    );
    // Now the parent exists: the same command behaves as if it never failed.
    apply(
        &mut InsertNodes::new(PAGE, None, 0, vec![group(vec![])]),
        &mut doc,
    )
    .unwrap();
    let mut fresh = doc.clone();
    let mut twin = InsertNodes::new(PAGE, Some(NodeId::new(2)), 0, vec![shape()]);
    apply(&mut twin, &mut fresh).unwrap();
    assert_eq!(apply(&mut insert, &mut doc), Ok(Applied::Changed));
    assert_eq!(doc, fresh);
}

#[test]
fn reverting_a_command_that_never_applied_fails() {
    let mut doc = empty(2);
    let mut insert = InsertNodes::new(PAGE, None, 0, vec![shape()]);
    assert_eq!(revert(&mut insert, &mut doc), Err(CommandError::NotApplied));
}

#[test]
fn a_failed_transaction_gives_the_ids_back() {
    let original = empty(10);
    let mut doc = original.clone();
    let ok = InsertNodes::new(PAGE, None, 0, vec![shape()]);
    let bad = InsertNodes::new(PAGE, None, 9, vec![shape()]);
    let mut tx = Transaction::new(MessageKey("test.tx"), vec![Box::new(ok), Box::new(bad)]);
    assert!(apply(&mut tx, &mut doc).is_err());
    assert_eq!(doc, original);
}
