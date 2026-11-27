//! Copy the style, the effects or the properties of one element and paste
//! them on another (spec 105): the changes a paste makes, as properties of
//! `SetProperty`, so one paste is one undo step.

use gafira_doc::{Node, NodeKind, RichText, SpanStyle};
use gafira_edit::Property;

/// What a copy takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    /// How the words or the shape look: font, size, colour, alignment and
    /// spacing; fill, gradient, outline and corners (S105-03).
    Style,
    /// The effects of a text, the light of a shape, the look of an image
    /// (S105-04).
    Effects,
    /// Style and effects, and the opacity and blend of any element (S105-05).
    Properties,
}

/// A copy waiting to be pasted: what it takes and the element it came from.
#[derive(Clone, Debug)]
pub(crate) struct Copied {
    pub part: Part,
    pub from: Node,
}

/// `content` with every span in `style`: the words stay.
fn restyled(content: &RichText, style: &SpanStyle) -> RichText {
    let mut out = content.clone();
    for paragraph in &mut out.paragraphs {
        for span in &mut paragraph.spans {
            span.style = style.clone();
        }
    }
    out
}

/// The style of the words of `node`, if it has words.
fn words_style(node: &Node) -> Option<&SpanStyle> {
    match &node.kind {
        NodeKind::Text(text) => text.content.first_style(),
        NodeKind::Shape(shape) => shape.text.as_ref()?.content.first_style(),
        _ => None,
    }
}

fn style(from: &Node, to: &Node) -> Vec<Property> {
    let mut out = Vec::new();
    let words = words_style(from);
    match (&from.kind, &to.kind) {
        (NodeKind::Text(a), NodeKind::Text(b)) => {
            if let Some(style) = words {
                out.push(Property::TextContent(restyled(&b.content, style)));
            }
            out.push(Property::TextAlign(a.align));
            out.push(Property::TextSpacing(a.letter_spacing, a.line_height));
            out.push(Property::TextCurve(a.curve));
            out.push(Property::TextGradient(a.gradient.clone()));
        }
        (_, NodeKind::Text(b)) => {
            if let Some(style) = words {
                out.push(Property::TextContent(restyled(&b.content, style)));
            }
        }
        (NodeKind::Shape(a), NodeKind::Shape(b)) => {
            out.push(Property::Fill(a.fill));
            out.push(Property::Gradient(a.gradient.clone()));
            out.push(Property::Stroke(a.stroke));
            out.push(Property::CornerRadius(a.corner_radius));
            if let (Some(style), Some(text)) = (words, &b.text) {
                let mut text = text.clone();
                text.content = restyled(&text.content, style);
                out.push(Property::ShapeText(Some(text)));
            }
        }
        (_, NodeKind::Shape(b)) => {
            if let (Some(style), Some(text)) = (words, &b.text) {
                let mut text = text.clone();
                text.content = restyled(&text.content, style);
                out.push(Property::ShapeText(Some(text)));
            }
        }
        _ => {}
    }
    out
}

fn effects(from: &Node, to: &Node) -> Vec<Property> {
    match (&from.kind, &to.kind) {
        (NodeKind::Text(a), NodeKind::Text(_)) => vec![Property::TextEffects(a.effects.clone())],
        (NodeKind::Shape(a), NodeKind::Shape(_)) => vec![Property::Effect(a.effect.clone())],
        (NodeKind::Image(a), NodeKind::Image(b)) => {
            let mut image = b.clone();
            image.look.clone_from(&a.look);
            vec![Property::Image(image)]
        }
        _ => Vec::new(),
    }
}

/// The changes that paste `copied` on `to`, none when nothing applies.
/// Style changing the words keeps them, and only what both elements have
/// is pasted.
pub(crate) fn pasted(copied: &Copied, to: &Node) -> Vec<Property> {
    let from = &copied.from;
    let mut out = match copied.part {
        Part::Style => style(from, to),
        Part::Effects => effects(from, to),
        Part::Properties => {
            let mut all = style(from, to);
            all.extend(effects(from, to));
            all
        }
    };
    if copied.part == Part::Properties {
        out.push(Property::Opacity(from.base.opacity));
        out.push(Property::Blend(from.base.blend));
    }
    out
}

#[cfg(test)]
mod tests {
    use gafira_color::{Color, Rgba};
    use gafira_doc::{
        ElementBase, NodeId, Placement, ShapeGeometry, ShapeNode, ShapeText, TextEffect,
        TextEffectKind, TextNode,
    };

    use super::*;

