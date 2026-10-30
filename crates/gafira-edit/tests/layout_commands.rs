//! `Property::{Bleed, Margins, Guides, Grid}` through `SetProperty`: FD-03(d),
//! FD-04(c), FD-05, FD-07(e), design D2 and ARQ-110.

use std::sync::Arc;
use std::time::Duration;

use gafira_doc::{
    Document, Dpi, GridSettings, GridStyle, Guide, GuideAxis, GuideSet, Margins, Page, PageField,
    PageId, PageLayoutError, PageSize, ValidationError,
};
use gafira_edit::{Applied, Command, CommandError, GestureId, History, Property, SetProperty};
use gafira_geom::{Length, Unit};
use gafira_testkit::{bleed, grid_settings, guide_set, margins, prop_command};
use proptest::prelude::*;

const P1: PageId = PageId::new(1);

fn px(v: f64) -> Length {
    Length::new(v, Unit::Px)
}

fn mm(v: f64) -> Length {
    Length::new(v, Unit::Mm)
}

fn guide(axis: GuideAxis, v: f64, locked: bool) -> Guide {
    Guide {
        axis,
        position: px(v),
        locked,
    }
}

/// The guides of the fixture: a duplicate, a lock and an order that is not
/// sorted, so any reordering or dedup shows.
fn old_guides() -> GuideSet {
    GuideSet(vec![
        guide(GuideAxis::Vertical, 50.0, true),
        guide(GuideAxis::Horizontal, 5.0, false),
        guide(GuideAxis::Vertical, 50.0, true),
        guide(GuideAxis::Vertical, 1.0, false),
    ])
}

fn old_margins() -> Margins {
    Margins {
        top: px(1.0),
        right: Length::new(2.0, Unit::Mm),
        bottom: Length::new(3.0, Unit::Cm),
        left: Length::new(4.0, Unit::In),
        linked: false,
    }
}

fn old_grid() -> GridSettings {
    GridSettings {
        visible: true,
        step: Length::new(2.5, Unit::Mm),
        subdivisions: 4,
        style: GridStyle::Dots,
        snap: false,
    }
}

/// One 800 x 600 px page whose layout fields all differ from the defaults.
fn fixture(dpi: u32) -> Document {
    let mut page = Page::new(
        P1,
        "one".into(),
        PageSize {
            width: px(800.0),
            height: px(600.0),
        },
    );
    page.bleed = mm(3.0);
    page.margins = old_margins();
    page.guides = old_guides();
    page.grid = old_grid();
    Document::from_parts(2, Dpi::new(dpi), vec![Arc::new(page)]).expect("valid")
}

fn token() -> gafira_doc::EditToken {
    gafira_doc::EditToken::__new_for_gafira_edit_only()
}

fn apply(command: &mut impl Command, doc: &mut Document) -> Result<Applied, CommandError> {
    command.apply(&mut doc.edit(&token()))
}

fn revert(command: &mut impl Command, doc: &mut Document) -> Result<(), CommandError> {
    command.revert(&mut doc.edit(&token()))
}

fn set(value: Property) -> SetProperty {
    SetProperty::page(P1, value, None)
}

fn layout(field: PageField, value: Option<f64>) -> CommandError {
    let error = match value {
        Some(value) => PageLayoutError::OutOfRange { field, value },
        None => PageLayoutError::NonFinite { field },
    };
    CommandError::InvalidDocument(ValidationError::PageLayout { page: P1, error })
}

/// The value of a layout property that differs from the fixture's.
fn new_values() -> Vec<Property> {
    vec![
        Property::Bleed(mm(5.0)),
        Property::Margins(Margins::default()),
        Property::Guides(GuideSet(vec![guide(GuideAxis::Horizontal, 9.0, false)])),
        Property::Guides(GuideSet::default()),
        Property::Grid(GridSettings::default()),
    ]
}

