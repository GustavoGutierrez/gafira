//! The keyframe index of a medium (ARQ-144).

mod support;

use std::path::Path;
use std::time::Duration;

use gafira_doc::contract::{CancelToken, NoProgress};
use gafira_media::{KeyframeIndex, Medium, Ports, SeekOutcome};
use gafira_ports::media::{SeekMode, StreamIdx};
use gafira_testkit::ports::container::{FakeContainer, FakePacket};
use gafira_testkit::ports::decoder::FrameDecoder;
use gafira_time::Flicks;
use support::{FakeClock, FakeFileSystem, FakePool, MEDIUM};

/// The medium of the acceptance criteria: keyframes at 0 s, 2 s and 4 s, a
/// frame every second and one at 3,5 s.
fn acceptance_packets() -> Vec<FakePacket> {
    vec![
        FakePacket::Keyframe(0),
        FakePacket::Frame(1_000),
        FakePacket::Keyframe(2_000),
        FakePacket::Frame(3_000),
        FakePacket::Frame(3_500),
        FakePacket::Keyframe(4_000),
        FakePacket::Frame(5_000),
    ]
}

/// Opens a medium built from these packets.
fn open(packets: &[FakePacket], duration_ms: i64, fps: u32) -> Medium {
    let bytes = FakeContainer::medium(1920, 1080, fps, duration_ms, packets);
    let file_system = FakeFileSystem::with_file(MEDIUM, bytes);
    let pool = FakePool::new(2);
    let clock = FakeClock::new(Duration::from_millis(10));
    let ports = Ports {
        file_system: &file_system,
        pool: &pool,
        clock: &clock,
    };
    let token = CancelToken::new();
    Medium::open(
        &ports,
        Path::new(MEDIUM),
        &FakeContainer,
        StreamIdx(0),
        &token,
        &NoProgress,
    )
    .expect("the medium opens")
}

// ARQ-144: a medium with keyframes at 0 s, 2 s and 4 s: a seek to 3,5 s
// lands on the 2 s keyframe and, exactly, on the 3,5 s frame.
#[test]
fn arq_144_the_seek_lands_on_the_previous_keyframe_and_the_exact_seek_on_the_frame() {
    let mut medium = open(&acceptance_packets(), 6_000, 2);
    let token = CancelToken::new();
    let time = Flicks::from_millis(3_500).expect("3,5 s fits");

    assert_eq!(
        medium
            .seek(
                &mut FrameDecoder::new(),
                time,
                SeekMode::Keyframe,
                &token,
                &NoProgress,
            )
            .expect("the seek lands"),
        SeekOutcome::Keyframe(Flicks::from_seconds(2).expect("2 s fits")),
    );
    assert_eq!(
        medium
            .seek(
                &mut FrameDecoder::new(),
                time,
                SeekMode::Exact,
                &token,
                &NoProgress,
            )
            .expect("the seek lands"),
        SeekOutcome::Exact(Flicks::from_millis(3_500).expect("3,5 s fits")),
    );
}

// ARQ-144: a medium whose first keyframe is at 5 s, asked for 3 s, returns
// the 5 s keyframe.
#[test]
fn arq_144_a_time_before_the_first_keyframe_returns_the_first_keyframe() {
    let mut medium = open(
        &[
            FakePacket::Keyframe(5_000),
            FakePacket::Frame(6_000),
            FakePacket::Keyframe(8_000),
        ],
        8_000,
        2,
    );
    let token = CancelToken::new();

    assert_eq!(
        medium
            .seek(
                &mut FrameDecoder::new(),
                Flicks::from_seconds(3).expect("3 s fits"),
                SeekMode::Keyframe,
                &token,
                &NoProgress,
            )
            .expect("the seek lands"),
        SeekOutcome::Keyframe(Flicks::from_seconds(5).expect("5 s fits")),
    );
}

