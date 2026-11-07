//! CAL-11's guard and the determinism of the frame it measures (`CB6`).
//!
//! The number is criterion's (`benches/export_png_1080.rs`); this file only
//! keeps it honest: an optimised build that takes 2 s or more for one 1080p
//! frame rendered and encoded fails, and a debug build skips the check and
//! says so, because its time says nothing about the budget (`D10`). Nothing
//! here is a CI gate on the number (ARQ-40 is a separate change).
//!
//! ARQ-114 says the §6.7 figures are measured by a bench and are not a
//! `cargo test` gate. This guard is not that gate (`D10`): the gate ARQ-114
//! forbids is a timing assert that `cargo test` runs on every build and that a
//! slow machine or a debug run fails for the wrong reason. Here the CI's
//! `cargo test` builds in debug, where the guard skips and never fails on
//! time; the assert runs only in a build someone optimised on purpose
//! (`cargo test --release`), and the number itself is the bench's.
//!
//! The crate reads no clock (ARQ-02, `clippy.toml`), and that holds for its
//! tests too: the time comes from criterion's `WallTime`, as in
//! `gafira-segment`'s benches.

#[path = "support/frame_1080.rs"]
mod frame_1080;

use std::time::Duration;

use criterion::measurement::{Measurement, WallTime};
use gafira_ports::render::RenderQuality;

/// CAL-11: one 1080p frame rendered and encoded, strictly under 2 s.
const BUDGET: Duration = Duration::from_secs(2);

/// What the guard concluded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verdict {
    /// The build is not optimised, so nothing was measured.
    Skipped,
    /// The frame took this long, under the budget.
    Within(Duration),
    /// The frame took this long, at or over the budget.
    Over(Duration),
}

/// The guard's rule: measure only an optimised build, and compare strictly.
fn judge(optimised: bool, budget: Duration, measure: impl FnOnce() -> Duration) -> Verdict {
    if !optimised {
        return Verdict::Skipped;
    }
    let took = measure();
    if took < budget {
        Verdict::Within(took)
    } else {
        Verdict::Over(took)
    }
}

/// The wall time of one export frame and its PNG, on criterion's clock.
fn time_one_frame() -> Duration {
    let plan = frame_1080::plan(RenderQuality::Export);
    let clock = WallTime;
    let start = clock.start();
    let png = frame_1080::export_png(&plan);
    let elapsed = clock.end(start);
    assert!(!png.is_empty(), "the frame encoded");
    elapsed
}

// CAL-11, D10: the guard measures an optimised build and only that, and the
// budget is strict — 2 s is over it.
#[test]
fn cal_11_the_guard_measures_only_an_optimised_build_and_is_strict() {
    let cases = [
        (false, Duration::from_secs(9), Verdict::Skipped),
        (
            true,
            Duration::from_millis(1999),
            Verdict::Within(Duration::from_millis(1999)),
        ),
        (
            true,
            Duration::from_secs(2),
            Verdict::Over(Duration::from_secs(2)),
        ),
        (
            true,
            Duration::from_secs(3),
            Verdict::Over(Duration::from_secs(3)),
        ),
    ];
    for (optimised, took, expected) in cases {
        let mut ran = false;
        let verdict = judge(optimised, BUDGET, || {
            ran = true;
            took
        });
        assert_eq!(verdict, expected, "optimised {optimised}, {took:?}");
        assert_eq!(ran, optimised, "the frame runs only in an optimised build");
    }
}

// CAL-11, D10: one 1080p frame rendered and encoded takes < 2 s in an
// optimised build; a debug build skips, says so, and never fails on time.
#[test]
fn cal_11_one_1080p_frame_renders_and_encodes_under_two_seconds() {
    let optimised = !cfg!(debug_assertions);
    let verdict = judge(optimised, BUDGET, time_one_frame);
    assert_eq!(
        verdict == Verdict::Skipped,
        cfg!(debug_assertions),
        "the guard skips a debug build and measures an optimised one: {verdict:?}"
    );
    match verdict {
        Verdict::Skipped => eprintln!(
            "CAL-11 guard skipped: a debug build is not optimised, so its time says \
             nothing about the 2 s budget; run `cargo test --release` to measure it"
        ),
        Verdict::Within(took) => {
            eprintln!("CAL-11 guard: one 1080p frame rendered and encoded in {took:?} (< 2 s)");
        }
        Verdict::Over(took) => {
            panic!("CAL-11: one 1080p frame took {took:?}, the budget is < 2 s")
        }
    }
}

// CAL-11, ARQ-111: the frame is built from constants, so two builds of it
// render byte-identical PNGs of the whole page.
#[test]
fn cal_11_the_fixed_frame_renders_byte_identical_pngs() {
    let first = frame_1080::export_png(&frame_1080::plan(RenderQuality::Export));
    let second = frame_1080::export_png(&frame_1080::plan(RenderQuality::Export));
    // IHDR's width and height, big-endian, at bytes 16..24 of a PNG.
    assert_eq!(
        &first[16..20],
        &frame_1080::WIDTH.to_be_bytes(),
        "the page's width"
    );
    assert_eq!(
        &first[20..24],
        &frame_1080::HEIGHT.to_be_bytes(),
        "the page's height"
    );
    assert!(
        first == second,
        "two builds of the frame encode the same bytes"
    );
}
