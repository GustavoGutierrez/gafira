//! The duration of a page: the trigger of the `gafira-doc -> gafira-time`
//! edge of ADR 0007 (ARQ-01, ARQ-10, ARQ-13, ARQ-136).
//!
//! The slice card of the change `gafira-timeline` names these as AC1 (the
//! edge), AC3 (the wire form omits an absent duration) and AC7 (the field
//! takes part in equality and in the canonical order of the bytes).

use gafira_doc::{
    Document, LoadError, Page, PageField, PageId, PageLayoutError, PageSize, ValidationError,
};
use gafira_geom::{Length, Unit};
use gafira_time::Flicks;

/// A page size of 100 by 200 pixels.
fn size() -> PageSize {
    PageSize {
        width: Length::new(100.0, Unit::Px),
        height: Length::new(200.0, Unit::Px),
    }
}

/// The canonical bytes of a document whose only page has no duration: what a
/// document written before the edge carries (ARQ-10, ADR 0006 rule 1).
const DOC_WITHOUT_DURATION: &str = concat!(
    r#"{"schema_version":1,"next_id":2,"dpi":96,"pages":[{"id":1,"name":"P","#,
    r#""size":{"width":{"value":100.0,"unit":"px"},"height":{"value":200.0,"unit":"px"}},"#,
    r#""bleed":{"value":0.0,"unit":"mm"},"margins":{"top":{"value":0.0,"unit":"px"},"#,
    r#""right":{"value":0.0,"unit":"px"},"bottom":{"value":0.0,"unit":"px"},"#,
    r#""left":{"value":0.0,"unit":"px"},"linked":true},"guides":[],"#,
    r#""grid":{"visible":false,"step":{"value":10.0,"unit":"px"},"subdivisions":1,"#,
    r#""style":"lines","snap":true},"locked":false,"hidden":false,"roots":[],"nodes":[]}],"#,
    r#""sequences":[],"assets":{}}"#
);

/// The same document with a five-second duration on the page, in flicks:
/// 5 s `*` 705 600 000 flicks per second (V-2). The field sits after the grid
/// and before `locked`, which is where §3.1 puts it.
const DOC_WITH_DURATION: &str = concat!(
    r#"{"schema_version":1,"next_id":2,"dpi":96,"pages":[{"id":1,"name":"P","#,
    r#""size":{"width":{"value":100.0,"unit":"px"},"height":{"value":200.0,"unit":"px"}},"#,
    r#""bleed":{"value":0.0,"unit":"mm"},"margins":{"top":{"value":0.0,"unit":"px"},"#,
    r#""right":{"value":0.0,"unit":"px"},"bottom":{"value":0.0,"unit":"px"},"#,
    r#""left":{"value":0.0,"unit":"px"},"linked":true},"guides":[],"#,
    r#""grid":{"visible":false,"step":{"value":10.0,"unit":"px"},"subdivisions":1,"#,
    r#""style":"lines","snap":true},"duration":3528000000,"locked":false,"#,
    r#""hidden":false,"roots":[],"nodes":[]}],"sequences":[],"assets":{}}"#
);

// ARQ-01 (AC1): the crate declares the `gafira-time` edge of ADR 0007, the
// one whose trigger is `Page.duration`. The layer table of the CI step (V5)
// is checked by the command gate, not here.
#[test]
fn arq_01_the_crate_declares_the_gafira_time_edge() {
    let manifest = include_str!("../Cargo.toml");
    let declared = manifest
        .lines()
        .map(str::trim)
        .any(|line| line == "gafira-time.workspace = true");
    assert!(
        declared,
        "the crate must declare `gafira-time.workspace = true`:\n{manifest}"
    );
}

