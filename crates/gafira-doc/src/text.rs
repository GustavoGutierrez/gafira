//! The text element of 02 §3.1 (`TextNode`, ADR 0008), first slice
//! (spec 027): rich text in paragraphs of styled spans, how the box sizes,
//! and how lines align. Columns, «Ajustar al cuadro», «Fijar tipografía»,
//! tracking, line height and text on a path join schema 1 later as
//! additive fields (ADR 0006).

use gafira_color::Color;
use serde::{Deserialize, Serialize};

/// The style of a span: font family, size in px, colour, and its bold and
/// italic faces (spec 062).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpanStyle {
    /// The font family name, for example `"Inter"`.
    pub font: String,
    /// The font size in px (`> 0`, at most [`TextNode::MAX_SIZE`]).
    pub size: f64,
    /// The text colour.
    pub color: Color,
    /// The bold face of the family (spec 062). Written only when set.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bold: bool,
    /// The italic face of the family (spec 062). Written only when set.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub italic: bool,
    /// A gradient for the letters of this span instead of its colour (spec
    /// 106), over the box of the text. Written only when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient: Option<crate::Gradient>,
}

/// The name a face is registered under for the render and the canvas (spec 062):
/// the family, then «Bold», «Italic» or «Bold Italic».
#[must_use]
pub fn face_key(family: &str, bold: bool, italic: bool) -> String {
    match (bold, italic) {
        (false, false) => family.to_owned(),
        (true, false) => format!("{family} Bold"),
        (false, true) => format!("{family} Italic"),
        (true, true) => format!("{family} Bold Italic"),
    }
}

/// The face to use for `family` with `bold` and `italic` when `has` tells
/// which faces exist: the one asked for, else without italic, else without
/// bold, else the regular one. `None` when the family has no face at all.
#[must_use]
pub fn nearest_face(
    has: impl Fn(bool, bool) -> bool,
    bold: bool,
    italic: bool,
) -> Option<(bool, bool)> {
    [
        (bold, italic),
        (bold, false),
        (false, italic),
        (false, false),
    ]
    .into_iter()
    .find(|(b, i)| has(*b, *i))
}

/// A run of text with one style.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Span {
    pub text: String,
    pub style: SpanStyle,
}

/// A paragraph: its spans, in reading order.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Paragraph {
    pub spans: Vec<Span>,
}

/// Rich text: paragraphs in order (02 §3.1).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RichText {
    pub paragraphs: Vec<Paragraph>,
}

impl RichText {
    /// `text` in one style, one paragraph per line (`\n`).
    #[must_use]
    pub fn plain(text: &str, style: &SpanStyle) -> Self {
        Self {
            paragraphs: text
                .split('\n')
                .map(|line| Paragraph {
                    spans: vec![Span {
                        text: line.to_owned(),
                        style: style.clone(),
                    }],
                })
                .collect(),
        }
    }