// FD-03, FD-05, FD-07: each property is stored on its page, nothing else
// changes, and revert gives the exact previous document back.
#[test]
fn fd_05_every_layout_property_is_set_and_restored() {
    for value in new_values() {
        let original = fixture(96);
        let mut doc = original.clone();
        let mut command = set(value.clone());
        assert_eq!(apply(&mut command, &mut doc), Ok(Applied::Changed));
        let (before, after) = (original.page(P1).unwrap(), doc.page(P1).unwrap());
        match &value {
            Property::Bleed(v) => assert_eq!(after.bleed, *v),
            Property::Margins(v) => assert_eq!(after.margins, *v),
            Property::Guides(v) => assert_eq!(after.guides, *v),
            Property::Grid(v) => assert_eq!(after.grid, *v),
            other => panic!("{other:?}"),
        }
        let expect = |same: bool, name: &str, a: bool| assert_eq!(a, same, "{name} {value:?}");
        expect(
            matches!(value, Property::Bleed(_)),
            "bleed",
            before.bleed != after.bleed,
        );
        expect(
            matches!(value, Property::Margins(_)),
            "margins",
            before.margins != after.margins,
        );
        expect(
            matches!(value, Property::Guides(_)),
            "guides",
            before.guides != after.guides,
        );
        expect(
            matches!(value, Property::Grid(_)),
            "grid",
            before.grid != after.grid,
        );
        assert_eq!(doc.validate(), Ok(()));
        let touched = command.touches();
        assert_eq!(touched.pages, [P1].into_iter().collect());
        assert!(touched.nodes.is_empty());
        revert(&mut command, &mut doc).unwrap();
        assert_eq!(doc, original, "{value:?}");
    }
}

// FD-04(c): applying guides and undoing restores exactly the previous set,
// order, duplicates and locks included, whether the history still holds the
// snapshot or has compacted it and must revert the command.
#[test]
fn fd_04_c_applying_guides_and_undoing_restores_the_exact_previous_set() {
    let new = GuideSet(vec![
        guide(GuideAxis::Horizontal, 7.0, true),
        guide(GuideAxis::Vertical, -3.0, false),
    ]);
    for budget in [usize::MAX, 0] {
        let mut history = History::with_budget(fixture(96), budget).unwrap();
        let guides = |h: &History| h.document().page(P1).unwrap().guides.clone();
        let at = Duration::ZERO;
        let command = || Box::new(set(Property::Guides(new.clone())));
        history.execute(command(), at).unwrap();
        // A second, different change makes the first entry the older one.
        let other = set(Property::Guides(GuideSet::default()));
        history
            .execute(Box::new(other), Duration::from_secs(9))
            .unwrap();
        assert!(guides(&history).0.is_empty());
        history.undo().unwrap();
        assert_eq!(guides(&history), new, "budget {budget}");
        history.undo().unwrap();
        assert_eq!(guides(&history), old_guides(), "budget {budget}");
        history.redo().unwrap();
        assert_eq!(guides(&history), new, "budget {budget}");
    }
}

// FD-04(c) at the command level: the previous set comes back from the
// command itself, also after being applied twice.
#[test]
fn fd_04_c_revert_gives_back_the_set_the_last_apply_replaced() {
    let mut doc = fixture(96);
    let first = GuideSet(vec![guide(GuideAxis::Vertical, 2.0, false)]);
    let mut command = set(Property::Guides(first.clone()));
    apply(&mut command, &mut doc).unwrap();
    revert(&mut command, &mut doc).unwrap();
    assert_eq!(doc.page(P1).unwrap().guides, old_guides());
    // Applied again on a document that holds another set.
    let mut doc = fixture(96);
    apply(&mut set(Property::Guides(GuideSet::default())), &mut doc).unwrap();
    apply(&mut command, &mut doc).unwrap();
    revert(&mut command, &mut doc).unwrap();
    assert!(doc.page(P1).unwrap().guides.0.is_empty());
}

