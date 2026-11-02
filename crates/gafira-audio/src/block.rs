//! The planar audio block and the allocation-free parameter snapshot.

/// How many `(id, value)` pairs a [`ParamSnapshot`] holds.
pub const MAX_PARAMS: usize = 16;

/// A planar block: one slice per channel.
///
/// The slices are expected to have equal lengths. A block whose slices differ
/// is not processed: `process` only cleans it (ARQ-123).
#[derive(Debug)]
pub struct AudioBlock<'a> {
    /// One slice per channel.
    pub channels: &'a mut [&'a mut [f32]],
}

impl<'a> AudioBlock<'a> {
    /// Plain data: nothing is validated.
    #[must_use]
    pub fn new(channels: &'a mut [&'a mut [f32]]) -> Self {
        Self { channels }
    }

    /// Frames per channel: the shortest channel, 0 with no channel.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.channels.iter().map(|c| c.len()).min().unwrap_or(0)
    }

    /// True when every channel has the same length.
    #[must_use]
    pub fn is_uniform(&self) -> bool {
        let frames = self.frames();
        self.channels.iter().all(|c| c.len() == frames)
    }
}

/// Up to [`MAX_PARAMS`] `(id, value)` pairs, `Copy`, so reading it never
/// allocates (ARQ-23). Ids are the `ParamDef` ids of the processor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParamSnapshot {
    entries: [(&'static str, f64); MAX_PARAMS],
    len: usize,
}

impl Default for ParamSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

impl ParamSnapshot {
    /// An empty snapshot.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: [("", 0.0); MAX_PARAMS],
            len: 0,
        }
    }

    /// Sets `id`, replacing an earlier value of the same id. Returns `false`,
    /// leaving the snapshot unchanged, when a new id does not fit.
    pub fn set(&mut self, id: &'static str, value: f64) -> bool {
        for entry in &mut self.entries[..self.len] {
            if entry.0 == id {
                entry.1 = value;
                return true;
            }
        }
        if self.len == MAX_PARAMS {
            return false;
        }
        self.entries[self.len] = (id, value);
        self.len += 1;
        true
    }

    /// The raw value of `id`: not validated, it may be NaN or infinite.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<f64> {
        self.entries[..self.len]
            .iter()
            .find(|(k, _)| *k == id)
            .map(|(_, v)| *v)
    }

    /// Number of pairs.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// True with no pair.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}
