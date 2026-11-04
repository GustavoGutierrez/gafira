//! What a caller injects into the media engine (ARQ-138, ADR 0016).

use gafira_ports::system::{Clock, FileSystem};

/// One unit of work the pool runs.
pub type Job = Box<dyn FnOnce() + Send + Sync>;

/// The workers an operation runs on.
///
/// The pool belongs to the caller: the crate creates no thread (ARQ-02), so
/// the pool is a parameter. The jobs the crate hands over are independent
/// and it merges their results by index, so the result of an operation
/// never depends on the size of the pool nor on the order the jobs finish
/// in (ARQ-140).
pub trait WorkerPool: Send + Sync {
    /// How many workers the pool has.
    fn workers(&self) -> usize;

    /// Runs every job and returns when all of them are done.
    fn execute_all(&self, jobs: Vec<Job>);
}

/// The ports an operation of the crate needs from its caller.
///
/// The file system comes from `ports::FileSystem`, the time from
/// `ports::Clock` and the workers from the pool, never from the crate
/// itself (ARQ-138).
pub struct Ports<'a> {
    /// The file system the media is read from.
    pub file_system: &'a dyn FileSystem,
    /// The pool the operation runs on.
    pub pool: &'a dyn WorkerPool,
    /// The clock the operation measures with.
    pub clock: &'a dyn Clock,
}
