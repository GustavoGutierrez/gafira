//! Benchmarks of `blend_u8` at the XP1 spike's unit of work.
//!
//! `specs/done/legacy/history/2026-11-07-gafira-render-cpu/spike.md` measured `blend_u8` and `blend_f32` over
//! 2 073 600 pixels (1920×1080) × 3 layers = 6 220 800 calls: 640.7 ms (`Normal`) and
//! 718.2 ms (`Multiply`) for `blend_u8`, against 90.7 ms and 213.1 ms for `blend_f32`.
//! These benches keep that unit so their numbers compare with the spike's directly.
//!
//! The bytes of `blend_u8` are frozen by the differential test in `src/blend.rs`; a
//! change that moves one byte breaks ARQ-111, not a benchmark budget.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use gafira_doc::BlendMode;
use gafira_raster::blend_u8;

const W: u32 = 1920;
const H: u32 = 1080;
const PIXELS: usize = W as usize * H as usize;

/// A premultiplied frame with plausible content: high alphas and a wide colour spread,
/// so no pixel is special. The generation is integer arithmetic, deterministic.
fn frame(seed: u64) -> Vec<[u8; 4]> {
    let mut px = Vec::with_capacity(PIXELS);
    for i in 0..PIXELS {
        let mut x = (i as u64)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(seed);
        x ^= x >> 33;
        x = x.wrapping_mul(0xA276_ED2C_D61F_7F7F);
        x ^= x >> 29;
        let a = 200 + (x >> 8) as u8 % 56;
        px.push([
            (x as u8).min(a),
            ((x >> 8) as u8).min(a),
            ((x >> 16) as u8).min(a),
            a,
        ]);
    }
    px
}

/// XP1's unit of work: one 1080p frame composited across three layers, `mode` per layer.
fn blend_1080p_three_layers(
    mode: BlendMode,
    base: &[[u8; 4]],
    l1: &[[u8; 4]],
    l2: &[[u8; 4]],
) -> u64 {
    let mut checksum = 0u64;
    for i in 0..PIXELS {
        let o1 = blend_u8(mode, base[i], l1[i]);
        let o2 = blend_u8(mode, o1, l2[i]);
        let o3 = blend_u8(mode, o2, l1[(i * 7 + 3) % PIXELS]);
        checksum = checksum.wrapping_add(o3[0] as u64 + o3[1] as u64 + o3[2] as u64);
    }
    checksum
}

fn bench_blend_u8_1080p_three_layers(c: &mut Criterion) {
    let base = frame(0);
    let l1 = frame(0x0F0F);
    let l2 = frame(0x7331);
    let mut group = c.benchmark_group("blend");
    group.warm_up_time(std::time::Duration::from_millis(400));
    group.sample_size(10);
    group.bench_function("blend_u8_normal_1080p_3l", |b| {
        b.iter(|| black_box(blend_1080p_three_layers(BlendMode::Normal, &base, &l1, &l2)))
    });
    group.bench_function("blend_u8_multiply_1080p_3l", |b| {
        b.iter(|| {
            black_box(blend_1080p_three_layers(
                BlendMode::Multiply,
                &base,
                &l1,
                &l2,
            ))
        })
    });
    group.finish();
}

criterion_group!(benches, bench_blend_u8_1080p_three_layers);
criterion_main!(benches);
