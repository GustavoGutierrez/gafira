//! The fake codec of ARQ-112: the port double a test injects in place of a
//! real codec (02 §4.8).

use gafira_doc::contract::{Describe, Descriptor};
use gafira_doc::msg::MessageKey;
use gafira_ports::media::{
    Codec, CodecCaps, CodecId, CodecParams, Decoder, Encoder, EncoderConfig, MediaError,
};

/// The descriptor of the fake codec.
static DESCRIPTOR: Descriptor = Descriptor::new("test.codec", MessageKey("test.codec"), 1);

/// The fake codec. A stub: the media stream fills it.
pub struct FakeCodec;

impl Describe for FakeCodec {
    /// The descriptor of the double.
    fn descriptor(&self) -> &Descriptor {
        &DESCRIPTOR
    }
}

impl Codec for FakeCodec {
    /// Not filled yet.
    fn codec_id(&self) -> CodecId {
        panic!("stub: the codec double is not filled yet")
    }

    /// Not filled yet.
    fn caps(&self) -> CodecCaps {
        panic!("stub: the codec double is not filled yet")
    }

    /// Not filled yet.
    fn decoder(&self, _params: &CodecParams) -> Result<Box<dyn Decoder>, MediaError> {
        panic!("stub: the codec double is not filled yet")
    }

    /// Not filled yet.
    fn encoder(&self, _config: &EncoderConfig) -> Result<Box<dyn Encoder>, MediaError> {
        panic!("stub: the codec double is not filled yet")
    }
}
