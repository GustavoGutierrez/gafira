//! `ProjectService`: the session's lifecycle (02 §5.4 line 1357).
//!
//! `new_document` (the `new` of §5.4), `open` and `is_modified`. Every file
//! operation goes through the injected ports; the engine opens nothing itself
//! (P1, design P18).

use std::path::Path;
use std::sync::Arc;

use gafira_doc::{Document, Dpi};
use gafira_edit::{DirtySet, History};
use gafira_ports::repository::ProjectRepository;

use crate::engine::{EngineError, Ports};
use crate::session::{Session, Shared};

/// Creates, opens and saves the session's document (02 §5.4 line 1357).
pub struct ProjectService {
    shared: Shared,
    repository: Arc<dyn ProjectRepository>,
}

impl ProjectService {
    pub(crate) fn over(shared: Shared, ports: &Ports) -> Self {
        Self {
            shared,
            repository: Arc::clone(&ports.repository),
        }
    }

    /// Replace the session with an empty document and a fresh history.
    ///
    /// The old history is dropped: nothing made before `new` can be undone
    /// after it. The engine does not ask first; the UI does, with
    /// [`ProjectService::is_modified`] (design P15). Bumps the version and
    /// publishes a `DocumentChanged` that lists every page and node of the new
    /// document, so an observer drops what it holds for any other page.
    ///
    /// This is the `new` of 02 §5.4 line 1357; the name differs because
    /// clippy's `new_ret_no_self` reserves `new` for constructors.
    pub fn new_document(&self) -> Result<(), EngineError> {
        let document = Document::from_parts(1, Dpi::DEFAULT, Vec::new())?;
        self.install(document)
    }

    /// Replace the session with the project at `path` (02 §5.4 line 1357).
    ///
    /// The document is loaded and validated before anything changes, so a
    /// failed `open` leaves the old session, its version and its history as
    /// they were and publishes nothing (design P15, P18). On success the old
    /// history is dropped, the version bumps once and one `DocumentChanged`
    /// lists every page and node of the loaded document.
    pub fn open(&self, path: &Path) -> Result<(), EngineError> {
        let loaded = self.repository.load(path)?;
        self.install(loaded.document)
    }

    /// Validate `document` into a history, then swap the session for it.
    fn install(&self, document: Document) -> Result<(), EngineError> {
        let dirty = every_page_and_node(&document);
        let history = History::new(document)?;
        let mut cell = self.shared.lock();
        let clean_version = self.shared.changed(dirty);
        *cell = Some(Session {
            history,
            clean_version,
        });
        Ok(())
    }

    /// Whether the session holds changes no `new_document`, `open` or `save`
    /// has accounted for: its version differs from its clean version. Undoing back
    /// to the clean content still counts as modified (conservative, P15).
    /// `false` with no session: there is nothing to lose.
    #[must_use]
    pub fn is_modified(&self) -> bool {
        self.shared
            .lock()
            .as_ref()
            .is_some_and(|session| session.clean_version != self.shared.version())
    }
}

