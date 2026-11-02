//! The box layout and its input and output types.

use std::borrow::Cow;
use std::ops::Range;
use std::sync::Arc;

use gafira_geom::Size;
use parley::fontique::Blob;
use parley::{FontContext, FontFamily, FontFamilyName, LayoutContext, OverflowWrap, StyleProperty};

use crate::TextError;

/// Text style numbers (FD-41).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    /// Font size in points, in `[1, 1000]`.
    pub size_pt: f64,
    /// Letter spacing in thousandths of an em, in `[-100, 500]`.
    pub letter_spacing: f64,
    /// Line height multiplier, in `[0.5, 4.0]`.
    pub line_height: f64,
}

/// The laid out text, without pixels (ARQ-116).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextLayout {
    /// The lines, top to bottom.
    pub lines: Vec<Line>,
    /// True when a line, without its trailing white space, is wider than the box.
    pub overflow: bool,
}

/// One line of text.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Line {
    /// Distance from the top of the layout to the baseline, in px.
    pub baseline: f32,
    /// The sum of the advances of its clusters, in px.
    pub advance: f32,
    /// The grapheme clusters, in visual order (left to right).
    pub clusters: Vec<Cluster>,
}

/// One grapheme cluster of the text.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cluster {
    /// Byte range of the cluster in the text; `range.start` is its index.
    pub range: Range<usize>,
    /// Left edge from the left of the box, in px.
    pub x: f32,
    /// Advance, in px.
    pub advance: f32,
    /// True when the cluster belongs to a right-to-left run.
    pub rtl: bool,
    /// True when a glyph of the cluster has no cover and is the font's `.notdef`.
    pub notdef: bool,
    /// Position in the font stack of the font that shaped the cluster.
    pub font: usize,
    /// Glyph ids of the cluster in the font; none for a forced break.
    pub glyphs: Vec<u32>,
}

/// Lays `text` out in a box of `bounds` px.
///
/// `fonts` is the ordered font stack, as file bytes; each cluster uses the font
/// that covers it, and a cluster no font covers keeps a `.notdef` glyph and is
/// flagged. All numbers are checked before any work: a NaN or an infinity (also
/// a computed one, in the input or the output) gives `Err(NonFinite)` and a style
/// value outside its range gives `Err(OutOfRange)`, each naming the field
/// (ARQ-110, FD-41). `style.letter_spacing` and `style.line_height` are checked
/// here and applied from the next delivery.
///
/// A word wider than the box breaks by cluster, so `overflow` is true only when
/// a single cluster is wider than the box. A final newline starts one more,
/// empty line, as in an editor, and an empty text is one empty line.
pub fn layout_box(
    text: &str,
    fonts: &[&[u8]],
    style: &TextStyle,
    dpi: f64,
    bounds: Size,
) -> Result<TextLayout, TextError> {
    let size_px = check_input(style, dpi, bounds)?;
    let mut fcx = FontContext::new();
    let mut names = Vec::with_capacity(fonts.len());
    let mut blob_ids = Vec::with_capacity(fonts.len());
    for (index, bytes) in fonts.iter().enumerate() {
        let blob = Blob::new(Arc::new(bytes.to_vec()));
        blob_ids.push(blob.id());
        let family = fcx
            .collection
            .register_fonts(blob, None)
            .first()
            .map(|(id, _)| *id)
            .ok_or(TextError::InvalidFont { index })?;
        let name = fcx.collection.family_name(family);
        names.push(FontFamilyName::Named(Cow::Owned(
            name.ok_or(TextError::InvalidFont { index })?.to_string(),
        )));
    }
    let mut lcx: LayoutContext<()> = LayoutContext::new();
    let mut builder = lcx.ranged_builder(&mut fcx, text, 1.0, false);
    builder.push_default(StyleProperty::FontSize(size_px));
    builder.push_default(StyleProperty::FontFamily(FontFamily::List(Cow::Owned(
        names,
    ))));
    builder.push_default(StyleProperty::OverflowWrap(OverflowWrap::Anywhere));
    let mut laid = builder.build(text);
    let width = bounds.width().max(0.0);
    laid.break_all_lines(Some(width.min(f64::from(f32::MAX)) as f32));

    let mut out = TextLayout::default();
    for line in laid.lines() {
        let metrics = line.metrics();
        let mut clusters = Vec::new();
        let mut x = metrics.offset;
        for run in line.runs() {
            let font = blob_ids
                .iter()
                .position(|id| *id == run.font().data.id())
                .unwrap_or(0);
            for c in run.visual_clusters() {
                let glyphs: Vec<u32> = c.glyphs().map(|g| g.id).collect();
                let advance = c.advance();
                clusters.push(Cluster {
                    range: c.text_range(),
                    x,
                    advance,
                    rtl: c.is_rtl(),
                    notdef: glyphs.contains(&0),
                    font,
                    glyphs,
                });
                x += advance;
            }
        }
        let advance: f32 = clusters.iter().map(|c| c.advance).sum();
        let content = advance - metrics.hanging_advance;
        out.overflow |= f64::from(content) > width;
        out.lines.push(Line {
            baseline: metrics.baseline,
            advance,
            clusters,
        });
    }
    check_output(&out)?;
    Ok(out)
}

/// ARQ-110: a non-finite result is an `Err`, never an output.
fn check_output(layout: &TextLayout) -> Result<(), TextError> {
    let all_finite = layout.lines.iter().all(|l| {
        l.baseline.is_finite()
            && l.advance.is_finite()
            && l.clusters
                .iter()
                .all(|c| c.x.is_finite() && c.advance.is_finite())
    });
    if all_finite {
        Ok(())
    } else {
        Err(TextError::NonFinite { field: "output" })
    }
}

const SIZE_PT: (f64, f64) = (1.0, 1000.0);
const LETTER_SPACING: (f64, f64) = (-100.0, 500.0);
const LINE_HEIGHT: (f64, f64) = (0.5, 4.0);

fn finite(field: &'static str, value: f64) -> Result<f64, TextError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(TextError::NonFinite { field })
    }
}

fn in_range(field: &'static str, value: f64, (lo, hi): (f64, f64)) -> Result<(), TextError> {
    if (lo..=hi).contains(&value) {
        Ok(())
    } else {
        Err(TextError::OutOfRange { field })
    }
}

/// Non-finite checks of every input first, then the ranges, then the computed size.
/// Returns the font size in px.
fn check_input(style: &TextStyle, dpi: f64, bounds: Size) -> Result<f32, TextError> {
    let size_pt = finite("style.size_pt", style.size_pt)?;
    let spacing = finite("style.letter_spacing", style.letter_spacing)?;
    let line_height = finite("style.line_height", style.line_height)?;
    finite("dpi", dpi)?;
    finite("bounds.width", bounds.width())?;
    finite("bounds.height", bounds.height())?;
    in_range("style.size_pt", size_pt, SIZE_PT)?;
    in_range("style.letter_spacing", spacing, LETTER_SPACING)?;
    in_range("style.line_height", line_height, LINE_HEIGHT)?;
    let size_px = finite("size_px", size_pt * dpi / 72.0)? as f32;
    if size_px.is_finite() {
        Ok(size_px)
    } else {
        Err(TextError::NonFinite { field: "size_px" })
    }
}
