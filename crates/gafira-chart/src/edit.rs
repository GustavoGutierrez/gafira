//! Changes to the data of a chart (S83-07): each leaves a chart a document
//! holds, or leaves it as it was.

use gafira_doc::ChartNode;

/// The data row by row.
#[must_use]
pub fn grid(chart: &ChartNode) -> Vec<Vec<String>> {
    chart
        .cells
        .chunks(chart.columns.max(1))
        .map(<[String]>::to_vec)
        .collect()
}

/// Puts `words` at `row`, `col`, with more rows or columns if it takes them;
/// the empty ones at the end go. Whether it changed.
pub fn set_cell(chart: &mut ChartNode, row: usize, col: usize, words: &str) -> bool {
    if row >= ChartNode::MAX_ROWS
        || col >= ChartNode::MAX_COLUMNS
        || words.len() > ChartNode::MAX_WORDS
        || chart.cell(row, col) == words
    {
        return false;
    }
    let mut all = grid(chart);
    if all.len() <= row {
        all.resize(row + 1, Vec::new());
    }
    if all[row].len() <= col {
        all[row].resize(col + 1, String::new());
    }
    words.clone_into(&mut all[row][col]);
    put(chart, &all);
    true
}

/// The rows as columns and the columns as rows; false when they do not fit.
pub fn transpose(chart: &mut ChartNode) -> bool {
    let (rows, columns) = (chart.rows(), chart.columns);
    if rows > ChartNode::MAX_COLUMNS || columns > ChartNode::MAX_ROWS {
        return false;
    }
    let turned: Vec<Vec<String>> = (0..columns)
        .map(|c| (0..rows).map(|r| chart.cell(r, c).to_owned()).collect())
        .collect();
    // The columns are others.
    chart.labels = None;
    chart.values.clear();
    put(chart, &turned);
    true
}

/// No data.
pub fn clear(chart: &mut ChartNode) {
    chart.labels = None;
    chart.values.clear();
    put(chart, &[]);
}

/// The data of `grid`, as much as a chart holds; false when there are none.
pub fn from_grid(chart: &mut ChartNode, grid: &[Vec<String>]) -> bool {
    if grid.iter().flatten().all(|words| words.trim().is_empty()) {
        return false;
    }
    chart.labels = None;
    chart.values.clear();
    put(chart, grid);
    true
}

/// `words` cut to the bytes a cell holds.
fn cut(words: &str) -> String {
    let mut end = words.len().min(ChartNode::MAX_WORDS);
    while !words.is_char_boundary(end) {
        end -= 1;
    }
    words[..end].to_owned()
}

/// Keeps the rows and columns up to the last ones with words, one at least.
fn put(chart: &mut ChartNode, all: &[Vec<String>]) {
    let all = &all[..all.len().min(ChartNode::MAX_ROWS)];
    let filled = |row: &Vec<String>| {
        row.iter()
            .take(ChartNode::MAX_COLUMNS)
            .rposition(|words| !words.is_empty())
            .map_or(0, |c| c + 1)
    };
    let columns = all.iter().map(filled).max().unwrap_or(0).max(1);
    let rows = all
        .iter()
        .rposition(|row| filled(row) > 0)
        .map_or(1, |r| r + 1);
    chart.columns = columns;
    chart.cells = (0..rows)
        .flat_map(|r| (0..columns).map(move |c| (r, c)))
        .map(|(r, c)| {
            let words = all.get(r).and_then(|row| row.get(c));
            words.map_or_else(String::new, |words| cut(words))
        })
        .collect();
    chart.labels = chart.labels.filter(|c| *c < columns);
    chart.values.retain(|c| *c < columns);
}
