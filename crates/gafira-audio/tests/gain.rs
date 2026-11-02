//! ARQ-124 (gain), ARQ-110 and ARQ-123 for `Gain`; FV-10 (a).

use gafira_audio::{
    AudioBlock, AudioError, AudioProcessor, Gain, PARAM_VOLUME, ParamSnapshot, db_to_gain,
    percent_to_gain,
};

fn snap(volume: f64) -> ParamSnapshot {
    let mut s = ParamSnapshot::new();
    s.set(PARAM_VOLUME, volume);
    s
}

fn run(p: &mut Gain, chans: &mut [Vec<f32>], params: &ParamSnapshot) {
    let mut slices: Vec<&mut [f32]> = chans.iter_mut().map(Vec::as_mut_slice).collect();
    p.process(&mut AudioBlock::new(&mut slices), params);
}

fn mono(frames: usize, value: f32) -> Vec<Vec<f32>> {
    vec![vec![value; frames]]
}

fn prepared(rate: u32) -> Gain {
    let mut g = Gain::new();
    g.prepare(rate, 4096, 1);
    g
}

#[test]
fn arq_124_db_to_gain_follows_ten_to_the_db_over_twenty() {
    let table = [
        (6.0, 1.9953, 1e-4),
        (0.0, 1.0, 1e-12),
        (-20.0, 0.1, 1e-12),
        (20.0, 10.0, 1e-12),
        (-6.0, 0.5012, 1e-4),
    ];
    for (db, want, tol) in table {
        let got = db_to_gain(db).unwrap();
        assert!((got - want).abs() <= tol, "{db} dB gave {got}");
    }
}

#[test]
fn arq_124_minus_infinity_db_is_exactly_zero() {
    assert_eq!(db_to_gain(f64::NEG_INFINITY), Ok(0.0));
}

#[test]
fn arq_110_db_to_gain_rejects_nan_and_plus_infinity_and_overflow() {
    for db in [f64::NAN, f64::INFINITY, 1e6] {
        assert_eq!(
            db_to_gain(db),
            Err(AudioError::NonFinite { field: "db" }),
            "{db}"
        );
    }
}

#[test]
fn arq_124_two_hundred_percent_is_exactly_two() {
    let table = [(200.0, 2.0), (100.0, 1.0), (0.0, 0.0), (50.0, 0.5)];
    for (pct, want) in table {
        assert_eq!(percent_to_gain(pct), Ok(want), "{pct}");
    }
}

#[test]
fn arq_110_percent_to_gain_names_percent_for_non_finite() {
    for pct in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            percent_to_gain(pct),
            Err(AudioError::NonFinite { field: "percent" })
        );
    }
}

#[test]
fn arq_124_ramp_lasts_ten_milliseconds_at_every_rate() {
    for (rate, ramp) in [(44_100_u32, 441_usize), (48_000, 480), (96_000, 960)] {
        let mut g = prepared(rate);
        let mut warm = mono(2048, 1.0);
        run(&mut g, &mut warm, &snap(0.0));
        let mut up = mono(2048, 1.0);
        run(&mut g, &mut up, &snap(100.0));
        let out = &up[0];
        assert!(out[ramp - 2] < 1.0, "{rate}: still ramping");
        assert_eq!(out[ramp - 1], 1.0, "{rate}: reached at sample {ramp}");
        assert!(out[0] > 0.0 && out[0] <= 1.0 / ramp as f32 + 1e-6);
    }
}

#[test]
fn arq_124_a_jump_from_zero_to_one_changes_at_most_one_480th_per_sample() {
    let mut g = prepared(48_000);
    let mut warm = mono(2048, 1.0);
    run(&mut g, &mut warm, &snap(0.0));
    assert!(warm[0][2047] == 0.0);
    // 100 sample blocks: the ramp must continue across block edges.
    let mut previous = 0.0_f32;
    let mut worst = 0.0_f32;
    for _ in 0..8 {
        let mut b = mono(100, 1.0);
        run(&mut g, &mut b, &snap(100.0));
        for &x in &b[0] {
            worst = worst.max((x - previous).abs());
            previous = x;
        }
    }
    assert!(worst > 0.0 && worst <= 1.0 / 480.0 + 1e-6, "{worst}");
    assert_eq!(previous, 1.0);
}

