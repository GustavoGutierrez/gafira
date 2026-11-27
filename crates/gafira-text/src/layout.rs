//! The box layout and its input and output types.

use std::borrow::Cow;
use std::ops::Range;
use std::sync::Arc;

use gafira_geom::Size;
use parley::fontique::Blob;
use parley::{
    FontContext, FontFamily, FontFamilyName, FontFeatures, LayoutContext, LineHeight, OverflowWrap,
    StyleProperty,
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
    /// Full Unicode upper case, with no language (`ß` gives `SS`). Cluster ranges
    /// still index the input; the clusters of one character share its range.
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
    /// line that ends in a forced break and a line of one word are not stretched; they
    /// start where their bidi paragraph starts.
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
    /// The lines, top to bottom.
    pub lines: Vec<Line>,
    /// True when a cluster, not counting the white space at the end of its line, is wider than
    /// the box.
    pub overflow: bool,
}

/// One line of text.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Line {
    /// Distance from the top of the layout to the baseline, in px.
    pub baseline: f32,
    /// The sum of the advances of its clusters, in px.
    pub advance: f32,
    /// Height of the line, in px: the baseline distance of lines of one font, times the
    /// line height. The heights of all the lines add up to [`TextLayout::height`].
    pub height: f32,
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
    /// True when the cluster is white space at the logical end of its line, of any direction:
    /// the white space that hangs for the letter spacing, the overflow and the curve (FD-41,
    /// FD-44).
    pub trailing: bool,
    /// Position in the font stack of the font that shaped the cluster.
    pub font: usize,
    /// Glyph ids of the cluster in the font; none for a forced break.
    pub glyphs: Vec<u32>,
    /// Where each glyph of `glyphs` starts: the distance from the left edge of the cluster and
    /// from the baseline (positive is down), in px.
    pub glyph_offsets: Vec<(f32, f32)>,
    /// Font size of the cluster, in px.
    pub size_px: f32,
    /// The underline of the cluster, when the style asks for one.
    pub underline: Option<Decoration>,
    /// The strikethrough of the cluster, when the style asks for one.
    pub strikethrough: Option<Decoration>,
}

/// Relative slack of the fit. Parley sums the advances of a line in its own order and the
/// output sums them again without the last spacing, so a box equal to the advance the output
/// reports can be a few f32 ulps short for parley. 4e-6 is 0.0004 px at 100 px and above the
/// 7e-7 measured. It is also the limit of the overflow: a line within it neither breaks nor
/// overflows. The overflow reads the widest cluster, not the sum of the line, and a justified
/// line parley keeps a hair over the box still reaches the edge, so no width of box breaks
/// the 0.01 px of the spec.
const FIT_TOLERANCE: f64 = 4e-6;

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
///
/// Every forced break closes a bidi paragraph (FD-41, Párrafos bidi): U+000A, U+000D, the pair
/// U+000D U+000A as one, U+2028 and U+2029. Each paragraph takes its direction from its first
/// strong character, and has it in all its lines, for the hang and for `Justify`.
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
    let mut stack = Stack::new(fonts)?;
    layout_with(&mut stack, text, style, size_px, &[], bounds.width(), &[])
}

/// `layout_box` in the face `(bold, italic)` of the family of the first font
/// (spec 062): the stack holds the faces of the family and the layout picks
/// the one that matches.
pub fn layout_box_face(
    text: &str,
    fonts: &[&[u8]],
    style: &TextStyle,
    face: (bool, bool),
    dpi: f64,
    bounds: Size,
) -> Result<TextLayout, TextError> {
    let size_px = check_input(style, dpi, bounds)?;
    if fonts.is_empty() {
        return Err(TextError::NoFont);
    }
    let mut stack = Stack::new(fonts)?;
    stack.face = face;
    layout_with(&mut stack, text, style, size_px, &[], bounds.width(), &[])
}

/// A part of the text in its own face and size (spec 106): its byte range,
/// its size in px, the index in the stack of the first font of its family,
/// and its weight and style.
#[derive(Clone, Debug, PartialEq)]
pub struct FaceRun {
    pub range: Range<usize>,
    pub size_px: f32,
    pub lead: usize,
    pub bold: bool,
    pub italic: bool,
}

