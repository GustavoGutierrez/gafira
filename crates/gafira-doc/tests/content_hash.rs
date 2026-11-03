//! `ContentHash` (design `gafira-ports` P5): a plain 32-byte value with the
//! order and equality a content-addressed store needs.

use std::collections::BTreeSet;

use gafira_doc::ContentHash;

// ARQ-01 (P5): the type is plain data, ordered byte-wise from the first byte.
#[test]
fn arq_01_content_hash_is_a_copy_ordered_value() {
    let mut low = [0u8; 32];
    low[31] = 0xFF;
    let mut high = [0u8; 32];
    high[0] = 1;
    let (a, b) = (ContentHash(low), ContentHash(high));
    let copy = a;
    assert_eq!(a, copy);
    assert!(a < b, "the first byte weighs most");
    let set: BTreeSet<_> = [b, a, copy].into_iter().collect();
    assert_eq!(set.into_iter().collect::<Vec<_>>(), vec![a, b]);
}
