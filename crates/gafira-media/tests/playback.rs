//! The playback clock, the A/V drift and the frame dropping (ARQ-147, FV-11).
//!
//! The master is the audio clock, the video drops the frames it cannot reach
//! in time and never blocks the audio, and the drift of the preview stays
//! below the 40 ms of 02 §6.5.

mod playback_support;

use std::time::Duration;

use gafira_doc::contract::{CancelToken, NoProgress};
use gafira_media::{FrameDecision, PlaybackClock, SYNC_TOLERANCE, Speed, StepDirection};
use gafira_ports::media::MediaError;
use gafira_testkit::progress::{ProgressLog, check_progress_contract};
use gafira_time::{Flicks, FrameRate, TimeRange};
use playback_support::{Frames, ManualClock};

/// A time in milliseconds.
fn ms(value: i64) -> Flicks {
    Flicks::from_millis(value).expect("the time fits a flick count")
}

/// The range `[0 s, seconds)`.
fn range_seconds(seconds: i64) -> TimeRange {
    TimeRange::from_bounds(
        Flicks::ZERO,
        Flicks::from_seconds(seconds).expect("the range fits a flick count"),
    )
    .expect("the bounds are ordered")
}

/// A playback of `range` at `speed`, stepping whole 30 fps frames.
fn start(clock: &ManualClock, range: TimeRange, speed: Speed) -> PlaybackClock<'_> {
    PlaybackClock::new(clock, range, speed, FrameRate::FPS_30)
}

/// The duration of one frame of a 30 fps medium.
const ONE_FRAME: i64 = 23_520_000;

// ARQ-147: the master is the audio clock. It advances with the audio even
// when no video frame arrives, a dropped frame does not move it, and a whole
// playback that drops every frame still plays to the end (02 §6.5).
#[test]
fn arq_147_the_master_is_the_audio_and_the_video_never_blocks_it() {
    let clock = ManualClock::new();
    let mut playback = start(&clock, range_seconds(10), Speed::Normal);

    assert_eq!(
        playback.master(),
        Flicks::ZERO,
        "the master starts at the start of the range"
    );

    // No video frame arrives: the audio goes on alone.
    clock.advance(Duration::from_millis(100));
    assert_eq!(
        playback.master(),
        ms(100),
        "the master follows the audio with no video frame"
    );

    // A frame 150 ms late is dropped and the master keeps the audio time.
    clock.advance(Duration::from_millis(100));
    assert_eq!(
        playback.decide(ms(50)),
        FrameDecision::Drop,
        "a frame 150 ms late is dropped"
    );
    assert_eq!(
        playback.master(),
        ms(200),
        "a dropped frame does not move the master"
    );

    // A frame under the tolerance is presented, and it does not move the
    // master either: the master is the audio, never the video.
    assert_eq!(
        playback.decide(ms(190)),
        FrameDecision::Present,
        "a frame 10 ms late is presented"
    );
    assert_eq!(
        playback.master(),
        ms(200),
        "a presented frame does not move the master"
    );

    // A playback whose frames are all late runs to the end of the range
    // without the video holding the audio back (FV-11 (c)).
    let clock = ManualClock::new();
    let mut playback = start(&clock, range_seconds(1), Speed::Normal);
    let times = [ms(0), ms(10), ms(20), ms(30)];
    let mut frames = Frames::moving(&clock, &times, Duration::from_millis(200));
    let report = playback
        .play(&mut frames, &CancelToken::new(), &NoProgress)
        .expect("the playback ends");

    assert_eq!(
        (report.presented, report.dropped),
        (0, 4),
        "the video drops every late frame"
    );
    assert_eq!(
        playback.master(),
        ms(800),
        "the audio reached 800 ms meanwhile: the video never cut it"
    );
}

// ARQ-147: the boundary of §6.5 in both directions. A frame 39 ms late drops
// nothing; a frame 40 ms late or later is dropped and the next one is
// presented.
#[test]
fn arq_147_the_drop_boundary_is_39_ms_late_and_40_ms_late() {
    assert_eq!(
        SYNC_TOLERANCE,
        ms(40),
        "the tolerance of the preview is 40 ms (02 §6.5)"
    );

    let cases = [
        (0_i64, FrameDecision::Present),
        (1, FrameDecision::Present),
        (38, FrameDecision::Present),
        (39, FrameDecision::Present),
        (40, FrameDecision::Drop),
        (41, FrameDecision::Drop),
        (1_000, FrameDecision::Drop),
    ];
    for (late, expected) in cases {
        let clock = ManualClock::new();
        let mut playback = start(&clock, range_seconds(10), Speed::Normal);
        clock.advance(Duration::from_millis(
            u64::try_from(late).expect("the lateness is not negative"),
        ));

        assert_eq!(
            playback.decide(Flicks::ZERO),
            expected,
            "a frame {late} ms late against the tolerance of 40 ms"
        );
    }

    // The frame after a dropped one is presented.
    let clock = ManualClock::new();
    let mut playback = start(&clock, range_seconds(10), Speed::Normal);
    clock.advance(Duration::from_millis(40));
    assert_eq!(
        playback.decide(Flicks::ZERO),
        FrameDecision::Drop,
        "the frame 40 ms late is the dropped one"
    );
    clock.advance(Duration::from_millis(10));
    assert_eq!(
        playback.decide(ms(50)),
        FrameDecision::Present,
        "the next frame is presented at its own time"
    );
}

