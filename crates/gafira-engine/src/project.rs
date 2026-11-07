//! `ProjectService`: the session's lifecycle (02 §5.4 line 1357).
//!
//! `new_document` (the `new` of §5.4), `open`, `save`, `is_modified` and the
//! autosave schedule (`autosave_due`, `autosave_finished`, design P13).
//! Every file operation goes through the injected ports; the engine opens
//! nothing itself (P1, design P18).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use gafira_doc::{Document, Dpi};
use gafira_edit::{DirtySet, History};
use gafira_ports::repository::ProjectRepository;
use gafira_ports::system::{Clock, FileSystem};

use crate::autosave::{AutosaveJob, AutosaveOutcome, SLOTS};
use crate::engine::{EngineConfig, EngineError, Ports};
use crate::session::{Autosave, Session, Shared};

/// Creates, opens and saves the session's document (02 §5.4 line 1357).
pub struct ProjectService {
    pub(crate) shared: Shared,
    repository: Arc<dyn ProjectRepository>,
    filesystem: Arc<dyn FileSystem>,
    clock: Arc<dyn Clock>,
    /// How long a change waits before it is autosaved (P16).
    autosave_interval: Duration,
    /// `data_dir/recovery`: one folder per session's project key (P13).
    recovery: PathBuf,
    /// Numbers the sessions of this engine, for their project keys (P13).
    sessions: AtomicU64,
    /// Numbers the temporary files of `save`, so no two share a name (P18).
    temporaries: AtomicU64,
}

