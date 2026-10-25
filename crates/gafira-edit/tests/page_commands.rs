//! `SetProperty` on pages and the DPI, `InsertPage` and `RemovePage` through
//! their public contract (Q4).

use std::sync::Arc;
use std::time::Duration;

use gafira_doc::msg::MessageKey;
use gafira_doc::{
    Axis, Document, Dpi, EditToken, NodeId, NodeKind, Page, PageId, PageSize, PageSizeError,
    Placement, ShapeGeometry, ShapeNode, ValidationError,
};
use gafira_edit::{
    Applied, Command, CommandError, GestureId, History, InsertNodes, InsertPage, MergeKey,
    NodeSpec, Property, RemovePage, SetProperty, Transaction,
};
use gafira_geom::{Length, Unit};

const P1: PageId = PageId::new(1);
const P2: PageId = PageId::new(2);

fn token() -> EditToken {
    EditToken::__new_for_gafira_edit_only()
}

fn px(w: f64, h: f64) -> PageSize {
    PageSize {
        width: Length::new(w, Unit::Px),
        height: Length::new(h, Unit::Px),
    }
}

/// ISO A0: 841 x 1189 mm.
fn a0() -> PageSize {
    PageSize {
        width: Length::new(841.0, Unit::Mm),
        height: Length::new(1189.0, Unit::Mm),
    }
}

fn page(id: PageId, name: &str, size: PageSize) -> Arc<Page> {
    Arc::new(Page::new(id, name.into(), size))
}

/// Page 1 (800 x 600 px) and page 2 (A0) at `dpi`; the counter stands at 3.
fn two_pages(dpi: u32) -> Document {
    let pages = vec![page(P1, "one", px(800.0, 600.0)), page(P2, "two", a0())];
    Document::from_parts(3, Dpi::new(dpi), pages).expect("valid")
}

fn apply(command: &mut impl Command, doc: &mut Document) -> Result<Applied, CommandError> {
    command.apply(&mut doc.edit(&token()))
}

fn revert(command: &mut impl Command, doc: &mut Document) -> Result<(), CommandError> {
    command.revert(&mut doc.edit(&token()))
}

/// Fails unless `command` refused with `error` and left `doc` pointer-equal.
fn refused(mut command: impl Command, doc: &Document, error: CommandError) {
    let mut copy = doc.clone();
    assert_eq!(apply(&mut command, &mut copy), Err(error));
    assert_eq!(copy, *doc);
    assert!(
        copy.pages()
            .iter()
            .zip(doc.pages())
            .all(|(a, b)| Arc::ptr_eq(a, b))
    );
}

/// Fails unless `command` refused on `page`, `axis` being out of range, and
/// left `doc` pointer-equal.
fn refused_size(mut command: impl Command, doc: &Document, page: PageId, axis: Axis) {
    let mut copy = doc.clone();
    let result = apply(&mut command, &mut copy);
    assert!(
        matches!(
            result,
            Err(CommandError::InvalidDocument(ValidationError::PageSize {
                page: p,
                error: PageSizeError::OutOfRange { axis: a, .. },
            })) if p == page && a == axis
        ),
        "{result:?}"
    );
    assert_eq!(copy, *doc);
    assert!(
        copy.pages()
            .iter()
            .zip(doc.pages())
            .all(|(a, b)| Arc::ptr_eq(a, b))
    );
}

#[test]
fn every_page_field_is_set_and_restored() {
    let values = [
        Property::PageName("renamed".into()),
        Property::PageSize(px(1024.0, 768.0)),
        Property::PageLocked(true),
        Property::PageHidden(true),
    ];
    for value in values {
        let original = two_pages(96);
        let mut doc = original.clone();
        let mut command = SetProperty::page(P1, value.clone(), None);
        assert_eq!(apply(&mut command, &mut doc), Ok(Applied::Changed));
        let (before, after) = (original.page(P1).unwrap(), doc.page(P1).unwrap());
        assert_ne!(before, after, "{value:?}");
        assert!(Arc::ptr_eq(&original.pages()[1], &doc.pages()[1]));
        assert_eq!(doc.validate(), Ok(()));
        let touched = command.touches();
        assert_eq!(touched.pages, [P1].into_iter().collect());
        assert!(touched.nodes.is_empty());
        revert(&mut command, &mut doc).unwrap();
        assert_eq!(doc, original, "{value:?}");
    }
}

