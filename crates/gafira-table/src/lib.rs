//! Tables for Gafira (spec 081, FD-21): where the cells lie, the changes to
//! rows, columns, cells and borders, what goes to the clipboard, and the
//! values the cells hold.
//!
//! Pure engine code: no I/O, no `unsafe`. Every change leaves a table a
//! document holds, or leaves it as it was.

#![forbid(unsafe_code)]

mod borders;
mod clipboard;
mod layout;
mod ops;
mod values;

pub use borders::{BorderSet, Edge, edges, set_borders};
pub use clipboard::{copy, parse_grid, paste};
pub use layout::{Layout, Range, anchors};
pub use ops::{
    MIN_SIZE, clear, duplicate_rows, equalize_columns, equalize_rows, insert_column, insert_row,
    merge, move_column, move_row, paste_style, remove_columns, remove_rows, resize_column,
    resize_row, restyle, size, to_px, unmerge,
};
pub use values::{CellValue, NumberFormat, TableData, data, value};
