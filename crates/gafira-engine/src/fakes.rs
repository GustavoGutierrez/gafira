//! Private doubles of the ports for the engine's own tests (design P16).
//!
//! They stay inside the crate until a second crate needs the same doubles.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use gafira_doc::msg::{CommandLabel, MessageKey};
use gafira_doc::{
    ContentHash, DocMut, Document, Dpi, ElementBase, GroupNode, Node, NodeId, NodeKind, Page,
    PageId, PageSize, Placement,
};
use gafira_edit::{
    Applied, Command, CommandError, DirtySet, GestureId, History, InsertPage, Property, SetProperty,
};
use gafira_geom::{Length, Unit};
use gafira_ports::common::UnixTime;
use gafira_ports::license::{LicenseError, LicenseKey, LicenseStatus, LicenseVerifier};
use gafira_ports::repository::{LoadedProject, ProjectRepository, RepositoryError};
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::{Clock, FileSystem, FileWrite, Notifier, ReadSeek};

use crate::bus::EventChannel;
use crate::engine::{EngineConfig, Gafira, Ports, Registries};
use crate::event::{EngineEvent, NoopWaker};

pub(crate) struct NoRepository;

impl ProjectRepository for NoRepository {
    fn load(&self, _path: &Path) -> Result<LoadedProject, RepositoryError> {
        Err(RepositoryError::Io(io::Error::other("unused")))
    }

    fn save(&self, _path: &Path, _document: &Document) -> Result<(), RepositoryError> {
        Err(RepositoryError::Io(io::Error::other("unused")))
    }
}

pub(crate) struct NoStore;

impl ContentStore for NoStore {
    fn put(&self, _src: &mut dyn io::Read) -> Result<ContentHash, StoreError> {
        Err(StoreError::Io(io::Error::other("unused")))
    }

    fn open(&self, _hash: ContentHash) -> Result<Box<dyn ReadSeek + Send>, StoreError> {
        Err(StoreError::Io(io::Error::other("unused")))
    }

    fn contains(&self, _hash: ContentHash) -> Result<bool, StoreError> {
        Err(StoreError::Io(io::Error::other("unused")))
    }
}

pub(crate) struct NoFilesystem;

impl FileSystem for NoFilesystem {
    fn open(&self, _path: &Path) -> io::Result<Box<dyn ReadSeek + Send>> {
        Err(io::Error::other("unused"))
    }

    fn create(&self, _path: &Path) -> io::Result<Box<dyn FileWrite>> {
        Err(io::Error::other("unused"))
    }

    fn replace(&self, _from: &Path, _to: &Path) -> io::Result<()> {
        Err(io::Error::other("unused"))
    }

    fn remove(&self, _path: &Path) -> io::Result<()> {
        Err(io::Error::other("unused"))
    }

    fn exists(&self, _path: &Path) -> io::Result<bool> {
        Err(io::Error::other("unused"))
    }

    fn create_dir_all(&self, _path: &Path) -> io::Result<()> {
        Err(io::Error::other("unused"))
    }

    fn read_dir(&self, _path: &Path) -> io::Result<Vec<PathBuf>> {
        Err(io::Error::other("unused"))
    }
}

pub(crate) struct NoClock;

impl Clock for NoClock {
    fn now(&self) -> UnixTime {
        UnixTime(0)
    }

    fn monotonic(&self) -> Duration {
        Duration::ZERO
    }
}

pub(crate) struct NoNotifier;

impl Notifier for NoNotifier {
    fn notify(&self, _title: &str, _body: &str) {}
}

pub(crate) struct NoLicense;

impl LicenseVerifier for NoLicense {
    fn status(&self, _now: UnixTime) -> LicenseStatus {
        LicenseStatus::Unlicensed
    }

    fn activate(
        &self,
        _key: &LicenseKey,
        _label: Option<&str>,
    ) -> Result<LicenseStatus, LicenseError> {
        Err(LicenseError::Storage)
    }

    fn validate(&self, _now: UnixTime) -> Result<LicenseStatus, LicenseError> {
        Err(LicenseError::Storage)
    }

    fn deactivate(&self) -> Result<(), LicenseError> {
        Err(LicenseError::Storage)
    }
}

pub(crate) fn config() -> EngineConfig {
    EngineConfig::new("/data/gafira").expect("a valid data directory")
}

pub(crate) fn six_ports() -> Ports {
    Ports::new(
        Arc::new(NoRepository),
        Arc::new(NoStore),
        Arc::new(NoFilesystem),
        Arc::new(NoClock),
        Arc::new(NoNotifier),
        Arc::new(NoLicense),
    )
}

