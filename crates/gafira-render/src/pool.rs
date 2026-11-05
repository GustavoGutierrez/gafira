//! The pool of workers the engine borrows (ARQ-138).
//!
//! The crate never creates a thread: the caller owns the pool and hands it in
//! as a parameter. The engine only asks the pool to run one job per index and
//! to hand the results back in the order of the indices, so the result never
//! depends on the order the jobs finish in (ARQ-140).

/// The pool of workers the engine borrows.
///
/// The pool belongs to the caller. The engine asks it to run one job per
/// index and to return the results in the order of the indices, so that the
/// graph and the keys come out the same whatever the pool does (ARQ-140).
pub trait WorkerPool: Sync {
    /// Runs `work` once for every index in `0..count` and returns the results
    /// in the order of the indices.
    fn map<T: Send>(&self, count: usize, work: impl Fn(usize) -> T + Sync) -> Vec<T>;
}

/// A pool that runs the jobs on the calling thread, in order.
#[derive(Clone, Copy, Debug, Default)]
pub struct InlinePool;

impl WorkerPool for InlinePool {
    fn map<T: Send>(&self, count: usize, work: impl Fn(usize) -> T + Sync) -> Vec<T> {
        (0..count).map(work).collect()
    }
}

impl<P: WorkerPool + ?Sized> WorkerPool for &P {
    fn map<T: Send>(&self, count: usize, work: impl Fn(usize) -> T + Sync) -> Vec<T> {
        (**self).map(count, work)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ARQ-140: the results come back in the order of the indices, whatever
    // order the jobs ran in.
    // ARQ-138: the pool of workers is a parameter of the crate: the jobs run
    // on the pool the caller hands in, here on the calling thread, and the
    // crate creates no thread of its own.
    #[test]
    fn arq_140_the_results_keep_the_order_of_the_indices() {
        let pool = InlinePool;
        assert_eq!(pool.map(4, |at| at * 2), vec![0, 2, 4, 6]);
    }
}
