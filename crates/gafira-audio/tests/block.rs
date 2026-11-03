//! ARQ-23 (snapshot shape, ADR 0005), ARQ-102 and ARQ-110 for the shared types.

use gafira_audio::{AudioBlock, AudioError, MAX_PARAMS, ParamSnapshot};
use gafira_doc::msg::{Message, MessageKey, MsgArg, ToMessage, is_valid_key};

const IDS: [&str; 17] = [
    "p00", "p01", "p02", "p03", "p04", "p05", "p06", "p07", "p08", "p09", "p10", "p11", "p12",
    "p13", "p14", "p15", "p16",
];

#[test]
fn arq_23_snapshot_holds_sixteen_pairs_and_refuses_the_seventeenth() {
    let mut s = ParamSnapshot::new();
    assert!(s.is_empty());
    for (i, id) in IDS[..MAX_PARAMS].iter().enumerate() {
        assert!(s.set(id, i as f64), "{id}");
    }
    assert_eq!(s.len(), 16);
    let before = s;
    assert!(!s.set(IDS[16], 99.0));
    assert_eq!(s, before, "a refused pair changes nothing");
    assert_eq!(s.get(IDS[16]), None);
    for (i, id) in IDS[..MAX_PARAMS].iter().enumerate() {
        assert_eq!(s.get(id), Some(i as f64));
    }
}

#[test]
fn arq_23_setting_an_existing_id_replaces_it_even_when_full() {
    let mut s = ParamSnapshot::new();
    for id in &IDS[..MAX_PARAMS] {
        assert!(s.set(id, 1.0));
    }
    assert!(s.set("p03", 7.5));
    assert_eq!(s.len(), 16);
    assert_eq!(s.get("p03"), Some(7.5));
}

#[test]
fn arq_23_snapshot_is_copy_and_keeps_non_finite_values_raw() {
    let mut s = ParamSnapshot::new();
    s.set("a", f64::NAN);
    let copy = s;
    assert!(copy.get("a").is_some_and(f64::is_nan));
    assert!(s.get("a").is_some_and(f64::is_nan));
    assert_eq!(s.get("missing"), None);
}

#[test]
fn adr_0005_block_frames_are_the_shortest_channel() {
    let mut empty: [&mut [f32]; 0] = [];
    let b = AudioBlock::new(&mut empty);
    assert_eq!((b.frames(), b.is_uniform()), (0, true));

    let (mut l, mut r) = ([0.0_f32; 4], [0.0_f32; 3]);
    let mut chans = [l.as_mut_slice(), r.as_mut_slice()];
    let b = AudioBlock::new(&mut chans);
    assert_eq!((b.frames(), b.is_uniform()), (3, false));
}

#[test]
fn arq_102_audio_error_gives_its_key_and_names_the_field() {
    let e = AudioError::NonFinite { field: "db" };
    let expected = Message {
        key: MessageKey("error.audio.non_finite"),
        args: vec![("field", MsgArg::Text("db".to_string()))],
    };
    assert_eq!(e.to_message(), expected);
    assert!(is_valid_key(e.to_message().key.0));
    assert!(e.to_string().contains("db"));
}
