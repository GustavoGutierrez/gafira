//! ARQ-123, ARQ-124: a block `process` does not process leaves the state as it
//! was, and the last valid parameter value survives `prepare`.
//!
//! These tests pin behaviour that already holds; each one is proved by a
//! scratch mutant (see `apply-progress.md`).

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
            chain.push(Box::new(Fade::new(FadeDirection::In)));
            chain.push(Box::new(Eq3Band::new()));
            chain.push(Box::new(Pan::new()));
            Box::new(chain)
        }),
    ]
}

fn snapshot(volume: f64, duration: f64, pan: f64) -> ParamSnapshot {
    let mut s = ParamSnapshot::new();
    s.set(PARAM_VOLUME, volume);
    s.set(PARAM_DURATION, duration);
    s.set(PARAM_PAN, pan);
    // The equaliser bands follow the same three numbers, so every snapshot
    // moves them: `pan` -1..1 -> low -12..12, `volume` -> mid, `duration` -> high.
    s.set(PARAM_LOW, pan * 12.0);
    s.set(PARAM_MID, volume / 10.0 - 3.0);
    s.set(PARAM_HIGH, -duration * 1000.0);
    s
}

fn run(p: &mut dyn AudioProcessor, chans: &mut [Vec<f32>], params: &ParamSnapshot) {
    let mut slices: Vec<&mut [f32]> = chans.iter_mut().map(Vec::as_mut_slice).collect();
    p.process(&mut AudioBlock::new(&mut slices), params);
}

fn ones(channels: usize, frames: usize) -> Vec<Vec<f32>> {
    vec![vec![1.0; frames]; channels]
}

fn bits(chans: &[Vec<f32>]) -> Vec<Vec<u32>> {
    chans
        .iter()
        .map(|c| c.iter().map(|f| f.to_bits()).collect())
        .collect()
}

/// Processes one block of ones on two channels and returns it.
fn step(p: &mut dyn AudioProcessor, frames: usize, params: &ParamSnapshot) -> Vec<Vec<f32>> {
    let mut b = ones(2, frames);
    run(p, &mut b, params);
    b
}

/// Prepared for 48 kHz, 64 frames, stereo.
const MAX: usize = 64;

/// Blocks a stereo processor prepared with `MAX` does not process.
fn refused_blocks() -> Vec<(&'static str, Vec<Vec<f32>>)> {
    vec![
        ("no channel", vec![]),
        ("empty", vec![vec![], vec![]]),
        ("above max_block", ones(2, MAX + 1)),
        ("one channel", ones(1, 8)),
        ("three channels", ones(3, 8)),
        ("lopsided", vec![vec![1.0; 8], vec![1.0; 7]]),
    ]
}

#[test]
fn arq_124_a_refused_block_does_not_consume_the_first_block_of_a_gain() {
    // Control: the first block after `prepare` applies its value at once. If a
    // refused non-empty block used that turn up, the next block would ramp.
    for (what, refused) in refused_blocks() {
        let mut gain = Gain::new();
        gain.prepare(48_000, MAX, 2);
        let mut b = refused;
        run(&mut gain, &mut b, &snapshot(200.0, 0.0, 0.0));
        let out = step(&mut gain, 8, &snapshot(0.0, 0.0, 0.0));
        assert_eq!(out, vec![vec![0.0; 8]; 2], "{what}");
    }
}

#[test]
fn arq_123_a_refused_block_during_a_running_state_leaves_the_next_samples_as_they_were() {
    // Block 1 is the first (Gain applies 100 %, Fade latches 10 ms). Block 2
    // starts the gain ramp to 0 and moves the fade on. Then the refused block,
    // which carries values of its own. Blocks 3 and 4 must equal a run that
    // never saw it. Block 2's duration differs from block 1's: it must not
    // become the fade length, so a refused block that clears the latch shows.
    let first = snapshot(100.0, 0.01, 0.3);
    let second = snapshot(0.0, 0.02, -0.5);
    let odd = snapshot(200.0, 0.0005, 1.0);
    for (name, make) in makers() {
        for (what, refused) in refused_blocks() {
            let mut reference = make();
            let mut p = make();
            reference.prepare(48_000, MAX, 2);
            p.prepare(48_000, MAX, 2);
            let mut want = Vec::new();
            for params in [&first, &second, &second, &second] {
                want.push(bits(&step(reference.as_mut(), MAX, params)));
            }
            let mut got = Vec::new();
            for (i, params) in [&first, &second, &second, &second].into_iter().enumerate() {
                if i == 2 {
                    let mut b = refused.clone();
                    run(p.as_mut(), &mut b, &odd);
                }
                got.push(bits(&step(p.as_mut(), MAX, params)));
            }
            assert_eq!(got, want, "{name} {what}");
        }
    }
}

