//! FD-03(b) guides survive save and open; FD-05 page layout data; ADR 0006
//! always-written wire form.

use gafira_doc::{
    Document, GridSettings, GridStyle, Guide, GuideAxis, GuideSet, Margins, Page, PageId, PageSize,
};
use gafira_geom::{Length, Unit};
use gafira_testkit::{bleed, grid_settings, guide_set, margins};
use proptest::prelude::*;

const PRE_MINIMAL: &str = include_str!("golden/pre_layout/minimal.json");
const PRE_FULL: &str = include_str!("golden/pre_layout/full.json");
const PRE_EXTENSION: &str = include_str!("golden/pre_layout/extension.json");

fn px(v: f64) -> Length {
    Length::new(v, Unit::Px)
}

fn size() -> PageSize {
    PageSize {
        width: px(100.0),
        height: px(50.0),
    }
}

fn page() -> Page {
    Page::new(PageId::new(1), "P".to_string(), size())
}

fn json(p: &Page) -> String {
    serde_json::to_string(p).unwrap()
}

fn doc_with(p: Page) -> Document {
    let text = format!(
        r#"{{"schema_version":1,"next_id":2,"pages":[{}]}}"#,
        json(&p)
    );
    Document::from_json(text.as_bytes()).unwrap()
}

fn guide(axis: GuideAxis, v: f64, unit: Unit, locked: bool) -> Guide {
    Guide {
        axis,
        position: Length::new(v, unit),
        locked,
    }
}

// FD-05 (ADR 0006): a document saved before the layout fields existed loads
// with their defaults, and the schema stays 1.
#[test]
fn fd_05_documents_without_layout_fields_load_with_defaults() {
    for (name, text) in [
        ("minimal", PRE_MINIMAL),
        ("full", PRE_FULL),
        ("extension", PRE_EXTENSION),
    ] {
        assert!(!text.contains("bleed"), "{name} must be the old bytes");
        let doc = Document::from_json(text.as_bytes()).unwrap();
        assert_eq!(doc.schema_version(), 1, "{name}");
        for p in doc.pages() {
            assert_eq!(p.bleed, Length::new(0.0, Unit::Mm), "{name} bleed");
            assert_eq!(p.margins, Margins::default(), "{name} margins");
            assert!(p.guides.0.is_empty(), "{name} guides");
            assert_eq!(p.grid, GridSettings::default(), "{name} grid");
        }
    }
}

// ADR 0006 consequences: the old bytes load and re-save in the new canonical
// form, which is the current golden.
#[test]
fn adr_0006_old_bytes_resave_in_the_canonical_form() {
    let resave = |t: &str| {
        Document::from_json(t.as_bytes())
            .unwrap()
            .to_json()
            .unwrap()
    };
    assert_eq!(resave(PRE_MINIMAL), include_str!("golden/minimal.json"));
    assert_eq!(resave(PRE_EXTENSION), include_str!("golden/extension.json"));
    // `full.json` now carries layout data on page 1 and a duration on page 2;
    // the bytes of the previous session carry neither.
    let full = resave(PRE_FULL);
    let mut want: serde_json::Value =
        serde_json::from_str(include_str!("golden/full.json")).unwrap();
    let got: serde_json::Value = serde_json::from_str(&full).unwrap();
    let fresh = &got["pages"][0];
    for key in ["bleed", "margins", "guides", "grid"] {
        want["pages"][0][key] = fresh[key].clone();
    }
    // The duration is absent in the wire form when the page has none, so the
    // old bytes re-save in the canonical form of a document without one.
    want["pages"][1]
        .as_object_mut()
        .expect("a page is an object")
        .remove("duration");
    assert_eq!(got, want);
    assert_eq!(resave(&full), full);
}

