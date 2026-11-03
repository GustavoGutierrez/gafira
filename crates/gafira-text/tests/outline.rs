//! FD-43 (a): the contours of a text come from its layout and its fonts, so an outline effect
//! (a stroke of 4 px) is drawn around real glyphs and does not move or resize the text.

use gafira_geom::Size;
use gafira_pathops::{BezPath, Cap, Join, PathEl, StrokeStyle};
use gafira_testkit::fonts;
use gafira_text::{
    ClusterOutline, FitBox, FitRun, TextAlign, TextError, TextLayout, TextStyle, fit_box,
    layout_box, outline_text,
};
use skrifa::instance::{LocationRef, Size as FontSize};
use skrifa::{FontRef, GlyphId, MetadataProvider};

const INTER: &[u8] = fonts::INTER.bytes;
const ARABIC: &[u8] = fonts::NOTO_SANS_ARABIC.bytes;
const EMOJI: &[u8] = fonts::NOTO_EMOJI.bytes;
const STACK: [&[u8]; 3] = [INTER, ARABIC, EMOJI];
const DPI: f64 = 96.0;
/// 75 pt at 96 DPI is 100 px.
const SIZE_PT: f64 = 75.0;

fn style(align: TextAlign) -> TextStyle {
    TextStyle {
        size_pt: SIZE_PT,
        align,
        ..TextStyle::default()
    }
}

fn lay(text: &str, width: f64, align: TextAlign) -> TextLayout {
    layout_box(text, &STACK, &style(align), DPI, Size::new(width, 1000.0)).expect("valid input")
}

