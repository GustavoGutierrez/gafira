//! ARQ-116 (layout output), FD-41 (a) (accents, bidi, emoji), ARQ-110 (finite
//! output) and ARQ-111 (determinism) for `layout_box`.

use gafira_geom::Size;
use gafira_testkit::fonts;
use gafira_text::{TextError, TextLayout, TextStyle, layout_box};
use proptest::prelude::*;

const INTER: &[u8] = fonts::INTER.bytes;
const MANROPE: &[u8] = fonts::MANROPE.bytes;
const ARABIC: &[u8] = fonts::NOTO_SANS_ARABIC.bytes;
const EMOJI: &[u8] = fonts::NOTO_EMOJI.bytes;

/// 24 pt at 96 dpi is 32 px.
fn style() -> TextStyle {
    TextStyle {
        size_pt: 24.0,
        letter_spacing: 0.0,
        line_height: 1.0,
    }
}

fn lay_with(
    text: &str,
    stack: &[&[u8]],
    style: &TextStyle,
    dpi: f64,
    width: f64,
) -> Result<TextLayout, TextError> {
    layout_box(text, stack, style, dpi, Size::new(width, 1000.0))
}

fn lay_in(text: &str, stack: &[&[u8]], width: f64) -> TextLayout {
    lay_with(text, stack, &style(), 96.0, width).expect("valid input")
}

fn lay(text: &str) -> TextLayout {
    lay_in(text, &[INTER], 10_000.0)
}

/// The text of every cluster in logical order, and whether they tile the input.
fn logical_text(text: &str, layout: &TextLayout) -> (String, bool) {
    let mut ranges: Vec<_> = layout
        .lines
        .iter()
        .flat_map(|l| l.clusters.iter().map(|c| c.range.clone()))
        .collect();
    ranges.sort_by_key(|r| r.start);
    let mut end = 0;
    let mut tiles = true;
    let mut out = String::new();
    for r in ranges {
        tiles &= r.start == end;
        end = r.end;
        out.push_str(&text[r]);
    }
    (out, tiles && end == text.len())
}

/// The text of every cluster in visual order, line by line.
fn visual_clusters<'a>(text: &'a str, layout: &TextLayout) -> Vec<&'a str> {
    layout
        .lines
        .iter()
        .flat_map(|l| l.clusters.iter().map(|c| &text[c.range.clone()]))
        .collect()
}

#[test]
fn arq_116_three_forced_lines_give_three_lines() {
    assert_eq!(lay("one\ntwo\nthree").lines.len(), 3);
}

#[test]
fn arq_116_a_trailing_newline_starts_one_more_empty_line() {
    // Not stated by the spec: the count follows an editor, and the choice is recorded.
    let table = [
        ("", 1),
        ("\n", 2),
        ("a", 1),
        ("a\n", 2),
        ("a\nb", 2),
        ("a\nb\nc\n", 4),
        ("a\n\nb", 3),
    ];
    for (text, lines) in table {
        let layout = lay(text);
        assert_eq!(layout.lines.len(), lines, "{text:?}");
        let last = layout.lines.last().expect("at least one line");
        assert_eq!(
            last.clusters.is_empty(),
            text.is_empty() || text.ends_with('\n'),
            "{text:?}: only a line after a final newline, or an empty text, is empty"
        );
    }
}

#[test]
fn arq_116_lines_carry_a_growing_baseline_and_the_sum_of_their_advances() {
    let layout = lay("one\ntwo\nthree");
    assert_eq!(layout.lines.len(), 3);
    let mut previous = 0.0_f32;
    for line in &layout.lines {
        assert!(line.baseline > previous, "baselines grow: {layout:?}");
        previous = line.baseline;
        let sum: f32 = line.clusters.iter().map(|c| c.advance).sum();
        assert_eq!(line.advance, sum);
        let mut x = 0.0;
        for c in &line.clusters {
            assert_eq!(c.x, x, "clusters are placed one after another");
            x += c.advance;
        }
    }
}

#[test]
fn arq_116_a_cluster_advance_follows_the_size_in_px() {
    // Inter "a" advances 17.96875 px at 32 px (24 pt at 96 dpi), a linear scale of the size.
    let table = [
        (24.0, 96.0, 17.96875),
        (12.0, 96.0, 8.984375),
        (24.0, 48.0, 8.984375),
    ];
    for (size_pt, dpi, expected) in table {
        let s = TextStyle { size_pt, ..style() };
        let layout = lay_with("a", &[INTER], &s, dpi, 10_000.0).expect("valid input");
        let advance = layout.lines[0].clusters[0].advance;
        assert!(
            (advance - expected).abs() < 0.01,
            "{size_pt} pt at {dpi} dpi: {advance}"
        );
    }
}