#[test]
fn a_page_field_takes_the_value_given() {
    let mut doc = two_pages(96);
    let mut rename = SetProperty::page(P1, Property::PageName("x".into()), None);
    apply(&mut rename, &mut doc).unwrap();
    apply(
        &mut SetProperty::page(P1, Property::PageSize(px(10.0, 20.0)), None),
        &mut doc,
    )
    .unwrap();
    apply(
        &mut SetProperty::page(P1, Property::PageLocked(true), None),
        &mut doc,
    )
    .unwrap();
    apply(
        &mut SetProperty::page(P1, Property::PageHidden(true), None),
        &mut doc,
    )
    .unwrap();
    let page = doc.page(P1).unwrap();
    assert_eq!(page.name, "x");
    assert_eq!(page.size, px(10.0, 20.0));
    assert!(page.locked && page.hidden);
    assert_eq!(doc.page(P2).unwrap().name, "two");
}

#[test]
fn the_same_page_value_is_unchanged_and_shares_the_page() {
    let original = two_pages(96);
    let same = [
        Property::PageName("one".into()),
        Property::PageSize(px(800.0, 600.0)),
        Property::PageLocked(false),
        Property::PageHidden(false),
    ];
    for value in same {
        let mut doc = original.clone();
        let mut command = SetProperty::page(P1, value, None);
        assert_eq!(apply(&mut command, &mut doc), Ok(Applied::Unchanged));
        assert!(Arc::ptr_eq(&doc.pages()[0], &original.pages()[0]));
        revert(&mut command, &mut doc).unwrap();
        assert!(Arc::ptr_eq(&doc.pages()[0], &original.pages()[0]));
    }
}

#[test]
fn a_locked_page_is_still_editable() {
    let mut doc = two_pages(96);
    apply(
        &mut SetProperty::page(P1, Property::PageLocked(true), None),
        &mut doc,
    )
    .unwrap();
    let rename = Property::PageName("still editable".into());
    assert_eq!(
        apply(&mut SetProperty::page(P1, rename, None), &mut doc),
        Ok(Applied::Changed)
    );
}

#[test]
fn a_dpi_that_pushes_a_page_over_the_limit_is_refused() {
    let original = two_pages(300);
    let over = SetProperty::document(Property::Dpi(Dpi::new(351)), None);
    refused_size(over, &original, P2, Axis::Height);
    assert_eq!(original.dpi(), Dpi::new(300));
    // 350 dpi still fits (16 383.9 px), and a revert gives 300 back.
    let mut doc = original.clone();
    let mut fits = SetProperty::document(Property::Dpi(Dpi::new(350)), None);
    assert_eq!(apply(&mut fits, &mut doc), Ok(Applied::Changed));
    assert_eq!(doc.dpi(), Dpi::new(350));
    assert_eq!(doc.validate(), Ok(()));
    revert(&mut fits, &mut doc).unwrap();
    assert_eq!(doc, original);
}

#[test]
fn a_dpi_is_checked_against_every_page() {
    // Page 1 is in pixels and always fits; only page 2 (the last) fails.
    let command = SetProperty::document(Property::Dpi(Dpi::new(10_000)), None);
    refused_size(command, &two_pages(96), P2, Axis::Width);
}

#[test]
fn the_dpi_range_edges() {
    let pixels = Document::from_parts(2, Dpi::new(96), vec![page(P1, "p", px(800.0, 600.0))])
        .expect("valid");
    for dpi in [Dpi::MIN, Dpi::MAX] {
        let mut doc = pixels.clone();
        let mut command = SetProperty::document(Property::Dpi(Dpi::new(dpi)), None);
        assert_eq!(apply(&mut command, &mut doc), Ok(Applied::Changed));
        assert_eq!(doc.dpi(), Dpi::new(dpi));
        assert_eq!(doc.validate(), Ok(()));
    }
    for dpi in [0, Dpi::MAX + 1, u32::MAX] {
        let command = SetProperty::document(Property::Dpi(Dpi::new(dpi)), None);
        refused(command, &pixels, CommandError::InvalidValue("dpi"));
    }
}

