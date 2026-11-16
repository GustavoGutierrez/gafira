//! The store of the light copies of the media (ARQ-146, FV-22).

mod proxy_support;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use gafira_doc::ContentHash;
use gafira_doc::contract::{CancelToken, NoProgress, Progress};
use gafira_media::{
    DEFAULT_PROXY_BUDGET, ProxyKey, ProxyLevel, ProxyProducer, ProxySource, ProxyStore, ProxyUse,
};
use gafira_ports::common::Bytes;
use gafira_ports::media::MediaError;
use gafira_ports::render::Priority;
use gafira_ports::system::FileSystem;
use gafira_testkit::progress::{ProgressLog, check_progress_contract};
use proxy_support::FakeFileSystem;

/// The user cache of the test.
const CACHE: &str = "/home/user/.cache/gafira";

/// The project of the test: the `.gaf` file.
const PROJECT: &str = "/home/user/Documents/clip.gaf";

/// The hash of the content of a medium.
fn content(seed: u8) -> ContentHash {
    ContentHash([seed; 32])
}

/// The key of the light copy of a medium.
fn key(seed: u8, level: ProxyLevel) -> ProxyKey {
    ProxyKey::of(content(seed), level).expect("the level is a light copy")
}

/// A store under the user cache of the test.
fn new_store(file_system: &FakeFileSystem) -> ProxyStore {
    ProxyStore::open_with_default_budget(file_system, PathBuf::from(CACHE))
        .expect("the store opens")
}

/// A store with this budget, under the user cache of the test.
fn new_store_with_budget(file_system: &FakeFileSystem, bytes: u64) -> ProxyStore {
    ProxyStore::open(file_system, PathBuf::from(CACHE), Bytes(bytes)).expect("the store opens")
}

/// The directory of the copies in the user cache.
fn proxy_dir() -> PathBuf {
    Path::new(CACHE).join("proxies")
}

/// The hex name of a content hash.
fn hex(content: ContentHash) -> String {
    content.0.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The name of the copy of the medium `seed` at `level`.
fn copy_name(seed: u8, level: ProxyLevel) -> String {
    let tag = match level {
        ProxyLevel::P540 => "540p",
        ProxyLevel::P720 => "720p",
        ProxyLevel::Original => "",
    };
    format!("{}-{tag}.mjpeg", hex(content(seed)))
}

/// The path of the copy of the medium `seed` at `level`.
fn copy_path(seed: u8, level: ProxyLevel) -> PathBuf {
    proxy_dir().join(copy_name(seed, level))
}

/// The path of the interrupted copy of the medium `seed` at `level`: the
/// name a generation that was cut short leaves behind.
fn interrupted_path(seed: u8, level: ProxyLevel) -> PathBuf {
    let mut path = copy_path(seed, level).into_os_string();
    path.push(".part");
    PathBuf::from(path)
}

/// Writes `bytes` bytes in the file of `path`.
fn put(file_system: &FakeFileSystem, path: &Path, bytes: usize) {
    let mut file = file_system.create(path).expect("the file is created");
    file.write_all(&vec![7; bytes])
        .expect("the bytes are written");
}

/// A producer of copies: `frames` chunks of the same bytes.
struct Chunks {
    frames: u32,
    chunk: Vec<u8>,
    asked: u32,
}

impl Chunks {
    /// A producer of `frames` chunks of `size` bytes.
    fn new(frames: u32, size: usize) -> Self {
        Self {
            frames,
            chunk: vec![7; size],
            asked: 0,
        }
    }

    /// How many chunks the producer was asked for.
    fn asked(&self) -> u32 {
        self.asked
    }
}

impl ProxyProducer for Chunks {
    /// The frames of the copy.
    fn frames(&self) -> u32 {
        self.frames
    }

    /// The next chunk, or `None` at the end.
    fn next(&mut self) -> Result<Option<Vec<u8>>, MediaError> {
        if self.asked >= self.frames {
            return Ok(None);
        }
        self.asked = self.asked.saturating_add(1);
        Ok(Some(self.chunk.clone()))
    }
}

/// A progress that cancels the token at report `at`, logs every report and
/// records whether the final name of the copy is taken at that moment.
struct CancelAfter<'a> {
    token: &'a CancelToken,
    at: usize,
    file_system: &'a FakeFileSystem,
    final_path: PathBuf,
    reports: Mutex<Vec<f32>>,
    visible: Mutex<Vec<bool>>,
}

