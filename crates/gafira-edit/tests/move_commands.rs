//! `Reorder` and `Reparent` through their public contract.

use std::sync::Arc;

use gafira_doc::msg::MessageKey;
use gafira_doc::{
    Document, Dpi, EditToken, GroupNode, NodeId, NodeKind, Page, PageId, PageSize, Placement,
    ShapeGeometry, ShapeNode,
};
use gafira_edit::{
    Applied, Command, CommandError, InsertNodes, NodeSpec, RemoveNodes, Reorder, Reparent,
    Transaction,
};
use gafira_geom::{Length, Unit};

const PAGE: PageId = PageId::new(1);

fn token() -> EditToken {
    EditToken::__new_for_gafira_edit_only()
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

fn n(id: u64) -> NodeId {
    NodeId::new(id)
}

fn ids(list: &[u64]) -> Vec<NodeId> {
    list.iter().copied().map(NodeId::new).collect()
}

const A: u64 = 2;
const G: u64 = 3;
const C: u64 = 4;
const H: u64 = 5;
const D: u64 = 6;
const B: u64 = 7;
const E: u64 = 8;

/// Roots `[a, g, b, e]`; `g` holds `[c, h]`, `h` holds `[d]`, `e` is empty.
fn tree() -> Document {
    let size = PageSize {
        width: Length::new(800.0, Unit::Px),
        height: Length::new(600.0, Unit::Px),
    };
    let page = Arc::new(Page::new(PAGE, "p".into(), size));
    let mut doc = Document::from_parts(2, Dpi::default(), vec![page]).expect("valid");
    let specs = vec![
        shape(),
        group(vec![shape(), group(vec![shape()])]),
        shape(),
        group(vec![]),
    ];
    apply(&mut InsertNodes::new(PAGE, None, 0, specs), &mut doc).unwrap();
    assert_eq!(page_of(&doc).roots, ids(&[A, G, B, E]));
    doc
}

fn apply(command: &mut impl Command, doc: &mut Document) -> Result<Applied, CommandError> {
    command.apply(&mut doc.edit(&token()))
}

fn revert(command: &mut impl Command, doc: &mut Document) -> Result<(), CommandError> {
    command.revert(&mut doc.edit(&token()))
}

fn page_of(doc: &Document) -> &Page {
    doc.page(PAGE).expect("page")
}

fn children(doc: &Document, parent: u64) -> Vec<NodeId> {
    page_of(doc).node(n(parent)).expect("node").children.clone()
}

/// Fails unless `command` refused with `error` and left `doc` pointer-equal.
fn refused(mut command: impl Command, doc: &Document, error: CommandError) {
    let mut copy = doc.clone();
    assert_eq!(apply(&mut command, &mut copy), Err(error));
    assert_eq!(copy, *doc);
    assert!(Arc::ptr_eq(&copy.pages()[0], &doc.pages()[0]));
}

#[test]
fn bring_to_front_and_back() {
    let original = tree();
    let mut doc = original.clone();
    let mut reorder = Reorder::new(PAGE, n(A), 3);
    assert_eq!(apply(&mut reorder, &mut doc), Ok(Applied::Changed));
    assert_eq!(page_of(&doc).roots, ids(&[G, B, E, A]));
    assert_eq!(doc.validate(), Ok(()));
    revert(&mut reorder, &mut doc).unwrap();
    assert_eq!(doc, original);
}

#[test]
fn reordering_inside_a_group_touches_the_group_and_not_the_page() {
    let original = tree();
    let mut doc = original.clone();
    let mut reorder = Reorder::new(PAGE, n(H), 0);
    assert_eq!(apply(&mut reorder, &mut doc), Ok(Applied::Changed));
    assert_eq!(children(&doc, G), ids(&[H, C]));
    let touched = reorder.touches();
    assert!(touched.pages.is_empty());
    assert_eq!(touched.nodes, ids(&[G, H]).into_iter().collect());
    revert(&mut reorder, &mut doc).unwrap();
    assert_eq!(doc, original);
}

#[test]
fn reordering_a_root_touches_the_page() {
    let mut doc = tree();
    let mut reorder = Reorder::new(PAGE, n(B), 0);
    apply(&mut reorder, &mut doc).unwrap();
    let touched = reorder.touches();
    assert_eq!(touched.pages, [PAGE].into_iter().collect());
    assert_eq!(touched.nodes, ids(&[B]).into_iter().collect());
}

#[test]
fn the_same_index_is_unchanged() {
    let original = tree();
    let mut doc = original.clone();
    let mut reorder = Reorder::new(PAGE, n(G), 1);
    assert_eq!(apply(&mut reorder, &mut doc), Ok(Applied::Unchanged));
    assert!(Arc::ptr_eq(&doc.pages()[0], &original.pages()[0]));
    assert!(reorder.touches().pages.is_empty() && reorder.touches().nodes.is_empty());
    assert_eq!(revert(&mut reorder, &mut doc), Ok(()));
    assert_eq!(doc, original);
}

#[test]
fn reverting_a_move_that_changed_nothing_is_a_no_op_whatever_happened_since() {
    let mut doc = tree();
    let mut reorder = Reorder::new(PAGE, n(A), 0);
    assert_eq!(apply(&mut reorder, &mut doc), Ok(Applied::Unchanged));
    doc.edit(&token()).pages_mut().clear();
    assert_eq!(revert(&mut reorder, &mut doc), Ok(()));
}

#[test]
fn a_reorder_index_must_be_below_the_sibling_count() {
    let doc = tree();
    refused(
        Reorder::new(PAGE, n(A), 4),
        &doc,
        CommandError::IndexOutOfRange { index: 4, len: 4 },
    );
    refused(
        Reorder::new(PAGE, n(C), 2),
        &doc,
        CommandError::IndexOutOfRange { index: 2, len: 2 },
    );
}

#[test]
fn a_reorder_of_a_missing_page_or_node_is_refused() {
    let doc = tree();
    refused(
        Reorder::new(PageId::new(9), n(A), 0),
        &doc,
        CommandError::MissingPage(PageId::new(9)),
    );
    refused(
        Reorder::new(PAGE, n(99), 0),
        &doc,
        CommandError::MissingNode(n(99)),
    );
}

#[test]
fn a_reorder_is_reusable_after_a_failed_apply() {
    let mut command = Reorder::new(PAGE, n(A), 3);
    let mut bare = tree();
    apply(&mut RemoveNodes::new(PAGE, vec![n(B), n(E)]), &mut bare).unwrap();
    assert_eq!(
        apply(&mut command, &mut bare),
        Err(CommandError::IndexOutOfRange { index: 3, len: 2 })
    );
    let mut doc = tree();
    let mut fresh = doc.clone();
    apply(&mut Reorder::new(PAGE, n(A), 3), &mut fresh).unwrap();
    assert_eq!(apply(&mut command, &mut doc), Ok(Applied::Changed));
    assert_eq!(doc, fresh);
}

#[test]
fn reverting_a_reorder_that_never_applied_fails() {
    let mut doc = tree();
    let mut reorder = Reorder::new(PAGE, n(A), 3);
    assert_eq!(
        revert(&mut reorder, &mut doc),
        Err(CommandError::NotApplied)
    );
}

#[test]
fn a_reorder_revert_over_a_changed_list_fails_and_changes_nothing() {
    let mut doc = tree();
    let mut reorder = Reorder::new(PAGE, n(A), 3);
    apply(&mut reorder, &mut doc).unwrap();
    apply(&mut Reorder::new(PAGE, n(B), 0), &mut doc).unwrap();
    let before = doc.clone();
    assert_eq!(
        revert(&mut reorder, &mut doc),
        Err(CommandError::InvalidValue("the lists changed since apply"))
    );
    assert_eq!(doc, before);
}

#[test]
fn a_reorder_revert_whose_group_is_gone_fails_and_changes_nothing() {
    let mut doc = tree();
    let mut reorder = Reorder::new(PAGE, n(H), 0);
    apply(&mut reorder, &mut doc).unwrap();
    apply(&mut RemoveNodes::new(PAGE, vec![n(G)]), &mut doc).unwrap();
    let before = doc.clone();
    assert_eq!(
        revert(&mut reorder, &mut doc),
        Err(CommandError::MissingNode(n(G)))
    );
    assert_eq!(doc, before);
}

#[test]
fn a_revert_whose_page_is_gone_fails_and_changes_nothing() {
    let mut doc = tree();
    let mut reorder = Reorder::new(PAGE, n(A), 3);
    apply(&mut reorder, &mut doc).unwrap();
    doc.edit(&token()).pages_mut().clear();
    let before = doc.clone();
    assert_eq!(
        revert(&mut reorder, &mut doc),
        Err(CommandError::MissingPage(PAGE))
    );
    assert_eq!(doc, before);
}

#[test]
fn group_two_nodes() {
    let original = tree();
    let mut doc = original.clone();
    let mut reparent = Reparent::new(PAGE, vec![n(A), n(B)], Some(n(E)), 0);
    assert_eq!(apply(&mut reparent, &mut doc), Ok(Applied::Changed));
    assert_eq!(page_of(&doc).roots, ids(&[G, E]));
    assert_eq!(children(&doc, E), ids(&[A, B]));
    assert_eq!(doc.validate(), Ok(()));
    revert(&mut reparent, &mut doc).unwrap();
    assert_eq!(doc, original);
}

#[test]
fn moved_nodes_keep_their_document_order_whatever_the_list_order() {
    let mut doc = tree();
    // `d` sits deep inside `g`, `a` and `b` are roots: document order is a, d, b.
    let mut reparent = Reparent::new(PAGE, vec![n(B), n(D), n(A)], Some(n(E)), 0);
    apply(&mut reparent, &mut doc).unwrap();
    assert_eq!(children(&doc, E), ids(&[A, D, B]));
    assert_eq!(doc.validate(), Ok(()));
}

#[test]
fn nodes_leave_every_old_parent_and_come_back_to_it() {
    let original = tree();
    let mut doc = original.clone();
    let mut reparent = Reparent::new(PAGE, vec![n(C), n(D)], None, 1);
    apply(&mut reparent, &mut doc).unwrap();
    assert_eq!(page_of(&doc).roots, ids(&[A, C, D, G, B, E]));
    assert_eq!(children(&doc, G), ids(&[H]));
    assert!(children(&doc, H).is_empty());
    assert_eq!(doc.validate(), Ok(()));
    revert(&mut reparent, &mut doc).unwrap();
    assert_eq!(doc, original);
}

#[test]
fn the_index_counts_the_target_list_without_the_moved_nodes() {
    let doc = tree();
    // Roots without `a` and `b` are `[g, e]`: 2 is the end, 3 is past it.
    let mut ok = doc.clone();
    apply(&mut Reparent::new(PAGE, vec![n(A), n(B)], None, 2), &mut ok).unwrap();
    assert_eq!(page_of(&ok).roots, ids(&[G, E, A, B]));
    refused(
        Reparent::new(PAGE, vec![n(A), n(B)], None, 3),
        &doc,
        CommandError::IndexOutOfRange { index: 3, len: 2 },
    );
}

#[test]
fn a_node_listed_with_its_descendant_moves_once_as_the_ancestor() {
    let mut doc = tree();
    let mut reparent = Reparent::new(PAGE, vec![n(D), n(G), n(H)], Some(n(E)), 0);
    apply(&mut reparent, &mut doc).unwrap();
    assert_eq!(page_of(&doc).roots, ids(&[A, B, E]));
    assert_eq!(children(&doc, E), ids(&[G]));
    assert_eq!(children(&doc, H), ids(&[D]));
    assert_eq!(doc.validate(), Ok(()));
}

#[test]
fn a_move_that_changes_nothing_is_unchanged() {
    let original = tree();
    let mut doc = original.clone();
    let mut reparent = Reparent::new(PAGE, vec![n(G), n(B)], None, 1);
    assert_eq!(apply(&mut reparent, &mut doc), Ok(Applied::Unchanged));
    assert!(Arc::ptr_eq(&doc.pages()[0], &original.pages()[0]));
    assert!(reparent.touches().pages.is_empty() && reparent.touches().nodes.is_empty());
    assert_eq!(revert(&mut reparent, &mut doc), Ok(()));
    assert_eq!(doc, original);
}

#[test]
fn a_group_cannot_move_under_itself_or_a_descendant() {
    let doc = tree();
    refused(
        Reparent::new(PAGE, vec![n(G)], Some(n(H)), 0),
        &doc,
        CommandError::WouldCreateCycle(n(G)),
    );
    refused(
        Reparent::new(PAGE, vec![n(G)], Some(n(G)), 0),
        &doc,
        CommandError::WouldCreateCycle(n(G)),
    );
    // One of several nodes is the culprit; another one is harmless.
    refused(
        Reparent::new(PAGE, vec![n(A), n(H)], Some(n(H)), 0),
        &doc,
        CommandError::WouldCreateCycle(n(H)),
    );
}

#[test]
fn a_shape_is_not_a_parent() {
    let doc = tree();
    refused(
        Reparent::new(PAGE, vec![n(B)], Some(n(A)), 0),
        &doc,
        CommandError::NotAContainer(n(A)),
    );
}

#[test]
fn missing_pages_nodes_parents_and_empty_lists_are_refused() {
    let doc = tree();
    refused(
        Reparent::new(PageId::new(9), vec![n(A)], None, 0),
        &doc,
        CommandError::MissingPage(PageId::new(9)),
    );
    refused(
        Reparent::new(PAGE, vec![n(A), n(99)], None, 0),
        &doc,
        CommandError::MissingNode(n(99)),
    );
    refused(
        Reparent::new(PAGE, vec![n(A)], Some(n(99)), 0),
        &doc,
        CommandError::MissingNode(n(99)),
    );
    refused(
        Reparent::new(PAGE, vec![], None, 0),
        &doc,
        CommandError::InvalidValue("an empty list of nodes"),
    );
}

#[test]
fn a_reparent_touches_both_parents_and_the_moved_nodes() {
    let mut doc = tree();
    let mut reparent = Reparent::new(PAGE, vec![n(C)], Some(n(E)), 0);
    apply(&mut reparent, &mut doc).unwrap();
    let touched = reparent.touches();
    assert!(touched.pages.is_empty());
    assert_eq!(touched.nodes, ids(&[G, E, C]).into_iter().collect());

    let mut to_roots = Reparent::new(PAGE, vec![n(D)], None, 0);
    apply(&mut to_roots, &mut doc).unwrap();
    let touched = to_roots.touches();
    assert_eq!(touched.pages, [PAGE].into_iter().collect());
    assert_eq!(touched.nodes, ids(&[H, D]).into_iter().collect());
}

#[test]
fn a_reparent_is_reusable_after_a_failed_apply() {
    let mut command = Reparent::new(PAGE, vec![n(A)], Some(n(E)), 0);
    let mut bare = tree();
    apply(&mut RemoveNodes::new(PAGE, vec![n(E)]), &mut bare).unwrap();
    assert_eq!(
        apply(&mut command, &mut bare),
        Err(CommandError::MissingNode(n(E)))
    );
    let mut doc = tree();
    let mut fresh = doc.clone();
    apply(
        &mut Reparent::new(PAGE, vec![n(A)], Some(n(E)), 0),
        &mut fresh,
    )
    .unwrap();
    assert_eq!(apply(&mut command, &mut doc), Ok(Applied::Changed));
    assert_eq!(doc, fresh);
}

#[test]
fn reverting_a_reparent_that_never_applied_fails() {
    let mut doc = tree();
    let mut reparent = Reparent::new(PAGE, vec![n(A)], Some(n(E)), 0);
    assert_eq!(
        revert(&mut reparent, &mut doc),
        Err(CommandError::NotApplied)
    );
}

#[test]
fn a_reparent_revert_over_a_changed_list_fails_and_changes_nothing() {
    let mut doc = tree();
    let mut reparent = Reparent::new(PAGE, vec![n(A)], Some(n(E)), 0);
    apply(&mut reparent, &mut doc).unwrap();
    apply(&mut Reorder::new(PAGE, n(B), 0), &mut doc).unwrap();
    let before = doc.clone();
    assert_eq!(
        revert(&mut reparent, &mut doc),
        Err(CommandError::InvalidValue("the lists changed since apply"))
    );
    assert_eq!(doc, before);
}

#[test]
fn a_reparent_revert_whose_old_parent_is_gone_fails_and_changes_nothing() {
    let mut doc = tree();
    let mut reparent = Reparent::new(PAGE, vec![n(D)], Some(n(E)), 0);
    apply(&mut reparent, &mut doc).unwrap();
    apply(&mut RemoveNodes::new(PAGE, vec![n(H)]), &mut doc).unwrap();
    let before = doc.clone();
    assert_eq!(
        revert(&mut reparent, &mut doc),
        Err(CommandError::MissingNode(n(H)))
    );
    assert_eq!(doc, before);
}

#[test]
fn a_failed_transaction_undoes_a_reparent() {
    let original = tree();
    let mut doc = original.clone();
    let ok = Reparent::new(PAGE, vec![n(A)], Some(n(E)), 0);
    let bad = Reorder::new(PAGE, n(B), 9);
    let mut tx = Transaction::new(MessageKey("test.tx"), vec![Box::new(ok), Box::new(bad)]);
    assert!(apply(&mut tx, &mut doc).is_err());
    assert_eq!(doc, original);
}
