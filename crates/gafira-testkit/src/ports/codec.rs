//! The fake codec of ARQ-112: the port double a test injects in place of a
//! real codec (02 §4.8).

use gafira_doc::contract::{Describe, Descriptor};
use gafira_doc::msg::MessageKey;
use gafira_ports::common::LegalClass;
use gafira_ports::media::{
    Codec, CodecCaps, CodecId, CodecParams, Decoder, Encoder, EncoderConfig, MediaError,
};

use super::decoder::FrameDecoder;
use super::encoder::PacketEncoder;

/// The descriptor of the fake codec.
static DESCRIPTOR: Descriptor = Descriptor::new("test.codec", MessageKey("test.codec"), 1);

/// The fake codec: it hands out the decoder that yields a frame per packet
/// and the encoder that yields a packet per frame.
pub struct FakeCodec;

impl Describe for FakeCodec {
    /// The descriptor of the double.
    fn descriptor(&self) -> &Descriptor {
        &DESCRIPTOR
    }
}

impl Codec for FakeCodec {
    /// The codec the double stands for.
    fn codec_id(&self) -> CodecId {
        CodecId::H264
    }

    /// The double decodes and encodes, without hardware.
    fn caps(&self) -> CodecCaps {
        CodecCaps {
            decode: true,
            encode: true,
            hardware: false,
            legal: LegalClass::RoyaltyFree,
        }
    }

    /// The decoder of the double.
    fn decoder(&self, _params: &CodecParams) -> Result<Box<dyn Decoder>, MediaError> {
        Ok(Box::new(FrameDecoder::new()))
    }

    /// The encoder of the double.
    fn encoder(&self, _config: &EncoderConfig) -> Result<Box<dyn Encoder>, MediaError> {
        Ok(Box::new(PacketEncoder::new()))
    }
}
