//! A preallocated open-addressing table of `u64` pair keys (XP11, ARQ-111).
//!
//! `std::collections::HashMap` is banned in domain crates. The table never
//! grows: the caller states the largest number of distinct pairs up front,
//! and draining sorts, so the output does not depend on insertion order.

const EMPTY: u64 = u64::MAX;

pub(crate) struct PairTable {
    slots: Vec<u64>,
    mask: usize,
}

impl PairTable {
    /// Room for `max_pairs` distinct pairs at a load of at most one half.
    pub(crate) fn with_bound(max_pairs: usize) -> Self {
        let capacity = (max_pairs.max(8) * 2).next_power_of_two();
        Self {
            slots: vec![EMPTY; capacity],
            mask: capacity - 1,
        }
    }

    #[cfg(test)]
    pub(crate) fn capacity(&self) -> usize {
        self.slots.len()
    }

    fn key(a: u32, b: u32) -> u64 {
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        (u64::from(lo) << 32) | u64::from(hi)
    }

    /// Adds the unordered pair `{a, b}` once. A pair beyond the stated bound
    /// that finds no free slot is dropped: the caller's bound makes that
    /// unreachable, and the table never reallocates.
    pub(crate) fn insert(&mut self, a: u32, b: u32) {
        let key = Self::key(a, b);
        let mut slot = (key.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 20) as usize & self.mask;
        for _ in 0..self.slots.len() {
            let found = self.slots[slot];
            if found == key {
                return;
            }
            if found == EMPTY {
                self.slots[slot] = key;
                return;
            }
            slot = (slot + 1) & self.mask;
        }
    }

    /// Every pair, `(low, high)`, ascending.
    pub(crate) fn drain_sorted(self) -> Vec<(u32, u32)> {
        let mut keys: Vec<u64> = self.slots.into_iter().filter(|&k| k != EMPTY).collect();
        keys.sort_unstable();
        keys.into_iter()
            .map(|k| ((k >> 32) as u32, (k & 0xFFFF_FFFF) as u32))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(k: u32) -> Vec<(u32, u32)> {
        (0..3 * k)
            .map(|i| (i % k, k + (i % k + i / k) % k))
            .collect()
    }

    // ARQ-111: the drained list does not depend on the order of insertion.
    #[test]
    fn arq_111_drain_order_does_not_depend_on_insertion_order() {
        let list = pairs(40);
        let mut forward = PairTable::with_bound(list.len());
        let mut backward = PairTable::with_bound(list.len());
        for &(a, b) in &list {
            forward.insert(a, b);
        }
        for &(a, b) in list.iter().rev() {
            backward.insert(b, a);
        }
        let (f, b) = (forward.drain_sorted(), backward.drain_sorted());
        assert_eq!(f, b);
        assert!(f.windows(2).all(|w| w[0] < w[1]), "ascending, no repeats");
    }

    // XP11: the table never grows; inserting up to the stated bound keeps its size.
    #[test]
    fn xp11_table_does_not_regrow_up_to_its_bound() {
        let list = pairs(50);
        let mut table = PairTable::with_bound(list.len());
        let before = table.capacity();
        for &(a, b) in &list {
            table.insert(a, b);
        }
        assert_eq!(table.capacity(), before);
        assert_eq!(table.drain_sorted().len(), list.len(), "pairs are distinct");
    }
}
