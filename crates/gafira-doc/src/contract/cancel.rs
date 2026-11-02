//! Cancellation and progress of long operations (ARQ-112, ADR 0012).

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::msg::{Message, MessageKey, ToMessage};

/// A cancellation handle bound to one generation (02 §4.7).
///
/// Clones share the generation counter. A handle is cancelled once the counter
/// is no longer the generation it was issued with.
#[derive(Clone, Debug, Default)]
pub struct CancelToken {
    generation: Arc<AtomicU64>,
    issued: u64,
}

impl CancelToken {
    /// A live token on a new generation counter.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts a new generation. Every token bound to an older one is cancelled.
    #[must_use]
    pub fn next_generation(&self) -> CancelToken {
        let previous = self.generation.fetch_add(1, Ordering::Relaxed);
        CancelToken {
            generation: Arc::clone(&self.generation),
            issued: previous.saturating_add(1),
        }
    }

    /// Cancels this token's generation. A no-op from a stale token.
    pub fn cancel(&self) {
        // `fetch_max`, not `fetch_add`: a stale handle must not cancel a newer generation.
        self.generation
            .fetch_max(self.issued.saturating_add(1), Ordering::Relaxed);
    }

    /// True once this token's generation is over.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.generation.load(Ordering::Relaxed) != self.issued
    }

    /// `Err(Cancelled)` when [`is_cancelled`](Self::is_cancelled).
    ///
    /// # Errors
    /// [`Cancelled`] when the token is cancelled.
    pub fn check(&self) -> Result<(), Cancelled> {
        if self.is_cancelled() {
            Err(Cancelled)
        } else {
            Ok(())
        }
    }
}

/// The one cancelled value of every engine (ADR 0012).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cancelled;

impl fmt::Display for Cancelled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("operation cancelled")
    }
}

impl std::error::Error for Cancelled {}

impl ToMessage for Cancelled {
    fn to_message(&self) -> Message {
        Message {
            key: MessageKey("error.job.cancelled"),
            args: vec![],
        }
    }
}
