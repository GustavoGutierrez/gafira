//! FD-41 (i): a justified line ends at the edge of the box, and the last line of a
//! paragraph, a line with a forced break and a line of one word are not stretched.
//! The other alignments of FD-41 are checked on one line.

use gafira_geom::Size;
use gafira_testkit::fonts;
use gafira_text::{Line, TextAlign, TextLayout, TextStyle, layout_box};

const INTER: &[u8] = fonts::INTER.bytes;
const ARABIC: &[u8] = fonts::NOTO_SANS_ARABIC.bytes;
const WIDTH: f64 = 200.0;

fn lay(text: &str, align: TextAlign, width: f64) -> TextLayout {
    lay_t(text, align, width, 0.0)
}

fn lay_t(text: &str, align: TextAlign, width: f64, letter_spacing: f64) -> TextLayout {
    let s = TextStyle {
        size_pt: 12.0,
        align,
        letter_spacing,
        ..TextStyle::default()
    };
    layout_box(text, &[INTER], &s, 96.0, Size::new(width, 1000.0)).expect("valid input")
}

/// `text` laid out with a font stack, at 96 dpi.
fn lay_with(
    text: &str,
    stack: &[&[u8]],
    size_pt: f64,
    align: TextAlign,
    width: f64,
    letter_spacing: f64,
) -> TextLayout {
    let s = TextStyle {
        size_pt,
        align,
        letter_spacing,
        ..TextStyle::default()
    };
    layout_box(text, stack, &s, 96.0, Size::new(width, 1000.0)).expect("valid input")
}

/// Left and right edge of the content of a line: the clusters that are not white space.
fn edges(text: &str, line: &Line) -> (f64, f64) {
    let mut ink = line
        .clusters
        .iter()
        .filter(|c| !text[c.range.clone()].trim().is_empty());
    let first = ink.next().expect("a line with text");
    ink.fold(
        (f64::from(first.x), f64::from(first.x + first.advance)),
        |(l, r), c| (l.min(f64::from(c.x)), r.max(f64::from(c.x + c.advance))),
    )
}

const PARAGRAPH: &str = "aa bb cc dd ee ff gg hh ii jj kk ll mm nn oo pp qq rr ss tt uu";

#[test]
fn fd_41_i_every_line_but_the_last_ends_at_the_edge_of_the_box() {
    let layout = lay(PARAGRAPH, TextAlign::Justify, WIDTH);
    assert!(layout.lines.len() >= 3);
    let (last, rest) = layout.lines.split_last().expect("lines");
    for line in rest {
        let (left, right) = edges(PARAGRAPH, line);
        assert!(left.abs() <= 0.01, "starts at the left edge: {left}");
        assert!((right - WIDTH).abs() <= 0.01, "ends at {right}");
    }
    // The last line is as in a left aligned text.
    let plain = lay(PARAGRAPH, TextAlign::Left, WIDTH);
    assert_eq!(plain.lines.len(), layout.lines.len());
    assert_eq!(last, plain.lines.last().expect("lines"));
    assert!(edges(PARAGRAPH, last).1 < WIDTH - 1.0);
}

#[test]
fn fd_41_i_a_line_with_a_forced_break_is_not_stretched() {
    let text = "aa bb cc\ndd ee ff gg hh ii jj kk ll mm nn oo pp qq rr ss tt";
    let justified = lay(text, TextAlign::Justify, WIDTH);
    let plain = lay(text, TextAlign::Left, WIDTH);
    assert_eq!(justified.lines[0], plain.lines[0]);
    // The wrapped line after it is stretched.
    assert!((edges(text, &justified.lines[1]).1 - WIDTH).abs() <= 0.01);
}

#[test]
fn fd_41_i_a_line_of_one_word_is_not_stretched() {
    // The first line holds only the long word, then the wrapped second line follows.
    let text = "abcdefghij klmnopqrstuvwxyz abcdefghij klm";
    let justified = lay(text, TextAlign::Justify, 150.0);
    let plain = lay(text, TextAlign::Left, 150.0);
    assert!(plain.lines.len() >= 3);
    let one_word = plain
        .lines
        .iter()
        .position(|l| text[l.clusters[0].range.start..].starts_with("klmnopqrstuvwxyz"))
        .expect("the long word is alone on its line");
    assert_eq!(justified.lines[one_word], plain.lines[one_word]);
}

