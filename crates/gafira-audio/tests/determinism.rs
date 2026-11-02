//! ARQ-111: each pure function of the crate, run twice, gives the same bytes.

use gafira_audio::{crossfade_gains, db_to_gain, flicks_to_samples, percent_to_gain};
use gafira_doc::contract::AudioCrossfade;
use gafira_time::{Flicks, SampleRate};

const F64S: [f64; 12] = [
    0.0,
    -0.0,
    1.0,
    -6.0,
    6.0,
    200.0,
    0.5,
    1e300,
    f64::NAN,
    f64::INFINITY,
    f64::NEG_INFINITY,
    f64::MIN_POSITIVE,
];

fn bits<T, E>(r: &Result<T, E>, f: impl Fn(&T) -> Vec<u64>) -> Option<Vec<u64>> {
    r.as_ref().ok().map(f)
}

#[test]
fn arq_111_db_to_gain_and_percent_to_gain_repeat_byte_for_byte() {
    for x in F64S {
        let (a, b) = (db_to_gain(x), db_to_gain(x));
        assert_eq!(a, b, "{x}");
        assert_eq!(
            bits(&a, |v| vec![v.to_bits()]),
            bits(&b, |v| vec![v.to_bits()])
        );
        let (a, b) = (percent_to_gain(x), percent_to_gain(x));
        assert_eq!(a, b, "{x}");
        assert_eq!(
            bits(&a, |v| vec![v.to_bits()]),
            bits(&b, |v| vec![v.to_bits()])
        );
    }
}

#[test]
fn arq_111_crossfade_gains_repeat_byte_for_byte() {
    for x in F64S {
        let a = crossfade_gains(AudioCrossfade::EqualPower, x);
        let b = crossfade_gains(AudioCrossfade::EqualPower, x);
        assert_eq!(
            bits(&a, |v| vec![v.0.to_bits(), v.1.to_bits()]),
            bits(&b, |v| vec![v.0.to_bits(), v.1.to_bits()]),
            "{x}"
        );
        assert_eq!(a.is_ok(), b.is_ok());
    }
}

#[test]
fn arq_111_flick_offsets_repeat() {
    let ticks = [0, 1, -1, 7_350, -7_350, 705_600, i64::MAX, i64::MIN];
    for t in ticks {
        for rate in [
            SampleRate::HZ_44_100,
            SampleRate::HZ_48_000,
            SampleRate::HZ_96_000,
        ] {
            let once = flicks_to_samples(Flicks::from_ticks(t), rate);
            let twice = flicks_to_samples(Flicks::from_ticks(t), rate);
            assert_eq!(once, twice, "{t} at {}", rate.hz());
        }
    }
}
