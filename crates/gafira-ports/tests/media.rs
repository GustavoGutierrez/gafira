//! Media ports (design P8): decoder policy, `AudioBuf::new`, `MediaError` messages.

use std::sync::Arc;

use gafira_doc::contract::{Cancelled, Describe, Descriptor};
use gafira_doc::msg::{MessageKey, MsgArg, ToMessage, is_valid_key};
use gafira_ports::common::LegalClass;
use gafira_ports::media::{
    AudioBuf, Codec, CodecCaps, CodecId, CodecParams, CodecRegistry, Container, Decoder, Demuxer,
    Encoder, EncoderConfig, MediaError, Muxer,
};
use gafira_ports::registry::RegistryError;
use gafira_time::{Flicks, SampleRate};

struct Fake {
    descriptor: Descriptor,
    id: CodecId,
    caps: CodecCaps,
}

impl Describe for Fake {
    fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }
}

impl Codec for Fake {
    fn codec_id(&self) -> CodecId {
        self.id
    }
    fn caps(&self) -> CodecCaps {
        self.caps
    }
    fn decoder(&self, _: &CodecParams) -> Result<Box<dyn Decoder>, MediaError> {
        Err(MediaError::SystemCodecUnavailable)
    }
    fn encoder(&self, _: &EncoderConfig) -> Result<Box<dyn Encoder>, MediaError> {
        Err(MediaError::SystemCodecUnavailable)
    }
}

fn caps(decode: bool, hardware: bool, legal: LegalClass) -> CodecCaps {
    CodecCaps {
        decode,
        encode: true,
        hardware,
        legal,
    }
}

fn fake(name: &'static str, id: CodecId, caps: CodecCaps) -> Arc<dyn Codec> {
    Arc::new(Fake {
        descriptor: Descriptor::new(name, MessageKey("codec.name"), 1),
        id,
        caps,
    })
}

fn registry(codecs: Vec<Arc<dyn Codec>>) -> CodecRegistry {
    let mut reg = CodecRegistry::new();
    for c in codecs {
        reg.register(c).unwrap();
    }
    reg
}

fn best(reg: &CodecRegistry, id: CodecId) -> Option<&'static str> {
    reg.best_decoder(id).map(|c| c.descriptor().id)
}

const RF: LegalClass = LegalClass::RoyaltyFree;
const OS: LegalClass = LegalClass::OsLicensed;
const ENC: LegalClass = LegalClass::Encumbered;

// ARQ-77: an `Encumbered` codec is never chosen, hardware or not.
#[test]
fn arq_77_encumbered_is_never_chosen() {
    let reg = registry(vec![
        fake("enc-hw", CodecId::H264, caps(true, true, ENC)),
        fake("enc-sw", CodecId::H264, caps(true, false, ENC)),
    ]);
    assert_eq!(best(&reg, CodecId::H264), None);
    let reg = registry(vec![
        fake("enc", CodecId::H264, caps(true, true, ENC)),
        fake("rf", CodecId::H264, caps(true, false, RF)),
    ]);
    assert_eq!(best(&reg, CodecId::H264), Some("rf"));
}

// ARQ-77: an encode-only codec is not a decoder.
#[test]
fn arq_77_encode_only_is_not_chosen() {
    let reg = registry(vec![
        fake("only-enc", CodecId::Av1, caps(false, true, RF)),
        fake("dec", CodecId::Av1, caps(true, false, RF)),
    ]);
    assert_eq!(best(&reg, CodecId::Av1), Some("dec"));
    let reg = registry(vec![fake("only-enc", CodecId::Av1, caps(false, true, RF))]);
    assert_eq!(best(&reg, CodecId::Av1), None);
}

