//! `ProjectRepository` and `RepositoryError` (design P4, P6, P8).

use std::path::Path;
use std::sync::{Arc, Mutex};

use gafira_doc::Document;
use gafira_doc::PageId;
use gafira_doc::msg::{MessageKey, MsgArg, ToMessage, is_valid_key};
use gafira_ports::common::UnixTime;
use gafira_ports::repository::{
    LoadedProject, ProjectRepository, RepositoryError, THUMBNAIL_SIDE, Thumbnail, Version,
    VersionId, VersionInfo, VersionKind,
};
use gafira_raster::{ImageBuf, PixelBuf};

/// An in-memory double that keeps one document and its versions.
struct Memory(Mutex<Option<(Document, Vec<Version>)>>);

impl ProjectRepository for Memory {
    fn load(&self, _path: &Path) -> Result<LoadedProject, RepositoryError> {
        match &*self.0.lock().unwrap() {
            Some((document, versions)) => Ok(LoadedProject {
                document: document.clone(),
                read_only: false,
                versions: versions.clone(),
            }),
            None => Err(RepositoryError::Corrupt {
                detail: "empty".into(),
            }),
        }
    }
    fn save(
        &self,
        _path: &Path,
        document: &Document,
        versions: &[Version],
    ) -> Result<(), RepositoryError> {
        *self.0.lock().unwrap() = Some((document.clone(), versions.to_vec()));
        Ok(())
    }
}

// FD-64 / S15: the port is object-safe and works on `Document`.
#[test]
fn p4_repository_is_object_safe_and_round_trips_a_document() {
    let repo: Box<dyn ProjectRepository> = Box::new(Memory(Mutex::new(None)));
    let path = Path::new("a.gaf");
    assert!(matches!(
        repo.load(path),
        Err(RepositoryError::Corrupt { .. })
    ));
    let doc = Document::default();
    repo.save(path, &doc, &[]).unwrap();
    let loaded = repo.load(path).unwrap();
    assert_eq!(loaded.document, doc);
    assert!(!loaded.read_only);
    assert!(loaded.versions.is_empty());
}

// S31-01: the port carries the versions a file keeps, in order.
#[test]
fn s31_01_a_save_hands_its_versions_back_on_load() {
    let repo: Box<dyn ProjectRepository> = Box::new(Memory(Mutex::new(None)));
    let path = Path::new("a.gaf");
    let version = |id: u64, kind: VersionKind| Version {
        info: VersionInfo {
            id: VersionId(id),
            name: format!("v{id}"),
            kind,
            created: UnixTime(1_700_000_000 + i64::from(id == 2)),
            app: "0.1.0".to_owned(),
            schema: Document::SCHEMA_VERSION,
        },
        document: Arc::new(Document::default()),
    };
    let versions = vec![
        version(2, VersionKind::Auto),
        version(1, VersionKind::Named),
    ];
    repo.save(path, &Document::default(), &versions).unwrap();
    assert_eq!(repo.load(path).unwrap().versions, versions);
}

// S33-01: a repository that keeps no thumbnail still saves through
// `save_with_thumbnail`, which falls back on `save`.
#[test]
fn s33_01_a_save_with_a_thumbnail_falls_back_on_save() {
    let repo: Box<dyn ProjectRepository> = Box::new(Memory(Mutex::new(None)));
    let path = Path::new("a.gaf");
    let thumbnail = Thumbnail {
        page: PageId::new(1),
        image: ImageBuf::new(2, 1, PixelBuf::Rgba8(vec![255; 8])).unwrap(),
    };
    let doc = Document::default();
    repo.save_with_thumbnail(path, &doc, &[], Some(&thumbnail))
        .unwrap();
    assert_eq!(repo.load(path).unwrap().document, doc);
    assert_eq!(THUMBNAIL_SIDE, 512, "02 §7.1: thumbnails of 512 px");
}

// ARQ-102 (ADR 0012): one fixture per variant.
#[test]
fn arq_102_repository_error_messages() {
    let corrupt = RepositoryError::Corrupt {
        detail: "secret detail".into(),
    }
    .to_message();
    let too_new = RepositoryError::TooNew {
        found: 7,
        supported: 3,
    }
    .to_message();
    let io = RepositoryError::Io(std::io::Error::other("secret detail")).to_message();
    for m in [&corrupt, &too_new, &io] {
        assert!(is_valid_key(m.key.0), "{:?}", m.key);
        assert!(m.key.0.starts_with("error.repository."), "{:?}", m.key);
    }
    assert_eq!(corrupt.key, MessageKey("error.repository.corrupt"));
    assert!(corrupt.args.is_empty(), "detail never becomes an argument");
    assert_eq!(too_new.key, MessageKey("error.repository.too_new"));
    assert_eq!(
        too_new.args,
        vec![("found", MsgArg::Count(7)), ("supported", MsgArg::Count(3))]
    );
    assert_eq!(io.key, MessageKey("error.repository.io"));
    assert!(io.args.is_empty());
}
