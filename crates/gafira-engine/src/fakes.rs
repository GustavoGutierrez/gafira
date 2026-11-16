//! Private doubles of the ports for the engine's own tests (design P16).
//!
//! They stay inside the crate until a second crate needs the same doubles.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use gafira_doc::contract::CancelToken;
use gafira_doc::msg::{CommandLabel, MessageKey};
use gafira_doc::{
    ContentHash, DocMut, Document, Dpi, ElementBase, GroupNode, Node, NodeId, NodeKind, Page,
    PageId, PageSize, Placement, ProjectId,
};
use gafira_edit::{
    Applied, Command, CommandError, DirtySet, GestureId, History, InsertPage, Property, SetProperty,
};
use gafira_geom::{Length, Unit};
use gafira_ports::common::UnixTime;
use gafira_ports::export::{ImagePixels, PageRenderRequest, PageRenderer, PageRenderers};
use gafira_ports::i18n::{
    FormatLocale, LanguagePreference, Locale, LocaleResolver, LocaleSource, ResolvedLocale,
};
use gafira_ports::license::{LicenseError, LicenseKey, LicenseStatus, LicenseVerifier};
use gafira_ports::render::RenderError;
use gafira_ports::repository::{
    LoadedProject, ProjectRepository, RepositoryError, Version, VersionId, VersionInfo, VersionKind,
};
use gafira_ports::settings::{Settings, SettingsError, SettingsRepository};
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::{
    Clock, FileSystem, FileWrite, IdSource, Notifier, ProcessId, ProcessProbe, ReadSeek,
};
use gafira_raster::{ImageBuf, PixelBuf};

use crate::bus::EventChannel;
use crate::engine::{EngineConfig, Gafira, Ports, Registries, SettingsPorts, SystemPorts};
use crate::event::{EngineEvent, NoopWaker};

pub(crate) struct NoRepository;

impl ProjectRepository for NoRepository {
    fn load(&self, _path: &Path) -> Result<LoadedProject, RepositoryError> {
        Err(RepositoryError::Io(io::Error::other("unused")))
    }