#[test]
fn arq_116_clusters_in_logical_order_concatenate_to_the_input() {
    let table: [(&str, Vec<&[u8]>); 8] = [
        ("Hello world", vec![INTER]),
        ("one\ntwo\nthree\n", vec![INTER]),
        ("caf\u{e9} e\u{301}", vec![INTER]),
        ("abc \u{627}\u{628}\u{62c}", vec![INTER, ARABIC]),
        ("\u{627}\u{628}\u{62c} abc", vec![INTER, ARABIC]),
        (
            "\u{627}\u{628}\u{62c} 12 \u{627}\n abc",
            vec![INTER, ARABIC],
        ),
        ("\u{1f469}\u{200d}\u{1f4bb} \u{1f600}", vec![INTER, EMOJI]),
        ("fi fl ffi", vec![MANROPE]),
    ];
    for (text, stack) in table {
        let layout = lay_in(text, &stack, 10_000.0);
        assert_eq!(
            logical_text(text, &layout),
            (text.to_string(), true),
            "{text:?}"
        );
    }
}

#[test]
fn arq_116_wrapped_text_keeps_every_cluster_in_logical_order() {
    let text = "Hello wide world \u{627}\u{628}\u{62c} again";
    let layout = lay_in(text, &[INTER, ARABIC], 120.0);
    assert!(layout.lines.len() > 2, "the box is narrow");
    assert_eq!(logical_text(text, &layout), (text.to_string(), true));
}

#[test]
fn arq_116_a_composed_e_acute_is_one_cluster() {
    let table = [
        ("e\u{301}", 1),
        ("\u{e9}", 1),
        ("ae\u{301}b", 3),
        ("e\u{301}e\u{301}", 2),
    ];
    for (text, clusters) in table {
        let layout = lay(text);
        assert_eq!(layout.lines[0].clusters.len(), clusters, "{text:?}");
    }
    let layout = lay("e\u{301}");
    assert_eq!(layout.lines[0].clusters[0].range, 0..3);
}

#[test]
fn arq_116_a_text_wider_than_a_fixed_box_flags_overflow_and_keeps_all_text() {
    let text = "Wide";
    let layout = lay_in(text, &[INTER], 5.0);
    assert!(layout.overflow);
    assert_eq!(logical_text(text, &layout), (text.to_string(), true));
}

#[test]
fn arq_116_overflow_compares_with_greater_than() {
    let advance = f64::from(lay("W").lines[0].clusters[0].advance);
    for (width, overflow) in [
        (advance, false),
        (advance - 0.001, true),
        (advance + 0.001, false),
    ] {
        assert_eq!(
            lay_in("W", &[INTER], width).overflow,
            overflow,
            "width {width}"
        );
    }
}

#[test]
fn arq_116_a_word_wider_than_the_box_breaks_by_cluster_and_does_not_overflow() {
    // Only a single cluster wider than the box overflows (FD-41, Saltos).
    let text = "Hello";
    let layout = lay_in(text, &[INTER], 40.0);
    assert!(layout.lines.len() > 1);
    assert!(!layout.overflow);
    assert_eq!(logical_text(text, &layout), (text.to_string(), true));
}

#[test]
fn arq_116_trailing_space_that_hangs_past_the_box_is_not_overflow() {
    let advance = f64::from(lay("W").lines[0].clusters[0].advance);
    assert!(!lay_in("W ", &[INTER], advance).overflow);
}

#[test]
fn arq_116_overflow_in_a_box_of_size_zero_depends_on_having_text() {
    assert!(lay_in("a", &[INTER], 0.0).overflow);
    assert!(!lay_in("", &[INTER], 0.0).overflow);
}

#[test]
fn arq_116_clusters_without_cover_are_flagged_notdef() {
    let layout = lay("a\u{4e2d}b");
    let flags: Vec<bool> = layout.lines[0].clusters.iter().map(|c| c.notdef).collect();
    assert_eq!(flags, [false, true, false]);
    // Arabic has no cover in Inter alone.
    let arabic = lay("\u{627}\u{628}\u{62c}");
    assert_eq!(arabic.lines[0].clusters.len(), 3);
    assert!(arabic.lines[0].clusters.iter().all(|c| c.notdef));
}

