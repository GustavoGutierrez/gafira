//! The store of the light copies of the media (ARQ-146, FV-22).
//!
//! A light copy is an MJPEG intra copy at 540p or 720p of a medium that is
//! expensive to decode (FV-22). The copies live in the user cache, outside
//! the `.gafira` (02 §7.1), under a byte budget of ten gigabytes by default
//! that discards the least used ones, and the export never reads one
//! (ARQ-146, FV-22 (b)).
//!
//! The store writes through `ports::FileSystem` and never through the file
//! system of the standard library (ADR 0016, ARQ-02). The MJPEG codec is
//! layer 4, so the crate does not encode: the caller produces the chunks of
//! the copy and the store writes them, at [`Priority::Background`], with
//! progress and cancellation (02 §6.3, ARQ-139).

use std::collections::BTreeMap;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use gafira_doc::ContentHash;
use gafira_doc::contract::{CancelToken, Progress};
use gafira_ports::common::Bytes;
use gafira_ports::media::MediaError;
use gafira_ports::render::Priority;
use gafira_ports::system::FileSystem;

use crate::cache::ProxyLevel;

/// The default budget of the copies: ten gigabytes (ARQ-146, FV-22).
pub const DEFAULT_PROXY_BUDGET: Bytes = Bytes(10 << 30);

/// The directory of the copies inside the user cache (FV-22).
const PROXY_DIR: &str = "proxies";

/// The suffix of the name of a light copy: MJPEG intra (FV-22).
const PROXY_SUFFIX: &str = ".mjpeg";

/// The suffix of a copy that is still being written (ARQ-146).
const TEMP_SUFFIX: &str = ".part";

/// The key of one light copy (ARQ-146, FV-22).
///
/// A copy is named by the hash of the content of the file and the level,
/// never by the id of the resource: the store lives outside the project, so
/// the same file gives the same copy in any project (02 §6.5, §7.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProxyKey {
    content: ContentHash,
    level: ProxyLevel,
}

impl ProxyKey {
    /// The key of the light copy of `content` at `level`.
    ///
    /// `None` when `level` is [`ProxyLevel::Original`]: the original is not
    /// a light copy, and the export reads it instead of one (FV-22 (b)).
    #[must_use]
    pub const fn of(content: ContentHash, level: ProxyLevel) -> Option<Self> {
        match level {
            ProxyLevel::Original => None,
            ProxyLevel::P540 | ProxyLevel::P720 => Some(Self { content, level }),
        }
    }

    /// The hash of the content of the medium.
    #[must_use]
    pub const fn content(&self) -> ContentHash {
        self.content
    }

    /// The level of the copy.
    #[must_use]
    pub const fn level(&self) -> ProxyLevel {
        self.level
    }
}

/// What a caller reads a medium for (ARQ-146).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProxyUse {
    /// Scrubbing and playback: the light copy when the store holds one.
    Preview,
    /// The export: always the original (FV-22 (b), V-3).
    Export,
}

/// The file a caller reads a medium from (ARQ-146).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProxySource {
    /// The level of the file: the original, or the copy that was read.
    pub level: ProxyLevel,
    /// The path of the file.
    pub path: PathBuf,
}

/// Produces the bytes of one light copy (ARQ-146, FV-22).
///
/// The MJPEG codec is layer 4, so the crate does not encode: the caller
/// decodes the original, encodes it and hands the chunks over, and the store
/// writes them, reports the progress and checks the cancellation (02 §6.3).
/// One chunk is one frame, because the checkpoint of the cancel token is per
/// frame (ARQ-139).
pub trait ProxyProducer {
    /// The frames the copy will hold: the units the progress counts.
    fn frames(&self) -> u32;

    /// The next chunk of the copy, or `None` when the copy is complete.
    ///
    /// # Errors
    /// [`MediaError`] when this caller cannot produce the chunk.
    fn next(&mut self) -> Result<Option<Vec<u8>>, MediaError>;
}

/// What the generation of a light copy produced (ARQ-146).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProxyReport {
    /// The bytes of the copy in the cache.
    pub size: Bytes,
    /// The frames of the copy.
    pub frames: u32,
    /// The priority the generation ran at: `Background` (02 §6.3).
    pub priority: Priority,
}

/// One stored copy and what the store knows about it.
struct Entry {
    /// The bytes it takes.
    size: Bytes,
    /// The tick of its last use: the LRU order.
    used: u64,
}