// ARQ-13, ARQ-10 (AC3): with no duration the field is absent in the wire
// form, so a document written before the edge re-saves byte for byte, and the
// round trip keeps the page without a duration.
#[test]
fn arq_13_a_page_without_a_duration_writes_no_field() {
    let document =
        Document::from_json(DOC_WITHOUT_DURATION.as_bytes()).expect("the old bytes load");
    let resaved = document.to_json().expect("the document writes");
    assert_eq!(
        resaved, DOC_WITHOUT_DURATION,
        "a document without a duration must re-save without the field"
    );
    assert!(
        !resaved.contains("duration"),
        "the field is omitted, not written as null:\n{resaved}"
    );
    let fresh = Page::new(PageId::new(1), "P".to_string(), size());
    let page = serde_json::to_string(&fresh).expect("the page writes");
    assert!(
        !page.contains("duration"),
        "a page built without a duration writes no field:\n{page}"
    );
}

// ARQ-10, ARQ-13 (AC3, AC5, AC7): a page that carries a duration writes it as
// a count of flicks in the canonical position, and reads it back unchanged.
#[test]
fn arq_10_a_page_duration_round_trips_through_the_wire_form() {
    let document = Document::from_json(DOC_WITH_DURATION.as_bytes()).expect("the bytes load");
    let resaved = document.to_json().expect("the document writes");
    assert_eq!(
        resaved, DOC_WITH_DURATION,
        "a page duration must round trip in its canonical position"
    );
}

// ARQ-10 (AC7): the duration takes part in the equality of the document, so
// two documents that differ only in the duration of a page are not the same.
#[test]
fn arq_10_the_duration_takes_part_in_the_equality_of_the_document() {
    let without = Document::from_json(DOC_WITHOUT_DURATION.as_bytes()).expect("the bytes load");
    let with = Document::from_json(DOC_WITH_DURATION.as_bytes()).expect("the bytes load");
    assert_ne!(
        without, with,
        "the duration of a page must take part in equality"
    );
    assert_eq!(without.pages()[0].id, PageId::new(1));
    assert_eq!(with.pages()[0].size, size());
    assert_eq!(with.clone(), with, "cloning a document keeps its duration");
}

// ARQ-136, V-2 (AC2): the duration is an `Option<Flicks>` of `gafira-time`,
// not an integer of the crate's own. The type is the evidence: the two
// bindings below only compile with that exact type.
#[test]
fn arq_136_the_duration_is_a_flicks_of_gafira_time() {
    let mut page = Page::new(PageId::new(1), "P".to_string(), size());
    let none: Option<Flicks> = page.duration;
    assert_eq!(none, None, "a page has no duration by default");

    page.duration = Some(Flicks::from_millis(1_500).expect("the time fits"));
    let ticks: Option<i64> = page.duration.map(Flicks::ticks);
    assert_eq!(ticks, Some(1_058_400_000), "one and a half seconds");
    let saved: Option<Flicks> = page.duration;
    assert_eq!(saved, Some(Flicks::from_millis(1_500).unwrap()));
}

// ARQ-13 (design `TL1`: a page duration is a `Flicks` value >= 0): the load
// validates the document, so a negative duration is refused and zero is a
// duration the page may have.
#[test]
fn arq_13_a_negative_page_duration_fails_to_load() {
    let negative = DOC_WITH_DURATION.replace(r#""duration":3528000000"#, r#""duration":-5"#);
    match Document::from_json(negative.as_bytes()) {
        Err(LoadError::Invalid(ValidationError::PageLayout { page, error })) => {
            assert_eq!(page, PageId::new(1), "the failing page is reported");
            assert_eq!(
                error,
                PageLayoutError::OutOfRange {
                    field: PageField::DurationTicks,
                    value: -5.0,
                },
                "the range error names the duration and the value"
            );
        }
        other => panic!("a negative page duration must fail to load, got {other:?}"),
    }

    let zero = DOC_WITH_DURATION.replace(r#""duration":3528000000"#, r#""duration":0"#);
    let loaded = Document::from_json(zero.as_bytes()).expect("a zero duration loads");
    assert_eq!(
        loaded.pages()[0].duration,
        Some(Flicks::from_ticks(0)),
        "zero is a duration, not the absence of one"
    );
}
