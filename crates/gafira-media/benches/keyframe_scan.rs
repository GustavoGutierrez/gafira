//! The keyframe scan of ARQ-144 (02 §6.5, FIJADA: ≤ 1 s per minute of 1080p
//! material) and the XP6 claim of the design of layer 3.
//!
//! The crate has no clock (ARQ-02, ADR 0016), so the bench never reads the
//! wall clock: the only time source is the `Clock` double it injects. The
//! double advances with the work the scan does, one packet read at a time,
//! so the time the scan reports is the work it did, counted by the injected
//! clock.
//!
//! The material is one minute of 1080p at 30 fps: 1800 packets with a
//! keyframe every 150 of them and the payload of the real file the layer-3
//! spike measured (12 keyframes, 1800 packets, about 1007 bytes per packet).
//!
//! The unit of the clock is the cost of one packet read the spike measured
//! on that real file: 86 ns (0.086 µs, median, with the reservation of the
//! payload, the dearest of its three variants). The number the bench reports
//! is therefore the cost of the scan over that file, and not a number of the
//! bench itself.
//!
//! The bench fails when the scan does not fit in the budget of the crate, so
//! the claim of ARQ-144 is checked every time the bench runs.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use gafira_doc::contract::{CancelToken, NoProgress};
use gafira_media::{Job, KeyframeIndex, WorkerPool};
use gafira_ports::common::UnixTime;
use gafira_ports::media::{
    CodecId, CodecParams, Demuxer, MediaError, Packet, SeekMode, StreamIdx, StreamInfo,
};
use gafira_ports::system::Clock;
use gafira_testkit::ports::container::FakeDemuxer;
use gafira_time::{Flicks, FrameRate};

/// The width of the material, in pixels.
const WIDTH: u32 = 1920;
/// The height of the material, in pixels.
const HEIGHT: u32 = 1080;
/// The rate of the material, in frames per second.
const FPS: u32 = 30;
/// The length of the material, in seconds.
const SECONDS: u64 = 60;
/// The frames between two keyframes: the file of the spike had 12 keyframes
/// in 1800 frames.
const KEYFRAME_EVERY: u64 = 150;
/// The payload of a packet, in bytes: the average of the real file
/// (1 812 355 bytes over 1800 packets).
const PAYLOAD: usize = 1007;
/// The cost of one packet read, the one the spike of layer 3 measured on the
/// real file (0.086 µs, median, with the reservation of the payload).
const COST_PER_PACKET: Duration = Duration::from_nanos(86);

/// The `Clock` double of the bench: its instant advances with the work the
/// scan does, never with the wall clock (ARQ-02).
#[derive(Clone, Default)]
struct WorkClock {
    nanos: Arc<AtomicU64>,
}

impl WorkClock {
    /// The instant the clock is at.
    fn at(&self) -> Duration {
        Duration::from_nanos(self.nanos.load(Ordering::Relaxed))
    }

    /// The clock moves forward by `step`.
    fn advance(&self, step: Duration) {
        self.nanos.fetch_add(
            u64::try_from(step.as_nanos()).unwrap_or(u64::MAX),
            Ordering::Relaxed,
        );
    }
}

impl Clock for WorkClock {
    /// The wall time of the bench is always the epoch.
    fn now(&self) -> UnixTime {
        UnixTime(0)
    }

    /// The instant the work has reached.
    fn monotonic(&self) -> Duration {
        self.at()
    }
}

/// The demuxer of the bench: the packets of the material, one read at a
/// time, and the clock moves with every packet read.
struct WorkDemuxer {
    inner: FakeDemuxer,
    clock: WorkClock,
    read: usize,
}

impl Demuxer for WorkDemuxer {
    /// The streams of the material.
    fn streams(&self) -> &[StreamInfo] {
        self.inner.streams()
    }

