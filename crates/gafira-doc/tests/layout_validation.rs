//! FD-03(d), FD-05(d), FD-07(e), design L1: the page-layout ranges that
//! `Document::validate` enforces. Nothing is clamped.

use gafira_doc::{
    Document, Dpi, Guide, GuideAxis, GuideSet, LoadError, Page, PageField, PageId, PageLayoutError,
    PageSize, ValidationError,
};
use gafira_geom::{Length, Unit};
use gafira_testkit::{bleed, grid_settings, guide_set, margins};
use proptest::prelude::*;

const PAGE: PageId = PageId::new(1);

fn px(v: f64) -> Length {
    Length::new(v, Unit::Px)
}

fn mm(v: f64) -> Length {
    Length::new(v, Unit::Mm)
}

fn page() -> Page {
    let size = PageSize {
        width: px(100.0),
        height: px(50.0),
    };
    Page::new(PAGE, "P".into(), size)
}

fn validate(p: Page) -> Result<(), ValidationError> {
    Document::from_parts(2, Dpi::new(96), vec![p.into()]).map(|_| ())
}

fn layout_error(error: PageLayoutError) -> Result<(), ValidationError> {
    Err(ValidationError::PageLayout { page: PAGE, error })
}

fn out(field: PageField, value: f64) -> Result<(), ValidationError> {
    layout_error(PageLayoutError::OutOfRange { field, value })
}

fn non_finite(field: PageField) -> Result<(), ValidationError> {
    layout_error(PageLayoutError::NonFinite { field })
}

fn with_margin(side: usize, v: f64) -> Page {
    let mut p = page();
    let m = &mut p.margins;
    *[&mut m.top, &mut m.right, &mut m.bottom, &mut m.left][side] = px(v);
    p
}

const SIDES: [PageField; 4] = [
    PageField::MarginTop,
    PageField::MarginRight,
    PageField::MarginBottom,
    PageField::MarginLeft,
];

// FD-05(d): bleed 0 and 10 mm are valid, 10.01 mm and below 0 are errors.
#[test]
fn fd_05_bleed_bounds_are_inclusive_on_both_ends() {
    let rows = [
        (0.0, true),
        (10.0, true),
        (5.0, true),
        (10.01, false),
        (-0.01, false),
    ];
    for (v, ok) in rows {
        let mut p = page();
        p.bleed = mm(v);
        let want = if ok { Ok(()) } else { out(PageField::Bleed, v) };
        assert_eq!(validate(p), want, "{v} mm");
    }
}

// FD-05: the bleed range is in millimetres whatever unit it is stored in.
#[test]
fn fd_05_bleed_range_is_in_millimetres_for_any_unit() {
    let rows = [
        (Length::new(0.39, Unit::In), true),
        (Length::new(0.40, Unit::In), false),
        (Length::new(0.9, Unit::Cm), true),
        (Length::new(1.1, Unit::Cm), false),
        (Length::new(28.0, Unit::Pt), true),
        (Length::new(29.0, Unit::Pt), false),
        (px(37.0), true),
        (px(38.0), false),
        (px(-1.0), false),
    ];
    for (bleed, ok) in rows {
        let mut p = page();
        p.bleed = bleed;
        assert_eq!(validate(p).is_ok(), ok, "{bleed:?}");
    }
}

// FD-05(d) at the largest valid point in other units: exactly 10 mm.
#[test]
fn fd_05_ten_millimetres_in_inches_is_valid() {
    let mut p = page();
    p.bleed = Length::new(10.0 / 25.4, Unit::In);
    assert_eq!(validate(p), Ok(()));
}

// FD-05: margins are finite and not negative; zero is valid; there is no
// upper bound.
#[test]
fn fd_05_margin_bounds_are_inclusive_on_the_zero_end() {
    for (side, field) in SIDES.into_iter().enumerate() {
        assert_eq!(validate(with_margin(side, 0.0)), Ok(()), "{field}");
        assert_eq!(validate(with_margin(side, 1e9)), Ok(()), "{field}");
        assert_eq!(validate(with_margin(side, -0.01)), out(field, -0.01));
        assert_eq!(validate(with_margin(side, -1e9)), out(field, -1e9));
    }
}

