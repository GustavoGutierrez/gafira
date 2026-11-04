//! The fake container of ARQ-112: the port double a test injects in place of
//! a real container (02 §4.8).

use gafira_doc::contract::{Describe, Descriptor};
use gafira_doc::msg::MessageKey;
use gafira_ports::common::Confidence;
use gafira_ports::media::{Container, Demuxer, MediaError, MuxConfig, Muxer};
use gafira_ports::system::{ReadSeek, WriteSeek};

/// The descriptor of the fake container.
static DESCRIPTOR: Descriptor = Descriptor::new("test.container", MessageKey("test.container"), 1);

/// The fake container. A stub: the media stream fills it.
pub struct FakeContainer;

impl Describe for FakeContainer {
    /// The descriptor of the double.
    fn descriptor(&self) -> &Descriptor {
        &DESCRIPTOR
    }
}

impl Container for FakeContainer {
    /// Not filled yet.
    fn probe(&self, _head: &[u8]) -> Confidence {
        panic!("stub: the container double is not filled yet")
    }

    /// Not filled yet.
    fn open(&self, _src: Box<dyn ReadSeek + Send>) -> Result<Box<dyn Demuxer>, MediaError> {
        panic!("stub: the container double is not filled yet")
    }

    /// Not filled yet.
    fn create(
        &self,
        _dst: Box<dyn WriteSeek + Send>,
        _config: &MuxConfig,
    ) -> Result<Box<dyn Muxer>, MediaError> {
        panic!("stub: the container double is not filled yet")
    }
}