/// A box with the extremes of a path, sampling its curves.
#[derive(Clone, Copy, Debug)]
struct Bounds {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

fn bounds_of(path: &gafira_pathops::Path) -> Option<Bounds> {
    let mut points: Vec<(f64, f64)> = Vec::new();
    let mut at = (0.0, 0.0);
    for el in path.as_bez_path().elements() {
        match *el {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => {
                at = (p.x, p.y);
                points.push(at);
            }
            PathEl::QuadTo(c, p) => {
                for i in 1..=64 {
                    let t = f64::from(i) / 64.0;
                    let u = 1.0 - t;
                    points.push((
                        u * u * at.0 + 2.0 * u * t * c.x + t * t * p.x,
                        u * u * at.1 + 2.0 * u * t * c.y + t * t * p.y,
                    ));
                }
                at = (p.x, p.y);
            }
            PathEl::CurveTo(c1, c2, p) => {
                for i in 1..=64 {
                    let t = f64::from(i) / 64.0;
                    let u = 1.0 - t;
                    points.push((
                        u * u * u * at.0
                            + 3.0 * u * u * t * c1.x
                            + 3.0 * u * t * t * c2.x
                            + t * t * t * p.x,
                        u * u * u * at.1
                            + 3.0 * u * u * t * c1.y
                            + 3.0 * u * t * t * c2.y
                            + t * t * t * p.y,
                    ));
                }
                at = (p.x, p.y);
            }
            PathEl::ClosePath => {}
        }
    }
    let first = *points.first()?;
    Some(points.iter().fold(
        Bounds {
            x0: first.0,
            y0: first.1,
            x1: first.0,
            y1: first.1,
        },
        |b, p| Bounds {
            x0: b.x0.min(p.0),
            y0: b.y0.min(p.1),
            x1: b.x1.max(p.0),
            y1: b.y1.max(p.1),
        },
    ))
}

/// The box of each contour of a path.
fn contours(path: &gafira_pathops::Path) -> Vec<Bounds> {
    let mut parts: Vec<BezPath> = Vec::new();
    for el in path.as_bez_path().elements() {
        if matches!(el, PathEl::MoveTo(_)) {
            parts.push(BezPath::new());
        }
        parts
            .last_mut()
            .expect("a path starts with a move")
            .push(*el);
    }
    parts
        .into_iter()
        .filter_map(|p| {
            bounds_of(&gafira_pathops::Path::new(
                p,
                gafira_pathops::FillRule::NonZero,
            ))
        })
        .collect()
}

/// The box of a glyph in the font header, scaled to `size_px`, placed with its pen at
/// (`pen_x`, `baseline`) and y down.
fn header_box(font: &[u8], glyph: u32, size_px: f64, pen_x: f64, baseline: f64) -> Option<Bounds> {
    let font = FontRef::new(font).expect("a font");
    let upem = f64::from(
        font.metrics(FontSize::unscaled(), LocationRef::default())
            .units_per_em,
    );
    let b = font
        .glyph_metrics(FontSize::unscaled(), LocationRef::default())
        .bounds(GlyphId::new(glyph))?;
    let k = size_px / upem;
    Some(Bounds {
        x0: pen_x + f64::from(b.x_min) * k,
        x1: pen_x + f64::from(b.x_max) * k,
        y0: baseline - f64::from(b.y_max) * k,
        y1: baseline - f64::from(b.y_min) * k,
    })
}

fn near(a: Bounds, b: Bounds) -> bool {
    [(a.x0, b.x0), (a.y0, b.y0), (a.x1, b.x1), (a.y1, b.y1)]
        .iter()
        .all(|(p, q)| (p - q).abs() <= 0.05)
}

fn pen(layout: &TextLayout, outline: &ClusterOutline) -> (f64, f64) {
    let line = &layout.lines[outline.line];
    (
        f64::from(line.clusters[outline.cluster].x),
        f64::from(line.baseline),
    )
}

#[test]
fn fd_43_a_each_cluster_has_the_outline_of_its_glyph_in_its_font_at_its_place() {
    // Inter, Arabic and the emoji font, over two lines and right aligned, so nothing is at the
    // origin.
    let text = "HEL\n\u{627}\u{1F600}T";
    let layout = lay(text, 400.0, TextAlign::Right);
    let outlines = outline_text(&layout, &STACK).expect("valid input");
    let clusters: usize = layout.lines.iter().map(|l| l.clusters.len()).sum();
    assert_eq!(outlines.len(), clusters, "one outline for each cluster");
    let mut fonts_seen = [false; 3];
    let mut non_empty = 0;
    for o in &outlines {
        let c = &layout.lines[o.line].clusters[o.cluster];
        let (x, baseline) = pen(&layout, o);
        let shown = &text[c.range.clone()];
        if shown.trim().is_empty() {
            assert!(o.path.is_empty(), "{shown:?} has no contour");
            continue;
        }
        fonts_seen[c.font] = true;
        non_empty += 1;
        assert_eq!(c.glyphs.len(), 1, "{shown:?}");
        let expected = header_box(STACK[c.font], c.glyphs[0], 100.0, x, baseline)
            .expect("the glyph has a box in its font");
        let got = bounds_of(&o.path).unwrap_or_else(|| panic!("{shown:?} has an outline"));
        assert!(
            near(got, expected),
            "{shown:?}: {got:?} instead of {expected:?}"
        );
        assert!(o.path.subpath_count() >= 1);
        assert_eq!(o.path.fill_rule(), gafira_pathops::FillRule::NonZero);
    }
    assert_eq!(
        fonts_seen,
        [true, true, true],
        "each of the three fonts drew one"
    );
    assert!(non_empty >= 5);
}

/// What the font itself says about a glyph: the pen calls of `skrifa`, kept as path elements at
/// `size_px`, with y down and the pen at (`x`, `baseline`).
struct Recorder {
    els: Vec<PathEl>,
    origin: (f64, f64),
}

impl Recorder {
    fn p(&self, x: f32, y: f32) -> gafira_pathops::Point {
        gafira_pathops::Point::new(self.origin.0 + f64::from(x), self.origin.1 - f64::from(y))
    }
}

impl skrifa::outline::OutlinePen for Recorder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.els.push(PathEl::MoveTo(self.p(x, y)));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.els.push(PathEl::LineTo(self.p(x, y)));
    }
    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.els
            .push(PathEl::QuadTo(self.p(cx0, cy0), self.p(x, y)));
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.els.push(PathEl::CurveTo(
            self.p(cx0, cy0),
            self.p(cx1, cy1),
            self.p(x, y),
        ));
    }
    fn close(&mut self) {
        self.els.push(PathEl::ClosePath);
    }
}

