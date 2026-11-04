//! The keyframe index of one medium (ARQ-144).
//!
//! The index is built by scanning the packets of the container and keeping
//! the time of the keyframes. It lives in memory: in this cut it is written
//! neither to the `.gafira` (02 §7.1) nor to the user cache, and it is
//! rebuilt when the medium is opened again.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use gafira_doc::contract::{CancelToken, Progress};
use gafira_ports::media::{CodecParams, Demuxer, MediaError, StreamIdx, StreamInfo};
use gafira_ports::system::Clock;
use gafira_time::{FLICKS_PER_SECOND, Flicks};

use crate::ports::{Job, WorkerPool};

/// The budget of an index scan: one second per minute of material
/// (ARQ-144, 02 §6.5, FIJADA).
pub const INDEX_BUDGET_PER_MINUTE: Duration = Duration::from_secs(1);

/// What one index build cost.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexReport {
    /// The time the scan took, from the injected clock.
    pub elapsed: Duration,
    /// The budget the scan had, or `None` when the medium has no duration
    /// to compute it from.
    pub budget: Option<Duration>,
}

impl IndexReport {
    /// True when the scan stayed inside its budget.
    #[must_use]
    pub fn within_budget(&self) -> bool {
        self.budget.is_none_or(|budget| self.elapsed <= budget)
    }
}

/// The index of the keyframes of one medium (ARQ-144).
///
/// The keyframe times are kept in ascending order, so a search is a binary
/// one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyframeIndex {
    times: Vec<Flicks>,
}

impl KeyframeIndex {
    /// An index with no keyframe: there is nothing to land on.
    #[must_use]
    pub fn empty() -> Self {
        Self { times: Vec::new() }
    }

    /// The keyframe times, in ascending order.
    #[must_use]
    pub fn times(&self) -> &[Flicks] {
        &self.times
    }

    /// How many keyframes the index holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.times.len()
    }

    /// True when the index holds no keyframe.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.times.is_empty()
    }

    /// The keyframe at or before `time`.
    ///
    /// A time before the first keyframe lands on that first keyframe: it is
    /// the keyframe that covers the time and the one the decode starts
    /// from. An empty index has no keyframe to return and says so instead
    /// of failing (ARQ-144).
    #[must_use]
    pub fn keyframe_at_or_before(&self, time: Flicks) -> Option<Flicks> {
        let at = self.times.partition_point(|t| *t <= time);
        if at == 0 {
            self.times.first().copied()
        } else {
            self.times.get(at - 1).copied()
        }
    }

    /// The bytes that identify the index: the keyframe times, little-endian.
    ///
    /// Two builds of the same medium have the same bytes, whatever the
    /// pool (ARQ-140).
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.times.len() * 8);
        for time in &self.times {
            bytes.extend_from_slice(&time.ticks().to_le_bytes());
        }
        bytes
    }

    /// Builds the index by scanning the packets of `stream` (ARQ-144).
    ///
    /// The scan reads every packet of the container and keeps the time of
    /// the keyframes of `stream`. It checks the cancel token once per
    /// packet, so at least once per frame, and reports the work done over
    /// the work planned (ARQ-139). The keyframe times are computed over the
    /// pool and merged by chunk, so the index does not depend on the pool
    /// (ARQ-140).
    ///
    /// # Errors
    ///
    /// [`MediaError::Cancelled`] when the caller cancels the scan, and the
    /// error of the container when reading a packet fails.
    pub fn build(
        demuxer: &mut dyn Demuxer,
        stream: StreamIdx,
        pool: &dyn WorkerPool,
        clock: &dyn Clock,
        cancel: &CancelToken,
        progress: &dyn Progress,
    ) -> Result<(Self, IndexReport), MediaError> {
        let started = clock.monotonic();
        let planned = planned_frames(demuxer.streams(), stream);
        let mut work = Work::new(planned);
        let mut records = Vec::new();
        while let Some(packet) = demuxer.read_packet()? {
            cancel.check()?;
            if packet.stream == stream {
                work.step(progress);
                records.push(Record {
                    time: packet.pts,
                    keyframe: packet.keyframe,
                });
            }
        }
        let times = keyframe_times(&records, pool);
        cancel.check()?;
        work.finish(progress);
        let elapsed = clock.monotonic().saturating_sub(started);
        Ok((
            Self { times },
            IndexReport {
                elapsed,
                budget: budget_for(demuxer.streams(), stream),
            },
        ))
    }
}