    /// The text without styles, paragraphs joined by `\n`.
    #[must_use]
    pub fn plain_text(&self) -> String {
        self.paragraphs
            .iter()
            .map(|p| p.spans.iter().map(|s| s.text.as_str()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The style of the first span, if any: what a new line takes.
    #[must_use]
    pub fn first_style(&self) -> Option<&SpanStyle> {
        self.paragraphs
            .iter()
            .flat_map(|p| &p.spans)
            .map(|s| &s.style)
            .next()
    }

    /// Every span, in reading order.
    pub fn spans(&self) -> impl Iterator<Item = &Span> {
        self.paragraphs.iter().flat_map(|p| &p.spans)
    }
    /// Whether its spans have more than one style (spec 106).
    #[must_use]
    pub fn mixed(&self) -> bool {
        let mut spans = self.spans().filter(|s| !s.text.is_empty());
        let Some(first) = spans.next() else {
            return false;
        };
        spans.any(|s| s.style != first.style)
    }

    /// The characters of the text, a line break between paragraphs
    /// counting as one (the indices of [`RichText::restyle`]).
    #[must_use]
    pub fn char_count(&self) -> usize {
        let letters: usize = self.spans().map(|s| s.text.chars().count()).sum();
        letters + self.paragraphs.len().saturating_sub(1)
    }

    /// The style of the character at `at` (a line break has the style of the
    /// letter before it); `None` past the end.
    #[must_use]
    pub fn style_at(&self, at: usize) -> Option<&SpanStyle> {
        let mut left = at;
        for (k, paragraph) in self.paragraphs.iter().enumerate() {
            for span in &paragraph.spans {
                let count = span.text.chars().count();
                if left < count {
                    return Some(&span.style);
                }
                left -= count;
            }
            if k + 1 < self.paragraphs.len() {
                if left == 0 {
                    return paragraph
                        .spans
                        .last()
                        .map(|s| &s.style)
                        .or(self.first_style());
                }
                left -= 1;
            }
        }
        None
    }

    /// Changes with `change` the style of the characters in `range` (spec
    /// 106): spans are cut where the range starts and ends, and spans of
    /// one style next to each other become one. An empty range changes
    /// nothing.
    pub fn restyle(&mut self, range: std::ops::Range<usize>, change: &dyn Fn(&mut SpanStyle)) {
        if range.start >= range.end {
            return;
        }
        let mut letters = self.letters();
        for (at, letter) in letters.iter_mut().enumerate() {
            if range.contains(&at)
                && let Letter::Char(_, style) = letter
            {
                change(style);
            }
        }
        *self = Self::from_letters(&letters, self.first_style().cloned());
    }

    /// The text with `words` as its letters (spec 106): what stayed of the
    /// old words keeps its styles, what was typed takes the style of the
    /// letter before it (or after it, at the start), and a line break starts
    /// a paragraph. The words are compared from both ends, as an edit at
    /// one place changes them.
    #[must_use]
    pub fn edited(&self, words: &str) -> RichText {
        let old = self.letters();
        let new: Vec<char> = words.chars().collect();
        let as_char = |l: &Letter| match l {
            Letter::Char(c, _) => *c,
            Letter::Break => '\n',
        };
        let same_start = old
            .iter()
            .zip(&new)
            .take_while(|(a, b)| as_char(a) == **b)
            .count();
        let room = old.len().min(new.len()) - same_start;
        let same_end = old
            .iter()
            .rev()
            .zip(new.iter().rev())
            .take(room)
            .take_while(|(a, b)| as_char(a) == **b)
            .count();
        let style_of = |l: &Letter| match l {
            Letter::Char(_, s) => Some(s.clone()),
            Letter::Break => None,
        };
        // The style typed letters take: the letter before, else after.
        let typed = old[..same_start]
            .iter()
            .rev()
            .find_map(style_of)
            .or_else(|| old[old.len() - same_end..].iter().find_map(style_of))
            .or_else(|| self.first_style().cloned());
        let mut letters: Vec<Letter> = old[..same_start].to_vec();
        for c in &new[same_start..new.len() - same_end] {
            letters.push(match (c, &typed) {
                ('\n', _) => Letter::Break,
                (c, Some(style)) => Letter::Char(*c, style.clone()),
                (_, None) => continue,
            });
        }
        letters.extend_from_slice(&old[old.len() - same_end..]);
        Self::from_letters(&letters, self.first_style().cloned())
    }

    /// Every character with its style, paragraphs apart by a break.
    fn letters(&self) -> Vec<Letter> {
        let mut out = Vec::new();
        for (k, paragraph) in self.paragraphs.iter().enumerate() {
            if k > 0 {
                out.push(Letter::Break);
            }
            for span in &paragraph.spans {
                out.extend(
                    span.text
                        .chars()
                        .map(|c| Letter::Char(c, span.style.clone())),
                );
            }
        }
        out
    }

    /// The text of `letters`, one span per run of one style; an empty
    /// paragraph keeps one empty span in the style around it, or `fallback`.
    fn from_letters(letters: &[Letter], fallback: Option<SpanStyle>) -> Self {
        let mut paragraphs = vec![Paragraph::default()];
        let mut last = fallback;
        for letter in letters {
            match letter {
                Letter::Break => {
                    if let (Some(current), Some(style)) = (paragraphs.last_mut(), &last)
                        && current.spans.is_empty()
                    {
                        current.spans.push(Span {
                            text: String::new(),
                            style: style.clone(),
                        });
                    }
                    paragraphs.push(Paragraph::default());
                }
                Letter::Char(c, style) => {
                    last = Some(style.clone());
                    let Some(current) = paragraphs.last_mut() else {
                        continue;
                    };
                    match current.spans.last_mut() {
                        Some(span) if span.style == *style => span.text.push(*c),
                        _ => current.spans.push(Span {
                            text: c.to_string(),
                            style: style.clone(),
                        }),
                    }
                }
            }
        }
        if let (Some(current), Some(style)) = (paragraphs.last_mut(), &last)
            && current.spans.is_empty()
        {
            current.spans.push(Span {
                text: String::new(),
                style: style.clone(),
            });
        }
        Self { paragraphs }
    }
}

/// A character of a rich text and its style, or a break between paragraphs.
#[derive(Clone)]
enum Letter {
    Char(char, SpanStyle),
    Break,
}

/// How the box follows the text (02 §3.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextSizing {
    /// The box grows sideways with the text.
    AutoWidth,
    /// The width is fixed and the box grows downwards.
    #[default]
    AutoHeight,
    /// Both sides are fixed.
    Fixed,
}

/// How the lines align in the box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

/// A text element (02 §3.1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextNode {
    pub content: RichText,
    #[serde(default)]
    pub sizing: TextSizing,
    #[serde(default)]
    pub align: TextAlign,
    /// How much the lines bend (FD-44, D-26), in `[-100, 100]`: positive
    /// is an arch, negative a cup, 0 straight. Additive to schema 1.
    #[serde(default, skip_serializing_if = "is_straight")]
    pub curve: f64,
    /// Room between letters, in thousandths of an em, in `[-100, 500]`
    /// (FD-41). Additive to schema 1.
    #[serde(default, skip_serializing_if = "is_straight")]
    pub letter_spacing: f64,
    /// Line height as a multiple of the size, in `[0.5, 4]` (FD-41); none
    /// is the font's own. Additive to schema 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_height: Option<f64>,
    /// A gradient the letters are filled with instead of their colour, over
    /// the box (spec 070). Additive to schema 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient: Option<crate::Gradient>,
    /// What is done to its letters (spec 089). Additive to schema 1.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<crate::TextEffect>,
}

fn is_straight(curve: &f64) -> bool {
    *curve == 0.0
}

impl TextNode {
    /// The largest font size, in px.
    pub const MAX_SIZE: f64 = 4096.0;

