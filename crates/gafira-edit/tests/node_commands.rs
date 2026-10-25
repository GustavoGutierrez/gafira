//! `InsertNodes` and `RemoveNodes` through their public contract.

use std::sync::Arc;
use std::time::Duration;

use gafira_doc::msg::MessageKey;
use gafira_doc::{
    Document, Dpi, EditToken, GroupNode, NodeId, NodeKind, Page, PageId, PageSize, Placement,
    ShapeGeometry, ShapeNode, ValidationError,
};
use gafira_edit::{
    Applied, Command, CommandError, History, InsertNodes, NodeSpec, RemoveNodes, Transaction,
};
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

fn n(id: u64) -> NodeId {
    NodeId::new(id)
}

/// Roots `[a, g, b]` = `[2, 3, 6]`; group `g` has children `[c, d]` = `[4, 5]`.
fn tree() -> Document {
    let mut doc = empty(2);
    let specs = vec![shape(), group(vec![shape(), shape()]), shape()];
    apply(&mut InsertNodes::new(PAGE, None, 0, specs), &mut doc).unwrap();
    assert_eq!(page(&doc).roots, ids(&[2, 3, 6]));
    doc
}

#[test]
fn remove_a_subtree_and_restore_the_order() {
    let original = tree();
    let mut doc = original.clone();
    let mut remove = RemoveNodes::new(PAGE, vec![n(3), n(5)]);
    assert_eq!(apply(&mut remove, &mut doc), Ok(Applied::Changed));
    assert_eq!(page(&doc).roots, ids(&[2, 6]));
    let left: Vec<NodeId> = page(&doc).nodes.keys().copied().collect();
    assert_eq!(left, ids(&[2, 6]));
    assert_eq!(doc.validate(), Ok(()));

    revert(&mut remove, &mut doc).unwrap();
    assert_eq!(doc, original);
    assert_eq!(page(&doc).roots, ids(&[2, 3, 6]));
    assert_eq!(page(&doc).node(n(3)).unwrap().children, ids(&[4, 5]));
}

#[test]
fn listing_a_node_and_its_descendant_is_listing_the_ancestor() {
    let alone = {
        let mut doc = tree();
        let mut remove = RemoveNodes::new(PAGE, vec![n(3)]);
        apply(&mut remove, &mut doc).unwrap();
        (doc, remove.touches())
    };
    for list in [vec![n(3), n(5)], vec![n(5), n(3)], vec![n(3), n(3)]] {
        let mut doc = tree();
        let mut remove = RemoveNodes::new(PAGE, list);
        apply(&mut remove, &mut doc).unwrap();
        assert_eq!(doc, alone.0);
        assert_eq!(remove.touches(), alone.1);
    }
}

#[test]
fn siblings_return_to_their_exact_positions() {
    let mut doc = empty(2);
    let kids = vec![shape(), shape(), shape(), shape()];
    apply(
        &mut InsertNodes::new(PAGE, None, 0, vec![group(kids)]),
        &mut doc,
    )
    .unwrap();
    // Group 2, children 3..=6.
    let original = doc.clone();
    for list in [vec![n(3), n(5)], vec![n(5), n(3)], vec![n(3), n(4), n(6)]] {
        let mut copy = original.clone();
        let mut remove = RemoveNodes::new(PAGE, list.clone());
        apply(&mut remove, &mut copy).unwrap();
        let kept: Vec<NodeId> = [3, 4, 5, 6]
            .into_iter()
            .map(n)
            .filter(|id| !list.contains(id))
            .collect();
        assert_eq!(page(&copy).node(n(2)).unwrap().children, kept);
        revert(&mut remove, &mut copy).unwrap();
        assert_eq!(copy, original, "{list:?}");
    }
}

#[test]
fn removal_touches_the_parent_and_the_removed_nodes() {
    let mut doc = tree();
    let mut inner = RemoveNodes::new(PAGE, vec![n(4)]);
    apply(&mut inner, &mut doc).unwrap();
    let touched = inner.touches();
    assert!(touched.pages.is_empty());
    assert_eq!(touched.nodes, ids(&[3, 4]).into_iter().collect());

    let mut roots = RemoveNodes::new(PAGE, vec![n(3)]);
    apply(&mut roots, &mut doc).unwrap();
    let touched = roots.touches();
    assert_eq!(touched.pages, [PAGE].into_iter().collect());
    assert_eq!(touched.nodes, ids(&[3, 5]).into_iter().collect());
}

