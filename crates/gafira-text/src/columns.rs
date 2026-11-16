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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every split of `n` lines into `count` runs (empty runs allowed), as exclusive ends.
    fn splits(n: usize, count: usize) -> Vec<Vec<usize>> {
        if count == 1 {
            return vec![vec![n]];
        }
        let mut out = Vec::new();
        for first in 0..=n {
            for rest in splits(n - first, count - 1) {
                let mut ends = vec![first];
                ends.extend(rest.iter().map(|e| e + first));
                out.push(ends);
            }
        }
        out
    }

    /// FD-45 by exhaustive search: the smallest spread within `limit`, and among equal
    /// spreads the larger ends. The heights are multiples of 0.5, so every sum is exact and
    /// a tie is an exact tie.
    fn exhaustive(heights: &[f64], count: usize, limit: f64) -> Vec<usize> {
        let mut best: Option<(f64, Vec<usize>)> = None;
        for ends in splits(heights.len(), count) {
            let cols = split_heights(heights, &ends);
            if cols.iter().any(|h| *h > limit) {
                continue;
            }
            let d = cols.iter().copied().fold(f64::MIN, f64::max)
                - cols.iter().copied().fold(f64::MAX, f64::min);
            let better = best
                .as_ref()
                .is_none_or(|(bd, be)| d < *bd || (d == *bd && ends > *be));
            if better {
                best = Some((d, ends));
            }
        }
        best.expect("the filled split fits").1
    }

    /// All the sequences of up to `len` heights drawn from `alphabet`.
    fn sequences(alphabet: &[f64], len: usize) -> Vec<Vec<f64>> {
        let mut all = vec![Vec::new()];
        let mut level = vec![Vec::new()];
        for _ in 0..len {
            level = level
                .iter()
                .flat_map(|s: &Vec<f64>| {
                    alphabet.iter().map(move |h| {
                        let mut t = s.clone();
                        t.push(*h);
                        t
                    })
                })
                .collect();
            all.extend(level.iter().cloned());
        }
        all
    }

    // S13-01, FD-45: the balanced split is the exhaustive optimum, ties included, for every
    // sequence of up to seven lines of three heights, two to four columns, with and without a
    // limit.
    #[test]
    fn fd_45_balance_matches_an_exhaustive_search() {
        let mut checked = 0;
        for heights in sequences(&[1.0, 2.0, 3.5], 7) {
            let total: f64 = heights.iter().sum();
            for count in 2..=4 {
                let mean = total / count as f64;
                for limit in [f64::INFINITY, (mean + 1.0).ceil(), (mean + 3.0).ceil()] {
                    let filled = fill(&heights, count, limit);
                    if split_heights(&heights, &filled).iter().any(|h| *h > limit) {
                        continue;
                    }
                    assert_eq!(
                        balance(&heights, count, limit),
                        exhaustive(&heights, count, limit),
                        "{heights:?} in {count} within {limit}"
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 10_000, "{checked}");
    }

    // S13-01, FD-45: a tie on the spread goes to the larger ends, whichever split the search
    // meets first; and a spread that is better by less than the tie tolerance is still a tie.
    #[test]
    fn fd_45_ties_go_to_the_larger_ends() {
        // [1, 1] | [2] and [1] | [1, 2]... only the first has spread 0.
        assert_eq!(balance(&[1.0, 1.0, 2.0], 2, f64::INFINITY), vec![2, 3]);
        // Spread 1 both ways: [2] | [1, 2] and [2, 1] | [2]; the larger ends win.
        assert_eq!(balance(&[2.0, 1.0, 2.0], 2, f64::INFINITY), vec![2, 3]);
        // Within the tolerance: 1 + 1e-12 against 1 is a tie, so the larger ends win.
        assert_eq!(
            balance(&[1.0 + 1e-12, 1.0, 1.0], 2, f64::INFINITY),
            vec![2, 3]
        );
    }
}
