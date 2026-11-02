//! FD-41: where a line sits in the box. One fit rule for the break and the overflow, the
//! hang before the room, justified positions summed in f64 and no stretch of leading
//! white space. (l): advances are the linear scaling of `hmtx`.

use gafira_geom::Size;
use gafira_testkit::fonts;
use gafira_text::{Line, TextAlign, TextLayout, TextStyle, layout_box};

const INTER: &[u8] = fonts::INTER.bytes;
const ARABIC: &[u8] = fonts::NOTO_SANS_ARABIC.bytes;

fn lay(text: &str, stack: &[&[u8]], align: TextAlign, width: f64, t: f64) -> TextLayout {
    let s = TextStyle {
        size_pt: 12.0,
        align,
        letter_spacing: t,
        ..TextStyle::default()
    };
    layout_box(text, stack, &s, 96.0, Size::new(width, 100_000.0)).expect("valid input")
}

fn ink(text: &str, line: &Line) -> (f64, f64) {
    let mut it = line
        .clusters
        .iter()
        .filter(|c| !text[c.range.clone()].trim().is_empty());
    let first = it.next().expect("a line with text");
    it.fold(
        (
            f64::from(first.x),
            f64::from(first.x) + f64::from(first.advance),
        ),
        |(l, r), c| {
            (
                l.min(f64::from(c.x)),
                r.max(f64::from(c.x) + f64::from(c.advance)),
            )
        },
    )
}

#[test]
fn fd_41_c_a_box_within_the_fit_slack_neither_breaks_nor_overflows() {
    // C18: the same rule decides the break and the overflow. A box a hair narrower than
    // the line keeps it whole, and then no cluster is wider than the box.
    for text in ["Hello", "Hello world", "Gafira"] {
        let exact = lay(text, &[INTER], TextAlign::Left, 100_000.0, 0.0).lines[0].advance;
        let widest = lay(text, &[INTER], TextAlign::Left, 100_000.0, 0.0).lines[0]
            .clusters
            .iter()
            .map(|c| f64::from(c.advance))
            .fold(0.0, f64::max);
        for k in [0.5e-6, 1e-6, 2e-6, 3e-6] {
            let width = f64::from(exact) * (1.0 - k);
            let out = lay(text, &[INTER], TextAlign::Left, width, 0.0);
            if out.lines.len() == 1 {
                assert!(widest <= width, "test needs a narrow cluster");
                assert!(!out.overflow, "{text} k={k}: kept whole yet overflow");
            }
        }
    }
}

#[test]
fn fd_41_c_overflow_is_true_only_with_a_cluster_wider_than_the_box() {
    // C18 again, from the other side: a box clearly narrower than a cluster overflows.
    let out = lay("a", &[INTER], TextAlign::Left, 1.0, 0.0);
    assert!(out.overflow);
    let out = lay("Hello", &[INTER], TextAlign::Left, 5.0, 0.0);
    assert!(out.overflow);
}

#[test]
fn fd_41_hang_applies_when_there_is_no_room_in_a_right_to_left_line() {
    // C19: the paragraph-level white space hangs on the left (negative x) whatever the
    // room, also when the content is wider than the box.
    for align in [TextAlign::Left, TextAlign::Center, TextAlign::Right] {
        let text = "ا  ";
        let out = lay(text, &[INTER, ARABIC], align, 1.0, 0.0);
        let line = &out.lines[0];
        let (left, _) = ink(text, line);
        assert!(left.abs() <= 0.01, "{align:?}: content starts at {left}");
        let hung: Vec<f32> = line
            .clusters
            .iter()
            .filter(|c| text[c.range.clone()].trim().is_empty())
            .map(|c| c.x)
            .collect();
        assert!(
            !hung.is_empty() && hung.iter().all(|x| *x < 0.0),
            "{align:?}: {hung:?}"
        );
    }
}

#[test]
fn fd_41_i_a_justified_line_ends_at_the_edge_in_a_very_wide_box() {
    // C20: positions are summed in f64 from the origin of the line.
    let text = "ab ".repeat(2600);
    let width = 16_000.0;
    let out = lay(&text, &[INTER], TextAlign::Justify, width, 0.0);
    assert!(out.lines.len() >= 2);
    let first = &out.lines[0];
    let (_, right) = ink(&text, first);
    assert!((right - width).abs() <= 0.002, "ends at {right}");
}

#[test]
fn fd_41_i_leading_white_space_takes_no_extra_spacing() {
    // C21.
    let text = "   aa bb cc dd ee ff gg hh ii jj kk ll mm nn oo pp qq rr ss tt uu vv";
    let plain = lay(text, &[INTER], TextAlign::Left, 200.0, 0.0);
    let just = lay(text, &[INTER], TextAlign::Justify, 200.0, 0.0);
    assert!(just.lines.len() >= 2);
    let (l, r) = ink(text, &just.lines[0]);
    for i in 0..3 {
        assert_eq!(
            just.lines[0].clusters[i].advance,
            plain.lines[0].clusters[i].advance
        );
    }
    assert!((r - 200.0).abs() <= 0.01, "ends at {r}");
    assert!(
        (l - 3.0 * plain.lines[0].clusters[0].advance as f64).abs() <= 0.01,
        "starts at {l}"
    );
}

#[test]
fn fd_41_l_advances_are_the_linear_scaling_of_hmtx() {
    // Table `hmtx` scaled by size / unitsPerEm, with no rounding at any fractional size.
    let be16 = |b: &[u8], o: usize| usize::from(u16::from_be_bytes([b[o], b[o + 1]]));
    let table = |tag: &[u8; 4]| -> (usize, usize) {
        let n = be16(INTER, 4);
        (0..n)
            .map(|i| 12 + 16 * i)
            .find(|r| &INTER[*r..*r + 4] == tag)
            .map(|r| {
                let o = u32::from_be_bytes(INTER[r + 8..r + 12].try_into().unwrap()) as usize;
                (o, 0)
            })
            .expect("table")
    };
    let upem = f64::from(u16::try_from(be16(INTER, table(b"head").0 + 18)).unwrap());
    let long_metrics = be16(INTER, table(b"hhea").0 + 34);
    let hmtx = table(b"hmtx").0;
    let advance_units = |gid: usize| be16(INTER, hmtx + 4 * gid.min(long_metrics - 1));
    let mut checked = 0;
    for step in 0..400 {
        let pt = 6.0 + f64::from(step) * 0.37 + 0.013;
        let size_px = pt * 96.0 / 72.0;
        for ch in ["H", "o", "g", "W", "7"] {
            let s = TextStyle {
                size_pt: pt,
                ..TextStyle::default()
            };
            let out = layout_box(ch, &[INTER], &s, 96.0, Size::new(1e6, 1e6)).unwrap();
            let c = &out.lines[0].clusters[0];
            let want = advance_units(c.glyphs[0] as usize) as f64 * size_px / upem;
            assert!(
                (f64::from(c.advance) - want).abs() <= want * 1e-6,
                "{ch} {pt}: {} vs {want}",
                c.advance
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 2000);
}
