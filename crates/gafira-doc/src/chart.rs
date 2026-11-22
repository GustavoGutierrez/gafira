//! Charts (spec 083, FD-22, D-114): their data, as words, and how they look.
//! `gafira-chart` reads the data and draws them.

use gafira_color::Color;
use serde::{Deserialize, Serialize};

use crate::TextNode;

/// The kinds of chart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChartKind {
    Columns,
    Rows,
    GroupedColumns,
    GroupedRows,
    StackedColumns,
    StackedRows,
    Line,
    Pie,
    Donut,
    StackedArea,
    Area,
    Stream,
    PercentArea,
    Scatter,
    Bubbles,
    DotColumns,
    DotRows,
    Pack,
    Treemap,
    NestedPack,
    Sunburst,
    ProgressRing,
    ProgressArc,
    ProgressBar,
    Gauge,
    Histogram,
    Funnel,
    Radar,
}

/// What rows with the same label become.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChartCombine {
    #[default]
    None,
    Sum,
    Mean,
    Min,
    Max,
    Count,
}

/// The numbers written on the parts of a chart.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChartLabels {
    #[default]
    None,
    /// As the data have them.
    Value,
    /// Their share of the whole.
    Percent,
}

/// How the words of a chart look.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChartText {
    pub font: String,
    /// In document px.
    pub size: f64,
    pub color: Color,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
}

/// The most colours of a chart.
pub const MAX_CHART_COLORS: usize = 24;

/// A chart (spec 083).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartNode {
    pub kind: ChartKind,
    /// How many columns the data have.
    pub columns: usize,
    /// The data row by row, left to right; the first row names the columns.
    pub cells: Vec<String>,
    /// The column of the labels; `None` finds it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<usize>,
    /// The columns of the values; empty finds them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<usize>,
    #[serde(default)]
    pub combine: ChartCombine,
    /// In the order of the series or of the labels, over and over.
    pub colors: Vec<Color>,
    /// Whether each label of a single series has a colour of its own.
    #[serde(default)]
    pub by_label: bool,
    pub text: ChartText,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(default)]
    pub legend: bool,
    /// Whether the labels of the data are written.
    #[serde(default)]
    pub names: bool,
    #[serde(default)]
    pub numbers: ChartLabels,
    /// Whether the axis of the values is written.
    #[serde(default)]
    pub axis: bool,
    #[serde(default)]
    pub grid: bool,
    #[serde(default)]
    pub smooth: bool,
    #[serde(default)]
    pub markers: bool,
    /// How round the ends of the bars are, 0 to 1.
    #[serde(default)]
    pub round: f64,
    /// The share of the room of a bar left between it and the next, 0 to
    /// [`Self::MAX_GAP`].
    #[serde(default)]
    pub gap: f64,
    /// The share of a ring that is its hole, 0 to [`Self::MAX_HOLE`].
    #[serde(default)]
    pub hole: f64,
    /// The width of the lines, in document px.
    pub weight: f64,
}

impl ChartNode {
    /// The most rows, with the one of the names, and columns.
    pub const MAX_ROWS: usize = 201;
    pub const MAX_COLUMNS: usize = 20;
    /// The most bytes of a cell and of the title.
    pub const MAX_WORDS: usize = 1024;
    pub const MAX_GAP: f64 = 0.9;
    pub const MAX_HOLE: f64 = 0.95;
    pub const MAX_WEIGHT: f64 = 200.0;

    /// A chart of `kind` with its data, its colours and its words, and
    /// nothing else written.
    #[must_use]
    pub fn new(
        kind: ChartKind,
        (columns, cells): (usize, Vec<String>),
        colors: Vec<Color>,
        text: ChartText,
    ) -> Self {
        Self {
            kind,
            columns,
            cells,
            labels: None,
            values: Vec::new(),
            combine: ChartCombine::None,
            colors,
            by_label: false,
            text,
            title: String::new(),
            legend: false,
            names: false,
            numbers: ChartLabels::None,
            axis: false,
            grid: false,
            smooth: false,
            markers: false,
            round: 0.0,
            gap: 0.3,
            hole: 0.6,
            weight: 3.0,
        }
    }

    /// The rows, with the one of the names.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.cells.len().checked_div(self.columns).unwrap_or(0)
    }

    /// The words at `row`, `col`; none outside the data.
    #[must_use]
    pub fn cell(&self, row: usize, col: usize) -> &str {
        if col >= self.columns {
            return "";
        }
        self.cells
            .get(row * self.columns + col)
            .map_or("", String::as_str)
    }

    /// Whether a document holds it.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let color = |c: Color| c.is_finite() && (0.0..=1.0).contains(&c.alpha());
        let words = |text: &String| text.len() <= Self::MAX_WORDS;
        let data = (1..=Self::MAX_COLUMNS).contains(&self.columns)
            && self.cells.len().is_multiple_of(self.columns)
            && (1..=Self::MAX_ROWS).contains(&self.rows())
            && self.cells.iter().all(words);
        let once = |at: usize| !self.values[..at].contains(&self.values[at]);
        let chosen = self.labels.is_none_or(|c| c < self.columns)
            && self.values.iter().all(|c| *c < self.columns)
            && (0..self.values.len()).all(once);
        let text = color(self.text.color)
            && self.text.size > 0.0
            && self.text.size <= TextNode::MAX_SIZE
            && words(&self.title);
        data && chosen
            && text
            && (1..=MAX_CHART_COLORS).contains(&self.colors.len())
            && self.colors.iter().all(|c| color(*c))
            && (0.0..=1.0).contains(&self.round)
            && (0.0..=Self::MAX_GAP).contains(&self.gap)
            && (0.0..=Self::MAX_HOLE).contains(&self.hole)
            && self.weight > 0.0
            && self.weight <= Self::MAX_WEIGHT
    }
}
