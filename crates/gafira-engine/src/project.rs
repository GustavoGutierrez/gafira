//! `ProjectService`: the session's lifecycle (02 §5.4 line 1357).
//!
//! `new_document` (the `new` of §5.4), `open`, `save`, `save_as` and
//! `save_copy` (spec 012), `is_modified` and the autosave schedule
//! (`autosave_due`, `autosave_finished`, design P13). Every save keeps one
//! `.gaf.bak` generation (02 §7.2 step 4) and every session carries the
//! project id of D-63. A session with a file holds its `<file>.lock`
//! (spec 028). A session carries the versions of its file and every save
//! writes them; `save_version`, `list_versions` and `restore_version` are
//! the versions of 02 §7.1 (spec 031). Every save but the autosave also
//! hands the repository the thumbnail of the first page (spec 033).
//! Every file operation goes through the injected ports; the engine opens
//! nothing itself (P1, design P18).

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use gafira_doc::{Document, Dpi, Page, PageId, PageSize, ProjectId};
use gafira_edit::{DirtySet, History, RestoreVersion};
use gafira_ports::export::PageRenderers;
use gafira_ports::repository::{ProjectRepository, Version, VersionId, VersionInfo};
use gafira_ports::system::{Clock, FileSystem, IdSource, ProcessId, ProcessProbe};

use crate::autosave::{AutosaveJob, AutosaveOutcome, MARKER, Marker, SLOTS, parse_marker};
use crate::engine::{EngineConfig, EngineError, Ports};
use crate::event::{EngineEvent, RecoveryEntry};
use crate::lock::{FileLock, lock_path, lock_text, parse_lock};
use crate::session::{Autosave, DocVersion, Session, Shared};
use crate::{thumbnail, versions};