/// [`layout_box`] with parts in their own face and size (spec 106). The
/// text outside every run is in the style of the call and the first font of
/// the stack; a run names the font its family starts with, the others of
/// the stack covering what it lacks. With no run it is [`layout_box_face`]
/// with no bold or italic.
pub fn layout_box_runs(
    text: &str,
    fonts: &[&[u8]],
    style: &TextStyle,
    dpi: f64,
    bounds: Size,
    runs: &[FaceRun],
) -> Result<TextLayout, TextError> {
    let size_px = check_input(style, dpi, bounds)?;
    if fonts.is_empty() {
        return Err(TextError::NoFont);
    }
    if runs
        .iter()
        .any(|r| !r.size_px.is_finite() || r.size_px <= 0.0 || r.lead >= fonts.len())
    {
        return Err(TextError::OutOfRange { field: "run" });
    }
    let mut stack = Stack::new(fonts)?;
    let mut sizes: Vec<SizeRun> = runs
        .iter()
        .map(|r| SizeRun {
            range: r.range.clone(),
            size_px: r.size_px,
        })
        .collect();
    // The text outside the runs keeps the size of the call: the size the
    // layout is given is the largest, for the break (see `layout_with`).
    let mut covered: Vec<Range<usize>> = runs.iter().map(|r| r.range.clone()).collect();
    covered.sort_by_key(|r| r.start);
    let mut at = 0;
    for part in covered {
        if part.start > at {
            sizes.push(SizeRun {
                range: at..part.start,
                size_px,
            });
        }
        at = at.max(part.end);
    }
    if at < text.len() {
        sizes.push(SizeRun {
            range: at..text.len(),
            size_px,
        });
    }
    let largest = runs.iter().map(|r| r.size_px).fold(size_px, f32::max);
    layout_with(
        &mut stack,
        text,
        style,
        largest,
        &sizes,
        bounds.width(),
        runs,
    )
}

/// Fonts ready to lay many texts out (spec 081): registering them is the
/// costly part of [`layout_box`], and it is done once. The result is the
/// same.
pub struct FontStack(Stack);

impl FontStack {
    /// Registers the fonts, in order; `InvalidFont` names the first that
    /// cannot be read.
    pub fn new(fonts: &[&[u8]]) -> Result<Self, TextError> {
        if fonts.is_empty() {
            return Err(TextError::NoFont);
        }
        Stack::new(fonts).map(Self)
    }

    /// [`layout_box_face`] with these fonts.
    pub fn layout_box_face(
        &mut self,
        text: &str,
        style: &TextStyle,
        face: (bool, bool),
        dpi: f64,
        bounds: Size,
    ) -> Result<TextLayout, TextError> {
        let size_px = check_input(style, dpi, bounds)?;
        self.0.face = face;
        layout_with(&mut self.0, text, style, size_px, &[], bounds.width(), &[])
    }
}

/// The font stack of a call: registered once, so a search that lays the same text out many
/// times does not register the fonts again.
pub(crate) struct Stack {
    fcx: FontContext,
    lcx: LayoutContext<()>,
    names: Vec<FontFamilyName<'static>>,
    blob_ids: Vec<u64>,
    /// The face asked for, bold then italic (spec 062).
    face: (bool, bool),
}

impl Stack {
    /// Registers the fonts, in order; `InvalidFont` names the first that cannot be read.
    pub(crate) fn new(fonts: &[&[u8]]) -> Result<Self, TextError> {
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
        Ok(Self {
            fcx,
            lcx: LayoutContext::new(),
            names,
            blob_ids,
            face: (false, false),
        })
    }
}

/// A size for a byte range of the input text. The text outside every run has the size of the
/// call.
#[derive(Clone, Debug)]
pub(crate) struct SizeRun {
    pub(crate) range: Range<usize>,
    pub(crate) size_px: f32,
}