// FD-05: margins are checked top, right, bottom, left; the first fails.
#[test]
fn fd_05_margins_report_the_first_bad_side() {
    let mut p = page();
    p.margins.bottom = px(-1.0);
    p.margins.left = px(-2.0);
    assert_eq!(validate(p), out(PageField::MarginBottom, -1.0));
}

// FD-07(e): step 1 and 500 are valid, 0.99 and 500.01 are errors; the range
// is on the stored number, in its own unit.
#[test]
fn fd_07_grid_step_bounds_are_inclusive_on_both_ends() {
    for unit in Unit::ALL {
        let rows = [(1.0, true), (500.0, true), (0.99, false), (500.01, false)];
        for (v, ok) in rows {
            let mut p = page();
            p.grid.step = Length::new(v, unit);
            let want = if ok {
                Ok(())
            } else {
                out(PageField::GridStep, v)
            };
            assert_eq!(validate(p), want, "{v} {unit:?}");
        }
    }
}

// FD-07(e): subdivisions 1 and 10 are valid, 0 and 11 are errors.
#[test]
fn fd_07_subdivision_bounds_are_inclusive_on_both_ends() {
    let rows = [(1, true), (10, true), (5, true), (0, false), (11, false)];
    for (n, ok) in rows {
        let mut p = page();
        p.grid.subdivisions = n;
        let want = if ok {
            Ok(())
        } else {
            out(PageField::GridSubdivisions, f64::from(n))
        };
        assert_eq!(validate(p), want, "{n}");
    }
}

// FD-03(d): a guide position may be any finite value, off the page included;
// it is never range-limited.
#[test]
fn fd_03_guide_positions_are_finite_only() {
    for v in [0.0, -3.0, 1e12, -1e12, f64::MAX, f64::MIN] {
        let mut p = page();
        let at = |v| Guide {
            axis: GuideAxis::Vertical,
            position: px(v),
            locked: false,
        };
        p.guides = GuideSet(vec![at(v), at(0.0)]);
        assert_eq!(validate(p), Ok(()), "{v}");
    }
}

// ARQ-110, FD-03(d), FD-05, FD-07: NaN and infinity are rejected in every
// new field, naming the field.
#[test]
fn arq_110_non_finite_values_are_rejected_in_every_new_field() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut p = page();
        p.bleed = mm(bad);
        assert_eq!(validate(p), non_finite(PageField::Bleed), "bleed {bad}");

        for (side, field) in SIDES.into_iter().enumerate() {
            assert_eq!(validate(with_margin(side, bad)), non_finite(field));
        }

        let mut p = page();
        p.guides = GuideSet(vec![Guide {
            axis: GuideAxis::Horizontal,
            position: Length::new(bad, Unit::Mm),
            locked: true,
        }]);
        assert_eq!(validate(p), non_finite(PageField::GuidePosition), "{bad}");

        let mut p = page();
        p.grid.step = px(bad);
        assert_eq!(validate(p), non_finite(PageField::GridStep), "step {bad}");
    }
}

// FD-03(d): a bad guide anywhere in the set fails the document.
#[test]
fn fd_03_a_bad_guide_in_any_position_fails() {
    let at = |v| Guide {
        axis: GuideAxis::Vertical,
        position: px(v),
        locked: false,
    };
    for bad in 0..3 {
        let mut guides = vec![at(1.0), at(2.0), at(3.0)];
        guides[bad] = at(f64::NAN);
        let mut p = page();
        p.guides = GuideSet(guides);
        assert_eq!(validate(p), non_finite(PageField::GuidePosition), "{bad}");
    }
}

