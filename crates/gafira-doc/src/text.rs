//! The text element of 02 §3.1 (`TextNode`, ADR 0008), first slice
//! (spec 027): rich text in paragraphs of styled spans, how the box sizes,
//! and how lines align. Columns, «Ajustar al cuadro», «Fijar tipografía»,
//! tracking, line height and text on a path join schema 1 later as
//! additive fields (ADR 0006).

use gafira_color::Color;
use serde::{Deserialize, Serialize};

/// The style of a span: font family, size in px and colour.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpanStyle {
    /// The font family name, for example `"Inter"`.
    pub font: String,
    /// The font size in px (`> 0`, at most [`TextNode::MAX_SIZE`]).
    pub size: f64,
    /// The text colour.
    pub color: Color,
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
}

impl TextNode {
    /// The largest font size, in px.
    pub const MAX_SIZE: f64 = 4096.0;

    /// `text` in one style, left aligned, growing downwards.
    #[must_use]
    pub fn plain(text: &str, style: &SpanStyle) -> Self {
        Self {
            content: RichText::plain(text, style),
            sizing: TextSizing::AutoHeight,
            align: TextAlign::Left,
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
