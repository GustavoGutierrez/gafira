//! The cache of the decoded frames of the media (ARQ-145).
//!
//! The cache is an LRU with a budget of RAM: it holds the decoded frames of
//! the media, keyed by the content of the medium, the proxy level and the
//! index of the frame in that level (02 §6.5). The budget counts the bytes
//! of the frames, and the cache counts inside the 1,5 GB of D-03, so the
//! `MemoryGovernor` trims it first (ARQ-44, O2).

use std::collections::BTreeMap;
use std::sync::Arc;

use gafira_doc::ContentHash;
use gafira_ports::assets::ProviderAssetId;
use gafira_ports::common::Bytes;
use gafira_ports::media::VideoFrame;
use gafira_raster::PixelBuf;
use gafira_time::Flicks;

/// The default budget of the cache: one gigabyte (ARQ-145, 02 §6.5, FIJADA).
pub const DEFAULT_BUDGET: Bytes = Bytes(1 << 30);

/// The proxy level of a frame (ARQ-145, 02 §6.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProxyLevel {
    /// The original file: not a light copy.
    Original,
    /// The light copy at 540p.
    P540,
    /// The light copy at 720p.
    P720,
}

/// The key of one cached frame (ARQ-145, 02 §6.5).
///
/// The three parts are the hash of the content of the medium, the proxy
/// level and the index of the frame in that level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameKey {
    content: ContentHash,
    level: ProxyLevel,
    index: u32,
}

impl FrameKey {
    /// The key of the frame `index` of `content` at `level`.
    #[must_use]
    pub const fn new(content: ContentHash, level: ProxyLevel, index: u32) -> Self {
        Self {
            content,
            level,
            index,
        }
    }

    /// The hash of the content of the medium.
    #[must_use]
    pub const fn content(&self) -> ContentHash {
        self.content
    }

    /// The proxy level of the frame.
    #[must_use]
    pub const fn level(&self) -> ProxyLevel {
        self.level
    }

    /// The index of the frame in that level.
    #[must_use]
    pub const fn index(&self) -> u32 {
        self.index
    }
}

/// How the caller names a medium (ARQ-145).
///
/// The medium has two names: the id it carries inside the project and the
/// hash of the content of the file. The cache keys by the hash, so the same
/// file gives the same cached frame in any project (02 §6.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediumRef {
    /// The id of the resource inside the project: the cache ignores it.
    pub asset: ProviderAssetId,
    /// The hash of the content of the file.
    pub content: ContentHash,
}

impl MediumRef {
    /// A medium with an id inside the project and a hash of its content.
    #[must_use]
    pub fn new(asset: ProviderAssetId, content: ContentHash) -> Self {
        Self { asset, content }
    }
}

/// What the caller asks the cache for (ARQ-145).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameRequest {
    /// The medium the frame belongs to.
    pub medium: MediumRef,
    /// The proxy level of the frame.
    pub level: ProxyLevel,
    /// The index of the frame in that level.
    pub index: u32,
    /// The time of the frame.
    pub time: Flicks,
}

impl FrameRequest {
    /// A request for the frame `index` of `medium` at `level` and `time`.
    #[must_use]
    pub fn new(medium: MediumRef, level: ProxyLevel, index: u32, time: Flicks) -> Self {
        Self {
            medium,
            level,
            index,
            time,
        }
    }

    /// The key the request resolves to.
    #[must_use]
    pub fn key(&self) -> FrameKey {
        FrameKey::new(self.medium.content, self.level, self.index)
    }
}

/// The direction the head of the scrub moves in (ARQ-145, 02 §6.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeadDirection {
    /// The head moves forward in time.
    Forward,
    /// The head moves backward in time.
    Backward,
}

/// What the cache answers to a request (ARQ-145).
#[derive(Clone, Debug, PartialEq)]
pub enum FrameHit {
    /// The requested frame is in the cache.
    Exact(Arc<VideoFrame>),
    /// The requested frame is not in the cache: this is the nearest one that
    /// is, and the exact one is left to the decode.
    Nearest(Arc<VideoFrame>),
    /// The cache holds nothing of that medium at that level.
    Empty,
}

/// One cached frame and what the cache knows about it.
struct Entry {
    /// The frame itself.
    frame: Arc<VideoFrame>,
    /// The bytes it holds.
    size: Bytes,
    /// The tick of its last use: the LRU order.
    used: u64,
}

/// The cache of the decoded frames of the media (ARQ-145).
///
/// The cache holds the frames of the media under a budget of RAM and frees
/// the least recently used ones when it goes over it. `bytes()` tells how
/// much it holds and `trim(budget)` frees it until it holds at most that
/// budget, the two in bytes: the `MemoryGovernor` asks the caches that way
/// (ARQ-44).
pub struct FrameCache {
    /// The budget of the cache.
    budget: Bytes,
    /// The bytes the cache holds now.
    size: Bytes,
    /// The frames, by key.
    entries: BTreeMap<FrameKey, Entry>,
    /// The keys, by the tick of their last use: the LRU order.
    recency: BTreeMap<u64, FrameKey>,
    /// The tick of the last use.
    tick: u64,
}

impl FrameCache {
    /// A cache with a budget of `budget` bytes.
    #[must_use]
    pub fn new(budget: Bytes) -> Self {
        Self {
            budget,
            size: Bytes(0),
            entries: BTreeMap::new(),
            recency: BTreeMap::new(),
            tick: 0,
        }
    }

    /// The budget of the cache.
    #[must_use]
    pub const fn budget(&self) -> Bytes {
        self.budget
    }

    /// The bytes the cache holds now (ARQ-44).
    #[must_use]
    pub const fn bytes(&self) -> Bytes {
        self.size
    }