#[test]
fn the_same_dpi_is_unchanged() {
    let original = two_pages(300);
    let mut doc = original.clone();
    let mut command = SetProperty::document(Property::Dpi(Dpi::new(300)), None);
    assert_eq!(apply(&mut command, &mut doc), Ok(Applied::Unchanged));
    assert_eq!(doc, original);
    revert(&mut command, &mut doc).unwrap();
    assert_eq!(doc, original);
}

#[test]
fn a_page_size_outside_the_pixel_limits_is_refused() {
    let doc = two_pages(96);
    let wide = PageSize {
        width: Length::new(200.0, Unit::In),
        height: Length::new(10.0, Unit::In),
    };
    let rows = [
        (px(0.5, 100.0), Axis::Width),
        (px(100.0, 0.5), Axis::Height),
        (px(100.0, 16_385.0), Axis::Height),
        (wide, Axis::Width),
    ];
    for (size, axis) in rows {
        let command = SetProperty::page(P1, Property::PageSize(size), None);
        refused_size(command, &doc, P1, axis);
    }
    let mut nan = SetProperty::page(P1, Property::PageSize(px(f64::NAN, 10.0)), None);
    let mut copy = doc.clone();
    assert!(matches!(
        apply(&mut nan, &mut copy),
        Err(CommandError::InvalidDocument(ValidationError::PageSize {
            error: PageSizeError::Conversion { .. },
            ..
        }))
    ));
    assert_eq!(copy, doc);
}

#[test]
fn a_missing_page_is_refused() {
    let doc = two_pages(96);
    let ghost = PageId::new(9);
    let command = SetProperty::page(ghost, Property::PageLocked(true), None);
    refused(command, &doc, CommandError::MissingPage(ghost));
}

#[test]
fn a_page_property_cannot_name_nodes() {
    let doc = two_pages(96);
    let mut command = SetProperty::new(
        P1,
        vec![NodeId::new(5)],
        Property::PageName("x".into()),
        None,
    );
    let mut copy = doc.clone();
    assert!(matches!(
        apply(&mut command, &mut copy),
        Err(CommandError::InvalidValue(_))
    ));
    assert_eq!(copy, doc);
}

#[test]
fn only_a_page_property_works_without_nodes() {
    // A node property through the page constructor has no target.
    let doc = two_pages(96);
    let mut command = SetProperty::page(P1, Property::Visible(false), None);
    let mut copy = doc.clone();
    assert!(matches!(
        apply(&mut command, &mut copy),
        Err(CommandError::InvalidValue(_))
    ));
    assert_eq!(copy, doc);
}

#[test]
fn a_dpi_change_touches_no_page_and_a_page_change_touches_its_page() {
    let mut doc = two_pages(96);
    let mut dpi = SetProperty::document(Property::Dpi(Dpi::new(100)), None);
    let mut rename = SetProperty::page(P2, Property::PageName("n".into()), None);
    for command in [&mut dpi, &mut rename] {
        apply(command, &mut doc).unwrap();
    }
    assert!(dpi.touches().pages.is_empty() && dpi.touches().nodes.is_empty());
    assert_eq!(rename.touches().pages, [P2].into_iter().collect());
}

