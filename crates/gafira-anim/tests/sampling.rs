use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use gafira_anim::{Animatable, Easing, Interpolate, KeyTrack, Keyframe};
use gafira_geom::Vec2;
use gafira_time::Flicks;
use proptest::prelude::*;

fn fl(t: i64) -> Flicks {
    Flicks::from_ticks(t)
}

fn key<T>(at: i64, value: T, out: Easing) -> Keyframe<T> {
    Keyframe {
        at: fl(at),
        value,
        out,
    }
}

fn track<T: Interpolate>(keys: Vec<Keyframe<T>>) -> Animatable<T> {
    Animatable::Keyed(KeyTrack::new(keys).expect("valid track"))
}

/// Keys at 100, 200, 300 with values 0, 10, 20. The `out` of each key is
/// given, so tests can tell which key's curve is used.
fn three(out0: Easing, out1: Easing, out2: Easing) -> Animatable<f32> {
    track(vec![
        key(100, 0.0, out0),
        key(200, 10.0, out1),
        key(300, 20.0, out2),
    ])
}

// ARQ-16: Static(v) is v for any t.
#[test]
fn arq_16_static_is_constant() {
    let a = Animatable::Static(3.5_f32);
    for t in [i64::MIN, -1, 0, 1, i64::MAX] {
        assert_eq!(a.sample(fl(t)), 3.5);
    }
}

// ARQ-16: t <= first key holds the first value, including one flick before and far before.
#[test]
fn arq_16_before_the_first_key_holds_its_value() {
    let a = three(Easing::Linear, Easing::Linear, Easing::Linear);
    for t in [i64::MIN, -100, 98, 99, 100] {
        assert_eq!(a.sample(fl(t)), 0.0, "t = {t}");
    }
}

// ARQ-16: t >= last key holds the last value; the last key's `out` is not used.
#[test]
fn arq_16_from_the_last_key_holds_its_value() {
    for out in [Easing::Hold, Easing::Linear, Easing::EaseIn] {
        let a = three(Easing::Linear, Easing::Linear, out);
        for t in [300, 301, 1_000, i64::MAX] {
            assert_eq!(a.sample(fl(t)), 20.0, "t = {t}");
        }
    }
}

// ARQ-16: t == keys[i].at returns keys[i].value without interpolating.
#[test]
fn arq_16_at_a_key_returns_its_value() {
    let a = three(Easing::EaseIn, Easing::Hold, Easing::Linear);
    assert_eq!(a.sample(fl(100)), 0.0);
    assert_eq!(a.sample(fl(200)), 10.0);
    assert_eq!(a.sample(fl(300)), 20.0);
}

// ARQ-16: between keys, u = (t - at_i) / (at_{i+1} - at_i) and the previous key's curve applies.
#[test]
fn arq_16_between_keys_uses_the_previous_keys_curve() {
    let a = three(Easing::Linear, Easing::EaseIn, Easing::EaseOut);
    // Segment 0 -> 1, Linear: u = 0.25, 0.5, 0.75.
    assert_eq!(a.sample(fl(125)), 2.5);
    assert_eq!(a.sample(fl(150)), 5.0);
    assert_eq!(a.sample(fl(175)), 7.5);
    // Segment 1 -> 2 uses keys[1].out = EaseIn, not keys[2].out = EaseOut.
    let e = Easing::EaseIn.apply(0.5);
    assert_eq!(a.sample(fl(250)), 10.0_f32.interpolate(&20.0, e));
    assert_ne!(e, Easing::EaseOut.apply(0.5));
    assert_ne!(e, 0.5);
}

// ARQ-16: one flick after a key and one flick before the next, in a 3-key track.
#[test]
fn arq_16_one_flick_from_each_key() {
    let a = three(Easing::Linear, Easing::Linear, Easing::Linear);
    for (t, want) in [(101, 0.1_f32), (199, 9.9), (201, 10.1), (299, 19.9)] {
        let got = a.sample(fl(t));
        assert!((got - want).abs() < 1e-5, "t = {t}: {got} vs {want}");
    }
}

// ARQ-16: Hold keeps the previous value until the next key, where it jumps.
#[test]
fn arq_16_hold_keeps_the_previous_value_until_the_next_key() {
    let a = three(Easing::Hold, Easing::Hold, Easing::Linear);
    assert_eq!(a.sample(fl(101)), 0.0);
    assert_eq!(a.sample(fl(199)), 0.0);
    assert_eq!(a.sample(fl(200)), 10.0);
    assert_eq!(a.sample(fl(299)), 10.0);
    assert_eq!(a.sample(fl(300)), 20.0);
}