#[test]
fn arq_123_a_refused_block_after_a_prepare_with_a_zero_reads_no_parameter() {
    // `prepare` with a 0 refuses every block. The refused one carries values;
    // after a valid `prepare` a block with none must behave as on a processor
    // that never saw them.
    let odd = snapshot(20.0, 0.0005, -1.0);
    for (name, make) in makers() {
        for (rate, max, channels) in [(0, MAX, 2), (48_000, 0, 2), (48_000, MAX, 0)] {
            let mut p = make();
            let mut fresh = make();
            p.prepare(rate, max, channels);
            let mut b = ones(2, 12);
            run(p.as_mut(), &mut b, &odd);
            p.prepare(48_000, MAX, 2);
            fresh.prepare(48_000, MAX, 2);
            let none = ParamSnapshot::new();
            assert_eq!(
                bits(&step(p.as_mut(), 8, &none)),
                bits(&step(fresh.as_mut(), 8, &none)),
                "{name} {rate} {max} {channels}"
            );
        }
    }
}

#[test]
fn arq_124_the_last_valid_value_survives_prepare() {
    // ARQ-124: the first block after `prepare` or `reset` uses the last valid
    // value when it carries none.
    let set = snapshot(50.0, 0.001, -1.0);
    for (name, make) in makers() {
        let mut after = make();
        after.prepare(48_000, MAX, 2);
        let _ = step(after.as_mut(), 8, &set);
        after.prepare(48_000, MAX, 2);
        let got = step(after.as_mut(), MAX, &ParamSnapshot::new());

        let mut with_values = make();
        with_values.prepare(48_000, MAX, 2);
        let want = step(with_values.as_mut(), MAX, &set);
        assert_eq!(bits(&got), bits(&want), "{name}");

        let mut defaults = make();
        defaults.prepare(48_000, MAX, 2);
        let plain = step(defaults.as_mut(), MAX, &ParamSnapshot::new());
        assert_ne!(bits(&got), bits(&plain), "{name}: the values must matter");
    }
}

/// Two channels of a signal that is not constant, so a filter or a ramp shows.
fn signal(frames: usize) -> Vec<Vec<f32>> {
    (0..2)
        .map(|c| {
            (0..frames)
                .map(|i| (((i * 37 + c * 11) % 101) as f32 - 50.0) / 60.0)
                .collect()
        })
        .collect()
}

fn play(p: &mut dyn AudioProcessor, params: &ParamSnapshot) -> Vec<Vec<f32>> {
    let mut b = signal(MAX);
    run(p, &mut b, params);
    b
}

#[test]
fn arq_124_the_last_valid_value_survives_reset() {
    // ARQ-124: the first block after `reset` uses the last valid value when it
    // carries none. Twin of the `prepare` test above.
    let set = snapshot(50.0, 0.001, -1.0);
    for (name, make) in makers() {
        let mut after = make();
        after.prepare(48_000, MAX, 2);
        let _ = play(after.as_mut(), &set);
        after.reset();
        let got = play(after.as_mut(), &ParamSnapshot::new());

        let mut with_values = make();
        with_values.prepare(48_000, MAX, 2);
        let want = play(with_values.as_mut(), &set);
        assert_eq!(bits(&got), bits(&want), "{name}");

        let mut defaults = make();
        defaults.prepare(48_000, MAX, 2);
        let plain = play(defaults.as_mut(), &ParamSnapshot::new());
        assert_ne!(bits(&got), bits(&plain), "{name}: the values must matter");
    }
}

#[test]
fn arq_125_prepare_at_another_rate_designs_for_the_new_rate_with_the_last_valid_value() {
    // The stream shape of the new `prepare` rules, the value is the last valid
    // one: the result equals a processor prepared at the new rate that got the
    // value in its block.
    let set = snapshot(50.0, 0.001, -1.0);
    for (name, make) in makers() {
        let mut moved = make();
        moved.prepare(48_000, MAX, 2);
        let _ = play(moved.as_mut(), &set);
        moved.prepare(96_000, MAX, 2);
        let got = play(moved.as_mut(), &ParamSnapshot::new());

        let mut fresh = make();
        fresh.prepare(96_000, MAX, 2);
        let want = play(fresh.as_mut(), &set);
        assert_eq!(bits(&got), bits(&want), "{name}");
    }
}