/// The store of the light copies of the media (ARQ-146, FV-22).
///
/// The store keeps the copies in the user cache, under a byte budget, and
/// frees the least used ones when it goes over it. It writes nothing else:
/// the `.gafira` holds no copy (02 §7.1).
pub struct ProxyStore {
    /// The root of the user cache: a parameter, never read from the system
    /// (ARQ-138).
    root: PathBuf,
    /// The budget of the copies.
    budget: Bytes,
    /// The bytes the copies take now.
    size: Bytes,
    /// The copies, by key.
    entries: BTreeMap<ProxyKey, Entry>,
    /// The keys, by the tick of their last use: the LRU order.
    recency: BTreeMap<u64, ProxyKey>,
    /// The tick of the last use.
    tick: u64,
}

impl ProxyStore {
    /// Opens the store of the light copies under `root` (ARQ-146, FV-22).
    ///
    /// The store of a session adopts the copies of the previous ones: the
    /// files of the proxy directory are measured and counted against the
    /// budget, so a copy written by a previous session still takes its bytes
    /// out of the ten gigabytes. The interrupted copies, the `*.part` files
    /// of a generation that was cut short, are removed: no garbage is kept
    /// between sessions.
    ///
    /// The adopted copies have no known order of use, so they are treated as
    /// the oldest ones: the eviction of the session prefers them.
    ///
    /// # Errors
    ///
    /// [`MediaError::Io`] when the file system refuses the directory or a
    /// file of it.
    pub fn open(
        file_system: &dyn FileSystem,
        root: PathBuf,
        budget: Bytes,
    ) -> Result<Self, MediaError> {
        let mut store = Self {
            root,
            budget,
            size: Bytes(0),
            entries: BTreeMap::new(),
            recency: BTreeMap::new(),
            tick: 0,
        };
        store.adopt(file_system)?;
        Ok(store)
    }

    /// Opens the store of the light copies under `root` with the default
    /// budget of ten gigabytes (ARQ-146, FV-22).
    ///
    /// # Errors
    ///
    /// [`MediaError::Io`] when the file system refuses the directory or a
    /// file of it.
    pub fn open_with_default_budget(
        file_system: &dyn FileSystem,
        root: PathBuf,
    ) -> Result<Self, MediaError> {
        Self::open(file_system, root, DEFAULT_PROXY_BUDGET)
    }

    /// The root of the store: the user cache.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The budget of the copies.
    #[must_use]
    pub const fn budget(&self) -> Bytes {
        self.budget
    }

    /// The bytes the copies take now.
    #[must_use]
    pub const fn bytes(&self) -> Bytes {
        self.size
    }

    /// How many copies the store holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when the store holds no copy.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// True when the store holds the copy of `key`.
    #[must_use]
    pub fn contains(&self, key: &ProxyKey) -> bool {
        self.entries.contains_key(key)
    }

    /// The path of the copy of `key` in the user cache (ARQ-146).
    ///
    /// The name is the hash of the content and the level, so the copy of a
    /// medium is the same file in any project and the two levels never share
    /// a file (02 §6.5, §7.1).
    #[must_use]
    pub fn path(&self, key: &ProxyKey) -> PathBuf {
        self.directory().join(name(key))
    }

    /// The file a caller reads a medium from (ARQ-146, FV-22 (b)).
    ///
    /// A preview reads the light copy when the store holds one, so the
    /// frames come from the level the source names. An export always reads
    /// the original: a light copy has another resolution than the original,
    /// so the export never uses one (FV-22 (b), V-3).
    pub fn read(&mut self, key: &ProxyKey, original: &Path, use_of: ProxyUse) -> ProxySource {
        let original_source = ProxySource {
            level: ProxyLevel::Original,
            path: original.to_path_buf(),
        };
        if use_of == ProxyUse::Export || !self.entries.contains_key(key) {
            return original_source;
        }
        self.touch(key);
        ProxySource {
            level: key.level,
            path: self.path(key),
        }
    }

    /// Generates the light copy of `key` (ARQ-146, FV-22).
    ///
    /// The copy is written under a temporary name in the proxy directory of
    /// the user cache and moved to its final name only once it is complete,
    /// so a cancelled generation leaves no half-written copy and the cache
    /// stays usable (ARQ-146). Nothing is written outside the cache: the
    /// `.gafira` holds no copy (02 §7.1).
    ///
    /// The generation is a long operation: it runs at
    /// [`Priority::Background`] (02 §6.3), checks the cancel token once per
    /// frame and reports the frames done over the frames planned, which
    /// only reaches `1.0` when the copy is in place (ARQ-139).
    ///
    /// # Errors
    ///
    /// [`MediaError::Cancelled`] when the caller cancels the generation and
    /// [`MediaError::Io`] when the file system refuses the copy.
    pub fn generate(
        &mut self,
        file_system: &dyn FileSystem,
        key: &ProxyKey,
        producer: &mut dyn ProxyProducer,
        cancel: &CancelToken,
        progress: &dyn Progress,
    ) -> Result<ProxyReport, MediaError> {
        file_system
            .create_dir_all(&self.directory())
            .map_err(MediaError::Io)?;
        let final_path = self.path(key);
        let temporary = temporary_path(&final_path);
        let frames = producer.frames();
        // The finish of the copy is one more unit of work, so the fraction
        // of the frames done never reaches 1.0 before the copy is in place
        // (ARQ-139).
        let planned = frames.saturating_add(1);
        let written =
            self.write_chunks(file_system, &temporary, producer, planned, cancel, progress);
        let size = match written {
            Ok(size) => size,
            Err(error) => {
                // The temporary name never reaches the final one: the cache
                // keeps no half-written copy (ARQ-146).
                let _ = file_system.remove(&temporary);
                return Err(error);
            }
        };
        if let Err(error) = file_system.replace(&temporary, &final_path) {
            let _ = file_system.remove(&temporary);
            return Err(MediaError::Io(error));
        }
        self.insert(file_system, key, size);
        progress.report(1.0);
        Ok(ProxyReport {
            size,
            frames,
            priority: Priority::Background,
        })
    }