/// Every page and every node of `document`.
fn every_page_and_node(document: &Document) -> DirtySet {
    DirtySet {
        pages: document.pages().iter().map(|page| page.id).collect(),
        nodes: document
            .pages()
            .iter()
            .flat_map(|page| page.nodes.keys().copied())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use gafira_doc::{NodeId, PageId};
    use gafira_edit::DirtySet;
    use gafira_ports::repository::RepositoryError;

    use crate::engine::{EngineError, Gafira};
    use crate::event::EngineEvent;
    use crate::fakes::*;
    use crate::session::DocVersion;

    const TARGET: &str = "/work/poster.gafira";

    fn changed(version: u64, dirty: DirtySet) -> EngineEvent {
        EngineEvent::DocumentChanged {
            version: DocVersion(version),
            dirty,
        }
    }

    /// An engine over a memory file system holding [`two_page_document`] at
    /// [`TARGET`], loads marked `read_only`, nothing opened yet.
    fn engine_with_target(read_only: bool) -> (Gafira, MemoryFs, Arc<MemoryRepository>) {
        let fs = MemoryFs::default();
        let json = two_page_document().to_json().expect("json");
        fs.put(TARGET, json.as_bytes());
        let repository = Arc::new(MemoryRepository::new(&fs, read_only));
        (engine_on(&fs, &repository), fs, repository)
    }

    /// After `new` (version 1) and one appended page (version 2), events
    /// drained: modified, one undo entry.
    fn edited_new_document(engine: &mut Gafira) -> PageId {
        engine.projects().new_document().expect("a new document");
        let page = append_page(engine);
        assert_eq!(drain(engine).len(), 2, "the new and the append");
        assert!(engine.projects().is_modified(), "the append");
        page
    }

    /// AC8: what a failed `open` must leave alone, after `edited_new_document`.
    fn assert_old_session_kept(engine: &mut Gafira, page: PageId) {
        assert_eq!(drain(engine), Vec::new(), "a failed open publishes nothing");
        assert!(engine.projects().is_modified(), "still modified");
        let page_count =
            |engine: &mut Gafira| engine.edit().document().expect("a session").pages().len();
        assert_eq!(page_count(engine), 1, "the old document");
        assert!(
            engine.edit().undo().expect("an undo").is_some(),
            "the old history"
        );
        assert_eq!(page_count(engine), 0, "undone to the new document");
        assert_eq!(
            drain(engine),
            vec![changed(3, pages(&[page]))],
            "the version did not move"
        );
    }

    fn pages_of(ids: &[u64]) -> DirtySet {
        pages(&ids.iter().copied().map(PageId::new).collect::<Vec<_>>())
    }

    // ARQ-30
    #[test]
    fn ac1_ac4_new_bumps_once_and_lists_the_whole_empty_document() {
        let engine = engine_over(quiet_channel());
        engine.projects().new_document().expect("a new document");
        assert_eq!(
            drain(&engine),
            vec![EngineEvent::DocumentChanged {
                version: DocVersion(1),
                dirty: DirtySet::default(),
            }]
        );
    }

    // ARQ-30
    #[test]
    fn ac8_modified_after_an_edit_and_after_undoing_it() {
        let mut engine = engine_over(quiet_channel());
        assert!(!engine.projects().is_modified(), "no session");
        engine.projects().new_document().expect("a new document");
        assert!(!engine.projects().is_modified(), "after new");

        append_page(&mut engine);
        assert!(engine.projects().is_modified(), "after an edit");

        assert!(engine.edit().undo().expect("an undo").is_some());
        assert!(engine.projects().is_modified(), "after undoing back");

        engine.projects().new_document().expect("a second new");
        assert!(!engine.projects().is_modified(), "after a second new");
    }

    // ARQ-30
    #[test]
    fn ac9_open_bumps_once_lists_the_whole_loaded_document_and_drops_the_old_history() {
        let (mut engine, _fs, repository) = engine_with_target(false);
        edited_new_document(&mut engine);

        engine.projects().open(Path::new(TARGET)).expect("an open");

        assert_eq!(repository.take_calls(), vec![RepoCall::Load(TARGET.into())]);
        let mut dirty = pages_of(&[1, 4]);
        dirty.nodes = [2, 3, 5].into_iter().map(NodeId::new).collect();
        assert_eq!(drain(&engine), vec![changed(3, dirty)]);
        assert!(!engine.projects().is_modified(), "clean after open");
        assert_eq!(
            *engine.edit().document().expect("a session"),
            two_page_document()
        );
        assert!(
            engine.edit().undo().expect("an undo").is_none(),
            "no undo across open"
        );
    }

    #[test]
    fn ac8_an_open_that_fails_to_load_leaves_the_old_session() {
        let (mut engine, _fs, _repository) = engine_with_target(false);
        let page = edited_new_document(&mut engine);

        let error = engine.projects().open(Path::new("/work/missing.gafira"));

        assert!(
            matches!(error, Err(EngineError::Repository(RepositoryError::Io(_)))),
            "{error:?}"
        );
        assert_old_session_kept(&mut engine, page);
    }

    #[test]
    fn ac8_an_open_of_an_invalid_document_leaves_the_old_session() {
        let (mut engine, _fs, repository) = engine_with_target(false);
        let page = edited_new_document(&mut engine);
        repository.hand_over(invalid_document());

        let error = engine.projects().open(Path::new(TARGET));

        assert!(
            matches!(error, Err(EngineError::InvalidDocument(_))),
            "{error:?}"
        );
        assert_old_session_kept(&mut engine, page);
    }
}
