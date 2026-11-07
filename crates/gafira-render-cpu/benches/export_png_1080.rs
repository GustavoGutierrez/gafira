//! CAL-11's number (`CB6`): `bench_export_png_1080`, one fixed 1080p page
//! rendered on an `RgbaF32` export target from a cold cache, read back and
//! encoded to PNG bytes through `png` — single-threaded, in-crate, with `png`
//! standing in for `gafira-io-image` (FD-55). The frame is the one the guard in
//! `tests/cal_11_export_1080.rs` measures.
//!
//! The crate reads no clock (ARQ-02, `clippy.toml`): every time here is
//! criterion's. The number is reported, not gated (`D10`, `SP5`).

#[path = "../tests/support/frame_1080.rs"]
mod frame_1080;

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use gafira_ports::render::RenderQuality;

/// CAL-11: one frame, built once outside the timed loop, rendered from a cold
/// cache and encoded inside it.
fn bench_export_png_1080(c: &mut Criterion) {
    let plan = frame_1080::plan(RenderQuality::Export);
    let mut group = c.benchmark_group("cal_11");
    group.sample_size(10);
    group.bench_function("bench_export_png_1080", |b| {
        b.iter(|| black_box(frame_1080::export_png(&plan)));
    });
    group.finish();
}

criterion_group!(benches, bench_export_png_1080);
criterion_main!(benches);
