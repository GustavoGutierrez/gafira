//! The minimal asset table the clips need (P9 of the design): one entry per
//! content hash, with the two fields the document needs to locate the bytes.
//!
//! The reference is the hash, so replacing a file on disk is a *new* asset and
//! every existing clip keeps playing the old content; relinking a *linked*
//! asset rewrites its storage path and no clip or timing changes. The import
//! metadata (duration, rate, legal class) arrives with the import feature, not
//! here.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ContentHash;

/// The reference a clip names its media with: the hash of the content, so reloca-
/// ting it or replacing it does not move any clip (§3.1, P9).
///
/// The wire form is the 64-character hex hash, the same spelling `assets/<blake3-hex>`
/// uses (§7.1); the persistence lands with the tables change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetRef(ContentHash);

impl AssetRef {
    /// Wraps a content hash as a clip's media reference.
    pub const fn new(hash: ContentHash) -> Self {
        Self(hash)
    }

    /// The content hash the reference names.
    pub const fn as_hash(self) -> ContentHash {
        self.0
    }

    /// Reads a reference from the 64-character hex wire form (§7.1).
    pub fn from_hex(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        if bytes.len() != 64 {
            return None;
        }
        let mut raw = [0u8; 32];
        for (slot, pair) in raw.iter_mut().zip(bytes.chunks_exact(2)) {
            let hi = hex_digit(pair[0])?;
            let lo = hex_digit(pair[1])?;
            *slot = hi << 4 | lo;
        }
        Some(Self::new(ContentHash(raw)))
    }

    /// The 64-character hex wire form (§7.1).
    pub fn hex(self) -> String {
        let mut out = String::with_capacity(64);
        for byte in self.0.0 {
            out.push(std::char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'));
            out.push(std::char::from_digit(u32::from(byte & 0x0F), 16).unwrap_or('0'));
        }
        out
    }
}

/// One hex digit, uppercase or lowercase, or `None`.
fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

impl fmt::Display for AssetRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.hex())
    }
}

impl Serialize for AssetRef {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.hex())
    }
}

impl<'de> Deserialize<'de> for AssetRef {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::from_hex(&text)
            .ok_or_else(|| serde::de::Error::custom("a reference needs 64 hex characters"))
    }
}

/// An entry of the asset table: what the document needs to locate the bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetEntry {
    /// The file or clip name the import wrote down.
    pub name: String,
    /// How the bytes are reached.
    pub storage: AssetStorage,
}

/// How the bytes of an asset are reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetStorage {
    /// The bytes live inside the saved document.
    Embedded,
    /// The bytes live outside the document at a path the document records apart.
    Linked,
}

/// The asset table of §3.1, keyed by [`AssetRef`] so two entries with the same
/// content cannot exist and ordering is byte-exact (`BTreeMap` per ARQ-01).
pub type AssetTable = BTreeMap<AssetRef, AssetEntry>;