/// Creates, opens and saves the session's document (02 §5.4 line 1357).
pub struct ProjectService {
    pub(crate) shared: Shared,
    repository: Arc<dyn ProjectRepository>,
    filesystem: Arc<dyn FileSystem>,
    clock: Arc<dyn Clock>,
    /// Draws the thumbnail of the first page on every save (spec 033).
    pages: Arc<dyn PageRenderers>,
    /// Where the image bytes of the thumbnail come from (spec 032, 033).
    store: Arc<dyn gafira_ports::store::ContentStore>,
    /// Decodes them.
    images: Arc<dyn gafira_ports::export::ImageDecoder>,
    /// Mints the identity of a new project and of a copy (D-63).
    ids: Arc<dyn IdSource>,
    /// This process and the liveness of others (spec 028).
    processes: Arc<dyn ProcessProbe>,
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
            pages: Arc::clone(&ports.pages),
            store: Arc::clone(&ports.store),
            images: Arc::clone(&ports.images),
            ids: Arc::clone(&ports.ids),
            processes: Arc::clone(&ports.processes),
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
        self.install(document, None, false, Vec::new())
    }

    /// [`ProjectService::new_document`] with a first page of `size`, part of
    /// the new document and not an edit of it: undo cannot take it away
    /// (03 §6.2 «Elegir», spec 016).
    ///
    /// # Errors
    /// [`EngineError::InvalidDocument`] when `size` is outside the page
    /// limits; the old session stays.
    pub fn new_document_with(&self, size: PageSize) -> Result<(), EngineError> {
        self.new_document_from(Page::new(PageId::new(1), String::new(), size))
    }

    /// A new document whose one page is `page` (its size, guides, margins),
    /// for «Crear un diseño» (spec 076). The page takes id 1 and no nodes.
    pub fn new_document_from(&self, mut page: Page) -> Result<(), EngineError> {
        page.id = PageId::new(1);
        page.nodes.clear();
        page.roots.clear();
        let document = Document::from_parts(2, Dpi::DEFAULT, vec![Arc::new(page)])?;
        self.install(document, None, false, Vec::new())
    }

    /// A new document with the content of the template at `path` (spec 091,
    /// D-81): a project of its own, with a new id, no file, no versions and
    /// nothing to save yet. The template is only read: it is not locked and
    /// it never becomes the session's file.
    ///
    /// # Errors
    /// Those of [`ProjectService::open`]; the old session stays.
    pub fn new_document_from_template(&self, path: &Path) -> Result<(), EngineError> {
        let loaded = self.repository.load(path)?;
        let document = loaded.document.with_project_id(self.ids.project_id());
        self.install(document, None, false, Vec::new())
    }

    /// The document of the template at `path`, read and nothing else
    /// (spec 091): the session does not change, and the template's media
    /// land in the content store, so its pages can be put on a page of the
    /// session.
    ///
    /// # Errors
    /// Those of [`ProjectService::open`].
    pub fn read_template(&self, path: &Path) -> Result<Document, EngineError> {
        Ok(self.repository.load(path)?.document)
    }

    /// End the session: the document and its history go, and so do its
    /// recovery slots, because the person chose to leave (spec 016). The UI
    /// asks «¿Guardar los cambios?» first (03 §6.1). Nothing is published.
    pub fn close(&self) {
        let ended = self.shared.lock().take();
        if let Some(session) = ended {
            self.release_lock(session.lock.as_deref());
            self.discard_slots(session.history.document().project_id());
        }
    }

    /// The projects with recovery slots left by a run that did not end
    /// them: a crash, or a close that never came (02 §7.2 «Recuperación»).
    /// The open session's own project is not listed. Publishes
    /// `RecoveryAvailable` when there is any.
    ///
    /// A project whose marker names another process that still runs is not
    /// listed either: it is a second instance's open session (spec 028).
    ///
    /// # Errors
    /// [`EngineError::Io`] when the recovery folder cannot be listed; a
    /// missing folder is no error.
    pub fn recoverable(&self) -> Result<Vec<RecoveryEntry>, EngineError> {
        if !self.filesystem.exists(&self.recovery)? {
            return Ok(Vec::new());
        }
        let current = self
            .shared
            .lock()
            .as_ref()
            .map(|session| session.history.document().project_id());
        let mut entries = Vec::new();
        for folder in self.filesystem.read_dir(&self.recovery)? {
            let Some(project_id) = folder
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.parse::<ProjectId>().ok())
            else {
                continue;
            };
            if Some(project_id) == current {
                continue;
            }
            if let Some(marker) = self.marker(project_id) {
                if self.another_live(marker.owner) {
                    // Another running instance's session (spec 028).
                    continue;
                }
                entries.push(RecoveryEntry {
                    project_id,
                    origin: marker.origin,
                });
            }
        }
        entries.sort_by_key(|entry| entry.project_id);
        if !entries.is_empty() {
            self.shared
                .bus()
                .publish(EngineEvent::RecoveryAvailable(entries.clone()));
        }
        Ok(entries)
    }

    /// Replace the session with the newest readable slot of `project_id`
    /// (02 §7.2: a slot that does not load gives way to the one before).
    /// The recovered session lives at the file it came from and is modified:
    /// its changes are not in that file yet. The slots stay until the next
    /// save or close.
    ///
    /// # Errors
    /// [`EngineError::NoRecovery`] when the project has no marker; the error
    /// of the oldest slot when none loads. The old session stays then.
    pub fn recover(&self, project_id: ProjectId) -> Result<(), EngineError> {
        let Marker {
            slot: newest,
            origin,
            ..
        } = self.marker(project_id).ok_or(EngineError::NoRecovery)?;
        let folder = self.recovery.join(project_id.to_string());
        let mut last = EngineError::NoRecovery;
        for back in 0..SLOTS {
            let slot = (newest + SLOTS - back) % SLOTS;
            match self
                .repository
                .load(&folder.join(format!("slot-{slot}.gaf-journal")))
            {
                Ok(loaded) => {
                    self.install(loaded.document, origin, false, loaded.versions)?;
                    if let Some(session) = self.shared.lock().as_mut() {
                        // Nothing the file holds: modified until saved.
                        session.clean_version = DocVersion(0);
                    }
                    return Ok(());
                }
                Err(error) => last = error.into(),
            }
        }
        Err(last)
    }

    /// Delete the recovery slots of `project_id`: the person chose
    /// «Descartar» (02 §7.2).
    pub fn discard_recovery(&self, project_id: ProjectId) {
        self.discard_slots(project_id);
    }

    /// Whether `pid` is another process of this machine that still runs.
    fn another_live(&self, pid: ProcessId) -> bool {
        pid != self.processes.current() && self.processes.is_alive(pid)
    }

    /// The marker of `project_id`'s slots, if it reads.
    fn marker(&self, project_id: ProjectId) -> Option<Marker> {
        let path = self.recovery.join(project_id.to_string()).join(MARKER);
        let mut text = String::new();
        self.filesystem
            .open(&path)
            .ok()?
            .read_to_string(&mut text)
            .ok()?;
        parse_marker(&text)
    }

    /// Remove the marker and the slots of `project_id`, best effort: a file
    /// that is not there is fine, and a failure only leaves an offer of
    /// recovery behind.
    fn discard_slots(&self, project_id: ProjectId) {
        let folder = self.recovery.join(project_id.to_string());
        let files = std::iter::once(folder.join(MARKER))
            .chain((0..SLOTS).map(|slot| folder.join(format!("slot-{slot}.gaf-journal"))));
        for file in files {
            if self.filesystem.exists(&file).unwrap_or(false) {
                let _ = self.filesystem.remove(&file);
            }
        }
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
    /// and [`ProjectService::save`] refuses it (ARQ-55). So does a file
    /// another running instance, or another machine, holds the lock of:
    /// [`ProjectService::locked_by_other`] names it. Otherwise the session
    /// takes `<path>.lock` and the old session's lock goes (spec 028).
    pub fn open(&self, path: &Path) -> Result<(), EngineError> {
        let loaded = self.repository.load(path)?;
        self.install(
            loaded.document,
            Some(path.to_owned()),
            loaded.read_only,
            loaded.versions,
        )
    }

    /// Write the session's document over its file, atomically (02 §7.2
    /// steps 1-4, ARQ-50).
    ///
    /// Refused with [`EngineError::NoSession`], then [`EngineError::ReadOnly`]
    /// (ARQ-55), then [`EngineError::NoPath`], all before any port call. The
    /// document and its version are taken under the lock; with the lock
    /// released, the [`SaveJob`] writes it. On a failure
    /// the first error is returned and the session stays modified. On success
    /// the session is clean at the saved version, if it is still the session
    /// that was saved: an edit made meanwhile keeps it modified, and pending
    /// for autosave (P13). `save` never bumps the version.
    ///
    /// The file keeps the session's versions, plus an automatic one of the
    /// saved document when the newest automatic one is more than 30 min old
    /// (02 §7.1, spec 031); the session holds them once the file does.
    pub fn save(&self) -> Result<(), EngineError> {
        self.save_finished(self.prepare_save(SaveRequest::Save)?.run())
    }

    /// Take everything a save of `request` needs, with its refusals, and
    /// write nothing (spec 035, 036): the returned [`SaveJob`] is
    /// `Send + 'static`, the host runs it on its own thread, and hands its
    /// outcome to [`ProjectService::save_finished`]. The session goes on
    /// meanwhile: an edit made while the job runs keeps it modified.
    ///
    /// The refusals and the versions the file keeps are those of
    /// [`ProjectService::save`], [`ProjectService::save_as`],
    /// [`ProjectService::save_copy`] and [`ProjectService::save_version`].
    /// [`SaveRequest::Version`] with a path is «Guardar como» of a project
    /// never saved, with the named version and no automatic one.
    ///
    /// # Errors
    /// [`EngineError::NoSession`], [`EngineError::ReadOnly`] and
    /// [`EngineError::NoPath`] as those methods say, before any port call.
    pub fn prepare_save(&self, request: SaveRequest) -> Result<SaveJob, EngineError> {
        let now = self.clock.now();
        let (document, versions, path, account) = match request {
            SaveRequest::Save => {
                let (snapshot, path) = self.writable(|snapshot| {
                    versions::on_save(&snapshot.versions, &snapshot.document, now)
                })?;
                Self::job_parts(snapshot, path, None)
            }
            SaveRequest::Version { name, at: None } => {
                let (snapshot, path) = self.writable(|snapshot| {
                    versions::with_named(&snapshot.versions, &name, &snapshot.document, now)
                })?;
                Self::job_parts(snapshot, path, None)
            }
            SaveRequest::Version {
                name,
                at: Some(path),
            } => {
                let snapshot = self.moved(|snapshot| {
                    versions::with_named(&snapshot.versions, &name, &snapshot.document, now)
                })?;
                Self::job_parts(snapshot, path.clone(), Some(path))
            }
            SaveRequest::SaveAs(path) => {
                let snapshot = self.moved(|snapshot| {
                    versions::on_save(&snapshot.versions, &snapshot.document, now)
                })?;
                Self::job_parts(snapshot, path.clone(), Some(path))
            }
            SaveRequest::SaveCopy(path) => {
                let (document, kept) = {
                    let cell = self.shared.lock();
                    let session = cell.as_ref().ok_or(EngineError::NoSession)?;
                    (
                        Arc::clone(session.history.document()),
                        session.versions.clone(),
                    )
                };
                let copy = Document::clone(&document).with_project_id(self.ids.project_id());
                (Arc::new(copy), kept, path, None)
            }
        };
        Ok(SaveJob {
            writer: self.writer(),
            temporary: self.temporaries.fetch_add(1, Ordering::SeqCst),
            document,
            versions,
            path,
            account,
        })
    }

    /// A writer of a document that is not the session's (spec 104: the
    /// editable `.gaf` of a bulk run, made off the window thread). It
    /// writes under a new project id and with no versions; the session does
    /// not change and nothing is accounted for. Its assets come from the
    /// same store.
    #[must_use]
    pub fn document_saver(&self) -> DocumentSaver {
        DocumentSaver {
            writer: self.writer(),
            temporary: self.temporaries.fetch_add(1, Ordering::SeqCst),
            project_id: self.ids.project_id(),
        }
    }

    /// Account for a [`SaveJob`] that ran: on success the session is clean
    /// at the saved version, holds its versions and, after «Guardar como»,
    /// lives at its new file with the lock moved; a session replaced or
    /// closed meanwhile is left alone. A copy changes nothing.
    ///
    /// # Errors
    /// The first error of the repository or the file system; the session
    /// stays as it was then.
    pub fn save_finished(&self, outcome: SaveOutcome) -> Result<(), EngineError> {
        outcome.result?;
        if let Some((snapshot, path)) = outcome.account {
            self.saved(&snapshot, path.as_deref());
        }
        Ok(())
    }

    /// The parts of a job that writes `snapshot` at `path` and accounts for
    /// it; `moved_to` is the new file of «Guardar como».
    fn job_parts(
        snapshot: Snapshot,
        path: PathBuf,
        moved_to: Option<PathBuf>,
    ) -> (Arc<Document>, Vec<Version>, PathBuf, Account) {
        (
            Arc::clone(&snapshot.document),
            snapshot.versions.clone(),
            path,
            Some((snapshot, moved_to)),
        )
    }

    /// The snapshot of the session for «Guardar como», with the versions
    /// `keep` gives; any session may be saved elsewhere (ARQ-55).
    fn moved(&self, keep: impl FnOnce(&Snapshot) -> Vec<Version>) -> Result<Snapshot, EngineError> {
        let cell = self.shared.lock();
        let session = cell.as_ref().ok_or(EngineError::NoSession)?;
        let mut snapshot = self.snapshot_of(session);
        snapshot.versions = keep(&snapshot);
        Ok(snapshot)
    }

    /// What writes a file, owned, for a job on another thread.
    fn writer(&self) -> Writer {
        Writer {
            repository: Arc::clone(&self.repository),
            filesystem: Arc::clone(&self.filesystem),
            pages: Arc::clone(&self.pages),
            store: Arc::clone(&self.store),
            images: Arc::clone(&self.images),
        }
    }

    /// Add a named version of the session's document called `name` (trimmed)
    /// and write the file with it: Archivo → Guardar versión (02 §5.4,
    /// §7.1; spec 031). Same refusals, order and failure rules as
    /// [`ProjectService::save`]: on a failure the session has no new
    /// version; on success it is clean at the saved version. No automatic
    /// version is added.
    ///
    /// # Errors
    /// As [`ProjectService::save`].
    pub fn save_version(&self, name: &str) -> Result<(), EngineError> {
        let request = SaveRequest::Version {
            name: name.to_owned(),
            at: None,
        };
        self.save_finished(self.prepare_save(request)?.run())
    }

    /// The versions of the open project, oldest first; empty with no
    /// session (spec 031).
    #[must_use]
    pub fn list_versions(&self) -> Vec<VersionInfo> {
        self.shared
            .lock()
            .as_ref()
            .map_or_else(Vec::new, |session| {
                session.versions.iter().map(|v| v.info.clone()).collect()
            })
    }

    /// Put the content of version `id` back into the document with one
    /// [`RestoreVersion`] command: one undo entry, one `DocumentChanged`,
    /// and the session is modified until saved (02 §7.1, spec 031). Undo
    /// takes it back like any edit. The versions do not change.
    ///
    /// # Errors
    /// [`EngineError::NoSession`], then [`EngineError::NoVersion`] when the
    /// project has no version `id`; nothing changes then.
    pub fn restore_version(&self, id: VersionId) -> Result<(), EngineError> {
        let version = {
            let cell = self.shared.lock();
            let session = cell.as_ref().ok_or(EngineError::NoSession)?;
            session
                .versions
                .iter()
                .find(|v| v.info.id == id)
                .map(|v| Arc::clone(&v.document))
                .ok_or(EngineError::NoVersion)?
        };
        crate::edit::run(
            &self.shared,
            &*self.clock,
            Box::new(RestoreVersion::new(&version)),
        )?;
        Ok(())
    }

    /// The snapshot of a session that may be saved over its file, with the
    /// versions `keep` gives, and that file; the refusals of
    /// [`ProjectService::save`], in its order, before any port call.
    fn writable(
        &self,
        keep: impl FnOnce(&Snapshot) -> Vec<Version>,
    ) -> Result<(Snapshot, PathBuf), EngineError> {
        let cell = self.shared.lock();
        let session = cell.as_ref().ok_or(EngineError::NoSession)?;
        if session.read_only {
            return Err(EngineError::ReadOnly);
        }
        let path = session.path.clone().ok_or(EngineError::NoPath)?;
        let mut snapshot = self.snapshot_of(session);
        snapshot.versions = keep(&snapshot);
        Ok((snapshot, path))
    }

    /// Write the session's document at `path` and make `path` the session's
    /// file (02 §5.4 line 1358, spec 012).
    ///
    /// The way out of a new document and of a read-only session (ARQ-55): on
    /// success the session lives at `path`, may be saved over and is clean at
    /// the saved version, with the same project id (D-63). Same order and
    /// same failure rules as [`ProjectService::save`]; a failure changes
    /// nothing in the session. The automatic-version rule of
    /// [`ProjectService::save`] applies.
    ///
    /// # Errors
    /// [`EngineError::NoSession`] before any port call; then the first error
    /// of the repository or the file system.
    pub fn save_as(&self, path: &Path) -> Result<(), EngineError> {
        self.save_finished(
            self.prepare_save(SaveRequest::SaveAs(path.to_owned()))?
                .run(),
        )
    }

    /// Write a copy of the session's document at `path` under a new project
    /// id (D-63: «Guardar copia»), leaving the session as it was: its file,
    /// its id, its read-only flag and its modified state. The copy keeps the
    /// session's versions as they are (spec 031).
    ///
    /// # Errors
    /// [`EngineError::NoSession`] before any port call; then the first error
    /// of the repository or the file system.
    pub fn save_copy(&self, path: &Path) -> Result<(), EngineError> {
        self.save_finished(
            self.prepare_save(SaveRequest::SaveCopy(path.to_owned()))?
                .run(),
        )
    }

    /// The document, the version, the key and the versions of `session`,
    /// taken under the caller's lock.
    fn snapshot_of(&self, session: &Session) -> Snapshot {
        Snapshot {
            document: Arc::clone(session.history.document()),
            version: self.shared.version(),
            key: session.session_key.clone(),
            versions: session.versions.clone(),
        }
    }

    /// Account for a save of `snapshot`: clean at its version, holding its
    /// versions and, for `save_as`, living at `path` and writable. A session
    /// replaced meanwhile is left alone.
    fn saved(&self, snapshot: &Snapshot, path: Option<&Path>) {
        let (key, version) = (snapshot.key.as_str(), snapshot.version);
        let mut cell = self.shared.lock();
        let Some(session) = cell.as_mut().filter(|s| s.session_key == key) else {
            return;
        };
        session.versions.clone_from(&snapshot.versions);
        let moved_lock = path.map(|path| {
            session.path = Some(path.to_owned());
            session.read_only = false;
            session.locked_by = None;
            session.lock.take()
        });
        session.clean_version = version;
        let project_id = session.history.document().project_id();
        // The file holds every change up to the snapshot; one made while it
        // was written stays pending for autosave.
        let all_saved = self.shared.version() == version;
        if all_saved {
            session.autosave.pending_since = None;
        }
        drop(cell);
        // `save_as` moved the session to `path`: its lock moves too
        // (spec 028).
        if let (Some(old), Some(path)) = (moved_lock, path) {
            self.release_lock(old.as_deref());
            self.lock_file(key, path);
        }
        // The file now holds everything the slots held: they would only
        // offer a stale recovery (spec 016).
        if all_saved {
            self.discard_slots(project_id);
        }
    }

    /// Validate `document` into a history, then swap the session for it.
    fn install(
        &self,
        document: Document,
        path: Option<PathBuf>,
        read_only: bool,
        versions: Vec<Version>,
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
        // Spec 028: another instance's live lock makes the session
        // read-only; otherwise the file is taken below.
        let locked_by = path.as_deref().and_then(|path| self.foreign_lock(path));
        let read_only = read_only || locked_by.is_some();
        let take = path.clone().filter(|_| locked_by.is_none());
        let session_key = format!(
            "{}-{}",
            self.clock.now().0,
            self.sessions.fetch_add(1, Ordering::SeqCst)
        );
        let mut cell = self.shared.lock();
        let clean_version = self.shared.changed(dirty);
        let replaced = cell.replace(Session {
            history,
            path,
            read_only,
            clean_version,
            session_key: session_key.clone(),
            autosave: Autosave::default(),
            lock: None,
            locked_by,
            versions,
        });
        drop(cell);
        // The old lock goes first, so reopening the same file keeps one.
        self.release_lock(replaced.as_ref().and_then(|old| old.lock.as_deref()));
        if let Some(path) = take {
            self.lock_file(&session_key, &path);
        }
        Ok(())
    }

    /// The lock on `path` when it belongs to another process that runs,
    /// or to any process of another machine, whose liveness cannot be
    /// checked (spec 028). A missing or unreadable lock, or one of a dead
    /// process of this machine, is `None`: it may be taken over.
    fn foreign_lock(&self, path: &Path) -> Option<FileLock> {
        let mut text = String::new();
        self.filesystem
            .open(&lock_path(path))
            .ok()?
            .read_to_string(&mut text)
            .ok()?;
        let lock = parse_lock(&text)?;
        let foreign = lock.host != self.processes.host() || self.another_live(lock.pid);
        foreign.then_some(lock)
    }

    /// Write this process's lock on `path` atomically (a temporary, synced,
    /// then `replace`) and hand it to the session named `key`. Best effort:
    /// a file that cannot be locked still opens, with no lock to remove.
    fn lock_file(&self, key: &str, path: &Path) {
        let lock = lock_path(path);
        let tmp = suffixed(&lock, ".tmp");
        let text = lock_text(&FileLock {
            pid: self.processes.current(),
            host: self.processes.host(),
            started: self.clock.now(),
        });
        let written = (|| -> std::io::Result<()> {
            let mut file = self.filesystem.create(&tmp)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
            drop(file);
            self.filesystem.replace(&tmp, &lock)
        })();
        if written.is_err() {
            let _ = self.filesystem.remove(&tmp);
            return;
        }
        let mut cell = self.shared.lock();
        match cell.as_mut().filter(|s| s.session_key == key) {
            Some(session) => session.lock = Some(lock),
            None => {
                // The session ended meanwhile: its lock must not outlive it.
                drop(cell);
                let _ = self.filesystem.remove(&lock);
            }
        }
    }

    /// Remove a lock this process wrote, best effort: a lock left behind is
    /// stale once the process ends and is taken over then.
    fn release_lock(&self, lock: Option<&Path>) {
        if let Some(lock) = lock {
            let _ = self.filesystem.remove(lock);
        }
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
            versions: session.versions.clone(),
            version: self.shared.version(),
            slot: session.autosave.slot,
            folder: self
                .recovery
                .join(session.history.document().project_id().to_string()),
            session_key: session.session_key.clone(),
            origin: session.path.clone(),
            owner: self.processes.current(),
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

    /// Who holds the open file's lock when it is another running instance
    /// or another machine: the session was opened read-only because of it
    /// (spec 028, ARQ-55). `None` with no session, no file, or a lock that
    /// is this session's own.
    #[must_use]
    pub fn locked_by_other(&self) -> Option<FileLock> {
        self.shared
            .lock()
            .as_ref()
            .and_then(|session| session.locked_by.clone())
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

/// What a save writes, taken under the session lock: the document, the
/// version it was at, the session it came from and the versions the file
/// keeps.
struct Snapshot {
    document: Arc<Document>,
    version: DocVersion,
    key: String,
    versions: Vec<Version>,
}

/// What a save asks for (spec 036).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveRequest {
    /// «Guardar» over the session's file.
    Save,
    /// «Guardar como» at a new file, which becomes the session's.
    SaveAs(PathBuf),
    /// «Guardar copia» at a file, under a new project id.
    SaveCopy(PathBuf),
    /// «Guardar versión…» with this name; `at` is the file of a project
    /// never saved, chosen first.
    Version {
        /// The name of the version, trimmed by the engine.
        name: String,
        /// The new file, for a project with none yet.
        at: Option<PathBuf>,
    },
}

/// The snapshot a save accounts for and the new file of «Guardar como».
type Account = Option<(Snapshot, Option<PathBuf>)>;

/// A checked save that owns what it writes: `Send + 'static`, so the host
/// runs it off the window thread (spec 036).
pub struct SaveJob {
    writer: Writer,
    temporary: u64,
    document: Arc<Document>,
    versions: Vec<Version>,
    path: PathBuf,
    account: Account,
}

impl std::fmt::Debug for SaveJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SaveJob")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl SaveJob {
    /// The file the job writes.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Write the file atomically, with its thumbnail and its `.bak` (02
    /// §7.2). Its outcome goes back to [`ProjectService::save_finished`].
    #[must_use]
    pub fn run(self) -> SaveOutcome {
        let result = self.writer.write_atomically(
            &self.path,
            &self.document,
            &self.versions,
            self.temporary,
        );
        SaveOutcome {
            result,
            account: self.account,
        }
    }
}

/// Writes one document that is not the session's (spec 104); `Send +
/// 'static`, so it runs on the thread that made the document.
pub struct DocumentSaver {
    writer: Writer,
    temporary: u64,
    project_id: ProjectId,
}

impl std::fmt::Debug for DocumentSaver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocumentSaver").finish_non_exhaustive()
    }
}

