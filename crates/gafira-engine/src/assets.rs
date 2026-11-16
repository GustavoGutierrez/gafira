//! `AssetService` of 02 §5.4 line 1360 and spec 011: imports a file as an
//! embedded resource of the open document.
//!
//! The order is session, then file, then store, then command: without a
//! document nothing is read or stored (S11-05), and a failed port leaves the
//! document as it was (S11-06). The entry goes in through `InsertAsset` on the
//! shared command path, so the import is one undo entry (ARQ-30). Linked media
//! and `relocate` are Fase 2 (02 §7.1).

use std::path::Path;
use std::sync::Arc;

use gafira_doc::{AssetEntry, AssetRef, AssetStorage};
use gafira_edit::InsertAsset;
use gafira_ports::store::ContentStore;
use gafira_ports::system::{Clock, FileSystem};

use crate::edit;
use crate::engine::{EngineError, Ports};
use crate::session::Shared;

/// Imports the user's files into the open document (02 §5.4 line 1360).
pub struct AssetService {
    shared: Shared,
    filesystem: Arc<dyn FileSystem>,
    store: Arc<dyn ContentStore>,
    clock: Arc<dyn Clock>,
}

impl AssetService {
    pub(crate) fn over(shared: Shared, ports: &Ports) -> Self {
        Self {
            shared,
            filesystem: Arc::clone(&ports.filesystem),
            store: Arc::clone(&ports.store),
            clock: Arc::clone(&ports.clock),
        }
    }

    /// Copy the file at `path` into the content store and add it to the open
    /// document as an embedded asset named after the file. The returned
    /// reference is the content's hash, so importing the same content again
    /// returns the same reference and changes nothing (S11-03).
    ///
    /// # Errors
    /// [`EngineError::NoSession`] with no open document, before any port
    /// call; [`EngineError::Io`] when the file cannot be opened;
    /// [`EngineError::Store`] when the store fails. Nothing changes then.
    pub fn import(&self, path: &Path) -> Result<AssetRef, EngineError> {
        if self.shared.lock().is_none() {
            return Err(EngineError::NoSession);
        }
        let mut file = self.filesystem.open(path)?;
        let reference = AssetRef::new(self.store.put(&mut file)?);
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let entry = AssetEntry {
            name,
            storage: AssetStorage::Embedded,
        };
        edit::run(
            &self.shared,
            &*self.clock,
            Box::new(InsertAsset::new(reference, entry)),
        )?;
        Ok(reference)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use gafira_doc::{AssetEntry, AssetRef, AssetStorage};
    use gafira_ports::store::StoreError;

    use crate::Gafira;
    use crate::engine::EngineError;
    use crate::event::EngineEvent;
    use crate::fakes::{MemoryFs, MemoryStore, drain, engine_with_store};

    const PHOTO: &str = "/in/photo.png";
    const BYTES: &[u8] = b"\x89PNG not really";

    /// An engine with a new document (its event drained) over a file system
    /// holding [`PHOTO`].
    fn opened() -> (Gafira, MemoryFs, Arc<MemoryStore>) {
        let fs = MemoryFs::default();
        fs.put(PHOTO, BYTES);
        let store = Arc::new(MemoryStore::default());
        let engine = engine_with_store(&fs, &store);
        engine.projects().new_document().expect("a new document");
        drain(&engine);
        (engine, fs, store)
    }

    fn changes(events: &[EngineEvent]) -> usize {
        events
            .iter()
            .filter(|e| matches!(e, EngineEvent::DocumentChanged { .. }))
            .count()
    }

    fn assets_of(engine: &mut Gafira) -> Vec<(AssetRef, AssetEntry)> {
        let doc = engine.edit().document().expect("a session");
        doc.assets().iter().map(|(r, e)| (*r, e.clone())).collect()
    }

    #[test]
    fn s11_02_import_stores_the_bytes_and_adds_an_embedded_entry() {
        let (mut engine, _fs, store) = opened();

        let reference = engine.assets().import(Path::new(PHOTO)).expect("imports");

        assert_eq!(reference, AssetRef::new(MemoryStore::hash_of(BYTES)));
        assert_eq!(store.get(reference.as_hash()).as_deref(), Some(BYTES));
        assert_eq!(
            assets_of(&mut engine),
            [(
                reference,
                AssetEntry {
                    name: "photo.png".to_owned(),
                    storage: AssetStorage::Embedded,
                }
            )]
        );
        assert_eq!(changes(&drain(&engine)), 1);
        assert!(engine.projects().is_modified());
    }

    #[test]
    fn s11_03_the_same_content_twice_is_one_entry_and_one_event() {
        let (mut engine, fs, _store) = opened();
        fs.put("/in/copy.png", BYTES);

        let first = engine.assets().import(Path::new(PHOTO)).expect("imports");
        drain(&engine);
        let second = engine
            .assets()
            .import(Path::new("/in/copy.png"))
            .expect("imports");

        assert_eq!(first, second);
        let assets = assets_of(&mut engine);
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].1.name, "photo.png", "the first entry stays");
        assert!(
            drain(&engine).is_empty(),
            "nothing changed, nothing published"
        );
    }

    #[test]
    fn s11_04_undo_removes_the_import_and_redo_restores_it() {
        let (mut engine, _fs, _store) = opened();
        let reference = engine.assets().import(Path::new(PHOTO)).expect("imports");
        drain(&engine);

        engine.edit().undo().expect("undoes");
        assert!(assets_of(&mut engine).is_empty());
        assert_eq!(changes(&drain(&engine)), 1);

        engine.edit().redo().expect("redoes");
        assert_eq!(assets_of(&mut engine)[0].0, reference);
    }

    #[test]
    fn s11_05_without_a_document_nothing_is_read_or_stored() {
        let fs = MemoryFs::default();
        let store = Arc::new(MemoryStore::default());
        let engine = engine_with_store(&fs, &store);

        // The file does not exist: an `Io` error would mean the engine read
        // before it checked the session.
        let result = engine.assets().import(Path::new("/missing.png"));

        assert!(matches!(result, Err(EngineError::NoSession)));
        assert_eq!(store.len(), 0);
        assert!(drain(&engine).is_empty());
    }

    #[test]
    fn s11_06_a_failed_port_changes_nothing() {
        let (mut engine, _fs, store) = opened();

        let missing = engine.assets().import(Path::new("/in/missing.png"));
        assert!(matches!(missing, Err(EngineError::Io(_))));

        store.fail();
        let stored = engine.assets().import(Path::new(PHOTO));
        assert!(matches!(stored, Err(EngineError::Store(StoreError::Io(_)))));

        assert!(assets_of(&mut engine).is_empty());
        assert!(!engine.projects().is_modified());
        assert!(drain(&engine).is_empty());
    }
}