    /// Writes the chunks of the copy to `path` (ARQ-146).
    ///
    /// One checkpoint per frame: the token is checked before every chunk and
    /// the progress is reported after it, so a cancelled generation reports
    /// nothing more and never reaches `1.0` (ARQ-139).
    fn write_chunks(
        &self,
        file_system: &dyn FileSystem,
        path: &Path,
        producer: &mut dyn ProxyProducer,
        planned: u32,
        cancel: &CancelToken,
        progress: &dyn Progress,
    ) -> Result<Bytes, MediaError> {
        let mut file = file_system.create(path).map_err(MediaError::Io)?;
        let mut size = 0_u64;
        for frame in 0..producer.frames() {
            cancel.check()?;
            let Some(chunk) = producer.next()? else {
                break;
            };
            file.write_all(&chunk).map_err(MediaError::Io)?;
            size = size.saturating_add(chunk.len() as u64);
            progress.report(fraction(frame.saturating_add(1), planned));
        }
        file.sync_all().map_err(MediaError::Io)?;
        // The last check is before the copy is moved to its final name, so
        // a cancel after the last frame never reaches `1.0` (ARQ-139).
        cancel.check()?;
        Ok(Bytes(size))
    }

    /// Adopts the copies of the previous sessions (ARQ-146, FV-22).
    ///
    /// The interrupted copies, the `*.part` files, are removed and the
    /// complete ones are measured and counted against the budget. A file of
    /// the directory that is not the name of a copy is left alone: it is not
    /// a copy of this store.
    fn adopt(&mut self, file_system: &dyn FileSystem) -> Result<(), MediaError> {
        file_system
            .create_dir_all(&self.directory())
            .map_err(MediaError::Io)?;
        let entries = file_system
            .read_dir(&self.directory())
            .map_err(MediaError::Io)?;
        let mut copies: Vec<(ProxyKey, Bytes)> = Vec::new();
        for entry in entries {
            let Some(file_name) = entry.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if file_name.ends_with(TEMP_SUFFIX) {
                // A generation that was cut short leaves this file behind: it
                // is not a copy and it is of no use (ARQ-146).
                let _ = file_system.remove(&entry);
                continue;
            }
            let Some(key) = key_of_name(file_name) else {
                continue;
            };
            let size = size_of(file_system, &entry)?;
            copies.push((key, size));
        }
        // The order of use of the adopted copies is unknown, so they are
        // taken in the order of their keys and treated as the oldest ones:
        // the eviction of the session prefers them.
        copies.sort_by(|left, right| left.0.cmp(&right.0));
        for (key, size) in copies {
            self.tick = self.tick.saturating_add(1);
            let used = self.tick;
            self.size = Bytes(self.size.0.saturating_add(size.0));
            self.entries.insert(key, Entry { size, used });
            self.recency.insert(used, key);
        }
        self.evict_to(file_system, self.budget);
        Ok(())
    }

    /// Stores the copy of `key` and frees the least used ones over the
    /// budget (ARQ-146, FV-22).
    fn insert(&mut self, file_system: &dyn FileSystem, key: &ProxyKey, size: Bytes) {
        if let Some(old) = self.entries.remove(key) {
            self.size = Bytes(self.size.0.saturating_sub(old.size.0));
            self.recency.remove(&old.used);
        }
        self.tick = self.tick.saturating_add(1);
        let used = self.tick;
        self.size = Bytes(self.size.0.saturating_add(size.0));
        self.entries.insert(*key, Entry { size, used });
        self.recency.insert(used, *key);
        self.evict_to(file_system, self.budget);
    }

