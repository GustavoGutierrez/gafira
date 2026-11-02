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

/// How the clusters look: advance bits, position bits and glyphs, with no ranges.
fn shape(layout: &TextLayout) -> Vec<(u32, u32, Vec<u32>)> {
    layout
        .lines
        .iter()
        .flat_map(|l| {
            l.clusters
                .iter()
                .map(|c| (c.x.to_bits(), c.advance.to_bits(), c.glyphs.clone()))
        })
        .collect()
}

#[test]
fn fd_41_h_sharp_s_gives_ss_and_is_shaped_as_the_converted_text() {
    let layout = lay("stra\u{df}e", true);
    assert_eq!(layout.lines[0].clusters.len(), 7);
    assert_eq!(shape(&layout), shape(&lay("STRASSE", false)));
    assert_ne!(shape(&layout), shape(&lay("strasse", false)));
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
        assert_eq!(
            shape(&lay(input, true)),
            shape(&lay(expected, false)),
            "{input:?}"
        );
    }
}

#[test]
fn fd_41_h_without_the_option_the_text_is_shaped_as_it_is() {
    assert_ne!(
        shape(&lay("stra\u{df}e", false)),
        shape(&lay("STRASSE", false))
    );
}

/// The ranges of the clusters in visual order.
fn ranges(layout: &TextLayout) -> Vec<std::ops::Range<usize>> {
    layout
        .lines
        .iter()
        .flat_map(|l| l.clusters.iter().map(|c| c.range.clone()))
        .collect()
}

#[test]
fn arq_116_ranges_after_upper_case_index_the_input_and_clusters_of_one_character_share_it() {
    // "straße": the two S of the sharp s share 4..6, and E is 6..7.
    let layout = lay("stra\u{df}e", true);
    assert_eq!(layout.lines[0].clusters.len(), 7);
    assert_eq!(ranges(&layout), [0..1, 1..2, 2..3, 3..4, 4..6, 4..6, 6..7]);
}

#[test]
fn arq_116_ranges_after_upper_case_follow_other_expansions_and_marks() {
    let table: [(&str, Vec<std::ops::Range<usize>>); 4] = [
        ("\u{fb01}x", vec![0..3, 0..3, 3..4]), // fi ligature: F, I
        ("\u{149}", vec![0..2, 0..2]),         // n with a leading apostrophe
        ("e\u{301}a", vec![0..3, 3..4]),       // a mark stays in its cluster
        ("a\u{df}\u{df}", vec![0..1, 1..3, 1..3, 3..5, 3..5]),
    ];
    for (input, expected) in table {
        assert_eq!(ranges(&lay(input, true)), expected, "{input:?}");
    }
}

#[test]
fn arq_116_ranges_without_upper_case_are_the_input_ranges() {
    assert_eq!(
        ranges(&lay("stra\u{df}e", false)),
        [0..1, 1..2, 2..3, 3..4, 4..6, 6..7]
    );
}