fn page_with(extra: &str) -> Result<Page, serde_json::Error> {
    let size = r#""size":{"width":{"value":1,"unit":"px"},"height":{"value":2,"unit":"px"}}"#;
    serde_json::from_str(&format!(r#"{{"id":1,{size}{extra}}}"#))
}

// ADR 0006 rule 4, FD-07: every grid field reads its default when absent.
#[test]
fn fd_07_each_absent_grid_field_reads_its_default() {
    let full = [
        r#""visible":true"#,
        r#""step":{"value":2.0,"unit":"mm"}"#,
        r#""subdivisions":4"#,
        r#""style":"dots""#,
        r#""snap":false"#,
    ];
    let d = GridSettings::default();
    for skip in 0..full.len() {
        let kept: Vec<&str> = full
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != skip)
            .map(|(_, f)| *f)
            .collect();
        let p = page_with(&format!(r#","grid":{{{}}}"#, kept.join(","))).unwrap();
        let g = p.grid;
        let want = GridSettings {
            visible: if skip == 0 { d.visible } else { true },
            step: if skip == 1 {
                d.step
            } else {
                Length::new(2.0, Unit::Mm)
            },
            subdivisions: if skip == 2 { d.subdivisions } else { 4 },
            style: if skip == 3 { d.style } else { GridStyle::Dots },
            snap: if skip == 4 { d.snap } else { false },
        };
        assert_eq!(g, want, "without {}", full[skip]);
    }
    assert_eq!(page_with(r#","grid":{}"#).unwrap().grid, d);
}

// FD-05: `linked` reads true when absent; FD-03: `locked` reads false.
#[test]
fn fd_05_absent_linked_and_locked_read_their_defaults() {
    let z = r#"{"value":0.0,"unit":"px"}"#;
    let m = format!(r#","margins":{{"top":{z},"right":{z},"bottom":{z},"left":{z}}}"#);
    assert!(page_with(&m).unwrap().margins.linked);
    let g = format!(r#","guides":[{{"axis":"vertical","position":{z}}}]"#);
    let guides = page_with(&g).unwrap().guides;
    assert_eq!(guides.0.len(), 1);
    assert!(!guides.0[0].locked);
}

// ADR 0006 rule 4: GridStyle and GuideAxis are closed; unknown values fail.
#[test]
fn adr_0006_unknown_enum_values_fail_to_load() {
    assert!(page_with(r#","grid":{"style":"cross"}"#).is_err());
    let z = r#"{"value":0.0,"unit":"px"}"#;
    let g = format!(r#","guides":[{{"axis":"diagonal","position":{z},"locked":false}}]"#);
    assert!(page_with(&g).is_err());
}

// FD-05, FD-07: the defaults the spec states.
#[test]
fn fd_05_fd_07_defaults_match_the_spec() {
    let p = page();
    assert_eq!(p.bleed, Length::new(0.0, Unit::Mm));
    let m = p.margins;
    for side in [m.top, m.right, m.bottom, m.left] {
        assert_eq!(side, px(0.0));
    }
    assert!(m.linked);
    assert!(p.guides.0.is_empty());
    let g = p.grid;
    assert!(!g.visible);
    assert_eq!(g.step, px(10.0));
    assert_eq!(g.subdivisions, 1);
    assert_eq!(g.style, GridStyle::Lines);
    assert!(g.snap);
}

// ADR 0006: every layout field is written, defaults included, in 02 §3.1
// order after `size`.
#[test]
fn adr_0006_default_valued_layout_fields_are_written() {
    let zero = r#"{"value":0.0,"unit":"px"}"#;
    let want = format!(
        concat!(
            r#""bleed":{{"value":0.0,"unit":"mm"}},"#,
            r#""margins":{{"top":{z},"right":{z},"bottom":{z},"left":{z},"linked":true}},"#,
            r#""guides":[],"#,
            r#""grid":{{"visible":false,"step":{{"value":10.0,"unit":"px"}},"#,
            r#""subdivisions":1,"style":"lines","snap":true}},"locked":false"#,
        ),
        z = zero
    );
    let out = json(&page());
    assert!(out.contains(&want), "{out}");
    let height = r#""height":{"value":50.0,"unit":"px"}},"#;
    assert!(out.contains(&format!("{height}{want}")), "{out}");
}

#[test]
fn grid_style_and_guide_axis_use_snake_case_on_the_wire() {
    let mut p = page();
    p.grid.style = GridStyle::Dots;
    p.guides = GuideSet(vec![
        guide(GuideAxis::Vertical, 1.0, Unit::Px, false),
        guide(GuideAxis::Horizontal, 2.0, Unit::Px, true),
    ]);
    let out = json(&p);
    assert!(out.contains(r#""style":"dots""#), "{out}");
    assert!(out.contains(r#""axis":"vertical""#), "{out}");
    assert!(out.contains(r#""axis":"horizontal""#), "{out}");
    assert_eq!(serde_json::from_str::<Page>(&out).unwrap(), p);
}

// FD-03(b): each field of a guide survives save and open, unrounded and in
// the unit it was written in.
#[test]
fn fd_03_every_guide_field_survives_save_and_open() {
    let rows = [
        guide(GuideAxis::Vertical, 12.345_678_901_234_5, Unit::Mm, true),
        guide(GuideAxis::Horizontal, -3.0, Unit::In, false),
        guide(GuideAxis::Vertical, 1e9, Unit::Pt, false),
        guide(GuideAxis::Horizontal, 0.0, Unit::Cm, true),
    ];
    for g in rows {
        let mut p = page();
        p.guides = GuideSet(vec![g]);
        let back = doc_with(p.clone());
        let got = back.pages()[0].guides.0[0];
        assert_eq!(got.axis, g.axis);
        assert_eq!(got.locked, g.locked);
        assert_eq!(got.position, g.position);
        let again = Document::from_json(back.to_json().unwrap().as_bytes()).unwrap();
        assert_eq!(again, back);
    }
}

// FD-03(b): the order is kept and duplicates are not merged.
#[test]
fn fd_03_guide_order_and_duplicates_survive() {
    let a = guide(GuideAxis::Vertical, 50.0, Unit::Px, false);
    let b = guide(GuideAxis::Horizontal, 5.0, Unit::Px, true);
    let c = guide(GuideAxis::Vertical, 1.0, Unit::Px, false);
    let set = vec![a, b, a, c, a];
    let mut p = page();
    p.guides = GuideSet(set.clone());
    let back = doc_with(p);
    assert_eq!(back.pages()[0].guides.0, set);
}

// FD-05, FD-07: bleed, margins and grid keep every field through a round trip.
#[test]
fn fd_05_fd_07_layout_fields_survive_save_and_open() {
    let mut p = page();
    p.bleed = Length::new(3.0, Unit::Mm);
    p.margins = Margins {
        top: px(1.0),
        right: Length::new(2.0, Unit::Mm),
        bottom: Length::new(3.0, Unit::Cm),
        left: Length::new(4.0, Unit::In),
        linked: false,
    };
    p.grid = GridSettings {
        visible: true,
        step: Length::new(2.5, Unit::Mm),
        subdivisions: 4,
        style: GridStyle::Dots,
        snap: false,
    };
    let back = doc_with(p.clone());
    assert_eq!(*back.pages()[0], p);
}

proptest! {
    // FD-03(b), FD-05, FD-07: any layout data survives and re-saves identically.
    #[test]
    fn layout_data_round_trips(
        b in bleed(), m in margins(), g in guide_set(), s in grid_settings()
    ) {
        let mut p = page();
        (p.bleed, p.margins, p.guides, p.grid) = (b, m, g, s);
        let doc = doc_with(p.clone());
        prop_assert_eq!(&*doc.pages()[0], &p);
        let text = doc.to_json().unwrap();
        prop_assert_eq!(Document::from_json(text.as_bytes()).unwrap().to_json().unwrap(), text);
    }
}
