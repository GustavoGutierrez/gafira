//! FD-41, Párrafos bidi: every forced break closes one bidi paragraph and opens another, each
//! one with its own direction, and the lines keep their ranges against the whole text.

use gafira_geom::Size;
use gafira_testkit::fonts;
use gafira_text::{
    Cluster, Line, ListItem, ListKind, TextAlign, TextLayout, TextStyle, layout_box, layout_list,
};

const INTER: &[u8] = fonts::INTER.bytes;
const ARABIC: &[u8] = fonts::NOTO_SANS_ARABIC.bytes;
const STACK: [&[u8]; 2] = [INTER, ARABIC];
const ABJ: &str = "\u{627}\u{628}\u{62c}";
const MIXED: &str = "abc \u{627}\u{628}\u{62c} def";
const BREAKS: [&str; 5] = ["\n", "\r", "\r\n", "\u{2028}", "\u{2029}"];
const NOT_BREAKS: [&str; 6] = ["\u{b}", "\u{c}", "\u{85}", "\u{1c}", "\u{1d}", "\u{1e}"];

fn lay_at(text: &str, align: TextAlign, width: f64) -> TextLayout {
    let style = TextStyle {
        size_pt: 12.0,
        align,
        ..TextStyle::default()
    };
    layout_box(text, &STACK, &style, 96.0, Size::new(width, 1000.0)).expect("valid input")
}

fn lay(text: &str, align: TextAlign) -> TextLayout {
    lay_at(text, align, 100.0)
}

fn shown<'a>(text: &'a str, line: &Line) -> Vec<&'a str> {
    line.clusters
        .iter()
        .map(|c| &text[c.range.clone()])
        .filter(|s| !is_break(s))
        .collect()
}

fn is_break(s: &str) -> bool {
    BREAKS.contains(&s)
}

fn ink(text: &str, line: &Line) -> (f64, f64) {
    let edges = line
        .clusters
        .iter()
        .filter(|c| !text[c.range.clone()].trim().is_empty())
        .map(|c| (f64::from(c.x), f64::from(c.x) + f64::from(c.advance)));
    edges
        .reduce(|(l, r), (a, b)| (l.min(a), r.max(b)))
        .expect("a line with ink")
}

fn cluster<'a>(text: &str, line: &'a Line, s: &str) -> &'a Cluster {
    line.clusters
        .iter()
        .find(|c| &text[c.range.clone()] == s)
        .expect("the cluster")
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 0.01
}

/// Where the first character of the text sits.
fn first_letter_x(line: &Line) -> f32 {
    line.clusters
        .iter()
        .find(|c| c.range.start == 0)
        .expect("the first cluster")
        .x
}

/// In a right-to-left paragraph the forced break sits at the left of its line.
fn starts_with_the_break(text: &str, line: &Line) -> bool {
    is_break(&text[line.clusters[0].range.clone()])
}

#[test]
fn fd_41_n_each_forced_break_opens_a_paragraph_with_its_own_direction() {
    let alone = lay(MIXED, TextAlign::Left);
    assert_eq!(alone.lines.len(), 1, "100 px holds the line");
    for brk in BREAKS {
        let text = format!("{ABJ}{brk}{MIXED}");
        let out = lay(&text, TextAlign::Left);
        assert_eq!(out.lines.len(), 2, "{brk:?}");
        assert!(starts_with_the_break(&text, &out.lines[0]), "{brk:?}");
        assert_eq!(
            shown(&text, &out.lines[0]),
            ["\u{62c}", "\u{628}", "\u{627}"],
            "{brk:?}"
        );
        assert_eq!(
            shown(&text, &out.lines[1]),
            [
                "a", "b", "c", " ", "\u{62c}", "\u{628}", "\u{627}", " ", "d", "e", "f"
            ],
            "{brk:?}"
        );
        for (c, solo) in out.lines[1].clusters.iter().zip(&alone.lines[0].clusters) {
            assert!(
                near(f64::from(c.x), f64::from(solo.x)),
                "{brk:?}: {} and {}",
                c.x,
                solo.x
            );
        }
    }
}

