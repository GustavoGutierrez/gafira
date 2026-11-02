//! ARQ-123 and ARQ-124: a block with no frames is never processed. It reads no
//! parameter, changes no state, latches nothing and is not the first block.

use gafira_audio::{
    AudioBlock, AudioProcessor, Eq3Band, Fade, FadeDirection, Gain, PARAM_DURATION, PARAM_HIGH,
    PARAM_LOW, PARAM_MID, PARAM_PAN, PARAM_VOLUME, Pan, ParamSnapshot,
};

type Maker = fn() -> Box<dyn AudioProcessor>;

fn run(p: &mut dyn AudioProcessor, chans: &mut [Vec<f32>], params: &ParamSnapshot) {
    let mut slices: Vec<&mut [f32]> = chans.iter_mut().map(Vec::as_mut_slice).collect();
    p.process(&mut AudioBlock::new(&mut slices), params);
}

fn snap(id: &'static str, value: f64) -> ParamSnapshot {
    let mut s = ParamSnapshot::new();
    s.set(id, value);
    s
}

fn loud() -> ParamSnapshot {
    let mut s = ParamSnapshot::new();
    s.set(PARAM_VOLUME, 50.0);
    s.set(PARAM_DURATION, 0.001);
    s.set(PARAM_PAN, -1.0);
    s.set(PARAM_LOW, 12.0);
    s.set(PARAM_MID, -12.0);
    s.set(PARAM_HIGH, 12.0);
    s
}

/// The two shapes of an empty block for `channels` prepared channels: no
/// channel at all, and channels of 0 samples.
fn empties(channels: usize) -> Vec<Vec<Vec<f32>>> {
    vec![Vec::new(), vec![Vec::new(); channels]]
}

/// Each processor with the sample its next block must keep untouched.
fn makers() -> Vec<(&'static str, Maker)> {
    vec![
        ("gain", || Box::new(Gain::new())),
        ("fade_in", || Box::new(Fade::new(FadeDirection::In))),
        ("fade_out", || Box::new(Fade::new(FadeDirection::Out))),
        ("pan", || Box::new(Pan::new())),
        ("eq3band", || Box::new(Eq3Band::new())),
    ]
}

/// A block after the empty one, with no parameters: it must come out as if
/// the empty one had never arrived, that is, x 1.0.
fn assert_unity(name: &str, p: &mut dyn AudioProcessor, context: &str) {
    let mut b = vec![vec![1.0_f32; 8], vec![1.0_f32; 8]];
    run(p, &mut b, &ParamSnapshot::new());
    assert!(
        b.iter().flatten().all(|&x| x == 1.0),
        "{name} {context}: {b:?}"
    );
}

#[test]
fn arq_123_an_empty_block_after_a_valid_prepare_leaves_the_next_block_untouched() {
    for (name, make) in makers() {
        for (n, mut empty) in empties(2).into_iter().enumerate() {
            let mut p = make();
            p.prepare(48_000, 64, 2);
            run(p.as_mut(), &mut empty, &loud());
            assert_unity(name, p.as_mut(), &format!("shape {n}"));
        }
    }
}

#[test]
fn arq_123_an_empty_block_after_a_prepare_with_a_zero_leaves_a_later_prepare_untouched() {
    // (max_block, channels): each zero alone and both.
    for (max_block, channels) in [(0_usize, 2_u16), (64, 0), (0, 0)] {
        for (name, make) in makers() {
            for (n, mut empty) in empties(usize::from(channels)).into_iter().enumerate() {
                let mut p = make();
                p.prepare(48_000, max_block, channels);
                run(p.as_mut(), &mut empty, &loud());
                p.prepare(48_000, 64, 2);
                assert_unity(
                    name,
                    p.as_mut(),
                    &format!("{max_block}/{channels} shape {n}"),
                );
            }
        }
    }
}