impl ProjectService {
    pub(crate) fn over(shared: Shared, ports: &Ports, config: &EngineConfig) -> Self {
        Self {
            shared,
            repository: Arc::clone(&ports.repository),
            filesystem: Arc::clone(&ports.filesystem),
            clock: Arc::clone(&ports.clock),
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
    /// steps 1-3, ARQ-50).
    ///
    /// Refused with [`EngineError::NoSession`], then [`EngineError::ReadOnly`]
    /// (ARQ-55), then [`EngineError::NoPath`], all before any port call. The
    /// document and its version are taken under the lock; with the lock
    /// released, the repository writes and syncs a temporary file in the
    /// target's folder, and only once it returns `Ok` does
    /// `FileSystem::replace` move it over the target. On a failure the
    /// temporary file is removed on a best-effort basis, the first error is
    /// returned and the session stays modified. On success the session is
    /// clean at the saved version, if it is still the session that was
    /// saved: an edit made meanwhile keeps it modified, and pending for
    /// autosave (P13). `save` never bumps the version.
    pub fn save(&self) -> Result<(), EngineError> {
        let (snapshot, version, path, key) = {
            let cell = self.shared.lock();
            let session = cell.as_ref().ok_or(EngineError::NoSession)?;
            if session.read_only {
                return Err(EngineError::ReadOnly);
            }
            let path = session.path.clone().ok_or(EngineError::NoPath)?;
            let snapshot = Arc::clone(session.history.document());
            (
                snapshot,
                self.shared.version(),
                path,
                session.project_key.clone(),
            )
        };
        let tmp = self.temporary_for(&path);
        if let Err(error) = self.write_then_replace(&tmp, &path, &snapshot) {
            // Best effort: the first error is the one the caller needs.
            let _ = self.filesystem.remove(&tmp);
            return Err(error);
        }
        let mut cell = self.shared.lock();
        if let Some(session) = cell.as_mut().filter(|s| s.project_key == key) {
            session.clean_version = version;
            // The file holds every change up to the snapshot; one made while
            // it was written stays pending for autosave.
            if self.shared.version() == version {
                session.autosave.pending_since = None;
            }
        }
        Ok(())
    }

    /// Steps 1-3 of §7.2: the repository writes and syncs `tmp` (SP6), then
    /// the file system renames it over `path` and syncs the folder.
    fn write_then_replace(
        &self,
        tmp: &Path,
        path: &Path,
        snapshot: &Document,
    ) -> Result<(), EngineError> {
        self.repository.save(tmp, snapshot)?;
        self.filesystem.replace(tmp, path)?;
        Ok(())
    }

    /// `<path>.tmp-<n>`: in the target's folder, with a name no earlier save
    /// of this engine used (P18; no `rand`, P1).
    fn temporary_for(&self, path: &Path) -> PathBuf {
        let n = self.temporaries.fetch_add(1, Ordering::SeqCst);
        let mut name = path.as_os_str().to_owned();
        name.push(format!(".tmp-{n}"));
        PathBuf::from(name)
    }

    /// Validate `document` into a history, then swap the session for it.
    fn install(
        &self,
        document: Document,
        path: Option<PathBuf>,
        read_only: bool,
    ) -> Result<(), EngineError> {
        let dirty = every_page_and_node(&document);
        let history = History::new(document)?;
        let project_key = format!(
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
            project_key,
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
            folder: self.recovery.join(&session.project_key),
            project_key: session.project_key.clone(),
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
            .filter(|session| session.project_key == outcome.project_key)
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
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use gafira_doc::{NodeId, PageId};
    use gafira_edit::{Applied, DirtySet};
    use gafira_ports::repository::RepositoryError;

    use crate::edit::EditService;
    use crate::engine::{EngineError, Gafira};
    use crate::event::EngineEvent;
    use crate::fakes::*;
    use crate::session::{DocVersion, Session};

    const TARGET: &str = "/work/poster.gafira";
    const TMP0: &str = "/work/poster.gafira.tmp-0";
    const TMP1: &str = "/work/poster.gafira.tmp-1";

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

    /// The trace of one save of `len` bytes through `tmp`: the repository's
    /// two writes and its sync (SP6), then the engine's `replace`.
    fn save_trace(tmp: &str, len: usize) -> Vec<FsOp> {
        let tmp = || PathBuf::from(tmp);
        vec![
            FsOp::Create(tmp()),
            FsOp::Write(tmp(), len / 2),
            FsOp::Write(tmp(), len - len / 2),
            FsOp::SyncAll(tmp()),
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
        assert_eq!(trace, save_trace(TMP0, new.len()));
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
        assert_eq!(fs.take_trace(), save_trace(TMP0, len));

        // A fault at each operation of the repository's save: no `replace`.
        for fault in 0..4 {
            let (engine, fs, _repository) = opened_and_edited(false);
            fs.fail_at(&[fault]);
            assert!(engine.projects().save().is_err(), "fault at {fault}");
            let mut expected = save_trace(TMP0, len)[..=fault].to_vec();
            expected.push(FsOp::Remove(TMP0.into()));
            assert_eq!(fs.take_trace(), expected, "fault at {fault}");
        }
    }

    // ARQ-50
    #[test]
    fn ac3_a_failed_save_removes_the_temporary_returns_the_first_error_and_stays_modified() {
        // Indices of `save_trace`: 3 is the repository's sync, 4 the
        // `replace`; the `remove` follows the failed one.
        let cases = [
            (vec![3], ("repository", "fault at 3"), false),
            (vec![4], ("filesystem", "fault at 4"), false),
            (vec![3, 4], ("repository", "fault at 3"), true),
            (vec![4, 5], ("filesystem", "fault at 4"), true),
        ];
        for (faults, (port, text), left_behind) in cases {
            let (mut engine, fs, _repository) = opened_and_edited(false);
            let len = new_bytes(&mut engine).len();
            fs.fail_at(&faults);
            let error = engine.projects().save().expect_err("a failed save");
            assert_eq!(origin(&error), (port, text.to_owned()), "{faults:?}");
            let mut expected = save_trace(TMP0, len)[..=faults[0]].to_vec();
            expected.push(FsOp::Remove(TMP0.into()));
            assert_eq!(fs.take_trace(), expected, "{faults:?}");
            let tmp_left = fs.file(TMP0).is_some();
            assert_eq!(tmp_left, left_behind, "{faults:?}: the temporary");
            assert!(
                engine.projects().is_modified(),
                "{faults:?}: still modified"
            );
            assert_eq!(drain(&engine), Vec::new(), "{faults:?}: nothing published");
        }
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
                project_key: "another".to_owned(),
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
        let mut expected = save_trace(TMP0, len);
        expected.extend(save_trace(TMP1, len));
        assert_eq!(fs.take_trace(), expected);
        assert_eq!(
            repository.take_calls(),
            vec![RepoCall::Save(TMP0.into()), RepoCall::Save(TMP1.into())]
        );
    }
}
