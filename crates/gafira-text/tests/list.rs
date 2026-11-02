//! FD-41 (j) and Listas: the marker of level 1 sits at the edge, a level indents 1.5 em,
//! the text hangs under the widest marker of its level, numbers count per level, bullets
//! cycle through three marks, and a list has up to 4 levels.

use gafira_geom::Size;
use gafira_testkit::fonts;
use gafira_text::{
    ListEntry, ListItem, ListKind, TextAlign, TextError, TextStyle, layout_box, layout_list,
};

const INTER: &[u8] = fonts::INTER.bytes;
/// 24 pt at 96 dpi.
const SIZE_PX: f64 = 32.0;

fn style() -> TextStyle {
    TextStyle {
        size_pt: 24.0,
        ..TextStyle::default()
    }
}

fn item(text: &str, level: u8, kind: ListKind) -> ListItem<'_> {
    ListItem { text, level, kind }
}

fn lay_in(items: &[ListItem<'_>], width: f64) -> Result<Vec<ListEntry>, TextError> {
    layout_list(items, &[INTER], &style(), 96.0, Size::new(width, 1000.0))
}

fn lay(items: &[ListItem<'_>]) -> Result<Vec<ListEntry>, TextError> {
    lay_in(items, 2000.0)
}

fn markers(entries: &[ListEntry]) -> Vec<&str> {
    entries.iter().map(|e| e.marker.as_str()).collect()
}

/// The x of the first cluster of the first line of an entry.
fn text_x(e: &ListEntry) -> f64 {
    f64::from(e.layout.lines[0].clusters[0].x)
}

use ListKind::{Bullet, Numbered};

#[test]
fn fd_41_j_a_list_of_three_levels_puts_level_1_at_the_edge_and_indents_1_5_em_per_level() {
    let entries = lay(&[
        item("a", 1, Bullet),
        item("b", 2, Bullet),
        item("c", 3, Bullet),
    ])
    .expect("valid list");
    assert_eq!(entries.len(), 3);
    for (depth, e) in entries.iter().enumerate() {
        let expected = depth as f64 * 1.5 * SIZE_PX;
        assert!(
            (f64::from(e.indent) - expected).abs() <= 0.01,
            "level {}: {}",
            depth + 1,
            e.indent
        );
    }
    assert_eq!(entries[0].indent, 0.0);
}

#[test]
fn fd_41_j_items_of_one_level_number_1_2_3() {
    let entries = lay(&[
        item("a", 1, Numbered),
        item("b", 1, Numbered),
        item("c", 1, Numbered),
    ])
    .expect("valid list");
    assert_eq!(markers(&entries), ["1.", "2.", "3."]);
}

#[test]
fn fd_41_j_numbering_restarts_at_each_new_descent() {
    let entries = lay(&[
        item("a", 1, Numbered),
        item("b", 2, Numbered),
        item("c", 2, Numbered),
        item("d", 1, Numbered),
        item("e", 2, Numbered),
        item("f", 3, Numbered),
        item("g", 2, Numbered),
    ])
    .expect("valid list");
    assert_eq!(
        markers(&entries),
        ["1.", "1.", "2.", "2.", "1.", "1.", "2."]
    );
}

#[test]
fn fd_41_j_a_bullet_neither_adds_to_nor_restarts_the_count_of_its_level() {
    let entries = lay(&[
        item("a", 1, Numbered),
        item("b", 1, Numbered),
        item("c", 1, Bullet),
        item("d", 1, Numbered),
    ])
    .expect("valid list");
    assert_eq!(markers(&entries), ["1.", "2.", "\u{2022}", "3."]);
}

#[test]
fn fd_41_j_an_item_of_a_shallower_level_restarts_the_deeper_levels_even_if_it_is_a_bullet() {
    let entries = lay(&[
        item("a", 1, Numbered),
        item("b", 2, Numbered),
        item("c", 2, Numbered),
        item("d", 1, Bullet),
        item("e", 2, Numbered),
    ])
    .expect("valid list");
    assert_eq!(markers(&entries), ["1.", "1.", "2.", "\u{2022}", "1."]);
}

#[test]
fn fd_41_j_bullets_cycle_through_three_marks_over_four_levels() {
    let items: Vec<ListItem<'_>> = (1..=4).map(|l| item("x", l, Bullet)).collect();
    let entries = lay(&items).expect("valid list");
    assert_eq!(
        markers(&entries),
        ["\u{2022}", "\u{25e6}", "\u{25aa}", "\u{2022}"]
    );
}

#[test]
fn fd_41_j_a_fifth_level_or_level_0_is_out_of_range() {
    for level in [0_u8, 5, 255] {
        assert_eq!(
            lay(&[item("a", level, Bullet)]).err(),
            Some(TextError::OutOfRange {
                field: "items.level"
            }),
            "level {level}"
        );
    }
    assert!(lay(&[item("a", 4, Bullet)]).is_ok());
    assert!(lay(&[item("a", 1, Bullet)]).is_ok());
}

fn marker_advance(marker: &str) -> f64 {
    let out =
        layout_box(marker, &[INTER], &style(), 96.0, Size::new(10_000.0, 100.0)).expect("layout");
    f64::from(out.lines[0].advance)
}

fn ten_numbered<'a>(last: &'a str) -> Vec<ListItem<'a>> {
    let mut items: Vec<ListItem<'a>> = (0..9).map(|_| item("short", 1, Numbered)).collect();
    items.push(item(last, 1, Numbered));
    items
}