/// The layout of `layout_box` over a prepared stack, with the numbers already checked. The
/// letter spacing is in em, so each run has its own in px; the break uses the one of `size_px`,
/// because parley takes one width for the whole text, so `size_px` must be the largest size.
pub(crate) fn layout_with(
    stack: &mut Stack,
    text: &str,
    style: &TextStyle,
    size_px: f32,
    runs: &[SizeRun],
    width: f64,
    faces: &[FaceRun],
) -> Result<TextLayout, TextError> {
    let Stack {
        fcx,
        lcx,
        names,
        blob_ids,
        face,
    } = stack;
    let (bold, italic) = *face;
    let input = text;
    let converted;
    let mut source_map = None;
    let text = if style.uppercase {
        let (upper, map) = uppercase_with_map(input);
        converted = upper;
        source_map = Some(map);
        converted.as_str()
    } else {
        text
    };
    let runs: Vec<SizeRun> = runs
        .iter()
        .map(|run| SizeRun {
            range: source_map.as_ref().map_or(run.range.clone(), |map| {
                map.converted(run.range.start)..map.converted(run.range.end)
            }),
            size_px: run.size_px,
        })
        .collect();
    let spacing_em = style.letter_spacing / 1000.0;
    let spacing = spacing_em * f64::from(size_px);
    let width = width.max(0.0);
    // Parley adds the spacing after every cluster, the last one of a line too; the output
    // does not keep that one, so the box is that much wider for the fit.
    // One fit rule for the break and the overflow: a line fits a box of `limit` px.
    let limit = width * (1.0 + FIT_TOLERANCE);
    // The last cluster of a line may be of any run, so the box is widened by the smallest
    // spacing it can have there (the one of the smallest run, or of the largest when the
    // spacing is negative): the line then never passes the box (FD-46).
    let sizes = runs.iter().map(|r| r.size_px).chain([size_px]);
    let edge = if spacing_em < 0.0 {
        sizes.fold(f32::MIN, f32::max)
    } else {
        sizes.fold(f32::MAX, f32::min)
    };
    let fit = (limit + spacing_em * f64::from(edge)).clamp(0.0, f64::from(f32::MAX)) as f32;
    let mut lines = Vec::new();
    let mut overflow = false;
    let mut top = 0.0_f64;
    for range in bidi_paragraphs(text) {
        let paragraph = &text[range.clone()];
        let mut builder = lcx.ranged_builder(fcx, paragraph, 1.0, false);
        builder.push_default(StyleProperty::FontSize(size_px));
        builder.push_default(StyleProperty::FontFamily(FontFamily::List(Cow::Borrowed(
            names.as_slice(),
        ))));
        builder.push_default(StyleProperty::OverflowWrap(OverflowWrap::Anywhere));
        // The face of the family: the stack holds its faces and the
        // layout takes the one with this weight and style (spec 062).
        if bold {
            builder.push_default(StyleProperty::FontWeight(parley::FontWeight::BOLD));
        }
        if italic {
            builder.push_default(StyleProperty::FontStyle(parley::FontStyle::Italic));
        }
        builder.push_default(StyleProperty::LineHeight(LineHeight::MetricsRelative(
            style.line_height as f32,
        )));
        let mut here = Vec::new();
        for run in &runs {
            let (from, to) = (
                run.range.start.max(range.start),
                run.range.end.min(range.end),
            );
            if from < to {
                here.push((from - range.start..to - range.start, run.size_px));
            }
        }
        // The parts in their own face (spec 106): their family first, the
        // others of the stack after it for what it lacks.
        for run in faces {
            let (from, to) = (
                run.range.start.max(range.start),
                run.range.end.min(range.end),
            );
            if from >= to {
                continue;
            }
            let part = from - range.start..to - range.start;
            let mut list: Vec<FontFamilyName<'static>> = Vec::with_capacity(names.len());
            list.extend(names[run.lead..].iter().cloned());
            list.extend(names[..run.lead].iter().cloned());
            builder.push(
                StyleProperty::FontFamily(FontFamily::List(Cow::Owned(list))),
                part.clone(),
            );
            let weight = if run.bold {
                parley::FontWeight::BOLD
            } else {
                parley::FontWeight::NORMAL
            };
            builder.push(StyleProperty::FontWeight(weight), part.clone());
            let slant = if run.italic {
                parley::FontStyle::Italic
            } else {
                parley::FontStyle::Normal
            };
            builder.push(StyleProperty::FontStyle(slant), part.clone());
        }
        for (part, px) in &here {
            builder.push(StyleProperty::FontSize(*px), part.clone());
            if spacing_em != 0.0 {
                builder.push(
                    StyleProperty::LetterSpacing((spacing_em * f64::from(*px)) as f32),
                    part.clone(),
                );
            }
        }
        // The spacing in px of the cluster that starts at a byte of this paragraph.
        let spacing_at = |at: usize| -> f32 {
            let px = here
                .iter()
                .find(|(part, _)| part.contains(&at))
                .map_or(size_px, |(_, px)| *px);
            (spacing_em * f64::from(px)) as f32
        };
        let spaced = spacing != 0.0
            || runs
                .iter()
                .any(|r| spacing_em * f64::from(r.size_px) != 0.0);
        if spacing != 0.0 {
            builder.push_default(StyleProperty::LetterSpacing(spacing as f32));
        }
        if spaced {
            // CSS: optional ligatures are off with any spacing (parley ignores a tiny one);
            // the required ligatures and joining forms of a script stay.
            builder.push_default(StyleProperty::FontFeatures(FontFeatures::from(
                "\"liga\" 0, \"clig\" 0, \"dlig\" 0, \"hlig\" 0",
            )));
        }
        let mut laid = builder.build(paragraph);
        laid.break_all_lines(Some(fit));

        // Parley ends a text with a forced break in one more, empty line; here that line is
        // the next paragraph, which `bidi_paragraphs` always gives.
        let kept = laid.len() - usize::from(paragraph.ends_with(is_forced_break));
        let mut read = read_lines(
            &laid,
            kept,
            blob_ids,
            paragraph,
            spaced.then_some(&spacing_at as &dyn Fn(usize) -> f32),
            style,
        )?;
        overflow |= read
            .iter()
            .any(|l| widest_content(paragraph, &l.clusters) > limit);
        // Parley resolves the first strong character of each paragraph by itself (P2, P3).
        let rtl = laid.is_rtl();
        let last = read.len().saturating_sub(1);
        for (i, line) in read.iter_mut().enumerate() {
            // A forced break closes its paragraph, so only the last line can end in one.
            let stretch = i != last;
            align_line(line, paragraph, style.align, rtl, width, stretch);
            line.baseline = (top + f64::from(line.baseline)) as f32;
            for c in &mut line.clusters {
                c.range = c.range.start + range.start..c.range.end + range.start;
            }
        }
        lines.append(&mut read);
        top += laid
            .lines()
            .take(kept)
            .map(|l| f64::from(l.metrics().line_height))
            .sum::<f64>();
    }
    // Ranges index the input text: after upper case, the clusters that come from one
    // character share its range.
    if let Some(map) = &source_map {
        for line in &mut lines {
            for c in &mut line.clusters {
                c.range = map.input_range(&c.range);
            }
        }
    }
    let out = TextLayout {
        height: top as f32,
        lines,
        overflow,
    };
    check_output(&out)?;
    Ok(out)
}

