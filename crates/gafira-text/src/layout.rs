//! The box layout and its input and output types.

use std::borrow::Cow;
use std::ops::Range;
use std::sync::Arc;

use gafira_geom::Size;
use parley::fontique::Blob;
use parley::{
    Alignment, AlignmentOptions, FontContext, FontFamily, FontFamilyName, FontFeatures,
    LayoutContext, LineHeight, OverflowWrap, StyleProperty,
};

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
    /// Full Unicode upper case, with no language (`ß` gives `SS`). The ranges of
    /// the output then index [`TextLayout::text`], not the input.
    pub uppercase: bool,
    /// Where the lines of the box sit across its width.
    pub align: TextAlign,
    /// Gives every cluster an [`Cluster::underline`].
    pub underline: bool,
    /// Gives every cluster a [`Cluster::strikethrough`].
    pub strikethrough: bool,
}

/// A line drawn along a cluster, in px (FD-41, Subrayado y tachado).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Decoration {
    /// Distance from the baseline to the top of the line, positive above it, as the font
    /// tables give it: negative for a usual underline.
    pub offset: f32,
    /// Thickness of the line.
    pub thickness: f32,
}

/// Horizontal alignment of the lines in the box (FD-41).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAlign {
    /// Against the left edge.
    #[default]
    Left,
    /// Centred in the box.
    Center,
    /// Against the right edge.
    Right,
    /// Both edges: the spare room goes between words. The last line of a paragraph, a
    /// line that ends in a forced break and a line of one word are not stretched.
    Justify,
}

impl Default for TextStyle {
    /// 16 pt, no letter spacing, line height 1.0.
    fn default() -> Self {
        Self {
            size_pt: 16.0,
            letter_spacing: 0.0,
            line_height: 1.0,
            uppercase: false,
            align: TextAlign::Left,
            underline: false,
            strikethrough: false,
        }
    }
}

/// The laid out text, without pixels (ARQ-116).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextLayout {
    /// Height of all the lines, in px.
    pub height: f32,
    /// The text that was laid out: the input, in upper case when asked. Cluster ranges
    /// index it.
    pub text: String,
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
    /// The underline of the cluster, when the style asks for one.
    pub underline: Option<Decoration>,
    /// The strikethrough of the cluster, when the style asks for one.
    pub strikethrough: Option<Decoration>,
}

/// Lays `text` out in a box of `bounds` px.
///
/// `fonts` is the ordered font stack, as file bytes; each cluster uses the font
/// that covers it, and a cluster no font covers keeps a `.notdef` glyph and is
/// flagged. All numbers are checked before any work: a NaN or an infinity (also
/// a computed one, in the input or the output) gives `Err(NonFinite)` and a style
/// value outside its range gives `Err(OutOfRange)`, each naming the field
/// (ARQ-110, FD-41). `style.letter_spacing` adds its share after each cluster of a
/// line but the last, and turns off the optional ligatures when it is not 0;
/// `style.line_height` multiplies the natural height of the line, the largest among
/// its fonts. A font with a natural height of 0 or less that shapes a cluster gives
/// `Err(InvalidFont)`.
///
/// An empty font stack gives `Err(NoFont)`, after the number checks and also for
/// an empty text. The white space at the logical end of a line does not count
/// for `overflow`, in any direction.
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
    if fonts.is_empty() {
        return Err(TextError::NoFont);
    }
    let converted;
    let text = if style.uppercase {
        converted = text.to_uppercase();
        converted.as_str()
    } else {
        text
    };
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
    builder.push_default(StyleProperty::LineHeight(LineHeight::MetricsRelative(
        style.line_height as f32,
    )));
    let spacing = style.letter_spacing / 1000.0 * f64::from(size_px);
    if spacing != 0.0 {
        builder.push_default(StyleProperty::LetterSpacing(spacing as f32));
        // CSS: optional ligatures are off with any spacing (parley ignores a tiny one);
        // the required ligatures and joining forms of a script stay.
        builder.push_default(StyleProperty::FontFeatures(FontFeatures::from(
            "\"liga\" 0, \"clig\" 0, \"dlig\" 0, \"hlig\" 0",
        )));
    }
    let mut laid = builder.build(text);
    let width = bounds.width().max(0.0);
    laid.break_all_lines(Some(width.min(f64::from(f32::MAX)) as f32));

    let spacing = spacing as f32;
    let mut lines = read_lines(&laid, &blob_ids, spacing, style, size_px)?;
    // Overflow is read before aligning: justify stretches the spaces of a line up to the box.
    let overflow = lines
        .iter()
        .any(|l| f64::from(content_advance(text, &l.clusters)) > width);
    let alignment = match style.align {
        TextAlign::Left => None,
        TextAlign::Center => Some(Alignment::Center),
        TextAlign::Right => Some(Alignment::Right),
        TextAlign::Justify => Some(Alignment::Justify),
    };
    if let Some(alignment) = alignment {
        laid.align(alignment, AlignmentOptions::default());
        lines = read_lines(&laid, &blob_ids, spacing, style, size_px)?;
    }
    let out = TextLayout {
        height: laid.height(),
        text: text.to_string(),
        lines,
        overflow,
    };
    check_output(&out)?;
    Ok(out)
}

