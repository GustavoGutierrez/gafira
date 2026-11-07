//! The numbers of `CB6`, all on one fixed 1080p page — the frame the guard in
//! `tests/cal_11_export_1080.rs` measures:
//!
//! - `bench_export_png_1080` (CAL-11): an `RgbaF32` export target from a cold
//!   cache, read back and encoded to PNG bytes through `png` — single-threaded,
//!   in-crate, with `png` standing in for `gafira-io-image` (FD-55).
//! - `bench_preview_1080_cold` / `_warm` (ARQ-143, `O14`, `W18`): an `Rgba8`
//!   target, `execute` + `read_back`, every tile a miss / every tile a hit.
//! - `bench_cancel_gap_1080` (FD-55 (d), ARQ-43): the time from the token's
//!   cancel to `execute` returning, with the cancel at the worst point of the
//!   heaviest tile.
//!
//! The crate reads no clock (ARQ-02, `clippy.toml`): every time here is
//! criterion's. The number is reported, not gated (`D10`, `SP5`).

#[path = "../tests/support/frame_1080.rs"]
mod frame_1080;

use std::cell::Cell;
use std::hint::black_box;
use std::sync::Arc;
use std::time::Duration;

use criterion::measurement::{Measurement, WallTime};
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use gafira_doc::contract::CancelToken;
use gafira_ports::common::Bytes;
use gafira_ports::render::{
    Deadline, RenderBackend, RenderError, RenderPlan, RenderQuality, TileCache, TileKey,
};
use gafira_raster::{ImageBuf, PixelFormat};
use gafira_render_cpu::CpuBackend;

/// CAL-11: one frame, built once outside the timed loop, rendered from a cold
/// cache and encoded inside it.
///
/// ARQ-114: CAL-11 is a §6.7 figure of this layer-2 card, so it has a bench
/// with that row's name — `bench_export_png_1080`, exactly (`D10`).
fn bench_export_png_1080(c: &mut Criterion) {
    let plan = frame_1080::plan(RenderQuality::Export);
    let mut group = c.benchmark_group("cal_11");
    group.sample_size(10);
    group.bench_function("bench_export_png_1080", |b| {
        b.iter(|| black_box(frame_1080::export_png(&plan)));
    });
    group.finish();
}

/// W18, ARQ-143: the same page on an `Rgba8` target, `execute` + `read_back`,
/// cold (every tile a miss) and warm (every tile a hit). One backend, as a
/// preview session keeps one; the cold cache is built outside the timed loop.
fn bench_preview_1080(c: &mut Criterion) {
    let plan = frame_1080::plan(RenderQuality::Preview);
    let mut backend = CpuBackend::new();
    let mut group = c.benchmark_group("arq_143");
    group.sample_size(10);
    group.bench_function("bench_preview_1080_cold", |b| {
        b.iter_batched(
            frame_1080::cache,
            |mut cache| frame_1080::render(&mut backend, &plan, PixelFormat::Rgba8, &mut cache),
            BatchSize::LargeInput,
        );
    });
    let mut warm = frame_1080::cache();
    frame_1080::render(&mut backend, &plan, PixelFormat::Rgba8, &mut warm);
    group.bench_function("bench_preview_1080_warm", |b| {
        b.iter(|| frame_1080::render(&mut backend, &plan, PixelFormat::Rgba8, &mut warm));
    });
    group.finish();
}

/// The tile the cancel lands in: row 3, column 1 holds most of the image over
/// the background, the most work of any tile of the frame — a probe of all 40
/// tiles measured it the longest stretch between two token checks.
const HEAVIEST: usize = 3 * 8 + 1;

/// A cold cache that cancels the token on the `at`-th lookup and stamps the
/// moment on criterion's clock. The lookup is the first thing the loop does
/// after its per-tile check, so the whole tile still runs: the worst gap.
struct CancelAt {
    token: CancelToken,
    at: usize,
    gets: Cell<usize>,
    cancelled: Cell<Option<<WallTime as Measurement>::Intermediate>>,
}

impl TileCache for CancelAt {
    fn get(&self, _key: &TileKey) -> Option<Arc<ImageBuf>> {
        let gets = self.gets.get() + 1;
        self.gets.set(gets);
        if gets == self.at {
            self.token.cancel();
            self.cancelled.set(Some(WallTime.start()));
        }
        None
    }

    fn put(&mut self, _key: TileKey, _tile: Arc<ImageBuf>) {}

    fn bytes(&self) -> Bytes {
        Bytes(0)
    }

    fn trim(&mut self, _budget: Bytes) {}
}

/// FD-55 (d), ARQ-43: from the token's cancel to `execute` returning.
fn cancel_gap(plan: &RenderPlan) -> Duration {
    let token = CancelToken::new();
    let mut cache = CancelAt {
        token: token.clone(),
        at: HEAVIEST + 1,
        gets: Cell::new(0),
        cancelled: Cell::new(None),
    };
    let mut backend = CpuBackend::new();
    let mut cx = backend
        .begin_frame(&frame_1080::target(PixelFormat::RgbaF32))
        .expect("the frame opens");
    let outcome = backend.execute(&mut cx, plan, &mut cache, Deadline::Unbounded, &token);
    let cancelled = cache.cancelled.get().expect("the token was cancelled");
    let gap = WallTime.end(cancelled);
    assert!(
        matches!(outcome, Err(RenderError::Cancelled(_))),
        "the frame answers the cancel"
    );
    gap
}

/// FD-55 (d), ARQ-43: the cancel gap of the export frame, against 100 ms.
fn bench_cancel_gap_1080(c: &mut Criterion) {
    let plan = frame_1080::plan(RenderQuality::Export);
    let mut group = c.benchmark_group("arq_43");
    group.sample_size(10);
    group.bench_function("bench_cancel_gap_1080", |b| {
        b.iter_custom(|iters| (0..iters).map(|_| cancel_gap(&plan)).sum());
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_export_png_1080,
    bench_preview_1080,
    bench_cancel_gap_1080
);
criterion_main!(benches);