#[test]
fn fd_41_n_u_0085_keeps_one_right_to_left_paragraph() {
    let text = format!("{ABJ}\u{85}{MIXED}");
    let out = lay_at(&text, TextAlign::Left, 1000.0);
    assert_eq!(out.lines.len(), 1);
    let first = first_letter_x(&out.lines[0]);
    let a = cluster(&text, &out.lines[0], "a").x;
    assert!(a < first, "a at {a}, the first letter at {first}");
    // In the box of (n) it wraps, and its last line still starts at the right edge.
    let boxed = lay(&text, TextAlign::Justify);
    let last = boxed.lines.last().expect("lines");
    assert!(near(ink(&text, last).1, 100.0), "{}", ink(&text, last).1);
}

#[test]
fn fd_41_controls_that_are_not_forced_breaks_do_not_open_a_paragraph() {
    for ctrl in NOT_BREAKS {
        let text = format!("{ABJ}{ctrl}{MIXED}");
        let out = lay_at(&text, TextAlign::Left, 1000.0);
        assert_eq!(out.lines.len(), 1, "{ctrl:?}");
        let first = first_letter_x(&out.lines[0]);
        let a = cluster(&text, &out.lines[0], "a").x;
        assert!(a < first, "{ctrl:?}: a at {a}, the first letter at {first}");
    }
}

#[test]
fn fd_41_a_crlf_is_one_break_and_lone_cr_and_lf_are_one_each() {
    let table = [
        ("a\r\nb", 2),
        ("a\n\rb", 3),
        ("a\r\rb", 3),
        ("a\r\n\r\nb", 3),
        ("a\r\n", 2),
    ];
    for (text, lines) in table {
        assert_eq!(lay(text, TextAlign::Left).lines.len(), lines, "{text:?}");
    }
    // A CRLF between Arabic and Latin must not leave an empty paragraph behind.
    let text = format!("{ABJ}\r\n{MIXED}");
    let out = lay(&text, TextAlign::Left);
    assert_eq!(out.lines.len(), 2);
    assert_eq!(cluster(&text, &out.lines[0], "\r\n").range, 6..8);
}

#[test]
fn fd_41_n_justify_starts_a_line_where_its_paragraph_starts() {
    let text = format!("{ABJ}\nab cd");
    let out = lay(&text, TextAlign::Justify);
    assert!(
        near(ink(&text, &out.lines[0]).1, 100.0),
        "line 1 ends at the edge"
    );
    assert!(near(f64::from(cluster(&text, &out.lines[1], "a").x), 0.0));

    let text = format!("abc\n{ABJ}");
    let out = lay(&text, TextAlign::Justify);
    assert!(near(ink(&text, &out.lines[0]).0, 0.0));
    assert!(
        near(ink(&text, &out.lines[1]).1, 100.0),
        "line 2 ends at the edge"
    );

    let text = format!("{ABJ}\n123");
    let out = lay(&text, TextAlign::Justify);
    assert!(
        near(f64::from(cluster(&text, &out.lines[1], "1").x), 0.0),
        "no strong character: left to right"
    );
}

#[test]
fn fd_41_n_white_space_hangs_where_its_own_paragraph_ends() {
    let text = format!("{ABJ}\nab  ");
    let out = lay(&text, TextAlign::Left);
    let line = &out.lines[1];
    let b = cluster(&text, line, "b");
    let spaces: Vec<&Cluster> = line
        .clusters
        .iter()
        .filter(|c| &text[c.range.clone()] == " ")
        .collect();
    assert_eq!(spaces.len(), 2);
    assert!(near(f64::from(spaces[0].x), f64::from(b.x + b.advance)));
    assert!(spaces[1].x > spaces[0].x);

    let text = format!("abc\n{ABJ}  ");
    let out = lay(&text, TextAlign::Left);
    let line = &out.lines[1];
    let spaces: Vec<f64> = line
        .clusters
        .iter()
        .filter(|c| &text[c.range.clone()] == " ")
        .map(|c| f64::from(c.x))
        .collect();
    assert_eq!(spaces.len(), 2);
    assert!(spaces.iter().all(|x| *x < 0.0), "spaces at {spaces:?}");
    assert!(near(ink(&text, line).0, 0.0));
}

