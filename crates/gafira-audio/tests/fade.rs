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
        f.reset();
        // The first block after the reset has no valid value: 0.02 s rules.
        let out = ones(&mut f, 3, &snap(bad));
        assert!((out[1] - 1.0 / 960.0).abs() < 1e-7, "{bad}");
    }
}

#[test]
fn arq_124_the_duration_is_read_once_per_ramp_and_a_change_waits_for_reset() {
    let reference = {
        let mut f = prepared(FadeDirection::In);
        ones(&mut f, 2000, &snap(0.02))
    };
    let mut f = prepared(FadeDirection::In);
    let mut got = ones(&mut f, 300, &snap(0.02));
    got.extend(ones(&mut f, 1700, &snap(0.005))); // shorter, mid-ramp
    assert_eq!(got, reference, "no sample changes");
    f.reset();
    let after = ones(&mut f, 400, &snap(0.005)); // 240 samples
    assert_eq!(after[240], 1.0);
    assert!((after[120] - 0.5).abs() < 1e-6);
}

#[test]
fn arq_124_going_from_duration_zero_to_two_seconds_waits_for_reset_too() {
    for dir in [FadeDirection::In, FadeDirection::Out] {
        let mut f = prepared(dir);
        let mut first = ones(&mut f, 100, &snap(0.0));
        first.extend(ones(&mut f, 1000, &snap(2.0)));
        assert!(first.iter().all(|&x| x == 1.0), "{dir:?}");
        f.reset();
        let again = ones(&mut f, 1000, &snap(2.0));
        assert!(
            again.iter().any(|&x| x != 1.0),
            "{dir:?}: the ramp runs now"
        );
    }
}

#[test]
fn fv_10_the_fade_in_and_the_fade_out_have_independent_durations() {
    let mut fade_in = prepared(FadeDirection::In);
    let mut fade_out = prepared(FadeDirection::Out);
    let a = ones(&mut fade_in, 1000, &snap(0.01)); // 480 samples
    let b = ones(&mut fade_out, 1000, &snap(0.02)); // 960 samples
    assert_eq!((a[480], b[480]), (1.0, 0.5));
    assert_eq!(b[960], 0.0);
    assert!(a[479] < 1.0 && b[959] > 0.0);
}

#[test]
fn fv_10_a_stereo_fade_runs_at_the_same_speed_on_both_channels() {
    for dir in [FadeDirection::In, FadeDirection::Out] {
        let mut f = Fade::new(dir);
        f.prepare(48_000, 512, 2);
        let mut left = Vec::new();
        let mut right = Vec::new();
        let params = snap(2.0);
        while left.len() < 96_000 + 512 {
            let mut chans = [vec![1.0_f32; 512], vec![1.0_f32; 512]];
            let mut slices: Vec<&mut [f32]> = chans.iter_mut().map(Vec::as_mut_slice).collect();
            f.process(&mut AudioBlock::new(&mut slices), &params);
            left.extend_from_slice(&chans[0]);
            right.extend_from_slice(&chans[1]);
        }
        assert_eq!(left, right, "{dir:?}");
        let (start, end) = if dir == FadeDirection::In {
            (0.0, 1.0)
        } else {
            (1.0, 0.0)
        };
        assert_eq!((left[0], left[96_000]), (start, end), "{dir:?}");
        let step = 1.0 / 96_000.0 + 1e-6;
        for w in left.windows(2) {
            assert!(f64::from((w[1] - w[0]).abs()) <= step, "{dir:?}");
        }
        assert!(
            left[95_999] != end,
            "{dir:?}: not finished one sample early"
        );
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