#[test]
fn arq_116_a_forced_break_and_a_space_are_not_notdef() {
    let layout = lay("a b\nc");
    let total: usize = layout.lines.iter().map(|l| l.clusters.len()).sum();
    assert_eq!(total, 5, "a, space, b, break, c");
    assert!(
        layout
            .lines
            .iter()
            .flat_map(|l| &l.clusters)
            .all(|c| !c.notdef)
    );
}

#[test]
fn fd_41_a_bidi_text_is_ordered_visually_by_the_unicode_algorithm() {
    // abc ابج: a, b, c, space, then the Arabic letters from right to left.
    let text = "abc \u{627}\u{628}\u{62c}";
    let layout = lay_in(text, &[INTER, ARABIC], 10_000.0);
    assert_eq!(
        visual_clusters(text, &layout),
        ["a", "b", "c", " ", "\u{62c}", "\u{628}", "\u{627}"]
    );
    let clusters = &layout.lines[0].clusters;
    assert!(clusters[..4].iter().all(|c| !c.rtl));
    assert!(clusters[4..].iter().all(|c| c.rtl));
    assert!(clusters.iter().all(|c| !c.notdef));
    // Latin comes from the first font, Arabic from the second.
    assert_eq!(
        clusters.iter().map(|c| c.font).collect::<Vec<_>>(),
        [0, 0, 0, 0, 1, 1, 1]
    );
}

#[test]
fn fd_41_a_a_right_to_left_paragraph_keeps_logical_text_and_orders_visually() {
    let text = "\u{627}\u{628}\u{62c} abc";
    let layout = lay_in(text, &[INTER, ARABIC], 10_000.0);
    assert_eq!(
        visual_clusters(text, &layout),
        ["a", "b", "c", " ", "\u{62c}", "\u{628}", "\u{627}"]
    );
}

#[test]
fn fd_41_a_a_zwj_sequence_is_one_cluster_and_the_emoji_font_covers_it() {
    let text = "\u{1f469}\u{200d}\u{1f4bb}";
    let layout = lay_in(text, &[INTER, EMOJI], 10_000.0);
    let clusters = &layout.lines[0].clusters;
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].range, 0..text.len());
    assert!(!clusters[0].notdef);
    assert_eq!(clusters[0].font, 1);
}

#[test]
fn fd_41_a_the_emoji_subset_gives_a_smile_a_real_non_empty_outline() {
    let layout = lay_in("\u{1f600}", &[INTER, EMOJI], 10_000.0);
    let cluster = &layout.lines[0].clusters[0];
    assert_eq!((cluster.font, cluster.notdef), (1, false));
    let glyph = *cluster.glyphs.first().expect("a glyph");
    assert_ne!(glyph, 0);
    assert!(
        contours(EMOJI, glyph).is_some_and(|n| n != 0),
        "glyph {glyph} has no outline"
    );
    // Without the emoji font the same cluster is .notdef.
    assert!(lay("\u{1f600}").lines[0].clusters[0].notdef);
}

#[test]
fn fd_41_a_the_outline_reader_tells_a_letter_from_a_space() {
    let layout = lay("a b");
    let glyph = |i: usize| layout.lines[0].clusters[i].glyphs[0];
    assert!(contours(INTER, glyph(0)).is_some_and(|n| n != 0));
    assert_eq!(contours(INTER, glyph(1)), None);
}

/// Number of contours of a TrueType glyph (negative for a composite), or `None`
/// when the glyph has no outline data. Test-only, no dependency.
fn contours(font: &[u8], glyph: u32) -> Option<i16> {
    let be16 = |at: usize| -> Option<u16> {
        let b = font.get(at..at + 2)?;
        Some(u16::from_be_bytes([b[0], b[1]]))
    };
    let be32 = |at: usize| -> Option<usize> {
        let b = font.get(at..at + 4)?;
        usize::try_from(u32::from_be_bytes([b[0], b[1], b[2], b[3]])).ok()
    };
    let table = |tag: &[u8; 4]| -> Option<usize> {
        (0..usize::from(be16(4)?)).find_map(|i| {
            let record = 12 + 16 * i;
            if font.get(record..record + 4)? == tag {
                be32(record + 8)
            } else {
                None
            }
        })
    };
    let (head, loca, glyf) = (table(b"head")?, table(b"loca")?, table(b"glyf")?);
    let long = be16(head + 50)? != 0;
    let offset = |g: usize| -> Option<usize> {
        if long {
            be32(loca + 4 * g)
        } else {
            Some(usize::from(be16(loca + 2 * g)?) * 2)
        }
    };
    let g = usize::try_from(glyph).ok()?;
    let (start, end) = (offset(g)?, offset(g + 1)?);
    if end <= start {
        return None;
    }
    be16(glyf + start).map(|n| i16::from_be_bytes(n.to_be_bytes()))
}

