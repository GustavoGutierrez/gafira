//! ARQ-110 at the extremes: no `Ok` carries an infinity, in layouts or in lists; and the
//! 0.05 em thickness of a decoration whose table is missing (FD-41).

use gafira_geom::Size;
use gafira_testkit::fonts;
use gafira_text::{ListItem, ListKind, TextError, TextStyle, layout_box, layout_list};

const INTER: &[u8] = fonts::INTER.bytes;

fn non_finite_output() -> Option<TextError> {
    Some(TextError::NonFinite { field: "output" })
}

#[test]
fn arq_110_an_infinite_height_with_finite_lines_is_an_error() {
    // 1000 pt at 5.2e36 dpi: every baseline fits in f32 (the last is 3.3e38), the height does not.
    let s = TextStyle {
        size_pt: 1000.0,
        ..TextStyle::default()
    };
    let out = layout_box("ab\ncd", &[INTER], &s, 5.2e36, Size::new(10.0, 10.0));
    assert_eq!(out.err(), non_finite_output());
}

fn big_style(letter_spacing: f64) -> TextStyle {
    TextStyle {
        size_pt: 1000.0,
        letter_spacing,
        ..TextStyle::default()
    }
}

#[test]
fn arq_110_a_list_indent_that_overflows_is_an_error() {
    // Level 4: the indent is 4.5 em, which passes f32::MAX before the text does.
    let items = [ListItem {
        text: "",
        level: 4,
        kind: ListKind::Bullet,
    }];
    let out = layout_list(
        &items,
        &[INTER],
        &big_style(0.0),
        5.76e36,
        Size::new(10.0, 10.0),
    );
    assert_eq!(out.err(), non_finite_output());
}

#[test]
fn arq_110_a_list_top_that_overflows_is_an_error() {
    let item = |text| ListItem {
        text,
        level: 1,
        kind: ListKind::Bullet,
    };
    let items = [item("a\nb"), item("c\nd"), item("e\nf")];
    let out = layout_list(
        &items,
        &[INTER],
        &big_style(500.0),
        6e36,
        Size::new(10.0, 10.0),
    );
    assert_eq!(out.err(), non_finite_output());
}

#[test]
fn arq_110_a_list_at_a_huge_but_workable_size_has_only_finite_numbers() {
    let items = [ListItem {
        text: "a",
        level: 2,
        kind: ListKind::Numbered,
    }];
    let e = layout_list(
        &items,
        &[INTER],
        &big_style(0.0),
        1e30,
        Size::new(10.0, 10.0),
    )
    .expect("finite");
    assert!(e[0].indent.is_finite() && e[0].top.is_finite());
}

// ---- a missing table: 0.05 em ------------------------------------------------------------

/// A copy whose table directory calls `tag` something else, so the table is absent.
fn without_table(font: &[u8], tag: &[u8; 4]) -> Vec<u8> {
    let count = usize::from(u16::from_be_bytes([font[4], font[5]]));
    let mut copy = font.to_vec();
    let rec = (0..count)
        .map(|i| 12 + 16 * i)
        .find(|rec| &font[*rec..*rec + 4] == tag)
        .expect("table");
    copy[rec..rec + 4].copy_from_slice(b"zzzz");
    copy
}

fn decorated(font: &[u8]) -> (f32, f32) {
    let s = TextStyle {
        size_pt: 75.0,
        underline: true,
        strikethrough: true,
        ..TextStyle::default()
    };
    let out = layout_box("a", &[font], &s, 96.0, Size::new(1000.0, 1000.0)).expect("layout");
    let c = &out.lines[0].clusters[0];
    (
        c.underline.expect("underline").thickness,
        c.strikethrough.expect("strikethrough").thickness,
    )
}

#[test]
fn fd_41_a_font_without_post_or_os2_gives_a_thickness_of_five_hundredths_of_an_em() {
    // 75 pt at 96 dpi is 100 px, so 0.05 em is 5 px and parley's 1/18 em is 5.56 px.
    let (u, s) = decorated(&without_table(INTER, b"post"));
    assert!((u - 5.0).abs() <= 0.01, "underline {u}");
    assert!(
        (s - 140.0 * 100.0 / 2048.0).abs() <= 0.01,
        "strikethrough keeps OS/2: {s}"
    );
    let (u, s) = decorated(&without_table(INTER, b"OS/2"));
    assert!((s - 5.0).abs() <= 0.01, "strikethrough {s}");
    assert!(
        (u - 140.0 * 100.0 / 2048.0).abs() <= 0.01,
        "underline keeps post: {u}"
    );
}
