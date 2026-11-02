//! FD-41 (c) identity, (e) letter spacing and optional ligatures, (f) line height,
//! and "Fuente sin métricas" (02 section 4.17.3).

use std::borrow::Cow;
use std::sync::Arc;

use gafira_geom::Size;
use gafira_testkit::fonts;
use gafira_text::{TextError, TextLayout, TextStyle, layout_box};
use parley::fontique::Blob;
use parley::{FontContext, FontFamily, FontFamilyName, LayoutContext, OverflowWrap, StyleProperty};

const INTER: &[u8] = fonts::INTER.bytes;
const MANROPE: &[u8] = fonts::MANROPE.bytes;
const ARABIC: &[u8] = fonts::NOTO_SANS_ARABIC.bytes;

/// 24 pt at 96 dpi is 32 px.
const SIZE_PX: f64 = 32.0;

fn style(letter_spacing: f64, line_height: f64) -> TextStyle {
    TextStyle {
        size_pt: 24.0,
        letter_spacing,
        line_height,
        ..TextStyle::default()
    }
}

fn lay(text: &str, stack: &[&[u8]], s: &TextStyle, width: f64) -> TextLayout {
    layout_box(text, stack, s, 96.0, Size::new(width, 1000.0)).expect("valid input")
}

// ---- a font table reader and patcher, test only --------------------------------------------

fn be16(font: &[u8], at: usize) -> i16 {
    i16::from_be_bytes([font[at], font[at + 1]])
}

/// Offset of a table of the font.
fn table(font: &[u8], tag: &[u8; 4]) -> usize {
    let count = usize::from(u16::from_be_bytes([font[4], font[5]]));
    (0..count)
        .map(|i| 12 + 16 * i)
        .find(|rec| &font[*rec..*rec + 4] == tag)
        .map(|rec| {
            let b = &font[rec + 8..rec + 12];
            u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize
        })
        .expect("table")
}

/// A copy of the font with 16-bit values written into one table.
fn patched(font: &[u8], tag: &[u8; 4], edits: &[(usize, i16)]) -> Vec<u8> {
    let base = table(font, tag);
    let mut copy = font.to_vec();
    for (offset, value) in edits {
        copy[base + offset..base + offset + 2].copy_from_slice(&value.to_be_bytes());
    }
    copy
}

/// The natural line height in px of 04 FD-41: ascent - descent + line gap, from `OS/2`
/// when `USE_TYPO_METRICS` is set and from `hhea` otherwise.
fn natural_height(font: &[u8], size_px: f64) -> f64 {
    let upem = f64::from(be16(font, table(font, b"head") + 18));
    let os2 = table(font, b"OS/2");
    let (asc, desc, gap) = if be16(font, os2 + 62) & 0x80 != 0 {
        (
            be16(font, os2 + 68),
            be16(font, os2 + 70),
            be16(font, os2 + 72),
        )
    } else {
        let hhea = table(font, b"hhea");
        (
            be16(font, hhea + 4),
            be16(font, hhea + 6),
            be16(font, hhea + 8),
        )
    };
    f64::from(i32::from(asc) - i32::from(desc) + i32::from(gap)) * size_px / upem
}

// ---- FD-41 (c): the identity --------------------------------------------------------------