    fn span(font: &str, size: f64, bold: bool) -> SpanStyle {
        SpanStyle {
            font: font.to_owned(),
            size,
            color: Color::Srgb(Rgba::new(0.2, 0.3, 0.4, 1.0)),
            bold,
            italic: false,
            gradient: None,
        }
    }

    fn node(id: u64, kind: NodeKind) -> Node {
        Node {
            base: ElementBase::new(NodeId::new(id), Placement::new(0.0, 0.0, 100.0, 40.0)),
            kind,
            children: Vec::new(),
        }
    }

    fn text(id: u64, words: &str, style: &SpanStyle) -> Node {
        node(id, NodeKind::Text(TextNode::plain(words, style)))
    }

    fn shape(id: u64) -> Node {
        let mut s = ShapeNode::new(ShapeGeometry::Rect);
        s.fill = Some(Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0)));
        s.corner_radius = 12.0;
        node(id, NodeKind::Shape(s))
    }

    // S105-03: the style of a text goes to another; its words stay.
    #[test]
    fn s105_03_text_style_keeps_the_words() {
        let mut from = text(1, "Origen", &span("Lora", 40.0, true));
        if let NodeKind::Text(t) = &mut from.kind {
            t.align = gafira_doc::TextAlign::Center;
            t.letter_spacing = 50.0;
        }
        let to = text(2, "Hola\nmundo", &span("Inter", 16.0, false));
        let copied = Copied {
            part: Part::Style,
            from,
        };
        let changes = pasted(&copied, &to);
        let content = changes.iter().find_map(|p| match p {
            Property::TextContent(c) => Some(c.clone()),
            _ => None,
        });
        let content = content.expect("the words");
        assert_eq!(content.plain_text(), "Hola\nmundo");
        assert!(content.spans().all(|s| s.style == span("Lora", 40.0, true)));
        assert!(changes.contains(&Property::TextAlign(gafira_doc::TextAlign::Center)));
        assert!(changes.contains(&Property::TextSpacing(50.0, None)));
        assert!(!changes.iter().any(|p| matches!(p, Property::Opacity(_))));
    }

    // S105-03: a shape's look goes to a shape; the words inside take the style.
    #[test]
    fn s105_03_shape_style() {
        let from = shape(1);
        let mut to = shape(2);
        if let NodeKind::Shape(s) = &mut to.kind {
            s.fill = None;
            s.text = Some(ShapeText::plain("Dentro", &span("Inter", 12.0, false)));
        }
        let mut from_with_words = from.clone();
        if let NodeKind::Shape(s) = &mut from_with_words.kind {
            s.text = Some(ShapeText::plain("x", &span("Lora", 30.0, true)));
        }
        let copied = Copied {
            part: Part::Style,
            from: from_with_words,
        };
        let changes = pasted(&copied, &to);
        assert!(changes.contains(&Property::Fill(Some(Color::Srgb(Rgba::new(
            1.0, 0.0, 0.0, 1.0
        ))))));
        assert!(changes.contains(&Property::CornerRadius(12.0)));
        let inside = changes.iter().find_map(|p| match p {
            Property::ShapeText(Some(t)) => Some(t.content.clone()),
            _ => None,
        });
        let inside = inside.expect("the words inside");
        assert_eq!(inside.plain_text(), "Dentro");
        assert_eq!(inside.first_style(), Some(&span("Lora", 30.0, true)));
    }

    // S105-04: effects go between elements of a kind; across kinds, nothing.
    #[test]
    fn s105_04_effects_only_between_alike() {
        let mut from = text(1, "Brillo", &span("Inter", 20.0, false));
        if let NodeKind::Text(t) = &mut from.kind {
            t.effects = vec![TextEffect::new(TextEffectKind::Drop)];
        }
        let copied = Copied {
            part: Part::Effects,
            from,
        };
        let to_text = text(2, "Otro", &span("Inter", 20.0, false));
        let changes = pasted(&copied, &to_text);
        assert!(matches!(&changes[..], [Property::TextEffects(e)] if e.len() == 1));
        assert!(pasted(&copied, &shape(3)).is_empty());
    }

    // S105-05: properties across kinds are only the opacity and the blend.
    #[test]
    fn s105_05_properties_across_kinds() {
        let mut from = shape(1);
        from.base.opacity = 0.5;
        let copied = Copied {
            part: Part::Properties,
            from,
        };
        let to = text(2, "Texto", &span("Inter", 20.0, false));
        let changes = pasted(&copied, &to);
        assert_eq!(
            changes,
            vec![
                Property::Opacity(0.5),
                Property::Blend(gafira_doc::BlendMode::default())
            ]
        );
    }
}