#[test]
fn arq_124_a_downward_change_ramps_too() {
    let mut g = prepared(48_000);
    let mut first = mono(8, 1.0);
    run(&mut g, &mut first, &ParamSnapshot::new());
    assert_eq!(first[0], vec![1.0; 8], "no value: the default applies");
    let mut b = mono(1024, 1.0);
    run(&mut g, &mut b, &snap(0.0));
    let out = &b[0];
    assert!(out[0] < 1.0 && out[0] >= 1.0 - 1.0 / 480.0 - 1e-6);
    assert_eq!(out[479], 0.0);
    assert!(out[1023] == 0.0);
}

#[test]
fn arq_124_the_first_value_applies_from_sample_zero_without_a_ramp() {
    for (volume, want) in [(0.0, 0.0), (50.0, 0.5), (200.0, 2.0), (500.0, 2.0)] {
        let mut g = prepared(48_000);
        let mut b = mono(64, 1.0);
        run(&mut g, &mut b, &snap(volume));
        assert_eq!(b[0], vec![want; 64], "{volume}");
    }
}

#[test]
fn arq_124_after_reset_the_next_value_applies_at_once_too() {
    let mut g = prepared(48_000);
    let mut b = mono(2048, 1.0);
    run(&mut g, &mut b, &snap(200.0));
    g.reset();
    let mut c = mono(64, 1.0);
    run(&mut g, &mut c, &snap(50.0));
    assert_eq!(c[0], vec![0.5; 64]);
}

#[test]
fn arq_124_a_first_block_without_a_value_uses_the_last_valid_one() {
    let mut g = prepared(48_000);
    let mut b = mono(8, 1.0);
    run(&mut g, &mut b, &snap(50.0));
    g.reset();
    let mut c = mono(8, 1.0);
    run(&mut g, &mut c, &ParamSnapshot::new());
    assert_eq!(c[0], vec![0.5; 8], "the last valid value, with no ramp");
}

#[test]
fn arq_124_a_first_block_without_a_value_then_a_zero_ramps() {
    let mut g = prepared(48_000);
    let mut first = mono(8, 1.0);
    run(&mut g, &mut first, &ParamSnapshot::new());
    let mut second = mono(1024, 1.0);
    run(&mut g, &mut second, &snap(0.0));
    assert!(second[0][0] > 0.9, "a ramp, not a step");
    assert_eq!(second[0][479], 0.0);
    assert!(second[0][478] > 0.0);
}

#[test]
fn arq_124_a_change_during_a_ramp_continues_from_the_current_gain() {
    let mut g = prepared(48_000);
    let mut settle = mono(8, 1.0);
    run(&mut g, &mut settle, &snap(0.0));
    // Up towards 1.0, interrupted after 100 samples by a target of 0.5.
    let mut up = mono(100, 1.0);
    run(&mut g, &mut up, &snap(100.0));
    let reached = up[0][99];
    assert!(reached > 0.15 && reached < 0.25, "{reached}");
    let mut down = mono(1000, 1.0);
    run(&mut g, &mut down, &snap(50.0));
    let all: Vec<f32> = up[0].iter().chain(down[0].iter()).copied().collect();
    let worst = all
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .fold(0.0_f32, f32::max);
    assert!(
        worst <= 1.0 / 480.0 + 1e-6,
        "no jump at the change: {worst}"
    );
    assert!(down[0][0] > reached, "still rising on the first sample");
    assert_eq!(
        down[0][479], 0.5,
        "the new ramp lasts 10 ms from the change"
    );
    assert!(down[0][478] != 0.5);
    assert!(down[0][480..].iter().all(|&x| x == 0.5));
}

#[test]
fn arq_124_a_change_back_to_the_same_value_does_not_restart_the_ramp() {
    let mut g = prepared(48_000);
    let mut first = mono(8, 1.0);
    run(&mut g, &mut first, &snap(0.0));
    let mut a = mono(100, 1.0);
    run(&mut g, &mut a, &snap(100.0));
    let mut b = mono(380, 1.0);
    run(&mut g, &mut b, &snap(100.0));
    assert_eq!(b[0][379], 1.0, "480 samples after the first change in all");
}