#[test]
fn the_merge_key_names_the_page_field_or_the_document() {
    let rows = [
        (Property::PageName(String::new()), "set_property.page_name"),
        (Property::PageSize(px(1.0, 1.0)), "set_property.page_size"),
        (Property::PageLocked(true), "set_property.page_locked"),
        (Property::PageHidden(true), "set_property.page_hidden"),
    ];
    for (value, kind) in rows {
        let key = SetProperty::page(P2, value, Some(GestureId(3))).merge_key();
        assert_eq!(
            key,
            Some(MergeKey {
                kind,
                page: Some(P2),
                nodes: vec![],
                gesture: Some(GestureId(3)),
            })
        );
    }
    let dpi = SetProperty::document(Property::Dpi(Dpi::new(72)), None).merge_key();
    assert_eq!(
        dpi,
        Some(MergeKey {
            kind: "set_property.dpi",
            page: None,
            nodes: vec![],
            gesture: None,
        })
    );
}

#[test]
fn a_merged_resize_drag_keeps_the_origin_and_the_last_value() {
    let original = two_pages(96);
    let step = |w: f64| SetProperty::page(P1, Property::PageSize(px(w, 600.0)), Some(GestureId(1)));
    let mut doc = original.clone();
    let (mut first, mut second, mut third) = (step(900.0), step(1000.0), step(1100.0));
    for command in [&mut first, &mut second, &mut third] {
        apply(command, &mut doc).unwrap();
    }
    assert!(first.merge(Box::new(second)).is_ok());
    assert!(first.merge(Box::new(third)).is_ok());
    revert(&mut first, &mut doc).unwrap();
    assert_eq!(doc, original);
    apply(&mut first, &mut doc).unwrap();
    assert_eq!(doc.page(P1).unwrap().size, px(1100.0, 600.0));

    let mut dpi = SetProperty::document(Property::Dpi(Dpi::new(100)), None);
    let next = SetProperty::document(Property::Dpi(Dpi::new(120)), None);
    assert!(dpi.merge(Box::new(next)).is_ok());
    let mut doc = original.clone();
    apply(&mut dpi, &mut doc).unwrap();
    assert_eq!(doc.dpi(), Dpi::new(120));
}

#[test]
fn reverting_a_page_command_that_never_applied_fails() {
    let mut doc = two_pages(96);
    let mut page = SetProperty::page(P1, Property::PageLocked(true), None);
    assert_eq!(revert(&mut page, &mut doc), Err(CommandError::NotApplied));
    let mut dpi = SetProperty::document(Property::Dpi(Dpi::new(72)), None);
    assert_eq!(revert(&mut dpi, &mut doc), Err(CommandError::NotApplied));
}

#[test]
fn a_page_revert_without_its_page_fails_and_changes_nothing() {
    let mut doc = two_pages(96);
    let mut command = SetProperty::page(P1, Property::PageLocked(true), None);
    apply(&mut command, &mut doc).unwrap();
    doc.edit(&token()).pages_mut().remove(0);
    let before = doc.clone();
    assert_eq!(
        revert(&mut command, &mut doc),
        Err(CommandError::MissingPage(P1))
    );
    assert_eq!(doc, before);
}

#[test]
fn a_size_revert_that_would_leave_the_page_invalid_fails_and_changes_nothing() {
    // Page 2 (A0) shrinks to pixels at 300 dpi; later the DPI goes up by hand,
    // so the old A0 size no longer fits.
    let mut doc = two_pages(300);
    let mut command = SetProperty::page(P2, Property::PageSize(px(100.0, 100.0)), None);
    apply(&mut command, &mut doc).unwrap();
    doc.edit(&token()).set_dpi(Dpi::new(351));
    let before = doc.clone();
    let result = revert(&mut command, &mut doc);
    assert!(matches!(
        result,
        Err(CommandError::InvalidDocument(ValidationError::PageSize { page, .. })) if page == P2
    ));
    assert_eq!(doc, before);
}

#[test]
fn a_dpi_revert_that_would_leave_a_page_invalid_fails_and_changes_nothing() {
    let mut doc = two_pages(300);
    let mut command = SetProperty::document(Property::Dpi(Dpi::new(96)), None);
    apply(&mut command, &mut doc).unwrap();
    // Fits at 96 dpi, not at the 300 dpi the revert would restore.
    let taller = PageSize {
        width: Length::new(841.0, Unit::Mm),
        height: Length::new(1500.0, Unit::Mm),
    };
    doc.edit(&token()).page_mut(P2).unwrap().size = taller;
    assert_eq!(doc.validate(), Ok(()));
    let before = doc.clone();
    let result = revert(&mut command, &mut doc);
    assert!(matches!(
        result,
        Err(CommandError::InvalidDocument(ValidationError::PageSize { page, .. })) if page == P2
    ));
    assert_eq!(doc, before);
}