/// The forced breaks of FD-41. Not U+000B, U+000C, U+0085 or U+001C to U+001E, which UAX #9
/// or parley treat in other ways. A pair U+000D U+000A is one break.
fn is_forced_break(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// The byte ranges of the bidi paragraphs of the text (FD-41, Párrafos bidi). Each forced break
/// closes a paragraph and belongs to it; the text after the last one is a paragraph too, also
/// when it is empty, so the ranges cover the text and there is always at least one. Parley
/// runs its bidi once per layout, so each paragraph needs a layout of its own.
fn bidi_paragraphs(text: &str) -> Vec<Range<usize>> {
    let mut paragraphs = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((_, c)) = chars.next() {
        if is_forced_break(c) {
            if c == '\r' {
                chars.next_if(|(_, next)| *next == '\n');
            }
            let end = chars.peek().map_or(text.len(), |(next, _)| *next);
            paragraphs.push(start..end);
            start = end;
        }
    }
    paragraphs.push(start..text.len());
    paragraphs
}

/// The first `kept` lines of the layout of a paragraph as output lines, all starting at x = 0.
/// `spacing`, when there is any, gives the letter spacing in px that parley added after the
/// cluster that starts at a byte. Baselines and ranges are the ones of the paragraph.
fn read_lines(
    laid: &parley::Layout<()>,
    kept: usize,
    blob_ids: &[u64],
    text: &str,
    spacing: Option<&dyn Fn(usize) -> f32>,
    style: &TextStyle,
) -> Result<Vec<Line>, TextError> {
    let mut lines = Vec::new();
    for line in laid.lines().take(kept) {
        let metrics = line.metrics();
        let mut clusters = Vec::new();
        let mut hard_breaks = Vec::new();
        for run in line.runs() {
            let font = blob_ids
                .iter()
                .position(|id| *id == run.font().data.id())
                .unwrap_or(0);
            let m = run.font_metrics();
            let size_px = run.font_size();
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
                let mut pen = 0.0_f32;
                let mut offsets = Vec::with_capacity(glyphs.len());
                for g in c.glyphs() {
                    offsets.push((pen + g.x, g.y));
                    pen += g.advance;
                }
                hard_breaks.push(c.is_hard_line_break());
                clusters.push(Cluster {
                    range: c.text_range(),
                    x: 0.0,
                    advance: c.advance(),
                    rtl: c.is_rtl(),
                    notdef: glyphs.contains(&0),
                    trailing: false,
                    font,
                    glyphs,
                    glyph_offsets: offsets,
                    size_px,
                    underline,
                    strikethrough,
                });
            }
        }
        let hanging = hanging_mask(text, &clusters);
        for (c, h) in clusters.iter_mut().zip(&hanging) {
            c.trailing = *h;
        }
        if let Some(spacing) = spacing {
            // The spacing goes after each cluster but the last one of the line. The white
            // space that hangs at the end is neither that cluster nor does it carry the
            // spacing; a forced break gets none from parley.
            let last = (0..clusters.len())
                .filter(|i| !hanging[*i])
                .max_by_key(|i| clusters[*i].range.start);
            for (i, c) in clusters.iter_mut().enumerate() {
                let gap = spacing(c.range.start);
                if (last == Some(i) || hanging[i]) && !hard_breaks[i] {
                    c.advance -= gap;
                } else if c.rtl && !hard_breaks[i] {
                    // Parley puts the spacing on the right of an rtl glyph, at the left edge
                    // of the advance; "after" the cluster is its left, so the glyphs move.
                    for offset in &mut c.glyph_offsets {
                        offset.0 += gap;
                    }
                }
            }
        }
        let mut x = 0.0;
        for c in &mut clusters {
            c.x = x;
            x += c.advance;
        }
        let advance = clusters.iter().map(|c| f64::from(c.advance)).sum::<f64>() as f32;
        lines.push(Line {
            baseline: metrics.baseline,
            advance,
            height: metrics.line_height,
            clusters,
        });
    }
    Ok(lines)
}

