//! Columns (FD-45): equal-width columns that split a text by the height of the box, or
//! balance their heights.

use gafira_geom::Size;

use crate::layout::{check_input_with, in_range};
use crate::{Line, TextError, TextStyle, layout_box};

/// The columns of a box (02 §3.2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextColumns {
    /// Number of columns, from 1 to 8. With 1 there are no columns, and `gap` is checked and has no effect.
    pub count: u32,
    /// Space between two columns, in px, in `[0, 200]`.
    pub gap: f64,
    /// Balance the heights of the columns. Not needed with [`ColumnSizing::AutoHeight`],
    /// which always balances.
    pub balance: bool,
}

/// How the height of the box is decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnSizing {
    /// The box has the height of `bounds`.
    Fixed,
    /// The height comes out of the result; `bounds.height` is not used.
    AutoHeight,
}

/// One column of lines.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Column {
    /// Left edge from the left of the box, in px.
    pub x: f32,
    /// Width, in px.
    pub width: f32,
    /// Height of its lines, in px.
    pub height: f32,
    /// The lines. Cluster `x` counts from the left of the box and baselines from the top of
    /// the box, so every column starts at the top.
    pub lines: Vec<Line>,
}

/// The text laid out in columns, without pixels.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ColumnsLayout {
    /// The columns, left to right. The text reads down each one in turn.
    pub columns: Vec<Column>,
    /// Height of the tallest column, in px.
    pub height: f32,
    /// True when the text does not fit: a cluster wider than its column (FD-41), or, in a
    /// fixed box, a column taller than the box. No text is lost.
    pub overflow: bool,
}

/// Tolerance of a tie between two differences of heights, in px.
const TIE: f64 = 1e-9;

/// Lays `text` out in `columns.count` columns of equal width, `(width - (count - 1) gap) /
/// count`, which must be at least 1 px (a single column has no such limit).
///
/// The numbers are checked first, as in [`layout_box`], then the count, the gap and the width
/// of a column. The text is wrapped once at the width of a column. In a fixed box without
/// `balance`, each column takes lines while their heights add up to at most the height of the
/// box (equality fits), a column always takes one line, and what remains goes to the last
/// column. With `balance`, or with [`ColumnSizing::AutoHeight`], the lines are split into
/// `count` runs of the smallest difference between the tallest and the shortest column; among
/// equal splits, the earlier columns take more lines. A fixed box that cannot hold the text
/// is filled as without `balance`. One column is the layout of [`layout_box`], bit for bit.
pub fn layout_columns(
    text: &str,
    fonts: &[&[u8]],
    style: &TextStyle,
    dpi: f64,
    bounds: Size,
    columns: &TextColumns,
    sizing: ColumnSizing,
) -> Result<ColumnsLayout, TextError> {
    check_input_with(style, dpi, bounds, &[("columns.gap", columns.gap)])?;
    if !(1..=8).contains(&columns.count) {
        return Err(TextError::OutOfRange {
            field: "columns.count",
        });
    }
    in_range("columns.gap", columns.gap, (0.0, 200.0))?;
    let count = columns.count as usize;
    let width = if count == 1 {
        bounds.width()
    } else {
        (bounds.width() - (count - 1) as f64 * columns.gap) / count as f64
    };
    if count > 1 && width < 1.0 {
        return Err(TextError::OutOfRange {
            field: "columns.width",
        });
    }
    let laid = layout_box(text, fonts, style, dpi, Size::new(width, bounds.height()))?;
    let heights: Vec<f64> = laid.lines.iter().map(|l| f64::from(l.height)).collect();
    let limit = match sizing {
        ColumnSizing::Fixed => bounds.height(),
        ColumnSizing::AutoHeight => f64::INFINITY,
    };
    let mut ends = fill(&heights, count, limit);
    let fits = split_heights(&heights, &ends).iter().all(|h| *h <= limit);
    if fits && (columns.balance || sizing == ColumnSizing::AutoHeight) {
        ends = balance(&heights, count, limit);
    }

    let mut out = Vec::with_capacity(count);
    let mut lines = laid.lines.into_iter();
    let mut top = 0.0_f64;
    let mut start = 0;
    let mut overflow = laid.overflow;
    for (j, end) in ends.iter().enumerate() {
        let x = j as f64 * (width + columns.gap);
        let mut column: Vec<Line> = lines.by_ref().take(end - start).collect();
        let height: f64 = heights[start..*end].iter().sum();
        for line in &mut column {
            line.baseline = (f64::from(line.baseline) - top) as f32;
            for c in &mut line.clusters {
                c.x = (f64::from(c.x) + x) as f32;
            }
        }
        overflow |= height > limit;
        out.push(Column {
            x: x as f32,
            width: width as f32,
            height: height as f32,
            lines: column,
        });
        top += height;
        start = *end;
    }
    let height = out.iter().map(|c| c.height).fold(0.0, f32::max);
    Ok(ColumnsLayout {
        columns: out,
        height,
        overflow,
    })
}