/// What parley gives with none of the spacing or line height properties: baselines,
/// then `(x, advance)` of the clusters of each line, as bits.
fn raw_parley(text: &str, stack: &[&[u8]], width: f32) -> Vec<(u32, Vec<(u32, u32)>)> {
    let mut fcx = FontContext::new();
    let mut names = Vec::new();
    for bytes in stack {
        let regs = fcx
            .collection
            .register_fonts(Blob::new(Arc::new(bytes.to_vec())), None);
        let id = regs[0].0;
        let name = fcx.collection.family_name(id).expect("name").to_string();
        names.push(FontFamilyName::Named(Cow::Owned(name)));
    }
    let mut lcx: LayoutContext<()> = LayoutContext::new();
    let mut b = lcx.ranged_builder(&mut fcx, text, 1.0, false);
    b.push_default(StyleProperty::FontSize(SIZE_PX as f32));
    b.push_default(StyleProperty::FontFamily(FontFamily::List(Cow::Owned(
        names,
    ))));
    b.push_default(StyleProperty::OverflowWrap(OverflowWrap::Anywhere));
    let mut laid = b.build(text);
    laid.break_all_lines(Some(width));
    laid.lines()
        .map(|line| {
            let mut x = line.metrics().offset;
            let mut clusters = Vec::new();
            for run in line.runs() {
                for c in run.visual_clusters() {
                    clusters.push((x.to_bits(), c.advance().to_bits()));
                    x += c.advance();
                }
            }
            (line.metrics().baseline.to_bits(), clusters)
        })
        .collect()
}

fn bits(layout: &TextLayout) -> Vec<(u32, Vec<(u32, u32)>)> {
    layout
        .lines
        .iter()
        .map(|l| {
            let clusters = l
                .clusters
                .iter()
                .map(|c| (c.x.to_bits(), c.advance.to_bits()));
            (l.baseline.to_bits(), clusters.collect())
        })
        .collect()
}

#[test]
fn fd_41_c_spacing_0_and_line_height_1_are_the_identity_bit_for_bit() {
    let texts: [(&str, Vec<&[u8]>, f64); 5] = [
        ("Hello world, wide and wrapped", vec![INTER], 150.0),
        ("one\ntwo\n\nthree", vec![INTER], 10_000.0),
        (
            "abc \u{627}\u{628}\u{62c} \u{62f}\u{647}\u{648} again",
            vec![INTER, ARABIC],
            140.0,
        ),
        ("fi fl ffi", vec![MANROPE], 10_000.0),
        ("", vec![INTER], 100.0),
    ];
    for (text, stack, width) in texts {
        let ours = lay(text, &stack, &style(0.0, 1.0), width);
        let width = width as f32;
        assert_eq!(bits(&ours), raw_parley(text, &stack, width), "{text:?}");
    }
}

#[test]
fn fd_41_c_negative_zero_spacing_is_also_the_identity() {
    let ours = lay("Hello", &[INTER], &style(-0.0, 1.0), 1000.0);
    assert_eq!(bits(&ours), raw_parley("Hello", &[INTER], 1000.0));
}

// ---- FD-41 (e): letter spacing ----------------------------------------------------------------

/// Number of clusters of a line without the white space at its end, which hangs.
fn visible(text: &str, line: &gafira_text::Line) -> usize {
    let mut by_start: Vec<_> = line.clusters.iter().collect();
    by_start.sort_by_key(|c| c.range.start);
    by_start.len()
        - by_start
            .iter()
            .rev()
            .take_while(|c| text[c.range.clone()].trim().is_empty())
            .count()
}

#[test]
fn fd_41_e_a_line_grows_by_n_minus_1_times_the_spacing() {
    let texts: [(&str, Vec<&[u8]>); 6] = [
        ("Hello", vec![INTER]),
        ("a", vec![INTER]),
        ("ab  ", vec![INTER]),
        ("ab  \ncde  \n", vec![INTER]),
        ("ab\ncde\n", vec![INTER]),
        ("abc \u{627}\u{628}\u{62c}", vec![INTER, ARABIC]),
    ];
    for (text, stack) in texts {
        let base = lay(text, &stack, &style(0.0, 1.0), 10_000.0);
        for t in [-100.0, -10.5, 1.0, 100.0, 500.0] {
            let spaced = lay(text, &stack, &style(t, 1.0), 10_000.0);
            assert_eq!(spaced.lines.len(), base.lines.len(), "{text:?} t={t}");
            for (a, b) in base.lines.iter().zip(&spaced.lines) {
                let n = visible(text, a);
                let expected = (n.max(1) - 1) as f64 * t / 1000.0 * SIZE_PX;
                let got = f64::from(b.advance) - f64::from(a.advance);
                assert!(
                    (got - expected).abs() <= 0.01,
                    "{text:?} t={t}: {got} vs {expected}"
                );
            }
        }
    }
}

