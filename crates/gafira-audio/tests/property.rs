//! ARQ-110 (property) and ARQ-111: finite input in the domain gives finite,
//! normal output, and the same input gives the same bytes.

use gafira_audio::{
    AudioBlock, AudioProcessor, Fade, FadeDirection, Gain, MixTrack, PARAM_DURATION, PARAM_PAN,
    PARAM_VOLUME, Pan, ParamSnapshot, ProcessorChain, mix,
};
use proptest::prelude::*;

fn finite_sample() -> impl Strategy<Value = f32> {
    // Any finite value, and (half the time) a zero or subnormal one.
    (any::<u32>(), any::<bool>()).prop_map(|(bits, tiny)| {
        let x = f32::from_bits(if tiny { bits & 0x807F_FFFF } else { bits });
        if x.is_finite() { x } else { 0.25 }
    })
}

fn chain() -> ProcessorChain {
    let mut chain = ProcessorChain::new();
    chain.push(Box::new(Gain::new()));
    chain.push(Box::new(Fade::new(FadeDirection::In)));
    chain.push(Box::new(Fade::new(FadeDirection::Out)));
    chain.push(Box::new(Pan::new()));
    chain.prepare(48_000, 64, 2);
    chain
}

fn run_chain(input: &[Vec<f32>], volume: f64, duration: f64, pan: f64) -> Vec<Vec<f32>> {
    let mut params = ParamSnapshot::new();
    params.set(PARAM_VOLUME, volume);
    params.set(PARAM_DURATION, duration);
    params.set(PARAM_PAN, pan);
    let mut chain = chain();
    let mut out = input.to_vec();
    for _ in 0..3 {
        let mut slices: Vec<&mut [f32]> = out.iter_mut().map(Vec::as_mut_slice).collect();
        chain.process(&mut AudioBlock::new(&mut slices), &params);
    }
    out
}

proptest! {
    #[test]
    fn arq_110_finite_input_in_the_domain_gives_finite_normal_output(
        left in prop::collection::vec(finite_sample(), 32),
        right in prop::collection::vec(finite_sample(), 32),
        volume in 0.0_f64..=200.0,
        duration in 0.0_f64..=0.01,
        pan in -1.0_f64..=1.0,
    ) {
        let input = vec![left, right];
        let out = run_chain(&input, volume, duration, pan);
        prop_assert!(out.iter().flatten().all(|x| x.is_finite() && !x.is_subnormal()));
        let again = run_chain(&input, volume, duration, pan);
        let bits = |v: &[Vec<f32>]| v.iter().flatten().map(|x| x.to_bits()).collect::<Vec<_>>();
        prop_assert_eq!(bits(&out), bits(&again));
    }

    #[test]
    fn arq_110_the_mix_of_finite_tracks_is_finite_and_normal(
        a in prop::collection::vec(finite_sample(), 16),
        b in prop::collection::vec(finite_sample(), 16),
        c in prop::collection::vec(finite_sample(), 16),
    ) {
        let (sa, sb, sc) = ([a.as_slice()], [b.as_slice()], [c.as_slice()]);
        let tracks = [
            MixTrack { channels: &sa, muted: false },
            MixTrack { channels: &sb, muted: false },
            MixTrack { channels: &sc, muted: false },
        ];
        let mut out = vec![0.0_f32; 16];
        let mut slices = [out.as_mut_slice()];
        mix(&tracks, &mut AudioBlock::new(&mut slices));
        prop_assert!(out.iter().all(|x| x.is_finite() && !x.is_subnormal()));
    }
}
