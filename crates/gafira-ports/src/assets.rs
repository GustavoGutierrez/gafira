//! Asset providers (02 §4.11).

use std::fmt;
use std::io;

use gafira_color::Color;
use gafira_doc::ContentHash;
use gafira_doc::contract::Describe;
use gafira_doc::msg;
use gafira_doc::msg::{Message, ToMessage};
use gafira_raster::ImageBuf;

use crate::registry::Registry;
use crate::store::{ContentStore, StoreError};

/// What an asset is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AssetKind {
    Template,
    Element,
    Photo,
    Video,
    Audio,
    Font,
    Icon,
    AnimPreset,
}

/// The id of an asset inside one provider.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProviderAssetId(pub String);

/// A search. Empty fields match everything.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AssetQuery {
    /// Free text.
    pub text: Option<String>,
    /// Kinds to keep; empty keeps all.
    pub kinds: Vec<AssetKind>,
    /// Tags the asset must carry.
    pub tags: Vec<String>,
    /// A dominant colour to match.
    pub color: Option<Color>,
    /// A file format to match.
    pub format: Option<String>,
    /// Results to skip.
    pub offset: usize,
    /// Most results to return; 0 means no limit.
    pub limit: usize,
}

/// One search result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetItem {
    /// Its id in the provider.
    pub id: ProviderAssetId,
    /// What it is.
    pub kind: AssetKind,
    /// Display name.
    pub name: String,
    /// Its tags.
    pub tags: Vec<String>,
}

/// One page of results.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssetPage {
    /// The items of this page.
    pub items: Vec<AssetItem>,
    /// Matches in all pages.
    pub total: usize,
}

/// An asset copied into a [`ContentStore`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Materialized {
    /// Its hash in the store.
    pub hash: ContentHash,
    /// Its media type.
    pub mime: String,
}

/// Why an [`AssetProvider`] call failed.
#[derive(Debug)]
pub enum AssetError {
    /// The provider has no asset with this id.
    NotFound { id: ProviderAssetId },
    /// The content store failed.
    Store(StoreError),
    /// The provider's storage failed.
    Io(io::Error),
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { id } => write!(f, "asset `{}` not found", id.0),
            Self::Store(e) => write!(f, "asset store error: {e}"),
            Self::Io(e) => write!(f, "asset I/O error: {e}"),
        }
    }
}

impl std::error::Error for AssetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NotFound { .. } => None,
            Self::Store(e) => Some(e),
            Self::Io(e) => Some(e),
        }
    }
}

impl ToMessage for AssetError {
    fn to_message(&self) -> Message {
        match self {
            Self::NotFound { id } => msg!("error.asset.not_found", id = Text(id.0.clone())),
            Self::Store(e) => e.to_message(),
            Self::Io(_) => msg!("error.asset.io"),
        }
    }
}

/// A source of templates and library assets. Adapters implement it
/// (`gafira-assets`).
pub trait AssetProvider: Describe + Send + Sync {
    /// The kinds this provider serves.
    fn kinds(&self) -> &[AssetKind];

    /// Finds assets.
    ///
    /// # Errors
    /// [`AssetError::Io`] when the provider's index cannot be read.
    fn search(&self, q: &AssetQuery) -> Result<AssetPage, AssetError>;

    /// A preview of at most `px` pixels on its longest side.
    ///
    /// # Errors
    /// [`AssetError::NotFound`] for an unknown id.
    fn thumbnail(&self, id: &ProviderAssetId, px: u32) -> Result<ImageBuf, AssetError>;

    /// Copies the asset into `store`.
    ///
    /// # Errors
    /// [`AssetError::NotFound`] for an unknown id, [`AssetError::Store`]
    /// when `store` fails.
    fn materialize(
        &self,
        id: &ProviderAssetId,
        store: &dyn ContentStore,
    ) -> Result<Materialized, AssetError>;
}

/// The providers of a build, in registration order.
pub type AssetRegistry = Registry<dyn AssetProvider>;