/// The lines of a parley layout as output lines. `spacing` is the letter spacing in px
/// that parley added after every cluster.
fn read_lines(
    laid: &parley::Layout<()>,
    blob_ids: &[u64],
    spacing: f32,
    style: &TextStyle,
    size_px: f32,
) -> Result<Vec<Line>, TextError> {
    let mut lines = Vec::new();
    for line in laid.lines() {
        let metrics = line.metrics();
        let mut clusters = Vec::new();
        let mut hard_breaks = Vec::new();
        for run in line.runs() {
            let font = blob_ids
                .iter()
                .position(|id| *id == run.font().data.id())
                .unwrap_or(0);
            let m = run.font_metrics();
            let natural = m.ascent + m.descent + m.leading;
            if natural.is_nan() || natural <= 0.0 {
                return Err(TextError::InvalidFont { index: font });
            }
            let font_bytes: &[u8] = run.font().data.as_ref();
            let font_index = run.font().index;
            // Parley fills a missing table with 1/18 em; the rule here is 0.05 em.
            let underline = style.underline.then(|| {
                let size = if has_table(font_bytes, font_index, b"post") {
                    m.underline_size
                } else {
                    0.0
                };
                decoration(m.underline_offset, size, size_px)
            });
            let strikethrough = style.strikethrough.then(|| {
                let size = if has_table(font_bytes, font_index, b"OS/2") {
                    m.strikethrough_size
                } else {
                    0.0
                };
                decoration(m.strikethrough_offset, size, size_px)
            });
            for c in run.visual_clusters() {
                let glyphs: Vec<u32> = c.glyphs().map(|g| g.id).collect();
                hard_breaks.push(c.is_hard_line_break());
                clusters.push(Cluster {
                    range: c.text_range(),
                    x: 0.0,
                    advance: c.advance(),
                    rtl: c.is_rtl(),
                    notdef: glyphs.contains(&0),
                    font,
                    glyphs,
                    underline,
                    strikethrough,
                });
            }
        }
        if spacing != 0.0 {
            // The spacing goes after each cluster but the last one of the line. A forced
            // break gets none from parley and is not a cluster for this rule.
            let last = (0..clusters.len())
                .filter(|i| !hard_breaks[*i])
                .max_by_key(|i| clusters[*i].range.start);
            if let Some(i) = last {
                clusters[i].advance -= spacing;
            }
        }
        let mut x = metrics.offset;
        for c in &mut clusters {
            c.x = x;
            x += c.advance;
        }
        let advance: f32 = clusters.iter().map(|c| c.advance).sum();
        lines.push(Line {
            baseline: metrics.baseline,
            advance,
            clusters,
        });
    }
    Ok(lines)
}

/// Whether the font has the table, read from its table directory (index of a collection).
fn has_table(font: &[u8], index: u32, tag: &[u8; 4]) -> bool {
    let be32 = |at: usize| {
        font.get(at..at + 4)
            .map(|b| usize::try_from(u32::from_be_bytes([b[0], b[1], b[2], b[3]])).unwrap_or(0))
    };
    let start = if font.get(..4) == Some(b"ttcf") {
        be32(12 + 4 * index as usize)
    } else {
        Some(0)
    };
    let Some(start) = start else { return false };
    let Some(count) = font
        .get(start + 4..start + 6)
        .map(|b| usize::from(u16::from_be_bytes([b[0], b[1]])))
    else {
        return false;
    };
    (0..count).any(|i| {
        let record = start + 12 + 16 * i;
        font.get(record..record + 4) == Some(tag.as_slice())
    })
}

/// A thickness of 0 or none is 0.05 em (PROPUESTA, FD-41).
fn decoration(offset: f32, thickness: f32, size_px: f32) -> Decoration {
    let thickness = if thickness > 0.0 {
        thickness
    } else {
        0.05 * size_px
    };
    Decoration { offset, thickness }
}

/// The advance of a line without the white space at the end of its logical
/// text. That space hangs past the box, also inside a right-to-left run, where
/// it is not at the visual end of the line.
fn content_advance(text: &str, clusters: &[Cluster]) -> f32 {
    let mut logical: Vec<usize> = (0..clusters.len()).collect();
    logical.sort_by_key(|i| clusters[*i].range.start);
    let is_space = |i: &usize| {
        text[clusters[*i].range.clone()]
            .chars()
            .all(char::is_whitespace)
    };
    let keep = logical.len() - logical.iter().rev().take_while(|i| is_space(i)).count();
    let mut trailing = vec![false; clusters.len()];
    for i in &logical[keep..] {
        trailing[*i] = true;
    }
    clusters
        .iter()
        .zip(&trailing)
        .filter(|(_, t)| !**t)
        .map(|(c, _)| c.advance)
        .sum()
}

/// ARQ-110: a non-finite result is an `Err`, never an output.
pub(crate) fn check_output(layout: &TextLayout) -> Result<(), TextError> {
    let decoration_ok =
        |d: &Option<Decoration>| d.is_none_or(|d| d.offset.is_finite() && d.thickness.is_finite());
    let all_finite = layout.height.is_finite()
        && layout.lines.iter().all(|l| {
            l.baseline.is_finite()
                && l.advance.is_finite()
                && l.clusters.iter().all(|c| {
                    c.x.is_finite()
                        && c.advance.is_finite()
                        && decoration_ok(&c.underline)
                        && decoration_ok(&c.strikethrough)
                })
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
pub(crate) fn check_input(style: &TextStyle, dpi: f64, bounds: Size) -> Result<f32, TextError> {
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