// FD-05, FD-07: setting the value the page already holds changes nothing.
#[test]
fn fd_05_the_same_layout_value_is_unchanged_and_shares_the_page() {
    let original = fixture(96);
    let same = [
        Property::Bleed(mm(3.0)),
        Property::Margins(old_margins()),
        Property::Guides(old_guides()),
        Property::Grid(old_grid()),
    ];
    for value in same {
        let mut doc = original.clone();
        let mut command = set(value);
        assert_eq!(apply(&mut command, &mut doc), Ok(Applied::Unchanged));
        assert!(Arc::ptr_eq(&doc.pages()[0], &original.pages()[0]));
        revert(&mut command, &mut doc).unwrap();
        assert!(Arc::ptr_eq(&doc.pages()[0], &original.pages()[0]));
    }
}

fn bad_margins(side: usize, v: f64) -> Margins {
    let mut m = old_margins();
    *[&mut m.top, &mut m.right, &mut m.bottom, &mut m.left][side] = px(v);
    m
}

fn grid(f: impl FnOnce(&mut GridSettings)) -> Property {
    let mut g = old_grid();
    f(&mut g);
    Property::Grid(g)
}

fn one_guide(v: f64) -> Property {
    Property::Guides(GuideSet(vec![
        guide(GuideAxis::Vertical, 1.0, false),
        guide(GuideAxis::Horizontal, v, false),
    ]))
}

const SIDES: [PageField; 4] = [
    PageField::MarginTop,
    PageField::MarginRight,
    PageField::MarginBottom,
    PageField::MarginLeft,
];

// FD-03(d), FD-05(d), FD-07(e), ARQ-110: a value outside the ranges, or not
// finite, is refused with the error `validate()` would give, and the page
// is as it was.
#[test]
fn arq_110_invalid_layout_values_fail_and_change_nothing() {
    let nan = f64::NAN;
    let mut rows = vec![
        (
            Property::Bleed(mm(10.01)),
            layout(PageField::Bleed, Some(10.01)),
        ),
        (
            Property::Bleed(mm(-0.01)),
            layout(PageField::Bleed, Some(-0.01)),
        ),
        (Property::Bleed(mm(nan)), layout(PageField::Bleed, None)),
        (
            Property::Bleed(mm(f64::INFINITY)),
            layout(PageField::Bleed, None),
        ),
        (one_guide(nan), layout(PageField::GuidePosition, None)),
        (
            one_guide(f64::INFINITY),
            layout(PageField::GuidePosition, None),
        ),
        (
            one_guide(f64::NEG_INFINITY),
            layout(PageField::GuidePosition, None),
        ),
        (
            grid(|g| g.step = px(0.99)),
            layout(PageField::GridStep, Some(0.99)),
        ),
        (
            grid(|g| g.step = px(500.01)),
            layout(PageField::GridStep, Some(500.01)),
        ),
        (
            grid(|g| g.step = px(nan)),
            layout(PageField::GridStep, None),
        ),
        (
            grid(|g| g.subdivisions = 0),
            layout(PageField::GridSubdivisions, Some(0.0)),
        ),
        (
            grid(|g| g.subdivisions = 11),
            layout(PageField::GridSubdivisions, Some(11.0)),
        ),
    ];
    for (side, field) in SIDES.into_iter().enumerate() {
        rows.push((
            Property::Margins(bad_margins(side, -0.01)),
            layout(field, Some(-0.01)),
        ));
        rows.push((
            Property::Margins(bad_margins(side, nan)),
            layout(field, None),
        ));
    }
    for (value, error) in rows {
        let original = fixture(96);
        let mut doc = original.clone();
        let mut command = set(value.clone());
        assert_eq!(apply(&mut command, &mut doc), Err(error), "{value:?}");
        assert_eq!(doc, original);
        assert!(Arc::ptr_eq(&doc.pages()[0], &original.pages()[0]));
        assert_eq!(
            revert(&mut command, &mut doc),
            Err(CommandError::NotApplied)
        );
    }
}

