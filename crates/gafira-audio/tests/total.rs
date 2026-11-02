//! ARQ-123: `process` is total, for every processor of the crate.

use gafira_audio::{
    AudioBlock, AudioProcessor, Eq3Band, Fade, FadeDirection, Gain, PARAM_DURATION, PARAM_HIGH,
    PARAM_LOW, PARAM_MID, PARAM_PAN, PARAM_VOLUME, Pan, ParamSnapshot, ProcessorChain,
};

type Maker = fn() -> Box<dyn AudioProcessor>;

fn makers() -> Vec<(&'static str, Maker)> {
    vec![
        ("gain", || Box::new(Gain::new())),
        ("fade_in", || Box::new(Fade::new(FadeDirection::In))),
        ("fade_out", || Box::new(Fade::new(FadeDirection::Out))),
        ("pan", || Box::new(Pan::new())),
        ("eq3band", || Box::new(Eq3Band::new())),
        ("chain", || {
            let mut chain = ProcessorChain::new();
            chain.push(Box::new(Gain::new()));
            chain.push(Box::new(Eq3Band::new()));
            chain.push(Box::new(Pan::new()));
            Box::new(chain)
        }),
        ("empty_chain", || Box::new(ProcessorChain::new())),
    ]
}

fn params() -> ParamSnapshot {
    let mut s = ParamSnapshot::new();
    s.set(PARAM_VOLUME, 150.0);
    s.set(PARAM_DURATION, 0.001);
    s.set(PARAM_PAN, 0.3);
    s.set(PARAM_LOW, 5.0);
    s.set(PARAM_MID, -7.0);
    s.set(PARAM_HIGH, 9.0);
    s
}

fn run(p: &mut dyn AudioProcessor, chans: &mut [Vec<f32>], params: &ParamSnapshot) {
    let mut slices: Vec<&mut [f32]> = chans.iter_mut().map(Vec::as_mut_slice).collect();
    p.process(&mut AudioBlock::new(&mut slices), params);
}

fn bits(chans: &[Vec<f32>]) -> Vec<Vec<u32>> {
    chans
        .iter()
        .map(|c| c.iter().map(|f| f.to_bits()).collect())
        .collect()
}

fn hostile() -> Vec<Vec<f32>> {
    let row = |s: f32| {
        vec![
            0.25,
            f32::NAN,
            s,
            -0.5,
            f32::INFINITY,
            1.0,
            f32::NEG_INFINITY,
            0.0,
        ]
    };
    vec![row(0.1), row(-0.1)]
}

#[test]
fn arq_123_nan_and_infinities_give_finite_output_and_later_blocks_stay_finite() {
    for (name, make) in makers() {
        let mut p = make();
        p.prepare(48_000, 64, 2);
        let mut bad = hostile();
        run(p.as_mut(), &mut bad, &params());
        assert!(bad.iter().flatten().all(|x| x.is_finite()), "{name}");
        for round in 0..4 {
            let mut next = vec![vec![0.3_f32; 32]; 2];
            run(p.as_mut(), &mut next, &params());
            assert!(
                next.iter().flatten().all(|x| x.is_finite()),
                "{name} {round}"
            );
        }
    }
}

#[test]
fn arq_123_a_non_finite_sample_counts_as_zero() {
    for (name, make) in makers() {
        let mut p = make();
        p.prepare(48_000, 64, 1);
        let mut b = vec![vec![f32::NAN, f32::INFINITY, f32::NEG_INFINITY]];
        run(p.as_mut(), &mut b, &params());
        assert_eq!(b[0], vec![0.0; 3], "{name}");
    }
}

#[test]
fn arq_123_no_output_is_subnormal_after_signal_then_zeros_or_from_tiny_input() {
    for (name, make) in makers() {
        let mut p = make();
        p.prepare(48_000, 512, 2);
        let amp = 10_f32.powf(-6.0 / 20.0);
        let mut n = 0_usize;
        for _ in 0..(2 * 48_000 / 512) {
            let signal = n < 48_000;
            let mut b: Vec<Vec<f32>> = (0..2)
                .map(|_| {
                    (0..512)
                        .map(|i| {
                            if signal {
                                amp * (((n + i) as f32) * 0.05).sin()
                            } else {
                                0.0
                            }
                        })
                        .collect()
                })
                .collect();
            n += 512;
            run(p.as_mut(), &mut b, &params());
            assert!(b.iter().flatten().all(|x| !x.is_subnormal()), "{name}");
        }
        // Subnormal in, and a normal value that becomes subnormal when scaled.
        let tiny = f32::MIN_POSITIVE * 0.5;
        let mut b = vec![vec![tiny, -tiny, f32::MIN_POSITIVE, 1e-45]; 2];
        let mut half = ParamSnapshot::new();
        half.set(PARAM_VOLUME, 50.0);
        run(p.as_mut(), &mut b, &half);
        run(p.as_mut(), &mut b, &half);
        assert!(b.iter().flatten().all(|x| !x.is_subnormal()), "{name}");
    }
}

/// A sample as an unprocessed block must leave it: NaN, `±∞` and subnormals
/// become 0, everything else is identical.
fn cleaned(x: f32) -> f32 {
    if x.is_finite() && !x.is_subnormal() {
        x
    } else {
        0.0
    }
}

fn mixed(channels: usize, frames: usize) -> Vec<Vec<f32>> {
    (0..channels)
        .map(|c| {
            (0..frames)
                .map(|i| match i % 6 {
                    0 => f32::NAN,
                    1 => 0.5 + c as f32,
                    2 => f32::MIN_POSITIVE * 0.5,
                    3 => f32::INFINITY,
                    4 => f32::NEG_INFINITY,
                    _ => -0.25,
                })
                .collect()
        })
        .collect()
}

