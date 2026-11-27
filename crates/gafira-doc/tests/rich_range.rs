//! Spec 106: styles on part of a text — a range of characters restyled, and
//! the words edited as plain text keeping the styles of what stayed.

use gafira_color::{Color, Rgba};
use gafira_doc::{RichText, SpanStyle};

fn style(bold: bool, red: bool) -> SpanStyle {
    let ink = if red {
        Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0))
    } else {
        Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0))
    };
    SpanStyle {
        font: "Inter".to_owned(),
        size: 20.0,
        color: ink,
        bold,
        italic: false,
    }
}

fn runs(rich: &RichText) -> Vec<Vec<(String, bool, bool)>> {
    let red = Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0));
    rich.paragraphs
        .iter()
        .map(|p| {
            p.spans
                .iter()
                .map(|s| (s.text.clone(), s.style.bold, s.style.color == red))
                .collect()
        })
        .collect()
}

// S106-01: a range of a plain text gets bold and red; the rest stays, and
// the spans are cut only where the style changes (the owner's example).
#[test]
fn s106_01_restyle_a_word() {
    let mut text = RichText::plain("Agregar algo de texto", &style(false, false));
    text.restyle(8..12, &|s: &mut SpanStyle| {
        s.bold = true;
        s.color = Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0));
    });
    assert_eq!(
        runs(&text),
        vec![vec![
            ("Agregar ".to_owned(), false, false),
            ("algo".to_owned(), true, true),
            (" de texto".to_owned(), false, false),
        ]]
    );
    assert_eq!(text.plain_text(), "Agregar algo de texto");
}

// S106-02: a range across paragraphs; restyling back merges the spans.
#[test]
fn s106_02_across_lines_and_merge() {
    let mut text = RichText::plain("uno dos\ntres", &style(false, false));
    // «dos\ntr»: the line break counts as one character.
    text.restyle(4..10, &|s: &mut SpanStyle| s.bold = true);
    assert_eq!(
        runs(&text),
        vec![
            vec![
                ("uno ".to_owned(), false, false),
                ("dos".to_owned(), true, false)
            ],
            vec![
                ("tr".to_owned(), true, false),
                ("es".to_owned(), false, false)
            ],
        ]
    );
    text.restyle(0..12, &|s: &mut SpanStyle| s.bold = false);
    assert_eq!(
        runs(&text),
        vec![
            vec![("uno dos".to_owned(), false, false)],
            vec![("tres".to_owned(), false, false)],
        ]
    );
}

// S106-03: editing the words keeps the styles of what was not touched; new
// letters take the style of the letter before them.
#[test]
fn s106_03_edit_keeps_styles() {
    let mut text = RichText::plain("Agregar algo de texto", &style(false, false));
    text.restyle(8..12, &|s: &mut SpanStyle| s.bold = true);
    // Typed inside the bold word, then a word after it: each edit is one
    // change in one place, as when writing.
    let edited = text
        .edited("Agregar algoooo de texto")
        .edited("Agregar algoooo de mucho texto");
    assert_eq!(
        runs(&edited),
        vec![vec![
            ("Agregar ".to_owned(), false, false),
            ("algoooo".to_owned(), true, false),
            (" de mucho texto".to_owned(), false, false),
        ]]
    );
    // A line break typed splits the paragraph, styles kept.
    let broken = text.edited("Agregar al\ngo de texto");
    assert_eq!(
        runs(&broken),
        vec![
            vec![
                ("Agregar ".to_owned(), false, false),
                ("al".to_owned(), true, false)
            ],
            vec![
                ("go".to_owned(), true, false),
                (" de texto".to_owned(), false, false)
            ],
        ]
    );
    // Everything deleted keeps the first style for what is typed next.
    let empty = text.edited("");
    assert_eq!(empty.plain_text(), "");
    assert_eq!(empty.first_style(), Some(&style(false, false)));
}

// S106-04: the style at a character, for the bar while a range is chosen.
#[test]
fn s106_04_style_of_a_range() {
    let mut text = RichText::plain("ab cd", &style(false, false));
    text.restyle(3..5, &|s: &mut SpanStyle| s.bold = true);
    assert_eq!(text.style_at(0).map(|s| s.bold), Some(false));
    assert_eq!(text.style_at(3).map(|s| s.bold), Some(true));
    assert_eq!(text.char_count(), 5);
    // An empty or reversed range changes nothing.
    let before = text.clone();
    text.restyle(2..2, &|s: &mut SpanStyle| s.italic = true);
    assert_eq!(text, before);
}
