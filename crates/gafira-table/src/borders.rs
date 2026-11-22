//! The borders of the cells.

use gafira_doc::{ShapeStroke, TableNode};
use kurbo::Point;

use crate::{Layout, Range, anchors};

/// Which edges of a range of cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BorderSet {
    All,
    Outer,
    Inner,
    Top,
    Horizontal,
    Bottom,
    Left,
    Vertical,
    Right,
}

impl BorderSet {
    /// In the order of the menu: three by three.
    pub const ALL: [Self; 9] = [
        Self::All,
        Self::Outer,
        Self::Inner,
        Self::Top,
        Self::Horizontal,
        Self::Bottom,
        Self::Left,
        Self::Vertical,
        Self::Right,
    ];

    /// Whether it has the horizontal edges `(first, inner, last)`.
    fn horizontal(self) -> (bool, bool, bool) {
        match self {
            Self::All => (true, true, true),
            Self::Outer => (true, false, true),
            Self::Inner | Self::Horizontal => (false, true, false),
            Self::Top => (true, false, false),
            Self::Bottom => (false, false, true),
            Self::Left | Self::Vertical | Self::Right => (false, false, false),
        }
    }

    /// Whether it has the vertical edges `(first, inner, last)`.
    fn vertical(self) -> (bool, bool, bool) {
        match self {
            Self::All => (true, true, true),
            Self::Outer => (true, false, true),
            Self::Inner | Self::Vertical => (false, true, false),
            Self::Left => (true, false, false),
            Self::Right => (false, false, true),
            Self::Top | Self::Horizontal | Self::Bottom => (false, false, false),
        }
    }
}

/// Sets the edges `which` of `range`, or takes them off. Without room
/// between the cells, an edge two of them share is one: it changes in both.
pub fn set_borders(t: &mut TableNode, range: Range, which: BorderSet, stroke: Option<ShapeStroke>) {
    let range = range.over(t);
    let (top, across, bottom) = which.horizontal();
    let (left, down, right) = which.vertical();
    let shared = t.spacing == 0.0;
    for (r, c) in range.cells() {
        let edges = [
            (0, if r == range.r0 { top } else { across }),
            (1, if c == range.c1 { right } else { down }),
            (2, if r == range.r1 { bottom } else { across }),
            (3, if c == range.c0 { left } else { down }),
        ];
        for (edge, on) in edges {
            if !on {
                continue;
            }
            set_edge(t, (r, c), edge, stroke);
            if shared && let Some((next, opposite)) = across_edge((r, c), edge) {
                set_edge(t, next, opposite, stroke);
            }
        }
    }
}

/// The cell on the other side of an edge, and the edge as it sees it.
fn across_edge((r, c): (usize, usize), edge: u8) -> Option<((usize, usize), u8)> {
    match edge {
        0 => Some(((r.checked_sub(1)?, c), 2)),
        1 => Some(((r, c + 1), 3)),
        2 => Some(((r + 1, c), 0)),
        _ => Some(((r, c.checked_sub(1)?), 1)),
    }
}

fn set_edge(t: &mut TableNode, (r, c): (usize, usize), edge: u8, stroke: Option<ShapeStroke>) {
    if let Some(cell) = t.cell_mut(r, c) {
        let borders = &mut cell.style.borders;
        match edge {
            0 => borders.top = stroke,
            1 => borders.right = stroke,
            2 => borders.bottom = stroke,
            _ => borders.left = stroke,
        }
    }
}

/// A border drawn from `a` to `b`, in px of the box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Edge {
    pub a: Point,
    pub b: Point,
    pub stroke: ShapeStroke,
}

/// The borders of `t` as they are drawn, the ones of a line joined.
#[must_use]
pub fn edges(t: &TableNode, layout: &Layout) -> Vec<Edge> {
    let mut out = Vec::new();
    if t.spacing > 0.0 {
        for (r, c) in anchors(t) {
            let Some(cell) = t.cell(r, c) else { continue };
            let box_of = layout.cell(t, r, c);
            let (tl, br) = (box_of.origin(), Point::new(box_of.x1, box_of.y1));
            let (tr, bl) = (Point::new(br.x, tl.y), Point::new(tl.x, br.y));
            let b = cell.style.borders;
            for (stroke, from, to) in [
                (b.top, tl, tr),
                (b.right, tr, br),
                (b.bottom, bl, br),
                (b.left, tl, bl),
            ] {
                push(&mut out, stroke, from, to);
            }
        }
        return out;
    }
    let (rows, cols) = (t.rows.len(), t.columns.len());
    let same = |a: (usize, usize), b: (usize, usize)| {
        t.merge_at(a.0, a.1).is_some_and(|m| m.covers(b.0, b.1))
    };
    let y_of = |r: usize| layout.rows.get(r).map(|s| s.0);
    let x_of = |c: usize| layout.columns.get(c).map(|s| s.0);
    let (bottom, right) = (
        layout.rows.last().map_or(0.0, |s| s.1),
        layout.columns.last().map_or(0.0, |s| s.1),
    );
    let borders = |r: usize, c: usize| t.cell(r, c).map(|cell| cell.style.borders);
    for r in 0..=rows {
        let y = y_of(r).unwrap_or(bottom);
        for c in 0..cols {
            let over = r.checked_sub(1);
            if over.is_some_and(|over| r < rows && same((over, c), (r, c))) {
                continue;
            }
            let below = (r < rows).then(|| borders(r, c)).flatten();
            let above = over.and_then(|over| borders(over, c));
            let stroke = below.and_then(|b| b.top).or(above.and_then(|b| b.bottom));
            let (x0, x1) = layout.columns[c];
            push(&mut out, stroke, Point::new(x0, y), Point::new(x1, y));
        }
    }
    for c in 0..=cols {
        let x = x_of(c).unwrap_or(right);
        for r in 0..rows {
            let before = c.checked_sub(1);
            if before.is_some_and(|before| c < cols && same((r, before), (r, c))) {
                continue;
            }
            let after = (c < cols).then(|| borders(r, c)).flatten();
            let prior = before.and_then(|before| borders(r, before));
            let stroke = after.and_then(|b| b.left).or(prior.and_then(|b| b.right));
            let (y0, y1) = layout.rows[r];
            push(&mut out, stroke, Point::new(x, y0), Point::new(x, y1));
        }
    }
    out
}

/// Adds a border, joined to the last one when it goes on from it.
fn push(out: &mut Vec<Edge>, stroke: Option<ShapeStroke>, a: Point, b: Point) {
    let Some(stroke) = stroke.filter(|s| s.width > 0.0) else {
        return;
    };
    if let Some(last) = out.last_mut()
        && last.stroke == stroke
        && (last.b - a).hypot() < 1e-6
        && ((last.a.x == b.x && last.a.x == last.b.x) || (last.a.y == b.y && last.a.y == last.b.y))
    {
        last.b = b;
        return;
    }
    out.push(Edge { a, b, stroke });
}
