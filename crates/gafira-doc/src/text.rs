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
