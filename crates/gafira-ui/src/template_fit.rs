//! A template on a page of the editor (spec 091): which templates suit the
//! page's proportions, and the template's pages fitted to the page, whole
//! and centred, with their letters, outlines and corners scaled as well.

use gafira_doc::{Document, NodeKind, Page};

use crate::textbox;

/// How far, as a factor, a template's proportions may be from the page's:
/// A4 and Letter suit each other; a square and a 4:5 post do not.
const TOLERANCE: f64 = 1.12;

/// Whether a template of width over height `template` suits a page of
/// `page`.
pub(crate) fn fits(page: f64, template: f64) -> bool {
    if !(page.is_finite() && template.is_finite() && page > 0.0 && template > 0.0) {
        return false;
    }
    (page / template).ln().abs() <= TOLERANCE.ln()
}

/// The pages of `template`, each fitted to a page of `width` × `height`
/// px: the largest scale that keeps it whole, centred. The ids stay as the
/// template has them; `ApplyTemplate` gives new ones.
pub(crate) fn fitted(template: &Document, width: f64, height: f64) -> Vec<Page> {
    template
        .pages()
        .iter()
        .filter_map(|page| {
            let px = page.size.to_px(template.dpi()).ok()?;
            let (w, h) = (px.width(), px.height());
            if w <= 0.0 || h <= 0.0 {
                return None;
            }
            let s = (width / w).min(height / h);
            let (dx, dy) = ((width - w * s) / 2.0, (height - h * s) / 2.0);
            Some(scaled_page(page, s, dx, dy))
        })
        .collect()
}

/// `page` with every element scaled by `s` and moved by `dx, dy`.
fn scaled_page(page: &Page, s: f64, dx: f64, dy: f64) -> Page {
    let mut out = page.clone();
    for node in out.nodes.values_mut() {
        let node = std::sync::Arc::make_mut(node);
        let p = &mut node.base.placement;
        p.x = p.x * s + dx;
        p.y = p.y * s + dy;
        p.w *= s;
        p.h *= s;
        match &mut node.kind {
            NodeKind::Text(text) => text.content = textbox::scaled(&text.content, s),
            NodeKind::Shape(shape) => {
                shape.corner_radius *= s;
                if let Some(stroke) = &mut shape.stroke {
                    stroke.width *= s;
                }
                if let Some(text) = &mut shape.text {
                    text.content = textbox::scaled(&text.content, s);
                }
            }
            NodeKind::Line(line) => line.width *= s,
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gafira_doc::{
        Dpi, ElementBase, Node, NodeId, PageId, PageSize, Placement, ShapeGeometry, ShapeNode,
        ShapeStroke, SpanStyle, TextNode,
    };
    use gafira_geom::{Length, Unit};

    use super::*;

    #[test]
    fn s91_11_only_close_proportions_suit_a_page() {
        assert!(fits(1.0, 1.0));
        assert!(fits(297.0 / 210.0, 11.0 / 8.5), "A4 and Letter, lying");
        assert!(!fits(1.0, 0.8), "a square and a 4:5 post");
        assert!(!fits(16.0 / 9.0, 297.0 / 210.0), "16:9 and A4");
        assert!(!fits(1.0, 0.0));
        assert!(!fits(f64::NAN, 1.0));
    }

    fn style() -> SpanStyle {
        SpanStyle {
            font: "Inter".to_owned(),
            size: 20.0,
            color: gafira_color::Color::Srgb(gafira_color::Rgba::new(0.0, 0.0, 0.0, 1.0)),
            bold: false,
            italic: false,
        }
    }

    #[test]
    fn s91_11_a_template_is_scaled_whole_and_centred() {
        let size = PageSize {
            width: Length::new(100.0, Unit::Px),
            height: Length::new(50.0, Unit::Px),
        };
        let mut page = Page::new(PageId::new(1), String::new(), size);
        let mut shape = ShapeNode::new(ShapeGeometry::Rect);
        shape.corner_radius = 4.0;
        shape.stroke = Some(ShapeStroke::new(shape_colour(), 2.0));
        page.nodes.insert(
            NodeId::new(2),
            Arc::new(Node::new(
                ElementBase::new(NodeId::new(2), Placement::new(10.0, 10.0, 20.0, 10.0)),
                NodeKind::Shape(shape),
            )),
        );
        page.nodes.insert(
            NodeId::new(3),
            Arc::new(Node::new(
                ElementBase::new(NodeId::new(3), Placement::new(0.0, 0.0, 50.0, 25.0)),
                NodeKind::Text(TextNode::plain("Hola", &style())),
            )),
        );
        page.roots = vec![NodeId::new(2), NodeId::new(3)];
        let doc = Document::from_parts(4, Dpi::new(96), vec![Arc::new(page)]).expect("a document");

        // Onto 200 × 120: twice as large, with 10 px above and below.
        let pages = fitted(&doc, 200.0, 120.0);
        assert_eq!(pages.len(), 1);
        let rect = pages[0].node(NodeId::new(2)).expect("the rectangle");
        assert_eq!(rect.base.placement, Placement::new(20.0, 30.0, 40.0, 20.0));
        let NodeKind::Shape(shape) = &rect.kind else {
            panic!("a shape");
        };
        assert_eq!(shape.corner_radius, 8.0);
        assert_eq!(shape.stroke.map(|s| s.width), Some(4.0));
        let NodeKind::Text(text) = &pages[0].node(NodeId::new(3)).expect("text").kind else {
            panic!("a text");
        };
        assert_eq!(text.content.first_style().map(|s| s.size), Some(40.0));
    }

    fn shape_colour() -> gafira_color::Color {
        gafira_color::Color::Srgb(gafira_color::Rgba::new(1.0, 0.0, 0.0, 1.0))
    }
}