#[test]
fn arq_124_an_empty_block_is_not_the_first_block_of_a_gain() {
    let mut empty = vec![Vec::new()];
    let fresh = || {
        let mut g = Gain::new();
        g.prepare(48_000, 64, 1);
        g
    };

    // Empty without volume, then 0: 0 from sample 0, no ramp.
    let mut g = fresh();
    run(&mut g, &mut empty, &ParamSnapshot::new());
    let mut b = vec![vec![1.0_f32; 8]];
    run(&mut g, &mut b, &snap(PARAM_VOLUME, 0.0));
    assert_eq!(b[0], vec![0.0; 8], "empty then volume 0");

    // Empty with 0, then no volume: x 1.0.
    let mut g = fresh();
    run(&mut g, &mut empty, &snap(PARAM_VOLUME, 0.0));
    let mut b = vec![vec![1.0_f32; 8]];
    run(&mut g, &mut b, &ParamSnapshot::new());
    assert_eq!(b[0], vec![1.0; 8], "empty with 0 then no volume");

    // Volume 100, empty with 0, no volume: x 1.0, no ramp.
    let mut g = fresh();
    let mut b = vec![vec![1.0_f32; 8]];
    run(&mut g, &mut b, &snap(PARAM_VOLUME, 100.0));
    run(&mut g, &mut empty, &snap(PARAM_VOLUME, 0.0));
    let mut b = vec![vec![1.0_f32; 8]];
    run(&mut g, &mut b, &ParamSnapshot::new());
    assert_eq!(b[0], vec![1.0; 8], "volume 100, empty with 0, no volume");
}

fn ones(p: &mut dyn AudioProcessor, frames: usize, params: &ParamSnapshot) -> Vec<f32> {
    let mut out = Vec::with_capacity(frames);
    while out.len() < frames {
        let mut b = vec![vec![1.0_f32; 500.min(frames - out.len())]];
        run(p, &mut b, params);
        out.extend(&b[0]);
    }
    out
}

#[test]
fn arq_124_an_empty_block_does_not_latch_the_fade_length() {
    let mut f = Fade::new(FadeDirection::In);
    f.prepare(48_000, 512, 1);
    run(&mut f, &mut [Vec::new()], &ParamSnapshot::new());
    let out = ones(&mut f, 96_001, &snap(PARAM_DURATION, 2.0));
    assert_eq!(out[0], 0.0);
    assert!((out[48_000] - 0.5).abs() < 1e-6, "{}", out[48_000]);
    assert_eq!(out[96_000], 1.0);
}

/// Largest distance between a per-sample rise and `want`, over the ramp.
fn max_step_error(out: &[f32], ramp: usize, want: f64) -> f64 {
    out[..=ramp]
        .windows(2)
        .map(|w| (f64::from(w[1]) - f64::from(w[0]) - want).abs())
        .fold(0.0, f64::max)
}

/// Duration 0.02 in the first block, 0.005 in a block mid-ramp (`mid`), then
/// `reset` and a block carrying `after`.
fn rise_after_reset(mid: &mut dyn FnMut(&mut Fade), after: &ParamSnapshot) -> Vec<f32> {
    let mut f = Fade::new(FadeDirection::In);
    f.prepare(48_000, 512, 1);
    let _ = ones(&mut f, 100, &snap(PARAM_DURATION, 0.02));
    mid(&mut f);
    f.reset();
    ones(&mut f, 2_000, after)
}

#[test]
fn arq_124_a_duration_in_a_processed_block_mid_ramp_becomes_the_last_valid_one() {
    let mid = &mut |f: &mut Fade| {
        let _ = ones(f, 10, &snap(PARAM_DURATION, 0.005));
    };
    // 1/240 whether the block after reset has no duration or a NaN one.
    for after in [ParamSnapshot::new(), snap(PARAM_DURATION, f64::NAN)] {
        let out = rise_after_reset(mid, &after);
        assert!(max_step_error(&out, 240, 1.0 / 240.0) <= 1e-6);
        assert_eq!(out[240], 1.0);
    }
}

#[test]
fn arq_124_a_duration_in_an_empty_block_mid_ramp_is_not_read() {
    let mid = &mut |f: &mut Fade| run(f, &mut [Vec::new()], &snap(PARAM_DURATION, 0.005));
    let out = rise_after_reset(mid, &ParamSnapshot::new());
    assert!(max_step_error(&out, 960, 1.0 / 960.0) <= 1e-6);
    assert_eq!(out[960], 1.0);
}

#[test]
fn arq_124_the_block_after_reset_sets_the_length_with_its_own_duration() {
    let mid = &mut |f: &mut Fade| {
        let _ = ones(f, 10, &snap(PARAM_DURATION, 0.005));
    };
    let out = rise_after_reset(mid, &snap(PARAM_DURATION, 0.04));
    assert!(max_step_error(&out, 1_920, 1.0 / 1_920.0) <= 1e-6);
    assert_eq!(out[1_920], 1.0);
}