    /// The most a text bends, either way (FD-44).
    pub const MAX_CURVE: f64 = 100.0;

    /// The letter spacing range, in thousandths of an em (FD-41).
    pub const LETTER_SPACING: std::ops::RangeInclusive<f64> = -100.0..=500.0;

    /// The line height range, as a multiple of the size (FD-41).
    pub const LINE_HEIGHT: std::ops::RangeInclusive<f64> = 0.5..=4.0;

    /// Whether it is drawn from its outline (curve, spacing, line height,
    /// gradient or effects) rather than as plain lines of words.
    #[must_use]
    pub fn styled(&self) -> bool {
        self.curve != 0.0
            || self.letter_spacing != 0.0
            || self.line_height.is_some()
            || self.gradient.is_some()
            || !self.effects.is_empty()
            || self.content.mixed()
    }

    /// `text` in one style, left aligned, growing downwards.
    #[must_use]
    pub fn plain(text: &str, style: &SpanStyle) -> Self {
        Self {
            content: RichText::plain(text, style),
            sizing: TextSizing::AutoHeight,
            align: TextAlign::Left,
            curve: 0.0,
            letter_spacing: 0.0,
            line_height: None,
            gradient: None,
            effects: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use gafira_color::Rgba;

    use super::*;

    fn style() -> SpanStyle {
        SpanStyle {
            font: "Inter".to_owned(),
            size: 32.0,
            color: Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)),
            bold: false,
            italic: false,
            gradient: None,
        }
    }

    #[test]
    fn s27_01_plain_text_splits_lines_and_reads_back() {
        let text = RichText::plain("Hola\nmundo\n", &style());
        assert_eq!(
            text.paragraphs.len(),
            3,
            "a trailing newline is an empty paragraph"
        );
        assert_eq!(text.plain_text(), "Hola\nmundo\n");
        assert_eq!(text.first_style(), Some(&style()));
        assert_eq!(text.spans().count(), 3);
        assert_eq!(RichText::default().first_style(), None);
    }

    #[test]
    fn s27_01_the_wire_form_is_tagged_and_round_trips() {
        let node = TextNode::plain("Hola", &style());
        let json = serde_json::to_string(&node).expect("writes");
        assert!(json.contains(r#""sizing":"auto_height""#), "{json}");
        assert!(json.contains(r#""align":"left""#), "{json}");
        assert_eq!(
            serde_json::from_str::<TextNode>(&json).expect("reads"),
            node
        );
        let bare = r#"{"content":{"paragraphs":[]}}"#;
        let read: TextNode = serde_json::from_str(bare).expect("defaults");
        assert_eq!(
            (read.sizing, read.align),
            (TextSizing::AutoHeight, TextAlign::Left)
        );
    }
}
