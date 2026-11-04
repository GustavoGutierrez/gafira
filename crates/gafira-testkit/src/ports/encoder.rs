//! The fake encoder of ARQ-112: the port double a test injects in place of
//! a real encoder (02 §4.8).

use gafira_ports::media::{Encoder, Frame, MediaError, Packet, StreamIdx};

/// The fake encoder: the empty encoder, which accepts the frames and yields
/// no packet.
///
/// The encoder that yields a packet per frame is [`PacketEncoder`].
pub struct FakeEncoder;

impl Encoder for FakeEncoder {
    /// The empty encoder takes the frame and keeps nothing.
    fn send(&mut self, _frame: Option<&Frame>) -> Result<(), MediaError> {
        Ok(())
    }

    /// The empty encoder has no packet to give.
    fn receive(&mut self) -> Result<Option<Packet>, MediaError> {
        Ok(None)
    }
}

/// The encoder that turns every frame into one packet, at the time of the
/// frame.
#[derive(Clone, Debug, Default)]
pub struct PacketEncoder {
    pending: Option<Packet>,
}

impl PacketEncoder {
    /// An encoder with nothing pending.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Encoder for PacketEncoder {
    /// Keeps the packet of the frame.
    fn send(&mut self, frame: Option<&Frame>) -> Result<(), MediaError> {
        self.pending = frame.map(|frame| {
            let pts = match frame {
                Frame::Video(video) => video.pts,
                Frame::Audio(audio) => audio.pts(),
            };
            Packet {
                stream: StreamIdx(0),
                pts,
                dts: pts,
                keyframe: true,
                data: vec![0; 4],
            }
        });
        Ok(())
    }

    /// The packet of the last frame sent.
    fn receive(&mut self) -> Result<Option<Packet>, MediaError> {
        Ok(self.pending.take())
    }
}