impl<'a> CancelAfter<'a> {
    /// A progress that cancels at report `at`, numbering from 1, and watches
    /// the final name of `final_path`.
    fn new(
        token: &'a CancelToken,
        at: usize,
        file_system: &'a FakeFileSystem,
        final_path: PathBuf,
    ) -> Self {
        Self {
            token,
            at,
            file_system,
            final_path,
            reports: Mutex::new(Vec::new()),
            visible: Mutex::new(Vec::new()),
        }
    }

    /// The fractions reported so far, in order.
    fn reports(&self) -> Vec<f32> {
        self.reports
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Whether the final name of the copy existed at every report.
    fn visible(&self) -> Vec<bool> {
        self.visible
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Progress for CancelAfter<'_> {
    /// Logs the fraction and the state of the final name, and cancels the
    /// token at the `at`-th report.
    fn report(&self, fraction: f32) {
        let taken = self.file_system.paths().contains(&self.final_path);
        self.visible
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(taken);
        let mut reports = self.reports.lock().unwrap_or_else(PoisonError::into_inner);
        reports.push(fraction);
        if reports.len() == self.at {
            self.token.cancel();
        }
    }
}

// ARQ-146: a generated light copy appears in the user cache, and the
// `.gaf` holds no copy.
#[test]
fn arq_146_the_copy_lands_in_the_user_cache_and_not_in_the_project() {
    let file_system = FakeFileSystem::with_file(PROJECT, vec![1, 2, 3]);
    let mut store = new_store(&file_system);
    let key = key(1, ProxyLevel::P540);
    let mut producer = Chunks::new(2, 4);

    let report = store
        .generate(
            &file_system,
            &key,
            &mut producer,
            &CancelToken::new(),
            &NoProgress,
        )
        .expect("the copy is generated");

    let path = store.path(&key);
    assert_eq!(report.size, Bytes(8));
    assert_eq!(report.frames, 2);
    assert!(store.contains(&key));
    assert_eq!(
        file_system.read(&path),
        Some(vec![7; 8]),
        "the copy is in the user cache"
    );
    assert!(
        path.starts_with(Path::new(CACHE)),
        "the copy lives in the user cache: {path:?}"
    );
    assert!(
        file_system
            .paths()
            .iter()
            .all(|held| held.starts_with(Path::new(CACHE)) || held == Path::new(PROJECT)),
        "the `.gaf` holds no copy: {:?}",
        file_system.paths()
    );
    assert_eq!(
        file_system.read(Path::new(PROJECT)),
        Some(vec![1, 2, 3]),
        "the project is not touched"
    );
}

// ARQ-146: a generation cancelled half way leaves no half-written final file
// and leaves the cache usable.
// ARQ-43: the generation checks the token per chunk written, the checkpoint
// that bounds the answer of the cancellation. The 100 ms bound itself is not
// measured here: this crate has no bench.
#[test]
fn arq_146_a_cancelled_generation_leaves_no_half_written_file() {
    let file_system = FakeFileSystem::with_file(PROJECT, vec![1, 2, 3]);
    let mut store = new_store(&file_system);
    let key = key(1, ProxyLevel::P540);
    let token = CancelToken::new();
    let cancelling = CancelAfter::new(&token, 1, &file_system, store.path(&key));
    let mut producer = Chunks::new(4, 4);

    let cancelled = store.generate(&file_system, &key, &mut producer, &token, &cancelling);

    assert!(
        matches!(&cancelled, Err(MediaError::Cancelled(_))),
        "a cancelled generation answers the cancelled error: {cancelled:?}"
    );
    assert!(!store.contains(&key));
    assert_eq!(
        file_system.paths(),
        vec![PathBuf::from(PROJECT)],
        "the cache keeps no half-written copy and no temporary file"
    );
    assert_eq!(producer.asked(), 1, "the generation stops at the cancel");
    assert_eq!(cancelling.reports(), vec![0.2], "it reports nothing more");
    assert_eq!(
        cancelling.visible(),
        vec![false],
        "the copy is never visible under its final name while it is written"
    );

    // The cache stays usable: the same copy is generated again.
    let mut again = Chunks::new(4, 4);
    store
        .generate(
            &file_system,
            &key,
            &mut again,
            &CancelToken::new(),
            &NoProgress,
        )
        .expect("the cache is usable after a cancel");

    assert!(store.contains(&key));
    assert_eq!(file_system.read(&store.path(&key)), Some(vec![7; 16]));
}

// ARQ-146: the default budget of the copies is ten gigabytes.
#[test]
fn arq_146_the_default_budget_of_the_copies_is_ten_gigabytes() {
    let ten_gigabytes = Bytes(10 * (1 << 30));

    assert_eq!(DEFAULT_PROXY_BUDGET, ten_gigabytes);
    assert_eq!(new_store(&FakeFileSystem::new()).budget(), ten_gigabytes);
    assert_eq!(
        new_store_with_budget(&FakeFileSystem::new(), 4096).budget(),
        Bytes(4096)
    );
}

// ARQ-146: the budget discards the least used copies.
#[test]
fn arq_146_the_budget_discards_the_least_used_copies() {
    let file_system = FakeFileSystem::new();
    let mut store = new_store_with_budget(&file_system, 8);
    let first = key(1, ProxyLevel::P540);
    let second = key(2, ProxyLevel::P540);
    let third = key(3, ProxyLevel::P540);
    for copy in [first, second] {
        store
            .generate(
                &file_system,
                &copy,
                &mut Chunks::new(1, 4),
                &CancelToken::new(),
                &NoProgress,
            )
            .expect("the copy is generated");
    }
    // The first copy is used, so the second one is the least used one.
    store.read(&first, Path::new(PROJECT), ProxyUse::Preview);

    store
        .generate(
            &file_system,
            &third,
            &mut Chunks::new(1, 4),
            &CancelToken::new(),
            &NoProgress,
        )
        .expect("the third copy is generated");

    assert!(store.bytes() <= store.budget());
    assert!(
        !store.contains(&second),
        "the least used copy is the one discarded"
    );
    assert!(store.contains(&third), "the copy just generated stays");
    assert!(store.contains(&first), "the copy that was used stays");
    assert!(
        !file_system.paths().contains(&store.path(&second)),
        "the discarded copy leaves the user cache"
    );
}

// ARQ-146: a budget that fits no copy keeps none, and the copy does not stay
// in the cache either.
#[test]
fn arq_146_a_budget_of_zero_keeps_no_copy() {
    let file_system = FakeFileSystem::new();
    let mut store = new_store_with_budget(&file_system, 0);
    let key = key(1, ProxyLevel::P540);

    store
        .generate(
            &file_system,
            &key,
            &mut Chunks::new(2, 4),
            &CancelToken::new(),
            &NoProgress,
        )
        .expect("the copy is generated");

    assert_eq!(store.bytes(), Bytes(0));
    assert_eq!(store.len(), 0);
    assert!(
        !file_system.paths().contains(&store.path(&key)),
        "the copy is not left in the user cache"
    );
}

// ARQ-146: an export never reads a light copy; a preview reads one when the
// store holds it.
#[test]
fn arq_146_an_export_reads_the_original_and_never_the_copy() {
    let file_system = FakeFileSystem::new();
    let mut store = new_store(&file_system);
    let copy = key(1, ProxyLevel::P540);
    store
        .generate(
            &file_system,
            &copy,
            &mut Chunks::new(1, 4),
            &CancelToken::new(),
            &NoProgress,
        )
        .expect("the copy is generated");
    let original = Path::new(PROJECT);

    let exported = store.read(&copy, original, ProxyUse::Export);
    assert_eq!(
        exported,
        ProxySource {
            level: ProxyLevel::Original,
            path: PathBuf::from(PROJECT),
        },
        "the export reads the original and keeps the resolution of the original"
    );

    let previewed = store.read(&copy, original, ProxyUse::Preview);
    assert_eq!(
        previewed,
        ProxySource {
            level: ProxyLevel::P540,
            path: store.path(&copy),
        }
    );

    // Without a copy the preview reads the original too.
    let missing = key(2, ProxyLevel::P720);
    assert_eq!(
        store.read(&missing, original, ProxyUse::Preview).level,
        ProxyLevel::Original
    );
}

// ARQ-146 + ARQ-139: the generation is a long operation that reports a
// fraction that never falls, stays in `[0, 1]` and reaches `1.0` only on
// success, and that stops with the cancelled error when the caller cancels.
#[test]
fn arq_146_the_generation_follows_the_progress_contract() {
    // The checker places the cancel in reports 1, n/2 and n-1, so a
    // generation that reported only its end would not be checked at all.
    let log = ProgressLog::new();
    let file_system = FakeFileSystem::new();
    new_store(&file_system)
        .generate(
            &file_system,
            &key(1, ProxyLevel::P540),
            &mut Chunks::new(4, 2),
            &CancelToken::new(),
            &log,
        )
        .expect("the copy is generated");
    assert_eq!(
        log.fractions().len(),
        5,
        "the generation reports once per frame and once at the end"
    );

    let contract = check_progress_contract(0.25, |cancel, progress| {
        let file_system = FakeFileSystem::new();
        let mut store = new_store(&file_system);
        let mut producer = Chunks::new(4, 2);
        match store.generate(
            &file_system,
            &key(1, ProxyLevel::P540),
            &mut producer,
            cancel,
            progress,
        ) {
            Ok(_) => Ok(false),
            Err(MediaError::Cancelled(_)) => Ok(true),
            Err(other) => Err(other),
        }
    })
    .expect("the contract check itself does not fail");

    assert_eq!(contract, Ok(()));
}

// ARQ-146: the generation runs in the background and its fraction follows
// the frames done over the frames planned.
#[test]
fn arq_146_the_generation_runs_in_the_background_with_progress() {
    let file_system = FakeFileSystem::new();
    let mut store = new_store(&file_system);
    let log = ProgressLog::new();

    let report = store
        .generate(
            &file_system,
            &key(1, ProxyLevel::P540),
            &mut Chunks::new(4, 2),
            &CancelToken::new(),
            &log,
        )
        .expect("the copy is generated");

    assert_eq!(report.priority, Priority::Background);
    assert_eq!(report.size, Bytes(8));
    assert_eq!(
        log.fractions(),
        vec![0.2, 0.4, 0.6, 0.8, 1.0],
        "the fraction is the frames done over the frames planned, and 1.0 is the end"
    );
}

// FV-22: the store of a session adopts the copies of the previous ones: the
// bytes they take come out of the budget of the session.
#[test]
fn fv_22_the_store_adopts_the_copies_of_the_previous_sessions() {
    let file_system = FakeFileSystem::new();
    let first = key(1, ProxyLevel::P540);
    let second = key(2, ProxyLevel::P720);
    put(&file_system, &copy_path(1, ProxyLevel::P540), 10);
    put(&file_system, &copy_path(2, ProxyLevel::P720), 6);

    let store = ProxyStore::open_with_default_budget(&file_system, PathBuf::from(CACHE))
        .expect("the store opens");

    assert_eq!(
        store.bytes(),
        Bytes(16),
        "the adopted copies take their bytes out of the budget of the session"
    );
    assert_eq!(
        store.len(),
        2,
        "the two copies of the previous session are held"
    );
    assert!(
        store.contains(&first),
        "the copy of the previous session is held"
    );
    assert!(
        store.contains(&second),
        "the copy of the previous session is held"
    );
}

// ARQ-146: a generation that was cut short leaves a `*.part` file behind: the
// store removes it when it opens, so no garbage is kept between sessions.
#[test]
fn arq_146_the_store_removes_the_interrupted_copies_when_it_opens() {
    let file_system = FakeFileSystem::new();
    let interrupted = interrupted_path(1, ProxyLevel::P540);
    put(&file_system, &interrupted, 5);
    put(&file_system, &copy_path(2, ProxyLevel::P540), 4);

    let store =
        ProxyStore::open(&file_system, PathBuf::from(CACHE), Bytes(1024)).expect("the store opens");

    assert!(
        !file_system.paths().contains(&interrupted),
        "the interrupted copy leaves the user cache"
    );
    assert_eq!(store.len(), 1, "the complete copy is adopted");
    assert_eq!(store.bytes(), Bytes(4));
}

// FV-22: the adopted copies have no known order of use, so they are treated
// as the oldest ones: the eviction of the session prefers them.
#[test]
fn fv_22_the_eviction_prefers_the_adopted_copies() {
    let file_system = FakeFileSystem::new();
    let adopted = key(1, ProxyLevel::P540);
    put(&file_system, &copy_path(1, ProxyLevel::P540), 8);

    let mut store =
        ProxyStore::open(&file_system, PathBuf::from(CACHE), Bytes(8)).expect("the store opens");
    let fresh = key(2, ProxyLevel::P540);
    store
        .generate(
            &file_system,
            &fresh,
            &mut Chunks::new(1, 4),
            &CancelToken::new(),
            &NoProgress,
        )
        .expect("the copy is generated");

    assert!(
        !store.contains(&adopted),
        "the adopted copy is the oldest one and is the first to leave"
    );
    assert!(store.contains(&fresh), "the copy of the session stays");
    assert!(
        !file_system
            .paths()
            .contains(&copy_path(1, ProxyLevel::P540)),
        "the evicted copy leaves the user cache"
    );
    assert!(store.bytes() <= store.budget());
}

// FV-22: the store respects its budget right after it adopts the copies of
// the previous sessions.
#[test]
fn fv_22_the_store_respects_the_budget_after_adopting() {
    let file_system = FakeFileSystem::new();
    put(&file_system, &copy_path(1, ProxyLevel::P540), 8);
    put(&file_system, &copy_path(2, ProxyLevel::P720), 8);

    let store =
        ProxyStore::open(&file_system, PathBuf::from(CACHE), Bytes(10)).expect("the store opens");

    assert!(
        store.bytes() <= store.budget(),
        "the adopted copies do not take the store over its budget"
    );
    assert_eq!(store.len(), 1, "the copy that does not fit leaves");
    assert_eq!(
        file_system.paths().len(),
        1,
        "the evicted copy leaves the user cache"
    );
}

// ARQ-146: the name of a copy is the content and the level, so the two
// levels of a medium are two files in the user cache.
#[test]
fn arq_146_the_two_levels_of_a_medium_are_two_copies() {
    let file_system = FakeFileSystem::new();
    let mut store = new_store(&file_system);
    let p540 = key(1, ProxyLevel::P540);
    let p720 = key(1, ProxyLevel::P720);
    for (copy, size) in [(p540, 2_usize), (p720, 3_usize)] {
        store
            .generate(
                &file_system,
                &copy,
                &mut Chunks::new(1, size),
                &CancelToken::new(),
                &NoProgress,
            )
            .expect("the copy is generated");
    }

    assert_ne!(store.path(&p540), store.path(&p720));
    assert_eq!(file_system.read(&store.path(&p540)), Some(vec![7; 2]));
    assert_eq!(file_system.read(&store.path(&p720)), Some(vec![7; 3]));
    assert_eq!(store.len(), 2);
    // The same medium gives the same name in another session of the cache.
    assert_eq!(new_store(&file_system).path(&p540), store.path(&p540));
    // The original is not a light copy, so it has no key and no name.
    assert_eq!(ProxyKey::of(content(1), ProxyLevel::Original), None);
}