#[test]
fn a_page_command_is_reusable_after_a_failed_apply() {
    let mut command = SetProperty::page(P2, Property::PageName("n".into()), None);
    let mut doc = Document::default();
    assert_eq!(
        apply(&mut command, &mut doc),
        Err(CommandError::MissingPage(P2))
    );
    assert_eq!(
        revert(&mut command, &mut doc),
        Err(CommandError::NotApplied)
    );

    let original = two_pages(96);
    let mut doc = original.clone();
    assert_eq!(apply(&mut command, &mut doc), Ok(Applied::Changed));
    // A later failure keeps the record of the last good apply.
    assert!(apply(&mut command, &mut Document::default()).is_err());
    revert(&mut command, &mut doc).unwrap();
    assert_eq!(doc, original);
}

#[test]
fn a_dpi_command_is_reusable_after_a_failed_apply() {
    let mut command = SetProperty::document(Property::Dpi(Dpi::new(351)), None);
    let mut doc = two_pages(300);
    assert!(apply(&mut command, &mut doc).is_err());
    assert_eq!(
        revert(&mut command, &mut doc),
        Err(CommandError::NotApplied)
    );

    // Without the A0 page the same command fits.
    let original = Document::from_parts(2, Dpi::new(96), vec![page(P1, "p", px(800.0, 600.0))])
        .expect("valid");
    let mut doc = original.clone();
    assert_eq!(apply(&mut command, &mut doc), Ok(Applied::Changed));
    revert(&mut command, &mut doc).unwrap();
    assert_eq!(doc, original);
}

fn new_page(index: usize, name: &str) -> InsertPage {
    InsertPage::new(index, name.into(), px(100.0, 50.0))
}

fn ids(doc: &Document) -> Vec<PageId> {
    doc.pages().iter().map(|p| p.id).collect()
}

#[test]
fn insert_a_page_then_redo_reuses_its_id() {
    let original = two_pages(96);
    let mut doc = original.clone();
    let mut insert = new_page(1, "mid");
    assert_eq!(apply(&mut insert, &mut doc), Ok(Applied::Changed));
    assert_eq!(ids(&doc), [P1, PageId::new(3), P2]);
    assert_eq!(doc.next_id(), 4);
    let made = doc.page(PageId::new(3)).unwrap();
    assert_eq!((made.name.as_str(), made.size), ("mid", px(100.0, 50.0)));
    assert!(made.roots.is_empty() && made.nodes.is_empty() && !made.locked && !made.hidden);
    assert_eq!(doc.validate(), Ok(()));
    let touched = insert.touches();
    assert_eq!(touched.pages, [PageId::new(3)].into_iter().collect());
    assert!(touched.nodes.is_empty());

    let first = doc.clone();
    revert(&mut insert, &mut doc).unwrap();
    assert_eq!(doc, original);
    assert_eq!(apply(&mut insert, &mut doc), Ok(Applied::Changed));
    assert_eq!(doc, first);
}

#[test]
fn a_page_goes_in_at_any_index_up_to_the_length() {
    for (index, expected) in [(0, [3, 1, 2]), (2, [1, 2, 3])] {
        let mut doc = two_pages(96);
        apply(&mut new_page(index, "n"), &mut doc).unwrap();
        let got: Vec<u64> = ids(&doc).iter().map(|p| p.get()).collect();
        assert_eq!(got, expected);
    }
    let original = two_pages(96);
    let error = CommandError::IndexOutOfRange { index: 3, len: 2 };
    refused(new_page(3, "n"), &original, error);
}