/// Where each piece of the converted text came from in the input.
struct SourceMap {
    /// Start in the converted text, and the range of the input character it came from.
    pieces: Vec<(usize, Range<usize>)>,
    /// Length of the converted text.
    len: usize,
}

impl SourceMap {
    fn piece(&self, converted_byte: usize) -> &Range<usize> {
        let i = self
            .pieces
            .partition_point(|(start, _)| *start <= converted_byte);
        &self.pieces[i.saturating_sub(1)].1
    }

    /// The byte of the converted text where the input character that starts at `input_byte`
    /// starts; the end of the text for the end of the input.
    fn converted(&self, input_byte: usize) -> usize {
        let i = self.pieces.partition_point(|(_, r)| r.start < input_byte);
        self.pieces.get(i).map_or(self.len, |(start, _)| *start)
    }

    /// From the first to the last input character the converted range touches.
    fn input_range(&self, converted: &Range<usize>) -> Range<usize> {
        let last = converted.end.max(converted.start + 1) - 1;
        self.piece(converted.start).start..self.piece(last).end
    }
}

/// Full Unicode upper case with no language, and the map back to the input.
fn uppercase_with_map(input: &str) -> (String, SourceMap) {
    let mut upper = String::with_capacity(input.len());
    let mut pieces = Vec::new();
    for (start, c) in input.char_indices() {
        pieces.push((upper.len(), start..start + c.len_utf8()));
        upper.extend(c.to_uppercase());
    }
    let len = upper.len();
    (upper, SourceMap { pieces, len })
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

/// Which clusters are the white space at the logical end of the line. The rules of break,
/// letter spacing and overflow treat all of it as hanging. With `paragraph_rtl`, only the
/// white space whose run has the direction of the paragraph is kept: that space is at the
/// edge of the line and really hangs. Parley does not apply UAX #9 L1 (FD-41, accepted
/// limit), so the white space of a run of the other direction stays where bidi puts it.
fn trailing_space(text: &str, clusters: &[Cluster], paragraph_rtl: Option<bool>) -> Vec<bool> {
    let mut logical: Vec<usize> = (0..clusters.len()).collect();
    logical.sort_by_key(|i| clusters[*i].range.start);
    let hangs = |i: &usize| {
        text[clusters[*i].range.clone()]
            .chars()
            .all(char::is_whitespace)
            && paragraph_rtl.is_none_or(|rtl| clusters[*i].rtl == rtl)
    };
    let hanging = logical.iter().rev().take_while(|i| hangs(i)).count();
    let mut mask = vec![false; clusters.len()];
    for i in &logical[logical.len() - hanging..] {
        mask[*i] = true;
    }
    mask
}

fn hanging_mask(text: &str, clusters: &[Cluster]) -> Vec<bool> {
    trailing_space(text, clusters, None)
}

/// The advance of the widest cluster of a line that is not white space that hangs. The
/// overflow is about a cluster wider than the box, not about the sum of a line, which parley
/// decides in f32 and can differ from an f64 sum on a long line.
fn widest_content(text: &str, clusters: &[Cluster]) -> f64 {
    let hanging = hanging_mask(text, clusters);
    clusters
        .iter()
        .zip(&hanging)
        .filter(|(_, h)| !**h)
        .map(|(c, _)| f64::from(c.advance))
        .fold(0.0, f64::max)
}

/// Places the clusters of a line, which start at x = 0, in the box. Every alignment and
/// direction goes through the same three steps, in f64: the hang, then the room, then the
/// offsets. Positions that change (a stretched line) are summed in f64 from the origin of
/// the line, never by adding f32 positions one to the next; the others keep the position
/// of the layout (FD-41 (c), identity bit for bit) plus the offset.
///
/// 1. The white space at the end of the line whose run has the direction of the paragraph
///    hangs: it is outside the alignment, on the right of a left-to-right paragraph and on
///    the left (negative x) of a right-to-left one. The hang is applied whatever the room.
/// 2. The room is the box less the content, without any trailing white space. A line wider
///    than the box has none and stays where the hang puts it, except under `Justify` in a
///    right-to-left paragraph, which ends at the right edge.
/// 3. `Left` adds nothing; `Center` and `Right` move the content by the room; `Justify`
///    shares the room among the spaces that are neither trailing nor leading white space.
///    A line with none, the last line and a line with a forced break (`stretch` false) start
///    where the paragraph starts: at the left in a left-to-right paragraph and at the right
///    in a right-to-left one. The trailing space of a run of the other level is not a place
///    to stretch either (FD-41, accepted limit).
fn align_line(line: &mut Line, text: &str, align: TextAlign, rtl: bool, width: f64, stretch: bool) {
    let hung = trailing_space(text, &line.clusters, Some(rtl));
    let hung_width = zip_sum(&line.clusters, &hung);
    let advances: Vec<f64> = line.clusters.iter().map(|c| f64::from(c.advance)).collect();
    let mut extra = vec![0.0_f64; advances.len()];
    // 1. The hang.
    let mut offset = if rtl { -hung_width } else { 0.0 };
    let total: f64 = advances.iter().sum();
    match align {
        TextAlign::Left => {}
        TextAlign::Center | TextAlign::Right => {
            let trailing = hanging_mask(text, &line.clusters);
            let mut bounds: Option<(f64, f64)> = None;
            for ((c, adv), t) in line.clusters.iter().zip(&advances).zip(&trailing) {
                if !t {
                    let start = f64::from(c.x);
                    let (l, r) = bounds.unwrap_or((start, start + adv));
                    bounds = Some((l.min(start), r.max(start + adv)));
                }
            }
            // 2. The room.
            if let Some((l, r)) = bounds {
                let room = width - (r - l);
                if room > 0.0 {
                    // 3. The offset.
                    offset = if align == TextAlign::Center {
                        room / 2.0 - l
                    } else {
                        room - l
                    };
                }
            }
        }
        TextAlign::Justify => {
            let trailing = hanging_mask(text, &line.clusters);
            let leading = leading_space(text, &line.clusters);
            let separators: Vec<usize> = (0..advances.len())
                .filter(|i| {
                    !trailing[*i]
                        && !leading[*i]
                        && matches!(&text[line.clusters[*i].range.clone()], " " | "\u{a0}")
                })
                .collect();
            let room = width - (total - hung_width);
            if stretch && !separators.is_empty() && room > -width * 1e-4 {
                let each = room / separators.len() as f64;
                for i in &separators {
                    extra[*i] = each;
                }
            } else if rtl {
                offset = width - total;
            }
        }
    }
    if extra.iter().any(|e| *e != 0.0) {
        let mut x = 0.0_f64;
        for ((c, adv), more) in line.clusters.iter_mut().zip(&advances).zip(&extra) {
            c.x = (offset + x) as f32;
            c.advance = (adv + more) as f32;
            x += adv + more;
        }
        line.advance = x as f32;
    } else {
        for c in &mut line.clusters {
            c.x = (f64::from(c.x) + offset) as f32;
        }
    }
}

/// Which clusters are the white space at the logical start of the line.
fn leading_space(text: &str, clusters: &[Cluster]) -> Vec<bool> {
    let mut logical: Vec<usize> = (0..clusters.len()).collect();
    logical.sort_by_key(|i| clusters[*i].range.start);
    let mut mask = vec![false; clusters.len()];
    for i in logical.into_iter().take_while(|i| {
        text[clusters[*i].range.clone()]
            .chars()
            .all(char::is_whitespace)
    }) {
        mask[i] = true;
    }
    mask
}

fn zip_sum(clusters: &[Cluster], mask: &[bool]) -> f64 {
    clusters
        .iter()
        .zip(mask)
        .filter(|(_, m)| **m)
        .map(|(c, _)| f64::from(c.advance))
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

pub(crate) fn finite(field: &'static str, value: f64) -> Result<f64, TextError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(TextError::NonFinite { field })
    }
}

pub(crate) fn in_range(
    field: &'static str,
    value: f64,
    (lo, hi): (f64, f64),
) -> Result<(), TextError> {
    if (lo..=hi).contains(&value) {
        Ok(())
    } else {
        Err(TextError::OutOfRange { field })
    }
}

/// Non-finite checks of every input first, then the ranges, then the computed size.
/// Returns the font size in px.
pub(crate) fn check_input(style: &TextStyle, dpi: f64, bounds: Size) -> Result<f32, TextError> {
    check_input_with(style, dpi, bounds, &[])
}

/// As [`check_input`], with the numbers of a caller (`extra`) checked for NaN and infinity
/// after the common ones and before any range.
pub(crate) fn check_input_with(
    style: &TextStyle,
    dpi: f64,
    bounds: Size,
    extra: &[(&'static str, f64)],
) -> Result<f32, TextError> {
    let size_pt = finite("style.size_pt", style.size_pt)?;
    let spacing = finite("style.letter_spacing", style.letter_spacing)?;
    let line_height = finite("style.line_height", style.line_height)?;
    finite("dpi", dpi)?;
    finite("bounds.width", bounds.width())?;
    finite("bounds.height", bounds.height())?;
    for (field, value) in extra {
        finite(field, *value)?;
    }
    in_range("style.size_pt", size_pt, SIZE_PT)?;
    in_range("style.letter_spacing", spacing, LETTER_SPACING)?;
    in_range("style.line_height", line_height, LINE_HEIGHT)?;
    to_px(size_pt, dpi)
}

/// A size in pt as px at `dpi`; a result that is not finite, also in f32, is an error.
pub(crate) fn to_px(size_pt: f64, dpi: f64) -> Result<f32, TextError> {
    let size_px = finite("size_px", size_pt * dpi / 72.0)? as f32;
    if size_px.is_finite() {
        Ok(size_px)
    } else {
        Err(TextError::NonFinite { field: "size_px" })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decorated() -> TextLayout {
        let decoration = Some(Decoration {
            offset: -1.0,
            thickness: 1.0,
        });
        TextLayout {
            height: 10.0,
            lines: vec![Line {
                baseline: 8.0,
                advance: 5.0,
                height: 10.0,
                clusters: vec![Cluster {
                    range: 0..1,
                    x: 0.0,
                    advance: 5.0,
                    underline: decoration,
                    strikethrough: decoration,
                    ..Cluster::default()
                }],
            }],
            overflow: false,
        }
    }

    /// A named change to one field of a layout.
    type Edit = (&'static str, fn(&mut TextLayout));

    // S13-13, ARQ-110: every number of the output is checked on its own: one non-finite
    // field at a time, the others finite, is an error.
    #[test]
    fn arq_110_check_output_rejects_each_non_finite_field_alone() {
        assert_eq!(check_output(&decorated()), Ok(()));
        let edits: [Edit; 8] = [
            ("line.baseline", |l| l.lines[0].baseline = f32::NAN),
            ("line.advance", |l| l.lines[0].advance = f32::INFINITY),
            ("cluster.x", |l| l.lines[0].clusters[0].x = f32::NAN),
            ("cluster.advance", |l| {
                l.lines[0].clusters[0].advance = f32::NAN;
            }),
            ("underline.offset", |l| {
                if let Some(d) = &mut l.lines[0].clusters[0].underline {
                    d.offset = f32::NAN;
                }
            }),
            ("underline.thickness", |l| {
                if let Some(d) = &mut l.lines[0].clusters[0].underline {
                    d.thickness = f32::NAN;
                }
            }),
            ("strikethrough.offset", |l| {
                if let Some(d) = &mut l.lines[0].clusters[0].strikethrough {
                    d.offset = f32::NEG_INFINITY;
                }
            }),
            ("strikethrough.thickness", |l| {
                if let Some(d) = &mut l.lines[0].clusters[0].strikethrough {
                    d.thickness = f32::NAN;
                }
            }),
        ];
        for (name, edit) in edits {
            let mut layout = decorated();
            edit(&mut layout);
            assert_eq!(
                check_output(&layout),
                Err(TextError::NonFinite { field: "output" }),
                "{name}"
            );
        }
    }

    /// A table directory at `start` with the given tags.
    fn directory(start: usize, tags: &[&[u8; 4]]) -> Vec<u8> {
        let mut out = vec![0_u8; start];
        out.extend_from_slice(&[0, 1, 0, 0]);
        out.extend_from_slice(&u16::try_from(tags.len()).expect("few").to_be_bytes());
        out.extend_from_slice(&[0; 6]);
        for tag in tags {
            out.extend_from_slice(*tag);
            out.extend_from_slice(&[0; 12]);
        }
        out
    }

    /// A collection header for `offsets`, padded to `len`.
    fn collection(offsets: &[u32], len: usize) -> Vec<u8> {
        let mut out = b"ttcf".to_vec();
        // Version 2.0: a number no offset below uses.
        out.extend_from_slice(&[0, 2, 0, 0]);
        out.extend_from_slice(&u32::try_from(offsets.len()).expect("few").to_be_bytes());
        for offset in offsets {
            out.extend_from_slice(&offset.to_be_bytes());
        }
        out.resize(len, 0);
        out
    }

    // S13-12: the table directory of a single font and of each font of a collection is read
    // at its own offset.
    #[test]
    fn has_table_reads_the_directory_of_a_font_and_of_each_font_of_a_collection() {
        let single = directory(0, &[b"OS/2", b"post"]);
        assert!(has_table(&single, 0, b"post"));
        assert!(!has_table(&single, 0, b"glyf"));

        // The smallest collection of one font: 16 bytes of header and a directory of one
        // table, 44 bytes in all.
        let mut small = collection(&[16], 16);
        small.extend_from_slice(&directory(0, &[b"post"]));
        assert_eq!(small.len(), 44);
        assert!(has_table(&small, 0, b"post"));

        // Two fonts: the first has `post`, the second `OS/2`.
        let mut two = collection(&[20, 48], 20);
        two.extend_from_slice(&directory(0, &[b"post"]));
        two.extend_from_slice(&directory(0, &[b"OS/2"]));
        assert!(has_table(&two, 0, b"post"));
        assert!(!has_table(&two, 0, b"OS/2"));
        assert!(has_table(&two, 1, b"OS/2"));
        assert!(!has_table(&two, 1, b"post"));
        // A font the header does not list, and a directory cut short.
        assert!(!has_table(&two, 2, b"post"));
        assert!(!has_table(&two[..24], 0, b"post"));
    }
}
