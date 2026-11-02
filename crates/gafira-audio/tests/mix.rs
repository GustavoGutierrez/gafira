//! ARQ-129 (mix), ARQ-123 and ARQ-110 for `mix`; CAL-15 is AU2.

use gafira_audio::{AudioBlock, MixTrack, mix};

fn lcg(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
}

fn tracks_data(count: usize, frames: usize) -> Vec<Vec<Vec<f32>>> {
    let mut seed = 7;
    (0..count)
        .map(|_| {
            (0..2)
                .map(|_| (0..frames).map(|_| lcg(&mut seed)).collect())
                .collect()
        })
        .collect()
}

fn views(data: &[Vec<Vec<f32>>]) -> Vec<Vec<&[f32]>> {
    data.iter()
        .map(|t| t.iter().map(Vec::as_slice).collect())
        .collect()
}

fn run_mix(data: &[Vec<Vec<f32>>], muted: &[bool], out: &mut [Vec<f32>]) {
    let v = views(data);
    let tracks: Vec<MixTrack<'_>> = v
        .iter()
        .enumerate()
        .map(|(i, t)| MixTrack {
            channels: t.as_slice(),
            muted: muted.get(i).copied().unwrap_or(false),
        })
        .collect();
    let mut slices: Vec<&mut [f32]> = out.iter_mut().map(Vec::as_mut_slice).collect();
    mix(&tracks, &mut AudioBlock::new(&mut slices));
}

#[test]
fn arq_129_the_mix_of_16_tracks_matches_the_manual_sum() {
    let data = tracks_data(16, 512);
    let mut out = vec![vec![9.0_f32; 512]; 2];
    run_mix(&data, &[], &mut out);
    for c in 0..2 {
        for i in 0..512 {
            let want: f64 = data.iter().map(|t| f64::from(t[c][i])).sum();
            assert!((f64::from(out[c][i]) - want).abs() <= 1e-6, "{c} {i}");
        }
    }
}

#[test]
fn arq_129_there_is_no_track_limit() {
    let data = tracks_data(40, 64);
    let mut out = vec![vec![0.0_f32; 64]; 2];
    run_mix(&data, &[], &mut out);
    let want: f64 = data.iter().map(|t| f64::from(t[1][63])).sum();
    assert!((f64::from(out[1][63]) - want).abs() <= 1e-5);
}

#[test]
fn arq_129_a_muted_track_adds_nothing() {
    let mut data = tracks_data(3, 64);
    data[1] = vec![vec![f32::NAN; 64], vec![100.0; 64]];
    let mut muted_out = vec![vec![0.0_f32; 64]; 2];
    run_mix(&data, &[false, true, false], &mut muted_out);
    let rest = vec![data[0].clone(), data[2].clone()];
    let mut want = vec![vec![0.0_f32; 64]; 2];
    run_mix(&rest, &[], &mut want);
    assert!(want.iter().flatten().any(|&x| x != 0.0));
    assert_eq!(muted_out, want);
    assert!(muted_out.iter().flatten().all(|x| x.abs() < 3.0));
}

#[test]
fn arq_129_with_every_track_muted_or_none_the_output_is_silent() {
    let data = tracks_data(2, 16);
    for muted in [&[true, true][..], &[][..]] {
        let tracks = if muted.is_empty() {
            &data[..0]
        } else {
            &data[..]
        };
        let mut out = vec![vec![5.0_f32; 16]; 2];
        run_mix(tracks, muted, &mut out);
        assert_eq!(out, vec![vec![0.0_f32; 16]; 2]);
    }
}

#[test]
fn arq_123_the_mix_does_not_clip_at_zero_dbfs() {
    let data = vec![vec![vec![0.9_f32; 4]], vec![vec![0.9_f32; 4]]];
    let mut out = vec![vec![0.0_f32; 4]];
    run_mix(&data, &[], &mut out);
    assert!((out[0][0] - 1.8).abs() < 1e-6);
}

#[test]
fn arq_123_the_mix_stays_finite_when_the_sum_overflows_f32() {
    let data = [
        vec![vec![f32::MAX; 2]],
        vec![vec![f32::MAX; 2]],
        vec![vec![-f32::MAX; 2]],
        vec![vec![-f32::MAX; 2]],
        vec![vec![-f32::MAX; 2]],
    ];
    let mut out = vec![vec![0.0_f32; 2]];
    run_mix(&data[..2], &[], &mut out);
    assert_eq!(out[0], vec![f32::MAX; 2]);
    run_mix(&data[2..], &[], &mut out);
    assert_eq!(out[0], vec![-f32::MAX; 2]);
}

#[test]
fn arq_123_non_finite_samples_count_as_zero_and_nothing_is_subnormal() {
    let tiny = f32::MIN_POSITIVE * 0.25;
    let data = vec![
        vec![vec![f32::NAN, f32::INFINITY, f32::NEG_INFINITY, tiny]],
        vec![vec![0.5, 0.5, 0.5, tiny]],
    ];
    let mut out = vec![vec![1.0_f32; 4]];
    run_mix(&data, &[], &mut out);
    assert_eq!(out[0], vec![0.5, 0.5, 0.5, 0.0]);
}

#[test]
fn arq_123_tracks_shorter_or_with_fewer_channels_add_only_what_they_have() {
    let data = vec![
        vec![vec![1.0_f32; 2]],
        vec![vec![1.0_f32; 4], vec![2.0_f32; 4], vec![3.0_f32; 4]],
    ];
    let mut out = vec![vec![0.0_f32; 4], vec![0.0_f32; 4]];
    run_mix(&data, &[], &mut out);
    assert_eq!(out[0], vec![2.0, 2.0, 1.0, 1.0]);
    assert_eq!(out[1], vec![2.0; 4]);
}

#[test]
fn arq_111_the_same_tracks_give_the_same_bytes() {
    let data = tracks_data(16, 128);
    let go = || {
        let mut out = vec![vec![0.0_f32; 128]; 2];
        run_mix(&data, &[], &mut out);
        out.iter()
            .flatten()
            .map(|f| f.to_bits())
            .collect::<Vec<_>>()
    };
    assert_eq!(go(), go());
}
