//! ARQ-124 (pan, equal power), ARQ-110 and ARQ-123 for `Pan`; D-22.

use std::f64::consts::PI;

use gafira_audio::{
    AudioBlock, AudioError, AudioProcessor, PARAM_PAN, Pan, ParamSnapshot, crossfade_gains,
};
use gafira_doc::contract::AudioCrossfade;

fn snap(p: f64) -> ParamSnapshot {
    let mut s = ParamSnapshot::new();
    s.set(PARAM_PAN, p);
    s
}

fn run(p: &mut Pan, chans: &mut [Vec<f32>], params: &ParamSnapshot) {
    let mut slices: Vec<&mut [f32]> = chans.iter_mut().map(Vec::as_mut_slice).collect();
    p.process(&mut AudioBlock::new(&mut slices), params);
}

fn pan_of(position: f64) -> (f32, f32) {
    let mut p = Pan::new();
    p.prepare(48_000, 64, 2);
    let mut b = vec![vec![1.0_f32; 4], vec![1.0_f32; 4]];
    run(&mut p, &mut b, &snap(position));
    assert!(b[0].iter().all(|&x| x == b[0][0]));
    (b[0][0], b[1][0])
}

#[test]
fn arq_124_pan_centre_leaves_the_block_equal() {
    let (l, r) = pan_of(0.0);
    assert!(
        (l - 1.0).abs() <= 1e-6 && (r - 1.0).abs() <= 1e-6,
        "{l} {r}"
    );
}

#[test]
fn arq_124_pan_hard_left_silences_the_right_channel() {
    let (l, r) = pan_of(-1.0);
    assert!((l - 1.0).abs() <= 1e-6 && r.abs() <= 1e-6, "{l} {r}");
}

#[test]
fn arq_124_pan_hard_right_silences_the_left_channel() {
    let (l, r) = pan_of(1.0);
    assert!(l.abs() <= 1e-6 && (r - 1.0).abs() <= 1e-6, "{l} {r}");
}

#[test]
fn arq_124_pan_half_right_gives_0_5412_on_the_left_and_the_right_equal() {
    let (l, r) = pan_of(0.5);
    assert!((l - 0.5412).abs() <= 1e-4, "{l}");
    assert!((r - 1.0).abs() <= 1e-6, "{r}");
    let (l, r) = pan_of(-0.5);
    assert!(
        (r - 0.5412).abs() <= 1e-4 && (l - 1.0).abs() <= 1e-6,
        "{l} {r}"
    );
}

#[test]
fn arq_124_pan_follows_the_minus_three_db_law_over_the_whole_range() {
    for i in 0..=40 {
        let p = -1.0 + f64::from(i) * 0.05;
        let theta = (p + 1.0) * PI / 4.0;
        let (l, r) = pan_of(p);
        let wl = (2_f64.sqrt() * theta.cos()).min(1.0);
        let wr = (2_f64.sqrt() * theta.sin()).min(1.0);
        assert!(
            (f64::from(l) - wl).abs() < 1e-6 && (f64::from(r) - wr).abs() < 1e-6,
            "{p}"
        );
    }
}

#[test]
fn arq_124_pan_changes_nothing_on_a_block_that_is_not_stereo() {
    for channels in [1_usize, 3] {
        let mut p = Pan::new();
        p.prepare(48_000, 64, channels as u16);
        let mut b: Vec<Vec<f32>> = (0..channels).map(|c| vec![0.25 + c as f32; 4]).collect();
        let before = b.clone();
        run(&mut p, &mut b, &snap(-1.0));
        assert_eq!(b, before, "{channels} channels");
    }
}

#[test]
fn arq_123_pan_out_of_range_is_clipped_and_non_finite_is_ignored() {
    assert_eq!(pan_of(7.0), pan_of(1.0));
    assert_eq!(pan_of(-1e300), pan_of(-1.0));
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut p = Pan::new();
        p.prepare(48_000, 64, 2);
        let mut first = vec![vec![1.0_f32; 2], vec![1.0_f32; 2]];
        run(&mut p, &mut first, &snap(-1.0));
        let mut next = vec![vec![1.0_f32; 2], vec![1.0_f32; 2]];
        run(&mut p, &mut next, &snap(bad));
        assert_eq!(next, first, "{bad}: the last valid pan stays");
    }
}

#[test]
fn arq_123_pan_cleans_non_finite_samples_on_any_block() {
    let mut p = Pan::new();
    p.prepare(48_000, 64, 1);
    let mut mono = vec![vec![f32::NAN, f32::INFINITY, 0.5]];
    run(&mut p, &mut mono, &snap(0.3));
    assert_eq!(mono[0], vec![0.0, 0.0, 0.5]);
}

#[test]
fn arq_124_equal_power_gains_square_to_one_everywhere() {
    for i in -20..=120 {
        let p = f64::from(i) / 100.0;
        let (a, b) = crossfade_gains(AudioCrossfade::EqualPower, p).unwrap();
        assert!((a * a + b * b - 1.0).abs() <= 1e-6, "{p}");
    }
}

#[test]
fn arq_124_equal_power_is_cos_for_the_outgoing_side_and_sin_for_the_incoming() {
    let table = [
        (0.0, 1.0, 0.0),
        (1.0, 0.0, 1.0),
        (0.5, 0.5_f64.sqrt(), 0.5_f64.sqrt()),
    ];
    for (p, a, b) in table {
        let (ga, gb) = crossfade_gains(AudioCrossfade::EqualPower, p).unwrap();
        assert!((ga - a).abs() < 1e-12 && (gb - b).abs() < 1e-12, "{p}");
    }
    let (ga, gb) = crossfade_gains(AudioCrossfade::EqualPower, 0.25).unwrap();
    assert!((ga - (PI / 8.0).cos()).abs() < 1e-12);
    assert!((gb - (PI / 8.0).sin()).abs() < 1e-12);
}

#[test]
fn arq_124_equal_power_clips_p_to_zero_and_one() {
    for (p, want) in [
        (-0.5, (1.0, 0.0)),
        (-1e300, (1.0, 0.0)),
        (2.0, (0.0, 1.0)),
        (1e300, (0.0, 1.0)),
    ] {
        let (a, b) = crossfade_gains(AudioCrossfade::EqualPower, p).unwrap();
        assert!(
            (a - want.0).abs() < 1e-12 && (b - want.1).abs() < 1e-12,
            "{p}"
        );
    }
}

#[test]
fn arq_110_equal_power_rejects_a_non_finite_p() {
    for p in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            crossfade_gains(AudioCrossfade::EqualPower, p),
            Err(AudioError::NonFinite { field: "p" })
        );
    }
}
