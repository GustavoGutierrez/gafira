//! A grid of cells as the table of bulk create (spec 104 §5): the row of
//! names, empty titles, empty rows and rows of another length.

use std::collections::BTreeSet;
use std::sync::Arc;

use gafira_doc::{DataColumn, DataRow, DataSet};

use crate::keys::key_for;

/// Cells row by row, as a reader or the clipboard gives them.
pub type Grid = Vec<Vec<String>>;

/// How many of the first rows are looked at for the names (§5.2).
const LOOKED_AT: usize = 20;

/// A grid as a table, and what was done to it, to be told.
#[derive(Clone, Debug, PartialEq)]
pub struct Imported {
    pub set: DataSet,
    /// Rows with nothing in them, left out.
    pub empty_rows: usize,
    /// Rows with fewer or more cells than the names, kept and filled with
    /// empty cells.
    pub uneven_rows: usize,
    /// Rows past [`DataSet::MAX_ROWS`] and columns past
    /// [`DataSet::MAX_COLUMNS`], left out.
    pub cut_rows: usize,
    pub cut_columns: usize,
}

fn filled(row: &[String]) -> usize {
    row.iter().filter(|c| !c.trim().is_empty()).count()
}

/// The row of names it suggests: the first of the fullest among the first
/// rows; `None` for an empty grid. A suggestion the person may change.
#[must_use]
pub fn guess_header(grid: &Grid) -> Option<usize> {
    let first = &grid[..grid.len().min(LOOKED_AT)];
    let most = first.iter().map(|r| filled(r)).max().filter(|n| *n > 0)?;
    first.iter().position(|r| filled(r) == most)
}

/// `value` as a cell holds it: `\r\n` as `\n`, at most
/// [`DataSet::MAX_CELL_TEXT`] bytes.
fn clean(value: &str) -> String {
    let mut out = value.replace("\r\n", "\n").replace('\r', "\n");
    if out.len() > DataSet::MAX_CELL_TEXT {
        let mut end = DataSet::MAX_CELL_TEXT;
        while !out.is_char_boundary(end) {
            end -= 1;
        }
        out.truncate(end);
    }
    out
}

/// The table of `grid` with the names in the row `header` (none: every row
/// is data). A column without a title is named by `unnamed` with its index;
/// keys are unique. Rows before the names are left out.
#[must_use]
pub fn from_grid(
    grid: &Grid,
    header: Option<usize>,
    unnamed: &dyn Fn(usize) -> String,
) -> Imported {
    let start = header.map_or(0, |h| h + 1);
    let titles: &[String] = header.and_then(|h| grid.get(h)).map_or(&[], Vec::as_slice);
    let data = grid.get(start..).unwrap_or_default();
    // As wide as the widest row with words, or as the titles.
    let width = data
        .iter()
        .map(|r| {
            r.iter()
                .rposition(|c| !c.trim().is_empty())
                .map_or(0, |i| i + 1)
        })
        .chain(std::iter::once(
            titles
                .iter()
                .rposition(|c| !c.trim().is_empty())
                .map_or(0, |i| i + 1),
        ))
        .max()
        .unwrap_or(0);
    let kept = width.min(DataSet::MAX_COLUMNS);
    let mut next_id = 1;
    let mut taken = BTreeSet::new();
    let columns: Vec<DataColumn> = (0..kept)
        .map(|i| {
            let title = titles.get(i).map_or("", |t| t.trim());
            let name = if title.is_empty() {
                unnamed(i)
            } else {
                clean(title)
            };
            let key = key_for(&name, &taken);
            taken.insert(key.clone());
            let column = DataColumn {
                id: next_id,
                name,
                key,
            };
            next_id += 1;
            column
        })
        .collect();
    let expected = if titles.is_empty() {
        width
    } else {
        titles.len().max(width)
    };
    let (mut empty_rows, mut uneven_rows, mut cut_rows) = (0, 0, 0);
    let mut rows = Vec::new();
    for row in data {
        if filled(row) == 0 {
            empty_rows += 1;
            continue;
        }
        if rows.len() == DataSet::MAX_ROWS {
            cut_rows += 1;
            continue;
        }
        if row.len() != expected {
            uneven_rows += 1;
        }
        let values = (0..kept)
            .map(|i| row.get(i).map_or_else(String::new, |v| clean(v)))
            .collect();
        rows.push(Arc::new(DataRow::new(next_id, values)));
        next_id += 1;
    }
    Imported {
        set: DataSet {
            columns,
            rows,
            next_id,
            ..DataSet::default()
        },
        empty_rows,
        uneven_rows,
        cut_rows,
        cut_columns: width - kept,
    }
}
