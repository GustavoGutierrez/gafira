//! FD-38 (c). ARQ-112 and ARQ-132: progress, cancellation and determinism of `segment`.

use std::sync::Arc;

use gafira_doc::contract::{CancelToken, NoProgress, Progress};
use gafira_raster::{ImageView, PixelData};
use gafira_segment::{ClassicSegmenter, SegError, SegHints, SegZone, Segmentation, Segmenter};
use gafira_testkit::progress::{ProgressLog, check_progress_contract};
use gafira_testkit::scenes::{Scene, fixed_scene, reference_scene};

fn seg(threads: usize) -> ClassicSegmenter {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();
    ClassicSegmenter::new(Arc::new(pool))
}

fn run(
    seg: &ClassicSegmenter,
    scene: &Scene,
    token: &CancelToken,
    progress: &dyn Progress,
) -> Result<Segmentation, SegError> {
    let view = ImageView::new(scene.width, scene.height, PixelData::Rgba8(&scene.rgba)).unwrap();
    let hints = SegHints {
        zone: SegZone::Foreground,
        ..SegHints::default()
    };
    seg.segment(view, &hints, progress, token)
}

// ARQ-112: on the reference input (flat background, centred subject, 1024x768)
// the fraction rises by at most 0.02 between two reports, never decreases, stays
// in [0, 1], reaches 1.0 only at the end; cancelling at report 1, the middle one
// and the one before the last returns the cancelled error with at most one report
// after it and no output.
// Mutants: a report that goes down; reports after a cancel; a result delivered
// after a cancel; a step above 0.02.
#[test]
fn arq_112_progress_and_cancellation_on_the_reference_input() {
    let scene = reference_scene();
    let s = seg(2);
    let verdict = check_progress_contract(0.02, |token, progress| {
        match run(&s, &scene, token, progress) {
            Ok(_) => Ok(false),
            Err(SegError::Cancelled(_)) => Ok(true),
            Err(e) => Err(e),
        }
    })
    .unwrap();
    assert_eq!(verdict, Ok(()));
}

// ARQ-112: with a pool of one thread two runs give the same sequence of fractions.
// Mutant: a fraction that depends on a race or on the clock.
#[test]
fn arq_112_one_thread_gives_the_same_fractions_twice() {
    let scene = reference_scene();
    let s = seg(1);
    let once = || {
        let log = ProgressLog::new();
        run(&s, &scene, &CancelToken::new(), &log).unwrap();
        log.fractions()
    };
    let (a, b) = (once(), once());
    assert!(a.len() >= 50, "{} reports", a.len());
    assert_eq!(a, b);
}

// ARQ-112: the fraction follows the work model, not the time: the number of
// reports is the same whatever the pool.
#[test]
fn arq_112_the_report_count_does_not_depend_on_the_pool() {
    let scene = reference_scene();
    let count = |threads: usize| {
        let log = ProgressLog::new();
        run(&seg(threads), &scene, &CancelToken::new(), &log).unwrap();
        log.fractions().len()
    };
    assert_eq!(count(1), count(4));
}

// ARQ-111 / ARQ-130 / ARQ-10: the mask and `reliable` are the same whatever the thread
// count, on a scene that is reduced (4000x3000 > 2048) and one that is not.
// Mutant: a parallel reduction that sums in an order that depends on the pool.
#[test]
fn arq_130_the_mask_does_not_depend_on_the_thread_count() {
    for (index, w, h) in [(8, 4000, 3000), (13, 640, 480)] {
        let scene = fixed_scene(index, w, h);
        let base = run(&seg(1), &scene, &CancelToken::new(), &NoProgress).unwrap();
        for threads in [2, 4, 7] {
            let other = run(&seg(threads), &scene, &CancelToken::new(), &NoProgress).unwrap();
            assert_eq!(base, other, "{} {w}x{h}, {threads} threads", scene.name);
        }
    }
}
