//! The changes to the rows, columns and cells of a table.

use gafira_doc::{CellMerge, CellStyle, TableCell, TableNode};

use crate::Range;

/// The least size of a row or a column, in px.
pub const MIN_SIZE: f64 = 8.0;

/// Turns the sizes of `t` into px of its box, `w` × `h`, so they can be
/// changed one by one; [`size`] gives the box they ask for.
pub fn to_px(t: &mut TableNode, (w, h): (f64, f64)) {
    let layout = crate::Layout::of(t, (w, h));
    for (size, span) in t.columns.iter_mut().zip(&layout.columns) {
        *size = (span.1 - span.0).max(f64::MIN_POSITIVE);
    }
    for (size, span) in t.rows.iter_mut().zip(&layout.rows) {
        *size = (span.1 - span.0).max(f64::MIN_POSITIVE);
    }
}

/// The box the sizes of `t` ask for, in px.
#[must_use]
pub fn size(t: &TableNode) -> (f64, f64) {
    let side = |sizes: &[f64]| {
        sizes.iter().sum::<f64>() + t.spacing * (sizes.len().saturating_sub(1)) as f64
    };
    (side(&t.columns), side(&t.rows))
}

fn resize(sizes: &mut [f64], index: usize, delta: f64) {
    if let Some(size) = sizes.get_mut(index) {
        *size = (*size + delta).max(MIN_SIZE);
    }
}

/// Makes a column wider or narrower; the others keep their width.
pub fn resize_column(t: &mut TableNode, index: usize, delta: f64) {
    resize(&mut t.columns, index, delta);
}

/// Makes a row taller or shorter; the others keep their height.
pub fn resize_row(t: &mut TableNode, index: usize, delta: f64) {
    resize(&mut t.rows, index, delta);
}

#[derive(Clone, Copy, PartialEq)]
enum Axis {
    Rows,
    Columns,
}

/// A row or a column of the table being made.
#[derive(Clone, Copy, PartialEq)]
enum Line {
    /// The one that was at this index.
    Kept(usize),
    /// A new one with the looks of the one at this index.
    Like(usize),
    /// A new one with the looks and the words of the one at this index.
    Copy(usize),
}

impl Line {
    fn source(self) -> usize {
        match self {
            Self::Kept(i) | Self::Like(i) | Self::Copy(i) => i,
        }
    }
}

