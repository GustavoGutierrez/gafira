//! The data of a chart (S83-02).

use gafira_doc::{ChartCombine, ChartKind, ChartNode};
use gafira_table::{CellValue, value};

/// What a column of the data holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Column {
    Empty,
    Words,
    Numbers,
}

/// A column of values.
#[derive(Clone, Debug, PartialEq)]
pub struct Series {
    /// Its column in the data.
    pub column: usize,
    pub name: String,
    pub values: Vec<Option<f64>>,
    /// The values as they are written.
    pub words: Vec<String>,
}

/// What a chart draws.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Data {
    /// The label of each row.
    pub labels: Vec<String>,
    /// A second column of words, for the kinds that group.
    pub groups: Vec<String>,
    pub series: Vec<Series>,
    /// Whether decimals are written with a comma.
    pub comma: bool,
    /// Whether the values are percentages.
    pub percent: bool,
}

/// What has a colour of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Colored {
    Rows,
    Series,
    Groups,
}

/// The number `words` say, if they say one.
fn number(words: &str) -> Option<f64> {
    match value(words) {
        CellValue::Number(n, _) if n.is_finite() => Some(n),
        _ => None,
    }
}

/// What the column `col` holds: numbers when at least half of what it says
/// are.
#[must_use]
pub fn column(chart: &ChartNode, col: usize) -> Column {
    let filled = (1..chart.rows())
        .map(|r| chart.cell(r, col))
        .filter(|words| !words.trim().is_empty());
    let (mut all, mut numbers) = (0, 0);
    for words in filled {
        all += 1;
        numbers += usize::from(number(words).is_some());
    }
    match (all, numbers) {
        (0, _) => Column::Empty,
        (all, numbers) if numbers * 2 >= all => Column::Numbers,
        _ => Column::Words,
    }
}

/// The most columns of values a kind draws.
fn most_values(kind: ChartKind) -> usize {
    match kind {
        ChartKind::Columns
        | ChartKind::Rows
        | ChartKind::Pie
        | ChartKind::Donut
        | ChartKind::Pack
        | ChartKind::Treemap
        | ChartKind::NestedPack
        | ChartKind::Sunburst
        | ChartKind::Histogram
        | ChartKind::Funnel => 1,
        ChartKind::ProgressRing
        | ChartKind::ProgressArc
        | ChartKind::ProgressBar
        | ChartKind::Gauge
        | ChartKind::Scatter => 2,
        ChartKind::Bubbles => 3,
        _ => ChartNode::MAX_COLUMNS,
    }
}

/// Whether the rows of a kind have a label, so a lone column of numbers may
/// be it (the years).
fn has_labels(kind: ChartKind) -> bool {
    !matches!(
        kind,
        ChartKind::Scatter
            | ChartKind::Bubbles
            | ChartKind::Histogram
            | ChartKind::ProgressRing
            | ChartKind::ProgressArc
            | ChartKind::ProgressBar
            | ChartKind::Gauge
            | ChartKind::DotColumns
            | ChartKind::DotRows
    )
}

/// Whether a kind groups its rows by a second column of words.
fn has_groups(kind: ChartKind) -> bool {
    matches!(
        kind,
        ChartKind::NestedPack | ChartKind::Sunburst | ChartKind::Scatter | ChartKind::Bubbles
    )
}

/// The columns of the group, of the labels and of the values.
fn roles(chart: &ChartNode) -> (Option<usize>, Option<usize>, Vec<usize>) {
    let of = |kind: Column| -> Vec<usize> {
        (0..chart.columns)
            .filter(|c| column(chart, *c) == kind)
            .collect()
    };
    let words = of(Column::Words);
    let mut numbers = of(Column::Numbers);
    let mut group = None;
    let mut labels = chart.labels;
    if has_groups(chart.kind) {
        group = match labels {
            Some(chosen) => words.iter().copied().find(|c| *c != chosen),
            None if words.len() > 1 => Some(words[0]),
            None => None,
        };
    }
    if labels.is_none() {
        labels = words.iter().copied().find(|c| Some(*c) != group);
    }
    if labels.is_none() && has_labels(chart.kind) && numbers.len() > 1 && chart.values.is_empty() {
        labels = numbers.first().copied();
    }
    let mut values = if chart.values.is_empty() {
        numbers.retain(|c| Some(*c) != labels);
        numbers
    } else {
        chart.values.clone()
    };
    values.truncate(most_values(chart.kind));
    (group, labels, values)
}

/// The column of the labels and the ones of the values, chosen or found.
#[must_use]
pub fn columns(chart: &ChartNode) -> (Option<usize>, Vec<usize>) {
    let (_, labels, values) = roles(chart);
    (labels, values)
}

/// Whether `words` are a number a chart draws.
#[must_use]
pub fn is_number(words: &str) -> bool {
    number(words).is_some()
}

/// The place among the labels of each row of the data, the one of the names
/// first; none for the rows left out, or when the rows are combined.
#[must_use]
pub fn places(chart: &ChartNode) -> Vec<Option<usize>> {
    let (_, labels, values) = roles(chart);
    let mut next = 0;
    (0..chart.rows())
        .map(|r| {
            let said = |c: &usize| !chart.cell(r, *c).trim().is_empty();
            let drawn = r > 0 && (labels.iter().any(said) || values.iter().any(said));
            if !drawn || chart.combine != ChartCombine::None {
                return None;
            }
            next += 1;
            Some(next - 1)
        })
        .collect()
}