#[test]
fn fd_43_a_a_round_glyph_keeps_the_curves_of_its_font() {
    // `O` in Inter is made of curves: a path of straight segments between its on-curve points
    // is a different shape. The elements must be the ones the font draws, kind and point.
    let layout = lay("O", 400.0, TextAlign::Left);
    let outlines = outline_text(&layout, &STACK).expect("valid input");
    let c = &layout.lines[0].clusters[0];
    let font = FontRef::new(INTER).expect("a font");
    let mut rec = Recorder {
        els: Vec::new(),
        origin: (
            f64::from(c.x) + f64::from(c.glyph_offsets[0].0),
            f64::from(layout.lines[0].baseline) + f64::from(c.glyph_offsets[0].1),
        ),
    };
    font.outline_glyphs()
        .get(GlyphId::new(c.glyphs[0]))
        .expect("O has an outline")
        .draw(
            skrifa::outline::DrawSettings::unhinted(
                FontSize::new(c.size_px),
                LocationRef::default(),
            ),
            &mut rec,
        )
        .expect("drawn");
    let got = outlines[0].path.as_bez_path().elements();
    assert!(
        rec.els.iter().any(|e| matches!(e, PathEl::QuadTo(..))),
        "the fixture needs curves"
    );
    assert_eq!(got.len(), rec.els.len());
    for (g, w) in got.iter().zip(&rec.els) {
        match (g, w) {
            (PathEl::MoveTo(a), PathEl::MoveTo(b)) | (PathEl::LineTo(a), PathEl::LineTo(b)) => {
                assert!((*a - *b).hypot() < 1e-9, "{g:?} instead of {w:?}");
            }
            (PathEl::QuadTo(a, b), PathEl::QuadTo(c, d)) => {
                assert!(
                    (*a - *c).hypot() < 1e-9 && (*b - *d).hypot() < 1e-9,
                    "{g:?} instead of {w:?}"
                );
            }
            (PathEl::CurveTo(a, b, c), PathEl::CurveTo(d, e, f)) => {
                assert!(
                    (*a - *d).hypot() < 1e-9
                        && (*b - *e).hypot() < 1e-9
                        && (*c - *f).hypot() < 1e-9,
                    "{g:?} instead of {w:?}"
                );
            }
            (PathEl::ClosePath, PathEl::ClosePath) => {}
            _ => panic!("{g:?} instead of {w:?}"),
        }
    }
}

/// The ink of each cluster of the first line, left to right, with letter spacing `t`.
fn ink_of_line(text: &str, t: f64, align: TextAlign) -> Vec<Bounds> {
    let style = TextStyle {
        letter_spacing: t,
        ..style(align)
    };
    let layout = layout_box(text, &STACK, &style, DPI, Size::new(600.0, 1000.0)).expect("valid");
    outline_text(&layout, &STACK)
        .expect("valid input")
        .iter()
        .filter(|o| o.line == 0)
        .map(|o| bounds_of(&o.path).expect("every letter has ink"))
        .collect()
}

#[test]
fn fd_43_a_letter_spacing_opens_every_gap_between_inked_clusters_in_both_directions() {
    // FD-41 Interletrado: t = 100 at 100 px is 10 px after each cluster but the last of the line.
    // Latin is the control; Arabic is right to left, where "after" is on the left of the glyph.
    for text in ["HIHIH", "\u{62F}\u{627}\u{631}\u{627}\u{632}"] {
        let plain = ink_of_line(text, 0.0, TextAlign::Left);
        let spaced = ink_of_line(text, 100.0, TextAlign::Left);
        assert_eq!(plain.len(), 5, "{text:?}");
        for i in 1..plain.len() {
            let gain = (spaced[i].x0 - spaced[i - 1].x1) - (plain[i].x0 - plain[i - 1].x1);
            assert!(
                (gain - 10.0).abs() < 0.05,
                "{text:?}: gap {i} grew by {gain} instead of 10"
            );
        }
        // The first cluster does not move and the ink ends 4 gaps later.
        assert!((spaced[0].x0 - plain[0].x0).abs() < 0.05, "{text:?}");
    }
}

#[test]
fn fd_43_a_the_counters_of_a_letter_are_contours_of_their_own() {
    let layout = lay("HAB", 400.0, TextAlign::Left);
    let counts: Vec<usize> = outline_text(&layout, &STACK)
        .expect("valid input")
        .iter()
        .map(|o| o.path.subpath_count())
        .collect();
    // H is one contour, A has a counter, B has two.
    assert_eq!(counts, vec![1, 2, 3]);
}

#[test]
fn fd_43_a_a_4_px_stroke_grows_the_ink_by_2_px_and_leaves_the_layout_alone() {
    let text = "HIT\nLEH";
    let layout = lay(text, 300.0, TextAlign::Center);
    let before = layout.clone();
    let outlines = outline_text(&layout, &STACK).expect("valid input");
    let stroke = StrokeStyle {
        width: 4.0,
        cap: Cap::Butt,
        join: Join::Miter,
    };
    for o in outlines.iter().filter(|o| !o.path.is_empty()) {
        let fill = bounds_of(&o.path).expect("a letter");
        let stroked = o.path.stroke_outline(&stroke, 0.01).expect("a stroke");
        let around = bounds_of(&stroked).expect("a stroked letter");
        // Right-angled letters: the stroke is 2 px outside the fill on every side.
        let grown = Bounds {
            x0: fill.x0 - 2.0,
            y0: fill.y0 - 2.0,
            x1: fill.x1 + 2.0,
            y1: fill.y1 + 2.0,
        };
        assert!(near(around, grown), "{around:?} instead of {grown:?}");
    }
    assert!(outlines.iter().filter(|o| !o.path.is_empty()).count() >= 5);
    // The text is where it was: the stroke is drawn from the layout, not into it.
    assert_eq!(layout, before);
    assert_eq!(layout, lay(text, 300.0, TextAlign::Center));
    let again = outline_text(&layout, &STACK).expect("valid input");
    assert_eq!(again, outlines);
}

