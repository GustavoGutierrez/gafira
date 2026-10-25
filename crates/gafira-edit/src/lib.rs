//! Layer-1 edit commands for Gafira: the `Command` contract, `Transaction`
//! and the history built on them.
//!
//! Pure domain code: no I/O, no clocks, no `unsafe`. Depends only on
//! `gafira-geom`, `gafira-color` and `gafira-doc`.

#![forbid(unsafe_code)]

mod command;
mod dirty;
mod error;
mod set_property;
mod transaction;

pub use command::{Applied, Command, GestureId, MergeKey};
pub use dirty::DirtySet;
pub use error::CommandError;
pub use set_property::{Property, SetProperty};
pub use transaction::Transaction;