// ARQ-144: an empty index does not panic and reports the absence of a
// keyframe.
#[test]
fn arq_144_an_empty_index_reports_the_absence_of_a_keyframe() {
    let index = KeyframeIndex::empty();
    assert!(index.is_empty());
    assert_eq!(
        index.keyframe_at_or_before(Flicks::from_seconds(3).expect("3 s fits")),
        None,
    );

    let mut medium = open(&[FakePacket::Frame(1_000)], 1_000, 1);
    let token = CancelToken::new();
    assert_eq!(
        medium
            .seek(
                &mut FrameDecoder::new(),
                Flicks::from_seconds(1).expect("1 s fits"),
                SeekMode::Keyframe,
                &token,
                &NoProgress,
            )
            .expect("the seek does not fail"),
        SeekOutcome::NoKeyframe,
    );
}

// ARQ-144: a seek to the time of a keyframe lands on that keyframe.
#[test]
fn arq_144_a_seek_to_the_time_of_a_keyframe_lands_on_it() {
    let mut medium = open(&acceptance_packets(), 6_000, 2);
    let token = CancelToken::new();

    assert_eq!(
        medium
            .seek(
                &mut FrameDecoder::new(),
                Flicks::from_seconds(2).expect("2 s fits"),
                SeekMode::Keyframe,
                &token,
                &NoProgress,
            )
            .expect("the seek lands"),
        SeekOutcome::Keyframe(Flicks::from_seconds(2).expect("2 s fits")),
    );
}

// ARQ-144: closing and opening the medium again rebuilds the index and the
// search gives the same results.
#[test]
fn arq_144_reopening_the_medium_rebuilds_the_index() {
    let first = open(&acceptance_packets(), 6_000, 2);
    assert_eq!(first.index().len(), 3);
    let times = first.index().times().to_vec();
    let searched = first
        .index()
        .keyframe_at_or_before(Flicks::from_millis(3_500).expect("3,5 s fits"));
    drop(first);

    let second = open(&acceptance_packets(), 6_000, 2);
    assert_eq!(second.index().times(), times.as_slice());
    assert_eq!(
        second
            .index()
            .keyframe_at_or_before(Flicks::from_millis(3_500).expect("3,5 s fits")),
        searched,
    );
}

// ARQ-144: the index lives in memory: opening a medium writes no file, so
// no index file reaches the user cache.
#[test]
fn arq_144_opening_a_medium_writes_no_file() {
    let bytes = FakeContainer::medium(1920, 1080, 2, 6_000, &acceptance_packets());
    let file_system = FakeFileSystem::with_file(MEDIUM, bytes);
    let pool = FakePool::new(1);
    let clock = FakeClock::new(Duration::from_millis(10));
    let ports = Ports {
        file_system: &file_system,
        pool: &pool,
        clock: &clock,
    };
    let token = CancelToken::new();

    let medium = Medium::open(
        &ports,
        Path::new(MEDIUM),
        &FakeContainer,
        StreamIdx(0),
        &token,
        &NoProgress,
    )
    .expect("the medium opens");

    assert_eq!(medium.index().len(), 3);
    assert_eq!(file_system.calls(), vec!["open".to_string()]);
}

// ARQ-144: the scan measures its cost with the injected clock and compares
// it with the budget of one second per minute of material.
#[test]
fn arq_144_the_scan_reports_its_cost_against_the_budget() {
    for (step, within) in [
        (Duration::from_millis(10), true),
        (Duration::from_millis(200), false),
    ] {
        let bytes = support::reference_medium();
        let file_system = FakeFileSystem::with_file(MEDIUM, bytes);
        let pool = FakePool::new(1);
        let clock = FakeClock::new(step);
        let ports = Ports {
            file_system: &file_system,
            pool: &pool,
            clock: &clock,
        };
        let token = CancelToken::new();

        let medium = Medium::open(
            &ports,
            Path::new(MEDIUM),
            &FakeContainer,
            StreamIdx(0),
            &token,
            &NoProgress,
        )
        .expect("the medium opens");

        let report = medium.report();
        assert_eq!(report.budget, Some(Duration::from_millis(100)));
        assert_eq!(report.within_budget(), within);
    }
}
