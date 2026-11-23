//! Cells to and from the clipboard, as spreadsheets write them: a line a
//! row, tabs between the cells, quotes around one with a tab, a line break
//! or a quote (S81-11).

use gafira_doc::TableNode;

use crate::{Range, anchors, insert_column, insert_row};

/// The cells of `text`.
#[must_use]
pub fn parse_grid(text: &str) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut fresh = true;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    cell.push('"');
                    chars.next();
                }
                '"' => quoted = false,
                '\r' => {}
                _ => cell.push(c),
            }
            continue;
        }
        match c {
            // Only a quote that opens the cell quotes it: 5" is five inches.
            '"' if fresh => {
                quoted = true;
                fresh = false;
            }
            '\t' => {
                row.push(std::mem::take(&mut cell));
                fresh = true;
            }
            '\r' => {}
            '\n' => {
                row.push(std::mem::take(&mut cell));
                rows.push(std::mem::take(&mut row));
                fresh = true;
            }
            _ => {
                cell.push(c);
                fresh = false;
            }
        }
    }
    if !cell.is_empty() || !row.is_empty() {
        row.push(cell);
        rows.push(row);
    }
    rows
}

/// The cells of `range` as text.
#[must_use]
pub fn copy(t: &TableNode, range: Range) -> String {
    let range = range.over(t);
    let field = |r: usize, c: usize| {
        let text = t.cell(r, c).map_or("", |cell| cell.text.as_str());
        if text.contains(['\t', '\n', '\r', '"']) {
            format!("\"{}\"", text.replace('"', "\"\""))
        } else {
            text.to_owned()
        }
    };
    (range.r0..=range.r1)
        .map(|r| {
            (range.c0..=range.c1)
                .map(|c| field(r, c))
                .collect::<Vec<_>>()
                .join("\t")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Fills the cells from `at` with `grid`; the table grows to take it, up to
/// its limits, and what is left over is dropped.
pub fn paste(t: &mut TableNode, at: (usize, usize), grid: &[Vec<String>]) -> bool {
    let wide = grid.iter().map(Vec::len).max().unwrap_or(0);
    if wide == 0 || t.cell(at.0, at.1).is_none() {
        return false;
    }
    while t.rows.len() < at.0 + grid.len() {
        let count = t.rows.len();
        if !insert_row(t, count, count - 1) {
            break;
        }
    }
    while t.columns.len() < at.1 + wide {
        let count = t.columns.len();
        if !insert_column(t, count, count - 1) {
            break;
        }
    }
    // A cell a merge hides takes nothing.
    let seen: Vec<(usize, usize)> = anchors(t).collect();
    for (i, row) in grid.iter().enumerate() {
        for (j, words) in row.iter().enumerate() {
            let place = (at.0 + i, at.1 + j);
            if seen.contains(&place)
                && words.len() <= TableNode::MAX_CELL_TEXT
                && let Some(cell) = t.cell_mut(place.0, place.1)
            {
                cell.text.clone_from(words);
            }
        }
    }
    true
}