#[test]
fn fd_41_j_nine_and_ten_start_their_text_at_the_same_x_and_so_does_a_wrapped_line() {
    let long = "alpha beta gamma delta epsilon zeta eta theta iota kappa";
    let entries = lay_in(&ten_numbered(long), 400.0).expect("list");
    assert_eq!(entries[8].marker, "9.");
    assert_eq!(entries[9].marker, "10.");
    let x9 = text_x(&entries[8]);
    assert!(
        (text_x(&entries[9]) - x9).abs() <= 0.01,
        "{} vs {x9}",
        text_x(&entries[9])
    );
    assert!(entries[9].layout.lines.len() > 1, "the tenth item wraps");
    for line in &entries[9].layout.lines {
        let x = f64::from(line.clusters[0].x);
        assert!((x - x9).abs() <= 0.01, "a line starts at {x}, not {x9}");
    }
    // The text starts after the marker and a space.
    assert!(x9 > marker_advance("10."), "{x9}");
}

#[test]
fn fd_41_j_the_text_hangs_under_the_widest_marker_of_the_level_in_the_list() {
    let alone = lay(&[item("a", 1, Numbered)]).expect("list");
    let with_ten = lay(&ten_numbered("a")).expect("list");
    let growth = text_x(&with_ten[0]) - text_x(&alone[0]);
    let expected = marker_advance("10.") - marker_advance("1.");
    assert!((growth - expected).abs() <= 0.01, "{growth} vs {expected}");
}

#[test]
fn fd_41_j_a_narrower_marker_after_the_widest_does_not_pull_the_text_back() {
    let mut items = ten_numbered("a");
    let wide = text_x(&lay(&items).expect("list")[0]);
    items.push(item("bullet", 1, Bullet));
    let entries = lay(&items).expect("list");
    assert!((text_x(&entries[0]) - wide).abs() <= 0.01);
    assert!((text_x(&entries[10]) - wide).abs() <= 0.01);
}

#[test]
fn fd_41_j_each_level_hangs_under_its_own_widest_marker_after_its_indent() {
    let items = [
        item("a", 1, Numbered),
        item("b", 2, Numbered),
        item("c", 2, Bullet),
    ];
    let entries = lay(&items).expect("list");
    // Same level, same x; a deeper level starts 1.5 em further right.
    assert!((text_x(&entries[1]) - text_x(&entries[2])).abs() <= 0.01);
    assert!(text_x(&entries[1]) > f64::from(entries[1].indent));
    assert!(text_x(&entries[0]) > 0.0);
}

#[test]
fn fd_41_j_the_ranges_of_an_entry_index_its_own_text() {
    let entries = lay(&[item("one \u{e9}", 1, Numbered), item("two", 2, Bullet)]).expect("list");
    for (e, text) in entries.iter().zip(["one \u{e9}", "two"]) {
        let mut ranges: Vec<_> = e
            .layout
            .lines
            .iter()
            .flat_map(|l| l.clusters.iter().map(|c| c.range.clone()))
            .collect();
        ranges.sort_by_key(|r| r.start);
        let joined: String = ranges.into_iter().map(|r| &text[r]).collect();
        assert_eq!(joined, text);
    }
}

#[test]
fn fd_41_j_items_stack_by_the_height_of_the_ones_above() {
    // One line of Inter at 32 px is its natural height, 38.72 px (OS/2 and hhea agree).
    let line = (1984.0 + 494.0) * SIZE_PX / 2048.0;
    let entries = lay(&[
        item("one", 1, Numbered),
        item("two", 1, Numbered),
        item("a\nb", 1, Numbered),
        item("four", 1, Numbered),
    ])
    .expect("list");
    let tops: Vec<f64> = entries.iter().map(|e| f64::from(e.top)).collect();
    for (got, lines) in tops.iter().zip([0.0, 1.0, 2.0, 4.0]) {
        assert!((got - lines * line).abs() <= 0.01, "{tops:?}");
    }
}

