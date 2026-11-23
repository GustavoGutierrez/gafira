//! Charts for Gafira (spec 083, FD-22, D-114): the data a chart reads, and
//! the shapes and the words that draw it.
//!
//! Pure engine code: no I/O, no fonts (the caller measures the words), no
//! `unsafe`. Whatever the data hold, what is drawn is finite.

#![forbid(unsafe_code)]

mod areas;
mod axes;
mod bars;
mod data;
mod edit;
mod lines;
mod paths;
mod round;
mod scale;
mod scene;

pub use data::{
    Colored, Column, Data, Series, color_names, colored, column, columns, is_number, places, read,
};
pub use edit::{clear, from_grid, grid, set_cell, transpose};
pub use scale::ticks;
pub use scene::{Anchor, Measure, Piece, Words, draw, parts};