    fn save(&self, _: &Path, _: &Document, _: &[Version]) -> Result<(), RepositoryError> {
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

/// Mints `1, 2, 3, ...` as project ids, so a test can name the id a session
/// or a copy receives (spec 012).
#[derive(Default)]
pub(crate) struct SequentialIds {
    minted: Mutex<u128>,
}

impl SequentialIds {
    /// The `n`-th id this source mints, counting from 1.
    pub(crate) fn nth(n: u128) -> ProjectId {
        ProjectId::from_u128(n)
    }
}

impl IdSource for SequentialIds {
    fn project_id(&self) -> ProjectId {
        let mut minted = lock(&self.minted);
        *minted += 1;
        ProjectId::from_u128(*minted)
    }
}

pub(crate) struct NoNotifier;

/// A process probe the test drives: this process is [`FakeProcesses::current`]
/// on host [`FakeProcesses::HOST`], and only the pids put in `alive` (and
/// the current one) run (spec 028).
pub(crate) struct FakeProcesses {
    pub(crate) current: ProcessId,
    alive: Mutex<BTreeSet<ProcessId>>,
}

impl FakeProcesses {
    /// The host every fake runs on.
    pub(crate) const HOST: &'static str = "here";
    /// The pid of a default fake.
    pub(crate) const PID: ProcessId = ProcessId(1000);

    /// This process is `current`, and `alive` run besides it.
    pub(crate) fn new(current: u32, alive: &[u32]) -> Self {
        Self {
            current: ProcessId(current),
            alive: Mutex::new(alive.iter().copied().map(ProcessId).collect()),
        }
    }
}

impl Default for FakeProcesses {
    fn default() -> Self {
        Self::new(Self::PID.0, &[])
    }
}

impl ProcessProbe for FakeProcesses {
    fn current(&self) -> ProcessId {
        self.current
    }

    fn host(&self) -> String {
        Self::HOST.to_owned()
    }

    fn is_alive(&self, pid: ProcessId) -> bool {
        pid == self.current || lock(&self.alive).contains(&pid)
    }
}

impl Notifier for NoNotifier {
    fn notify(&self, _title: &str, _body: &str) {}
}

/// Renderers nobody asks for: every page is one clear pixel (spec 030).
pub(crate) struct NoPages;

struct ClearPixel;

impl PageRenderer for ClearPixel {
    fn render(
        &self,
        _req: &PageRenderRequest,
        _cancel: &CancelToken,
    ) -> Result<ImageBuf, RenderError> {
        Ok(ImageBuf::new(1, 1, PixelBuf::Rgba8(vec![0; 4]))?)
    }
}

impl PageRenderers for NoPages {
    fn for_document(&self, _document: &Document, _images: ImagePixels) -> Box<dyn PageRenderer> {
        Box::new(ClearPixel)
    }
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

/// `mutex`, held; a poisoned fake is a failed test anyway.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().expect("the fake's lock")
}

/// Settings nobody stores: every load is the defaults, every save succeeds.
pub(crate) struct NoSettings;

impl SettingsRepository for NoSettings {
    fn load(&self) -> Result<Settings, SettingsError> {
        Ok(Settings::default())
    }

    fn save(&self, _settings: &Settings) -> Result<(), SettingsError> {
        Ok(())
    }
}

/// Settings kept in memory; `fail` makes every later call fail.
#[derive(Default)]
pub(crate) struct MemorySettings {
    stored: Mutex<Option<Settings>>,
    corrupt: Mutex<bool>,
    fail_saves: Mutex<bool>,
    saves: Mutex<usize>,
}

impl MemorySettings {
    pub(crate) fn holding(settings: Settings) -> Self {
        let memory = Self::default();
        *lock(&memory.stored) = Some(settings);
        memory
    }

    /// The next loads report a stored value that cannot be read.
    pub(crate) fn corrupt(&self) {
        *lock(&self.corrupt) = true;
    }

    /// The next saves fail with an I/O error.
    pub(crate) fn fail_saves(&self) {
        *lock(&self.fail_saves) = true;
    }

    pub(crate) fn stored(&self) -> Option<Settings> {
        *lock(&self.stored)
    }

    pub(crate) fn saves(&self) -> usize {
        *lock(&self.saves)
    }
}

impl SettingsRepository for MemorySettings {
    fn load(&self) -> Result<Settings, SettingsError> {
        if *lock(&self.corrupt) {
            return Err(SettingsError::Corrupt {
                detail: "not json".to_owned(),
            });
        }
        Ok(lock(&self.stored).unwrap_or_default())
    }

    fn save(&self, settings: &Settings) -> Result<(), SettingsError> {
        if *lock(&self.fail_saves) {
            return Err(SettingsError::Io(io::Error::other("disk full")));
        }
        *lock(&self.saves) += 1;
        *lock(&self.stored) = Some(*settings);
        Ok(())
    }
}

/// D-08 without a system: `Es` and `En` are manual, `System` falls back to
/// English.
pub(crate) struct FallbackResolver;

impl LocaleResolver for FallbackResolver {
    fn resolve(&self, pref: LanguagePreference) -> ResolvedLocale {
        let (locale, source) = match pref {
            LanguagePreference::Es => (Locale::Es, LocaleSource::Manual),
            LanguagePreference::En => (Locale::En, LocaleSource::Manual),
            LanguagePreference::System => (Locale::En, LocaleSource::Fallback),
        };
        ResolvedLocale {
            locale,
            formats: FormatLocale {
                locale,
                region: None,
            },
            source,
        }
    }
}

/// The settings ports of the engines that never touch settings.
pub(crate) fn no_settings() -> SettingsPorts {
    SettingsPorts::new(Arc::new(NoSettings), Arc::new(FallbackResolver))
}

/// An engine with a new document whose settings live in `settings`.
pub(crate) fn engine_with_settings(settings: &Arc<MemorySettings>) -> Gafira {
    let mut ports = six_ports();
    ports.settings = SettingsPorts::new(
        Arc::clone(settings) as Arc<dyn SettingsRepository>,
        Arc::new(FallbackResolver),
    );
    let engine =
        Gafira::new(ports, Registries::new(), config(), quiet_channel()).expect("an engine");
    engine.projects().new_document().expect("a new document");
    assert_eq!(drain(&engine).len(), 1, "the new");
    engine
}

/// A verifier the test scripts: `status` returns the stored state, and each
/// `activate`, `validate` and `deactivate` pops the next scripted outcome.
pub(crate) struct ScriptedLicense {
    status: Mutex<LicenseStatus>,
    outcomes: Mutex<Vec<Result<LicenseStatus, LicenseError>>>,
    nows: Mutex<Vec<UnixTime>>,
}

impl ScriptedLicense {
    /// A verifier in `status`, with `outcomes` for the next network calls in
    /// call order. A successful outcome also becomes the stored state.
    pub(crate) fn new(
        status: LicenseStatus,
        outcomes: Vec<Result<LicenseStatus, LicenseError>>,
    ) -> Self {
        Self {
            status: Mutex::new(status),
            outcomes: Mutex::new(outcomes.into_iter().rev().collect()),
            nows: Mutex::new(Vec::new()),
        }
    }

    /// Every `now` the verifier was called with, in order.
    pub(crate) fn nows(&self) -> Vec<UnixTime> {
        lock(&self.nows).clone()
    }

    fn next(&self) -> Result<LicenseStatus, LicenseError> {
        let outcome = lock(&self.outcomes)
            .pop()
            .expect("a scripted outcome for this call");
        if let Ok(status) = &outcome {
            *lock(&self.status) = status.clone();
        }
        outcome
    }
}

impl LicenseVerifier for ScriptedLicense {
    fn status(&self, now: UnixTime) -> LicenseStatus {
        lock(&self.nows).push(now);
        lock(&self.status).clone()
    }

    fn activate(
        &self,
        _key: &LicenseKey,
        _label: Option<&str>,
    ) -> Result<LicenseStatus, LicenseError> {
        self.next()
    }

    fn validate(&self, now: UnixTime) -> Result<LicenseStatus, LicenseError> {
        lock(&self.nows).push(now);
        self.next()
    }

    fn deactivate(&self) -> Result<(), LicenseError> {
        self.next().map(|_| ())
    }
}

/// An engine whose licence is `license` and whose clock says `now` is
/// [`ManualClock::NOW`]; the licence is not started.
pub(crate) fn engine_with_license(license: Arc<dyn LicenseVerifier>) -> Gafira {
    let mut ports = six_ports();
    ports.license = license;
    ports.clock = Arc::new(ManualClock::at(Duration::ZERO));
    Gafira::new(ports, Registries::new(), config(), quiet_channel()).expect("an engine")
}

/// A content store in memory; the hash is a deterministic digest of the
/// bytes, so the same content always has the same hash. `fail` makes every
/// later `put` fail.
#[derive(Default)]
pub(crate) struct MemoryStore {
    contents: Mutex<BTreeMap<ContentHash, Vec<u8>>>,
    failing: Mutex<bool>,
}

impl MemoryStore {
    /// The hash this store gives `bytes`: FNV-1a over the bytes, spread over
    /// 32 bytes. Not BLAKE3; the engine only needs equal bytes, equal hash.
    pub(crate) fn hash_of(bytes: &[u8]) -> ContentHash {
        let mut out = [0u8; 32];
        for (lane, chunk) in out.chunks_exact_mut(8).enumerate() {
            let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ lane as u64;
            for b in bytes {
                h ^= u64::from(*b);
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
            chunk.copy_from_slice(&h.to_le_bytes());
        }
        ContentHash(out)
    }

    pub(crate) fn fail(&self) {
        *lock(&self.failing) = true;
    }

    pub(crate) fn get(&self, hash: ContentHash) -> Option<Vec<u8>> {
        lock(&self.contents).get(&hash).cloned()
    }

    pub(crate) fn len(&self) -> usize {
        lock(&self.contents).len()
    }
}

impl ContentStore for MemoryStore {
    fn put(&self, src: &mut dyn io::Read) -> Result<ContentHash, StoreError> {
        if *lock(&self.failing) {
            return Err(StoreError::Io(io::Error::other("store full")));
        }
        let mut bytes = Vec::new();
        src.read_to_end(&mut bytes).map_err(StoreError::Io)?;
        let hash = Self::hash_of(&bytes);
        lock(&self.contents).insert(hash, bytes);
        Ok(hash)
    }

    fn open(&self, hash: ContentHash) -> Result<Box<dyn ReadSeek + Send>, StoreError> {
        let bytes = self.get(hash).ok_or(StoreError::NotFound { hash })?;
        Ok(Box::new(io::Cursor::new(bytes)))
    }

    fn contains(&self, hash: ContentHash) -> Result<bool, StoreError> {
        Ok(lock(&self.contents).contains_key(&hash))
    }
}

/// An engine whose files are `fs` and whose content store is `store`; no
/// document is open.
pub(crate) fn engine_with_store(fs: &MemoryFs, store: &Arc<MemoryStore>) -> Gafira {
    let mut ports = six_ports();
    ports.filesystem = Arc::new(fs.clone());
    ports.store = Arc::clone(store) as Arc<dyn ContentStore>;
    Gafira::new(ports, Registries::new(), config(), quiet_channel()).expect("an engine")
}

pub(crate) fn config() -> EngineConfig {
    EngineConfig::new("/data/gafira").expect("a valid data directory")
}

pub(crate) fn six_ports() -> Ports {
    Ports::new(
        Arc::new(NoRepository),
        Arc::new(NoStore),
        SystemPorts::new(
            Arc::new(NoFilesystem),
            Arc::new(NoClock),
            Arc::new(SequentialIds::default()),
            Arc::new(FakeProcesses::default()),
        ),
        Arc::new(NoNotifier),
        Arc::new(NoLicense),
        no_settings(),
        Arc::new(NoPages),
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
    CreateDirAll(PathBuf),
    Create(PathBuf),
    /// The file and the number of bytes of one `write` call.
    Write(PathBuf, usize),
    SyncAll(PathBuf),
    Replace(PathBuf, PathBuf),
    Remove(PathBuf),
}

/// What a test runs inside a save, between the snapshot and the `replace`.
type Hook = Box<dyn FnOnce() + Send>;

#[derive(Default)]
struct FsState {
    files: BTreeMap<PathBuf, Vec<u8>>,
    trace: Vec<FsOp>,
    faults: BTreeSet<usize>,
    before_replace: Option<Hook>,
}

/// An in-memory `FileSystem` that records every `create_dir_all`, `create`, `write`,
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

    /// Run `hook` once, just before the next `replace` is traced, with no
    /// lock of the fake held.
    pub(crate) fn before_replace(&self, hook: impl FnOnce() + Send + 'static) {
        self.state().before_replace = Some(Box::new(hook));
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
        let hook = self.state().before_replace.take();
        if let Some(hook) = hook {
            hook();
        }
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

    /// A file, or a folder that holds one: folders exist through their files.
    fn exists(&self, path: &Path) -> io::Result<bool> {
        let state = self.state();
        Ok(state.files.contains_key(path)
            || state
                .files
                .keys()
                .any(|file| file.starts_with(path) && file != path))
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        self.traced(FsOp::CreateDirAll(path.to_owned()), |_| Ok(()))
    }

    /// The files and the folders right under `path`, each once, in order.
    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        let state = self.state();
        let inside: BTreeSet<PathBuf> = state
            .files
            .keys()
            .filter_map(|file| file.strip_prefix(path).ok())
            .filter_map(|rest| rest.components().next())
            .map(|first| path.join(first))
            .collect();
        Ok(inside.into_iter().collect())
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
        let (document, versions) = match handed {
            Some(document) => (document, Vec::new()),
            None => {
                let mut bytes = Vec::new();
                let mut file = self.fs.open(path).map_err(RepositoryError::Io)?;
                file.read_to_end(&mut bytes).map_err(RepositoryError::Io)?;
                decoded(&bytes).map_err(|detail| RepositoryError::Corrupt { detail })?
            }
        };
        Ok(LoadedProject {
            document,
            read_only: self.read_only,
            versions,
        })
    }

    fn save(
        &self,
        path: &Path,
        document: &Document,
        versions: &[Version],
    ) -> Result<(), RepositoryError> {
        self.record(RepoCall::Save(path.to_owned()));
        let bytes = encoded(document, versions);
        let (first, second) = bytes.split_at(bytes.len() / 2);
        let mut file = self.fs.create(path).map_err(RepositoryError::Io)?;
        file.write_all(first).map_err(RepositoryError::Io)?;
        file.write_all(second).map_err(RepositoryError::Io)?;
        file.sync_all().map_err(RepositoryError::Io)
    }
}

/// The bytes [`MemoryRepository`] writes: the document's canonical JSON,
/// then, per version, a line `id\tkind\tcreated\tschema\tapp\tname` and a
/// line with its document's JSON. Without versions, just the document's
/// JSON, as before spec 031.
pub(crate) fn encoded(document: &Document, versions: &[Version]) -> Vec<u8> {
    let json = |document: &Document| document.to_json().expect("a document serialises");
    let mut text = json(document);
    for version in versions {
        let info = &version.info;
        let kind = match info.kind {
            VersionKind::Named => "named",
            VersionKind::Auto => "auto",
        };
        text.push_str(&format!(
            "\n{}\t{kind}\t{}\t{}\t{}\t{}\n{}",
            info.id.0,
            info.created.0,
            info.schema,
            info.app,
            info.name,
            json(&version.document)
        ));
    }
    text.into_bytes()
}

/// What [`encoded`] wrote.
pub(crate) fn decoded(bytes: &[u8]) -> Result<(Document, Vec<Version>), String> {
    let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
    let mut lines = text.split('\n');
    let parse = |json: &str| Document::from_json(json.as_bytes()).map_err(|e| e.to_string());
    let document = parse(lines.next().unwrap_or_default())?;
    let mut versions = Vec::new();
    while let Some(info) = lines.next() {
        let fields: Vec<&str> = info.splitn(6, '\t').collect();
        let [id, kind, created, schema, app, name] = fields[..] else {
            return Err(format!("not a version line: {info:?}"));
        };
        let number = |field: &str| field.parse::<i64>().map_err(|e| e.to_string());
        versions.push(Version {
            info: VersionInfo {
                id: VersionId(number(id)?.unsigned_abs()),
                name: name.to_owned(),
                kind: if kind == "named" {
                    VersionKind::Named
                } else {
                    VersionKind::Auto
                },
                created: UnixTime(number(created)?),
                app: app.to_owned(),
                schema: u32::try_from(number(schema)?).map_err(|e| e.to_string())?,
            },
            document: Arc::new(parse(lines.next().unwrap_or_default())?),
        });
    }
    Ok((document, versions))
}

/// An engine whose repository and file system are `repository` and `fs`.
pub(crate) fn engine_on(fs: &MemoryFs, repository: &Arc<MemoryRepository>) -> Gafira {
    engine_with_clock(fs, repository, Arc::new(NoClock))
}

/// [`engine_on`], reading every instant from `clock`.
pub(crate) fn engine_with_clock(
    fs: &MemoryFs,
    repository: &Arc<MemoryRepository>,
    clock: Arc<dyn Clock>,
) -> Gafira {
    engine_with_processes(fs, repository, clock, Arc::new(FakeProcesses::default()))
}

/// [`engine_with_clock`], running as the process `processes` says (spec 028).
pub(crate) fn engine_with_processes(
    fs: &MemoryFs,
    repository: &Arc<MemoryRepository>,
    clock: Arc<dyn Clock>,
    processes: Arc<FakeProcesses>,
) -> Gafira {
    let ports = Ports::new(
        Arc::clone(repository) as Arc<dyn ProjectRepository>,
        Arc::new(NoStore),
        SystemPorts::new(
            Arc::new(fs.clone()),
            clock,
            Arc::new(SequentialIds::default()),
            processes,
        ),
        Arc::new(NoNotifier),
        Arc::new(NoLicense),
        no_settings(),
        Arc::new(NoPages),
    );
    Gafira::new(ports, Registries::new(), config(), quiet_channel()).expect("an engine")
}

/// A clock the test moves: `monotonic` is the last instant set, `now` is
/// [`ManualClock::NOW`] until [`ManualClock::set_now`] moves it.
pub(crate) struct ManualClock {
    at: Mutex<Duration>,
    wall: Mutex<UnixTime>,
}

impl ManualClock {
    /// The wall time `now` starts at.
    pub(crate) const NOW: UnixTime = UnixTime(1_700_000_000);

    pub(crate) fn at(at: Duration) -> Self {
        Self {
            at: Mutex::new(at),
            wall: Mutex::new(Self::NOW),
        }
    }

    pub(crate) fn set(&self, at: Duration) {
        *self.at.lock().expect("the fake's lock") = at;
    }

    /// Make `now` return `wall` (spec 031).
    pub(crate) fn set_now(&self, wall: UnixTime) {
        *self.wall.lock().expect("the fake's lock") = wall;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> UnixTime {
        *self.wall.lock().expect("the fake's lock")
    }

    fn monotonic(&self) -> Duration {
        *self.at.lock().expect("the fake's lock")
    }
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