// ARQ-16: a single-key track returns that value for any t.
#[test]
fn arq_16_single_key_is_constant() {
    let a = track(vec![key(5, 7.0_f32, Easing::Linear)]);
    for t in [i64::MIN, 4, 5, 6, i64::MAX] {
        assert_eq!(a.sample(fl(t)), 7.0);
    }
}

// ARQ-16: keys one flick apart, and keys at the i64 extremes, do not panic or overflow.
#[test]
fn arq_16_adjacent_and_extreme_keys_are_total() {
    let a = track(vec![
        key(0, 0.0_f32, Easing::Linear),
        key(1, 1.0, Easing::Linear),
    ]);
    assert_eq!(a.sample(fl(0)), 0.0);
    assert_eq!(a.sample(fl(1)), 1.0);
    let b = track(vec![
        key(i64::MIN, 0.0_f32, Easing::Linear),
        key(i64::MAX, 1.0, Easing::Linear),
    ]);
    assert_eq!(b.sample(fl(i64::MIN)), 0.0);
    assert_eq!(b.sample(fl(i64::MAX)), 1.0);
    assert_eq!(b.sample(fl(-1)), 0.5);
    assert_eq!(b.sample(fl(0)), 0.5);
    let c = track(vec![
        key(i64::MAX - 1, 0.0_f32, Easing::Linear),
        key(i64::MAX, 1.0, Easing::Linear),
    ]);
    assert_eq!(c.sample(fl(i64::MAX - 1)), 0.0);
    assert_eq!(c.sample(fl(i64::MIN)), 0.0);
}

/// A value that counts how many times `interpolate` is called on any copy.
#[derive(Clone, Debug)]
struct Counting {
    id: u32,
    calls: Arc<AtomicUsize>,
}

impl PartialEq for Counting {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Interpolate for Counting {
    fn interpolate(&self, _to: &Self, _t: f32) -> Self {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.clone()
    }

