//! The colours of a group, and changing one of them everywhere in it
//! (spec 067: an SVG brought in as vector shapes is a group).

use gafira_color::{Color, ColorSpaceId};
use gafira_doc::{NodeId, NodeKind, Page, ShapeStroke};
use gafira_edit::Property;

/// A colour as 8-bit sRGB, to tell two colours apart.
fn key(color: Color) -> [u8; 4] {
    match color.convert(ColorSpaceId::Srgb) {
        Color::Srgb(c) => [c.r, c.g, c.b, c.a].map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8),
        _ => [0; 4],
    }
}

/// Whether two colours are the same to the eye (8-bit sRGB).
pub(crate) fn same(a: Color, b: Color) -> bool {
    key(a) == key(b)
}

/// Every shape below `root`, `root` included, with its id.
fn shapes(page: &Page, root: NodeId) -> Vec<(NodeId, &gafira_doc::ShapeNode)> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        let Some(node) = page.node(id) else { continue };
        if let NodeKind::Shape(shape) = &node.kind {
            out.push((id, shape));
        }
        stack.extend(node.children.iter().rev());
    }
    out
}

/// The distinct colours of the shapes below `root`: fills, outlines and
/// gradient stops, in the order they are met.
pub(crate) fn colors(page: &Page, root: NodeId) -> Vec<Color> {
    let mut out: Vec<Color> = Vec::new();
    let mut add = |c: Color| {
        if !out.iter().any(|kept| same(*kept, c)) {
            out.push(c);
        }
    };
    for (_, shape) in shapes(page, root) {
        match &shape.gradient {
            Some(gradient) => gradient.stops.iter().for_each(|s| add(s.color)),
            None => shape.fill.into_iter().for_each(&mut add),
        }
        if let Some(stroke) = shape.stroke {
            add(stroke.color);
        }
    }
    out
}

/// The changes that turn `old` into `new` in every shape below `root`.
pub(crate) fn recolor(
    page: &Page,
    root: NodeId,
    old: Color,
    new: Color,
) -> Vec<(NodeId, Property)> {
    let mut out = Vec::new();
    for (id, shape) in shapes(page, root) {
        if let Some(gradient) = &shape.gradient {
            if gradient.stops.iter().any(|s| same(s.color, old)) {
                let mut changed = gradient.clone();
                for stop in &mut changed.stops {
                    if same(stop.color, old) {
                        stop.color = new;
                    }
                }
                out.push((id, Property::Gradient(Some(changed))));
            }
        } else if shape.fill.is_some_and(|f| same(f, old)) {
            out.push((id, Property::Fill(Some(new))));
        }
        if let Some(stroke) = shape.stroke
            && same(stroke.color, old)
        {
            out.push((
                id,
                Property::Stroke(Some(ShapeStroke::new(new, stroke.width))),
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gafira_color::Rgba;
    use gafira_doc::{
        ElementBase, GroupNode, Node, PageId, PageSize, Placement, ShapeGeometry, ShapeNode,
    };
    use gafira_geom::{Length, Unit};

    use super::*;

    fn rgb(r: f32, g: f32, b: f32) -> Color {
        Color::Srgb(Rgba::new(r, g, b, 1.0))
    }

    /// A group of a red shape with a blue outline and a red one.
    fn page() -> Page {
        let size = PageSize {
            width: Length::new(100.0, Unit::Px),
            height: Length::new(100.0, Unit::Px),
        };
        let mut page = Page::new(PageId::new(1), String::new(), size);
        let base = |id| ElementBase::new(NodeId::new(id), Placement::new(0.0, 0.0, 10.0, 10.0));
        let mut a = ShapeNode::new(ShapeGeometry::Rect);
        a.fill = Some(rgb(1.0, 0.0, 0.0));
        a.stroke = Some(ShapeStroke::new(rgb(0.0, 0.0, 1.0), 2.0));
        let mut b = ShapeNode::new(ShapeGeometry::Ellipse);
        b.fill = Some(rgb(1.0, 0.0, 0.0));
        let mut group = Node::new(base(2), NodeKind::Group(GroupNode {}));
        group.children = vec![NodeId::new(3), NodeId::new(4)];
        for node in [
            group,
            Node::new(base(3), NodeKind::Shape(a)),
            Node::new(base(4), NodeKind::Shape(b)),
        ] {
            page.nodes.insert(node.base.id, Arc::new(node));
        }
        page.roots = vec![NodeId::new(2)];
        page
    }

    // Spec 067: the colours of a group are found once each.
    #[test]
    fn s67_the_colours_of_a_group() {
        let found = colors(&page(), NodeId::new(2));
        assert_eq!(found.len(), 2);
        assert!(same(found[0], rgb(1.0, 0.0, 0.0)) && same(found[1], rgb(0.0, 0.0, 1.0)));
    }

    // Spec 067: changing a colour changes it in every shape that has it, as
    // a fill or as an outline, and nowhere else.
    #[test]
    fn s67_a_colour_changes_everywhere_in_the_group() {
        let green = rgb(0.0, 1.0, 0.0);
        let changes = recolor(&page(), NodeId::new(2), rgb(1.0, 0.0, 0.0), green);
        assert_eq!(changes.len(), 2, "the two red fills");
        assert!(
            changes
                .iter()
                .all(|(_, p)| matches!(p, Property::Fill(Some(c)) if same(*c, green)))
        );
        let outline = recolor(&page(), NodeId::new(2), rgb(0.0, 0.0, 1.0), green);
        assert!(matches!(
            outline.as_slice(),
            [(id, Property::Stroke(Some(s)))] if *id == NodeId::new(3) && s.width == 2.0
        ));
    }
}