#[test]
fn fd_41_i_justify_does_not_make_a_line_overflow() {
    for width in [100.0, 150.0, 200.0, 333.3] {
        assert!(
            !lay(PARAGRAPH, TextAlign::Justify, width).overflow,
            "{width}"
        );
    }
}

#[test]
fn fd_41_i_justify_keeps_all_clusters_in_order() {
    let layout = lay(PARAGRAPH, TextAlign::Justify, WIDTH);
    for line in &layout.lines {
        for pair in line.clusters.windows(2) {
            assert!(pair[1].x >= pair[0].x + pair[0].advance - 0.01);
        }
    }
    let total: usize = layout.lines.iter().map(|l| l.clusters.len()).sum();
    assert_eq!(total, PARAGRAPH.chars().count());
}

#[test]
fn fd_41_left_center_and_right_place_one_line_in_the_box() {
    let text = "ab cd";
    let content = {
        let (l, r) = edges(text, &lay(text, TextAlign::Left, WIDTH).lines[0]);
        r - l
    };
    let (l, _) = edges(text, &lay(text, TextAlign::Left, WIDTH).lines[0]);
    assert!(l.abs() <= 0.01);
    let (cl, cr) = edges(text, &lay(text, TextAlign::Center, WIDTH).lines[0]);
    assert!((cl - (WIDTH - content) / 2.0).abs() <= 0.01, "{cl}");
    assert!((cr - (WIDTH + content) / 2.0).abs() <= 0.01, "{cr}");
    let (_, rr) = edges(text, &lay(text, TextAlign::Right, WIDTH).lines[0]);
    assert!((rr - WIDTH).abs() <= 0.01, "{rr}");
}

const SPACINGS: [f64; 5] = [-100.0, -20.0, 100.0, 250.0, 500.0];

#[test]
fn fd_41_i_a_justified_line_ends_at_the_edge_with_any_spacing() {
    for t in SPACINGS {
        for step in 40..=200 {
            let width = f64::from(step);
            let layout = lay_t(PARAGRAPH, TextAlign::Justify, width, t);
            let (last, rest) = layout.lines.split_last().expect("lines");
            for line in rest {
                let words = line
                    .clusters
                    .iter()
                    .filter(|c| PARAGRAPH[c.range.clone()] == *" ")
                    .count();
                if words < 2 {
                    continue; // a line of one word is not stretched, its space hangs
                }
                let (left, right) = edges(PARAGRAPH, line);
                assert!(left.abs() <= 0.01, "t={t} w={width}: starts at {left}");
                assert!(
                    (right - width).abs() <= 0.01,
                    "t={t} w={width}: ends at {right}"
                );
            }
            assert!(edges(PARAGRAPH, last).1 <= width + 0.01);
            assert!(!layout.overflow, "t={t} w={width}");
        }
    }
}

#[test]
fn fd_41_right_and_centre_use_the_spacing_the_output_keeps() {
    for t in SPACINGS {
        let (l, r) = edges("Hello", &lay_t("Hello", TextAlign::Left, WIDTH, t).lines[0]);
        let content = r - l;
        let (_, right) = edges(
            "Hello",
            &lay_t("Hello", TextAlign::Right, WIDTH, t).lines[0],
        );
        assert!(
            (right - WIDTH).abs() <= 0.01,
            "t={t}: right ends at {right}"
        );
        let (cl, cr) = edges(
            "Hello",
            &lay_t("Hello", TextAlign::Center, WIDTH, t).lines[0],
        );
        assert!((cl - (WIDTH - content) / 2.0).abs() <= 0.01, "t={t}: {cl}");
        assert!((cr - (WIDTH + content) / 2.0).abs() <= 0.01, "t={t}: {cr}");
    }
}

