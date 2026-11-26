//! The connected data of bulk create (spec 104 §6, §11): a table the
//! document carries whole, so it works without the sheet it came from.
//!
//! A column is a field and a row a variation. Columns and rows have stable
//! ids from the table's own counter, so renaming a column keeps the fields
//! that name its `key`, and reordering rows keeps their identity.

use std::collections::BTreeSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::PageId;

/// A column: its id, the name it came with and the key the text fields
/// name it by (`{{key}}`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataColumn {
    pub id: u64,
    /// The original title, any Unicode.
    pub name: String,
    /// The simple key, `[A-Za-z_][A-Za-z0-9_]*`, unique in the table.
    pub key: String,
}

impl DataColumn {
    #[must_use]
    pub fn new(id: u64, name: &str, key: &str) -> Self {
        Self {
            id,
            name: name.to_owned(),
            key: key.to_owned(),
        }
    }
}

/// A row: its id and one value per column, as the source gave it (text).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataRow {
    pub id: u64,
    pub values: Vec<String>,
}

impl DataRow {
    #[must_use]
    pub fn new(id: u64, values: Vec<String>) -> Self {
        Self { id, values }
    }
}

/// Where the table came from: informative only, never needed to reopen.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataSource {
    /// `csv`, `xlsx`, `paste` or `manual`.
    pub kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub file_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheet: Option<String>,
    /// The row of the sheet the names came from, from 1; none without names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_row: Option<usize>,
}

/// What a bulk run writes (spec 104 §10.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BulkOutput {
    /// One PDF with every page, row after row.
    #[default]
    PdfCombined,
    /// One PDF per row.
    PdfEach,
    /// One PNG per row and page.
    Png,
    /// One JPEG per row and page.
    Jpeg,
    /// Only the editable `.gaf` with every page.
    Gaf,
}

/// The settings of the last bulk run, kept with the document (§11.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BulkProfile {
    #[serde(default)]
    pub output: BulkOutput,
    /// The names of the files, with fields: `certificado_{{nombre}}`.
    #[serde(default)]
    pub file_name: String,
    /// Also write a `.gaf` with every page, editable.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub editable: bool,
}

impl Default for BulkProfile {
    fn default() -> Self {
        Self {
            output: BulkOutput::default(),
            file_name: "diseno_{{fila}}".to_owned(),
            editable: false,
        }
    }
}

/// The connected data of a document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataSet {
    /// The version of this block of the schema (`bulk_data_v1`).
    #[serde(default = "first_version")]
    pub version: u32,
    /// The name shown: the file, or what the person called it.
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<DataSource>,
    pub columns: Vec<DataColumn>,
    pub rows: Vec<Arc<DataRow>>,
    /// The next id for a column or a row; above every id in use.
    pub next_id: u64,
    /// The template pages, in order; empty means every page.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub template_pages: Vec<PageId>,
    #[serde(default)]
    pub profile: BulkProfile,
}

fn first_version() -> u32 {
    1
}

impl Default for DataSet {
    fn default() -> Self {
        Self {
            version: 1,
            name: String::new(),
            source: None,
            columns: Vec::new(),
            rows: Vec::new(),
            next_id: 1,
            template_pages: Vec::new(),
            profile: BulkProfile::default(),
        }
    }
}

/// What makes a [`DataSet`] invalid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataProblem {
    /// More than [`DataSet::MAX_COLUMNS`] or [`DataSet::MAX_ROWS`].
    TooLarge,
    /// A key is not `[A-Za-z_][A-Za-z0-9_]*`.
    InvalidKey,
    /// Two columns share a key.
    DuplicateKey,
    /// Two columns or rows share an id.
    DuplicateId,
    /// An id is not below `next_id`.
    IdNotBelowNext,
    /// A row has not one value per column.
    RowWidth,
    /// A value is longer than [`DataSet::MAX_CELL_TEXT`].
    LongValue,
    /// The version of the block is not one this build reads.
    Version,
}

impl DataSet {
    /// The most columns (spec 104 §14: 500 is a stress case).
    pub const MAX_COLUMNS: usize = 500;
    /// The most rows (spec 104 §14: 100 000 is a stress case).
    pub const MAX_ROWS: usize = 100_000;
    /// The longest value of a cell, in bytes.
    pub const MAX_CELL_TEXT: usize = 32 * 1024;

    /// Whether `key` is a simple key: `[A-Za-z_][A-Za-z0-9_]*`.
    #[must_use]
    pub fn is_key(key: &str) -> bool {
        let mut chars = key.chars();
        chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    }

    /// The first rule the table breaks, if any.
    #[must_use]
    pub fn problem(&self) -> Option<DataProblem> {
        if self.version != 1 {
            return Some(DataProblem::Version);
        }
        if self.columns.len() > Self::MAX_COLUMNS || self.rows.len() > Self::MAX_ROWS {
            return Some(DataProblem::TooLarge);
        }
        if !self.columns.iter().all(|c| Self::is_key(&c.key)) {
            return Some(DataProblem::InvalidKey);
        }
        let mut keys = BTreeSet::new();
        if !self.columns.iter().all(|c| keys.insert(c.key.as_str())) {
            return Some(DataProblem::DuplicateKey);
        }
        let mut ids = BTreeSet::new();
        let all = self
            .columns
            .iter()
            .map(|c| c.id)
            .chain(self.rows.iter().map(|r| r.id));
        for id in all {
            if !ids.insert(id) {
                return Some(DataProblem::DuplicateId);
            }
            if id >= self.next_id {
                return Some(DataProblem::IdNotBelowNext);
            }
        }
        if self
            .rows
            .iter()
            .any(|r| r.values.len() != self.columns.len())
        {
            return Some(DataProblem::RowWidth);
        }
        let long = |r: &Arc<DataRow>| r.values.iter().any(|v| v.len() > Self::MAX_CELL_TEXT);
        if self.rows.iter().any(long) {
            return Some(DataProblem::LongValue);
        }
        None
    }

    /// The column whose key is `key`.
    #[must_use]
    pub fn column_by_key(&self, key: &str) -> Option<&DataColumn> {
        self.columns.iter().find(|c| c.key == key)
    }

    /// The column whose original name is `name`.
    #[must_use]
    pub fn column_by_name(&self, name: &str) -> Option<&DataColumn> {
        self.columns.iter().find(|c| c.name == name)
    }

    /// The value of the row at `row` in the column with id `column`.
    #[must_use]
    pub fn value(&self, row: usize, column: u64) -> Option<&str> {
        let at = self.columns.iter().position(|c| c.id == column)?;
        self.rows.get(row)?.values.get(at).map(String::as_str)
    }
}
