//! `ProjectService`: the session's lifecycle (02 §5.4 line 1357).
//!
//! `new_document` (the `new` of §5.4), `open`, `save`, `save_as` and
//! `save_copy` (spec 012), `is_modified` and the autosave schedule
//! (`autosave_due`, `autosave_finished`, design P13). Every save keeps one
//! `.gaf.bak` generation (02 §7.2 step 4) and every session carries the
//! project id of D-63.
//! Every file operation goes through the injected ports; the engine opens
//! nothing itself (P1, design P18).

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use gafira_doc::{Document, Dpi};
use gafira_edit::{DirtySet, History};
use gafira_ports::repository::ProjectRepository;
use gafira_ports::system::{Clock, FileSystem, IdSource};

use crate::autosave::{AutosaveJob, AutosaveOutcome, SLOTS};
use crate::engine::{EngineConfig, EngineError, Ports};
use crate::session::{Autosave, DocVersion, Session, Shared};

/// Creates, opens and saves the session's document (02 §5.4 line 1357).
pub struct ProjectService {
    pub(crate) shared: Shared,
    repository: Arc<dyn ProjectRepository>,
    filesystem: Arc<dyn FileSystem>,
    clock: Arc<dyn Clock>,
    /// Mints the identity of a new project and of a copy (D-63).
    ids: Arc<dyn IdSource>,
    /// How long a change waits before it is autosaved (P16).
    autosave_interval: Duration,
    /// `data_dir/recovery`: one folder per project id (P13, D-63).
    recovery: PathBuf,
    /// Numbers the sessions of this engine, for their project keys (P13).
    sessions: AtomicU64,
    /// Numbers the temporary files of every save, so no two share a name
    /// (P18).
    temporaries: AtomicU64,
}