// ARQ-77: only a codec with the asked id matches.
#[test]
fn arq_77_other_codec_id_is_not_chosen() {
    let reg = registry(vec![
        fake("vp9", CodecId::Vp9, caps(true, true, RF)),
        fake("av1", CodecId::Av1, caps(true, false, RF)),
    ]);
    assert_eq!(best(&reg, CodecId::Av1), Some("av1"));
    assert_eq!(best(&reg, CodecId::Vp9), Some("vp9"));
    assert_eq!(best(&reg, CodecId::Opus), None);
    assert_eq!(best(&CodecRegistry::new(), CodecId::Av1), None);
}

// ARQ-77: hardware beats `OsLicensed`, which beats `RoyaltyFree`, whatever the
// registration order.
#[test]
fn arq_77_order_is_hardware_then_os_then_royalty_free() {
    let hw = ("hw", caps(true, true, RF));
    let os = ("os", caps(true, false, OS));
    let rf = ("rf", caps(true, false, RF));
    let orders = [
        [hw, os, rf],
        [rf, os, hw],
        [os, rf, hw],
        [os, hw, rf],
        [rf, hw, os],
        [hw, rf, os],
    ];
    for order in orders {
        let all = order
            .iter()
            .map(|(n, c)| fake(n, CodecId::Aac, *c))
            .collect();
        assert_eq!(best(&registry(all), CodecId::Aac), Some("hw"), "{order:?}");
        let no_hw = order
            .iter()
            .filter(|(n, _)| *n != "hw")
            .map(|(n, c)| fake(n, CodecId::Aac, *c))
            .collect();
        assert_eq!(
            best(&registry(no_hw), CodecId::Aac),
            Some("os"),
            "{order:?}"
        );
    }
    // A hardware `OsLicensed` codec still beats a software one of the same class.
    let reg = registry(vec![
        fake("os-sw", CodecId::Aac, caps(true, false, OS)),
        fake("os-hw", CodecId::Aac, caps(true, true, OS)),
    ]);
    assert_eq!(best(&reg, CodecId::Aac), Some("os-hw"));
}

// ARQ-77: with equal rank the first registered wins.
#[test]
fn arq_77_tie_takes_the_first_registered() {
    for c in [
        caps(true, true, RF),
        caps(true, false, OS),
        caps(true, false, RF),
    ] {
        let reg = registry(vec![
            fake("a", CodecId::Opus, c),
            fake("b", CodecId::Opus, c),
            fake("c", CodecId::Opus, c),
        ]);
        assert_eq!(best(&reg, CodecId::Opus), Some("a"), "{c:?}");
    }
}

// ARQ-31: the codec registry is a registry: duplicate ids are refused, order kept.
#[test]
fn arq_31_codec_registry_is_a_registry() {
    let mut reg = CodecRegistry::new();
    assert!(reg.is_empty());
    reg.register(fake("z", CodecId::Av1, caps(true, false, RF)))
        .unwrap();
    reg.register(fake("a", CodecId::Opus, caps(true, false, RF)))
        .unwrap();
    let err = reg
        .register(fake("z", CodecId::Vp9, caps(true, false, RF)))
        .unwrap_err();
    assert_eq!(err, RegistryError::DuplicateId { id: "z" });
    assert_eq!(reg.len(), 2);
    let ids: Vec<_> = reg.iter().map(|c| c.descriptor().id).collect();
    assert_eq!(ids, ["z", "a"]);
    assert_eq!(reg.get("a").map(|c| c.codec_id()), Some(CodecId::Opus));
    assert!(reg.get("nope").is_none());
}

// V-5: `LegalClass` has exactly these three classes.
#[test]
fn v5_legal_class_has_three_classes() {
    for c in [RF, OS, ENC] {
        let n = match c {
            LegalClass::RoyaltyFree => 1,
            LegalClass::OsLicensed => 2,
            LegalClass::Encumbered => 3,
        };
        assert!((1..=3).contains(&n));
    }
}

// Every public trait is object-safe.
#[test]
fn media_traits_are_dyn_compatible() {
    fn _names(
        _: &dyn Container,
        _: &dyn Demuxer,
        _: &dyn Muxer,
        _: &dyn Codec,
        _: &dyn Decoder,
        _: &dyn Encoder,
    ) {
    }
}