#[test]
fn fd_43_a_a_cluster_with_a_mark_draws_the_glyphs_where_the_font_puts_them() {
    // Beh and fatha: one cluster, two glyphs, the mark above the letter.
    let text = "\u{628}\u{64e}";
    let layout = lay(text, 400.0, TextAlign::Left);
    let c = &layout.lines[0].clusters[0];
    assert!(c.glyphs.len() >= 2, "a letter and its mark: {:?}", c.glyphs);
    assert_eq!(c.glyph_offsets.len(), c.glyphs.len());
    let baseline = f64::from(layout.lines[0].baseline);
    let out = &outline_text(&layout, &STACK).expect("valid input")[0];
    let got = bounds_of(&out.path).expect("an outline");
    // Each glyph is placed at its own offset from the cluster: every side of its box in the font
    // header is the side of one of the contours drawn.
    let parts = contours(&out.path);
    for (glyph, (dx, dy)) in c.glyphs.iter().zip(&c.glyph_offsets) {
        let Some(want) = header_box(
            ARABIC,
            *glyph,
            100.0,
            f64::from(c.x) + f64::from(*dx),
            baseline + f64::from(*dy),
        ) else {
            continue;
        };
        let has = |side: &dyn Fn(&Bounds) -> f64, value: f64| {
            parts.iter().any(|b| (side(b) - value).abs() <= 0.05)
        };
        assert!(
            has(&|b| b.x0, want.x0),
            "glyph {glyph}: left {want:?} in {parts:?}"
        );
        assert!(has(&|b| b.x1, want.x1), "glyph {glyph}: right");
        assert!(has(&|b| b.y0, want.y0), "glyph {glyph}: top");
        assert!(has(&|b| b.y1, want.y1), "glyph {glyph}: bottom");
    }
    assert!(got.x1 > got.x0 && got.y1 > got.y0);
    // And they are where a reader sees them, whatever the offsets say: the topmost contour is
    // the fatha, a hair above the body of the letter and over it, not beside it.
    let mut by_top = parts.clone();
    by_top.sort_by(|a, b| a.y0.total_cmp(&b.y0));
    let (mark, body) = (by_top[0], by_top[1]);
    let gap = body.y0 - mark.y1;
    assert!(
        gap > 0.0 && gap < 15.0,
        "the fatha floats {gap} px above the letter"
    );
    let centre = (mark.x0 + mark.x1) / 2.0;
    // The two contours of the body, under the fatha and above the dot.
    let letter = by_top[1..3]
        .iter()
        .fold((f64::MAX, f64::MIN), |(l, r), b| (l.min(b.x0), r.max(b.x1)));
    assert!(
        (centre - (letter.0 + letter.1) / 2.0).abs() < 10.0,
        "centre {centre} over {letter:?}"
    );
    // The mark sits above the letter, so the cluster rises above the letter alone.
    let letter = lay("\u{628}", 400.0, TextAlign::Left);
    let letter_out = &outline_text(&letter, &STACK).expect("valid input")[0];
    let letter_top = bounds_of(&letter_out.path).expect("outline").y0;
    assert!(got.y0 < letter_top - 5.0, "{} above {letter_top}", got.y0);
}

#[test]
fn fd_43_a_every_size_of_a_layout_with_runs_is_drawn_at_its_size() {
    let runs = [
        FitRun {
            text: "H",
            size_pt: 10.0,
        },
        FitRun {
            text: "H",
            size_pt: 20.0,
        },
    ];
    let out = fit_box(
        &runs,
        &STACK,
        &style(TextAlign::Left),
        DPI,
        Size::new(1e6, 1e6),
        &FitBox {
            min_size: 30.0,
            max_size: 30.0,
        },
    )
    .expect("valid input");
    let outlines = outline_text(&out.layout, &STACK).expect("valid input");
    let widths: Vec<f64> = outlines
        .iter()
        .map(|o| {
            let b = bounds_of(&o.path).expect("an H");
            b.x1 - b.x0
        })
        .collect();
    assert_eq!(widths.len(), 2);
    assert!((widths[1] - 2.0 * widths[0]).abs() <= 0.05, "{widths:?}");
}