/// One packet of the scanned stream, kept without its payload.
#[derive(Clone, Copy, Debug)]
struct Record {
    time: Flicks,
    keyframe: bool,
}

/// The work model of a long operation (ARQ-139): the units done over the
/// units planned.
///
/// The fraction never falls, stays in `[0, 1]` and reaches `1.0` only when
/// the operation ends well.
struct Work {
    planned: u64,
    done: u64,
}

impl Work {
    /// A work of `planned` units. Zero planned units mean that the total is
    /// unknown: the operation then reports only its end.
    fn new(planned: u64) -> Self {
        Self { planned, done: 0 }
    }

    /// Reports the fraction of the work already done and counts one unit.
    fn step(&mut self, progress: &dyn Progress) {
        if self.planned > 0 && self.done < self.planned {
            progress.report(self.done as f32 / self.planned as f32);
        }
        self.done += 1;
    }

    /// Reports the end of the work.
    fn finish(&mut self, progress: &dyn Progress) {
        progress.report(1.0);
    }
}

/// The keyframe times of the records, computed over the pool (ARQ-140).
///
/// The records are split into as many chunks as the pool has workers and
/// the results are merged by chunk, so the index does not depend on the
/// pool nor on the order the chunks finish in.
fn keyframe_times(records: &[Record], pool: &dyn WorkerPool) -> Vec<Flicks> {
    let chunk_size = records.len().div_ceil(pool.workers().max(1)).max(1);
    let chunks: Vec<&[Record]> = records.chunks(chunk_size).collect();
    let slots: Arc<Mutex<Vec<Vec<Flicks>>>> = Arc::new(Mutex::new(vec![Vec::new(); chunks.len()]));
    let jobs: Vec<Job> = chunks
        .iter()
        .enumerate()
        .map(|(index, chunk)| {
            let slots = Arc::clone(&slots);
            let chunk = chunk.to_vec();
            let job: Job = Box::new(move || {
                let times: Vec<Flicks> = chunk
                    .iter()
                    .filter(|record| record.keyframe)
                    .map(|record| record.time)
                    .collect();
                let mut slots = slots.lock().unwrap_or_else(PoisonError::into_inner);
                slots[index] = times;
            });
            job
        })
        .collect();
    pool.execute_all(jobs);
    let slots = slots.lock().unwrap_or_else(PoisonError::into_inner);
    slots.iter().flatten().copied().collect()
}

/// The number of frames the scan plans to read: the duration of the stream
/// times its frame rate (ARQ-139: the units planned).
fn planned_frames(streams: &[StreamInfo], stream: StreamIdx) -> u64 {
    let Some(info) = streams.iter().find(|s| s.index == stream) else {
        return 0;
    };
    let CodecParams::Video { frame_rate, .. } = info.params else {
        return 0;
    };
    let Some(duration) = info.duration else {
        return 0;
    };
    let frames = (i128::from(duration.ticks()) * i128::from(frame_rate.num()))
        / (i128::from(FLICKS_PER_SECOND) * i128::from(frame_rate.den()));
    u64::try_from(frames).unwrap_or(u64::MAX)
}

/// The budget of the scan: one second per minute of material (ARQ-144).
fn budget_for(streams: &[StreamInfo], stream: StreamIdx) -> Option<Duration> {
    let info = streams.iter().find(|s| s.index == stream)?;
    let duration = info.duration?;
    let per_minute = i128::try_from(INDEX_BUDGET_PER_MINUTE.as_nanos()).unwrap_or(i128::MAX);
    let nanos = (i128::from(duration.ticks()) * per_minute) / (i128::from(FLICKS_PER_SECOND) * 60);
    Some(Duration::from_nanos(
        u64::try_from(nanos).unwrap_or(u64::MAX),
    ))
}