#[test]
fn fd_41_k_trailing_spaces_hang_to_the_right_and_centre_ignores_them() {
    let width = 100.0;
    for t in [0.0, 500.0] {
        let right = lay_t("ab  ", TextAlign::Right, width, t);
        let ink = edges("ab  ", &right.lines[0]).1;
        assert!((ink - width).abs() <= 0.01, "t={t}: b ends at {ink}");
        let plain = lay_t("ab", TextAlign::Center, width, t);
        let spaced = lay_t("ab  ", TextAlign::Center, width, t);
        for (a, b) in plain.lines[0]
            .clusters
            .iter()
            .zip(&spaced.lines[0].clusters)
        {
            assert!((a.x - b.x).abs() <= 0.01, "t={t}: {} vs {}", a.x, b.x);
        }
    }
}

#[test]
fn fd_41_k_a_space_before_a_forced_break_and_on_the_last_line_hangs_too() {
    let width = 100.0;
    let layout = lay_t("ab  \ncd  ", TextAlign::Right, width, 0.0);
    for line in &layout.lines {
        let ink = edges("ab  \ncd  ", line).1;
        assert!((ink - width).abs() <= 0.01, "{ink}");
    }
}

#[test]
fn fd_41_a_line_wider_than_the_box_stays_on_the_left_in_any_alignment() {
    // One cluster wider than the box: it is not centred or pushed to the right.
    for align in [TextAlign::Left, TextAlign::Center, TextAlign::Right] {
        let layout = lay("W", align, 3.0);
        assert_eq!(layout.lines[0].clusters[0].x, 0.0, "{align:?}");
    }
}

// ---- right-to-left paragraphs: the white space at the end hangs on the left ------------------

/// Arabic words, so the paragraph is right-to-left.
const ARABIC_PARAGRAPH: &str = "\u{627}\u{628}\u{62c} \u{62f}\u{647}\u{648} \u{648}\u{647}\u{62f} \u{627}\u{628}\u{62c} \u{62f}\u{647}\u{648} \u{648}\u{647}\u{62f} \u{627}\u{628}\u{62c} \u{62f}\u{647}\u{648} \u{648}\u{647}\u{62f}";

#[test]
fn fd_41_i_a_right_to_left_justified_line_ends_at_the_edge_with_any_spacing() {
    let stack = [ARABIC, INTER];
    for t in [0.0, -100.0, 100.0, 250.0, 500.0] {
        for step in 40..=200 {
            let width = f64::from(step);
            let layout = lay_with(ARABIC_PARAGRAPH, &stack, 12.0, TextAlign::Justify, width, t);
            let (_, rest) = layout.lines.split_last().expect("lines");
            for line in rest {
                let spaces = line
                    .clusters
                    .iter()
                    .filter(|c| ARABIC_PARAGRAPH[c.range.clone()] == *" ")
                    .count();
                if spaces < 2 {
                    continue; // one word and its hanging space
                }
                let (left, right) = edges(ARABIC_PARAGRAPH, line);
                assert!(left.abs() <= 0.01, "t={t} w={width}: starts at {left}");
                assert!(
                    (right - width).abs() <= 0.01,
                    "t={t} w={width}: ends at {right}"
                );
            }
        }
    }
}

#[test]
fn fd_41_hanging_space_of_a_right_to_left_paragraph_is_outside_the_ink_with_left() {
    let stack = [ARABIC, INTER];
    let word = "\u{627}\u{628}\u{62c} \u{62f}\u{647}\u{648}";
    for t in [0.0, 100.0, 500.0] {
        let plain = lay_with(word, &stack, 12.0, TextAlign::Left, 200.0, t);
        let (plain_left, plain_right) = edges(word, &plain.lines[0]);
        for tail in ["  ", " \n", "  \nabc"] {
            let text = format!("{word}{tail}");
            let layout = lay_with(&text, &stack, 12.0, TextAlign::Left, 200.0, t);
            let (left, right) = edges(&text, &layout.lines[0]);
            assert!(left.abs() <= 0.01, "t={t} {tail:?}: ink starts at {left}");
            assert!(
                (right - left - (plain_right - plain_left)).abs() <= 0.01,
                "t={t} {tail:?}: ink {left}..{right}"
            );
            assert!(plain_left.abs() <= 0.01);
        }
    }
}