/// `t` with its rows or columns made anew as `lines`; `false`, and nothing
/// changed, when that would pass the limits or tear a merge apart.
fn rebuild(t: &mut TableNode, axis: Axis, lines: &[Line]) -> bool {
    let (sizes, most) = match axis {
        Axis::Rows => (&t.rows, TableNode::MAX_ROWS),
        Axis::Columns => (&t.columns, TableNode::MAX_COLUMNS),
    };
    let count = sizes.len();
    if lines.is_empty() || lines.len() > most || lines.iter().any(|l| l.source() >= count) {
        return false;
    }
    let sizes: Vec<f64> = lines.iter().map(|line| sizes[line.source()]).collect();
    let mut merges = Vec::new();
    // Words of merges whose first line goes: (the merge now, its words).
    let mut moved: Vec<(CellMerge, String)> = Vec::new();
    for m in &t.merges {
        let (start, length) = match axis {
            Axis::Rows => (m.row, m.rows),
            Axis::Columns => (m.col, m.cols),
        };
        let inside =
            |line: &Line| matches!(line, Line::Kept(i) if (start..start + length).contains(i));
        let (Some(first), Some(last)) = (
            lines.iter().position(inside),
            lines.iter().rposition(inside),
        ) else {
            continue;
        };
        // Only new lines may come between its own.
        if lines[first..=last]
            .iter()
            .any(|line| matches!(line, Line::Kept(_)) && !inside(line))
        {
            return false;
        }
        let now = match axis {
            Axis::Rows => CellMerge {
                row: first,
                rows: last - first + 1,
                ..*m
            },
            Axis::Columns => CellMerge {
                col: first,
                cols: last - first + 1,
                ..*m
            },
        };
        if lines[first] != Line::Kept(start)
            && let Some(cell) = t.cell(m.row, m.col)
        {
            moved.push((now, cell.text.clone()));
        }
        if now.rows * now.cols > 1 {
            merges.push(now);
        }
    }
    let (rows, columns) = match axis {
        Axis::Rows => (sizes, t.columns.clone()),
        Axis::Columns => (t.rows.clone(), sizes),
    };
    let mut cells = Vec::with_capacity(rows.len() * columns.len());
    for r in 0..rows.len() {
        for c in 0..columns.len() {
            let (line, from) = match axis {
                Axis::Rows => (lines[r], (lines[r].source(), c)),
                Axis::Columns => (lines[c], (r, lines[c].source())),
            };
            let Some(cell) = t.cell(from.0, from.1) else {
                return false;
            };
            cells.push(match line {
                Line::Kept(_) | Line::Copy(_) => cell.clone(),
                Line::Like(_) => TableCell {
                    text: String::new(),
                    style: cell.style.clone(),
                },
            });
        }
    }
    let mut made = TableNode {
        columns,
        rows,
        cells,
        merges,
        ..t.clone()
    };
    for (now, words) in moved {
        if let Some(cell) = made.cell_mut(now.row, now.col) {
            cell.text = words;
        }
    }
    // A line copied into a merge is hidden by it: its words would be lost
    // to sight.
    let merges = made.merges.clone();
    for m in merges {
        for (r, c) in range_of(m).cells() {
            if (r, c) != (m.row, m.col)
                && let Some(cell) = made.cell_mut(r, c)
            {
                cell.text.clear();
            }
        }
    }
    if !made.is_valid() {
        return false;
    }
    *t = made;
    true
}

fn range_of(m: CellMerge) -> Range {
    Range::new((m.row, m.col), (m.row + m.rows - 1, m.col + m.cols - 1))
}

fn kept(count: usize) -> Vec<Line> {
    (0..count).map(Line::Kept).collect()
}

fn insert(t: &mut TableNode, axis: Axis, count: usize, at: usize, like: usize) -> bool {
    if at > count {
        return false;
    }
    let mut lines = kept(count);
    lines.insert(at, Line::Like(like));
    rebuild(t, axis, &lines)
}

/// A new row at `at`, with the looks of the row `like`.
pub fn insert_row(t: &mut TableNode, at: usize, like: usize) -> bool {
    insert(t, Axis::Rows, t.rows.len(), at, like)
}

/// A new column at `at`, with the looks of the column `like`.
pub fn insert_column(t: &mut TableNode, at: usize, like: usize) -> bool {
    insert(t, Axis::Columns, t.columns.len(), at, like)
}

fn remove(t: &mut TableNode, axis: Axis, count: usize, from: usize, to: usize) -> bool {
    let lines: Vec<Line> = (0..count)
        .filter(|i| !(from..=to).contains(i))
        .map(Line::Kept)
        .collect();
    lines.len() < count && rebuild(t, axis, &lines)
}

/// Takes out the rows `from` to `to`; never all of them.
pub fn remove_rows(t: &mut TableNode, from: usize, to: usize) -> bool {
    remove(t, Axis::Rows, t.rows.len(), from, to)
}

/// Takes out the columns `from` to `to`; never all of them.
pub fn remove_columns(t: &mut TableNode, from: usize, to: usize) -> bool {
    remove(t, Axis::Columns, t.columns.len(), from, to)
}

/// Copies the rows `from` to `to` right under them.
pub fn duplicate_rows(t: &mut TableNode, from: usize, to: usize) -> bool {
    if from > to || to >= t.rows.len() {
        return false;
    }
    let mut lines = kept(t.rows.len());
    lines.splice(to + 1..to + 1, (from..=to).map(Line::Copy));
    rebuild(t, Axis::Rows, &lines)
}