// Each step is its own: one bad field in an otherwise default page is found,
// and the passes run bleed, margins, guides, grid.
#[test]
fn layout_checks_run_in_a_fixed_order() {
    let mut p = page();
    p.grid.subdivisions = 0;
    p.grid.step = px(0.0);
    assert_eq!(validate(p.clone()), out(PageField::GridStep, 0.0));
    p.guides = GuideSet(vec![Guide {
        axis: GuideAxis::Vertical,
        position: px(f64::NAN),
        locked: false,
    }]);
    assert_eq!(validate(p.clone()), non_finite(PageField::GuidePosition));
    p.margins.left = px(-1.0);
    assert_eq!(validate(p.clone()), out(PageField::MarginLeft, -1.0));
    p.bleed = mm(11.0);
    assert_eq!(validate(p), out(PageField::Bleed, 11.0));
}

fn page_json(extra: &str) -> Vec<u8> {
    let size = r#""size":{"width":{"value":100,"unit":"px"},"height":{"value":50,"unit":"px"}}"#;
    format!(r#"{{"schema_version":1,"next_id":2,"pages":[{{"id":1,{size}{extra}}}]}}"#).into_bytes()
}

// S1, FD-05(d), FD-07(e): out-of-range values on load are rejected, never
// clamped. The generators stay in range, so these cases are written out.
#[test]
fn fd_05_fd_07_out_of_range_values_on_load_are_rejected_not_clamped() {
    let zero = r#"{"value":0,"unit":"px"}"#;
    let rows = [
        (
            r#","bleed":{"value":99,"unit":"mm"}"#.to_string(),
            PageField::Bleed,
            99.0,
        ),
        (
            r#","bleed":{"value":-1,"unit":"mm"}"#.to_string(),
            PageField::Bleed,
            -1.0,
        ),
        (
            format!(
                r#","margins":{{"top":{zero},"right":{{"value":-5,"unit":"px"}},"bottom":{zero},"left":{zero}}}"#
            ),
            PageField::MarginRight,
            -5.0,
        ),
        (
            r#","grid":{"step":{"value":0,"unit":"px"}}"#.to_string(),
            PageField::GridStep,
            0.0,
        ),
        (
            r#","grid":{"step":{"value":501,"unit":"mm"}}"#.to_string(),
            PageField::GridStep,
            501.0,
        ),
        (
            r#","grid":{"subdivisions":0}"#.to_string(),
            PageField::GridSubdivisions,
            0.0,
        ),
        (
            r#","grid":{"subdivisions":11}"#.to_string(),
            PageField::GridSubdivisions,
            11.0,
        ),
    ];
    for (extra, field, value) in rows {
        let want = ValidationError::PageLayout {
            page: PAGE,
            error: PageLayoutError::OutOfRange { field, value },
        };
        match Document::from_json(&page_json(&extra)) {
            Err(LoadError::Invalid(got)) => assert_eq!(got, want, "{extra}"),
            other => panic!("{extra}: expected a validation error, got {other:?}"),
        }
    }
}

// FD-05(d) on load: the inclusive ends load unchanged.
#[test]
fn fd_05_fd_07_range_ends_on_load_are_kept_as_written() {
    let extra = concat!(
        r#","bleed":{"value":10,"unit":"mm"},"#,
        r#""margins":{"top":{"value":0,"unit":"px"},"right":{"value":0,"unit":"px"},"#,
        r#""bottom":{"value":0,"unit":"px"},"left":{"value":0,"unit":"px"}},"#,
        r#""grid":{"step":{"value":500,"unit":"px"},"subdivisions":10}"#
    );
    let doc = Document::from_json(&page_json(extra)).unwrap();
    let p = &doc.pages()[0];
    assert_eq!(p.bleed, mm(10.0));
    assert_eq!(p.grid.step, px(500.0));
    assert_eq!(p.grid.subdivisions, 10);
}

proptest! {
    // FD-05, FD-07: layout data inside the ranges always validates.
    #[test]
    fn arq_110_in_range_layout_data_always_validates(
        b in bleed(), m in margins(), g in guide_set(), s in grid_settings()
    ) {
        let mut p = page();
        p.bleed = b;
        p.margins = m;
        p.guides = g;
        p.grid = s;
        prop_assert_eq!(validate(p), Ok(()));
    }

    // FD-05: any negative margin is an error naming a margin side.
    #[test]
    fn fd_05_any_negative_margin_is_rejected(v in -1.0e9..-1.0e-9_f64, side in 0..4usize) {
        prop_assert_eq!(validate(with_margin(side, v)), out(SIDES[side], v));
    }
}

fn second_page() -> Page {
    let mut p = page();
    p.id = PageId::new(2);
    p
}

fn validate_two(first: Page, second: Page) -> Result<(), ValidationError> {
    let pages = vec![first.into(), second.into()];
    Document::from_parts(3, Dpi::new(96), pages).map(|_| ())
}

fn on_page_two(error: PageLayoutError) -> Result<(), ValidationError> {
    Err(ValidationError::PageLayout {
        page: PageId::new(2),
        error,
    })
}

// C2, design L1: every page is checked, not only the first. One bad field per
// case on the second page; the first page is fine.
#[test]
fn fd_05_fd_07_a_bad_value_on_a_later_page_is_found() {
    let bad_guide = Guide {
        axis: GuideAxis::Vertical,
        position: px(f64::NAN),
        locked: false,
    };
    let cases: Vec<(Page, PageLayoutError)> = vec![
        (
            Page {
                bleed: mm(11.0),
                ..second_page()
            },
            PageLayoutError::OutOfRange {
                field: PageField::Bleed,
                value: 11.0,
            },
        ),
        (
            with_margin(2, -1.0),
            PageLayoutError::OutOfRange {
                field: PageField::MarginBottom,
                value: -1.0,
            },
        ),
        (
            Page {
                guides: GuideSet(vec![bad_guide]),
                ..second_page()
            },
            PageLayoutError::NonFinite {
                field: PageField::GuidePosition,
            },
        ),
        (
            {
                let mut p = second_page();
                p.grid.step = px(0.0);
                p
            },
            PageLayoutError::OutOfRange {
                field: PageField::GridStep,
                value: 0.0,
            },
        ),
        (
            {
                let mut p = second_page();
                p.grid.subdivisions = 11;
                p
            },
            PageLayoutError::OutOfRange {
                field: PageField::GridSubdivisions,
                value: 11.0,
            },
        ),
    ];
    for (mut bad, error) in cases {
        bad.id = PageId::new(2);
        assert_eq!(validate_two(page(), bad), on_page_two(error), "{error:?}");
    }
}

// C2 on load: the second page of a file is checked too.
#[test]
fn fd_05_a_later_page_with_a_bad_value_is_rejected_on_load() {
    let size = r#""size":{"width":{"value":100,"unit":"px"},"height":{"value":50,"unit":"px"}}"#;
    let text = format!(
        r#"{{"schema_version":1,"next_id":3,"pages":[{{"id":1,{size}}},{{"id":2,{size},"bleed":{{"value":99,"unit":"mm"}}}}]}}"#
    );
    match Document::from_json(text.as_bytes()) {
        Err(LoadError::Invalid(got)) => assert_eq!(
            Err(got),
            on_page_two(PageLayoutError::OutOfRange {
                field: PageField::Bleed,
                value: 99.0
            })
        ),
        other => panic!("{other:?}"),
    }
}

// C3, design L1: the layout step runs after page sizes, for all pages. A
// document with a bad size and a bad layout value reports the size, whichever
// page holds which.
#[test]
fn design_l1_page_sizes_are_reported_before_layout_data() {
    let tiny = PageSize {
        width: px(0.5),
        height: px(50.0),
    };
    let same_page = Page {
        size: tiny,
        bleed: mm(11.0),
        ..page()
    };
    let size_error = |id: u64| {
        Err(ValidationError::PageSize {
            page: PageId::new(id),
            error: tiny.check(Dpi::new(96)).unwrap_err(),
        })
    };
    assert_eq!(validate(same_page), size_error(1));
    // Layout error on page 1, size error on page 2: the size still wins.
    let bad_layout = Page {
        bleed: mm(11.0),
        ..page()
    };
    let bad_size = Page {
        size: tiny,
        ..second_page()
    };
    assert_eq!(validate_two(bad_layout, bad_size), size_error(2));
}
