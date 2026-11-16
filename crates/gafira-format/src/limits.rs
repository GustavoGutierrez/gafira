//! Caps that make a hostile file fail early (008 §11, ARQ-56).

/// What a read accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Largest number of entries in the ZIP.
    pub max_entries: usize,
    /// Largest uncompressed JSON entry, in bytes.
    pub max_json_bytes: u64,
    /// Largest ratio of uncompressed to compressed bytes for an entry over
    /// 1 MiB; a higher one is a ZIP bomb.
    pub max_ratio: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_entries: 100_000,
            max_json_bytes: 256 * 1024 * 1024,
            max_ratio: 200,
        }
    }
}