impl DocumentSaver {
    /// Write `document` at `path` atomically, with its thumbnail and the
    /// `.bak` of a file that was there (02 §7.2).
    ///
    /// # Errors
    /// The first error of the repository or the file system.
    pub fn run(self, document: Document, path: &Path) -> Result<(), EngineError> {
        let document = document.with_project_id(self.project_id);
        self.writer
            .write_atomically(path, &document, &[], self.temporary)
    }
}

/// How a [`SaveJob`] ended, for [`ProjectService::save_finished`].
pub struct SaveOutcome {
    result: Result<(), EngineError>,
    account: Account,
}

impl std::fmt::Debug for SaveOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SaveOutcome")
            .field("result", &self.result)
            .finish_non_exhaustive()
    }
}

impl SaveOutcome {
    /// The error of the write, if it failed.
    #[must_use]
    pub fn error(&self) -> Option<&EngineError> {
        self.result.as_ref().err()
    }
}

/// The ports that write a project file (02 §7.2), owned by a [`SaveJob`].
struct Writer {
    repository: Arc<dyn ProjectRepository>,
    filesystem: Arc<dyn FileSystem>,
    pages: Arc<dyn PageRenderers>,
    store: Arc<dyn gafira_ports::store::ContentStore>,
    images: Arc<dyn gafira_ports::export::ImageDecoder>,
}

