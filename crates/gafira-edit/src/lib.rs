//! Layer-1 edit commands for Gafira: the `Command` contract, `Transaction`
//! and the history built on them.
//!
//! Pure domain code: no I/O, no clocks, no `unsafe`. Depends only on
//! `gafira-geom`, `gafira-color` and `gafira-doc`.

#![forbid(unsafe_code)]