/// A channel nobody wakes, over the zero clock.
pub(crate) fn quiet_channel() -> EventChannel {
    EventChannel::new(Arc::new(NoopWaker), Arc::new(NoClock))
}

/// An engine over the fakes that publishes into `events`.
pub(crate) fn engine_over(events: EventChannel) -> Gafira {
    Gafira::new(six_ports(), Registries::new(), config(), events).expect("an engine")
}

/// The name of every page the tests append.
pub(crate) const NAME: &str = "page";

/// An engine after `new`, with its `DocumentChanged` (version 1) drained.
pub(crate) fn open_engine() -> Gafira {
    let engine = engine_over(quiet_channel());
    engine.projects().new_document().expect("a new document");
    assert_eq!(drain(&engine).len(), 1, "the new");
    engine
}

/// Every event waiting in the engine's queue, in order.
pub(crate) fn drain(engine: &Gafira) -> Vec<EngineEvent> {
    engine.events().try_iter().collect()
}

/// Append one page called [`NAME`] and return its id.
pub(crate) fn append_page(engine: &mut Gafira) -> PageId {
    let len = engine.edit().document().expect("a session").pages().len();
    let size = PageSize {
        width: Length::new(10.0, Unit::Px),
        height: Length::new(10.0, Unit::Px),
    };
    let insert = InsertPage::new(len, NAME.to_owned(), size);
    let applied = engine.edit().execute(Box::new(insert));
    assert_eq!(applied.ok(), Some(Applied::Changed));
    engine.edit().document().expect("a session").pages()[len].id
}

/// A command that renames `page`, inside `gesture` when one is given.
pub(crate) fn rename(page: PageId, name: &str, gesture: Option<GestureId>) -> Box<SetProperty> {
    Box::new(SetProperty::page(
        page,
        Property::PageName(name.to_owned()),
        gesture,
    ))
}

/// The name of the first page of the open document.
pub(crate) fn page_name(engine: &mut Gafira) -> String {
    engine.edit().document().expect("a session").pages()[0]
        .name
        .clone()
}

/// The dirty set of exactly these pages.
pub(crate) fn pages(ids: &[PageId]) -> DirtySet {
    DirtySet {
        pages: ids.iter().copied().collect(),
        nodes: Default::default(),
    }
}

/// One write-side operation the [`MemoryFs`] was asked for, failed or not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FsOp {
    Create(PathBuf),
    /// The file and the number of bytes of one `write` call.
    Write(PathBuf, usize),
    SyncAll(PathBuf),
    Replace(PathBuf, PathBuf),
    Remove(PathBuf),
}

#[derive(Default)]
struct FsState {
    files: BTreeMap<PathBuf, Vec<u8>>,
    trace: Vec<FsOp>,
    faults: BTreeSet<usize>,
}

/// An in-memory `FileSystem` that records every `create`, `write`,
/// `sync_all`, `replace` and `remove`, and fails the ones whose index in the
/// trace was armed with [`MemoryFs::fail_at`] (ARQ-50, design P16). A failed
/// operation changes nothing.
#[derive(Clone, Default)]
pub(crate) struct MemoryFs {
    state: Arc<Mutex<FsState>>,
}

impl MemoryFs {
    fn state(&self) -> MutexGuard<'_, FsState> {
        self.state.lock().expect("the fake's lock")
    }

    /// Put a file in place without tracing it.
    pub(crate) fn put(&self, path: &str, bytes: &[u8]) {
        self.state()
            .files
            .insert(PathBuf::from(path), bytes.to_vec());
    }

    /// The bytes of the file at `path`, if there is one.
    pub(crate) fn file(&self, path: &str) -> Option<Vec<u8>> {
        self.state().files.get(Path::new(path)).cloned()
    }

    /// The trace so far, which is then cleared.
    pub(crate) fn take_trace(&self) -> Vec<FsOp> {
        std::mem::take(&mut self.state().trace)
    }

    /// Fail the operations that will sit at these indices of the trace.
    pub(crate) fn fail_at(&self, indices: &[usize]) {
        self.state().faults = indices.iter().copied().collect();
    }

    /// Record `op`, then run `effect` unless its index was armed.
    fn traced<T>(
        &self,
        op: FsOp,
        effect: impl FnOnce(&mut FsState) -> io::Result<T>,
    ) -> io::Result<T> {
        let mut state = self.state();
        let index = state.trace.len();
        state.trace.push(op);
        if state.faults.contains(&index) {
            return Err(io::Error::other(format!("fault at {index}")));
        }
        effect(&mut state)
    }
}