// ARQ-147: the drift between the audio and the video in the preview never
// passes the 40 ms tolerance, whatever the lateness of the frames.
#[test]
fn arq_147_the_preview_drift_never_passes_40_ms() {
    let clock = ManualClock::new();
    let mut playback = start(&clock, range_seconds(10), Speed::Normal);
    clock.advance(Duration::from_millis(39));
    assert_eq!(
        playback.decide(Flicks::ZERO),
        FrameDecision::Present,
        "a frame 39 ms late is presented"
    );
    assert_eq!(
        playback.drift(),
        ms(39),
        "a presented frame reports its lateness as the drift"
    );
    assert!(
        playback.drift() < SYNC_TOLERANCE,
        "the drift of the preview stays below 40 ms"
    );

    for late in 0..=120_u64 {
        let clock = ManualClock::new();
        let mut playback = start(&clock, range_seconds(10), Speed::Normal);
        clock.advance(Duration::from_millis(late));
        let decision = playback.decide(Flicks::ZERO);

        let expected = if late < 40 {
            FrameDecision::Present
        } else {
            FrameDecision::Drop
        };
        assert_eq!(decision, expected, "a frame {late} ms late");
        assert!(
            playback.drift() < SYNC_TOLERANCE,
            "a frame {late} ms late leaves a drift of {:?}, not below 40 ms",
            playback.drift()
        );
    }
}

// FV-11: the three speeds scale the presentation time, and the master stays
// the audio clock at every one of them.
#[test]
fn fv_11_the_speed_scales_the_presentation_time() {
    let cases = [
        (Speed::Half, 250_i64),
        (Speed::Normal, 500),
        (Speed::Double, 1_000),
    ];
    for (speed, expected) in cases {
        let clock = ManualClock::new();
        let mut playback = start(&clock, range_seconds(10), speed);
        clock.advance(Duration::from_millis(500));

        assert_eq!(
            playback.master(),
            ms(expected),
            "500 ms of audio are {expected} ms of presentation at {speed:?}"
        );
    }

    // The frame at 960 ms is 40 ms behind the master at 2× after 500 ms of
    // audio, and ahead of it at 0,5× and at 1×: the decision follows the
    // scaled presentation time, never the wall time.
    let cases = [
        (Speed::Half, FrameDecision::Present),
        (Speed::Normal, FrameDecision::Present),
        (Speed::Double, FrameDecision::Drop),
    ];
    for (speed, expected) in cases {
        let clock = ManualClock::new();
        let mut playback = start(&clock, range_seconds(10), speed);
        clock.advance(Duration::from_millis(500));

        assert_eq!(
            playback.decide(ms(960)),
            expected,
            "the frame at 960 ms after 500 ms of audio at {speed:?}"
        );
    }
}

// FV-11: a step follows the direction and stays inside the range, and the
// playback continues from the frame stepped to.
#[test]
fn fv_11_the_step_follows_the_direction_inside_the_range() {
    let clock = ManualClock::new();
    let mut playback = start(&clock, range_seconds(10), Speed::Normal);

    assert_eq!(
        playback.step(StepDirection::Forward),
        Flicks::from_ticks(ONE_FRAME),
        "one frame forward is one frame of 30 fps"
    );
    assert_eq!(
        playback.step(StepDirection::Forward),
        Flicks::from_ticks(2 * ONE_FRAME),
        "two frames forward are two frames of 30 fps"
    );
    assert_eq!(
        playback.step(StepDirection::Back),
        Flicks::from_ticks(ONE_FRAME),
        "a step back goes back one frame"
    );

    // The playback continues from the frame stepped to.
    clock.advance(Duration::from_millis(10));
    assert_eq!(
        playback.master(),
        Flicks::from_ticks(ONE_FRAME + ms(10).ticks()),
        "the playback continues from the frame stepped to"
    );

    // Backwards at the start: the step stops at the start of the range.
    let clock = ManualClock::new();
    let mut playback = start(&clock, range_seconds(10), Speed::Normal);
    assert_eq!(
        playback.step(StepDirection::Back),
        Flicks::ZERO,
        "a step back at the start of the range stays at the start"
    );

    // Forwards at the end: the step stops at the end of the range.
    let clock = ManualClock::new();
    let range = TimeRange::from_bounds(Flicks::ZERO, ms(50)).expect("the bounds are ordered");
    let mut playback = start(&clock, range, Speed::Normal);
    let mut last = Flicks::ZERO;
    for _ in 0..4 {
        last = playback.step(StepDirection::Forward);
    }
    assert_eq!(
        last,
        ms(50),
        "the step stops at the end of the range and never passes it"
    );
}

