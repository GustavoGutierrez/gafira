//! Work-model progress and cancellation (ARQ-112): units done over units planned.

use std::sync::{Mutex, PoisonError};

use gafira_doc::contract::{CancelToken, Cancelled, Progress};

/// Rows per serial chunk of the SLIC passes, connectivity and adjacency (XP11).
pub(crate) const CHUNK_ROWS: usize = 64;

/// Counts planned and finished work units. Reports are serialised and the
/// token is read under the same lock, so fractions never decrease and no
/// report follows a cancel seen by the caller.
pub(crate) struct Meter<'a> {
    progress: &'a dyn Progress,
    cancel: &'a CancelToken,
    total: u64,
    done: Mutex<u64>,
}

impl<'a> Meter<'a> {
    /// `planned` units of real work plus a final reserve that only `finish` spends.
    pub(crate) fn new(progress: &'a dyn Progress, cancel: &'a CancelToken, planned: u64) -> Self {
        let reserve = (planned / 50).max(1);
        Self {
            progress,
            cancel,
            total: planned + reserve,
            done: Mutex::new(0),
        }
    }

    /// Checks the token, then books `units` and reports the new fraction.
    pub(crate) fn step(&self, units: u64) -> Result<(), Cancelled> {
        let mut done = self.done.lock().unwrap_or_else(PoisonError::into_inner);
        self.cancel.check()?;
        *done += units;
        let fraction = (*done as f64 / self.total as f64) as f32;
        self.progress.report(fraction);
        Ok(())
    }

    /// Reports `1.0` once the output is whole.
    pub(crate) fn finish(&self) -> Result<(), Cancelled> {
        let _guard = self.done.lock().unwrap_or_else(PoisonError::into_inner);
        self.cancel.check()?;
        self.progress.report(1.0);
        Ok(())
    }
}

impl Meter<'_> {
    /// Checks the token without booking work.
    pub(crate) fn check(&self) -> Result<(), Cancelled> {
        self.cancel.check()
    }
}