#[test]
fn fv_10_volume_scales_every_channel_alike() {
    let mut g = Gain::new();
    g.prepare(48_000, 4096, 2);
    let mut b = vec![vec![0.5; 1024], vec![-0.25; 1024]];
    run(&mut g, &mut b, &snap(50.0));
    assert_eq!(b[0][1023], 0.25);
    assert_eq!(b[1][1023], -0.125);
}

#[test]
fn arq_123_non_finite_parameter_keeps_the_last_valid_one() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut g = prepared(48_000);
        let mut b = mono(1024, 1.0);
        run(&mut g, &mut b, &snap(50.0));
        let mut c = mono(1024, 1.0);
        run(&mut g, &mut c, &snap(bad));
        assert!(c[0].iter().all(|&x| x == 0.5), "{bad}");
    }
}

#[test]
fn arq_123_parameter_out_of_range_is_clipped() {
    let table = [(500.0, 2.0), (200.0, 2.0), (-10.0, 0.0), (1e300, 2.0)];
    for (value, want) in table {
        let mut g = prepared(48_000);
        let mut b = mono(1024, 1.0);
        run(&mut g, &mut b, &snap(value));
        assert_eq!(b[0][1023], want, "{value}");
    }
}

#[test]
fn arq_123_an_unknown_id_is_never_read() {
    let mut with = snap(50.0);
    with.set("nope", f64::NAN);
    with.set("nope2", f64::INFINITY);
    let (mut a, mut b) = (prepared(48_000), prepared(48_000));
    let (mut x, mut y) = (mono(1024, 0.7), mono(1024, 0.7));
    run(&mut a, &mut x, &with);
    run(&mut b, &mut y, &snap(50.0));
    let bits = |v: &Vec<Vec<f32>>| v[0].iter().map(|f| f.to_bits()).collect::<Vec<_>>();
    assert_eq!(bits(&x), bits(&y));
}

#[test]
fn arq_123_there_is_no_clipping_at_zero_dbfs() {
    let mut g = prepared(48_000);
    let mut b = mono(1024, 0.9);
    run(&mut g, &mut b, &snap(200.0));
    assert!((b[0][1023] - 1.8).abs() < 1e-6);
    let mut big = mono(1024, f32::MAX);
    run(&mut g, &mut big, &snap(200.0));
    assert!(
        big[0].iter().all(|x| x.is_finite()),
        "overflow stays finite"
    );
    assert_eq!(big[0][1023], f32::MAX);
    let mut neg = mono(1024, -f32::MAX);
    run(&mut g, &mut neg, &snap(200.0));
    assert_eq!(neg[0][1023], -f32::MAX);
}

#[test]
fn arq_111_the_same_input_gives_the_same_bytes() {
    let go = || {
        let mut g = prepared(48_000);
        let mut b: Vec<Vec<f32>> = vec![(0..1024).map(|i| (i as f32 * 0.37).sin()).collect()];
        run(&mut g, &mut b, &snap(0.0));
        run(&mut g, &mut b, &snap(173.0));
        b[0].iter().map(|f| f.to_bits()).collect::<Vec<_>>()
    };
    assert_eq!(go(), go());
}

#[test]
fn arq_111_reset_ends_a_running_ramp() {
    let mut g = prepared(48_000);
    let mut first = mono(8, 1.0);
    run(&mut g, &mut first, &ParamSnapshot::new());
    let mut b = mono(100, 1.0);
    run(&mut g, &mut b, &snap(0.0));
    assert!(b[0][99] > 0.0, "the ramp is still running");
    g.reset();
    let mut c = mono(8, 1.0);
    run(&mut g, &mut c, &ParamSnapshot::new());
    assert_eq!(c[0], vec![0.0; 8], "the last valid value, at once");
}

#[test]
fn arq_124_prepare_again_makes_the_next_value_a_first_one() {
    let mut g = prepared(48_000);
    let mut b = mono(2048, 1.0);
    run(&mut g, &mut b, &snap(200.0));
    g.prepare(44_100, 4096, 1);
    let mut c = mono(64, 1.0);
    run(&mut g, &mut c, &snap(50.0));
    assert_eq!(c[0], vec![0.5; 64]);
}
