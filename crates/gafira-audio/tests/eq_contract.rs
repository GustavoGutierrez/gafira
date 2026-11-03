//! ARQ-125, ARQ-123, P13: the schema and the parameter rules of `Eq3Band`.

use gafira_audio::{AudioBlock, AudioProcessor, Eq3Band, PARAM_HIGH, PARAM_LOW, PARAM_MID};
use gafira_doc::contract::ParamKind;
use gafira_doc::msg::MessageKey;

fn one(low: Option<f64>, mid: Option<f64>, high: Option<f64>) -> impl Fn(&mut Eq3Band) -> Vec<u32> {
    move |e: &mut Eq3Band| {
        let mut s = gafira_audio::ParamSnapshot::new();
        for (id, v) in [("low", low), ("mid", mid), ("high", high)] {
            if let Some(v) = v {
                s.set(id, v);
            }
        }
        let mut chans: Vec<Vec<f32>> = vec![
            (0..512)
                .map(|i| ((i * 37 % 101) as f32 - 50.0) / 25.0)
                .collect(),
        ];
        let mut slices: Vec<&mut [f32]> = chans.iter_mut().map(Vec::as_mut_slice).collect();
        e.process(&mut AudioBlock::new(&mut slices), &s);
        chans[0].iter().map(|f| f.to_bits()).collect()
    }
}

fn prepared() -> Eq3Band {
    let mut e = Eq3Band::new();
    e.prepare(48_000, 512, 1);
    e
}

#[test]
fn p13_the_schema_has_the_three_bands_of_the_table() {
    let e = Eq3Band::new();
    let schema = e.params();
    assert_eq!(schema.validate_definition(), Ok(()));
    let rows = [
        ("low", "audio.eq3band.low"),
        ("mid", "audio.eq3band.mid"),
        ("high", "audio.eq3band.high"),
    ];
    assert_eq!(schema.params().len(), 3);
    for (def, (id, label)) in schema.params().iter().zip(rows) {
        assert_eq!(def.id, id);
        assert_eq!(def.label, MessageKey(label));
        assert!(!def.animatable, "{id}");
        assert_eq!(
            def.kind,
            ParamKind::Float {
                min: -15.0,
                max: 15.0,
                default: 0.0
            },
            "{id}"
        );
    }
    assert_eq!((PARAM_LOW, PARAM_MID, PARAM_HIGH), ("low", "mid", "high"));
}

#[test]
fn arq_123_a_band_beyond_15_db_is_clipped_to_15_and_at_the_extremes_it_is_kept() {
    for band in 0..3 {
        let pick = |v: f64| {
            let mut a = [None; 3];
            a[band] = Some(v);
            one(a[0], a[1], a[2])(&mut prepared())
        };
        assert_eq!(pick(16.0), pick(15.0), "band {band} +16");
        assert_eq!(pick(-16.0), pick(-15.0), "band {band} -16");
        assert_eq!(pick(1e300), pick(15.0), "band {band} huge");
        // A value at the extreme is applied, not clipped further or ignored.
        assert_ne!(pick(15.0), pick(14.9), "band {band} +15");
        assert_ne!(pick(-15.0), pick(-14.9), "band {band} -15");
        assert_ne!(pick(15.0), pick(0.0), "band {band} +15 is not 0");
    }
}

#[test]
fn arq_123_a_non_finite_band_value_keeps_the_last_valid_one() {
    for band in 0..3 {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let put = |v: Option<f64>| {
                let mut a = [None; 3];
                a[band] = v;
                one(a[0], a[1], a[2])
            };
            let mut with_bad = prepared();
            let mut steady = prepared();
            let _ = put(Some(6.0))(&mut with_bad);
            let _ = put(Some(6.0))(&mut steady);
            assert_eq!(
                put(Some(bad))(&mut with_bad),
                put(Some(6.0))(&mut steady),
                "band {band} {bad}"
            );
            // A missing value keeps it too.
            assert_eq!(
                put(None)(&mut with_bad),
                put(Some(6.0))(&mut steady),
                "band {band} none"
            );
        }
    }
}

#[test]
fn arq_123_an_id_that_is_not_a_band_changes_nothing() {
    let mut noisy = gafira_audio::ParamSnapshot::new();
    noisy.set("volume", 20.0);
    noisy.set("pan", -1.0);
    noisy.set("duration", f64::NAN);
    noisy.set("low", 0.0);
    let run = |s: &gafira_audio::ParamSnapshot| {
        let mut e = prepared();
        let mut chans = [vec![0.7_f32; 64]];
        let mut slices: Vec<&mut [f32]> = chans.iter_mut().map(Vec::as_mut_slice).collect();
        e.process(&mut AudioBlock::new(&mut slices), s);
        chans[0].iter().map(|f| f.to_bits()).collect::<Vec<u32>>()
    };
    assert_eq!(run(&noisy), run(&gafira_audio::ParamSnapshot::new()));
}
