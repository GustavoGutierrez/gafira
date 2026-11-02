//! Benchmarks of `ClassicSegmenter` (ARQ-114, 02 §6.7 and ARQ-43, CAL-06, P8 of
//! `specs/changes/layer-2/design.md`). The crate has no clock (ARQ-02), so the
//! timing comes from criterion's own measurement, never from `Instant::now`.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use criterion::measurement::{Measurement, WallTime};
use criterion::{Criterion, criterion_group, criterion_main};
use gafira_doc::contract::{CancelToken, NoProgress, Progress};
use gafira_raster::{ImageView, PixelData};
use gafira_segment::{ClassicSegmenter, SegHints, SegZone, Segmenter};
use gafira_testkit::scenes::{Scene, fixed_scene};

/// A textured scene with a holed subject, 4000x3000: a long side of 4000 px.
fn scene() -> Scene {
    fixed_scene(9, 4000, 3000)
}

/// A segmenter on a pool of 4 threads. Figures from other machines are indicative
/// until measured on the mid-range hardware of D-03 (Q-48).
fn segmenter() -> ClassicSegmenter {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .expect("a pool of 4 threads");
    ClassicSegmenter::new(Arc::new(pool))
}

fn hints() -> SegHints {
    SegHints {
        zone: SegZone::Foreground,
        ..SegHints::default()
    }
}

/// `bench_segment_4000`: a whole run on 4000x3000, CAL-06, FD-38 (b): < 5 s on CPU.
fn bench_segment_4000(c: &mut Criterion) {
    let scene = scene();
    let seg = segmenter();
    let hints = hints();
    let view = ImageView::new(scene.width, scene.height, PixelData::Rgba8(&scene.rgba)).unwrap();
    let mut group = c.benchmark_group("segment");
    group.sample_size(10);
    group.bench_function("bench_segment_4000", |b| {
        b.iter(|| {
            seg.segment(view, &hints, &NoProgress, &CancelToken::new())
                .expect("a mask")
        });
    });
    group.finish();
}

/// Stamps the time of every report (criterion's clock, from the start of the run).
struct Stamps {
    clock: WallTime,
    start: <WallTime as Measurement>::Intermediate,
    at: Mutex<Vec<Duration>>,
}

impl Progress for Stamps {
    fn report(&self, _fraction: f32) {
        let now = self.clock.end(self.start);
        self.at
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(now);
    }
}

/// The longest stretch of work between two token checks, on one run: every check
/// comes with a report (ARQ-112), so it is the largest gap between two reports,
/// the first measured from the start and the last to the end of the run. It
/// bounds the time `segment` takes to answer a cancel (ARQ-43).
fn longest_unit(seg: &ClassicSegmenter, scene: &Scene, hints: &SegHints) -> Duration {
    let clock = WallTime;
    let stamps = Stamps {
        clock: WallTime,
        start: clock.start(),
        at: Mutex::new(vec![Duration::ZERO]),
    };
    let view = ImageView::new(scene.width, scene.height, PixelData::Rgba8(&scene.rgba)).unwrap();
    seg.segment(view, hints, &stamps, &CancelToken::new())
        .expect("a mask");
    let end = clock.end(stamps.start);
    let mut at = stamps
        .at
        .into_inner()
        .unwrap_or_else(PoisonError::into_inner);
    at.push(end);
    at.windows(2).map(|w| w[1] - w[0]).max().unwrap_or_default()
}

/// `bench_cancel_segment`: the longest unit of work between two token checks on
/// 4000x3000, which bounds the cancel response of ARQ-43 (<= 100 ms).
fn bench_cancel_segment(c: &mut Criterion) {
    let scene = scene();
    let seg = segmenter();
    let hints = hints();
    let mut group = c.benchmark_group("segment");
    group.sample_size(10);
    group.bench_function("bench_cancel_segment", |b| {
        b.iter_custom(|iters| (0..iters).map(|_| longest_unit(&seg, &scene, &hints)).sum());
    });
    group.finish();
}

criterion_group!(benches, bench_segment_4000, bench_cancel_segment);
criterion_main!(benches);