// FD-05(d), FD-07(e): the inclusive ends are accepted.
#[test]
fn fd_05_fd_07_range_ends_are_accepted() {
    let values = [
        Property::Bleed(mm(0.0)),
        Property::Bleed(mm(10.0)),
        Property::Margins(bad_margins(0, 0.0)),
        Property::Margins(bad_margins(3, 1e9)),
        one_guide(1e12),
        one_guide(-1e12),
        grid(|g| g.step = px(1.0)),
        grid(|g| g.step = px(500.0)),
        grid(|g| g.subdivisions = 1),
        grid(|g| g.subdivisions = 10),
    ];
    for value in values {
        let mut doc = fixture(96);
        let result = apply(&mut set(value.clone()), &mut doc);
        assert!(result.is_ok(), "{value:?}: {result:?}");
        assert_eq!(doc.validate(), Ok(()));
    }
}

// FD-05(d): the bleed range is checked in millimetres at the document DPI.
#[test]
fn fd_05_bleed_is_checked_in_millimetres_at_the_document_dpi() {
    // 40 px is 10.58 mm at 96 dpi and 3.39 mm at 300 dpi.
    let forty = Property::Bleed(px(40.0));
    let mut high = fixture(300);
    assert_eq!(
        apply(&mut set(forty.clone()), &mut high),
        Ok(Applied::Changed)
    );
    let mut low = fixture(96);
    let error = apply(&mut set(forty), &mut low).unwrap_err();
    assert!(
        matches!(
            error,
            CommandError::InvalidDocument(ValidationError::PageLayout {
                error: PageLayoutError::OutOfRange {
                    field: PageField::Bleed,
                    ..
                },
                ..
            })
        ),
        "{error:?}"
    );
    let mut doc = fixture(96);
    let half_inch = Property::Bleed(Length::new(0.5, Unit::In));
    assert!(apply(&mut set(half_inch), &mut doc).is_err());
}

// A layout property on a missing page, or with node targets, is refused.
#[test]
fn a_layout_property_needs_an_existing_page_and_no_node_targets() {
    let original = fixture(96);
    let mut doc = original.clone();
    let ghost = PageId::new(9);
    let mut missing = SetProperty::page(ghost, Property::Bleed(mm(1.0)), None);
    assert_eq!(
        apply(&mut missing, &mut doc),
        Err(CommandError::MissingPage(ghost))
    );
    let nodes = vec![gafira_doc::NodeId::new(5)];
    let mut targeted = SetProperty::new(P1, nodes, Property::Bleed(mm(1.0)), None);
    assert!(matches!(
        apply(&mut targeted, &mut doc),
        Err(CommandError::InvalidValue(_))
    ));
    assert_eq!(doc, original);
}

// FD-04: guide drags of one gesture merge; the last set wins and one undo
// goes back to the origin.
#[test]
fn fd_04_a_guide_drag_merges_and_reverts_to_the_origin() {
    let original = fixture(96);
    let step = |v: f64| {
        let guides = GuideSet(vec![guide(GuideAxis::Vertical, v, false)]);
        SetProperty::page(P1, Property::Guides(guides), Some(GestureId(3)))
    };
    let mut doc = original.clone();
    let mut first = step(1.0);
    apply(&mut first, &mut doc).unwrap();
    let mut second = step(2.0);
    apply(&mut second, &mut doc).unwrap();
    assert!(first.merge(Box::new(second)).is_ok());
    revert(&mut first, &mut doc).unwrap();
    assert_eq!(doc, original);
    apply(&mut first, &mut doc).unwrap();
    assert_eq!(
        doc.page(P1).unwrap().guides.0[0].position,
        px(2.0),
        "the merged command applies the last value"
    );
}

// The merge key kind separates the four properties.
#[test]
fn layout_properties_have_distinct_merge_keys() {
    let kinds: Vec<&str> = new_values()
        .into_iter()
        .map(|v| set(v).merge_key().unwrap().kind)
        .collect();
    assert_eq!(
        kinds,
        [
            "set_property.bleed",
            "set_property.margins",
            "set_property.guides",
            "set_property.guides",
            "set_property.grid",
        ]
    );
}

