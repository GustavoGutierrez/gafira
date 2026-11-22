//! Elementos → Foto collage (spec 066, S66-09, D-106): layouts of photo
//! frames that, when added, fill the whole active page (owner request);
//! afterwards the group scales like any other.
//!
//! A layout is written as splits: `x` is one cell, `h(...)` lays its parts
//! side by side and `v(...)` stacks them, each part with its weight before
//! a colon (`h(2:x,1:v(1:x,1:x))`); a part with no colon is one cell
//! (`h(1,1)` is two columns).

use egui::{Color32, Pos2, Rect, Vec2};
use gafira_doc::{FrameNode, GroupNode, NodeKind, Placement, ShapeGeometry};
use gafira_edit::NodeSpec;

/// The layouts of the panel, in its order (after the owner's reference).
pub(crate) const LAYOUTS: [&str; 39] = [
    "x",
    "h(1,1)",
    "v(1,1)",
    "v(2,1)",
    "v(1,2)",
    "h(2,1)",
    "h(1,2)",
    "h(1,1,1)",
    "v(1,1,1)",
    "v(1:h(1,1),1:h(1,1))",
    "v(1:x,1:h(1,1))",
    "h(1:x,1:v(1,1))",
    "v(1:h(1,1),1:x)",
    "h(1:v(1,1),1:x)",
    "v(2:x,1:h(1,1,1))",
    "h(2:x,1:v(1,1,1))",
    "h(1:v(1,1,1),2:x)",
    "v(1:h(1,1,1),2:x)",
    "h(2:x,1:v(1,1))",
    "h(1:v(1,1),2:x)",
    "v(1:h(1,1),1:h(1,1),1:h(1,1))",
    "h(1:v(1,1,1),1:v(1,1,1))",
    "h(1,1,1,1)",
    "v(1,1,1,1)",
    "v(1:h(1,1,1),1:h(1,1,1),1:h(1,1,1))",
    "v(1:h(1,1,1),1:h(1,1,1))",
    "h(1:v(1,1),1:v(1,1),1:v(1,1))",
    "v(2:x,1:h(1,1))",
    "h(2:x,1:v(1,1,1,1))",
    "h(1:v(1,1,1,1),2:x)",
    "v(1:h(1,1),2:x,1:h(1,1))",
    "h(1:v(1,1),2:x,1:v(1,1))",
    "v(1:h(2,1),1:h(1,2))",
    "h(1:v(2,1),1:v(1,2))",
    "v(1:h(1,1,1,1),1:h(1,1,1,1))",
    "h(1,1,1,1,1)",
    "v(1,1,1,1,1)",
    "v(3:h(1,1),1:h(1,1,1))",
    "h(3:v(1,1),1:v(1,1,1))",
];

/// The room between two cells, as a share of the page's shorter side.
pub(crate) const GAP: f64 = 0.012;

/// A cell of a layout, in shares of the page: `(x, y, w, h)` in `0..=1`.
pub(crate) type Cell = (f64, f64, f64, f64);

/// The cells of `layout`, in shares of the page, or `None` when it does
/// not read.
pub(crate) fn cells(layout: &str) -> Option<Vec<Cell>> {
    let mut out = Vec::new();
    let mut at = layout.trim();
    split(&mut at, (0.0, 0.0, 1.0, 1.0), &mut out)?;
    at.is_empty().then_some(out)
}

/// Reads one part from the front of `at` into `cell`.
fn split(at: &mut &str, cell: Cell, out: &mut Vec<Cell>) -> Option<()> {
    if let Some(rest) = at.strip_prefix('x') {
        *at = rest;
        out.push(cell);
        return Some(());
    }
    let across = match at.chars().next()? {
        'h' => true,
        'v' => false,
        _ => return None,
    };
    *at = at.get(1..)?.strip_prefix('(')?;
    // The parts: a weight, and a layout after a colon or one cell.
    let mut parts: Vec<(f64, String)> = Vec::new();
    loop {
        let end = at.find(|c: char| !(c.is_ascii_digit() || c == '.'))?;
        let weight: f64 = at[..end].parse().ok()?;
        *at = &at[end..];
        let inner = if let Some(rest) = at.strip_prefix(':') {
            let close = matching_end(rest)?;
            let inner = rest[..close].to_owned();
            *at = &rest[close..];
            inner
        } else {
            "x".to_owned()
        };
        if !(weight.is_finite() && weight > 0.0) {
            return None;
        }
        parts.push((weight, inner));
        match at.chars().next()? {
            ',' => *at = &at[1..],
            ')' => {
                *at = &at[1..];
                break;
            }
            _ => return None,
        }
    }
    let total: f64 = parts.iter().map(|(w, _)| w).sum();
    let (x, y, w, h) = cell;
    let mut offset = 0.0;
    for (weight, inner) in parts {
        let share = weight / total;
        let part = if across {
            (x + w * offset, y, w * share, h)
        } else {
            (x, y + h * offset, w, h * share)
        };
        offset += share;
        let mut inner_at = inner.as_str();
        split(&mut inner_at, part, out)?;
        if !inner_at.is_empty() {
            return None;
        }
    }
    Some(())
}