fn rate() -> SampleRate {
    SampleRate::new(48_000).unwrap()
}

fn audio(channels: Vec<Vec<f32>>) -> Result<AudioBuf, MediaError> {
    AudioBuf::new(Flicks::ZERO, rate(), channels)
}

// P8: no channels gives an empty buffer; equal lengths are kept as given.
#[test]
fn p8_audio_buf_without_channels_is_empty() {
    let buf = audio(vec![]).unwrap();
    assert!(buf.channels().is_empty());
    assert_eq!(buf.frames(), 0);
    assert_eq!(buf.pts(), Flicks::ZERO);
    assert_eq!(buf.sample_rate(), rate());
}

#[test]
fn p8_audio_buf_with_equal_channels_keeps_them() {
    let buf = audio(vec![vec![0.0, 1.0, 2.0], vec![3.0, 4.0, 5.0]]).unwrap();
    assert_eq!(buf.frames(), 3);
    assert_eq!(buf.channels()[1], [3.0, 4.0, 5.0]);
    assert_eq!(audio(vec![vec![], vec![]]).unwrap().frames(), 0);
}

// P8: the error names the first bad channel; every channel is checked.
#[test]
fn p8_audio_buf_names_the_first_bad_channel() {
    let cases: [(Vec<Vec<f32>>, usize, usize, usize); 4] = [
        (vec![vec![0.0; 4], vec![0.0; 3]], 4, 3, 1),
        (vec![vec![0.0; 4], vec![0.0; 4], vec![0.0; 5]], 4, 5, 2),
        (vec![vec![0.0; 4], vec![0.0; 2], vec![0.0; 9]], 4, 2, 1),
        (vec![vec![], vec![0.0; 1]], 0, 1, 1),
    ];
    for (channels, expected, actual, channel) in cases {
        match audio(channels).unwrap_err() {
            MediaError::ChannelMismatch {
                expected: e,
                actual: a,
                channel: c,
            } => assert_eq!((e, a, c), (expected, actual, channel)),
            other => panic!("unexpected {other:?}"),
        }
    }
}

fn args(e: &MediaError) -> Vec<(&'static str, MsgArg)> {
    let m = e.to_message();
    assert!(is_valid_key(m.key.0), "{}", m.key.0);
    m.args
}

// ARQ-102: one fixture per variant; key area `error.media`; names are field names.
#[test]
fn arq_102_media_error_messages() {
    type Fixture = (MediaError, &'static str, Vec<(&'static str, MsgArg)>);
    let fixtures: Vec<Fixture> = vec![
        (
            MediaError::UnsupportedCodec {
                codec: CodecId::Hevc,
            },
            "error.media.unsupported_codec",
            vec![("codec", MsgArg::Text("hevc".into()))],
        ),
        (
            MediaError::Corrupt {
                detail: "secret".into(),
            },
            "error.media.corrupt",
            vec![],
        ),
        (
            MediaError::SystemCodecUnavailable,
            "error.media.system_codec_unavailable",
            vec![],
        ),
        (
            MediaError::ChannelMismatch {
                expected: 4,
                actual: 3,
                channel: 1,
            },
            "error.media.channel_mismatch",
            vec![
                ("expected", MsgArg::Count(4)),
                ("actual", MsgArg::Count(3)),
                ("channel", MsgArg::Count(1)),
            ],
        ),
        (
            MediaError::Io(std::io::Error::other("disk")),
            "error.media.io",
            vec![],
        ),
    ];
    for (err, key, want) in fixtures {
        let m = err.to_message();
        assert_eq!(m.key.0, key);
        assert!(is_valid_key(m.key.0));
        assert_eq!(args(&err), want, "{key}");
        assert!(!format!("{:?}", m.args).contains("secret"));
    }
    // `Cancelled` delegates (ADR 0012).
    assert_eq!(
        MediaError::Cancelled(Cancelled).to_message(),
        Cancelled.to_message()
    );
}
