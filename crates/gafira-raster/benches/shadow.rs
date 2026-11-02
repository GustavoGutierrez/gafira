//! Benchmark of the parallel shadow (ARQ-114, 02 §6.7 row "Sombra < 100 ms con radio ≤ 40 px en
//! 1080p", P11 of `specs/changes/layer-2/design.md`).

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use gafira_doc::contract::{ParamValue, ParamValues};
use gafira_raster::effects::{Effect, ParallelShadow};
use gafira_raster::{EffectCx, ImageView, ImageViewMut, PixelData, PixelDataMut};

const W: u32 = 1920;
const H: u32 = 1080;

/// A premultiplied frame whose alpha varies, so the blur has no flat shortcut.
fn frame() -> Vec<u8> {
    let mut v = Vec::with_capacity(W as usize * H as usize * 4);
    for i in 0..W * H {
        let x = i.wrapping_mul(2_654_435_761);
        let a = (x >> 24) as u8;
        v.extend([
            (x as u8).min(a),
            ((x >> 8) as u8).min(a),
            ((x >> 16) as u8).min(a),
            a,
        ]);
    }
    v
}

/// `bench_shadow_1080`: `Rgba8`, 1080p, radius 40 px (sigma 20), offset 8 px, scale 1.
fn bench_shadow_1080(c: &mut Criterion) {
    let src = frame();
    let mut dst = vec![0u8; src.len()];
    let shadow = ParallelShadow::new();
    let cx = EffectCx { scale: 1.0 };
    let mut params = ParamValues::new();
    for (id, v) in [
        ("blur", 40.0),
        ("offset_x", 8.0),
        ("offset_y", 8.0),
        ("opacity", 60.0),
    ] {
        params.set(id, ParamValue::Float(v));
    }
    let mut group = c.benchmark_group("effects");
    group.sample_size(10);
    group.bench_function("bench_shadow_1080", |b| {
        b.iter(|| {
            shadow
                .run_cpu(
                    &cx,
                    &params,
                    ImageView::new(W, H, PixelData::Rgba8(&src)).unwrap(),
                    ImageViewMut::new(W, H, PixelDataMut::Rgba8(&mut dst)).unwrap(),
                )
                .unwrap();
            black_box(&dst);
        });
    });
    group.finish();
}

criterion_group!(benches, bench_shadow_1080);
criterion_main!(benches);