#[test]
fn fd_41_e_spacing_goes_after_each_cluster_and_never_after_the_last_of_a_line() {
    let text = "abc";
    let base = lay(text, &[INTER], &style(0.0, 1.0), 10_000.0);
    let t = 250.0;
    let spaced = lay(text, &[INTER], &style(t, 1.0), 10_000.0);
    let step = t / 1000.0 * SIZE_PX;
    let (a, b) = (&base.lines[0].clusters, &spaced.lines[0].clusters);
    for i in 0..3 {
        let moved = f64::from(b[i].x) - f64::from(a[i].x);
        assert!(
            (moved - step * i as f64).abs() <= 0.01,
            "cluster {i} moves {moved}"
        );
    }
    // The last cluster keeps its own advance, the others gain the step.
    assert!((f64::from(b[2].advance) - f64::from(a[2].advance)).abs() <= 0.01);
    assert!((f64::from(b[0].advance) - f64::from(a[0].advance) - step).abs() <= 0.01);
}

#[test]
fn fd_41_e_optional_ligatures_form_with_0_and_not_with_any_other_spacing() {
    let text = "fi";
    let with = lay(text, &[MANROPE], &style(0.0, 1.0), 10_000.0);
    let glyphs = |l: &TextLayout| {
        l.lines[0]
            .clusters
            .iter()
            .map(|c| c.glyphs.clone())
            .collect::<Vec<_>>()
    };
    let formed = glyphs(&with);
    assert_eq!(formed[0].len(), 1, "one ligature glyph: {formed:?}");
    assert!(
        formed[1].is_empty(),
        "the second cluster is a continuation: {formed:?}"
    );
    for t in [-1.0, 1e-6, -1e-9, 0.5, 1.0, 500.0] {
        let spaced = glyphs(&lay(text, &[MANROPE], &style(t, 1.0), 10_000.0));
        assert!(spaced.iter().all(|g| g.len() == 1), "t={t}: {spaced:?}");
        assert_ne!(
            spaced[0], formed[0],
            "t={t}: the ligature glyph must not appear"
        );
    }
}