    fn is_finite(&self) -> bool {
        true
    }
}

// ARQ-12 (structure): at most one interpolate per sample, none at a key,
// outside the key range or with Hold.
#[test]
fn arq_12_sample_calls_interpolate_at_most_once() {
    let calls = Arc::new(AtomicUsize::new(0));
    let v = |id| Counting {
        id,
        calls: calls.clone(),
    };
    let a = track(vec![
        key(0, v(0), Easing::Linear),
        key(10, v(1), Easing::Hold),
        key(20, v(2), Easing::Linear),
        key(30, v(3), Easing::Linear),
    ]);
    let calls_for = |t: i64| {
        calls.store(0, Ordering::SeqCst);
        let _ = a.sample(fl(t));
        calls.load(Ordering::SeqCst)
    };
    // No call: before the range, at every key, after the range, inside a Hold segment.
    for t in [i64::MIN, -1, 0, 10, 20, 30, 31, i64::MAX, 11, 15, 19] {
        assert_eq!(calls_for(t), 0, "t = {t}");
    }
    // Exactly one call inside a non-Hold segment.
    for t in [1, 5, 9, 21, 25, 29] {
        assert_eq!(calls_for(t), 1, "t = {t}");
    }
}

fn instant() -> impl Strategy<Value = i64> {
    prop_oneof![
        3 => any::<i64>(),
        1 => Just(i64::MIN),
        1 => Just(i64::MAX),
        1 => -5_i64..5,
    ]
}

fn easing() -> impl Strategy<Value = Easing> {
    prop_oneof![
        Just(Easing::Hold),
        Just(Easing::Linear),
        Just(Easing::EaseIn),
        Just(Easing::EaseOut),
        Just(Easing::EaseInOut),
        (0.0_f32..=1.0, -2.0_f32..3.0, 0.0_f32..=1.0, -2.0_f32..3.0)
            .prop_map(|(x1, y1, x2, y2)| Easing::CubicBezier { x1, y1, x2, y2 }),
    ]
}

/// A valid f32 track plus the instants at, and one flick around, each key.
fn f32_track(outs: BoxedStrategy<Easing>) -> impl Strategy<Value = Animatable<f32>> {
    (
        proptest::collection::btree_set(instant(), 1..8),
        proptest::collection::vec((-100.0_f32..100.0, outs), 8),
    )
        .prop_map(|(ats, vals)| {
            let keys = ats
                .into_iter()
                .zip(vals)
                .map(|(at, (value, out))| key(at, value, out))
                .collect();
            track(keys)
        })
}

fn keys_of(a: &Animatable<f32>) -> &[Keyframe<f32>] {
    match a {
        Animatable::Keyed(t) => t.keys(),
        Animatable::Static(_) => unreachable!(),
    }
}

fn probes(a: &Animatable<f32>, extra: i64) -> Vec<i64> {
    let mut v = vec![extra, i64::MIN, i64::MAX];
    for k in keys_of(a) {
        let at = k.at.ticks();
        v.extend([at, at.saturating_sub(1), at.saturating_add(1)]);
    }
    v
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    // ARQ-16, ARQ-12: sample at each key's `at` is its value.
    #[test]
    fn arq_16_sample_at_each_key_returns_its_value(a in f32_track(easing().boxed())) {
        for k in keys_of(&a) {
            prop_assert_eq!(a.sample(k.at).to_bits(), k.value.to_bits());
        }
    }

    // ARQ-16: outside the key range the end values are held.
    #[test]
    fn arq_16_ends_are_held(a in f32_track(easing().boxed()), t in instant()) {
        let keys = keys_of(&a);
        let (first, last) = (&keys[0], &keys[keys.len() - 1]);
        if fl(t) <= first.at {
            prop_assert_eq!(a.sample(fl(t)).to_bits(), first.value.to_bits());
        }
        if fl(t) >= last.at {
            prop_assert_eq!(a.sample(fl(t)).to_bits(), last.value.to_bits());
        }
    }

    // ARQ-12: same track and same t give bit-identical results; sample never panics.
    #[test]
    fn arq_12_sample_is_deterministic_and_total(a in f32_track(easing().boxed()), t in instant()) {
        for t in probes(&a, t) {
            let (x, y) = (a.sample(fl(t)), a.clone().sample(fl(t)));
            prop_assert_eq!(x.to_bits(), y.to_bits());
        }
    }

    // ARQ-16: with Linear the result is between the neighbors (tolerance of ARQ-19).
    #[test]
    fn arq_16_linear_stays_between_neighbors(a in f32_track(Just(Easing::Linear).boxed()), t in instant()) {
        let keys = keys_of(&a);
        for t in probes(&a, t) {
            let Some(i) = keys.iter().rposition(|k| k.at.ticks() <= t) else { continue };
            let Some(next) = keys.get(i + 1) else { continue };
            let (lo, hi) = (keys[i].value, next.value);
            let tol = f32::EPSILON * lo.abs().max(hi.abs());
            let r = a.sample(fl(t));
            prop_assert!(r >= lo.min(hi) - tol && r <= lo.max(hi) + tol, "{lo} {hi} {r}");
        }
    }

    // ARQ-16: with Hold, keys[i].value holds for every t < keys[i+1].at.
    #[test]
    fn arq_16_hold_holds_until_the_next_key(a in f32_track(Just(Easing::Hold).boxed()), t in instant()) {
        let keys = keys_of(&a);
        for t in probes(&a, t) {
            let Some(i) = keys.iter().rposition(|k| k.at.ticks() <= t) else { continue };
            prop_assert_eq!(a.sample(fl(t)).to_bits(), keys[i].value.to_bits());
        }
    }

    // ARQ-16: within a segment u does not decrease as t grows, so an increasing
    // Linear track never decreases.
    #[test]
    fn arq_16_progress_is_monotone_in_t(ats in proptest::collection::btree_set(instant(), 2..6), t1 in instant(), t2 in instant()) {
        let keys = ats.into_iter().enumerate().map(|(i, at)| key(at, i as f32, Easing::Linear)).collect();
        let a = track(keys);
        let (lo, hi) = (t1.min(t2), t1.max(t2));
        prop_assert!(a.sample(fl(lo)) <= a.sample(fl(hi)));
    }
}

// ARQ-16: sample works for Vec2 through the same path.
#[test]
fn arq_16_vec2_midpoint() {
    let a = track(vec![
        key(0, Vec2::new(0.0, 10.0), Easing::Linear),
        key(100, Vec2::new(10.0, 20.0), Easing::Linear),
    ]);
    assert_eq!(a.sample(fl(50)), Vec2::new(5.0, 15.0));
}