// ARQ-147: a range of zero length completes cleanly and drops no frame.
#[test]
fn arq_147_a_zero_length_range_completes_without_dropping() {
    let range = TimeRange::from_bounds(ms(1_000), ms(1_000)).expect("the bounds are ordered");

    let clock = ManualClock::new();
    let mut playback = start(&clock, range, Speed::Normal);
    assert!(
        playback.is_finished(),
        "a range of zero length is over at once"
    );
    assert_eq!(
        playback.decide(ms(1_000)),
        FrameDecision::Present,
        "the frame at the start of a range of zero length is presented"
    );

    let clock = ManualClock::new();
    let mut playback = start(&clock, range, Speed::Normal);
    let times = [ms(1_000)];
    let mut frames = Frames::still(&clock, &times);
    let log = ProgressLog::new();
    let report = playback
        .play(&mut frames, &CancelToken::new(), &log)
        .expect("a range of zero length completes");

    assert_eq!(
        (report.presented, report.dropped),
        (0, 0),
        "a range of zero length drops no frame"
    );
    assert_eq!(
        log.fractions(),
        vec![1.0],
        "a range of zero length completes with a single end report"
    );
}

// ARQ-138: the master is monotonic and comes from the injected clock: two
// readings of the same instant are equal, and a clock that goes back never
// makes the reading go back.
#[test]
fn arq_138_the_master_is_monotonic_and_comes_from_the_clock_port() {
    let clock = ManualClock::new();
    let mut playback = start(&clock, range_seconds(10), Speed::Normal);

    clock.advance(Duration::from_millis(500));
    let first = playback.master();
    assert_eq!(
        first,
        ms(500),
        "the reading is the reading of the injected clock"
    );
    assert_eq!(
        playback.master(),
        first,
        "two readings of the same instant are equal"
    );

    clock.set(Duration::ZERO);
    assert_eq!(
        playback.master(),
        first,
        "a clock that goes back does not move the master back"
    );

    clock.advance(Duration::from_millis(600));
    assert_eq!(
        playback.master(),
        ms(600),
        "the master follows the clock again once it passes the last reading"
    );

    clock.set(Duration::from_millis(100));
    assert_eq!(
        playback.master(),
        ms(600),
        "a second rewind does not move the master back either"
    );
    clock.advance(Duration::from_millis(50));
    assert_eq!(
        playback.master(),
        ms(600),
        "a reading below the last one is not reported"
    );
    clock.advance(Duration::from_millis(500));
    assert_eq!(
        playback.master(),
        ms(650),
        "the master takes the clock again when it passes the last reading"
    );
}

// ARQ-139: the playback is a long operation: it checks the cancel token once
// per frame and its progress reaches 1.0 only on a normal end.
// ARQ-43: that per-frame checkpoint is the one that bounds the answer of a
// cancellation. The 100 ms bound itself is not measured here: this crate
// has no bench.
#[test]
fn arq_139_the_playback_checks_the_cancel_token_per_frame() {
    let times = [ms(100), ms(200), ms(300), ms(400)];

    let clock = ManualClock::new();
    let mut playback = start(&clock, range_seconds(10), Speed::Normal);
    let mut frames = Frames::moving(&clock, &times, Duration::from_millis(100));
    let log = ProgressLog::new();
    let report = playback
        .play(&mut frames, &CancelToken::new(), &log)
        .expect("the playback ends");

    assert_eq!(
        (report.presented, report.dropped),
        (4, 0),
        "every frame on time is presented"
    );
    assert_eq!(
        log.fractions(),
        vec![0.2, 0.4, 0.6, 0.8, 1.0],
        "the fraction is the frames done over the frames planned, and 1.0 is the end"
    );

    // A token cancelled before the first frame reports nothing and never
    // reaches 1.0.
    let clock = ManualClock::new();
    let mut playback = start(&clock, range_seconds(10), Speed::Normal);
    let mut frames = Frames::moving(&clock, &times, Duration::from_millis(100));
    let token = CancelToken::new();
    token.cancel();
    let log = ProgressLog::new();
    let cancelled = playback.play(&mut frames, &token, &log);

    assert!(
        matches!(cancelled, Err(MediaError::Cancelled(_))),
        "a cancelled playback answers the cancelled error: {cancelled:?}"
    );
    assert!(
        log.fractions().is_empty(),
        "a cancelled playback reports nothing more: {:?}",
        log.fractions()
    );
}

// ARQ-139: the playback follows the progress contract of the testkit, with a
// cancel in the first, the middle and the penultimate report.
#[test]
fn arq_139_the_playback_follows_the_progress_contract() {
    let contract = check_progress_contract(0.25, |cancel, progress| {
        let clock = ManualClock::new();
        let mut playback = start(&clock, range_seconds(10), Speed::Normal);
        let times = [ms(100), ms(200), ms(300), ms(400)];
        let mut frames = Frames::moving(&clock, &times, Duration::from_millis(100));
        match playback.play(&mut frames, cancel, progress) {
            Ok(_) => Ok(false),
            Err(MediaError::Cancelled(_)) => Ok(true),
            Err(other) => Err(other),
        }
    })
    .expect("the contract check itself does not fail");

    assert_eq!(contract, Ok(()));
}