fn layout_property() -> impl Strategy<Value = Property> {
    prop_oneof![
        bleed().prop_map(Property::Bleed),
        margins().prop_map(Property::Margins),
        guide_set().prop_map(Property::Guides),
        grid_settings().prop_map(Property::Grid),
        // Values `validate()` must refuse.
        prop_oneof![Just(-1.0_f64), Just(11.0), Just(f64::NAN)]
            .prop_map(|v| Property::Bleed(mm(v))),
        prop_oneof![Just(-1.0_f64), Just(f64::NAN)]
            .prop_map(|v| Property::Margins(bad_margins(1, v))),
        prop_oneof![Just(f64::INFINITY), Just(f64::NAN)].prop_map(one_guide),
        prop_oneof![Just(0.0_f64), Just(501.0), Just(f64::NAN)]
            .prop_map(|v| grid(|g| g.step = px(v))),
        prop_oneof![Just(0_u8), Just(11)].prop_map(|n| grid(|g| g.subdivisions = n)),
    ]
}

fn layout_command(doc: &Document) -> BoxedStrategy<SetProperty> {
    let pages: Vec<PageId> = doc.pages().iter().map(|p| p.id).collect();
    let missing = Just(PageId::new(u64::MAX));
    let page = if pages.is_empty() {
        missing.boxed()
    } else {
        prop_oneof![9 => prop::sample::select(pages), 1 => missing].boxed()
    };
    let gesture = prop_oneof![Just(None), (0..2_u64).prop_map(|g| Some(GestureId(g)))];
    (page, layout_property(), gesture)
        .prop_map(|(page, value, gesture)| SetProperty::page(page, value, gesture))
        .boxed()
}

// ARQ-81 laws for the four properties: failed apply changes nothing, revert
// restores, apply is idempotent, merge is sound.
#[test]
fn layout_properties_obey_the_command_laws() {
    prop_command(layout_command);
}

proptest! {
    #![proptest_config(ProptestConfig { failure_persistence: None, ..ProptestConfig::default() })]

    // FD-04(c): for any previous and any new set, apply then revert gives
    // the previous set back, order and duplicates included.
    #[test]
    fn fd_04_c_any_guide_set_reverts_to_the_previous_one(prev in guide_set(), new in guide_set()) {
        let mut doc = fixture(96);
        apply(&mut set(Property::Guides(prev.clone())), &mut doc).unwrap();
        let before = doc.clone();
        let mut command = set(Property::Guides(new.clone()));
        apply(&mut command, &mut doc).unwrap();
        prop_assert_eq!(&doc.page(P1).unwrap().guides, &new);
        revert(&mut command, &mut doc).unwrap();
        prop_assert_eq!(&doc, &before);
        prop_assert_eq!(&doc.page(P1).unwrap().guides, &prev);
    }
}

fn holding(count: usize) -> GuideSet {
    GuideSet(
        (0..count)
            .map(|i| guide(GuideAxis::Vertical, i as f64, false))
            .collect(),
    )
}

/// The history estimate after one change of a page that holds `count` guides:
/// the entry keeps the old page, so it carries its guides.
fn estimate_replacing(count: usize) -> usize {
    let mut doc = fixture(96);
    apply(&mut set(Property::Guides(holding(count))), &mut doc).unwrap();
    let mut history = History::new(doc).unwrap();
    let rename = set(Property::PageName("renamed".into()));
    history.execute(Box::new(rename), Duration::ZERO).unwrap();
    history.estimated_bytes()
}

// D1 verify W3, FD-04: the memory estimate of an entry grows with the
// guides of the page it replaced, one record per guide.
#[test]
fn fd_04_the_history_estimate_grows_with_the_guide_count() {
    let none = estimate_replacing(0);
    for count in [1, 10, 10_000] {
        let grown = estimate_replacing(count) - none;
        assert_eq!(grown, count * size_of::<Guide>(), "{count} guides");
    }
}

const P2: PageId = PageId::new(2);