#[test]
fn fd_41_a_wrapped_right_to_left_paragraph_with_left_keeps_its_ink_in_the_box() {
    let stack = [ARABIC, INTER];
    for t in [0.0, 100.0] {
        for step in 30..=120 {
            let width = f64::from(step);
            let layout = lay_with(ARABIC_PARAGRAPH, &stack, 12.0, TextAlign::Left, width, t);
            assert!(!layout.overflow, "t={t} w={width}");
            for line in &layout.lines {
                let (left, right) = edges(ARABIC_PARAGRAPH, line);
                assert!(left.abs() <= 0.01, "t={t} w={width}: starts at {left}");
                assert!(right <= width + 0.01, "t={t} w={width}: ends at {right}");
            }
        }
    }
}

// ---- the accepted limit: a trailing space of another bidi level (FD-41 m) --------------------

/// The texts of (m) alone, and behind a right-to-left paragraph, whose line comes first.
const M_PREFIXES: [(&str, usize); 2] = [("", 0), ("\u{627}\u{628}\u{62c}\n", 1)];

#[test]
fn fd_41_m_a_trailing_space_of_the_other_level_starts_the_line_at_the_edge() {
    for (prefix, before) in M_PREFIXES {
        let text = format!("{prefix}abc \u{627}\u{628}\u{62c} \u{62f}\u{647}\u{648} def");
        let layout = lay_with(&text, &[INTER, ARABIC], 12.0, TextAlign::Left, 40.0, 0.0);
        assert!(!layout.overflow);
        let second = &layout.lines[before + 1];
        let first = &second.clusters[0];
        assert_eq!(&text[first.range.clone()], " ");
        assert!(first.x.abs() <= 0.01, "the space is at {}", first.x);
        let (left, _) = edges(&text, second);
        assert!((left - 4.50).abs() <= 0.01, "the content starts at {left}");
    }
}

#[test]
fn fd_41_m_content_of_the_other_level_can_end_past_the_box_without_overflow() {
    for (prefix, before) in M_PREFIXES {
        let text = format!("{prefix}abc \u{627}\u{628}\u{62c} \u{62f}\u{647}\u{648} def");
        let layout = lay_with(&text, &[INTER, ARABIC], 9.0, TextAlign::Left, 39.3, 0.0);
        assert!(!layout.overflow);
        let (_, right) = edges(&text, &layout.lines[before]);
        assert!((right - 41.56).abs() <= 0.01, "the content ends at {right}");
        assert!((right - 39.3 - 2.26).abs() <= 0.01);
    }
}

#[test]
fn fd_41_m_a_trailing_space_of_the_paragraph_level_stays_behind_the_content() {
    for (prefix, before) in M_PREFIXES {
        let text = format!("{prefix}abc \u{627}\u{628}\u{62c} def");
        let layout = lay_with(&text, &[INTER, ARABIC], 9.0, TextAlign::Left, 39.3, 0.0);
        assert!(!layout.overflow);
        let line = &layout.lines[before];
        let (_, right) = edges(&text, line);
        assert!((right - 38.19).abs() <= 0.01, "the content ends at {right}");
        let space = line.clusters.last().expect("clusters");
        assert_eq!(&text[space.range.clone()], " ");
        assert!((f64::from(space.x) - right).abs() <= 0.01);
    }
}

#[test]
fn fd_41_i_a_one_word_line_behind_an_opposite_level_space_is_not_stretched() {
    // The second line is the Arabic word and its trailing space, which is not at the
    // paragraph level: the line has one word, so Justify leaves it as Left does.
    let text = "abc \u{627}\u{628}\u{62c} \u{62f}\u{647}\u{648} def";
    let stack = [INTER, ARABIC];
    let justified = lay_with(text, &stack, 12.0, TextAlign::Justify, 40.0, 0.0);
    let plain = lay_with(text, &stack, 12.0, TextAlign::Left, 40.0, 0.0);
    assert_eq!(justified.lines[1], plain.lines[1]);
    let (left, right) = edges(text, &justified.lines[1]);
    assert!((left - 4.50).abs() <= 0.01 && (right - 23.00).abs() <= 0.01);
}