#[test]
fn a_page_outside_the_pixel_limits_is_not_inserted() {
    let original = two_pages(96);
    let huge = InsertPage::new(0, "n".into(), px(20_000.0, 10.0));
    refused_size(huge, &original, PageId::new(3), Axis::Width);
    // Millimetres count at the document DPI.
    let at = |dpi| {
        let one = page(P1, "p", px(10.0, 10.0));
        Document::from_parts(2, Dpi::new(dpi), vec![one]).expect("valid")
    };
    let a0 = InsertPage::new(0, "a0".into(), a0());
    refused_size(a0.clone(), &at(351), PageId::new(2), Axis::Height);
    let mut doc = at(300);
    assert_eq!(apply(&mut { a0 }, &mut doc), Ok(Applied::Changed));
}

#[test]
fn an_exhausted_page_id_space_changes_nothing() {
    let full = Document::from_parts(u64::MAX, Dpi::new(96), vec![page(P1, "p", px(10.0, 10.0))])
        .expect("valid");
    refused(new_page(0, "n"), &full, CommandError::IdSpaceExhausted);
}

#[test]
fn an_insert_is_reusable_after_a_failed_apply() {
    let mut insert = new_page(1, "n");
    let mut empty = Document::default();
    assert!(apply(&mut insert, &mut empty).is_err());
    assert_eq!(
        revert(&mut insert, &mut empty),
        Err(CommandError::NotApplied)
    );

    let original = two_pages(96);
    let mut fresh = original.clone();
    apply(&mut new_page(1, "n"), &mut fresh).unwrap();
    let mut doc = original.clone();
    assert_eq!(apply(&mut insert, &mut doc), Ok(Applied::Changed));
    assert_eq!(doc, fresh);
    // A failed call after a good one keeps the record of the good one.
    assert!(apply(&mut insert, &mut Document::default()).is_err());
    revert(&mut insert, &mut doc).unwrap();
    assert_eq!(doc, original);
}

#[test]
fn an_insert_revert_checks_before_it_removes() {
    let mut doc = two_pages(96);
    let mut insert = new_page(0, "n");
    apply(&mut insert, &mut doc).unwrap();
    let made = PageId::new(3);

    // The page was edited since: reverting would lose that edit.
    let mut edited = doc.clone();
    edited.edit(&token()).page_mut(made).unwrap().name = "other".into();
    let before = edited.clone();
    assert_eq!(
        revert(&mut insert, &mut edited),
        Err(CommandError::InvalidValue("the inserted page was changed"))
    );
    assert_eq!(edited, before);

    let mut gone = doc.clone();
    gone.edit(&token()).pages_mut().remove(0);
    let before = gone.clone();
    assert_eq!(
        revert(&mut insert, &mut gone),
        Err(CommandError::MissingPage(made))
    );
    assert_eq!(gone, before);
}

#[test]
fn an_id_is_reissued_to_a_page_after_an_undo() {
    let mut history = History::new(two_pages(96)).expect("valid");
    let made = PageId::new(3);
    assert_eq!(
        history.execute(Box::new(new_page(0, "first")), Duration::ZERO),
        Ok(Applied::Changed)
    );
    assert!(history.undo().unwrap().is_some());
    assert_eq!(history.document().next_id(), 3);
    assert_eq!(
        history.execute(Box::new(new_page(2, "second")), Duration::ZERO),
        Ok(Applied::Changed)
    );
    let doc = history.document();
    assert_eq!(ids(doc), [P1, P2, made]);
    assert_eq!(doc.page(made).unwrap().name, "second");
    assert_eq!(doc.validate(), Ok(()));
}

#[test]
fn a_failed_transaction_gives_the_page_id_back() {
    let original = two_pages(96);
    let mut doc = original.clone();
    let steps: Vec<Box<dyn Command>> =
        vec![Box::new(new_page(0, "ok")), Box::new(new_page(9, "bad"))];
    let mut tx = Transaction::new(MessageKey("test.tx"), steps);
    assert!(apply(&mut tx, &mut doc).is_err());
    assert_eq!(doc, original);
}