#[test]
fn fd_41_j_an_item_wraps_in_the_box_minus_its_indent_and_marker() {
    let text = "alpha beta gamma delta epsilon zeta eta theta";
    let items = [item(text, 3, Bullet)];
    let e = &lay_in(&items, 300.0).expect("list")[0];
    assert!(e.layout.lines.len() > 1);
    for line in &e.layout.lines {
        // The white space at the end of a line hangs past the box.
        let right = line
            .clusters
            .iter()
            .filter(|c| !text[c.range.clone()].trim().is_empty())
            .map(|c| c.x + c.advance)
            .fold(0.0, f32::max);
        assert!(right <= 300.0 + 0.01, "{right}");
    }
    // The indent alone is wider than the box: no error.
    assert!(lay_in(&items, 50.0).is_ok());
}

#[test]
fn arq_110_a_list_checks_its_numbers_then_its_fonts_then_its_levels() {
    let bad = TextStyle {
        size_pt: f64::NAN,
        ..style()
    };
    let items = [item("a", 9, Bullet)];
    let size = Size::new(100.0, 100.0);
    assert_eq!(
        layout_list(&items, &[INTER], &bad, 96.0, size).err(),
        Some(TextError::NonFinite {
            field: "style.size_pt"
        })
    );
    assert_eq!(
        layout_list(&items, &[INTER], &style(), f64::NAN, size).err(),
        Some(TextError::NonFinite { field: "dpi" })
    );
    // No fonts and a bad level: the fonts come first.
    assert_eq!(
        layout_list(&items, &[], &style(), 96.0, size).err(),
        Some(TextError::NoFont)
    );
    // NaN with no fonts: the number comes first.
    assert_eq!(
        layout_list(&items, &[], &bad, 96.0, size).err(),
        Some(TextError::NonFinite {
            field: "style.size_pt"
        })
    );
}

#[test]
fn fd_41_an_empty_font_list_is_an_error_for_a_list_with_no_items_too() {
    let size = Size::new(100.0, 100.0);
    assert_eq!(
        layout_list(&[], &[], &style(), 96.0, size).err(),
        Some(TextError::NoFont)
    );
    assert_eq!(
        layout_list(&[], &[INTER], &style(), 96.0, size).expect("empty"),
        vec![]
    );
}

// ---- the hang does not depend on the alignment or on a box --------------------------------

const ITEM_TEXT: &str = "aa bb cc dd ee ff gg hh ii jj kk ll mm nn oo pp qq rr";

fn aligned(align: TextAlign, width: f64) -> Vec<ListEntry> {
    let s = TextStyle { align, ..style() };
    layout_list(
        &[item(ITEM_TEXT, 1, Numbered), item(ITEM_TEXT, 2, Bullet)],
        &[INTER],
        &s,
        96.0,
        Size::new(width, 1000.0),
    )
    .expect("valid list")
}

#[test]
fn fd_41_j_the_alignment_of_the_list_does_not_change_where_the_text_starts_or_its_room() {
    let width = 420.0;
    let left = aligned(TextAlign::Left, width);
    for align in [TextAlign::Center, TextAlign::Right, TextAlign::Justify] {
        let other = aligned(align, width);
        for (l, o) in left.iter().zip(&other) {
            let start = text_x(l);
            assert_eq!(
                o.layout.lines.len(),
                l.layout.lines.len(),
                "{align:?}: the room of the item"
            );
            assert!(!o.layout.overflow, "{align:?}");
            // The white space at the end of a line hangs past the box: only ink is checked.
            let ink = o.layout.lines.iter().flat_map(|line| &line.clusters);
            for c in ink.filter(|c| !ITEM_TEXT[c.range.clone()].trim().is_empty()) {
                assert!(
                    f64::from(c.x) >= start - 0.01 && f64::from(c.x + c.advance) <= width + 0.01,
                    "{align:?}: a cluster at {} past the hang {start}",
                    c.x
                );
            }
        }
    }
}

#[test]
fn fd_41_j_the_hang_is_in_proportion_to_the_dpi_at_any_dpi() {
    let hang = |dpi: f64| {
        let entries = layout_list(
            &[item("a", 1, Numbered)],
            &[INTER],
            &style(),
            dpi,
            Size::new(1e15, 1e15),
        )
        .expect("valid list");
        text_x(&entries[0])
    };
    let base = hang(96.0);
    assert!(base > 0.0);
    for dpi in [960.0, 1e6, 1e10] {
        let scaled = hang(dpi);
        let expected = base * dpi / 96.0;
        assert!(
            (scaled - expected).abs() <= expected * 1e-4,
            "dpi {dpi}: hang {scaled}, expected {expected}"
        );
    }
}