#[test]
fn fd_43_a_a_glyph_without_cover_draws_the_notdef_of_its_font() {
    let layout = lay("\u{4e00}", 400.0, TextAlign::Left);
    let c = &layout.lines[0].clusters[0];
    assert!(c.notdef && c.glyphs == vec![0]);
    let out = &outline_text(&layout, &STACK).expect("valid input")[0];
    let (x, baseline) = pen(&layout, out);
    // The test subsets leave glyph 0 without a contour, and the font header agrees.
    match header_box(STACK[c.font], 0, 100.0, x, baseline) {
        Some(want) => assert!(near(bounds_of(&out.path).expect("a box"), want)),
        None => assert!(out.path.is_empty()),
    }
}

#[test]
fn fd_43_a_a_font_that_cannot_be_read_is_an_error() {
    let layout = lay("H", 400.0, TextAlign::Left);
    let call = |fonts: &[&[u8]]| outline_text(&layout, fonts).err();
    assert_eq!(
        call(&[&[0_u8; 16]]),
        Some(TextError::InvalidFont { index: 0 })
    );
    // The cluster uses the font at position 0; one missing is an error too.
    assert_eq!(call(&[]), Some(TextError::InvalidFont { index: 0 }));
    // A text with no glyph reads no font.
    let empty = lay("", 400.0, TextAlign::Left);
    assert_eq!(outline_text(&empty, &[]).map(|o| o.len()), Ok(0));
}

#[test]
fn fd_43_a_the_same_layout_gives_the_same_outlines() {
    let layout = lay("Same, same", 300.0, TextAlign::Justify);
    let first = outline_text(&layout, &STACK).expect("valid input");
    let second = outline_text(&layout, &STACK).expect("valid input");
    assert_eq!(format!("{first:?}"), format!("{second:?}"));
}

/// The offset and length of a table in a font file.
fn table(font: &[u8], tag: &[u8; 4]) -> (usize, usize) {
    let count = usize::from(u16::from_be_bytes([font[4], font[5]]));
    (0..count)
        .map(|i| 12 + 16 * i)
        .find(|record| &font[*record..*record + 4] == tag)
        .map(|record| {
            let be32 = |at: usize| {
                usize::try_from(u32::from_be_bytes([
                    font[at],
                    font[at + 1],
                    font[at + 2],
                    font[at + 3],
                ]))
                .expect("fits")
            };
            (be32(record + 8), be32(record + 12))
        })
        .expect("the table")
}

#[test]
fn fd_43_a_a_glyph_that_cannot_be_drawn_is_an_error() {
    // The font reads, but the contour of "H" says it has 65 536 points and the table ends first.
    let layout = lay("H", 400.0, TextAlign::Left);
    let glyph = layout.lines[0].clusters[0].glyphs[0] as usize;
    let be16 = |font: &[u8], at: usize| usize::from(u16::from_be_bytes([font[at], font[at + 1]]));
    let (head, _) = table(INTER, b"head");
    let (loca, _) = table(INTER, b"loca");
    let (glyf, glyf_len) = table(INTER, b"glyf");
    let long = be16(INTER, head + 50) == 1;
    let offset = |g: usize| {
        if long {
            let at = loca + 4 * g;
            usize::try_from(u32::from_be_bytes([
                INTER[at],
                INTER[at + 1],
                INTER[at + 2],
                INTER[at + 3],
            ]))
            .expect("fits")
        } else {
            2 * be16(INTER, loca + 2 * g)
        }
    };
    let start = glyf + offset(glyph);
    let mut broken = INTER.to_vec();
    broken[start..start + 2].copy_from_slice(&1_i16.to_be_bytes());
    broken[start + 10..start + 12].copy_from_slice(&0xffff_u16.to_be_bytes());
    // Cut the table after the glyph header, so nothing follows the end point.
    broken[start + 12..glyf + glyf_len].fill(0);
    let stack: [&[u8]; 1] = [&broken];
    assert_eq!(
        outline_text(&layout, &stack).err(),
        Some(TextError::InvalidFont { index: 0 })
    );
}

#[test]
fn arq_110_an_outline_that_is_not_finite_is_an_error() {
    let mut layout = lay("H", 400.0, TextAlign::Left);
    layout.lines[0].clusters[0].x = f32::NAN;
    assert_eq!(
        outline_text(&layout, &STACK).err(),
        Some(TextError::NonFinite { field: "output" })
    );
    let mut layout = lay("H", 400.0, TextAlign::Left);
    layout.lines[0].baseline = f32::INFINITY;
    assert_eq!(
        outline_text(&layout, &STACK).err(),
        Some(TextError::NonFinite { field: "output" })
    );
}