/// Pages 1 to 3, page 2 holding a group-less shape (id 4); counter at 5.
fn with_nodes() -> Document {
    let base = Document::from_parts(
        4,
        Dpi::new(96),
        vec![
            page(P1, "one", px(800.0, 600.0)),
            page(P2, "two", px(800.0, 600.0)),
            page(PageId::new(3), "three", px(800.0, 600.0)),
        ],
    )
    .expect("valid");
    let shape = NodeSpec::new(
        NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect)),
        Placement::new(0.0, 0.0, 10.0, 10.0),
    );
    let mut doc = base;
    apply(&mut InsertNodes::new(P2, None, 0, vec![shape]), &mut doc).unwrap();
    doc
}

#[test]
fn remove_a_page_and_restore_it_at_its_index() {
    let original = with_nodes();
    let mut doc = original.clone();
    let mut remove = RemovePage::new(P2);
    assert_eq!(apply(&mut remove, &mut doc), Ok(Applied::Changed));
    assert_eq!(ids(&doc), [P1, PageId::new(3)]);
    assert_eq!(doc.validate(), Ok(()));
    let touched = remove.touches();
    assert_eq!(touched.pages, [P2].into_iter().collect());
    assert_eq!(touched.nodes, [NodeId::new(4)].into_iter().collect());
    revert(&mut remove, &mut doc).unwrap();
    assert_eq!(doc, original);
    assert!(Arc::ptr_eq(&doc.pages()[1], &original.pages()[1]));
    // Redo removes it again.
    assert_eq!(apply(&mut remove, &mut doc), Ok(Applied::Changed));
    assert_eq!(ids(&doc), [P1, PageId::new(3)]);
}

#[test]
fn the_last_page_can_go() {
    let original =
        Document::from_parts(2, Dpi::new(96), vec![page(P1, "p", px(10.0, 10.0))]).expect("valid");
    let mut doc = original.clone();
    let mut remove = RemovePage::new(P1);
    assert_eq!(apply(&mut remove, &mut doc), Ok(Applied::Changed));
    assert!(doc.pages().is_empty());
    assert_eq!(doc.validate(), Ok(()));
    revert(&mut remove, &mut doc).unwrap();
    assert_eq!(doc, original);
}

#[test]
fn a_missing_page_is_not_removed() {
    let ghost = PageId::new(9);
    refused(
        RemovePage::new(ghost),
        &two_pages(96),
        CommandError::MissingPage(ghost),
    );
}

#[test]
fn a_remove_is_reusable_after_a_failed_apply() {
    let mut remove = RemovePage::new(P2);
    let mut empty = Document::default();
    assert!(apply(&mut remove, &mut empty).is_err());
    assert_eq!(
        revert(&mut remove, &mut empty),
        Err(CommandError::NotApplied)
    );

    let original = with_nodes();
    let mut doc = original.clone();
    assert_eq!(apply(&mut remove, &mut doc), Ok(Applied::Changed));
    assert!(apply(&mut remove, &mut empty).is_err());
    revert(&mut remove, &mut doc).unwrap();
    assert_eq!(doc, original);
}

#[test]
fn a_remove_revert_checks_before_it_inserts() {
    let mut doc = with_nodes();
    let mut remove = RemovePage::new(PageId::new(3));
    apply(&mut remove, &mut doc).unwrap();

    // The page is already back (its id is in the list).
    let mut back = doc.clone();
    let mut again = remove.clone();
    revert(&mut again, &mut back).unwrap();
    let before = back.clone();
    assert_eq!(
        revert(&mut remove, &mut back),
        Err(CommandError::InvalidValue(
            "the removed page is already back"
        ))
    );
    assert_eq!(back, before);

    // Its index no longer exists: the list shrank below it.
    let mut short = doc.clone();
    short.edit(&token()).pages_mut().clear();
    let before = short.clone();
    assert_eq!(
        revert(&mut remove, &mut short),
        Err(CommandError::IndexOutOfRange { index: 2, len: 0 })
    );
    assert_eq!(short, before);
}
