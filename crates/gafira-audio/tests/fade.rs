//! ARQ-124 (fade), ARQ-123 and ARQ-110 for `Fade`; FV-10 (a).

use gafira_audio::{
    AudioBlock, AudioProcessor, Fade, FadeDirection, PARAM_DURATION, ParamSnapshot,
};

fn snap(seconds: f64) -> ParamSnapshot {
    let mut s = ParamSnapshot::new();
    s.set(PARAM_DURATION, seconds);
    s
}

fn run(p: &mut Fade, input: &mut Vec<f32>, params: &ParamSnapshot) {
    let mut slices = [input.as_mut_slice()];
    p.process(&mut AudioBlock::new(&mut slices), params);
}

/// Runs `frames` samples of 1.0 through `fade` in blocks of 500.
fn ones(fade: &mut Fade, frames: usize, params: &ParamSnapshot) -> Vec<f32> {
    let mut out = Vec::with_capacity(frames);
    while out.len() < frames {
        let mut b = vec![1.0_f32; 500.min(frames - out.len())];
        run(fade, &mut b, params);
        out.extend(b);
    }
    out
}

fn prepared(direction: FadeDirection) -> Fade {
    let mut f = Fade::new(direction);
    f.prepare(48_000, 512, 1);
    f
}

#[test]
fn fv_10_a_two_second_fade_in_rises_from_zero_to_one_continuously() {
    let mut f = prepared(FadeDirection::In);
    let out = ones(&mut f, 96_000 + 1000, &snap(2.0));
    assert_eq!(out[0], 0.0);
    let step = 1.0 / 96_000.0 + 1e-6;
    for w in out.windows(2) {
        assert!(w[1] >= w[0], "monotonic");
        assert!(f64::from(w[1] - w[0]) <= step, "step {}", w[1] - w[0]);
    }
    assert!(out[95_999] < 1.0 && out[95_999] > 0.999);
    assert_eq!(out[96_000], 1.0);
    assert!(out[96_000..].iter().all(|&x| x == 1.0));
}

#[test]
fn arq_124_a_fade_out_falls_from_one_to_zero_and_holds() {
    let mut f = prepared(FadeDirection::Out);
    let out = ones(&mut f, 96_000 + 1000, &snap(2.0));
    assert_eq!(out[0], 1.0);
    let step = 1.0 / 96_000.0 + 1e-6;
    for w in out.windows(2) {
        assert!(w[1] <= w[0], "monotonic");
        assert!(f64::from(w[0] - w[1]) <= step);
    }
    assert!(out[95_999] > 0.0 && out[95_999] < 0.001);
    assert_eq!(out[96_000], 0.0);
    assert!(out[96_000..].iter().all(|&x| x == 0.0));
}

#[test]
fn arq_124_the_midpoint_of_a_linear_fade_is_one_half() {
    for (dir, want) in [(FadeDirection::In, 0.5), (FadeDirection::Out, 0.5)] {
        let mut f = prepared(dir);
        let out = ones(&mut f, 1000, &snap(0.02)); // 960 samples
        assert!((out[480] - want).abs() < 1e-6, "{dir:?}");
    }
}

#[test]
fn arq_124_duration_zero_changes_nothing() {
    // 0, a negative duration (clipped to 0), and one shorter than half a sample.
    for seconds in [0.0, -3.0, 1e-9] {
        for dir in [FadeDirection::In, FadeDirection::Out] {
            let mut f = prepared(dir);
            let out = ones(&mut f, 1000, &snap(seconds));
            assert!(out.iter().all(|&x| x == 1.0), "{dir:?} {seconds}");
        }
    }
    let mut f = prepared(FadeDirection::In);
    let out = ones(&mut f, 1000, &ParamSnapshot::new());
    assert!(out.iter().all(|&x| x == 1.0), "no duration in the snapshot");
}

#[test]
fn arq_123_a_non_finite_duration_keeps_the_last_valid_one() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut f = prepared(FadeDirection::In);
        let mut first = vec![1.0_f32; 1];
        run(&mut f, &mut first, &snap(0.02));
        let mut next = vec![1.0_f32; 1];
        run(&mut f, &mut next, &snap(bad));
        // Sample 1 of a 960-sample fade-in.
        assert!((next[0] - 1.0 / 960.0).abs() < 1e-7, "{bad}");
    }
}

#[test]
fn arq_123_a_huge_finite_duration_does_not_panic_and_stays_finite() {
    for dir in [FadeDirection::In, FadeDirection::Out] {
        let mut f = prepared(dir);
        let out = ones(&mut f, 1000, &snap(1e300));
        assert!(out.iter().all(|x| x.is_finite() && !x.is_subnormal()));
    }
}

#[test]
fn arq_111_reset_restarts_the_fade() {
    let mut f = prepared(FadeDirection::In);
    let first = ones(&mut f, 1000, &snap(0.02));
    f.reset();
    let again = ones(&mut f, 1000, &snap(0.02));
    let bits = |v: &[f32]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
    assert_eq!(bits(&first), bits(&again));
}

#[test]
fn arq_123_a_refused_block_does_not_advance_the_fade() {
    let mut f = Fade::new(FadeDirection::In);
    f.prepare(48_000, 4, 1);
    let mut big = vec![1.0_f32; 8];
    run(&mut f, &mut big, &snap(0.02));
    assert_eq!(big, vec![1.0; 8]);
    let mut ok = vec![1.0_f32; 4];
    run(&mut f, &mut ok, &snap(0.02));
    assert_eq!(ok[0], 0.0, "still at the start of the fade");
}
