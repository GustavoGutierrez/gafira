//! The hash of a stored content (02 §5.3).

/// The 32-byte hash that names a content in the content store.
///
/// Plain data: it is persisted through `AssetRef`, so it lives here and not in
/// `gafira-ports` (ADR 0008 rule 1). The wire form arrives with `AssetTable`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentHash(pub [u8; 32]);
