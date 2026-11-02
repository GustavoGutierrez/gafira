//! FD-41 (h): full Unicode upper case with no language (ARQ-107).

use gafira_geom::Size;
use gafira_testkit::fonts;
use gafira_text::{TextLayout, TextStyle, layout_box};

const INTER: &[u8] = fonts::INTER.bytes;

fn lay(text: &str, uppercase: bool) -> TextLayout {
    let s = TextStyle {
        size_pt: 24.0,
        uppercase,
        ..TextStyle::default()
    };
    layout_box(text, &[INTER], &s, 96.0, Size::new(10_000.0, 1000.0)).expect("valid input")
}

/// The text of the clusters in logical order, cut from the laid out text.
fn clusters_text(layout: &TextLayout) -> String {
    let mut ranges: Vec<_> = layout
        .lines
        .iter()
        .flat_map(|l| l.clusters.iter().map(|c| c.range.clone()))
        .collect();
    ranges.sort_by_key(|r| r.start);
    ranges.into_iter().map(|r| &layout.text[r]).collect()
}

#[test]
fn fd_41_h_sharp_s_gives_ss_and_the_clusters_follow_the_converted_text() {
    let layout = lay("stra\u{df}e", true);
    assert_eq!(layout.text, "STRASSE");
    assert_eq!(clusters_text(&layout), "STRASSE");
    assert_eq!(layout.lines[0].clusters.len(), 7);
}

#[test]
fn fd_41_h_the_conversion_is_full_unicode_and_has_no_language() {
    let table = [
        ("stra\u{df}e", "STRASSE"),
        ("i", "I"),         // not a dotted capital I, as a Turkish rule would give
        ("\u{131}", "I"),   // dotless i
        ("\u{fb01}", "FI"), // fi ligature
        ("\u{149}", "\u{2bc}N"),
        ("\u{3bf}\u{3c2}", "\u{39f}\u{3a3}"), // final sigma
        ("caf\u{e9}", "CAF\u{c9}"),
        ("", ""),
    ];
    for (input, expected) in table {
        let layout = lay(input, true);
        assert_eq!(layout.text, expected, "{input:?}");
        assert_eq!(clusters_text(&layout), expected, "{input:?}");
    }
}

#[test]
fn fd_41_h_without_the_option_the_text_is_untouched() {
    let layout = lay("stra\u{df}e", false);
    assert_eq!(layout.text, "stra\u{df}e");
}

#[test]
fn fd_41_h_converted_text_is_shaped_as_the_upper_case_text() {
    let a = lay("stra\u{df}e", true);
    let b = lay("STRASSE", false);
    assert_eq!(a, b);
}
