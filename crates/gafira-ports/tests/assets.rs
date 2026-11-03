//! `AssetProvider`, `AssetRegistry` and `AssetError` (design P4, P6, P8).

use std::io::Cursor;
use std::sync::Arc;

use gafira_doc::ContentHash;
use gafira_doc::contract::{Describe, Descriptor};
use gafira_doc::msg::{MessageKey, MsgArg, ToMessage, is_valid_key};
use gafira_ports::assets::{
    AssetError, AssetItem, AssetKind, AssetPage, AssetProvider, AssetQuery, AssetRegistry,
    Materialized, ProviderAssetId,
};
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::ReadSeek;
use gafira_raster::ImageBuf;

struct Bundled {
    descriptor: Descriptor,
}

impl Bundled {
    fn arc(id: &'static str) -> Arc<dyn AssetProvider> {
        Arc::new(Self {
            descriptor: Descriptor::new(id, MessageKey("asset.test.bundled"), 1),
        })
    }
}

impl Describe for Bundled {
    fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }
}

impl AssetProvider for Bundled {
    fn kinds(&self) -> &[AssetKind] {
        &[AssetKind::Template]
    }
    fn search(&self, q: &AssetQuery) -> Result<AssetPage, AssetError> {
        let item = AssetItem {
            id: ProviderAssetId("t1".into()),
            kind: AssetKind::Template,
            name: q.text.clone().unwrap_or_default(),
            tags: vec![],
        };
        Ok(AssetPage {
            items: vec![item],
            total: 1,
        })
    }
    fn thumbnail(&self, id: &ProviderAssetId, _px: u32) -> Result<ImageBuf, AssetError> {
        Err(AssetError::NotFound { id: id.clone() })
    }
    fn materialize(
        &self,
        id: &ProviderAssetId,
        store: &dyn ContentStore,
    ) -> Result<Materialized, AssetError> {
        if id.0 != "t1" {
            return Err(AssetError::NotFound { id: id.clone() });
        }
        let hash = store
            .put(&mut Cursor::new(b"x".to_vec()))
            .map_err(AssetError::Store)?;
        Ok(Materialized {
            hash,
            mime: "image/png".into(),
        })
    }
}

struct Failing;

impl ContentStore for Failing {
    fn put(&self, _src: &mut dyn std::io::Read) -> Result<ContentHash, StoreError> {
        Err(StoreError::Io(std::io::Error::other("disk")))
    }
    fn open(&self, hash: ContentHash) -> Result<Box<dyn ReadSeek + Send>, StoreError> {
        Err(StoreError::NotFound { hash })
    }
    fn contains(&self, _hash: ContentHash) -> Result<bool, StoreError> {
        Ok(false)
    }
}

// ARQ-20: a new provider registers without touching the core, through `dyn`.
#[test]
fn arq_20_provider_registers_and_runs_through_dyn() {
    let mut registry = AssetRegistry::default();
    assert!(registry.is_empty());
    registry.register(Bundled::arc("bundled")).unwrap();
    assert!(registry.register(Bundled::arc("bundled")).is_err());
    let provider = registry.get("bundled").unwrap();
    assert_eq!(provider.kinds(), &[AssetKind::Template]);
    let query = AssetQuery {
        text: Some("cv".into()),
        ..AssetQuery::default()
    };
    let page = provider.search(&query).unwrap();
    assert_eq!((page.total, page.items[0].name.as_str()), (1, "cv"));
    let id = ProviderAssetId("t1".into());
    assert!(matches!(
        provider.thumbnail(&id, 64),
        Err(AssetError::NotFound { .. })
    ));
    assert!(matches!(
        provider.materialize(&id, &Failing),
        Err(AssetError::Store(StoreError::Io(_)))
    ));
}

// ARQ-102 (ADR 0012): one fixture per variant; `Store` delegates.
#[test]
fn arq_102_asset_error_messages() {
    let not_found = AssetError::NotFound {
        id: ProviderAssetId("secret/id".into()),
    }
    .to_message();
    let io = AssetError::Io(std::io::Error::other("secret detail")).to_message();
    let store = AssetError::Store(StoreError::NotFound {
        hash: ContentHash([1; 32]),
    });
    for m in [&not_found, &io] {
        assert!(is_valid_key(m.key.0), "{:?}", m.key);
        assert!(m.key.0.starts_with("error.asset."), "{:?}", m.key);
    }
    assert_eq!(not_found.key, MessageKey("error.asset.not_found"));
    assert_eq!(
        not_found.args,
        vec![("id", MsgArg::Text("secret/id".into()))]
    );
    assert_eq!(io.key, MessageKey("error.asset.io"));
    assert!(io.args.is_empty(), "library text never becomes an argument");
    let inner = StoreError::NotFound {
        hash: ContentHash([1; 32]),
    };
    assert_eq!(store.to_message(), inner.to_message());
}