/// All numbers of a layout as bits, to compare byte for byte.
fn fingerprint(layout: &TextLayout) -> Vec<u64> {
    let mut out = vec![u64::from(layout.overflow)];
    for line in &layout.lines {
        out.push(u64::from(line.baseline.to_bits()));
        out.push(u64::from(line.advance.to_bits()));
        for c in &line.clusters {
            out.push(c.range.start as u64);
            out.push(c.range.end as u64);
            out.push(u64::from(c.x.to_bits()));
            out.push(u64::from(c.advance.to_bits()));
            out.push(u64::from(c.rtl) | u64::from(c.notdef) << 1 | (c.font as u64) << 2);
            out.extend(c.glyphs.iter().map(|g| u64::from(*g)));
        }
    }
    out
}

#[test]
fn arq_111_the_same_input_gives_the_same_bytes_twice() {
    let stack: [&[u8]; 3] = [INTER, ARABIC, EMOJI];
    for text in [
        "Hello wide world \u{627}\u{628}\u{62c} \u{1f600} e\u{301}\n\u{4e2d}",
        "",
        "W",
    ] {
        for width in [10_000.0, 120.0, 0.0] {
            let a = lay_in(text, &stack, width);
            let b = lay_in(text, &stack, width);
            assert!(!a.lines.is_empty() && fingerprint(&a).len() > 1);
            assert_eq!(fingerprint(&a), fingerprint(&b), "{text:?} in {width}");
        }
    }
}

#[test]
fn arq_110_bytes_that_are_not_a_font_give_invalid_font_with_the_position() {
    let junk: &[u8] = b"not a font";
    for (stack, index) in [
        (vec![junk], 0),
        (vec![INTER, junk], 1),
        (vec![INTER, EMOJI, &[]], 2),
    ] {
        assert_eq!(
            lay_with("a", &stack, &style(), 96.0, 100.0).err(),
            Some(TextError::InvalidFont { index }),
            "{stack:?}"
        );
    }
}

#[test]
fn arq_110_a_sum_that_overflows_in_the_output_gives_err_and_no_infinity() {
    // Each advance is finite in f32 (about 1e37 px), the line is not.
    let s = TextStyle {
        size_pt: 1000.0,
        ..style()
    };
    let text = "W".repeat(100);
    assert_eq!(
        lay_with(&text, &[INTER], &s, 1e36, 100.0).err(),
        Some(TextError::NonFinite { field: "output" })
    );
}

/// C1: a trailing space inside a right-to-left run is not part of the content.
/// With break by cluster, a text overflows exactly when one cluster that is not
/// white space is wider than the box; the widths are swept in steps of 1/16 px.
#[test]
fn arq_116_overflow_in_mixed_direction_text_depends_only_on_the_widest_cluster() {
    let texts = [
        "a \u{627}\u{628}\u{62c} \u{62f}\u{647}\u{648}",
        "hello world \u{627}\u{628}\u{62c} \u{62f}\u{647}\u{648}",
        "\u{645}\u{631}\u{62d}\u{628}\u{627} \u{628}\u{627}\u{644}\u{639}\u{627}\u{644}\u{645} hello world",
    ];
    for text in texts {
        let wide = lay_in(text, &[INTER, ARABIC], 100_000.0);
        let widest = wide
            .lines
            .iter()
            .flat_map(|l| &l.clusters)
            .filter(|c| !text[c.range.clone()].trim().is_empty())
            .map(|c| f64::from(c.advance))
            .fold(0.0, f64::max);
        assert!(widest > 0.0);
        for step in 0..=4000 {
            let width = f64::from(step) / 16.0;
            let layout = lay_in(text, &[INTER, ARABIC], width);
            assert_eq!(layout.overflow, width < widest, "{text:?} in {width}");
        }
    }
}

#[test]
fn arq_116_overflow_looks_at_every_line() {
    // The first line holds the only cluster that is too wide.
    let wide = f64::from(lay("W").lines[0].clusters[0].advance);
    let layout = lay_in("W\ni", &[INTER], wide - 1.0);
    assert_eq!(layout.lines.len(), 2);
    assert!(layout.overflow);
}

#[test]
fn arq_116_a_cluster_with_a_later_glyph_without_cover_is_notdef() {
    // q + U+0301: Inter has the letter and not the mark, so the glyphs are [q, .notdef].
    let cluster = &lay("q\u{301}").lines[0].clusters[0];
    assert_eq!(cluster.glyphs.len(), 2);
    assert_ne!(cluster.glyphs[0], 0);
    assert!(cluster.notdef);
}

