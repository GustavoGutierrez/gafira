//! Where the cells of a table lie in its box.

use gafira_doc::TableNode;
use kurbo::Rect;

/// Cells from `(r0, c0)` to `(r1, c1)`, both in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub r0: usize,
    pub c0: usize,
    pub r1: usize,
    pub c1: usize,
}

impl Range {
    /// The cells between two corners, in any order.
    #[must_use]
    pub fn new(a: (usize, usize), b: (usize, usize)) -> Self {
        Self {
            r0: a.0.min(b.0),
            c0: a.1.min(b.1),
            r1: a.0.max(b.0),
            c1: a.1.max(b.1),
        }
    }

    /// One cell.
    #[must_use]
    pub fn cell(row: usize, col: usize) -> Self {
        Self::new((row, col), (row, col))
    }

    /// Every cell of `t`.
    #[must_use]
    pub fn all(t: &TableNode) -> Self {
        Self::new(
            (0, 0),
            (
                t.rows.len().saturating_sub(1),
                t.columns.len().saturating_sub(1),
            ),
        )
    }

    #[must_use]
    pub fn contains(&self, row: usize, col: usize) -> bool {
        (self.r0..=self.r1).contains(&row) && (self.c0..=self.c1).contains(&col)
    }

    /// Its cells, row by row.
    pub fn cells(self) -> impl Iterator<Item = (usize, usize)> {
        (self.r0..=self.r1).flat_map(move |r| (self.c0..=self.c1).map(move |c| (r, c)))
    }

    /// The range grown to take the whole of every merge it touches, within
    /// `t`.
    #[must_use]
    pub fn over(self, t: &TableNode) -> Self {
        let all = Self::all(t);
        let mut range = Self {
            r0: self.r0.min(all.r1),
            c0: self.c0.min(all.c1),
            r1: self.r1.min(all.r1),
            c1: self.c1.min(all.c1),
        };
        loop {
            let before = range;
            for m in &t.merges {
                let (r1, c1) = (m.row + m.rows - 1, m.col + m.cols - 1);
                let touches =
                    m.row <= range.r1 && range.r0 <= r1 && m.col <= range.c1 && range.c0 <= c1;
                if touches {
                    range = Self {
                        r0: range.r0.min(m.row),
                        c0: range.c0.min(m.col),
                        r1: range.r1.max(r1),
                        c1: range.c1.max(c1),
                    };
                }
            }
            if range == before {
                return range;
            }
        }
    }
}

/// The cells that are drawn: every one but those a merge hides.
pub fn anchors(t: &TableNode) -> impl Iterator<Item = (usize, usize)> + '_ {
    Range::all(t).cells().filter(|(r, c)| {
        t.merge_at(*r, *c)
            .is_none_or(|m| (m.row, m.col) == (*r, *c))
    })
}

/// Where each column and row starts and ends, in px of the box.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub columns: Vec<(f64, f64)>,
    pub rows: Vec<(f64, f64)>,
}

/// Where each of `sizes` starts and ends in `length`, `gap` between them.
fn spans(sizes: &[f64], length: f64, gap: f64) -> Vec<(f64, f64)> {
    let count = sizes.len().max(1) as f64;
    // The room never takes more than half of the box.
    let gap = gap.min(length / (2.0 * count)).max(0.0);
    let total: f64 = sizes.iter().sum();
    let scale = if total > 0.0 {
        (length - gap * (count - 1.0)) / total
    } else {
        0.0
    };
    let mut at = 0.0;
    sizes
        .iter()
        .map(|size| {
            let start = at;
            at += size * scale;
            let span = (start, at);
            at += gap;
            span
        })
        .collect()
}

impl Layout {
    /// The layout of `t` in a box of `w` × `h` px.
    #[must_use]
    pub fn of(t: &TableNode, (w, h): (f64, f64)) -> Self {
        Self {
            columns: spans(&t.columns, w, t.spacing),
            rows: spans(&t.rows, h, t.spacing),
        }
    }

    /// The box of a cell, or of the merge it starts.
    #[must_use]
    pub fn cell(&self, t: &TableNode, row: usize, col: usize) -> Rect {
        let (rows, cols) = t.merge_at(row, col).map_or((1, 1), |m| (m.rows, m.cols));
        self.range(Range::new((row, col), (row + rows - 1, col + cols - 1)))
    }

    /// The box around `range`.
    #[must_use]
    pub fn range(&self, range: Range) -> Rect {
        let side = |list: &[(f64, f64)], from: usize, to: usize| {
            let last = list.len().saturating_sub(1);
            let start = list.get(from.min(last)).map_or(0.0, |s| s.0);
            (start, list.get(to.min(last)).map_or(0.0, |s| s.1))
        };
        let (x0, x1) = side(&self.columns, range.c0, range.c1);
        let (y0, y1) = side(&self.rows, range.r0, range.r1);
        Rect::new(x0, y0, x1, y1)
    }

    /// The cell under a point of the box (the first one of a merge).
    #[must_use]
    pub fn at(&self, t: &TableNode, x: f64, y: f64) -> Option<(usize, usize)> {
        // The room between two cells belongs to the one after it.
        let find = |list: &[(f64, f64)], v: f64| {
            if v < 0.0 || list.last().is_none_or(|last| v > last.1) {
                return None;
            }
            list.iter().position(|span| v <= span.1)
        };
        let (row, col) = (find(&self.rows, y)?, find(&self.columns, x)?);
        Some(t.merge_at(row, col).map_or((row, col), |m| (m.row, m.col)))
    }

    /// The column whose right side is within `slack` of `x`.
    #[must_use]
    pub fn column_edge(&self, x: f64, slack: f64) -> Option<usize> {
        self.columns.iter().position(|s| (s.1 - x).abs() <= slack)
    }

    /// The row whose bottom is within `slack` of `y`.
    #[must_use]
    pub fn row_edge(&self, y: f64, slack: f64) -> Option<usize> {
        self.rows.iter().position(|s| (s.1 - y).abs() <= slack)
    }
}