#[test]
fn fd_41_a_paragraph_takes_its_direction_from_its_first_strong_character() {
    // Each row is a second paragraph after an Arabic one, and whether it is right to left.
    let table = [
        ("abc", false),
        (ABJ, true),
        ("\u{5d0}", true),
        ("123 abc", false),
        ("123", false),
        ("... \u{627}", true),
        ("\u{200f}abc", true),
        ("\u{200e}\u{627}", false),
        // Inside an isolate it does not count, closed or left open.
        ("\u{2067}\u{627}\u{2069} abc", false),
        ("\u{2067}\u{627}\u{2069}", false),
        ("\u{2067}\u{627}", false),
        ("\u{2066}abc\u{2069} \u{627}", true),
    ];
    for (paragraph, rtl) in table {
        let text = format!("{ABJ}\n{paragraph}");
        let out = lay(&text, TextAlign::Justify);
        let line = &out.lines[1];
        // A line that is not stretched starts at the edge of its paragraph.
        let right = line
            .clusters
            .iter()
            .map(|c| f64::from(c.x) + f64::from(c.advance))
            .fold(f64::MIN, f64::max);
        let left = line.clusters.iter().map(|c| c.x).fold(f32::MAX, f32::min);
        if rtl {
            assert!(near(right, 100.0), "{paragraph:?}: ends at {right}");
        } else {
            assert!(
                near(f64::from(left), 0.0),
                "{paragraph:?}: starts at {left}"
            );
        }
    }
}

#[test]
fn fd_41_an_open_isolate_ends_with_its_paragraph() {
    let text = format!("\u{2067}{ABJ}\n{ABJ}");
    let out = lay(&text, TextAlign::Justify);
    assert!(near(ink(&text, &out.lines[1]).1, 100.0));
}

#[test]
fn fd_41_an_empty_paragraph_is_left_to_right_and_does_not_inherit() {
    for rest in ["\n", "\n\n", "\r\n"] {
        let text = format!("{ABJ}\n{rest}{ABJ}");
        let out = lay(&text, TextAlign::Justify);
        let empty = &out.lines[1];
        assert_eq!(empty.clusters.len(), 1, "{rest:?}: the break alone");
        assert!(!empty.clusters[0].rtl, "{rest:?}: the empty paragraph");
        assert!(empty.clusters[0].x.abs() <= 0.01, "{rest:?}");
    }
    // Only white space: no strong character either, and the hang goes right.
    let text = format!("{ABJ}\n  \n{ABJ}");
    let out = lay(&text, TextAlign::Left);
    assert!(out.lines[1].clusters.iter().all(|c| c.x >= 0.0 && !c.rtl));
    // A direction does not leak over an empty paragraph.
    let text = format!("abc\n\n{ABJ}\n\nab cd");
    let out = lay(&text, TextAlign::Justify);
    assert!(near(ink(&text, &out.lines[2]).1, 100.0));
    assert!(near(ink(&text, &out.lines[4]).0, 0.0));
}

#[test]
fn fd_41_a_final_break_gives_one_more_empty_line_of_the_same_height() {
    for prefix in ["a", ABJ, ""] {
        let text = format!("{prefix}\n");
        let out = lay(&text, TextAlign::Left);
        assert_eq!(out.lines.len(), 2, "{text:?}");
        assert!(out.lines[1].clusters.is_empty(), "{text:?}");
        assert!(out.lines[1].baseline > out.lines[0].baseline, "{text:?}");
        let empty = lay("", TextAlign::Left);
        let closed = lay(&format!("{prefix}\nb"), TextAlign::Left);
        let bottom = out.height - empty.height;
        assert!(
            (closed.height - lay("b", TextAlign::Left).height - bottom).abs() <= 0.01,
            "{text:?}"
        );
    }
}

#[test]
fn fd_41_line_height_goes_on_across_paragraphs() {
    for line_height in [0.5, 1.0, 2.0] {
        let style = TextStyle {
            size_pt: 12.0,
            line_height,
            ..TextStyle::default()
        };
        let at = |text: &str| {
            layout_box(text, &STACK, &style, 96.0, Size::new(100.0, 1000.0)).expect("valid input")
        };
        let first = at("abc\n");
        let second = at(ABJ);
        let both = at(&format!("abc\n{ABJ}"));
        let top = first.height - at("").height;
        assert!(
            (both.lines[1].baseline - (top + second.lines[0].baseline)).abs() <= 0.01,
            "{line_height}: {} and {}",
            both.lines[1].baseline,
            top + second.lines[0].baseline
        );
        assert!((both.height - (top + second.height)).abs() <= 0.01);
    }
}

