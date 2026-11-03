//! ARQ-127, FV-10 (b): `LoudnessNormalizer` applies the Nivelar gain, aside
//! from the clip's Volume.

use gafira_audio::{
    AudioBlock, AudioProcessor, Gain, LoudnessNormalizer, PARAM_GAIN, PARAM_VOLUME, ParamSnapshot,
    ProcessorChain,
};

fn run(p: &mut dyn AudioProcessor, chans: &mut [Vec<f32>], params: &ParamSnapshot) {
    let mut slices: Vec<&mut [f32]> = chans.iter_mut().map(Vec::as_mut_slice).collect();
    p.process(&mut AudioBlock::new(&mut slices), params);
}

fn snapshot(pairs: &[(&'static str, f64)]) -> ParamSnapshot {
    let mut s = ParamSnapshot::new();
    for &(id, v) in pairs {
        s.set(id, v);
    }
    s
}

fn prepared() -> LoudnessNormalizer {
    let mut n = LoudnessNormalizer::new();
    n.prepare(48_000, 512, 2);
    n
}

fn db(db: f64) -> f64 {
    10_f64.powf(db / 20.0)
}

#[test]
fn arq_127_a_gain_of_16_db_is_applied_with_the_volume_at_100_percent_untouched() {
    // The clip's Gain stays at 100 % and Nivelar adds +16 dB after it.
    let mut chain = ProcessorChain::new();
    chain.push(Box::new(Gain::new()));
    chain.push(Box::new(LoudnessNormalizer::new()));
    chain.prepare(48_000, 512, 2);
    let params = snapshot(&[(PARAM_VOLUME, 100.0), (PARAM_GAIN, 16.0)]);
    let mut b = vec![vec![0.1_f32; 64]; 2];
    run(&mut chain, &mut b, &params);
    let want = (0.1_f64 * db(16.0)) as f32;
    assert!(
        b.iter().flatten().all(|&x| (x - want).abs() <= 1e-6),
        "{:?}",
        &b[0][..4]
    );
    assert!(want > 0.6, "the gain is not capped at the 200 % of Volume");
    // The Gain alone, with the same snapshot, still gives x1.0.
    let mut gain = Gain::new();
    gain.prepare(48_000, 512, 2);
    let mut b = vec![vec![0.1_f32; 64]; 2];
    run(&mut gain, &mut b, &params);
    assert!(b.iter().flatten().all(|&x| x == 0.1));
}

#[test]
fn arq_127_the_volume_of_the_clip_does_not_reach_the_normalizer() {
    let mut n = prepared();
    let mut b = vec![vec![0.5_f32; 16]; 2];
    run(&mut n, &mut b, &snapshot(&[(PARAM_VOLUME, 50.0)]));
    assert!(b.iter().flatten().all(|&x| x == 0.5), "volume alone: x1");
    let mut n = prepared();
    let mut b = vec![vec![0.5_f32; 16]; 2];
    run(
        &mut n,
        &mut b,
        &snapshot(&[(PARAM_VOLUME, 50.0), (PARAM_GAIN, 6.0)]),
    );
    let want = (0.5_f64 * db(6.0)) as f32;
    assert!(b.iter().flatten().all(|&x| (x - want).abs() <= 1e-6));
}

#[test]
fn arq_127_without_a_gain_the_signal_is_unchanged_bit_for_bit() {
    let mut n = prepared();
    let src: Vec<f32> = (0..64).map(|i| (i as f32 * 0.37).sin() * 0.9).collect();
    let mut b = vec![src.clone(), src.clone()];
    run(&mut n, &mut b, &ParamSnapshot::new());
    assert_eq!(b[0], src);
}

#[test]
fn arq_127_the_gain_is_constant_over_the_whole_signal_across_blocks() {
    let mut n = prepared();
    let params = snapshot(&[(PARAM_GAIN, -7.3)]);
    let src: Vec<f32> = (0..1000).map(|i| (i as f32 * 0.05).sin() * 0.8).collect();
    let g = db(-7.3);
    let mut at = 0;
    for len in [1, 17, 64, 200, 300, 418] {
        let mut b = vec![src[at..at + len].to_vec(); 2];
        run(&mut n, &mut b, &params);
        for (i, &x) in b[0].iter().enumerate() {
            let want = (f64::from(src[at + i]) * g) as f32;
            assert!((x - want).abs() <= 1e-6, "sample {}: {x} vs {want}", at + i);
        }
        at += len;
    }
    assert_eq!(at, 1000);
}

#[test]
fn arq_127_a_new_gain_applies_from_the_first_sample_of_the_next_block_without_a_ramp() {
    let mut n = prepared();
    let mut b = vec![vec![1.0_f32; 8]; 2];
    run(&mut n, &mut b, &snapshot(&[(PARAM_GAIN, 0.0)]));
    let mut b = vec![vec![1.0_f32; 8]; 2];
    run(&mut n, &mut b, &snapshot(&[(PARAM_GAIN, 6.0)]));
    let want = db(6.0) as f32;
    assert!(b.iter().flatten().all(|&x| (x - want).abs() <= 1e-6));
}

#[test]
fn arq_123_a_non_finite_gain_keeps_the_last_valid_one_and_huge_gains_stay_finite() {
    let mut n = prepared();
    run(
        &mut n,
        &mut vec![vec![1.0_f32; 4]; 2],
        &snapshot(&[(PARAM_GAIN, 6.0)]),
    );
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut b = vec![vec![1.0_f32; 4]; 2];
        run(&mut n, &mut b, &snapshot(&[(PARAM_GAIN, bad)]));
        assert!(
            b.iter()
                .flatten()
                .all(|&x| (x - db(6.0) as f32).abs() <= 1e-6)
        );
    }
    // The range is the whole of f64: the gain is large, the output saturates and stays finite.
    let mut n = prepared();
    let mut b = vec![vec![0.5_f32, 0.0, -0.5, 1e-30]; 2];
    run(&mut n, &mut b, &snapshot(&[(PARAM_GAIN, 1e300)]));
    assert!(b.iter().flatten().all(|x| x.is_finite()), "{b:?}");
    assert_eq!((b[0][0], b[0][1], b[0][2]), (f32::MAX, 0.0, -f32::MAX));
    let mut n = prepared();
    let mut b = vec![vec![0.5_f32; 4]; 2];
    run(&mut n, &mut b, &snapshot(&[(PARAM_GAIN, -1e300)]));
    assert!(b.iter().flatten().all(|&x| x == 0.0));
}
