//! Layer-1 edit commands for Gafira: the `Command` contract, `Transaction`
//! and the history built on them.
//!
//! Pure domain code: no I/O, no clocks, no `unsafe`. Depends only on
//! `gafira-geom`, `gafira-color` and `gafira-doc`.

#![forbid(unsafe_code)]

mod assets;
mod budget;
mod command;
mod dirty;
mod error;
mod history;
mod insert_nodes;
mod moves;
mod pages;
mod remove_nodes;
mod reorder;
mod reparent;
mod set_property;
mod transaction;

pub use assets::InsertAsset;
pub use command::{Applied, Command, GestureId, MergeKey};
pub use dirty::DirtySet;
pub use error::CommandError;
pub use history::{DEFAULT_BUDGET, History};
pub use insert_nodes::{InsertNodes, NodeSpec};
pub use pages::{InsertPage, RemovePage};
pub use remove_nodes::RemoveNodes;
pub use reorder::Reorder;
pub use reparent::Reparent;
pub use set_property::{Property, SetProperty};
pub use transaction::Transaction;