fn not_found(path: &Path) -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, path.display().to_string())
}

/// A file of a [`MemoryFs`] open for writing; every `write` appends.
struct MemoryFile {
    fs: MemoryFs,
    path: PathBuf,
}

impl io::Write for MemoryFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let path = &self.path;
        self.fs
            .traced(FsOp::Write(path.clone(), buf.len()), |state| {
                let file = state.files.get_mut(path).ok_or_else(|| not_found(path))?;
                file.extend_from_slice(buf);
                Ok(buf.len())
            })
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl io::Seek for MemoryFile {
    fn seek(&mut self, _pos: io::SeekFrom) -> io::Result<u64> {
        Err(io::ErrorKind::Unsupported.into())
    }
}

impl FileWrite for MemoryFile {
    fn sync_all(&mut self) -> io::Result<()> {
        self.fs.traced(FsOp::SyncAll(self.path.clone()), |_| Ok(()))
    }
}

impl FileSystem for MemoryFs {
    fn open(&self, path: &Path) -> io::Result<Box<dyn ReadSeek + Send>> {
        let bytes = self.state().files.get(path).cloned();
        let bytes = bytes.ok_or_else(|| not_found(path))?;
        Ok(Box::new(io::Cursor::new(bytes)))
    }

    fn create(&self, path: &Path) -> io::Result<Box<dyn FileWrite>> {
        self.traced(FsOp::Create(path.to_owned()), |state| {
            state.files.insert(path.to_owned(), Vec::new());
            Ok(())
        })?;
        Ok(Box::new(MemoryFile {
            fs: self.clone(),
            path: path.to_owned(),
        }))
    }

    fn replace(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.traced(FsOp::Replace(from.to_owned(), to.to_owned()), |state| {
            let bytes = state.files.remove(from).ok_or_else(|| not_found(from))?;
            state.files.insert(to.to_owned(), bytes);
            Ok(())
        })
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        self.traced(FsOp::Remove(path.to_owned()), |state| {
            state
                .files
                .remove(path)
                .map(drop)
                .ok_or_else(|| not_found(path))
        })
    }

    fn exists(&self, path: &Path) -> io::Result<bool> {
        Ok(self.state().files.contains_key(path))
    }

    fn create_dir_all(&self, _path: &Path) -> io::Result<()> {
        Ok(())
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        let state = self.state();
        let inside = state
            .files
            .keys()
            .filter(|file| file.parent() == Some(path));
        Ok(inside.cloned().collect())
    }
}

/// One call the [`MemoryRepository`] received.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RepoCall {
    Load(PathBuf),
    Save(PathBuf),
}

/// An in-memory `ProjectRepository` that reads and writes the canonical JSON
/// **through** a [`MemoryFs`], so the fault trace covers its writes (P16).
/// `save` writes the bytes in two halves, then syncs the file (SP6).
pub(crate) struct MemoryRepository {
    fs: MemoryFs,
    read_only: bool,
    calls: Mutex<Vec<RepoCall>>,
    hand_over: Mutex<Option<Document>>,
}

impl MemoryRepository {
    /// A repository over `fs` whose loads carry `read_only`.
    pub(crate) fn new(fs: &MemoryFs, read_only: bool) -> Self {
        Self {
            fs: fs.clone(),
            read_only,
            calls: Mutex::new(Vec::new()),
            hand_over: Mutex::new(None),
        }
    }

    /// The calls so far, which are then cleared.
    pub(crate) fn take_calls(&self) -> Vec<RepoCall> {
        std::mem::take(&mut self.calls.lock().expect("the fake's lock"))
    }

    /// Make the next `load` return `document` as it is, without reading.
    pub(crate) fn hand_over(&self, document: Document) {
        *self.hand_over.lock().expect("the fake's lock") = Some(document);
    }

    fn record(&self, call: RepoCall) {
        self.calls.lock().expect("the fake's lock").push(call);
    }
}

impl ProjectRepository for MemoryRepository {
    fn load(&self, path: &Path) -> Result<LoadedProject, RepositoryError> {
        self.record(RepoCall::Load(path.to_owned()));
        let handed = self.hand_over.lock().expect("the fake's lock").take();
        let document = match handed {
            Some(document) => document,
            None => {
                let mut bytes = Vec::new();
                let mut file = self.fs.open(path).map_err(RepositoryError::Io)?;
                file.read_to_end(&mut bytes).map_err(RepositoryError::Io)?;
                Document::from_json(&bytes).map_err(|e| RepositoryError::Corrupt {
                    detail: e.to_string(),
                })?
            }
        };
        Ok(LoadedProject {
            document,
            read_only: self.read_only,
        })
    }