impl ProjectService {
    pub(crate) fn over(shared: Shared, ports: &Ports, config: &EngineConfig) -> Self {
        Self {
            shared,
            repository: Arc::clone(&ports.repository),
            filesystem: Arc::clone(&ports.filesystem),
            clock: Arc::clone(&ports.clock),
            ids: Arc::clone(&ports.ids),
            autosave_interval: config.autosave_interval(),
            recovery: config.data_dir().join("recovery"),
            sessions: AtomicU64::new(0),
            temporaries: AtomicU64::new(0),
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
        self.install(document, None, false)
    }

    /// Replace the session with the project at `path` (02 §5.4 line 1357).
    ///
    /// The document is loaded and validated before anything changes, so a
    /// failed `open` leaves the old session, its version and its history as
    /// they were and publishes nothing (design P15, P18). On success the old
    /// history is dropped, the version bumps once and one `DocumentChanged`
    /// lists every page and node of the loaded document.
    ///
    /// A file of a newer schema opens read-only: it can be edited in memory,
    /// and [`ProjectService::save`] refuses it (ARQ-55).
    pub fn open(&self, path: &Path) -> Result<(), EngineError> {
        let loaded = self.repository.load(path)?;
        self.install(loaded.document, Some(path.to_owned()), loaded.read_only)
    }

    /// Write the session's document over its file, atomically (02 §7.2
    /// steps 1-4, ARQ-50).
    ///
    /// Refused with [`EngineError::NoSession`], then [`EngineError::ReadOnly`]
    /// (ARQ-55), then [`EngineError::NoPath`], all before any port call. The
    /// document and its version are taken under the lock; with the lock
    /// released, [`ProjectService::write_atomically`] writes it. On a failure
    /// the first error is returned and the session stays modified. On success
    /// the session is clean at the saved version, if it is still the session
    /// that was saved: an edit made meanwhile keeps it modified, and pending
    /// for autosave (P13). `save` never bumps the version.
    pub fn save(&self) -> Result<(), EngineError> {
        let (snapshot, version, key, path) = {
            let cell = self.shared.lock();
            let session = cell.as_ref().ok_or(EngineError::NoSession)?;
            if session.read_only {
                return Err(EngineError::ReadOnly);
            }
            let path = session.path.clone().ok_or(EngineError::NoPath)?;
            let (snapshot, version, key) = self.snapshot_of(session);
            (snapshot, version, key, path)
        };
        self.write_atomically(&path, &snapshot)?;
        self.saved(&key, version, None);
        Ok(())
    }

    /// Write the session's document at `path` and make `path` the session's
    /// file (02 §5.4 line 1358, spec 012).
    ///
    /// The way out of a new document and of a read-only session (ARQ-55): on
    /// success the session lives at `path`, may be saved over and is clean at
    /// the saved version, with the same project id (D-63). Same order and
    /// same failure rules as [`ProjectService::save`]; a failure changes
    /// nothing in the session.
    ///
    /// # Errors
    /// [`EngineError::NoSession`] before any port call; then the first error
    /// of the repository or the file system.
    pub fn save_as(&self, path: &Path) -> Result<(), EngineError> {
        let (snapshot, version, key) = {
            let cell = self.shared.lock();
            let session = cell.as_ref().ok_or(EngineError::NoSession)?;
            self.snapshot_of(session)
        };
        self.write_atomically(path, &snapshot)?;
        self.saved(&key, version, Some(path));
        Ok(())
    }

    /// Write a copy of the session's document at `path` under a new project
    /// id (D-63: «Guardar copia»), leaving the session as it was: its file,
    /// its id, its read-only flag and its modified state.
    ///
    /// # Errors
    /// [`EngineError::NoSession`] before any port call; then the first error
    /// of the repository or the file system.
    pub fn save_copy(&self, path: &Path) -> Result<(), EngineError> {
        let snapshot = {
            let cell = self.shared.lock();
            let session = cell.as_ref().ok_or(EngineError::NoSession)?;
            Arc::clone(session.history.document())
        };
        let copy = Document::clone(&snapshot).with_project_id(self.ids.project_id());
        self.write_atomically(path, &copy)
    }

    /// The document, the version and the key of `session`, taken under the
    /// caller's lock.
    fn snapshot_of(&self, session: &Session) -> (Arc<Document>, DocVersion, String) {
        (
            Arc::clone(session.history.document()),
            self.shared.version(),
            session.session_key.clone(),
        )
    }

    /// Account for a save of `version` of the session named `key`: clean at
    /// that version and, for `save_as`, living at `path` and writable. A
    /// session replaced meanwhile is left alone.
    fn saved(&self, key: &str, version: DocVersion, path: Option<&Path>) {
        let mut cell = self.shared.lock();
        let Some(session) = cell.as_mut().filter(|s| s.session_key == key) else {
            return;
        };
        if let Some(path) = path {
            session.path = Some(path.to_owned());
            session.read_only = false;
        }
        session.clean_version = version;
        // The file holds every change up to the snapshot; one made while it
        // was written stays pending for autosave.
        if self.shared.version() == version {
            session.autosave.pending_since = None;
        }
    }

    /// 02 §7.2: the repository writes and syncs `<path>.tmp-<n>` (steps 1-2,
    /// SP6); when `path` exists its bytes go to `<path>.bak` (step 4); then
    /// the file system renames the temporary over `path` and syncs the
    /// folder (step 3). Each step runs only after the one before returned
    /// `Ok`, so a failure anywhere leaves `path` with its old bytes. The
    /// temporaries are removed on a failure, on a best-effort basis: the
    /// first error is the one the caller needs.
    fn write_atomically(&self, path: &Path, document: &Document) -> Result<(), EngineError> {
        let n = self.temporaries.fetch_add(1, Ordering::SeqCst);
        let tmp = suffixed(path, &format!(".tmp-{n}"));
        let written = self
            .repository
            .save(&tmp, document)
            .map_err(EngineError::from)
            .and_then(|()| self.back_up(path, n))
            .and_then(|()| self.filesystem.replace(&tmp, path).map_err(EngineError::Io));
        if written.is_err() {
            let _ = self.filesystem.remove(&tmp);
        }
        written
    }

    /// Keep one generation of `path` as `<path>.bak` (02 §7.2 step 4): its
    /// bytes are copied to `<path>.bak.tmp-<n>`, synced, and renamed over the
    /// backup, so a failure never leaves a half backup either. Nothing to do
    /// when `path` does not exist yet.
    fn back_up(&self, path: &Path, n: u64) -> Result<(), EngineError> {
        if !self.filesystem.exists(path)? {
            return Ok(());
        }
        let backup = suffixed(path, ".bak");
        let tmp = suffixed(&backup, &format!(".tmp-{n}"));
        let copied = self
            .copy(path, &tmp)
            .and_then(|()| self.filesystem.replace(&tmp, &backup));
        if copied.is_err() {
            let _ = self.filesystem.remove(&tmp);
        }
        copied.map_err(EngineError::Io)
    }

    /// The bytes of `from` into a new, synced `to`.
    fn copy(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        let mut bytes = Vec::new();
        self.filesystem.open(from)?.read_to_end(&mut bytes)?;
        let mut file = self.filesystem.create(to)?;
        file.write_all(&bytes)?;
        file.sync_all()
    }

    /// Validate `document` into a history, then swap the session for it.
    fn install(
        &self,
        document: Document,
        path: Option<PathBuf>,
        read_only: bool,
    ) -> Result<(), EngineError> {
        // D-63: a new project, or a file written before the field, gets its
        // identity here, so every snapshot of the history carries it.
        let document = if document.project_id().is_nil() {
            document.with_project_id(self.ids.project_id())
        } else {
            document
        };
        let dirty = every_page_and_node(&document);
        let history = History::new(document)?;
        let session_key = format!(
            "{}-{}",
            self.clock.now().0,
            self.sessions.fetch_add(1, Ordering::SeqCst)
        );
        let mut cell = self.shared.lock();
        let clean_version = self.shared.changed(dirty);
        *cell = Some(Session {
            history,
            path,
            read_only,
            clean_version,
            session_key,
            autosave: Autosave::default(),
        });
        Ok(())
    }

    /// The job that writes the session's next recovery slot, when one is due
    /// (02 §7.2 "Autoguardado rotativo", design P13).
    ///
    /// `Some` exactly when a session is open, a change is pending, no job of
    /// it is in flight, and the clock's monotonic time is at least one
    /// interval past the first pending change. The job holds the document as
    /// it is now; taking it clears the pending change, so an edit made while
    /// it runs is pending again. The host pumps this; the engine has no timer.
    #[must_use]
    pub fn autosave_due(&self) -> Option<AutosaveJob> {
        let now = self.clock.monotonic();
        let mut cell = self.shared.lock();
        let session = cell.as_mut()?;
        let since = session.autosave.pending_since?;
        if session.autosave.in_flight || now.saturating_sub(since) < self.autosave_interval {
            return None;
        }
        session.autosave.pending_since = None;
        session.autosave.in_flight = true;
        Some(AutosaveJob {
            snapshot: Arc::clone(session.history.document()),
            version: self.shared.version(),
            slot: session.autosave.slot,
            folder: self
                .recovery
                .join(session.history.document().project_id().to_string()),
            session_key: session.session_key.clone(),
            repository: Arc::clone(&self.repository),
            filesystem: Arc::clone(&self.filesystem),
        })
    }

    /// Account for a job the host ran (design P13).
    ///
    /// A success moves the next job to the following slot (`0, 1, 2, 0`); a
    /// failure keeps the slot and makes the change pending from now, so a
    /// failing disk is retried once per interval, never once per tick. Either
    /// way the session may be handed its next job. An outcome of a session
    /// `new_document` or `open` replaced since is ignored. Neither clears
    /// "modified" nor bumps the version: the user's file did not change.
    pub fn autosave_finished(&self, outcome: AutosaveOutcome) {
        let now = self.clock.monotonic();
        let mut cell = self.shared.lock();
        let Some(session) = cell
            .as_mut()
            .filter(|session| session.session_key == outcome.session_key)
        else {
            return;
        };
        let autosave = &mut session.autosave;
        autosave.in_flight = false;
        match outcome.result {
            Ok(()) => autosave.slot = (autosave.slot + 1) % SLOTS,
            Err(_) => autosave.pending_since = Some(now),
        }
    }

    /// The file the session lives in: the one `open` read or `save_as`
    /// wrote; `None` with no session or for a document never saved. The UI
    /// names the project with it and picks between `save` and `save_as`.
    #[must_use]
    pub fn path(&self) -> Option<PathBuf> {
        self.shared
            .lock()
            .as_ref()
            .and_then(|session| session.path.clone())
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

/// `path` with `suffix` appended to its last component, in the same folder.
fn suffixed(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
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
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::Duration;

    use gafira_doc::{Document, NodeId, PageId, ProjectId};
    use gafira_edit::{Applied, DirtySet};
    use gafira_ports::repository::RepositoryError;

    use crate::edit::EditService;
    use crate::engine::{EngineError, Gafira};
    use crate::event::EngineEvent;
    use crate::fakes::*;
    use crate::session::{DocVersion, Session};

    const TARGET: &str = "/work/poster.gaf";
    const TMP0: &str = "/work/poster.gaf.tmp-0";
    const TMP1: &str = "/work/poster.gaf.tmp-1";
    const BAK: &str = "/work/poster.gaf.bak";
    const BAK_TMP0: &str = "/work/poster.gaf.bak.tmp-0";

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
            two_page_document().with_project_id(SequentialIds::nth(2)),
            "the file had no id, so the open minted the second one"
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

        let error = engine.projects().open(Path::new("/work/missing.gaf"));

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

    /// The bytes [`TARGET`] holds before any save.
    fn old_bytes() -> Vec<u8> {
        two_page_document().to_json().expect("json").into_bytes()
    }

    /// The bytes a save of the open document writes.
    fn new_bytes(engine: &mut Gafira) -> Vec<u8> {
        let document = engine.edit().document().expect("a session");
        document.to_json().expect("json").into_bytes()
    }

    /// [`TARGET`] opened (version 1) and one page appended (version 2), with
    /// the events, the trace and the repository calls cleared.
    fn opened_and_edited(read_only: bool) -> (Gafira, MemoryFs, Arc<MemoryRepository>) {
        let (mut engine, fs, repository) = engine_with_target(read_only);
        engine.projects().open(Path::new(TARGET)).expect("an open");
        append_page(&mut engine);
        assert_eq!(drain(&engine).len(), 2, "the open and the append");
        fs.take_trace();
        repository.take_calls();
        (engine, fs, repository)
    }

    /// The trace of save number `n` of `len` bytes over a [`TARGET`] that
    /// holds `old` bytes: the repository's two writes and its sync (SP6), the
    /// `.bak` copy written, synced and renamed (§7.2 step 4), then the
    /// engine's `replace`.
    fn save_trace(n: u64, len: usize, old: usize) -> Vec<FsOp> {
        let tmp = || PathBuf::from(format!("{TARGET}.tmp-{n}"));
        let bak_tmp = || PathBuf::from(format!("{BAK}.tmp-{n}"));
        vec![
            FsOp::Create(tmp()),
            FsOp::Write(tmp(), len / 2),
            FsOp::Write(tmp(), len - len / 2),
            FsOp::SyncAll(tmp()),
            FsOp::Create(bak_tmp()),
            FsOp::Write(bak_tmp(), old),
            FsOp::SyncAll(bak_tmp()),
            FsOp::Replace(bak_tmp(), PathBuf::from(BAK)),
            FsOp::Replace(tmp(), PathBuf::from(TARGET)),
        ]
    }

    /// Which port an error came from and the text of its `io::Error`.
    fn origin(error: &EngineError) -> (&'static str, String) {
        match error {
            EngineError::Repository(RepositoryError::Io(e)) => ("repository", e.to_string()),
            EngineError::Io(e) => ("filesystem", e.to_string()),
            other => panic!("not a port error: {other:?}"),
        }
    }

    // ARQ-50
    #[test]
    fn ac1_a_fault_at_every_operation_of_a_save_leaves_the_old_or_the_new_bytes_in_full() {
        let (mut engine, fs, _repository) = opened_and_edited(false);
        let new = new_bytes(&mut engine);
        engine.projects().save().expect("a save");
        let trace = fs.take_trace();
        assert_eq!(trace, save_trace(0, new.len(), old_bytes().len()));
        assert_eq!(
            fs.file(BAK),
            Some(old_bytes()),
            "the old bytes in the backup"
        );
        assert_eq!(fs.file(TARGET), Some(new), "the new bytes in full");

        for fault in 0..trace.len() {
            let (engine, fs, _repository) = opened_and_edited(false);
            fs.fail_at(&[fault]);
            assert!(engine.projects().save().is_err(), "fault at {fault}");
            assert_eq!(fs.file(TARGET), Some(old_bytes()), "fault at {fault}");
        }
    }

    // ARQ-50
    #[test]
    fn ac2_replace_runs_only_after_the_repository_save_returned_ok() {
        let (mut engine, fs, repository) = opened_and_edited(false);
        let len = new_bytes(&mut engine).len();
        engine.projects().save().expect("a save");
        assert_eq!(repository.take_calls(), vec![RepoCall::Save(TMP0.into())]);
        assert_eq!(fs.take_trace(), save_trace(0, len, old_bytes().len()));

        // A fault at each operation of the repository's save: no `replace`.
        for fault in 0..4 {
            let (engine, fs, _repository) = opened_and_edited(false);
            fs.fail_at(&[fault]);
            assert!(engine.projects().save().is_err(), "fault at {fault}");
            let mut expected = save_trace(0, len, old_bytes().len())[..=fault].to_vec();
            expected.push(FsOp::Remove(TMP0.into()));
            assert_eq!(fs.take_trace(), expected, "fault at {fault}");
        }
    }

    // ARQ-50
    #[test]
    fn ac3_a_failed_save_removes_the_temporary_returns_the_first_error_and_stays_modified() {
        // Indices of `save_trace`: 3 is the repository's sync, 4 to 7 the
        // backup (create, write, sync, rename) and 8 the `replace`; the
        // removes follow the failed one, the backup's temporary first.
        let cases = [
            (vec![3], ("repository", "fault at 3"), false),
            (vec![4], ("filesystem", "fault at 4"), false),
            (vec![5], ("filesystem", "fault at 5"), false),
            (vec![7], ("filesystem", "fault at 7"), false),
            (vec![8], ("filesystem", "fault at 8"), false),
            (vec![3, 4], ("repository", "fault at 3"), true),
            (vec![8, 9], ("filesystem", "fault at 8"), true),
        ];
        for (faults, (port, text), left_behind) in cases {
            let (mut engine, fs, _repository) = opened_and_edited(false);
            let len = new_bytes(&mut engine).len();
            fs.fail_at(&faults);
            let error = engine.projects().save().expect_err("a failed save");
            assert_eq!(origin(&error), (port, text.to_owned()), "{faults:?}");
            let mut expected = save_trace(0, len, old_bytes().len())[..=faults[0]].to_vec();
            if (4..=7).contains(&faults[0]) {
                expected.push(FsOp::Remove(BAK_TMP0.into()));
            }
            expected.push(FsOp::Remove(TMP0.into()));
            assert_eq!(fs.take_trace(), expected, "{faults:?}");
            let tmp_left = fs.file(TMP0).is_some();
            assert_eq!(tmp_left, left_behind, "{faults:?}: the temporary");
            assert_eq!(fs.file(BAK_TMP0), None, "{faults:?}: no backup temporary");
            assert_eq!(
                fs.file(TARGET),
                Some(old_bytes()),
                "{faults:?}: the old file"
            );
            assert!(
                engine.projects().is_modified(),
                "{faults:?}: still modified"
            );
            assert_eq!(drain(&engine), Vec::new(), "{faults:?}: nothing published");
        }
    }

    // Spec 010, S10-08 (D-55): an unlicensed engine saves like any other;
    // the licence takes no part in saving.
    #[test]
    fn s10_08_saving_never_asks_the_licence() {
        let (engine, _fs, _repository) = opened_and_edited(false);
        assert_eq!(
            engine.license().status(),
            gafira_ports::license::LicenseStatus::Unlicensed
        );
        assert!(!engine.license().can_export());
        engine.projects().save().expect("an unlicensed save");
        assert!(!engine.projects().is_modified());
    }

    // ARQ-50
    #[test]
    fn ac4_a_save_cleans_at_the_snapshot_and_a_later_edit_keeps_it_modified() {
        let (mut engine, _fs, _repository) = opened_and_edited(false);
        engine.projects().save().expect("a save");
        assert!(!engine.projects().is_modified(), "clean after a save");
        assert_eq!(drain(&engine), Vec::new(), "a save never bumps");
        append_page(&mut engine);
        assert!(engine.projects().is_modified(), "an edit after the save");

        // An edit between the snapshot (version 2) and the `replace`.
        let (engine, fs, _repository) = opened_and_edited(false);
        let mut edit = EditService::new(engine.projects().shared.clone(), Arc::new(NoClock));
        fs.before_replace(move || {
            let page = edit.document().expect("a session").pages()[0].id;
            let applied = edit.execute(rename(page, "later", None));
            assert_eq!(applied.ok(), Some(Applied::Changed));
        });
        engine.projects().save().expect("a save");
        assert_eq!(drain(&engine).len(), 1, "the edit published version 3");
        assert!(
            engine.projects().is_modified(),
            "clean at 2, the document at 3"
        );
    }

    #[test]
    fn a_save_that_finishes_after_the_session_was_replaced_leaves_the_new_one_alone() {
        let (engine, fs, _repository) = opened_and_edited(false);
        let shared = engine.projects().shared.clone();
        fs.before_replace(move || {
            let mut cell = shared.lock();
            let clean_version = shared.changed(DirtySet::default());
            *cell = Some(Session {
                history: gafira_edit::History::new(two_page_document()).expect("a history"),
                path: None,
                read_only: false,
                clean_version,
                session_key: "another".to_owned(),
                autosave: Default::default(),
            });
        });
        engine.projects().save().expect("a save");
        assert_eq!(drain(&engine).len(), 1, "the swap published version 3");
        assert!(
            !engine.projects().is_modified(),
            "the new session stays clean at version 3"
        );
    }

    // ARQ-55
    #[test]
    fn ac5_a_read_only_session_edits_undoes_and_redoes_like_any_other() {
        let mut runs = Vec::new();
        for read_only in [false, true] {
            let (mut engine, _fs, _repository) = engine_with_target(read_only);
            engine.projects().open(Path::new(TARGET)).expect("an open");
            append_page(&mut engine);
            assert!(engine.edit().undo().expect("an undo").is_some());
            assert!(engine.edit().redo().expect("a redo").is_some());
            let versions: Vec<u64> = drain(&engine)
                .into_iter()
                .map(|event| match event {
                    EngineEvent::DocumentChanged { version, .. } => version.get(),
                    other => panic!("{other:?}"),
                })
                .collect();
            let pages = engine.edit().document().expect("a session").pages().len();
            runs.push((versions, pages));
        }
        assert_eq!(runs[1], (vec![1, 2, 3, 4], 3), "read-only");
        assert_eq!(runs[0], runs[1], "the same as a writable session");
    }

    // ARQ-55
    #[test]
    fn ac6_a_read_only_save_is_refused_before_any_port_call() {
        let (engine, fs, repository) = opened_and_edited(true);
        let error = engine.projects().save();
        assert!(matches!(error, Err(EngineError::ReadOnly)), "{error:?}");
        assert_eq!(fs.take_trace(), Vec::new(), "no file operation");
        assert_eq!(repository.take_calls(), Vec::new(), "no repository call");
        assert_eq!(
            fs.file(TARGET),
            Some(old_bytes()),
            "the target keeps its bytes"
        );
        assert!(engine.projects().is_modified(), "still modified");
    }

    #[test]
    fn ac7_a_save_with_no_session_or_no_path_is_refused_before_any_port_call() {
        let (engine, fs, repository) = engine_with_target(false);
        let error = engine.projects().save();
        assert!(matches!(error, Err(EngineError::NoSession)), "{error:?}");

        engine.projects().new_document().expect("a new document");
        let error = engine.projects().save();
        assert!(matches!(error, Err(EngineError::NoPath)), "{error:?}");

        assert_eq!(fs.take_trace(), Vec::new(), "no file operation");
        assert_eq!(repository.take_calls(), Vec::new(), "no repository call");
    }

    #[test]
    fn ac10_two_saves_in_a_row_use_distinct_temporaries_in_the_target_folder() {
        let (mut engine, fs, repository) = opened_and_edited(false);
        let len = new_bytes(&mut engine).len();
        engine.projects().save().expect("a first save");
        engine.projects().save().expect("a second save");
        let mut expected = save_trace(0, len, old_bytes().len());
        expected.extend(save_trace(1, len, len));
        assert_eq!(fs.take_trace(), expected);
        assert_eq!(
            repository.take_calls(),
            vec![RepoCall::Save(TMP0.into()), RepoCall::Save(TMP1.into())]
        );
    }

    // ---- Spec 012: project identity, save_as, save_copy, `.gaf.bak` ----

    const OTHER: &str = "/work/other.gaf";
    const COPY: &str = "/work/copy.gaf";

    fn id(n: u128) -> ProjectId {
        SequentialIds::nth(n)
    }

    /// The document stored at `path`.
    fn stored(fs: &MemoryFs, path: &str) -> Document {
        Document::from_json(&fs.file(path).expect("a file")).expect("a document")
    }

    /// The project id of the session's document.
    fn session_id(engine: &mut Gafira) -> ProjectId {
        engine.edit().document().expect("a session").project_id()
    }

    // S12-03
    #[test]
    fn s12_03_new_mints_an_id_and_open_keeps_the_files_own() {
        let (mut engine, fs, _repository) = engine_with_target(false);

        engine.projects().new_document().expect("a new document");
        assert_eq!(session_id(&mut engine), id(1));
        assert!(!engine.projects().is_modified(), "the id is no edit");

        let mine = ProjectId::from_u128(0xfeed);
        let json = two_page_document()
            .with_project_id(mine)
            .to_json()
            .expect("json");
        fs.put(OTHER, json.as_bytes());
        engine.projects().open(Path::new(OTHER)).expect("an open");
        assert_eq!(session_id(&mut engine), mine, "the file's id is kept");

        engine.projects().open(Path::new(TARGET)).expect("an open");
        assert_eq!(session_id(&mut engine), id(2), "a nil id gets the next one");
        assert!(!engine.projects().is_modified());
    }

    // S12-04
    #[test]
    fn s12_04_reopening_a_file_uses_the_same_recovery_folder() {
        let fs = MemoryFs::default();
        let mine = ProjectId::from_u128(0xfeed);
        let json = two_page_document()
            .with_project_id(mine)
            .to_json()
            .expect("json");
        fs.put(TARGET, json.as_bytes());
        let repository = Arc::new(MemoryRepository::new(&fs, false));
        let clock = Arc::new(ManualClock::at(Duration::ZERO));
        let mut engine = engine_with_clock(&fs, &repository, Arc::clone(&clock) as _);
        let folder = PathBuf::from(format!("/data/gafira/recovery/{mine}"));

        for round in 1..=2_u64 {
            engine.projects().open(Path::new(TARGET)).expect("an open");
            append_page(&mut engine);
            clock.set(Duration::from_secs(60 * round));
            let job = engine.projects().autosave_due().expect("a job");
            assert_eq!(job.folder, folder, "round {round}");
        }
    }

    // S12-05
    #[test]
    fn s12_05_save_as_writes_the_new_path_and_adopts_it() {
        let (mut engine, fs, repository) = engine_with_target(false);
        edited_new_document(&mut engine);
        let new = new_bytes(&mut engine);

        engine
            .projects()
            .save_as(Path::new(OTHER))
            .expect("a save as");

        let tmp = PathBuf::from(format!("{OTHER}.tmp-0"));
        assert_eq!(repository.take_calls(), vec![RepoCall::Save(tmp.clone())]);
        assert_eq!(
            fs.take_trace(),
            vec![
                FsOp::Create(tmp.clone()),
                FsOp::Write(tmp.clone(), new.len() / 2),
                FsOp::Write(tmp.clone(), new.len() - new.len() / 2),
                FsOp::SyncAll(tmp.clone()),
                FsOp::Replace(tmp, OTHER.into()),
            ],
            "no backup of a file that did not exist"
        );
        assert_eq!(fs.file(OTHER), Some(new));
        assert_eq!(stored(&fs, OTHER).project_id(), id(1), "the id is kept");
        assert!(!engine.projects().is_modified(), "clean after save as");
        assert_eq!(drain(&engine), Vec::new(), "a save publishes nothing");

        append_page(&mut engine);
        engine.projects().save().expect("save now has a path");
        assert_eq!(fs.file(OTHER), Some(new_bytes(&mut engine)));
        assert_eq!(
            fs.file(TARGET),
            Some(old_bytes()),
            "the old file is untouched"
        );
    }

    // S12-05, ARQ-55
    #[test]
    fn s12_05_save_as_is_the_way_out_of_a_read_only_session() {
        let (mut engine, fs, _repository) = opened_and_edited(true);
        assert!(matches!(
            engine.projects().save(),
            Err(EngineError::ReadOnly)
        ));

        engine
            .projects()
            .save_as(Path::new(OTHER))
            .expect("a save as");
        assert_eq!(fs.file(OTHER), Some(new_bytes(&mut engine)));
        assert_eq!(fs.file(TARGET), Some(old_bytes()), "never overwritten");

        append_page(&mut engine);
        engine.projects().save().expect("no longer read-only");
        assert_eq!(fs.file(OTHER), Some(new_bytes(&mut engine)));
        assert_eq!(fs.file(TARGET), Some(old_bytes()));
    }

    // S12-06
    #[test]
    fn s12_06_save_copy_writes_a_new_identity_and_leaves_the_session() {
        let (mut engine, fs, _repository) = opened_and_edited(false);
        let before = engine.edit().document().expect("a session");

        engine
            .projects()
            .save_copy(Path::new(COPY))
            .expect("a copy");

        let copy = stored(&fs, COPY);
        assert_eq!(copy.project_id(), id(2), "the open minted 1, the copy 2");
        assert_eq!(copy, Document::clone(&before).with_project_id(id(2)));
        assert_eq!(session_id(&mut engine), id(1), "the session keeps its id");
        assert!(
            engine.projects().is_modified(),
            "the session's file did not change"
        );
        engine.projects().save().expect("a save");
        assert_eq!(fs.file(TARGET), Some(new_bytes(&mut engine)), "same path");
    }

    // S12-07
    #[test]
    fn s12_07_save_as_and_save_copy_over_a_file_keep_its_backup() {
        for copy in [false, true] {
            let (mut engine, fs, _repository) = opened_and_edited(false);
            fs.put(OTHER, b"older");
            if copy {
                engine
                    .projects()
                    .save_copy(Path::new(OTHER))
                    .expect("a copy");
            } else {
                engine
                    .projects()
                    .save_as(Path::new(OTHER))
                    .expect("a save as");
                assert_eq!(fs.file(OTHER), Some(new_bytes(&mut engine)));
            }
            let backup = format!("{OTHER}.bak");
            assert_eq!(fs.file(&backup), Some(b"older".to_vec()), "copy: {copy}");
            assert_eq!(fs.file(&format!("{backup}.tmp-0")), None, "copy: {copy}");
        }
    }

    // S12-08, ARQ-50
    #[test]
    fn s12_08_a_fault_anywhere_in_save_as_leaves_the_file_and_the_session() {
        let (engine, fs, _repository) = opened_and_edited(false);
        fs.put(OTHER, b"older");
        engine
            .projects()
            .save_as(Path::new(OTHER))
            .expect("a save as");
        let operations = fs.take_trace().len();
        assert_eq!(operations, 9, "the repository, the backup and the replace");

        for fault in 0..operations {
            let (mut engine, fs, _repository) = opened_and_edited(false);
            fs.put(OTHER, b"older");
            fs.fail_at(&[fault]);
            assert!(engine.projects().save_as(Path::new(OTHER)).is_err());
            assert_eq!(fs.file(OTHER), Some(b"older".to_vec()), "fault at {fault}");
            assert!(engine.projects().is_modified(), "fault at {fault}");
            fs.fail_at(&[]);
            append_page(&mut engine);
            engine
                .projects()
                .save()
                .expect("the session still saves to its file");
            assert_eq!(
                fs.file(TARGET),
                Some(new_bytes(&mut engine)),
                "fault at {fault}"
            );
        }
    }

    // S12-09
    #[test]
    fn s12_09_save_as_and_save_copy_need_a_session() {
        let (engine, fs, repository) = engine_with_target(false);
        assert!(matches!(
            engine.projects().save_as(Path::new(OTHER)),
            Err(EngineError::NoSession)
        ));
        assert!(matches!(
            engine.projects().save_copy(Path::new(COPY)),
            Err(EngineError::NoSession)
        ));
        assert_eq!(fs.take_trace(), Vec::new());
        assert_eq!(repository.take_calls(), Vec::new());
    }
}