/// The height of each run of lines; `ends` are the exclusive ends of the columns.
fn split_heights(heights: &[f64], ends: &[usize]) -> Vec<f64> {
    let mut start = 0;
    ends.iter()
        .map(|end| {
            let h = heights[start..*end].iter().sum();
            start = *end;
            h
        })
        .collect()
}

/// Fills each column to `limit`; the last one takes what remains. Returns the end of each.
fn fill(heights: &[f64], count: usize, limit: f64) -> Vec<usize> {
    let mut ends = Vec::with_capacity(count);
    let mut start = 0;
    for j in 0..count {
        let mut end = start;
        let mut sum = 0.0;
        if j + 1 == count {
            end = heights.len();
        } else {
            while end < heights.len() && (end == start || sum + heights[end] <= limit) {
                sum += heights[end];
                end += 1;
            }
        }
        ends.push(end);
        start = end;
    }
    ends
}

/// The split into `count` runs with the smallest difference between the tallest and the
/// shortest column, each within `limit`. Branch and bound over the ends of the columns, the
/// ones nearest to the mean height first; a tie goes to the larger ends, so the earlier
/// columns take more lines.
fn balance(heights: &[f64], count: usize, limit: f64) -> Vec<usize> {
    let mut prefix = Vec::with_capacity(heights.len() + 1);
    prefix.push(0.0);
    for h in heights {
        prefix.push(prefix[prefix.len() - 1] + h);
    }
    let total = prefix[heights.len()];
    // The filled split is a split within `limit` when the caller found that the text fits.
    let filled = fill(heights, count, limit);
    let spread = split_heights(heights, &filled);
    let d = spread.iter().copied().fold(f64::MIN, f64::max)
        - spread.iter().copied().fold(f64::MAX, f64::min);
    let mut search = Search {
        prefix: &prefix,
        count,
        limit,
        mean: total / count as f64,
        best: (d, filled),
    };
    search.go(0, &mut Vec::with_capacity(count), (f64::MAX, f64::MIN));
    search.best.1
}

struct Search<'a> {
    prefix: &'a [f64],
    count: usize,
    limit: f64,
    mean: f64,
    best: (f64, Vec<usize>),
}

impl Search<'_> {
    /// Tries the ends of the next column, which starts at line `start`; `ends` are the ends
    /// chosen so far and `(low, high)` the shortest and the tallest column among them.
    fn go(&mut self, start: usize, ends: &mut Vec<usize>, (low, high): (f64, f64)) {
        let n = self.prefix.len() - 1;
        let bound = self.best.0 + TIE;
        if ends.len() + 1 == self.count {
            let h = self.prefix[n] - self.prefix[start];
            if h <= self.limit {
                self.offer(ends, n, (low.min(h), high.max(h)));
            }
            return;
        }
        if high - low > bound {
            return;
        }
        // The heights that can still tie or beat the best are an interval: within the bound of
        // the mean (a split with a smaller spread has every column that close to it), within the
        // bound of the columns chosen, and not over the limit. Heights grow with the end.
        let upper = self.limit.min(self.mean + bound).min(low + bound);
        let lower = (self.mean - bound).max(high - bound);
        let base = self.prefix[start];
        let window = &self.prefix[start..=n];
        let first = start + window.partition_point(|p| p - base < lower);
        let end = start + window.partition_point(|p| p - base <= upper);
        let mut candidates: Vec<(f64, usize)> = (first..end)
            .map(|e| ((self.prefix[e] - base - self.mean).abs(), e))
            .collect();
        candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)));
        for (_, e) in candidates {
            let h = self.prefix[e] - self.prefix[start];
            ends.push(e);
            self.go(e, ends, (low.min(h), high.max(h)));
            ends.pop();
        }
    }

    fn offer(&mut self, ends: &mut Vec<usize>, last: usize, (low, high): (f64, f64)) {
        let d = high - low;
        ends.push(last);
        let (best, best_ends) = &self.best;
        if d < best - TIE || ((d - best).abs() <= TIE && *ends > *best_ends) {
            self.best = (d, ends.clone());
        }
        ends.pop();
    }
}
