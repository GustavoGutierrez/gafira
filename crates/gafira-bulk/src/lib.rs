//! Bulk create (spec 104): the pure part. The `{{field}}` syntax of text
//! fields, the keys of the columns, a grid of cells as a [`DataSet`], the
//! template filled in row by row, and the names of the files a run writes.
//!
//! Layer-2 domain code: no I/O, no fonts, no language. The interface turns
//! a grid read from a file into a table here, and hands the documents it
//! resolves to the exporters and the `.gaf` writer it already has.
//!
//! [`DataSet`]: gafira_doc::DataSet

#![forbid(unsafe_code)]

mod import;
mod keys;
mod names;
mod resolve;
mod syntax;

pub use import::{Grid, Imported, from_grid, guess_header};
pub use keys::{column_letter, key_for};
pub use names::{Names, sanitize};
pub use resolve::{
    Resolved, lookup, plan, resolve, resolve_plain, resolve_rich, template_pages, unknown, usage,
};
pub use syntax::{FieldRef, Piece, fields, parse, token};