    /// Marks the copy of `key` as the most recently used one.
    fn touch(&mut self, key: &ProxyKey) {
        let Some(old) = self.entries.get(key).map(|entry| entry.used) else {
            return;
        };
        self.recency.remove(&old);
        self.tick = self.tick.saturating_add(1);
        let now = self.tick;
        self.recency.insert(now, *key);
        if let Some(entry) = self.entries.get_mut(key) {
            entry.used = now;
        }
    }

    /// Frees the least used copies until the store holds at most `budget`
    /// bytes (ARQ-146, FV-22).
    ///
    /// The copy leaves the cache as well: a discarded copy is not left
    /// behind in the user cache (FV-22 (c)).
    fn evict_to(&mut self, file_system: &dyn FileSystem, budget: Bytes) {
        while self.size.0 > budget.0 {
            let Some((used, key)) = self.recency.iter().next().map(|(used, key)| (*used, *key))
            else {
                break;
            };
            self.recency.remove(&used);
            if let Some(entry) = self.entries.remove(&key) {
                self.size = Bytes(self.size.0.saturating_sub(entry.size.0));
                let _ = file_system.remove(&self.path(&key));
            }
        }
    }

    /// The directory of the copies inside the user cache (FV-22).
    fn directory(&self) -> PathBuf {
        self.root.join(PROXY_DIR)
    }
}

/// The name of the copy of `key` inside the user cache (FV-22).
///
/// The hash of the content names the medium and the level names the copy, so
/// the two levels of the same medium are two files.
fn name(key: &ProxyKey) -> String {
    // A key is never built at the original level (`ProxyKey::of`), so the
    // tag of the level is always there.
    let tag = match height(key.level) {
        Some(height) => format!("{height}p"),
        None => String::new(),
    };
    format!("{}-{tag}{PROXY_SUFFIX}", hex(&key.content))
}

/// The key of the copy a file name names, or `None` when the name is not the
/// name of a copy of this store (ARQ-146, FV-22).
///
/// The name of a copy is the hash of the content of the medium, the tag of
/// the level and the extension (02 §6.5, §7.1).
fn key_of_name(file_name: &str) -> Option<ProxyKey> {
    let (hash, tag) = file_name.strip_suffix(PROXY_SUFFIX)?.split_once('-')?;
    let level = match tag {
        "540p" => ProxyLevel::P540,
        "720p" => ProxyLevel::P720,
        _ => return None,
    };
    let content = content_of_hex(hash)?;
    ProxyKey::of(ContentHash(content), level)
}

/// The content hash a hex name holds, or `None` when the name is not one.
fn content_of_hex(hash: &str) -> Option<[u8; 32]> {
    if hash.len() != 64 {
        return None;
    }
    let mut content = [0_u8; 32];
    for (index, pair) in hash.as_bytes().chunks(2).enumerate() {
        content[index] = (digit(pair[0])? << 4) | digit(pair[1])?;
    }
    Some(content)
}

/// The value of a hex digit, or `None` when the byte is not one.
fn digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

/// The size of a file of the user cache, in bytes (ARQ-146, FV-22).
///
/// The file is opened and its end is sought, so the size comes from the
/// port and the file is not read (ARQ-02).
fn size_of(file_system: &dyn FileSystem, path: &Path) -> Result<Bytes, MediaError> {
    let mut file = file_system.open(path).map_err(MediaError::Io)?;
    let length = file.seek(SeekFrom::End(0)).map_err(MediaError::Io)?;
    Ok(Bytes(length))
}

/// The height in lines of a light copy; `None` for the original (FV-22).
fn height(level: ProxyLevel) -> Option<u32> {
    match level {
        ProxyLevel::Original => None,
        ProxyLevel::P540 => Some(540),
        ProxyLevel::P720 => Some(720),
    }
}

/// The hex name of a content hash: 64 lowercase characters (FV-22).
fn hex(content: &ContentHash) -> String {
    const DIGITS: [char; 16] = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
    ];
    let mut name = String::with_capacity(64);
    for byte in content.0 {
        name.push(DIGITS[usize::from(byte >> 4)]);
        name.push(DIGITS[usize::from(byte & 0x0f)]);
    }
    name
}

/// The name a copy carries while it is being written (ARQ-146).
///
/// The copy is written under this name and moved to its final one only when
/// it is complete, so a cancelled generation leaves no half-written copy
/// under the name the store reads (02 §7.2 step 3).
fn temporary_path(final_path: &Path) -> PathBuf {
    let mut name = final_path.as_os_str().to_os_string();
    name.push(TEMP_SUFFIX);
    PathBuf::from(name)
}

/// The fraction of the frames done over the frames planned (ARQ-139).
///
/// The units planned are the frames plus the finish of the copy, so the
/// fraction stays below `1.0` until the copy is in place.
fn fraction(done: u32, planned: u32) -> f32 {
    done as f32 / planned.max(1) as f32
}
