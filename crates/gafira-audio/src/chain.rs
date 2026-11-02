//! The track chain: processors applied in order (ARQ-129).

use gafira_doc::contract::{Describe, Descriptor};
use gafira_doc::msg::MessageKey;

use crate::dsp::clean_block;
use crate::{AudioBlock, AudioProcessor, ParamSnapshot};

static DESCRIPTOR: Descriptor = Descriptor::new("audio.chain", MessageKey("audio.chain"), 1);

/// Processors applied one after the other; every one reads the same
/// [`ParamSnapshot`]. Its latency is the sum of theirs.
#[derive(Default)]
pub struct ProcessorChain {
    processors: Vec<Box<dyn AudioProcessor>>,
}

impl ProcessorChain {
    /// An empty chain: it changes nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a processor at the end of the chain.
    pub fn push(&mut self, processor: Box<dyn AudioProcessor>) {
        self.processors.push(processor);
    }

    /// Number of processors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.processors.len()
    }

    /// True with no processor.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.processors.is_empty()
    }
}

impl Describe for ProcessorChain {
    fn descriptor(&self) -> &Descriptor {
        &DESCRIPTOR
    }
}

impl AudioProcessor for ProcessorChain {
    fn prepare(&mut self, sample_rate: u32, max_block: usize, channels: u16) {
        for processor in &mut self.processors {
            processor.prepare(sample_rate, max_block, channels);
        }
    }

    fn process(&mut self, block: &mut AudioBlock<'_>, params: &ParamSnapshot) {
        if self.processors.is_empty() {
            // An empty chain is still a processor: ARQ-123 holds for it.
            clean_block(block);
        }
        for processor in &mut self.processors {
            processor.process(block, params);
        }
    }

    fn latency(&self) -> u32 {
        self.processors
            .iter()
            .fold(0, |sum, p| sum.saturating_add(p.latency()))
    }

    fn tail(&self) -> u32 {
        self.processors
            .iter()
            .fold(0, |sum, p| sum.saturating_add(p.tail()))
    }

    fn reset(&mut self) {
        for processor in &mut self.processors {
            processor.reset();
        }
    }
}