fn step(t: &mut TableNode, axis: Axis, count: usize, index: usize, forward: bool) -> bool {
    let other = if forward {
        index.checked_add(1)
    } else {
        index.checked_sub(1)
    };
    let Some(other) = other.filter(|o| *o < count && index < count) else {
        return false;
    };
    let mut lines = kept(count);
    lines.swap(index, other);
    // Two rows of one merge change places: its words go with neither.
    let torn = t.merges.iter().any(|m| {
        let (start, length) = match axis {
            Axis::Rows => (m.row, m.rows),
            Axis::Columns => (m.col, m.cols),
        };
        let has = |i: usize| (start..start + length).contains(&i);
        length > 1 && (has(index) || has(other))
    });
    !torn && rebuild(t, axis, &lines)
}

/// Moves a row one place down (`forward`) or up.
pub fn move_row(t: &mut TableNode, index: usize, forward: bool) -> bool {
    step(t, Axis::Rows, t.rows.len(), index, forward)
}

/// Moves a column one place right (`forward`) or left.
pub fn move_column(t: &mut TableNode, index: usize, forward: bool) -> bool {
    step(t, Axis::Columns, t.columns.len(), index, forward)
}

/// Joins the cells of `range`; their words go one under another.
pub fn merge(t: &mut TableNode, range: Range) -> bool {
    let range = range.over(t);
    let m = CellMerge {
        row: range.r0,
        col: range.c0,
        rows: range.r1 - range.r0 + 1,
        cols: range.c1 - range.c0 + 1,
    };
    if m.rows * m.cols < 2 || t.merges.contains(&m) {
        return false;
    }
    let mut words: Vec<String> = Vec::new();
    for (r, c) in range.cells() {
        if let Some(cell) = t.cell_mut(r, c) {
            let text = std::mem::take(&mut cell.text);
            if !text.is_empty() {
                words.push(text);
            }
        }
    }
    if let Some(cell) = t.cell_mut(range.r0, range.c0) {
        let mut text = words.join("\n");
        if text.len() > TableNode::MAX_CELL_TEXT {
            let mut end = TableNode::MAX_CELL_TEXT;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            text.truncate(end);
        }
        cell.text = text;
    }
    t.merges.retain(|old| !range.contains(old.row, old.col));
    t.merges.push(m);
    true
}

/// Parts the merges `range` touches.
pub fn unmerge(t: &mut TableNode, range: Range) -> bool {
    let before = t.merges.len();
    let range = range.over(t);
    t.merges.retain(|m| !range.contains(m.row, m.col));
    t.merges.len() != before
}

fn equalize(sizes: &mut [f64], from: usize, to: usize) {
    let Some(some) = sizes.get_mut(from..=to) else {
        return;
    };
    let each = some.iter().sum::<f64>() / some.len() as f64;
    some.fill(each);
}

/// Makes the columns `from` to `to` as wide as each other.
pub fn equalize_columns(t: &mut TableNode, from: usize, to: usize) {
    equalize(&mut t.columns, from, to);
}

/// Makes the rows `from` to `to` as tall as each other.
pub fn equalize_rows(t: &mut TableNode, from: usize, to: usize) {
    equalize(&mut t.rows, from, to);
}

/// Changes the looks of the cells of `range`.
pub fn restyle(t: &mut TableNode, range: Range, change: impl Fn(&mut CellStyle)) {
    for (r, c) in range.over(t).cells() {
        if let Some(cell) = t.cell_mut(r, c) {
            change(&mut cell.style);
        }
    }
}

/// Gives the cells of `range` the looks of another cell.
pub fn paste_style(t: &mut TableNode, range: Range, style: &CellStyle) {
    restyle(t, range, |own| own.clone_from(style));
}

/// Takes the words out of the cells of `range`.
pub fn clear(t: &mut TableNode, range: Range) {
    for (r, c) in range.over(t).cells() {
        if let Some(cell) = t.cell_mut(r, c) {
            cell.text.clear();
        }
    }
}
