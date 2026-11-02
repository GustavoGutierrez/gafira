//! FD-41 (i): a justified line ends at the edge of the box, and the last line of a
//! paragraph, a line with a forced break and a line of one word are not stretched.
//! The other alignments of FD-41 are checked on one line.

use gafira_geom::Size;
use gafira_testkit::fonts;
use gafira_text::{Line, TextAlign, TextLayout, TextStyle, layout_box};

const INTER: &[u8] = fonts::INTER.bytes;
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