    fn save(&self, path: &Path, document: &Document) -> Result<(), RepositoryError> {
        self.record(RepoCall::Save(path.to_owned()));
        let bytes = document
            .to_json()
            .map_err(|e| RepositoryError::Io(io::Error::other(e)))?;
        let (first, second) = bytes.as_bytes().split_at(bytes.len() / 2);
        let mut file = self.fs.create(path).map_err(RepositoryError::Io)?;
        file.write_all(first).map_err(RepositoryError::Io)?;
        file.write_all(second).map_err(RepositoryError::Io)?;
        file.sync_all().map_err(RepositoryError::Io)
    }
}

/// An engine whose repository and file system are `repository` and `fs`.
pub(crate) fn engine_on(fs: &MemoryFs, repository: &Arc<MemoryRepository>) -> Gafira {
    let ports = Ports::new(
        Arc::clone(repository) as Arc<dyn ProjectRepository>,
        Arc::new(NoStore),
        Arc::new(fs.clone()),
        Arc::new(NoClock),
        Arc::new(NoNotifier),
        Arc::new(NoLicense),
    );
    Gafira::new(ports, Registries::new(), config(), quiet_channel()).expect("an engine")
}

/// A valid document of two pages and three nodes: page 1 holds nodes 2 and
/// 3, page 4 holds node 5.
pub(crate) fn two_page_document() -> Document {
    let size = PageSize {
        width: Length::new(10.0, Unit::Px),
        height: Length::new(10.0, Unit::Px),
    };
    let page = |id: u64, nodes: &[u64]| {
        let mut page = Page::new(PageId::new(id), format!("p{id}"), size);
        for &node in nodes {
            let id = NodeId::new(node);
            let base = ElementBase::new(id, Placement::new(0.0, 0.0, 1.0, 1.0));
            page.nodes
                .insert(id, Arc::new(Node::new(base, NodeKind::Group(GroupNode {}))));
            page.roots.push(id);
        }
        Arc::new(page)
    };
    Document::from_parts(6, Dpi::DEFAULT, vec![page(1, &[2, 3]), page(4, &[5])])
        .expect("a valid document")
}

/// A document with a DPI of 0, which only a broken adapter hands over.
///
/// Built by a command that copies the document it is applying to after
/// breaking it, then fails, so the history drops its copy and never
/// installs it.
pub(crate) fn invalid_document() -> Document {
    #[derive(Debug)]
    struct Capture(Arc<Mutex<Option<Document>>>);

    impl Command for Capture {
        fn label(&self) -> CommandLabel {
            MessageKey("test.capture")
        }
        fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
            doc.set_dpi(Dpi::new(0));
            *self.0.lock().expect("the slot") = Some(doc.as_doc().clone());
            Err(CommandError::NotApplied)
        }
        fn revert(&mut self, _: &mut DocMut<'_>) -> Result<(), CommandError> {
            Ok(())
        }
        fn touches(&self) -> DirtySet {
            DirtySet::default()
        }
    }

    let slot = Arc::new(Mutex::new(None));
    let history = History::new(Document::default()).expect("a history");
    let preview = history.preview(Box::new(Capture(Arc::clone(&slot))));
    assert!(preview.is_err(), "the capture never installs its copy");
    let document = slot.lock().expect("the slot").take().expect("the copy");
    assert!(document.validate().is_err(), "the copy is invalid");
    document
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_memory_fs_traces_every_write_side_call_and_fails_the_armed_one() {
        let fs = MemoryFs::default();
        fs.fail_at(&[2]);
        let mut file = fs.create(Path::new("/d/a")).expect("a create");
        file.write_all(b"ab").expect("a write");
        assert!(file.write_all(b"cd").is_err(), "the armed write fails");
        file.write_all(b"e").expect("a write");
        file.sync_all().expect("a sync");
        fs.replace(Path::new("/d/a"), Path::new("/d/b"))
            .expect("a replace");
        assert!(fs.remove(Path::new("/d/a")).is_err(), "moved away");
        assert_eq!(fs.file("/d/b"), Some(b"abe".to_vec()));
        let a = || PathBuf::from("/d/a");
        assert_eq!(
            fs.take_trace(),
            vec![
                FsOp::Create(a()),
                FsOp::Write(a(), 2),
                FsOp::Write(a(), 2),
                FsOp::Write(a(), 1),
                FsOp::SyncAll(a()),
                FsOp::Replace(a(), PathBuf::from("/d/b")),
                FsOp::Remove(a()),
            ]
        );
    }
}