/// Page 1 as in `fixture`, page 2 with a 100 px bleed: 8.47 mm at 300 dpi,
/// 26.46 mm at 96 dpi.
fn two_pages(dpi: u32) -> Document {
    let one = fixture(dpi).pages()[0].clone();
    let mut two = Page::new(
        P2,
        "two".into(),
        PageSize {
            width: px(800.0),
            height: px(600.0),
        },
    );
    two.bleed = px(100.0);
    Document::from_parts(3, Dpi::new(dpi), vec![one, Arc::new(two)]).expect("valid")
}

// ARQ-13, FD-05: a DPI change that would put the bleed of any page out of
// range is refused like setting that bleed directly, and changes nothing.
#[test]
fn arq_13_a_dpi_change_that_breaks_a_bleed_is_refused() {
    let original = two_pages(300);
    let mut doc = original.clone();
    let mut command = SetProperty::document(Property::Dpi(Dpi::new(96)), None);
    let error = apply(&mut command, &mut doc).unwrap_err();
    // The same 100 px bleed set directly at 96 dpi, on page 2's id.
    let mut direct = SetProperty::page(P1, Property::Bleed(px(100.0)), None);
    let want = apply(&mut direct, &mut fixture(96)).unwrap_err();
    let CommandError::InvalidDocument(ValidationError::PageLayout { error: layout, .. }) = want
    else {
        panic!("{want:?}");
    };
    let page_two = ValidationError::PageLayout {
        page: P2,
        error: layout,
    };
    assert_eq!(error, CommandError::InvalidDocument(page_two));
    assert_eq!(doc, original);
    assert_eq!(doc.dpi(), Dpi::new(300));
}

// ARQ-13: a DPI change that keeps every bleed in range still works, and
// revert gives the old DPI back.
#[test]
fn arq_13_a_dpi_change_that_keeps_bleeds_in_range_applies_and_reverts() {
    let original = two_pages(300);
    let mut doc = original.clone();
    // 100 px is 12.7 mm at 200 dpi, still out; 150 dpi is further out; 600 is in.
    let mut command = SetProperty::document(Property::Dpi(Dpi::new(600)), None);
    assert_eq!(apply(&mut command, &mut doc), Ok(Applied::Changed));
    assert_eq!(doc.validate(), Ok(()));
    revert(&mut command, &mut doc).unwrap();
    assert_eq!(doc, original);
}

// L1 order inside the DPI path: when the new DPI breaks both a page size and a
// bleed, the size is reported, as `validate()` does.
#[test]
fn arq_13_a_dpi_change_reports_page_sizes_before_layout() {
    let mut pages: Vec<Arc<Page>> = two_pages(300).pages().to_vec();
    // Page 2 has the 100 px bleed (out of range at 96 dpi). Its width of
    // 0.004 in is 1.2 px at 300 dpi and 0.38 px at 96 dpi.
    let mut thin = (*pages[1]).clone();
    thin.size.width = Length::new(0.004, Unit::In);
    pages[1] = Arc::new(thin);
    let original = Document::from_parts(3, Dpi::new(300), pages).unwrap();
    let mut doc = original.clone();
    let mut command = SetProperty::document(Property::Dpi(Dpi::new(96)), None);
    let error = apply(&mut command, &mut doc).unwrap_err();
    assert!(
        matches!(
            error,
            CommandError::InvalidDocument(ValidationError::PageSize { page: P2, .. })
        ),
        "{error:?}"
    );
    assert_eq!(doc, original);
}

// C2: a bad value on a later page is refused through the property path.
#[test]
fn fd_05_a_layout_value_is_checked_on_the_page_it_targets() {
    let original = two_pages(300);
    let mut doc = original.clone();
    let mut command = SetProperty::page(P2, Property::Bleed(mm(11.0)), None);
    let want = CommandError::InvalidDocument(ValidationError::PageLayout {
        page: P2,
        error: PageLayoutError::OutOfRange {
            field: PageField::Bleed,
            value: 11.0,
        },
    });
    assert_eq!(apply(&mut command, &mut doc), Err(want));
    assert_eq!(doc, original);
}