#[test]
fn fd_41_e_required_ligatures_of_a_script_stay_with_spacing() {
    // Lam + alef is a required ligature, and the joining forms are required too.
    let text = "\u{644}\u{627} \u{627}\u{628}\u{62c}";
    let glyphs = |t: f64| {
        let l = lay(text, &[INTER, ARABIC], &style(t, 1.0), 10_000.0);
        l.lines[0]
            .clusters
            .iter()
            .map(|c| c.glyphs.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(glyphs(0.0), glyphs(100.0));
    assert_eq!(glyphs(0.0), glyphs(-50.0));
}

// ---- FD-41 (f): line height -------------------------------------------------------------------

fn baselines(layout: &TextLayout) -> Vec<f64> {
    layout.lines.iter().map(|l| f64::from(l.baseline)).collect()
}

#[test]
fn fd_41_f_the_distance_between_baselines_is_line_height_times_the_natural_height() {
    let natural = natural_height(INTER, SIZE_PX);
    for lh in [0.5, 1.0, 1.5, 2.0, 4.0] {
        let b = baselines(&lay("a\nb\nc", &[INTER], &style(0.0, lh), 10_000.0));
        for pair in b.windows(2) {
            assert!(
                (pair[1] - pair[0] - lh * natural).abs() <= 0.01,
                "lh={lh}: {b:?}"
            );
        }
    }
}

#[test]
fn fd_41_f_2_is_twice_1_for_the_same_text() {
    let one = baselines(&lay("a\nb\nc", &[INTER], &style(0.0, 1.0), 10_000.0));
    let two = baselines(&lay("a\nb\nc", &[INTER], &style(0.0, 2.0), 10_000.0));
    assert!(((two[1] - two[0]) - 2.0 * (one[1] - one[0])).abs() <= 0.01);
}

#[test]
fn fd_41_f_the_natural_height_is_the_largest_among_the_fonts_of_the_line() {
    // Every line has Latin (Inter) first and Arabic (taller) after it.
    let text = "a \u{627}\na \u{627}\na \u{627}";
    let expected = natural_height(INTER, SIZE_PX).max(natural_height(ARABIC, SIZE_PX));
    assert!(
        expected > natural_height(INTER, SIZE_PX) + 1.0,
        "the fixture needs a taller font"
    );
    let b = baselines(&lay(text, &[INTER, ARABIC], &style(0.0, 1.0), 10_000.0));
    assert!(
        (b[1] - b[0] - expected).abs() <= 0.01,
        "{b:?} vs {expected}"
    );
    assert!(
        (b[2] - b[1] - expected).abs() <= 0.01,
        "{b:?} vs {expected}"
    );
}

#[test]
fn fd_41_f_the_metrics_come_from_os2_with_use_typo_metrics_and_from_hhea_without() {
    // OS/2: ascent, descent and gap at 68, 70 and 72; fsSelection at 62; hhea at 4, 6 and 8.
    let os2 = table(INTER, b"OS/2");
    let selection = be16(INTER, os2 + 62);
    let typo = [(68, 3000), (70, -1000), (72, 200)];
    let with_flag = {
        let mut edits = typo.to_vec();
        edits.push((62, selection | 0x80));
        patched(INTER, b"OS/2", &edits)
    };
    let without_flag = {
        let mut edits = typo.to_vec();
        edits.push((62, selection & !0x80));
        patched(INTER, b"OS/2", &edits)
    };
    let upem = 2048.0;
    let typo_height = (3000.0 + 1000.0 + 200.0) * SIZE_PX / upem;
    let hhea_height = (1984.0 + 494.0) * SIZE_PX / upem;
    for (font, expected) in [(&with_flag, typo_height), (&without_flag, hhea_height)] {
        assert!((natural_height(font, SIZE_PX) - expected).abs() < 1e-9);
        let b = baselines(&lay("a\nb", &[font.as_slice()], &style(0.0, 1.0), 10_000.0));
        assert!(
            (b[1] - b[0] - expected).abs() <= 0.01,
            "{b:?} vs {expected}"
        );
    }
}

#[test]
fn fd_41_f_the_line_gap_counts_in_the_natural_height() {
    let gap = patched(INTER, b"hhea", &[(8, 400)]);
    let os2 = table(INTER, b"OS/2");
    let font = patched(
        &gap,
        b"OS/2",
        &[(62, be16(INTER, os2 + 62) & !0x80), (72, 400)],
    );
    let expected = (1984.0 + 494.0 + 400.0) * SIZE_PX / 2048.0;
    let b = baselines(&lay("a\nb", &[font.as_slice()], &style(0.0, 1.0), 10_000.0));
    assert!(
        (b[1] - b[0] - expected).abs() <= 0.01,
        "{b:?} vs {expected}"
    );
}

// ---- "Fuente sin métricas" ---------------------------------------------------------------------

/// A copy of the font whose ascent, descent and line gap are 0 everywhere.
fn without_metrics(font: &[u8]) -> Vec<u8> {
    let zero_hhea = patched(font, b"hhea", &[(4, 0), (6, 0), (8, 0)]);
    let zero_typo = patched(&zero_hhea, b"OS/2", &[(68, 0), (70, 0), (72, 0)]);
    patched(&zero_typo, b"OS/2", &[(74, 0), (76, 0)])
}

#[test]
fn fd_41_a_font_without_metrics_gives_invalid_font_with_its_position() {
    let broken = without_metrics(INTER);
    let broken_arabic = without_metrics(ARABIC);
    let s = style(0.0, 1.0);
    let run =
        |text: &str, stack: &[&[u8]]| layout_box(text, stack, &s, 96.0, Size::new(500.0, 100.0));
    assert_eq!(
        run("a", &[broken.as_slice()]).err(),
        Some(TextError::InvalidFont { index: 0 })
    );
    assert_eq!(
        run("\u{627}", &[INTER, broken_arabic.as_slice()]).err(),
        Some(TextError::InvalidFont { index: 1 })
    );
    // A healthy font in the same stack is fine, and so is a broken one nothing uses.
    assert!(run("a", &[INTER, broken_arabic.as_slice()]).is_ok());
    assert!(run("a", &[INTER]).is_ok());
}

// ---- the spacing the output keeps is the spacing that breaks lines ---------------------------

/// Width of the one line of `text` in a very wide box.
fn one_line(text: &str, t: f64) -> f64 {
    let l = lay(text, &[INTER], &style(t, 1.0), 100_000.0);
    assert_eq!(l.lines.len(), 1);
    f64::from(l.lines[0].advance)
}

#[test]
fn fd_41_e_a_word_fits_a_box_as_wide_as_its_line_with_any_spacing() {
    for t in [500.0, 100.0, 37.5, -50.0, -100.0] {
        let advance = one_line("Hello", t);
        let fits = lay("Hello", &[INTER], &style(t, 1.0), advance + 0.001);
        assert_eq!(fits.lines.len(), 1, "t={t}");
        assert!(!fits.overflow, "t={t}");
    }
}

#[test]
fn fd_41_e_a_word_wider_than_the_box_breaks_by_cluster_with_any_spacing() {
    for t in [500.0, 100.0, -50.0, -100.0] {
        let advance = one_line("Hello", t);
        let narrow = lay("Hello", &[INTER], &style(t, 1.0), advance - 1.0);
        assert!(
            narrow.lines.len() >= 2,
            "t={t}: {} lines",
            narrow.lines.len()
        );
        assert!(!narrow.overflow, "t={t}: no cluster is wider than the box");
    }
}

fn advances(layout: &TextLayout) -> Vec<(std::ops::Range<usize>, f64)> {
    let mut all: Vec<_> = layout
        .lines
        .iter()
        .flat_map(|l| {
            l.clusters
                .iter()
                .map(|c| (c.range.clone(), f64::from(c.advance)))
        })
        .collect();
    all.sort_by_key(|(r, _)| r.start);
    all
}

/// The gain of each cluster, in logical order, from spacing 0 to spacing `t`.
fn gains(text: &str, stack: &[&[u8]], t: f64) -> Vec<f64> {
    let base = advances(&lay(text, stack, &style(0.0, 1.0), 100_000.0));
    let spaced = advances(&lay(text, stack, &style(t, 1.0), 100_000.0));
    assert_eq!(base.len(), spaced.len());
    base.iter().zip(&spaced).map(|(a, b)| b.1 - a.1).collect()
}

#[test]
fn fd_41_e_the_last_cluster_in_logical_order_is_the_one_without_spacing() {
    // Right to left: the logically last letter is the leftmost one.
    let step = 250.0 / 1000.0 * SIZE_PX;
    let g = gains("\u{627}\u{628}\u{62c}", &[INTER, ARABIC], 250.0);
    assert_eq!(g.len(), 3);
    assert!(
        (g[0] - step).abs() <= 0.01 && (g[1] - step).abs() <= 0.01,
        "{g:?}"
    );
    assert!(g[2].abs() <= 0.01, "{g:?}");
}

#[test]
fn fd_41_e_a_forced_break_is_not_the_last_cluster_and_gets_no_spacing() {
    let step = 250.0 / 1000.0 * SIZE_PX;
    let g = gains("ab\ncd", &[INTER], 250.0);
    // a, b, break, c, d: the break keeps its zero advance, b is the last of its line.
    assert_eq!(g.len(), 5);
    assert!((g[0] - step).abs() <= 0.01, "{g:?}");
    assert!(g[1].abs() <= 0.01, "{g:?}");
    assert!(g[2].abs() <= 0.01, "{g:?}");
    assert!((g[3] - step).abs() <= 0.01 && g[4].abs() <= 0.01, "{g:?}");
    let layout = lay("ab\ncd", &[INTER], &style(250.0, 1.0), 100_000.0);
    assert_eq!(layout.lines[0].clusters[2].advance, 0.0);
}

#[test]
fn fd_41_e_a_hanging_space_is_not_the_last_cluster_and_gets_no_spacing() {
    let step = 250.0 / 1000.0 * SIZE_PX;
    let g = gains("ab  ", &[INTER], 250.0);
    assert_eq!(g.len(), 4);
    assert!((g[0] - step).abs() <= 0.01, "{g:?}");
    assert!(
        g[1].abs() <= 0.01 && g[2].abs() <= 0.01 && g[3].abs() <= 0.01,
        "{g:?}"
    );
}

#[test]
fn fd_41_f_the_height_of_n_lines_is_n_times_the_line_height_times_the_natural_height() {
    let natural = natural_height(INTER, SIZE_PX);
    for (text, lines) in [("a", 1.0), ("a\nb", 2.0), ("a\nb\nc", 3.0), ("a\n", 2.0)] {
        for lh in [0.5, 1.0, 2.0, 4.0] {
            let h = f64::from(lay(text, &[INTER], &style(0.0, lh), 10_000.0).height);
            assert!(
                (h - lines * lh * natural).abs() <= 0.01,
                "{text:?} lh={lh}: {h}"
            );
        }
    }
}

#[test]
fn fd_41_a_zero_metrics_font_is_checked_with_an_empty_text_only_when_it_is_the_first() {
    let broken = without_metrics(INTER);
    let broken = broken.as_slice();
    let s = style(0.0, 1.0);
    let run =
        |text: &str, stack: &[&[u8]]| layout_box(text, stack, &s, 96.0, Size::new(500.0, 100.0));
    let bad = Some(TextError::InvalidFont { index: 0 });
    assert_eq!(run("", &[broken]).err(), bad);
    assert_eq!(run("abc", &[broken]).err(), bad);
    assert_eq!(run("", &[broken, INTER]).err(), bad);
    // Behind a healthy font nothing uses it: no error, with text or without.
    assert!(run("abc", &[INTER, broken]).is_ok());
    assert!(run("", &[INTER, broken]).is_ok());
}

#[test]
fn fd_41_f_inter_and_noto_emoji_on_each_line_give_61_74_121_29_and_241_99_at_100_px() {
    // 75 pt at 96 dpi is 100 px. The fonts do not nest: the line takes the larger rise and
    // the larger fall, so the distance is not proportional to the line height.
    let emoji = fonts::NOTO_EMOJI.bytes;
    let text = "a\u{1f600}\na\u{1f600}\na\u{1f600}";
    for (lh, expected) in [(0.5, 61.74), (1.0, 121.29), (2.0, 241.99)] {
        let s = TextStyle {
            size_pt: 75.0,
            line_height: lh,
            ..TextStyle::default()
        };
        let out = layout_box(text, &[INTER, emoji], &s, 96.0, Size::new(10_000.0, 1000.0))
            .expect("layout");
        assert!(
            out.lines[0].clusters.iter().any(|c| c.font == 1),
            "the emoji comes from font 1"
        );
        let b = baselines(&out);
        for pair in b.windows(2) {
            assert!(
                (pair[1] - pair[0] - expected).abs() <= 0.01,
                "lh={lh}: {b:?}"
            );
        }
    }
}
