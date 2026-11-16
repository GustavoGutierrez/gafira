//! Benchmarks of the transitions (XP13 of `specs/done/legacy/history/2026-11-02-layer-2/design.md`, RS3a).
//!
//! XP13 asks whether an `Rgba8` Dissolve at 1080p fits the 8 ms render share of 02 §6.2
//! with `Rgba8` mixed in linear light (D-48).

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use gafira_doc::contract::ParamValues;
use gafira_raster::transitions::{Dissolve, Transition};
use gafira_raster::{EffectCx, ImageView, ImageViewMut, PixelData, PixelDataMut};

const W: u32 = 1920;
const H: u32 = 1080;

/// A premultiplied, mostly opaque frame with some variation, so no pixel is special.
fn frame(seed: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(W as usize * H as usize * 4);
    for i in 0..W * H {
        let x = i.wrapping_mul(2_654_435_761).wrapping_add(seed);
        let a = 200 + (x >> 24) as u8 % 56;
        v.extend([
            (x as u8).min(a),
            ((x >> 8) as u8).min(a),
            ((x >> 16) as u8).min(a),
            a,
        ]);
    }
    v
}

/// XP13: `Rgba8` Dissolve, 1080p, `progress` 0.37 (a mid value, so every pixel is mixed).
fn bench_dissolve_rgba8_1080p(c: &mut Criterion) {
    let (from, to) = (frame(1), frame(2));
    let mut dst = vec![0u8; from.len()];
    let t = Dissolve::new();
    let cx = EffectCx::whole(1.0);
    let params = ParamValues::new();
    let mut group = c.benchmark_group("transitions");
    group.sample_size(10);
    group.bench_function("bench_dissolve_rgba8_1080p", |b| {
        b.iter(|| {
            t.run_cpu(
                &cx,
                &params,
                black_box(0.37),
                ImageView::new(W, H, PixelData::Rgba8(&from)).unwrap(),
                ImageView::new(W, H, PixelData::Rgba8(&to)).unwrap(),
                ImageViewMut::new(W, H, PixelDataMut::Rgba8(&mut dst)).unwrap(),
            )
            .unwrap();
            black_box(&dst);
        });
    });
    group.finish();
}

criterion_group!(benches, bench_dissolve_rgba8_1080p);
criterion_main!(benches);