#[test]
fn fd_41_a_space_of_the_other_level_in_a_right_to_left_paragraph_does_not_hang_with_left() {
    // Only white space with the level of the paragraph hangs. A Latin word and its trailing
    // space at the end of an Arabic paragraph stay inside the line at the left edge.
    let text = "\u{627}\u{628}\u{62c} abc def ghi \u{62f}\u{647}\u{648}";
    let stack = [ARABIC, INTER];
    let mut other_level_spaces = 0;
    for step in 20..=140 {
        let layout = lay_with(text, &stack, 12.0, TextAlign::Left, f64::from(step), 0.0);
        for line in &layout.lines {
            let ends_in_latin_space = line
                .clusters
                .iter()
                .any(|c| !c.rtl && text[c.range.clone()] == *" " && c.range.end == line_end(line));
            other_level_spaces += usize::from(ends_in_latin_space);
            for c in &line.clusters {
                assert!(
                    c.x >= -0.01 || (c.rtl && text[c.range.clone()].trim().is_empty()),
                    "w={step}: {:?} at {}",
                    &text[c.range.clone()],
                    c.x
                );
            }
        }
    }
    assert!(other_level_spaces > 0, "the sweep never ends a line there");
}

/// Logical end of a line: the largest end of its clusters.
fn line_end(line: &Line) -> usize {
    line.clusters.iter().map(|c| c.range.end).max().unwrap_or(0)
}

// S13-10, FD-41: in a wrapped right-to-left paragraph the space that hangs sits at the left of
// each line but the last, so the content does not start at x = 0; `Center` and `Right` still
// place the content (without that space) in the middle and at the right edge of the box. Lines
// of one letter are the case where the content is narrowest next to the space.
#[test]
fn fd_41_a_wrapped_right_to_left_paragraph_centres_and_right_aligns_its_content() {
    let stack = [ARABIC, INTER];
    let letters = "\u{628} \u{628} \u{628}";
    let mut hung = 0;
    for (text, widths) in [
        (ARABIC_PARAGRAPH, [60, 80, 100, 120]),
        (letters, [16, 18, 20, 22]),
    ] {
        for step in widths {
            let width = f64::from(step);
            let left = lay_with(text, &stack, 12.0, TextAlign::Left, width, 0.0);
            let center = lay_with(text, &stack, 12.0, TextAlign::Center, width, 0.0);
            let right = lay_with(text, &stack, 12.0, TextAlign::Right, width, 0.0);
            assert!(left.lines.len() >= 2, "w={width}");
            for ((l, c), r) in left.lines.iter().zip(&center.lines).zip(&right.lines) {
                hung += usize::from(l.clusters.iter().any(|k| k.trailing));
                let (ll, lr) = edges(text, l);
                let content = lr - ll;
                let (cl, cr) = edges(text, c);
                assert!(
                    (cl - (width - content) / 2.0).abs() <= 0.01,
                    "w={width}: {cl}"
                );
                assert!(
                    (cr - (width + content) / 2.0).abs() <= 0.01,
                    "w={width}: {cr}"
                );
                let (rl, rr) = edges(text, r);
                assert!((rr - width).abs() <= 0.01, "w={width}: {rr}");
                assert!((rl - (width - content)).abs() <= 0.01, "w={width}: {rl}");
            }
        }
    }
    assert!(hung > 0, "no line has a space that hangs");
}

// S13-11, FD-41 (i): a stretched line stays contiguous: each cluster ends where the next one
// starts, the stretched spaces included, and the advances add up to the advance of the line.
#[test]
fn fd_41_i_a_justified_line_keeps_its_clusters_contiguous() {
    let layout = lay(PARAGRAPH, TextAlign::Justify, WIDTH);
    let (_, rest) = layout.lines.split_last().expect("lines");
    for line in rest {
        for pair in line.clusters.windows(2) {
            let end = f64::from(pair[0].x) + f64::from(pair[0].advance);
            assert!(
                (end - f64::from(pair[1].x)).abs() <= 0.01,
                "{end} {}",
                pair[1].x
            );
        }
        let sum: f64 = line.clusters.iter().map(|c| f64::from(c.advance)).sum();
        assert!((sum - f64::from(line.advance)).abs() <= 0.01, "{sum}");
    }
}
