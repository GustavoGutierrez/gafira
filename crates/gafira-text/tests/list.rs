//! FD-41 (j): a list indents 1.5 em per level and numbers 1, 2, 3 at one level; bullets
//! cycle through three marks and a list has up to 4 levels.

use gafira_geom::Size;
use gafira_testkit::fonts;
use gafira_text::{ListEntry, ListItem, ListKind, TextError, TextStyle, layout_list};

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

fn lay(items: &[ListItem<'_>]) -> Result<Vec<ListEntry>, TextError> {
    layout_list(items, &[INTER], &style(), 96.0, Size::new(2000.0, 1000.0))
}

fn markers(entries: &[ListEntry]) -> Vec<&str> {
    entries.iter().map(|e| e.marker.as_str()).collect()
}

use ListKind::{Bullet, Numbered};

#[test]
fn fd_41_j_a_list_of_three_levels_indents_1_5_em_per_level() {
    let entries = lay(&[
        item("a", 1, Bullet),
        item("b", 2, Bullet),
        item("c", 3, Bullet),
    ])
    .expect("valid list");
    assert_eq!(entries.len(), 3);
    let indents: Vec<f64> = entries.iter().map(|e| f64::from(e.indent)).collect();
    for (level, indent) in indents.iter().enumerate() {
        let expected = level as f64 * 1.5 * SIZE_PX;
        assert!(
            (indent - expected).abs() <= 0.01,
            "level {}: {indent}",
            level + 1
        );
    }
    // The text starts at the indent.
    for e in &entries {
        let x = f64::from(e.layout.lines[0].clusters[0].x);
        assert!(
            (x - f64::from(e.indent)).abs() <= 0.01,
            "{x} vs {}",
            e.indent
        );
    }
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
fn fd_41_j_a_bullet_starts_a_new_count_at_its_level() {
    let entries = lay(&[
        item("a", 1, Numbered),
        item("b", 1, Numbered),
        item("c", 1, Bullet),
        item("d", 1, Numbered),
    ])
    .expect("valid list");
    assert_eq!(markers(&entries), ["1.", "2.", "\u{2022}", "1."]);
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

#[test]
fn fd_41_j_each_item_is_its_marker_a_space_and_its_text_stacked_from_the_top() {
    let entries = lay(&[item("one", 1, Numbered), item("two", 1, Numbered)]).expect("list");
    assert_eq!(entries[0].layout.text, "1. one");
    assert_eq!(entries[1].layout.text, "2. two");
    assert_eq!(entries[0].top, 0.0);
    assert!(entries[0].layout.height > 0.0);
    assert_eq!(entries[1].top, entries[0].layout.height);
}

#[test]
fn fd_41_j_an_item_wraps_in_the_box_minus_its_indent() {
    let text = "alpha beta gamma delta epsilon zeta eta theta";
    let items = [item(text, 3, Bullet)];
    let narrow =
        layout_list(&items, &[INTER], &style(), 96.0, Size::new(300.0, 1000.0)).expect("list");
    let e = &narrow[0];
    assert!(e.layout.lines.len() > 1);
    for line in &e.layout.lines {
        // The white space at the end of a line hangs past the box.
        let right = line
            .clusters
            .iter()
            .filter(|c| !e.layout.text[c.range.clone()].trim().is_empty())
            .map(|c| c.x + c.advance)
            .fold(0.0, f32::max);
        assert!(right <= 300.0 + 0.01, "{right}");
    }
    // The indent alone is wider than the box: no error.
    let tight = layout_list(&items, &[INTER], &style(), 96.0, Size::new(50.0, 1000.0));
    assert!(tight.is_ok());
}

#[test]
fn arq_110_a_list_checks_its_numbers_and_fonts_before_its_levels() {
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
    assert_eq!(
        layout_list(&[item("a", 1, Bullet)], &[], &style(), 96.0, size).err(),
        Some(TextError::NoFont)
    );
    assert_eq!(
        layout_list(&[], &[INTER], &style(), 96.0, size).expect("empty"),
        vec![]
    );
}