#[test]
fn fd_41_overflow_looks_at_every_paragraph() {
    let wide = lay("W", TextAlign::Left).lines[0].advance;
    let width = f64::from(wide) - 1.0;
    for (text, overflow) in [
        ("W\ni", true),
        ("i\nW", true),
        ("i\nW\ni", true),
        ("i\ni\ni", false),
    ] {
        assert_eq!(
            lay_at(text, TextAlign::Left, width).overflow,
            overflow,
            "{text:?}"
        );
    }
}

#[test]
fn fd_41_i_justify_stretches_inside_each_paragraph_and_not_its_last_line() {
    let latin = "aa bb cc dd ee ff gg hh ii jj";
    let arabic = "\u{627}\u{628} \u{62c}\u{62f} \u{647}\u{648} \u{627}\u{628} \u{62c}\u{62f} \u{647}\u{648} \u{627}\u{628} \u{62c}\u{62f} \u{647}\u{648} \u{627}\u{628} \u{62c}\u{62f} \u{647}\u{648}";
    let text = format!("{latin}\n{arabic}\n{latin}");
    let out = lay(&text, TextAlign::Justify);
    let alone_latin = lay(latin, TextAlign::Justify);
    let alone_arabic = lay(arabic, TextAlign::Justify);
    assert!(alone_latin.lines.len() > 1 && alone_arabic.lines.len() > 1);
    let mut lines = out.lines.iter();
    for alone in [&alone_latin, &alone_arabic, &alone_latin] {
        for solo in &alone.lines {
            let line = lines.next().expect("one line per line alone");
            assert!(line.clusters.len() - solo.clusters.len() <= 1);
            for (c, s) in line
                .clusters
                .iter()
                .filter(|c| !is_break(&text[c.range.clone()]))
                .zip(&solo.clusters)
            {
                assert!(near(f64::from(c.x), f64::from(s.x)), "{} and {}", c.x, s.x);
            }
        }
    }
    assert!(lines.next().is_none());
}

#[test]
fn arq_116_ranges_index_the_whole_text_across_paragraphs() {
    let table = [
        format!("abc\n{ABJ}\r\ndef\u{2028}{ABJ} x\u{2029}\n"),
        format!("{ABJ}\n\n{MIXED}\r{MIXED}"),
        format!("\n{ABJ}\r\n"),
    ];
    for text in table {
        for width in [40.0, 100.0, 4012.0] {
            let out = lay_at(&text, TextAlign::Left, width);
            let mut logical: Vec<_> = out
                .lines
                .iter()
                .flat_map(|l| l.clusters.iter().map(|c| c.range.clone()))
                .collect();
            logical.sort_by_key(|r| r.start);
            let joined: String = logical.iter().map(|r| &text[r.clone()]).collect();
            assert_eq!(joined, text, "width {width}");
        }
    }
}

#[test]
fn arq_116_upper_case_keeps_the_ranges_across_paragraphs() {
    let text = format!("{ABJ}\nstra\u{df}e\ncd");
    let style = TextStyle {
        size_pt: 12.0,
        uppercase: true,
        ..TextStyle::default()
    };
    let out =
        layout_box(&text, &STACK, &style, 96.0, Size::new(1000.0, 1000.0)).expect("valid input");
    assert_eq!(out.lines.len(), 3);
    let sharp = out.lines[1]
        .clusters
        .iter()
        .filter(|c| &text[c.range.clone()] == "\u{df}")
        .count();
    assert_eq!(sharp, 2, "the two S of the sharp s share its range");
    assert_eq!(cluster(&text, &out.lines[2], "d").range.end, text.len());
}

#[test]
fn fd_41_a_list_item_has_bidi_paragraphs_too() {
    let text = format!("{ABJ}\n{MIXED}");
    let item = |text| ListItem {
        text,
        level: 1,
        kind: ListKind::Numbered,
    };
    let style = TextStyle {
        size_pt: 12.0,
        ..TextStyle::default()
    };
    let list = |text| {
        layout_list(
            &[item(text)],
            &STACK,
            &style,
            96.0,
            Size::new(120.0, 1000.0),
        )
        .expect("valid input")
    };
    let two = list(&text);
    let solo = list(MIXED);
    let line = &two[0].layout.lines[1];
    assert_eq!(
        shown(&text, line),
        [
            "a", "b", "c", " ", "\u{62c}", "\u{628}", "\u{627}", " ", "d", "e", "f"
        ]
    );
    for (c, s) in line.clusters.iter().zip(&solo[0].layout.lines[0].clusters) {
        assert!(near(f64::from(c.x), f64::from(s.x)));
    }
}