#[test]
fn a_missing_node_page_or_list_is_refused() {
    let doc = tree();
    refused(
        RemoveNodes::new(PAGE, vec![n(2), n(77)]),
        &doc,
        CommandError::MissingNode(n(77)),
    );
    refused(
        RemoveNodes::new(PageId::new(9), vec![n(2)]),
        &doc,
        CommandError::MissingPage(PageId::new(9)),
    );
    refused(
        RemoveNodes::new(PAGE, vec![]),
        &doc,
        CommandError::InvalidValue("an empty list of nodes"),
    );
}

#[test]
fn a_remove_command_is_reusable_after_a_failed_apply() {
    let mut remove = RemoveNodes::new(PAGE, vec![n(3)]);
    let mut bare = empty(2);
    assert_eq!(
        apply(&mut remove, &mut bare),
        Err(CommandError::MissingNode(n(3)))
    );
    let mut doc = tree();
    let mut fresh = doc.clone();
    apply(&mut RemoveNodes::new(PAGE, vec![n(3)]), &mut fresh).unwrap();
    assert_eq!(apply(&mut remove, &mut doc), Ok(Applied::Changed));
    assert_eq!(doc, fresh);
    revert(&mut remove, &mut doc).unwrap();
    assert_eq!(doc, tree());
}

#[test]
fn reverting_a_remove_that_never_applied_fails() {
    let mut doc = tree();
    let mut remove = RemoveNodes::new(PAGE, vec![n(3)]);
    assert_eq!(revert(&mut remove, &mut doc), Err(CommandError::NotApplied));
}

#[test]
fn positions_follow_the_list_not_the_id_order() {
    // Inserting at the front makes the roots `[4, 3, 2]`: ids run against positions.
    let mut doc = empty(2);
    for _ in 0..3 {
        apply(
            &mut InsertNodes::new(PAGE, None, 0, vec![shape()]),
            &mut doc,
        )
        .unwrap();
    }
    assert_eq!(page(&doc).roots, ids(&[4, 3, 2]));
    let original = doc.clone();
    let mut remove = RemoveNodes::new(PAGE, vec![n(2), n(4)]);
    apply(&mut remove, &mut doc).unwrap();
    assert_eq!(page(&doc).roots, ids(&[3]));
    revert(&mut remove, &mut doc).unwrap();
    assert_eq!(doc, original);
}

#[test]
fn a_revert_that_cannot_find_its_place_fails_and_changes_nothing() {
    let mut doc = tree();
    let mut remove = RemoveNodes::new(PAGE, vec![n(6)]);
    apply(&mut remove, &mut doc).unwrap();
    apply(&mut RemoveNodes::new(PAGE, vec![n(2), n(3)]), &mut doc).unwrap();
    let before = doc.clone();
    assert_eq!(
        revert(&mut remove, &mut doc),
        Err(CommandError::IndexOutOfRange { index: 2, len: 0 })
    );
    assert_eq!(doc, before);
}

#[test]
fn an_id_is_reissued_after_an_undo() {
    let mut history = History::new(empty(10)).expect("valid");
    let insert = || Box::new(InsertNodes::new(PAGE, None, 0, vec![shape()]));
    assert_eq!(
        history.execute(insert(), Duration::ZERO),
        Ok(Applied::Changed)
    );
    assert_eq!(page(history.document()).roots, ids(&[10]));
    assert!(history.undo().unwrap().is_some());
    assert_eq!(history.document().next_id(), 10);
    assert_eq!(
        history.execute(insert(), Duration::ZERO),
        Ok(Applied::Changed)
    );
    assert_eq!(page(history.document()).roots, ids(&[10]));
    assert_eq!(history.document().validate(), Ok(()));
}

#[test]
fn a_revert_whose_parent_is_gone_fails_and_changes_nothing() {
    let mut doc = tree();
    let mut inner = RemoveNodes::new(PAGE, vec![n(4)]);
    apply(&mut inner, &mut doc).unwrap();
    apply(&mut RemoveNodes::new(PAGE, vec![n(3)]), &mut doc).unwrap();
    let before = doc.clone();
    assert_eq!(
        revert(&mut inner, &mut doc),
        Err(CommandError::MissingNode(n(3)))
    );
    assert_eq!(doc, before);
}

#[test]
fn a_revert_of_nodes_that_are_already_back_fails_and_changes_nothing() {
    let mut doc = tree();
    let mut remove = RemoveNodes::new(PAGE, vec![n(5)]);
    apply(&mut remove, &mut doc).unwrap();
    revert(&mut remove, &mut doc).unwrap();
    let before = doc.clone();
    assert_eq!(
        revert(&mut remove, &mut doc),
        Err(CommandError::InvalidValue("a removed node is already back"))
    );
    assert_eq!(doc, before);
    assert_eq!(doc.validate(), Ok(()));
}
