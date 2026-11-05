//! The duration of a page: the trigger of the `gafira-doc -> gafira-time`
//! edge of ADR 0007 (ARQ-01, ARQ-10, ARQ-13, ARQ-136).
//!
//! The slice card of the change `gafira-timeline` names these as AC1 (the
//! edge), AC3 (the wire form omits an absent duration) and AC7 (the field
//! takes part in equality and in the canonical order of the bytes).

use gafira_doc::{Document, Page, PageId, PageSize};
use gafira_geom::{Length, Unit};

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
    r#""style":"lines","snap":true},"locked":false,"hidden":false,"roots":[],"nodes":[]}]}"#
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
    r#""hidden":false,"roots":[],"nodes":[]}]}"#
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
