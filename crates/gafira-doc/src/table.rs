//! Tables (spec 081, FD-21): rows, columns and cells with their words and
//! their looks. `gafira-table` lays them out and changes them.

use gafira_color::Color;
use serde::{Deserialize, Serialize};

use crate::{ShapeStroke, TextAlign, TextNode, VerticalAlign};

/// The four edges of a cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CellBorders {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top: Option<ShapeStroke>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<ShapeStroke>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bottom: Option<ShapeStroke>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<ShapeStroke>,
}

impl CellBorders {
    fn all(&self) -> impl Iterator<Item = ShapeStroke> {
        [self.top, self.right, self.bottom, self.left]
            .into_iter()
            .flatten()
    }
}

/// How a cell looks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CellStyle {
    /// `None` shows what lies behind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<Color>,
    /// The colour of the words.
    pub color: Color,
    pub font: String,
    /// In document px.
    pub size: f64,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    pub align: TextAlign,
    pub valign: VerticalAlign,
    #[serde(default)]
    pub borders: CellBorders,
}

impl CellStyle {
    /// Plain words of `size` px in `font` and `color`, centred.
    #[must_use]
    pub fn new(font: &str, size: f64, color: Color) -> Self {
        Self {
            fill: None,
            color,
            font: font.to_owned(),
            size,
            bold: false,
            italic: false,
            align: TextAlign::Center,
            valign: VerticalAlign::Middle,
            borders: CellBorders::default(),
        }
    }

    fn is_valid(&self) -> bool {
        let color = |c: Color| c.is_finite() && (0.0..=1.0).contains(&c.alpha());
        self.fill.is_none_or(color)
            && color(self.color)
            && self.size.is_finite()
            && self.size > 0.0
            && self.size <= TextNode::MAX_SIZE
            && self
                .borders
                .all()
                .all(|b| color(b.color) && (0.0..=ShapeStroke::MAX_WIDTH).contains(&b.width))
    }
}

/// A cell: its words and its looks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TableCell {
    #[serde(default)]
    pub text: String,
    pub style: CellStyle,
}

/// Cells shown as one, from its top left.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellMerge {
    pub row: usize,
    pub col: usize,
    pub rows: usize,
    pub cols: usize,
}

impl CellMerge {
    /// Whether it covers the cell.
    #[must_use]
    pub fn covers(&self, row: usize, col: usize) -> bool {
        (self.row..self.row + self.rows).contains(&row)
            && (self.col..self.col + self.cols).contains(&col)
    }

    fn touches(&self, other: &Self) -> bool {
        self.row < other.row + other.rows
            && other.row < self.row + self.rows
            && self.col < other.col + other.cols
            && other.col < self.col + self.cols
    }
}

/// A table (spec 081). The sizes of its columns and rows are shares of its
/// box: changing the box changes them all alike.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TableNode {
    pub columns: Vec<f64>,
    pub rows: Vec<f64>,
    /// Row by row, left to right.
    pub cells: Vec<TableCell>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub merges: Vec<CellMerge>,
    /// The room between cells, in px.
    #[serde(default)]
    pub spacing: f64,
    /// The room between the edge of a cell and its words, in px.
    #[serde(default)]
    pub padding: f64,
    /// Whether the first row names the columns.
    #[serde(default)]
    pub header: bool,
}

impl TableNode {
    /// The most rows and columns (FD-21).
    pub const MAX_ROWS: usize = 50;
    pub const MAX_COLUMNS: usize = 20;
    /// The most room between or inside the cells, in px.
    pub const MAX_SPACING: f64 = 100.0;
    /// The most bytes of words in a cell.
    pub const MAX_CELL_TEXT: usize = 32 * 1024;

    /// A table of `rows` × `columns` equal cells of `style`.
    #[must_use]
    pub fn new(rows: usize, columns: usize, (w, h): (f64, f64), style: &CellStyle) -> Self {
        let cell = TableCell {
            text: String::new(),
            style: style.clone(),
        };
        Self {
            columns: vec![w; columns],
            rows: vec![h; rows],
            cells: vec![cell; rows * columns],
            merges: Vec::new(),
            spacing: 0.0,
            padding: 8.0,
            header: false,
        }
    }

    /// The cell at `row`, `col`.
    #[must_use]
    pub fn cell(&self, row: usize, col: usize) -> Option<&TableCell> {
        if col >= self.columns.len() {
            return None;
        }
        self.cells.get(row * self.columns.len() + col)
    }

    /// The cell at `row`, `col`, to change it.
    pub fn cell_mut(&mut self, row: usize, col: usize) -> Option<&mut TableCell> {
        if col >= self.columns.len() {
            return None;
        }
        self.cells.get_mut(row * self.columns.len() + col)
    }

    /// The merge that covers a cell, if any.
    #[must_use]
    pub fn merge_at(&self, row: usize, col: usize) -> Option<CellMerge> {
        self.merges.iter().copied().find(|m| m.covers(row, col))
    }

    /// Whether a document holds it: its sizes, cells and merges agree, and
    /// its numbers and colours can be drawn.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let sizes = |list: &[f64], most: usize| {
            (1..=most).contains(&list.len())
                && list.iter().all(|v| v.is_finite() && *v > 0.0)
                && list.iter().sum::<f64>().is_finite()
        };
        let room = 0.0..=Self::MAX_SPACING;
        sizes(&self.columns, Self::MAX_COLUMNS)
            && sizes(&self.rows, Self::MAX_ROWS)
            && self.cells.len() == self.rows.len() * self.columns.len()
            && room.contains(&self.spacing)
            && room.contains(&self.padding)
            && self
                .cells
                .iter()
                .all(|c| c.text.len() <= Self::MAX_CELL_TEXT && c.style.is_valid())
            && self.merges.iter().enumerate().all(|(i, m)| {
                m.rows >= 1
                    && m.cols >= 1
                    && m.row
                        .checked_add(m.rows)
                        .is_some_and(|end| end <= self.rows.len())
                    && m.col
                        .checked_add(m.cols)
                        .is_some_and(|end| end <= self.columns.len())
                    && m.rows * m.cols > 1
                    && self.merges[..i].iter().all(|other| !m.touches(other))
            })
    }
}
