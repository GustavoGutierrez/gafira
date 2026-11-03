//! ARQ-23: 16 active tracks of 512 frames are processed and mixed without
//! allocating. The guard only acts in debug builds; its negative control is in
//! `gafira-testkit` (`tests/alloc_guard.rs`).

#[cfg(debug_assertions)]
use assert_no_alloc::AllocDisabler;
use assert_no_alloc::assert_no_alloc;
use gafira_audio::{
    AudioBlock, AudioProcessor, Eq3Band, Fade, FadeDirection, Gain, LoudnessNormalizer, MixTrack,
    Pan, ParamSnapshot, ProcessorChain, mix,
};

#[cfg(debug_assertions)]
#[global_allocator]
static GUARD: AllocDisabler = AllocDisabler;

const TRACKS: usize = 16;
const FRAMES: usize = 512;

fn chain() -> ProcessorChain {
    let mut c = ProcessorChain::new();
    c.push(Box::new(Gain::new()));
    c.push(Box::new(Eq3Band::new()));
    c.push(Box::new(LoudnessNormalizer::new()));
    c.push(Box::new(Pan::new()));
    c
}

fn params(round: usize) -> ParamSnapshot {
    let r = round as f64;
    let mut s = ParamSnapshot::new();
    s.set("volume", 50.0 + 10.0 * r);
    s.set("pan", (r / 4.0).sin());
    s.set("low", 3.0 * r - 12.0);
    s.set("mid", 12.0 - 4.0 * r);
    s.set("high", if round.is_multiple_of(2) { 0.0 } else { 9.0 });
    s.set("duration", 0.01);
    s.set("gain", 2.0 * r - 6.0);
    s
}

#[test]
fn arq_23_sixteen_tracks_of_512_frames_process_and_mix_without_allocating() {
    let mut chains: Vec<ProcessorChain> = (0..TRACKS).map(|_| chain()).collect();
    let mut fades_in: Vec<Fade> = (0..TRACKS).map(|_| Fade::new(FadeDirection::In)).collect();
    let mut fades_out: Vec<Fade> = (0..TRACKS).map(|_| Fade::new(FadeDirection::Out)).collect();
    for c in &mut chains {
        c.prepare(48_000, FRAMES, 2);
    }
    for f in fades_in.iter_mut().chain(fades_out.iter_mut()) {
        f.prepare(48_000, FRAMES, 2);
    }
    let mut bufs: Vec<Vec<Vec<f32>>> = (0..TRACKS)
        .map(|t| {
            (0..2)
                .map(|c| {
                    (0..FRAMES)
                        .map(|i| match (i + t) % 97 {
                            0 => f32::NAN,
                            1 => f32::INFINITY,
                            _ => (((i * (t + 3 + c)) % 200) as f32 - 100.0) / 100.0,
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    let mut out = vec![vec![0.0_f32; FRAMES]; 2];
    for round in 0..8 {
        let snapshot = params(round);
        assert_no_alloc(|| {
            for (t, track) in bufs.iter_mut().enumerate() {
                if let [l, r] = track.as_mut_slice() {
                    let mut chans: [&mut [f32]; 2] = [l.as_mut_slice(), r.as_mut_slice()];
                    let mut block = AudioBlock::new(&mut chans);
                    chains[t].process(&mut block, &snapshot);
                    fades_in[t].process(&mut block, &snapshot);
                    fades_out[t].process(&mut block, &snapshot);
                }
            }
            let refs: [[&[f32]; 2]; TRACKS] =
                std::array::from_fn(|t| [bufs[t][0].as_slice(), bufs[t][1].as_slice()]);
            let tracks: [MixTrack<'_>; TRACKS] = std::array::from_fn(|t| MixTrack {
                channels: &refs[t],
                muted: t == 5,
            });
            if let [l, r] = out.as_mut_slice() {
                let mut chans: [&mut [f32]; 2] = [l.as_mut_slice(), r.as_mut_slice()];
                mix(&tracks, &mut AudioBlock::new(&mut chans));
            }
        });
        assert!(out.iter().flatten().all(|x| x.is_finite()), "round {round}");
    }
}