    /// The next packet of the material, and the clock moves with it.
    fn read_packet(&mut self) -> Result<Option<Packet>, MediaError> {
        let packet = self.inner.read_packet()?;
        if packet.is_some() {
            self.read += 1;
            self.clock.advance(COST_PER_PACKET);
        }
        Ok(packet)
    }

    /// Moves to `time`.
    fn seek(&mut self, time: Flicks, mode: SeekMode) -> Result<(), MediaError> {
        self.inner.seek(time, mode)
    }
}

/// The pool of the bench: it runs the jobs on the calling thread.
struct InlinePool;

impl WorkerPool for InlinePool {
    /// The pool has one worker.
    fn workers(&self) -> usize {
        1
    }

    /// Runs every job.
    fn execute_all(&self, jobs: Vec<Job>) {
        for job in jobs {
            job();
        }
    }
}

/// The video stream of the material.
fn stream() -> StreamInfo {
    StreamInfo {
        index: StreamIdx(0),
        codec: CodecId::H264,
        params: CodecParams::Video {
            width: WIDTH,
            height: HEIGHT,
            frame_rate: FrameRate::new(FPS, 1).expect("the rate fits"),
        },
        duration: Some(
            Flicks::from_seconds(i64::try_from(SECONDS).expect("the length fits"))
                .expect("the length fits"),
        ),
    }
}

/// The packets of the material: one per frame of the minute.
fn packets() -> Vec<Packet> {
    let frame = FrameRate::new(FPS, 1)
        .expect("the rate fits")
        .flicks_per_frame()
        .ticks();
    let frames = i64::from(FPS) * i64::try_from(SECONDS).expect("the length fits");
    let every = i64::try_from(KEYFRAME_EVERY).expect("the interval fits");
    (0..frames)
        .map(|index| {
            let at = index * frame;
            Packet {
                stream: StreamIdx(0),
                pts: Flicks::from_ticks(at),
                dts: Flicks::from_ticks(at),
                keyframe: index % every == 0,
                data: vec![0_u8; PAYLOAD],
            }
        })
        .collect()
}

/// One full scan of the material, with the clock the bench injects.
///
/// It returns the index, the report of the crate and the number of packets
/// the scan read.
fn scan(clock: &WorkClock) -> (KeyframeIndex, gafira_media::IndexReport, usize) {
    let mut demuxer = WorkDemuxer {
        inner: FakeDemuxer::new(vec![stream()], packets()),
        clock: clock.clone(),
        read: 0,
    };
    let pool = InlinePool;
    let cancel = CancelToken::new();
    let (index, report) = KeyframeIndex::build(
        &mut demuxer,
        StreamIdx(0),
        &pool,
        clock,
        &cancel,
        &NoProgress,
    )
    .expect("the scan finishes");
    (index, report, demuxer.read)
}

/// `bench_keyframe_scan_1080p_1min`: the full keyframe scan of one minute of
/// 1080p material, against the budget of ARQ-144.
///
/// The measure is the work the scan did, counted by the injected clock: the
/// packets it read and the keyframes it found. The wall clock never enters
/// the measure (ARQ-02, ADR 0016).
fn main() {
    let clock = WorkClock::default();
    let (index, report, read) = scan(&clock);

    println!(
        "bench_keyframe_scan_1080p_1min: {WIDTH}x{HEIGHT} @{FPS} fps, {SECONDS} s, \
         packets read {read}, keyframes found {}, clock unit {} ns per packet",
        index.len(),
        COST_PER_PACKET.as_nanos()
    );
    println!(
        "  scan reported   : {:?} ({} packets)",
        report.elapsed, read
    );
    println!(
        "  budget          : {:?} (one second per minute of material, ARQ-144)",
        report.budget
    );
    println!("  within budget   : {}", report.within_budget());

    assert!(
        report.within_budget(),
        "the scan of one minute of 1080p material does not fit in the budget of ARQ-144: \
         {:?} against {:?}",
        report.elapsed,
        report.budget
    );
}