/// `(prepare rate, max_block, channels, block channels, block frames)`.
const REFUSED: [(u32, usize, u16, usize, usize); 6] = [
    (0, 64, 2, 2, 12),
    (48_000, 0, 2, 2, 12),
    (48_000, 64, 0, 2, 12),
    (48_000, 11, 2, 2, 12), // one frame above max_block
    (48_000, 64, 2, 1, 12), // 1 channel after prepare with 2
    (48_000, 64, 2, 3, 12), // 3 channels after prepare with 2
];

#[test]
fn arq_123_an_unprocessed_block_is_only_cleaned() {
    for (name, make) in makers() {
        for (rate, max, ch, block_channels, frames) in REFUSED {
            let mut p = make();
            p.prepare(rate, max, ch);
            let mut b = mixed(block_channels, frames);
            let want: Vec<Vec<f32>> = b
                .iter()
                .map(|c| c.iter().map(|&x| cleaned(x)).collect())
                .collect();
            run(p.as_mut(), &mut b, &params());
            assert_eq!(
                bits(&b),
                bits(&want),
                "{name} {rate} {max} {ch} {block_channels}"
            );
        }
    }
}

#[test]
fn arq_123_a_processor_that_was_never_prepared_only_cleans_the_block() {
    for (name, make) in makers() {
        let mut p = make();
        let mut b = mixed(2, 12);
        let want: Vec<f32> = b[0].iter().map(|&x| cleaned(x)).collect();
        run(p.as_mut(), &mut b, &params());
        assert_eq!(bits(&b[..1]), bits(&[want]), "{name}");
    }
}

#[test]
fn arq_123_channels_of_different_lengths_are_cleaned_by_their_own_length() {
    for (name, make) in makers() {
        let mut p = make();
        p.prepare(48_000, 64, 2);
        let mut b = vec![mixed(1, 4).remove(0), mixed(1, 3).remove(0)];
        let want: Vec<Vec<f32>> = b
            .iter()
            .map(|c| c.iter().map(|&x| cleaned(x)).collect())
            .collect();
        run(p.as_mut(), &mut b, &params());
        assert_eq!(bits(&b), bits(&want), "{name}");
    }
}

#[test]
fn arq_123_an_unprocessed_block_leaves_no_trace_in_the_state() {
    // The refused block carries parameters; the valid one carries none. A
    // parameter read from the refused block, or an advanced clock, would show
    // in the valid block. A consumed `Gain` first block would not: the valid
    // block has no value to ramp to (`refused.rs` pins that, and the rows
    // skipped here, whose `prepare` has a 0, and a refused block mid-ramp).
    let mut odd = ParamSnapshot::new();
    odd.set(PARAM_VOLUME, 20.0);
    odd.set(PARAM_DURATION, 0.0005);
    odd.set(PARAM_PAN, -1.0);
    odd.set(PARAM_LOW, 15.0);
    odd.set(PARAM_MID, -15.0);
    odd.set(PARAM_HIGH, 15.0);
    for (name, make) in makers() {
        for (rate, max, ch, block_channels, frames) in REFUSED.iter().copied().skip(3) {
            let mut with_refused = make();
            let mut without = make();
            with_refused.prepare(rate, max, ch);
            without.prepare(rate, max, ch);
            let mut refused = mixed(block_channels, frames);
            run(with_refused.as_mut(), &mut refused, &odd);
            let (mut x, mut y) = (mixed(2, 8), mixed(2, 8));
            run(with_refused.as_mut(), &mut x, &ParamSnapshot::new());
            run(without.as_mut(), &mut y, &ParamSnapshot::new());
            assert_eq!(bits(&x), bits(&y), "{name} {block_channels}");
        }
    }
}

#[test]
fn arq_123_exactly_max_block_is_processed() {
    for (name, make) in makers() {
        let mut p = make();
        p.prepare(48_000, 8, 2);
        let mut b = vec![vec![f32::NAN; 8]; 2];
        run(p.as_mut(), &mut b, &params());
        assert!(b.iter().flatten().all(|&x| x == 0.0), "{name}");
    }
}

#[test]
fn arq_123_an_empty_block_and_a_block_with_no_channel_do_not_panic() {
    for (_, make) in makers() {
        let mut p = make();
        p.prepare(48_000, 64, 2);
        run(p.as_mut(), &mut [], &params());
        run(p.as_mut(), &mut [vec![], vec![]], &params());
    }
}

#[test]
fn arq_123_unknown_ids_with_non_finite_values_change_nothing() {
    for (name, make) in makers() {
        let mut noisy = params();
        noisy.set("not_a_param", f64::NAN);
        noisy.set("also_not", f64::NEG_INFINITY);
        let (mut a, mut b) = (make(), make());
        a.prepare(48_000, 64, 2);
        b.prepare(48_000, 64, 2);
        let (mut x, mut y) = (hostile(), hostile());
        run(a.as_mut(), &mut x, &noisy);
        run(b.as_mut(), &mut y, &params());
        assert_eq!(bits(&x), bits(&y), "{name}");
    }
}

#[test]
fn arq_111_two_runs_give_the_same_bytes_with_and_without_a_reset() {
    for (name, make) in makers() {
        let go = |reset_between: bool| {
            let mut p = make();
            p.prepare(48_000, 64, 2);
            let mut first = hostile();
            run(p.as_mut(), &mut first, &params());
            if reset_between {
                p.reset();
            }
            let mut second = hostile();
            run(p.as_mut(), &mut second, &params());
            (bits(&first), bits(&second))
        };
        assert_eq!(go(false), go(false), "{name}");
        assert_eq!(go(true), go(true), "{name}");
    }
}