#[test]
fn arq_110_the_numbers_are_checked_in_order_non_finite_then_range_then_fonts() {
    let junk: &[u8] = b"not a font";
    let bad_dpi = |size_pt, stack: &[&[u8]]| {
        let s = TextStyle { size_pt, ..style() };
        lay_with("a", stack, &s, f64::NAN, 100.0).err()
    };
    let non_finite = Some(TextError::NonFinite { field: "dpi" });
    // A range error does not hide a later non-finite number.
    assert_eq!(bad_dpi(0.5, &[INTER]), non_finite);
    // A bad font does not hide a non-finite number.
    assert_eq!(bad_dpi(24.0, &[junk]), non_finite);
    assert_eq!(bad_dpi(24.0, &[]), non_finite);
    // A range error comes before a bad font.
    let s = TextStyle {
        size_pt: 0.5,
        ..style()
    };
    assert_eq!(
        lay_with("a", &[junk], &s, 96.0, 100.0).err(),
        Some(TextError::OutOfRange {
            field: "style.size_pt"
        })
    );
}

#[test]
fn fd_41_an_empty_font_list_is_an_error_also_for_an_empty_text() {
    for text in ["abc", ""] {
        assert_eq!(
            lay_with(text, &[], &style(), 96.0, 100.0).err(),
            Some(TextError::NoFont),
            "{text:?}"
        );
    }
}

#[test]
fn arq_116_a_final_crlf_or_lf_opens_one_more_empty_line() {
    let table = [("abc\r\n", 2), ("abc\n", 2), ("\r\n", 2), ("a\r\n\r\nb", 3)];
    for (text, lines) in table {
        let layout = lay(text);
        assert_eq!(layout.lines.len(), lines, "{text:?}");
        assert_eq!(logical_text(text, &layout), (text.to_string(), true));
    }
    assert!(lay("abc\r\n").lines[1].clusters.is_empty());
}

#[test]
fn arq_116_in_a_box_of_size_zero_only_a_cluster_that_is_not_white_space_overflows() {
    // 02 4.17.3, Caja de tamaño 0: White_Space of Unicode does not count.
    let table = [
        (" ", false),
        ("   ", false),
        ("\t", false),
        ("\n", false),
        ("\r\n", false),
        ("\u{a0}", false),
        ("\u{2003}", false),
        ("\u{3000}", false),
        ("a", true),
        (" a", true),
        ("a ", true),
        ("\n\na", true),
    ];
    for (text, overflow) in table {
        assert_eq!(lay_in(text, &[INTER], 0.0).overflow, overflow, "{text:?}");
    }
}

#[test]
fn arq_116_a_very_long_run_of_trailing_white_space_is_laid_out() {
    // W12: the cost of the white space at the end of a line is linear.
    let text = format!("a{}", " ".repeat(100_000));
    let layout = lay_in(&text, &[INTER], 100_000.0);
    assert_eq!(layout.lines.len(), 1);
    assert_eq!(layout.lines[0].clusters.len(), 100_001);
    assert!(!layout.overflow);
}

fn pieces() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec![
        "a",
        "W",
        " ",
        "\n",
        "\u{e9}",
        "e\u{301}",
        "\u{627}",
        "\u{62c}",
        "\u{4e2d}",
        "\u{1f600}",
        "\u{1f469}\u{200d}\u{1f4bb}",
    ])
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    // ARQ-110: finite input in the domain gives only finite numbers.
    #[test]
    fn arq_110_finite_input_in_the_domain_gives_finite_output(
        parts in prop::collection::vec(pieces(), 0..12),
        size_pt in 1.0..=1000.0_f64,
        spacing in -100.0..=500.0_f64,
        line_height in 0.5..=4.0_f64,
        dpi in 1.0..=1200.0_f64,
        width in 0.0..=100_000.0_f64,
    ) {
        let text = parts.concat();
        let s = TextStyle { size_pt, letter_spacing: spacing, line_height };
        let layout = lay_with(&text, &[INTER, ARABIC, EMOJI], &s, dpi, width)
            .expect("input in the domain");
        for line in &layout.lines {
            prop_assert!(line.baseline.is_finite() && line.advance.is_finite());
            for c in &line.clusters {
                prop_assert!(c.x.is_finite() && c.advance.is_finite());
            }
        }
        prop_assert_eq!(logical_text(&text, &layout), (text.clone(), true));
    }
}