/// Where the layout that starts `text` ends: at the first `,` or `)` that
/// is not inside its own brackets.
fn matching_end(text: &str) -> Option<usize> {
    let mut depth = 0_i32;
    for (i, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' if depth == 0 => return Some(i),
            ')' => depth -= 1,
            ',' if depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

/// The cells of `layout` on a page of `(w, h)` px, `gap` px apart where two
/// cells meet and touching the page's edges.
pub(crate) fn placements(layout: &str, (w, h): (f64, f64), gap: f64) -> Option<Vec<Placement>> {
    let half = gap / 2.0;
    let inset = |edge: f64| {
        if edge <= 1e-9 || edge >= 1.0 - 1e-9 {
            0.0
        } else {
            half
        }
    };
    Some(
        cells(layout)?
            .into_iter()
            .map(|(x, y, cw, ch)| {
                let (left, top) = (inset(x), inset(y));
                let (right, bottom) = (inset(x + cw), inset(y + ch));
                Placement::new(
                    x * w + left,
                    y * h + top,
                    (cw * w - left - right).max(1.0),
                    (ch * h - top - bottom).max(1.0),
                )
            })
            .collect(),
    )
}

/// `layout` as one group of empty square frames filling a page of `(w, h)`
/// px (owner request), each showing the sample photo until it has one.
pub(crate) fn spec(layout: &str, (w, h): (f64, f64)) -> Option<NodeSpec> {
    let gap = GAP * w.min(h);
    let children = placements(layout, (w, h), gap)?
        .into_iter()
        .map(|placement| {
            NodeSpec::new(
                NodeKind::Frame(FrameNode::new(ShapeGeometry::Rect)),
                placement,
            )
        })
        .collect();
    Some(
        NodeSpec::new(
            NodeKind::Group(GroupNode {}),
            Placement::new(0.0, 0.0, w, h),
        )
        .with_children(children),
    )
}

/// The preview of `layout` in `rect`: each cell shows the sample photo of
/// the frames (`assets/samples/frame-sample.png`, owner request), cut to
/// cover it; `sample` is its texture and its size in px.
pub(crate) fn paint(
    painter: &egui::Painter,
    rect: Rect,
    layout: &str,
    sample: Option<(egui::TextureId, Vec2)>,
) {
    let Some(cells) = placements(
        layout,
        (f64::from(rect.width()), f64::from(rect.height())),
        3.0,
    ) else {
        return;
    };
    for cell in cells {
        let at = Rect::from_min_size(
            rect.min + Vec2::new(cell.x as f32, cell.y as f32),
            Vec2::new(cell.w as f32, cell.h as f32),
        );
        match sample {
            Some((texture, size)) => {
                painter.image(texture, at, cover(size, at.size()), Color32::WHITE);
            }
            None => {
                painter.rect_filled(at, 0.0, Color32::from_rgb(0xD6, 0xEF, 0xFB));
            }
        }
    }
}

/// The part of an image of `size` that covers a box of `into`, centred.
fn cover(size: Vec2, into: Vec2) -> Rect {
    if size.x <= 0.0 || size.y <= 0.0 || into.x <= 0.0 || into.y <= 0.0 {
        return Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    }
    let (image, cell) = (size.x / size.y, into.x / into.y);
    if image > cell {
        let w = cell / image;
        Rect::from_min_max(
            Pos2::new((1.0 - w) / 2.0, 0.0),
            Pos2::new((1.0 + w) / 2.0, 1.0),
        )
    } else {
        let h = image / cell;
        Rect::from_min_max(
            Pos2::new(0.0, (1.0 - h) / 2.0),
            Pos2::new(1.0, (1.0 + h) / 2.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // S66-09: every layout reads, and its cells cover the page once, with
    // no overlap.
    #[test]
    fn s66_09_every_layout_covers_the_page() {
        for layout in LAYOUTS {
            let cells = cells(layout).unwrap_or_else(|| panic!("{layout} reads"));
            let area: f64 = cells.iter().map(|(_, _, w, h)| w * h).sum();
            assert!((area - 1.0).abs() < 1e-9, "{layout}: {area}");
            for (i, a) in cells.iter().enumerate() {
                for b in &cells[i + 1..] {
                    let across = (a.0 + a.2).min(b.0 + b.2) - a.0.max(b.0);
                    let down = (a.1 + a.3).min(b.1 + b.3) - a.1.max(b.1);
                    assert!(across <= 1e-9 || down <= 1e-9, "{layout}: overlap");
                }
            }
        }
        assert!(cells("h(1,").is_none());
        assert!(cells("q").is_none());
        assert!(cells("x x").is_none());
    }

    // S66-09: a layout of two over two is four frames that fill the page,
    // touching its edges, with the gap between them (owner request).
    #[test]
    fn s66_09_a_collage_fills_the_page() {
        let spec = spec("v(1:h(1,1),1:h(1,1))", (1000.0, 800.0)).expect("a spec");
        assert!(matches!(spec.kind, NodeKind::Group(_)));
        assert_eq!(
            (
                spec.placement.x,
                spec.placement.y,
                spec.placement.w,
                spec.placement.h
            ),
            (0.0, 0.0, 1000.0, 800.0)
        );
        assert_eq!(spec.children.len(), 4);
        assert!(
            spec.children
                .iter()
                .all(|c| matches!(c.kind, NodeKind::Frame(_)))
        );
        let gap = GAP * 800.0;
        let first = spec.children[0].placement;
        assert_eq!((first.x, first.y), (0.0, 0.0), "touches the corner");
        assert!((first.w - (500.0 - gap / 2.0)).abs() < 1e-9);
        let last = spec.children[3].placement;
        assert!(
            (last.x + last.w - 1000.0).abs() < 1e-9,
            "touches the right edge"
        );
        assert!((last.y + last.h - 800.0).abs() < 1e-9, "touches the bottom");
        assert!((last.x - (500.0 + gap / 2.0)).abs() < 1e-9);
    }
}
