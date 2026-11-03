//! P13 and 02 §4.17.5: `params()` is the processor's `ParamSchema`, reached
//! through `dyn AudioProcessor`.

use gafira_audio::{
    AudioBlock, AudioProcessor, Fade, FadeDirection, Gain, LoudnessNormalizer, PARAM_DURATION,
    PARAM_GAIN, PARAM_PAN, PARAM_VOLUME, Pan, ParamSnapshot, ProcessorChain,
};
use gafira_doc::contract::{ParamKind, ParamSchema};
use gafira_doc::msg::MessageKey;

struct Row {
    name: &'static str,
    make: fn() -> Box<dyn AudioProcessor>,
    id: &'static str,
    min: f64,
    max: f64,
    default: f64,
    animatable: bool,
    label: &'static str,
}

fn table() -> Vec<Row> {
    vec![
        Row {
            name: "gain",
            make: || Box::new(Gain::new()),
            id: "volume",
            min: 0.0,
            max: 200.0,
            default: 100.0,
            animatable: true,
            label: "audio.gain.volume",
        },
        Row {
            name: "fade_in",
            make: || Box::new(Fade::new(FadeDirection::In)),
            id: "duration",
            min: 0.0,
            max: f64::MAX,
            default: 0.0,
            animatable: false,
            label: "audio.fade.duration",
        },
        Row {
            name: "fade_out",
            make: || Box::new(Fade::new(FadeDirection::Out)),
            id: "duration",
            min: 0.0,
            max: f64::MAX,
            default: 0.0,
            animatable: false,
            label: "audio.fade.duration",
        },
        Row {
            name: "pan",
            make: || Box::new(Pan::new()),
            id: "pan",
            min: -1.0,
            max: 1.0,
            default: 0.0,
            animatable: false,
            label: "audio.pan.pan",
        },
        Row {
            name: "loudness_normalizer",
            make: || Box::new(LoudnessNormalizer::new()),
            id: "gain",
            min: -f64::MAX,
            max: f64::MAX,
            default: 0.0,
            animatable: false,
            label: "audio.loudness_normalizer.gain",
        },
    ]
}

#[test]
fn p13_every_processor_schema_passes_validate_definition_and_matches_the_table() {
    for row in table() {
        let p = (row.make)();
        let schema = p.params();
        assert_eq!(schema.validate_definition(), Ok(()), "{}", row.name);
        assert_eq!(schema.params().len(), 1, "{}", row.name);
        let def = &schema.params()[0];
        assert_eq!(def.id, row.id, "{}", row.name);
        assert_eq!(def.animatable, row.animatable, "{}", row.name);
        assert_eq!(def.label, MessageKey(row.label), "{}", row.name);
        assert_eq!(
            def.kind,
            ParamKind::Float {
                min: row.min,
                max: row.max,
                default: row.default
            },
            "{}",
            row.name
        );
    }
}

#[test]
fn p13_the_ids_are_the_public_constants() {
    assert_eq!(
        (PARAM_VOLUME, PARAM_DURATION, PARAM_PAN, PARAM_GAIN),
        ("volume", "duration", "pan", "gain")
    );
}

#[test]
fn p13_a_chain_has_an_empty_schema_whatever_it_holds() {
    let mut chain = ProcessorChain::new();
    let empty: &ParamSchema = &ParamSchema::new(Vec::new());
    assert_eq!(chain.params(), empty);
    assert_eq!(chain.params().validate_definition(), Ok(()));
    chain.push(Box::new(Gain::new()));
    chain.push(Box::new(Pan::new()));
    chain.push(Box::new(Fade::new(FadeDirection::In)));
    chain.push(Box::new(Fade::new(FadeDirection::Out)));
    assert_eq!(chain.params(), empty, "stages keep their own schemas");
    assert!(chain.params().params().is_empty());
}

fn stereo_one(p: &mut dyn AudioProcessor, id: &'static str, value: f64) -> (f32, f32) {
    let mut s = ParamSnapshot::new();
    s.set(id, value);
    p.prepare(48_000, 64, 2);
    let mut chans = [vec![1.0_f32; 4], vec![1.0_f32; 4]];
    let mut slices: Vec<&mut [f32]> = chans.iter_mut().map(Vec::as_mut_slice).collect();
    p.process(&mut AudioBlock::new(&mut slices), &s);
    (chans[0][3], chans[1][3])
}

#[test]
fn arq_123_values_beyond_the_schema_bounds_are_clipped_to_them() {
    // The numbers come from the schema, so a bound that moved in only one
    // place would show here.
    for row in table().into_iter().filter(|r| r.name != "fade_out") {
        if row.name == "fade_in" {
            continue; // `duration` has no finite upper bound to cross
        }
        for (outside, inside) in [(row.max + 50.0, row.max), (row.min - 50.0, row.min)] {
            let a = stereo_one((row.make)().as_mut(), row.id, outside);
            let b = stereo_one((row.make)().as_mut(), row.id, inside);
            assert_eq!(a, b, "{} {outside}", row.name);
        }
    }
    // Spot values of the table: volume 250 -> 200 %, pan 2 -> 1, duration -1 -> 0.
    assert_eq!(stereo_one(&mut Gain::new(), "volume", 250.0).0, 2.0);
    assert_eq!(stereo_one(&mut Gain::new(), "volume", 200.0).0, 2.0);
    assert_eq!(stereo_one(&mut Gain::new(), "volume", 0.0).0, 0.0);
    assert_eq!(
        stereo_one(&mut Pan::new(), "pan", 2.0),
        stereo_one(&mut Pan::new(), "pan", 1.0)
    );
    assert_eq!(
        stereo_one(&mut Fade::new(FadeDirection::In), "duration", -1.0),
        (1.0, 1.0)
    );
}