    /// How many frames the cache holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when the cache holds no frame.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Stores a decoded frame (ARQ-145).
    ///
    /// The frame takes the place of any other frame with the same key, and
    /// the cache frees the least recently used frames when it goes over its
    /// budget.
    pub fn put(&mut self, request: &FrameRequest, frame: VideoFrame) {
        let key = request.key();
        if let Some(old) = self.entries.remove(&key) {
            self.size = Bytes(self.size.0.saturating_sub(old.size.0));
            self.recency.remove(&old.used);
        }
        let size = frame_size(&frame);
        self.tick = self.tick.saturating_add(1);
        let used = self.tick;
        self.size = Bytes(self.size.0.saturating_add(size.0));
        self.entries.insert(
            key,
            Entry {
                frame: Arc::new(frame),
                size,
                used,
            },
        );
        self.recency.insert(used, key);
        self.evict_to(self.budget);
    }

    /// The frame the request asks for (ARQ-145).
    ///
    /// The frame that is cached comes back as it is. A frame that is not
    /// cached is answered with the nearest frame that is, and the exact one
    /// is left to the decode.
    pub fn lookup(&mut self, request: &FrameRequest) -> FrameHit {
        let key = request.key();
        if let Some(entry) = self.entries.get(&key) {
            let frame = Arc::clone(&entry.frame);
            self.touch(&key);
            return FrameHit::Exact(frame);
        }
        match self.nearest(request) {
            Some(frame) => FrameHit::Nearest(frame),
            None => FrameHit::Empty,
        }
    }

    /// The frames to prefetch next (ARQ-145, 02 §6.5).
    ///
    /// The prefetch follows the direction the head of the scrub moves: the
    /// frames ahead of it, in that direction, that the cache does not hold
    /// yet. The caller decodes them and stores them.
    #[must_use]
    pub fn prefetch(
        &self,
        head: &FrameRequest,
        direction: HeadDirection,
        count: u32,
    ) -> Vec<FrameKey> {
        let mut keys = Vec::new();
        let mut index = head.index;
        for _ in 0..count {
            let next = match direction {
                HeadDirection::Forward => index.checked_add(1),
                HeadDirection::Backward => index.checked_sub(1),
            };
            let Some(next) = next else {
                break;
            };
            index = next;
            let key = FrameKey::new(head.medium.content, head.level, index);
            if !self.entries.contains_key(&key) {
                keys.push(key);
            }
        }
        keys
    }

    /// The nearest cached frame to the time of the request (ARQ-145).
    ///
    /// The search walks the frames of the medium and the level asked for and
    /// keeps the one whose time is the closest to the time of the request.
    /// At the same distance the earlier frame wins.
    fn nearest(&self, request: &FrameRequest) -> Option<Arc<VideoFrame>> {
        let first = FrameKey::new(request.medium.content, request.level, 0);
        let last = FrameKey::new(request.medium.content, request.level, u32::MAX);
        let mut nearest: Option<(i128, Flicks, Arc<VideoFrame>)> = None;
        for (_, entry) in self.entries.range(first..=last) {
            let distance = distance(entry.frame.pts, request.time);
            let closer = match &nearest {
                None => true,
                Some((best_distance, best_time, _)) => {
                    distance < *best_distance
                        || (distance == *best_distance && entry.frame.pts < *best_time)
                }
            };
            if closer {
                nearest = Some((distance, entry.frame.pts, Arc::clone(&entry.frame)));
            }
        }
        nearest.map(|(_, _, frame)| frame)
    }

    /// Frees the cache until it holds at most `budget` bytes (ARQ-44).
    ///
    /// The budget of the cache does not change: the governor asks for a
    /// trim, the cache frees what it holds and grows again under its own
    /// budget.
    pub fn trim(&mut self, budget: Bytes) {
        self.evict_to(budget);
    }

    /// Marks the frame of `key` as the most recently used one.
    fn touch(&mut self, key: &FrameKey) {
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

    /// Frees the least recently used frames until the cache holds at most
    /// `budget` bytes (ARQ-145, ARQ-44).
    fn evict_to(&mut self, budget: Bytes) {
        while self.size.0 > budget.0 {
            let Some((used, key)) = self.recency.iter().next().map(|(used, key)| (*used, *key))
            else {
                break;
            };
            self.recency.remove(&used);
            if let Some(entry) = self.entries.remove(&key) {
                self.size = Bytes(self.size.0.saturating_sub(entry.size.0));
            }
        }
    }
}

/// The bytes one frame holds (ARQ-145).
fn frame_size(frame: &VideoFrame) -> Bytes {
    let bytes = match frame.image.data() {
        PixelBuf::Rgba8(data) => data.len() as u64,
        PixelBuf::RgbaF32(data) => (data.len() as u64).saturating_mul(4),
        PixelBuf::Alpha8(data) => data.len() as u64,
    };
    Bytes(bytes)
}

/// The distance between two times, in flicks (ARQ-145).
fn distance(one: Flicks, other: Flicks) -> i128 {
    let difference = i128::from(one.ticks()) - i128::from(other.ticks());
    difference.abs()
}

/// The budget of the cache on a machine with `ram` bytes of RAM (ARQ-145).
///
/// The lesser of one gigabyte and 15 % of the RAM (02 §6.5, FIJADA). The
/// RAM size comes from the caller: the crate reads no value of the system
/// (ARQ-138).
#[must_use]
pub fn default_budget(ram: Bytes) -> Bytes {
    let fifteenth = u128::from(ram.0) * 15 / 100;
    DEFAULT_BUDGET.min(Bytes(u64::try_from(fifteenth).unwrap_or(u64::MAX)))
}
