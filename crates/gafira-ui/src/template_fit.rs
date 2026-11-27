//! A template on a page of the editor (spec 091): which templates suit the
//! page's proportions, and the template's pages fitted to the page, whole
//! and centred, with their letters, outlines and corners scaled as well.

use gafira_doc::{Document, Dpi, Guide, GuideAxis, NodeKind, Page, PageSize};
use gafira_geom::{Length, Unit};

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

/// `page` at `to` (spec 108, «Redimensionar el diseño»): its elements
/// scaled by the largest factor that keeps what fitted on it whole and
/// centred, with their letters, outlines, corners and lines; its guides
/// moved the same; margins, bleed, background and grid kept; the ids the
/// same. `None` when either size cannot be measured.
pub(crate) fn resized_page(page: &Page, dpi: Dpi, to: PageSize) -> Option<Page> {
    let from = page.size.to_px(dpi).ok()?;
    let into = to.to_px(dpi).ok()?;
    let (w, h, width, height) = (from.width(), from.height(), into.width(), into.height());
    if !(w > 0.0 && h > 0.0 && width > 0.0 && height > 0.0) {
        return None;
    }
    let s = (width / w).min(height / h);
    let (dx, dy) = ((width - w * s) / 2.0, (height - h * s) / 2.0);
    let mut out = scaled_page(page, s, dx, dy);
    out.size = to;
    let ppi = f64::from(dpi.get());
    for guide in &mut out.guides.0 {
        let at = guide.position.to_px(ppi).ok()?;
        let shift = match guide.axis {
            GuideAxis::Vertical => dx,
            GuideAxis::Horizontal => dy,
        };
        *guide = Guide {
            position: Length::new(at * s + shift, Unit::Px),
            ..*guide
        };
    }
    Some(out)
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
            gradient: None,
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

    // S108-04: a 100 × 50 page onto 300 × 300: three times as large,
    // centred with 75 px above and below; letters, outline and corners
    // scaled; a guide moved the same; the ids kept.
    #[test]
    fn s108_04_a_page_is_fitted_to_its_new_size() {
        let px = |v: f64| Length::new(v, Unit::Px);
        let size = PageSize {
            width: px(100.0),
            height: px(50.0),
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
        page.guides.0.push(Guide {
            axis: GuideAxis::Horizontal,
            position: px(10.0),
            locked: false,
        });
        page.guides.0.push(Guide {
            axis: GuideAxis::Vertical,
            position: Length::new(1.0, Unit::In),
            locked: true,
        });
        let to = PageSize {
            width: px(300.0),
            height: px(300.0),
        };
        let out = resized_page(&page, Dpi::new(96), to).expect("fitted");
        assert_eq!(out.size, to);
        assert_eq!(out.id, page.id);
        assert_eq!(out.roots, page.roots);
        let rect = out.node(NodeId::new(2)).expect("the rectangle");
        assert_eq!(rect.base.placement, Placement::new(30.0, 105.0, 60.0, 30.0));
        let NodeKind::Shape(shape) = &rect.kind else {
            panic!("a shape");
        };
        assert_eq!(shape.corner_radius, 12.0);
        assert_eq!(shape.stroke.map(|s| s.width), Some(6.0));
        let NodeKind::Text(text) = &out.node(NodeId::new(3)).expect("text").kind else {
            panic!("a text");
        };
        assert_eq!(text.content.first_style().map(|s| s.size), Some(60.0));
        assert_eq!(out.guides.0[0].position, px(105.0));
        assert_eq!(out.guides.0[1].position, px(288.0));
        assert!(out.guides.0[1].locked);
        // A narrower page: the width drives and the content is centred
        // across.
        let tall = PageSize {
            width: px(50.0),
            height: px(100.0),
        };
        let out = resized_page(&page, Dpi::new(96), tall).expect("fitted");
        let rect = out.node(NodeId::new(2)).expect("the rectangle");
        assert_eq!(rect.base.placement, Placement::new(5.0, 42.5, 10.0, 5.0));
        // A size that cannot be measured.
        let none = PageSize {
            width: px(0.0),
            height: px(100.0),
        };
        assert!(resized_page(&page, Dpi::new(96), none).is_none());
    }

    fn shape_colour() -> gafira_color::Color {
        gafira_color::Color::Srgb(gafira_color::Rgba::new(1.0, 0.0, 0.0, 1.0))
    }
}
