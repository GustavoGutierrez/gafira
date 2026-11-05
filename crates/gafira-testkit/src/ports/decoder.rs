//! The fake decoder of ARQ-112: the port double a test injects in place of
//! a real decoder (02 §4.8).

use gafira_ports::media::{Decoder, Frame, MediaError, Packet, VideoFrame};
use gafira_raster::{ImageBuf, PixelBuf};

/// The fake decoder: the empty decoder, which accepts the packets and
/// yields no frame.
///
/// The decoder that yields a frame per packet is [`FrameDecoder`].
pub struct FakeDecoder;

impl Decoder for FakeDecoder {
    /// The empty decoder takes the packet and keeps nothing.
    fn send(&mut self, _packet: Option<&Packet>) -> Result<(), MediaError> {
        Ok(())
    }

    /// The empty decoder has no frame to give.
    fn receive(&mut self) -> Result<Option<Frame>, MediaError> {
        Ok(None)
    }
}

/// The decoder that yields one video frame per packet, at the time of the
/// packet.
#[derive(Clone, Debug, Default)]
pub struct FrameDecoder {
    pending: Option<Frame>,
}

impl FrameDecoder {
    /// A decoder with nothing pending.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Decoder for FrameDecoder {
    /// Keeps the frame of the packet, if the packet is a video one.
    fn send(&mut self, packet: Option<&Packet>) -> Result<(), MediaError> {
        self.pending = packet.map(|packet| {
            Frame::Video(VideoFrame {
                pts: packet.pts,
                image: picture(),
            })
        });
        Ok(())
    }

    /// The frame of the last packet sent.
    fn receive(&mut self) -> Result<Option<Frame>, MediaError> {
        Ok(self.pending.take())
    }
}

/// The picture of every frame of the double: one opaque black pixel.
fn picture() -> ImageBuf {
    ImageBuf::new(1, 1, PixelBuf::Rgba8(vec![0, 0, 0, 255])).expect("one pixel fits its buffer")
}
