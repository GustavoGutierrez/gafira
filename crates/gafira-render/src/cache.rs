//! The content-addressed tile cache (ARQ-142, 02 §6.1 and §4.6).
//!
//! The cache is keyed by [`TileKey`]: the content key of the tile, its level
//! of detail, its column and its row. It holds `Arc<ImageBuf>` values under a
//! byte budget (P8 of the layer-3 design): a budget of zero keeps nothing,
//! `trim` shrinks the cache to a new budget, and on overflow the entry unused
//! for the longest time goes first.
//!
//! The crate is pure in the sense of ARQ-02 (ADR 0016), so the cache reads no
//! clock and opens no file: the instant of the last use of an entry is a
//! counter the cache owns.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use gafira_ports::common::Bytes;
use gafira_ports::render::{TileCache, TileKey};
use gafira_raster::{ImageBuf, PixelBuf};

/// A cache of tiles addressed by content, with a byte budget (ARQ-142, P8).
///
/// The entries are ordered by their last use, so the cache discards the entry
/// unused for the longest time when it overflows. The order and the total are
/// `BTreeMap`/`BTreeSet` based, never the hash types ARQ-10 bans.
pub struct ContentTileCache {
    budget: Bytes,
    state: RefCell<State>,
}

/// The contents of the cache.
#[derive(Default)]
struct State {
    /// The bytes the entries hold now.
    total: Bytes,
    /// The counter of the last use; it only grows.
    clock: u64,
    /// The entries, by key.
    entries: BTreeMap<TileKey, Entry>,
    /// The entries by their last use, so the least recently used one is first.
    order: BTreeSet<(u64, TileKey)>,
}

/// One cached tile.
struct Entry {
    tile: Arc<ImageBuf>,
    /// The bytes this tile holds, measured from its buffer.
    bytes: Bytes,
    /// The value of [`State::clock`] at its last use.
    used: u64,
}

impl ContentTileCache {
    /// A cache that holds at most `budget` bytes.
    #[must_use]
    pub fn new(budget: Bytes) -> Self {
        Self {
            budget,
            state: RefCell::new(State::default()),
        }
    }
}

impl TileCache for ContentTileCache {
    /// The cached tile. A read is a use: the entry moves to the most
    /// recently used end (ARQ-142).
    fn get(&self, key: &TileKey) -> Option<Arc<ImageBuf>> {
        let mut state = self.state.borrow_mut();
        state.clock += 1;
        let used = state.clock;
        let entry = state.entries.get_mut(key)?;
        let previous = entry.used;
        entry.used = used;
        let tile = Arc::clone(&entry.tile);
        // A read is a use: the entry moves to the most recently used end.
        state.order.remove(&(previous, *key));
        state.order.insert((used, *key));
        Some(tile)
    }

    /// Stores a tile and evicts from the least recently used end until the
    /// cache fits its budget (ARQ-142).
    fn put(&mut self, key: TileKey, tile: Arc<ImageBuf>) {
        let bytes = tile_bytes(&tile);
        let state = self.state.get_mut();
        // A second tile under the same key replaces the first one.
        if let Some(previous) = state.entries.remove(&key) {
            state.total = Bytes(state.total.0.saturating_sub(previous.bytes.0));
            state.order.remove(&(previous.used, key));
        }
        state.clock += 1;
        let used = state.clock;
        state.total = Bytes(state.total.0.saturating_add(bytes.0));
        state.entries.insert(key, Entry { tile, bytes, used });
        state.order.insert((used, key));
        evict_to(state, self.budget);
    }

    /// The bytes the entries hold now, measured from their buffers (ARQ-44).
    fn bytes(&self) -> Bytes {
        self.state.borrow().total
    }

    /// Shrinks the cache to `budget` and keeps that budget for the later
    /// inserts (ARQ-44, P8).
    fn trim(&mut self, budget: Bytes) {
        self.budget = budget;
        evict_to(self.state.get_mut(), budget);
    }
}