/// The names of what has a colour of its own in `chart`, in its order.
#[must_use]
pub fn color_names(chart: &ChartNode) -> Vec<String> {
    let data = read(chart);
    let each = |list: &[String]| {
        let mut names: Vec<String> = Vec::new();
        for name in list {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
        names
    };
    match colored(chart) {
        Colored::Rows => data.labels,
        Colored::Groups if data.groups.is_empty() => each(&data.labels),
        Colored::Groups => each(&data.groups),
        Colored::Series => data.series.into_iter().map(|s| s.name).collect(),
    }
}

/// What has a colour of its own in `chart`.
#[must_use]
pub fn colored(chart: &ChartNode) -> Colored {
    match chart.kind {
        ChartKind::Pie
        | ChartKind::Donut
        | ChartKind::Pack
        | ChartKind::Treemap
        | ChartKind::Funnel => Colored::Rows,
        ChartKind::NestedPack | ChartKind::Sunburst | ChartKind::Scatter | ChartKind::Bubbles => {
            Colored::Groups
        }
        ChartKind::Columns | ChartKind::Rows | ChartKind::ProgressBar if chart.by_label => {
            Colored::Rows
        }
        _ => Colored::Series,
    }
}

/// Whether `words` write their decimals with a comma.
fn has_comma(words: &str) -> bool {
    number(words).is_some()
        && words.matches(',').count() == 1
        && words.rfind(',') > words.rfind('.')
}

/// What `chart` draws.
#[must_use]
pub fn read(chart: &ChartNode) -> Data {
    let (group, labels, values) = roles(chart);
    let said = |row: usize, col: Option<usize>| col.map_or("", |c| chart.cell(row, c)).trim();
    let rows: Vec<usize> = (1..chart.rows())
        .filter(|r| {
            let named = !said(*r, labels).is_empty();
            named || values.iter().any(|c| !said(*r, Some(*c)).is_empty())
        })
        .collect();
    let words = |col: Option<usize>| -> Vec<String> {
        rows.iter().map(|r| said(*r, col).to_owned()).collect()
    };
    let series: Vec<Series> = values
        .iter()
        .map(|c| {
            let words = words(Some(*c));
            Series {
                column: *c,
                name: chart.cell(0, *c).trim().to_owned(),
                values: words.iter().map(|w| number(w)).collect(),
                words,
            }
        })
        .collect();
    let all = || {
        series
            .iter()
            .flat_map(|s| &s.words)
            .filter(|w| !w.is_empty())
    };
    let data = Data {
        labels: words(labels),
        groups: group.map_or_else(Vec::new, |c| words(Some(c))),
        comma: all().any(|w| has_comma(w)),
        percent: all().next().is_some() && all().all(|w| w.ends_with('%')),
        series,
    };
    match chart.combine {
        ChartCombine::None => data,
        how => combined(data, how),
    }
}

/// `data` with the rows of the same label and group as one.
fn combined(data: Data, how: ChartCombine) -> Data {
    let key = |i: usize| (data.groups.get(i).cloned(), data.labels[i].clone());
    let mut rows: Vec<Vec<usize>> = Vec::new();
    for i in 0..data.labels.len() {
        match rows.iter_mut().find(|row| key(row[0]) == key(i)) {
            Some(row) => row.push(i),
            None => rows.push(vec![i]),
        }
    }
    let value = |s: &Series, row: &[usize]| -> Option<f64> {
        let found: Vec<f64> = row.iter().filter_map(|i| s.values[*i]).collect();
        // Few rows: the cast is exact.
        let count = found.len() as f64;
        if how == ChartCombine::Count {
            return Some(count);
        }
        if found.is_empty() {
            return None;
        }
        let sum: f64 = found.iter().sum();
        Some(match how {
            ChartCombine::Mean => sum / count,
            ChartCombine::Min => found.iter().copied().fold(f64::INFINITY, f64::min),
            ChartCombine::Max => found.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            _ => sum,
        })
    };
    let percent = data.percent && how != ChartCombine::Count;
    let series = data
        .series
        .iter()
        .map(|s| {
            let values: Vec<Option<f64>> = rows.iter().map(|row| value(s, row)).collect();
            let words = values
                .iter()
                .map(|v| v.map_or_else(String::new, |v| plain(v, percent, data.comma)))
                .collect();
            Series {
                column: s.column,
                name: s.name.clone(),
                values,
                words,
            }
        })
        .collect();
    let first = |list: &[String]| -> Vec<String> {
        rows.iter()
            .filter_map(|row| list.get(row[0]).cloned())
            .collect()
    };
    Data {
        labels: first(&data.labels),
        groups: first(&data.groups),
        series,
        comma: data.comma,
        percent,
    }
}

/// `v` in full, with two decimals at most.
pub(crate) fn plain(v: f64, percent: bool, comma: bool) -> String {
    if percent {
        format!("{}%", crate::scale::number(v * 100.0, 1, comma))
    } else {
        crate::scale::number(v, 2, comma)
    }
}
