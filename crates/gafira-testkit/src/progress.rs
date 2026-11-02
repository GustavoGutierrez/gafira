//! The test double of ARQ-112: a recording [`Progress`] and a checker that runs
//! an operation against the contract (`check_progress_contract`).
//!
//! Used from integration tests only, like the rest of this crate.

use std::sync::{Mutex, PoisonError};

use gafira_doc::contract::{CancelToken, Progress};

/// Records every fraction it is given, in order.
#[derive(Debug, Default)]
pub struct ProgressLog {
    fractions: Mutex<Vec<f32>>,
}

impl ProgressLog {
    /// An empty log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Every fraction reported so far, in order.
    #[must_use]
    pub fn fractions(&self) -> Vec<f32> {
        self.fractions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Progress for ProgressLog {
    fn report(&self, fraction: f32) {
        self.fractions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(fraction);
    }
}

/// What a run of an operation did against ARQ-112. Indices are 0-based report
/// positions; `k` is the 1-based report inside which the token was cancelled.
#[derive(Clone, Debug, PartialEq)]
pub enum ProgressViolation {
    /// A fraction is below the one before it.
    Decreasing { at: usize },
    /// A fraction is outside `[0, 1]`, NaN included.
    OutOfRange { at: usize },
    /// `1.0` before the end of a good run, or anywhere in a cancelled one.
    EarlyCompletion { at: usize },
    /// The good run emitted nothing, or did not end at `1.0`.
    NotCompleted,
    /// A rise above `max_step` between two reports of the good run.
    StepTooLarge { at: usize },
    /// The operation did not react to a cancel at report `k`.
    NotCancelled { k: usize },
    /// The operation stopped after a cancel at `k` and still returned success.
    OutputAfterCancel { k: usize },
    /// More than one report after the cancel at `k`.
    ReportsAfterCancel { k: usize, extra: usize },
}

/// Runs `op` once for the report count `n`, then once per cancel point (rules
/// in `design.md`, `gafira_testkit::progress`). `op` returns whether it ended
/// in the engine's cancelled error; a successful run returns `Ok(false)`, and
/// any other failure is the caller's.
///
/// # Errors
/// The `Err(E)` of `op`, from any run, at once.
pub fn check_progress_contract<E>(
    max_step: f32,
    op: impl Fn(&CancelToken, &dyn Progress) -> Result<bool, E>,
) -> Result<Result<(), ProgressViolation>, E> {
    let token = CancelToken::new();
    let log = ProgressLog::new();
    if op(&token, &log)? {
        return Ok(Err(ProgressViolation::NotCompleted));
    }
    let baseline = log.fractions();
    if let Err(violation) = scan_baseline(&baseline, max_step) {
        return Ok(Err(violation));
    }
    for k in cancel_points(baseline.len()) {
        let token = CancelToken::new();
        let double = CancelAt {
            log: Mutex::new(Vec::new()),
            token: &token,
            at: k,
        };
        let cancelled = op(&token, &double)?;
        let fractions = double
            .log
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        if let Err(violation) = judge_cancelled(k, cancelled, &fractions) {
            return Ok(Err(violation));
        }
    }
    Ok(Ok(()))
}

/// Logs each report and cancels on the `k`-th. Reports may arrive in parallel
/// (`Progress` is `Sync`), so the position is read under the same lock as the
/// push: exactly one report is the `k`-th.
struct CancelAt<'a> {
    log: Mutex<Vec<f32>>,
    token: &'a CancelToken,
    at: usize,
}

impl Progress for CancelAt<'_> {
    fn report(&self, fraction: f32) {
        let mut log = self.log.lock().unwrap_or_else(PoisonError::into_inner);
        log.push(fraction);
        if log.len() == self.at {
            self.token.cancel();
        }
    }
}

/// `{1, n / 2, n - 1}` limited to `1..=n - 1`, ascending, without repeats.
fn cancel_points(n: usize) -> Vec<usize> {
    let mut points: Vec<usize> = [1, n / 2, n.saturating_sub(1)]
        .into_iter()
        .filter(|k| (1..n).contains(k))
        .collect();
    points.sort_unstable();
    points.dedup();
    points
}

/// The faults every run shares at index `at`: range, then order.
fn scan_common(fractions: &[f32], at: usize) -> Result<(), ProgressViolation> {
    let f = fractions[at];
    if !(0.0..=1.0).contains(&f) {
        return Err(ProgressViolation::OutOfRange { at });
    }
    if at > 0 && f < fractions[at - 1] {
        return Err(ProgressViolation::Decreasing { at });
    }
    Ok(())
}

fn scan_baseline(fractions: &[f32], max_step: f32) -> Result<(), ProgressViolation> {
    for at in 0..fractions.len() {
        scan_common(fractions, at)?;
        if fractions[at] == 1.0 && at + 1 < fractions.len() {
            return Err(ProgressViolation::EarlyCompletion { at });
        }
        if at > 0 && fractions[at] - fractions[at - 1] > max_step {
            return Err(ProgressViolation::StepTooLarge { at });
        }
    }
    if fractions.last() != Some(&1.0) {
        return Err(ProgressViolation::NotCompleted);
    }
    Ok(())
}

/// Fewer than `k` reports is judged first, whatever `op` returned; after that
/// the return value is judged before the rest of the log.
fn judge_cancelled(k: usize, cancelled: bool, fractions: &[f32]) -> Result<(), ProgressViolation> {
    if fractions.len() < k {
        return Err(ProgressViolation::NotCancelled { k });
    }
    if !cancelled {
        return Err(if fractions.last() == Some(&1.0) {
            ProgressViolation::NotCancelled { k }
        } else {
            ProgressViolation::OutputAfterCancel { k }
        });
    }
    for at in 0..fractions.len() {
        scan_common(fractions, at)?;
        if fractions[at] == 1.0 {
            return Err(ProgressViolation::EarlyCompletion { at });
        }
    }
    let extra = fractions.len() - k;
    if extra > 1 {
        return Err(ProgressViolation::ReportsAfterCancel { k, extra });
    }
    Ok(())
}
