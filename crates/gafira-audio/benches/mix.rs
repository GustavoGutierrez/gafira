//! `bench_mix_16` (CAL-15): 16 tracks, each Gain, Eq3Band and Pan, blocks of
//! 512 frames at 48 kHz, then the mix. The budget is 25 % of one core: one
//! block lasts 512 / 48 000 s = 10.67 ms, so a block must take under 2.67 ms.
//! The figure is read on the reference machine, not by `cargo test`.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use gafira_audio::{
    AudioBlock, AudioProcessor, Eq3Band, Gain, MixTrack, Pan, ParamSnapshot, ProcessorChain, mix,
};

const TRACKS: usize = 16;
const FRAMES: usize = 512;

fn bench_mix_16(c: &mut Criterion) {
    let mut chains: Vec<ProcessorChain> = (0..TRACKS)
        .map(|_| {
            let mut chain = ProcessorChain::new();
            chain.push(Box::new(Gain::new()));
            chain.push(Box::new(Eq3Band::new()));
            chain.push(Box::new(Pan::new()));
            chain.prepare(48_000, FRAMES, 2);
            chain
        })
        .collect();
    let mut params = ParamSnapshot::new();
    for (id, v) in [
        ("volume", 80.0),
        ("pan", 0.2),
        ("low", 6.0),
        ("mid", -4.0),
        ("high", 3.0),
    ] {
        params.set(id, v);
    }
    let source: Vec<Vec<Vec<f32>>> = (0..TRACKS)
        .map(|t| {
            (0..2)
                .map(|c| {
                    (0..FRAMES)
                        .map(|i| (((i * (t + 3 + c)) % 200) as f32 - 100.0) / 100.0)
                        .collect()
                })
                .collect()
        })
        .collect();
    let mut bufs = source.clone();
    let mut out = vec![vec![0.0_f32; FRAMES]; 2];
    c.bench_function("bench_mix_16", |b| {
        b.iter(|| {
            for (t, track) in bufs.iter_mut().enumerate() {
                for (dst, src) in track.iter_mut().zip(&source[t]) {
                    dst.copy_from_slice(src);
                }
                let mut slices: Vec<&mut [f32]> = track.iter_mut().map(Vec::as_mut_slice).collect();
                chains[t].process(&mut AudioBlock::new(&mut slices), &params);
            }
            let refs: Vec<Vec<&[f32]>> = bufs
                .iter()
                .map(|t| t.iter().map(Vec::as_slice).collect())
                .collect();
            let tracks: Vec<MixTrack<'_>> = refs
                .iter()
                .map(|r| MixTrack {
                    channels: r,
                    muted: false,
                })
                .collect();
            let mut slices: Vec<&mut [f32]> = out.iter_mut().map(Vec::as_mut_slice).collect();
            mix(&tracks, &mut AudioBlock::new(&mut slices));
            black_box(&out);
        });
    });
}

criterion_group!(benches, bench_mix_16);
criterion_main!(benches);