/// Discards the entries unused for the longest time until the cache holds at
/// most `budget` bytes (ARQ-142).
fn evict_to(state: &mut State, budget: Bytes) {
    while state.total > budget {
        let Some(&(used, key)) = state.order.first() else {
            break;
        };
        state.order.remove(&(used, key));
        if let Some(entry) = state.entries.remove(&key) {
            state.total = Bytes(state.total.0.saturating_sub(entry.bytes.0));
        }
    }
}

/// The bytes a tile holds: the length of its buffer, in bytes.
fn tile_bytes(tile: &ImageBuf) -> Bytes {
    let bytes = match tile.data() {
        PixelBuf::Rgba8(pixels) | PixelBuf::Alpha8(pixels) => pixels.len(),
        PixelBuf::RgbaF32(pixels) => pixels.len() * std::mem::size_of::<f32>(),
    };
    Bytes(u64::try_from(bytes).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use gafira_ports::render::ContentKey;

    use super::*;

    /// The bytes of a 256×256 tile: 256 KiB in `Rgba8` and 1 MiB in `RgbaF32`
    /// (XP4 of the layer-3 design).
    const RGBA8_TILE: u64 = 262_144;
    const RGBA_F32_TILE: u64 = 1_048_576;

    /// A key of the grid of a page.
    fn key(content: u128, level: u8, column: u32, row: u32) -> TileKey {
        TileKey {
            content: ContentKey(content),
            level,
            column,
            row,
        }
    }

    /// A 256×256 `Rgba8` tile.
    fn rgba8(value: u8) -> Arc<ImageBuf> {
        let pixels = vec![value; (256 * 256 * 4) as usize];
        Arc::new(ImageBuf::new(256, 256, PixelBuf::Rgba8(pixels)).expect("a 256×256 Rgba8 tile"))
    }

    /// A 256×256 `RgbaF32` tile.
    fn rgba_f32() -> Arc<ImageBuf> {
        let pixels = vec![0.0f32; (256 * 256 * 4) as usize];
        Arc::new(
            ImageBuf::new(256, 256, PixelBuf::RgbaF32(pixels)).expect("a 256×256 RgbaF32 tile"),
        )
    }

    // ARQ-142: a budget of zero keeps no entry and `bytes()` gives 0.
    #[test]
    fn arq_142_a_budget_of_zero_keeps_nothing() {
        let at = key(1, 0, 0, 0);
        let mut cache = ContentTileCache::new(Bytes(0));
        cache.put(at, rgba8(0x11));
        assert!(cache.get(&at).is_none(), "a zero budget keeps no entry");
        assert_eq!(cache.bytes(), Bytes(0));
    }

    // ARQ-44: a `trim` with a budget larger than the current size changes
    // nothing and does not fail.
    #[test]
    fn arq_44_a_trim_above_the_current_size_changes_nothing() {
        let (first, second) = (key(1, 0, 0, 0), key(2, 0, 1, 0));
        let mut cache = ContentTileCache::new(Bytes(2 * RGBA8_TILE));
        cache.put(first, rgba8(0x11));
        cache.put(second, rgba8(0x22));
        let before = cache.bytes();
        cache.trim(Bytes(before.0 + RGBA8_TILE));
        assert_eq!(cache.bytes(), before);
        assert!(cache.get(&first).is_some());
        assert!(cache.get(&second).is_some());
    }

    // ARQ-44: after a `trim`, `bytes()` is at most the budget given.
    #[test]
    fn arq_44_a_trim_leaves_the_cache_at_or_below_the_budget() {
        let cases: [(u64, u64); 5] = [
            (0, 0),
            (RGBA8_TILE / 2, 0),
            (RGBA8_TILE, RGBA8_TILE),
            (RGBA8_TILE + RGBA8_TILE / 2, RGBA8_TILE),
            (3 * RGBA8_TILE, 3 * RGBA8_TILE),
        ];
        for (budget, expected) in cases {
            let mut cache = ContentTileCache::new(Bytes(4 * RGBA8_TILE));
            for at in 0..4 {
                cache.put(key(u128::from(at) + 1, 0, at, 0), rgba8(0x33));
            }
            cache.trim(Bytes(budget));
            assert!(cache.bytes() <= Bytes(budget), "trim to {budget}");
            assert_eq!(cache.bytes(), Bytes(expected), "trim to {budget}");
        }
    }

    // ARQ-142: on overflow, the entry unused for the longest time is
    // discarded.
    #[test]
    fn arq_142_the_least_recently_used_entry_is_discarded() {
        let (first, second, third) = (key(1, 0, 0, 0), key(2, 0, 1, 0), key(3, 0, 2, 0));
        let mut cache = ContentTileCache::new(Bytes(2 * RGBA8_TILE));
        cache.put(first, rgba8(0x11));
        cache.put(second, rgba8(0x22));
        assert!(cache.get(&first).is_some(), "the first entry is read");
        cache.put(third, rgba8(0x33));
        assert!(cache.get(&first).is_some(), "the recently used entry stays");
        assert!(
            cache.get(&second).is_none(),
            "the least recently used entry goes"
        );
        assert!(cache.get(&third).is_some());
        assert_eq!(cache.bytes(), Bytes(2 * RGBA8_TILE));
    }

    // ARQ-142: the key of §4.6 is the content key, the level, the column and
    // the row.
    #[test]
    fn arq_142_the_key_is_the_content_the_level_the_column_and_the_row() {
        let base = key(7, 0, 3, 4);
        let others = [
            key(7, 1, 3, 4),
            key(7, 0, 4, 4),
            key(7, 0, 3, 5),
            key(8, 0, 3, 4),
        ];
        let mut cache = ContentTileCache::new(Bytes(5 * RGBA8_TILE));
        cache.put(base, rgba8(0x01));
        for other in others {
            cache.put(other, rgba8(0x02));
        }
        assert_eq!(
            cache.bytes(),
            Bytes(5 * RGBA8_TILE),
            "every variant is its own entry"
        );
        assert!(cache.get(&base).is_some());
        for other in others {
            assert!(cache.get(&other).is_some());
        }
    }

    // ARQ-142 and XP4: the accounting counts the bytes of the buffer.
    #[test]
    fn arq_142_the_byte_accounting_matches_the_buffer_size() {
        let mut cache = ContentTileCache::new(Bytes(2 * RGBA_F32_TILE));
        cache.put(key(1, 0, 0, 0), rgba8(0x11));
        assert_eq!(cache.bytes(), Bytes(RGBA8_TILE));
        cache.put(key(1, 0, 1, 0), rgba_f32());
        assert_eq!(cache.bytes(), Bytes(RGBA8_TILE + RGBA_F32_TILE));
    }

    // ARQ-44: storing the same key again replaces the entry instead of
    // adding a second one.
    #[test]
    fn arq_44_storing_the_same_key_again_replaces_the_entry() {
        let at = key(1, 0, 0, 0);
        let mut cache = ContentTileCache::new(Bytes(2 * RGBA_F32_TILE));
        cache.put(at, rgba8(0x11));
        cache.put(at, rgba_f32());
        assert_eq!(cache.bytes(), Bytes(RGBA_F32_TILE));
        assert!(cache.get(&at).is_some());
    }

    // ARQ-44: a `trim` lowers the budget the later inserts respect.
    #[test]
    fn arq_44_a_trim_lowers_the_budget_of_the_later_inserts() {
        let mut cache = ContentTileCache::new(Bytes(3 * RGBA8_TILE));
        cache.put(key(1, 0, 0, 0), rgba8(0x11));
        cache.trim(Bytes(RGBA8_TILE));
        cache.put(key(2, 0, 1, 0), rgba8(0x22));
        assert!(cache.bytes() <= Bytes(RGBA8_TILE));
        assert_eq!(cache.bytes(), Bytes(RGBA8_TILE));
    }

    // ARQ-44: the cache travels through the port, which is its only surface.
    #[test]
    fn arq_44_the_cache_travels_through_the_port() {
        let at = key(1, 0, 0, 0);
        let mut cache = ContentTileCache::new(Bytes(RGBA8_TILE));
        let port: &mut dyn TileCache = &mut cache;
        port.put(at, rgba8(0x11));
        assert_eq!(port.bytes(), Bytes(RGBA8_TILE));
        assert!(port.get(&at).is_some());
        port.trim(Bytes(0));
        assert_eq!(port.bytes(), Bytes(0));
        assert!(port.get(&at).is_none());
    }
}
