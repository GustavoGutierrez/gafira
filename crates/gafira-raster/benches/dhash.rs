//! Benchmark of the perceptual hash (ARQ-114, 02 §6.7 row "Hash perceptual de 12 MP < 100 ms",
//! 04 FD-28).

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use gafira_raster::{ImageView, PixelData, dhash};

const W: u32 = 4000;
const H: u32 = 3000;

/// A 12 MP frame with structure at every scale, so no cell is flat.
fn frame() -> Vec<u8> {
    let mut v = Vec::with_capacity(W as usize * H as usize * 4);
    for i in 0..W * H {
        let x = i.wrapping_mul(2_654_435_761);
        let (px, py) = (i % W, i / W);
        let g = ((px * 255 / W) ^ (py * 255 / H)) as u8;
        v.extend([
            g,
            (x >> 24) as u8,
            g.wrapping_add((x >> 16) as u8 & 15),
            255,
        ]);
    }
    v
}

/// `bench_dhash_12mp`: `Rgba8`, 4000 x 3000.
fn bench_dhash_12mp(c: &mut Criterion) {
    let src = frame();
    let view = ImageView::new(W, H, PixelData::Rgba8(&src)).unwrap();
    let mut group = c.benchmark_group("hash");
    group.sample_size(10);
    group.bench_function("bench_dhash_12mp", |b| {
        b.iter(|| black_box(dhash(black_box(view)).unwrap()));
    });
    group.finish();
}

criterion_group!(benches, bench_dhash_12mp);
criterion_main!(benches);
