//! `ProjectRepository` and `RepositoryError` (design P4, P6, P8).

use std::path::Path;
use std::sync::Mutex;

use gafira_doc::Document;
use gafira_doc::msg::{MessageKey, MsgArg, ToMessage, is_valid_key};
use gafira_ports::repository::{LoadedProject, ProjectRepository, RepositoryError};

/// An in-memory double that keeps one document.
struct Memory(Mutex<Option<Document>>);

impl ProjectRepository for Memory {
    fn load(&self, _path: &Path) -> Result<LoadedProject, RepositoryError> {
        match &*self.0.lock().unwrap() {
            Some(document) => Ok(LoadedProject {
                document: document.clone(),
                read_only: false,
            }),
            None => Err(RepositoryError::Corrupt {
                detail: "empty".into(),
            }),
        }
    }
    fn save(&self, _path: &Path, document: &Document) -> Result<(), RepositoryError> {
        *self.0.lock().unwrap() = Some(document.clone());
        Ok(())
    }
}

// FD-64 / S15: the port is object-safe and works on `Document`.
#[test]
fn p4_repository_is_object_safe_and_round_trips_a_document() {
    let repo: Box<dyn ProjectRepository> = Box::new(Memory(Mutex::new(None)));
    let path = Path::new("a.gafira");
    assert!(matches!(
        repo.load(path),
        Err(RepositoryError::Corrupt { .. })
    ));
    let doc = Document::default();
    repo.save(path, &doc).unwrap();
    let loaded = repo.load(path).unwrap();
    assert_eq!(loaded.document, doc);
    assert!(!loaded.read_only);
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