impl Writer {
    /// 02 §7.2: the repository writes and syncs `<path>.tmp-<n>` (steps 1-2,
    /// SP6), with the thumbnail of the first page when it can be drawn
    /// (spec 033); when `path` exists its bytes go to `<path>.bak` (step 4); then
    /// the file system renames the temporary over `path` and syncs the
    /// folder (step 3). Each step runs only after the one before returned
    /// `Ok`, so a failure anywhere leaves `path` with its old bytes. The
    /// temporaries are removed on a failure, on a best-effort basis: the
    /// first error is the one the caller needs.
    fn write_atomically(
        &self,
        path: &Path,
        document: &Document,
        versions: &[Version],
        n: u64,
    ) -> Result<(), EngineError> {
        // Spec 033: best effort, a page that cannot be drawn is saved
        // without its thumbnail.
        // Its images come from the store like an export's (spec 032); a
        // missing or broken image only costs the thumbnail its pixels.
        let first: Vec<_> = document
            .pages()
            .first()
            .map(|page| page.id)
            .into_iter()
            .collect();
        let images = crate::export::decode_images(document, &first, &*self.store, &*self.images)
            .unwrap_or_default();
        let thumbnail = thumbnail::first_page(document, &*self.pages, images);
        let tmp = suffixed(path, &format!(".tmp-{n}"));
        let written = self
            .repository
            .save_with_thumbnail(&tmp, document, versions, thumbnail.as_ref())
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
    /// With the versions a save keeps: before the first save of a session
    /// and after it, the same automatic one (spec 031).
    fn new_bytes(engine: &mut Gafira) -> Vec<u8> {
        let document = engine.edit().document().expect("a session");
        let projects = engine.projects();
        let kept = crate::versions::on_save(
            &projects.shared.lock().as_ref().expect("a session").versions,
            &document,
            projects.clock.now(),
        );
        encoded(&document, &kept)
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
                lock: None,
                locked_by: None,
                versions: Vec::new(),
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
        decoded(&fs.file(path).expect("a file"))
            .expect("a document")
            .0
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

    // S91-10
    #[test]
    fn s91_10_a_template_is_read_without_touching_the_session() {
        let (mut engine, fs, _repository) = engine_with_target(false);
        engine.projects().new_document().expect("a session");
        append_page(&mut engine);
        let modified = engine.projects().is_modified();
        let template = two_page_document().with_project_id(ProjectId::from_u128(0xfeed));
        fs.put(OTHER, template.to_json().expect("json").as_bytes());

        let read = engine
            .projects()
            .read_template(Path::new(OTHER))
            .expect("the template");

        assert_eq!(read.pages().len(), template.pages().len());
        assert_eq!(session_id(&mut engine), id(1), "the session stays");
        assert_eq!(engine.projects().is_modified(), modified);
        assert_eq!(engine.projects().path(), None);
        assert_eq!(fs.file(&format!("{OTHER}.lock")), None, "not locked");
    }

    // S91-03
    #[test]
    fn s91_03_a_template_opens_as_a_new_untitled_document() {
        use gafira_ports::common::UnixTime;
        use gafira_ports::repository::{Version, VersionId, VersionInfo, VersionKind};

        let fs = MemoryFs::default();
        let theirs = ProjectId::from_u128(0xfeed);
        let template = two_page_document().with_project_id(theirs);
        let version = Version {
            info: VersionInfo {
                id: VersionId(1),
                name: "draft".to_owned(),
                kind: VersionKind::Named,
                created: UnixTime(5),
                app: "Gafira".to_owned(),
                schema: 2,
            },
            document: Arc::new(template.clone()),
        };
        let bytes = encoded(&template, &[version]);
        fs.put(TARGET, &bytes);
        let repository = Arc::new(MemoryRepository::new(&fs, true));
        let mut engine = engine_on(&fs, &repository);

        engine
            .projects()
            .new_document_from_template(Path::new(TARGET))
            .expect("a document from the template");

        let document = engine.edit().document().expect("a session");
        assert_eq!(
            document.pages().len(),
            template.pages().len(),
            "its content"
        );
        assert_eq!(document.project_id(), id(1), "a project of its own");
        assert_eq!(engine.projects().path(), None, "never saved");
        assert!(engine.projects().list_versions().is_empty(), "no versions");
        assert!(!engine.projects().is_modified(), "nothing to save yet");
        assert!(
            matches!(engine.projects().save(), Err(EngineError::NoPath)),
            "writable, though the template loaded read-only: save asks for a path"
        );
        assert_eq!(fs.file(TARGET), Some(bytes), "the template stays as it was");
        assert_eq!(
            fs.file(&format!("{TARGET}.lock")),
            None,
            "and is not locked"
        );
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
        let lock = PathBuf::from(format!("{OTHER}.lock"));
        let lock_tmp = PathBuf::from(format!("{OTHER}.lock.tmp"));
        assert_eq!(repository.take_calls(), vec![RepoCall::Save(tmp.clone())]);
        assert_eq!(
            fs.take_trace(),
            vec![
                FsOp::Create(tmp.clone()),
                FsOp::Write(tmp.clone(), new.len() / 2),
                FsOp::Write(tmp.clone(), new.len() - new.len() / 2),
                FsOp::SyncAll(tmp.clone()),
                FsOp::Replace(tmp, OTHER.into()),
                // Spec 028: the session now holds the new file's lock.
                FsOp::Create(lock_tmp.clone()),
                FsOp::Write(lock_tmp.clone(), "1000\nhere\n0".len()),
                FsOp::SyncAll(lock_tmp.clone()),
                FsOp::Replace(lock_tmp, lock),
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

    // S104-40: the pages a bulk run made are saved at their own file under a
    // new identity, and the session does not change.
    #[test]
    fn s104_40_a_generated_document_saves_beside_the_session() {
        let (mut engine, fs, _repository) = opened_and_edited(false);
        let generated = Document::default();
        let saver = engine.projects().document_saver();
        fn send_static<T: Send + 'static>(_: &T) {}
        send_static(&saver);
        saver
            .run(generated.clone(), Path::new(COPY))
            .expect("saved");
        let copy = stored(&fs, COPY);
        assert_eq!(copy, generated.with_project_id(id(2)));
        assert_eq!(session_id(&mut engine), id(1), "the session keeps its id");
        assert!(engine.projects().is_modified(), "the session is not saved");
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
        assert_eq!(
            operations, 14,
            "the repository, the backup and the replace, then the lock moves"
        );

        // Spec 028: the lock is best effort; a fault while it moves keeps
        // the save.
        for fault in 9..operations {
            let (engine, fs, _repository) = opened_and_edited(false);
            fs.fail_at(&[fault]);
            engine
                .projects()
                .save_as(Path::new(OTHER))
                .expect("saved despite the lock");
            assert!(!engine.projects().is_modified(), "fault at {fault}");
        }

        for fault in 0..9 {
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

    // ---- Spec 016: first page, close, recovery after a crash ----

    use gafira_doc::PageSize;

    use crate::autosave::marker_text;
    use crate::event::RecoveryEntry;

    /// An engine over `fs` with a clock the test moves; `fs` survives it, as
    /// the disk survives a crash.
    fn rig_on(fs: &MemoryFs) -> (Gafira, Arc<ManualClock>) {
        let repository = Arc::new(MemoryRepository::new(fs, false));
        let clock = Arc::new(ManualClock::at(Duration::ZERO));
        let engine = engine_with_clock(fs, &repository, Arc::clone(&clock) as _);
        (engine, clock)
    }

    /// [`TARGET`] opened and edited, and one autosave run and accounted
    /// for: slot 0 and the marker are on disk. Returns the edited document.
    fn crashed_after_an_autosave(fs: &MemoryFs) -> Document {
        let (mut engine, clock) = rig_on(fs);
        engine.projects().open(Path::new(TARGET)).expect("an open");
        append_page(&mut engine);
        clock.set(Duration::from_secs(60));
        let job = engine.projects().autosave_due().expect("a job");
        let outcome = job.run();
        assert!(outcome.error().is_none());
        engine.projects().autosave_finished(outcome);
        Document::clone(&engine.edit().document().expect("a document"))
        // The engine is dropped without `close`: the crash.
    }

    fn target_fs() -> MemoryFs {
        let fs = MemoryFs::default();
        fs.put(
            TARGET,
            two_page_document().to_json().expect("json").as_bytes(),
        );
        fs
    }

    fn slot_file(n: u8) -> String {
        format!("/data/gafira/recovery/{}/slot-{n}.gaf-journal", id(1))
    }

    // S16-01
    #[test]
    fn s16_01_the_first_page_belongs_to_the_new_document_and_undo_keeps_it() {
        let (mut engine, _fs, _repository) = engine_with_target(false);
        let size = PageSize {
            width: gafira_geom::Length::new(1080.0, gafira_geom::Unit::Px),
            height: gafira_geom::Length::new(1920.0, gafira_geom::Unit::Px),
        };
        engine
            .projects()
            .new_document_with(size)
            .expect("a new document");
        let document = engine.edit().document().expect("a document");
        assert_eq!(document.pages().len(), 1);
        assert_eq!(document.pages()[0].size, size);
        assert!(!document.project_id().is_nil());
        assert!(!engine.projects().is_modified());
        assert_eq!(engine.edit().can_undo_redo(), (false, false));
        assert_eq!(engine.edit().undo().expect("an undo"), None);
        assert_eq!(
            engine.edit().document().expect("a document").pages().len(),
            1
        );
    }

    // S16-02
    #[test]
    fn s16_02_the_marker_names_the_slot_and_the_file() {
        use crate::autosave::parse_marker;
        let owner = FakeProcesses::PID;
        let text = marker_text(2, owner, Some(Path::new(TARGET)));
        let marker = parse_marker(&text).expect("a marker");
        assert_eq!(marker.slot, 2);
        assert_eq!(marker.origin, Some(PathBuf::from(TARGET)));
        let marker = parse_marker(&marker_text(0, owner, None)).expect("a marker");
        assert_eq!((marker.slot, marker.origin), (0, None));
        for bad in ["", "3\n1\n/x", "x\n1\n/x", "-1"] {
            assert!(parse_marker(bad).is_none(), "{bad:?}");
        }
    }

    // S16-03
    #[test]
    fn s16_03_after_a_crash_the_work_is_offered_and_recovered_where_it_belongs() {
        let fs = target_fs();
        let edited = crashed_after_an_autosave(&fs);

        let (mut engine, _clock) = rig_on(&fs);
        let entries = engine.projects().recoverable().expect("a scan");
        let entry = RecoveryEntry {
            project_id: id(1),
            origin: Some(PathBuf::from(TARGET)),
        };
        assert_eq!(entries, std::slice::from_ref(&entry));
        assert_eq!(
            drain(&engine),
            [EngineEvent::RecoveryAvailable(vec![entry])]
        );

        engine.projects().recover(id(1)).expect("recovers");
        assert_eq!(*engine.edit().document().expect("a document"), edited);
        assert_eq!(engine.projects().path(), Some(PathBuf::from(TARGET)));
        assert!(
            engine.projects().is_modified(),
            "the file does not hold it yet"
        );
        assert_eq!(
            engine.projects().recoverable().expect("a scan"),
            [],
            "its own slots are not offered"
        );
    }

    // S16-04
    #[test]
    fn s16_04_a_slot_that_does_not_load_gives_way_to_the_one_before() {
        let fs = target_fs();
        let edited = crashed_after_an_autosave(&fs);
        // A newer slot 1 that the crash cut short, named by the marker.
        fs.put(&slot_file(1), b"cut short");
        let marker = format!("/data/gafira/recovery/{}/latest", id(1));
        let text = marker_text(1, FakeProcesses::PID, Some(Path::new(TARGET)));
        fs.put(&marker, text.as_bytes());

        let (mut engine, _clock) = rig_on(&fs);
        engine.projects().recover(id(1)).expect("recovers slot 0");
        assert_eq!(*engine.edit().document().expect("a document"), edited);

        fs.put(&slot_file(0), b"broken too");
        let (engine, _clock) = rig_on(&fs);
        assert!(matches!(
            engine.projects().recover(id(1)),
            Err(EngineError::Repository(_))
        ));
        assert!(matches!(
            engine.projects().recover(id(9)),
            Err(EngineError::NoRecovery)
        ));
    }

    // S16-05
    #[test]
    fn s16_05_save_close_and_discard_remove_the_slots() {
        let marker = format!("/data/gafira/recovery/{}/latest", id(1));
        for way in ["save", "close", "discard"] {
            let fs = target_fs();
            crashed_after_an_autosave(&fs);
            let (engine, _clock) = rig_on(&fs);
            match way {
                "save" | "close" => {
                    engine.projects().recover(id(1)).expect("recovers");
                    if way == "save" {
                        engine.projects().save().expect("saves");
                    } else {
                        engine.projects().close();
                        assert!(!engine.projects().is_modified(), "no session");
                    }
                }
                _ => engine.projects().discard_recovery(id(1)),
            }
            assert_eq!(fs.file(&marker), None, "{way}");
            assert_eq!(fs.file(&slot_file(0)), None, "{way}");
            let (engine, _clock) = rig_on(&fs);
            assert_eq!(
                engine.projects().recoverable().expect("a scan"),
                [],
                "{way}"
            );
        }
    }

    // ---- Spec 028: file locks and another instance's slots ----

    use gafira_ports::system::ProcessId;

    /// An engine over `fs` running as `pid`, with `alive` running besides.
    fn instance(fs: &MemoryFs, pid: u32, alive: &[u32]) -> (Gafira, Arc<ManualClock>) {
        let repository = Arc::new(MemoryRepository::new(fs, false));
        let clock = Arc::new(ManualClock::at(Duration::ZERO));
        let processes = Arc::new(FakeProcesses::new(pid, alive));
        let engine = engine_with_processes(fs, &repository, Arc::clone(&clock) as _, processes);
        (engine, clock)
    }

    /// [`crashed_after_an_autosave`], run by the instance `pid`, which is
    /// returned still running.
    fn autosaved_by(fs: &MemoryFs, pid: u32) -> Gafira {
        let (mut engine, clock) = instance(fs, pid, &[]);
        engine.projects().open(Path::new(TARGET)).expect("an open");
        append_page(&mut engine);
        clock.set(Duration::from_secs(60));
        let outcome = engine.projects().autosave_due().expect("a job").run();
        assert!(outcome.error().is_none());
        engine.projects().autosave_finished(outcome);
        engine
    }

    // S28-01
    #[test]
    fn s28_01_the_marker_names_its_owner() {
        use crate::autosave::parse_marker;
        let text = marker_text(1, ProcessId(77), Some(Path::new(TARGET)));
        let marker = parse_marker(&text).expect("a marker");
        assert_eq!(marker.owner, ProcessId(77));
        assert_eq!(marker.slot, 1);
        assert_eq!(marker.origin, Some(PathBuf::from(TARGET)));
        for bad in ["1", "1\n", "1\nx\n/x", "1\n/x"] {
            assert!(parse_marker(bad).is_none(), "{bad:?}");
        }

        let fs = target_fs();
        let _running = autosaved_by(&fs, 7);
        let marker = format!("/data/gafira/recovery/{}/latest", id(1));
        let text = String::from_utf8(fs.file(&marker).expect("a marker")).expect("text");
        assert_eq!(
            parse_marker(&text).expect("a marker").owner,
            ProcessId(7),
            "the autosave writes its own pid"
        );
    }

    // S28-02
    #[test]
    fn s28_02_a_live_other_instances_slots_are_not_offered() {
        let fs = target_fs();
        let _running = autosaved_by(&fs, 7);
        let (engine, _clock) = instance(&fs, 8, &[7]);
        assert_eq!(engine.projects().recoverable().expect("a scan"), []);
        assert_eq!(drain(&engine), [], "no RecoveryAvailable");
    }

    // S28-03
    #[test]
    fn s28_03_a_dead_owners_slots_are_offered_and_recovered() {
        let fs = target_fs();
        drop(autosaved_by(&fs, 7));
        let (mut engine, _clock) = instance(&fs, 8, &[]);
        let entries = engine.projects().recoverable().expect("a scan");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].project_id, id(1));
        engine.projects().recover(id(1)).expect("recovers");
        let document = engine.edit().document().expect("a document");
        assert_eq!(document.pages().len(), 3, "the autosaved edit");
    }

    use crate::lock::FileLock;

    const LOCK: &str = "/work/poster.gaf.lock";
    const LOCK_TMP: &str = "/work/poster.gaf.lock.tmp";

    /// The text of a lock held by `pid` on `host`, started at `started`.
    fn lock_of(pid: u32, host: &str, started: i64) -> Vec<u8> {
        format!("{pid}\n{host}\n{started}").into_bytes()
    }

    /// The lock this test's engines write: the default fake pid and host,
    /// at the manual clock's wall time.
    fn own_lock() -> Vec<u8> {
        lock_of(
            FakeProcesses::PID.0,
            FakeProcesses::HOST,
            ManualClock::NOW.0,
        )
    }

    // S28-04
    #[test]
    fn s28_04_open_writes_the_lock_atomically_and_close_removes_it() {
        let fs = target_fs();
        let (engine, _clock) = rig_on(&fs);
        engine.projects().open(Path::new(TARGET)).expect("an open");
        let text = own_lock();
        assert_eq!(
            fs.take_trace(),
            vec![
                FsOp::Create(LOCK_TMP.into()),
                FsOp::Write(LOCK_TMP.into(), text.len()),
                FsOp::SyncAll(LOCK_TMP.into()),
                FsOp::Replace(LOCK_TMP.into(), LOCK.into()),
            ]
        );
        assert_eq!(fs.file(LOCK), Some(text));
        assert_eq!(engine.projects().locked_by_other(), None);
        engine.projects().close();
        assert_eq!(fs.file(LOCK), None, "close");
    }

    // S28-04
    #[test]
    fn s28_04_a_new_document_or_another_file_releases_the_lock() {
        let other_lock = format!("{OTHER}.lock");
        let fs = target_fs();
        fs.put(OTHER, &old_bytes());
        let (engine, _clock) = rig_on(&fs);
        engine.projects().open(Path::new(TARGET)).expect("an open");
        engine
            .projects()
            .open(Path::new(OTHER))
            .expect("another open");
        assert_eq!(fs.file(LOCK), None, "the first file's lock");
        assert_eq!(fs.file(&other_lock), Some(own_lock()));
        engine.projects().new_document().expect("a new document");
        assert_eq!(fs.file(&other_lock), None, "new_document");

        engine.projects().open(Path::new(TARGET)).expect("an open");
        engine
            .projects()
            .open(Path::new(TARGET))
            .expect("the same again");
        assert_eq!(fs.file(LOCK), Some(own_lock()), "reopening keeps it");
    }

    // S28-05, ARQ-55
    #[test]
    fn s28_05_a_live_foreign_lock_opens_read_only_and_is_left_alone() {
        for (host, alive) in [(FakeProcesses::HOST, &[7_u32][..]), ("there", &[][..])] {
            let fs = target_fs();
            let foreign = lock_of(7, host, 5);
            fs.put(LOCK, &foreign);
            let (mut engine, _clock) = instance(&fs, 8, alive);
            engine.projects().open(Path::new(TARGET)).expect("an open");
            assert_eq!(
                engine.projects().locked_by_other(),
                Some(FileLock {
                    pid: ProcessId(7),
                    host: host.to_owned(),
                    started: gafira_ports::common::UnixTime(5),
                }),
                "{host}"
            );
            append_page(&mut engine);
            assert!(
                matches!(engine.projects().save(), Err(EngineError::ReadOnly)),
                "{host}"
            );
            assert_eq!(fs.file(LOCK), Some(foreign.clone()), "{host}: kept");
            engine.projects().close();
            assert_eq!(fs.file(LOCK), Some(foreign), "{host}: not ours to remove");
        }
    }

    // S28-06
    #[test]
    fn s28_06_a_stale_or_unreadable_lock_is_taken_over() {
        for stale in [lock_of(7, FakeProcesses::HOST, 5), b"garbage".to_vec()] {
            let fs = target_fs();
            fs.put(LOCK, &stale);
            let (mut engine, _clock) = instance(&fs, FakeProcesses::PID.0, &[]);
            engine.projects().open(Path::new(TARGET)).expect("an open");
            assert_eq!(engine.projects().locked_by_other(), None);
            assert_eq!(fs.file(LOCK), Some(own_lock()), "taken over");
            append_page(&mut engine);
            engine.projects().save().expect("writable");
        }
    }

    // S28-04, S12-05
    #[test]
    fn s28_04_save_as_moves_the_lock_to_the_new_file() {
        let fs = target_fs();
        let (engine, _clock) = rig_on(&fs);
        engine.projects().open(Path::new(TARGET)).expect("an open");
        engine
            .projects()
            .save_as(Path::new(OTHER))
            .expect("a save as");
        assert_eq!(fs.file(LOCK), None, "the old file's lock");
        assert_eq!(fs.file(&format!("{OTHER}.lock")), Some(own_lock()));
    }

    // ---- Spec 031: versions inside the file ----

    use gafira_ports::common::UnixTime;
    use gafira_ports::repository::{VersionId, VersionKind};

    /// The versions the file at `path` holds, by id and kind.
    fn kept(fs: &MemoryFs, path: &str) -> Vec<(u64, VersionKind)> {
        decoded(&fs.file(path).expect("a file"))
            .expect("a project")
            .1
            .iter()
            .map(|v| (v.info.id.0, v.info.kind))
            .collect()
    }

    /// The versions of the session, by id and kind.
    fn listed(engine: &Gafira) -> Vec<(u64, VersionKind)> {
        engine
            .projects()
            .list_versions()
            .iter()
            .map(|info| (info.id.0, info.kind))
            .collect()
    }

    /// [`TARGET`] opened on an engine with a manual clock.
    fn opened_with_clock(fs: &MemoryFs) -> (Gafira, Arc<ManualClock>) {
        let (engine, clock) = rig_on(fs);
        engine.projects().open(Path::new(TARGET)).expect("an open");
        (engine, clock)
    }

    // S31-05
    #[test]
    fn s31_05_a_save_adds_an_automatic_version_only_every_30_minutes() {
        let fs = target_fs();
        let (mut engine, clock) = opened_with_clock(&fs);
        assert_eq!(listed(&engine), []);
        engine.projects().save().expect("a save");
        let auto = [(1, VersionKind::Auto)];
        assert_eq!(listed(&engine), auto, "the first save");
        assert_eq!(kept(&fs, TARGET), auto, "and the file holds it");
        let info = &engine.projects().list_versions()[0];
        assert_eq!(info.created, ManualClock::NOW);

        append_page(&mut engine);
        clock.set_now(UnixTime(ManualClock::NOW.0 + 30 * 60));
        engine.projects().save().expect("a save");
        assert_eq!(listed(&engine), auto, "30 min is not more than 30 min");

        clock.set_now(UnixTime(ManualClock::NOW.0 + 30 * 60 + 1));
        engine
            .projects()
            .save_as(Path::new(OTHER))
            .expect("a save as");
        let two = [(1, VersionKind::Auto), (2, VersionKind::Auto)];
        assert_eq!(listed(&engine), two, "save_as follows the rule too");
        assert_eq!(kept(&fs, OTHER), two);
        assert_eq!(
            engine.projects().list_versions()[1].created,
            UnixTime(ManualClock::NOW.0 + 30 * 60 + 1)
        );
    }

    // S31-05
    #[test]
    fn s31_05_save_copy_and_autosave_add_no_version() {
        let fs = target_fs();
        let (mut engine, clock) = opened_with_clock(&fs);
        engine
            .projects()
            .save_copy(Path::new(COPY))
            .expect("a copy");
        assert_eq!(kept(&fs, COPY), []);
        append_page(&mut engine);
        clock.set(Duration::from_secs(60));
        let job = engine.projects().autosave_due().expect("a job");
        engine.projects().autosave_finished(job.run());
        assert_eq!(kept(&fs, &slot_file(0)), []);
        assert_eq!(listed(&engine), []);
    }

    // S31-06
    #[test]
    fn s31_06_save_version_writes_a_named_version_and_cleans_the_session() {
        let fs = target_fs();
        let (mut engine, _clock) = opened_with_clock(&fs);
        append_page(&mut engine);
        engine
            .projects()
            .save_version("  Final  ")
            .expect("a version");
        let infos = engine.projects().list_versions();
        assert_eq!(infos.len(), 1, "no automatic one beside it");
        let info = &infos[0];
        assert_eq!(info.id, VersionId(1));
        assert_eq!(info.name, "Final");
        assert_eq!(info.kind, VersionKind::Named);
        assert_eq!(info.created, ManualClock::NOW);
        assert_eq!(info.app, env!("CARGO_PKG_VERSION"));
        assert_eq!(info.schema, Document::SCHEMA_VERSION);
        assert_eq!(kept(&fs, TARGET), [(1, VersionKind::Named)]);
        assert!(!engine.projects().is_modified(), "the file holds it all");
        let (_, versions) = decoded(&fs.file(TARGET).expect("a file")).expect("a project");
        assert_eq!(
            *versions[0].document,
            *engine.edit().document().expect("a session"),
            "the version is the saved document"
        );
    }

    // S31-06
    #[test]
    fn s31_06_save_version_is_refused_before_any_port_call() {
        let (engine, _fs, repository) = engine_with_target(false);
        assert!(matches!(
            engine.projects().save_version("x"),
            Err(EngineError::NoSession)
        ));
        engine.projects().new_document().expect("a new document");
        assert!(matches!(
            engine.projects().save_version("x"),
            Err(EngineError::NoPath)
        ));
        assert_eq!(listed(&engine), []);
        let (read_only, _fs, read_only_repository) = engine_with_target(true);
        read_only
            .projects()
            .open(Path::new(TARGET))
            .expect("an open");
        read_only_repository.take_calls();
        assert!(matches!(
            read_only.projects().save_version("x"),
            Err(EngineError::ReadOnly)
        ));
        assert_eq!(listed(&read_only), []);
        assert_eq!(read_only_repository.take_calls(), []);
        assert_eq!(repository.take_calls(), []);
    }

    // S31-07
    #[test]
    fn s31_07_versions_travel_with_the_file_its_copies_and_its_recovery() {
        let fs = target_fs();
        let (mut engine, clock) = opened_with_clock(&fs);
        engine.projects().save_version("a").expect("a version");
        engine.projects().open(Path::new(TARGET)).expect("a reopen");
        let named = [(1, VersionKind::Named)];
        assert_eq!(listed(&engine), named, "open reads them");

        engine
            .projects()
            .save_copy(Path::new(COPY))
            .expect("a copy");
        assert_eq!(kept(&fs, COPY), named, "the copy keeps them");

        engine.projects().save().expect("a save");
        let both = [(1, VersionKind::Named), (2, VersionKind::Auto)];
        assert_eq!(kept(&fs, TARGET), both);

        append_page(&mut engine);
        clock.set(Duration::from_secs(60));
        let job = engine.projects().autosave_due().expect("a job");
        engine.projects().autosave_finished(job.run());
        assert_eq!(kept(&fs, &slot_file(0)), both, "the slot keeps them");
        engine.projects().new_document().expect("a new document");
        assert_eq!(listed(&engine), []);
        engine.projects().recover(id(1)).expect("recovers");
        assert_eq!(listed(&engine), both, "and so does a recovery");
    }

    // S31-09
    #[test]
    fn s31_09_restoring_a_version_is_one_change_undo_takes_back() {
        let fs = target_fs();
        let (mut engine, _clock) = opened_with_clock(&fs);
        engine.projects().save_version("before").expect("a version");
        let page = append_page(&mut engine);
        drain(&engine);

        engine
            .projects()
            .restore_version(VersionId(1))
            .expect("restores");
        let events = drain(&engine);
        assert_eq!(events.len(), 1, "{events:?}");
        assert!(
            matches!(&events[0], EngineEvent::DocumentChanged { dirty, .. }
                if dirty.pages.contains(&page)),
            "the page that went is repainted: {events:?}"
        );
        let document = engine.edit().document().expect("a session");
        assert_eq!(document.pages(), two_page_document().pages());
        assert_eq!(document.project_id(), id(1));
        assert!(engine.projects().is_modified());
        assert_eq!(listed(&engine), [(1, VersionKind::Named)], "kept");

        engine.edit().undo().expect("undoes");
        let document = engine.edit().document().expect("a session");
        assert!(document.page(page).is_some(), "the edit is back");
    }

    // S31-09
    #[test]
    fn s31_09_an_unknown_version_or_no_session_is_refused() {
        let fs = target_fs();
        let (rig, _clock) = rig_on(&fs);
        assert!(matches!(
            rig.projects().restore_version(VersionId(1)),
            Err(EngineError::NoSession)
        ));
        let (engine, _clock) = opened_with_clock(&fs);
        drain(&engine);
        assert!(matches!(
            engine.projects().restore_version(VersionId(7)),
            Err(EngineError::NoVersion)
        ));
        assert_eq!(drain(&engine), []);
    }

    // S36-01: `prepare_save` writes nothing, refuses like `save` and gives a
    // job that can go to another thread.
    #[test]
    fn s36_01_prepare_save_writes_nothing_and_the_job_is_send() {
        fn send_static<T: Send + 'static>(_: &T) {}
        let (engine, fs, repository) = opened_and_edited(false);
        let job = engine
            .projects()
            .prepare_save(super::SaveRequest::Save)
            .expect("prepares");
        send_static(&job);
        assert_eq!(job.path(), Path::new(TARGET));
        assert!(fs.take_trace().is_empty(), "nothing written yet");
        assert!(repository.take_calls().is_empty());
        assert!(engine.projects().is_modified(), "not saved yet");
        engine
            .projects()
            .save_finished(job.run())
            .expect("the job saves");
        assert!(!engine.projects().is_modified());

        let (read_only, fs, _) = opened_and_edited(true);
        assert!(matches!(
            read_only.projects().prepare_save(super::SaveRequest::Save),
            Err(EngineError::ReadOnly)
        ));
        assert!(fs.take_trace().is_empty());
    }

    // S36-02: the job writes the document as it was when it was prepared;
    // an edit made while it runs keeps the session modified.
    #[test]
    fn s36_02_an_edit_while_the_job_runs_stays_modified() {
        let (mut engine, fs, _repository) = opened_and_edited(false);
        let prepared = new_bytes(&mut engine);
        let job = engine
            .projects()
            .prepare_save(super::SaveRequest::Save)
            .expect("prepares");
        append_page(&mut engine);
        let outcome = job.run();
        assert!(outcome.error().is_none());
        engine
            .projects()
            .save_finished(outcome)
            .expect("the job saves");
        assert_eq!(fs.file(TARGET), Some(prepared), "the prepared snapshot");
        assert!(
            engine.projects().is_modified(),
            "the later edit is not saved"
        );
    }

    // S36-03: a job that fails hands back its error and the session stays
    // modified, at its file.
    #[test]
    fn s36_03_a_failed_job_keeps_the_session_modified() {
        let (engine, fs, _repository) = opened_and_edited(false);
        let job = engine
            .projects()
            .prepare_save(super::SaveRequest::Save)
            .expect("prepares");
        fs.fail_at(&[3]);
        let outcome = job.run();
        assert!(outcome.error().is_some());
        assert!(engine.projects().save_finished(outcome).is_err());
        assert!(engine.projects().is_modified());
        assert_eq!(fs.file(TARGET), Some(old_bytes()), "the old bytes");
        assert_eq!(engine.projects().path().as_deref(), Some(Path::new(TARGET)));
    }

    // S36-04: a named version of a project never saved goes to the file
    // chosen first, which becomes the session's, with no automatic version.
    #[test]
    fn s36_04_a_version_of_a_new_project_saves_it_at_its_new_file() {
        let fs = target_fs();
        let (mut engine, _clock) = rig_on(&fs);
        edited_new_document(&mut engine);
        let job = engine
            .projects()
            .prepare_save(super::SaveRequest::Version {
                name: " Primera ".to_owned(),
                at: Some(PathBuf::from(OTHER)),
            })
            .expect("prepares");
        engine
            .projects()
            .save_finished(job.run())
            .expect("the job saves");
        assert_eq!(engine.projects().path().as_deref(), Some(Path::new(OTHER)));
        assert!(!engine.projects().is_modified());
        assert_eq!(kept(&fs, OTHER), [(1, VersionKind::Named)]);
        assert_eq!(engine.projects().list_versions()[0].name, "Primera");
    }

    // S36-05: a copy run as a job leaves the session as it was.
    #[test]
    fn s36_05_a_copy_job_leaves_the_session() {
        let (engine, fs, _repository) = opened_and_edited(false);
        let job = engine
            .projects()
            .prepare_save(super::SaveRequest::SaveCopy(PathBuf::from(OTHER)))
            .expect("prepares");
        engine
            .projects()
            .save_finished(job.run())
            .expect("the copy is written");
        assert!(fs.file(OTHER).is_some());
        assert!(engine.projects().is_modified());
        assert_eq!(engine.projects().path().as_deref(), Some(Path::new(TARGET)));
    }
}
