//! The cache of the decoded frames of the media (ARQ-145).

use gafira_doc::ContentHash;
use gafira_media::{
    FrameCache, FrameHit, FrameKey, FrameRequest, HeadDirection, MediumRef, ProxyLevel,
    default_budget,
};
use gafira_ports::assets::ProviderAssetId;
use gafira_ports::common::Bytes;
use gafira_ports::media::VideoFrame;
use gafira_raster::{ImageBuf, PixelBuf};
use gafira_time::Flicks;

/// The hash of the content of a medium.
fn content(seed: u8) -> ContentHash {
    ContentHash([seed; 32])
}

/// A medium: the id it has inside the project and the hash of its content.
fn medium(asset: &str, seed: u8) -> MediumRef {
    MediumRef::new(ProviderAssetId(asset.to_string()), content(seed))
}

/// A time in milliseconds.
fn at(millis: i64) -> Flicks {
    Flicks::from_millis(millis).expect("the time fits")
}

/// A frame of `side` pixels a side: `side * side * 4` bytes of RGBA.
fn frame(pts: Flicks, side: u32) -> VideoFrame {
    let pixels = (side as usize) * (side as usize) * 4;
    VideoFrame {
        pts,
        image: ImageBuf::new(side, side, PixelBuf::Rgba8(vec![0; pixels]))
            .expect("the image is well formed"),
    }
}

/// The bytes a frame of `side` pixels a side holds.
fn frame_bytes(side: u32) -> Bytes {
    Bytes(u64::from(side) * u64::from(side) * 4)
}

/// The indices of the frames of a prefetch.
fn indices(keys: &[FrameKey]) -> Vec<u32> {
    keys.iter().map(|key| key.index()).collect()
}

// ARQ-145: a budget of 0 keeps no frame and `bytes()` gives 0.
#[test]
fn arq_145_a_budget_of_zero_keeps_no_frame() {
    let mut cache = FrameCache::new(Bytes(0));
    let request = FrameRequest::new(medium("clip", 1), ProxyLevel::Original, 0, at(0));

    cache.put(&request, frame(at(0), 2));

    assert_eq!(cache.bytes(), Bytes(0));
    assert_eq!(cache.len(), 0);
    assert_eq!(cache.lookup(&request), FrameHit::Empty);
}

// ARQ-145: a budget that fits a single frame discards the first one when the
// third frame is asked for, and `bytes()` stays at or below the budget.
#[test]
fn arq_145_a_budget_of_one_frame_discards_the_first() {
    let budget = frame_bytes(2);
    let mut cache = FrameCache::new(budget);
    let first = FrameRequest::new(medium("clip", 1), ProxyLevel::Original, 0, at(0));
    let second = FrameRequest::new(medium("clip", 1), ProxyLevel::Original, 1, at(40));
    let third = FrameRequest::new(medium("clip", 1), ProxyLevel::Original, 2, at(80));

    cache.put(&first, frame(at(0), 2));
    cache.put(&second, frame(at(40), 2));
    cache.put(&third, frame(at(80), 2));

    assert!(cache.bytes() <= budget);
    assert_eq!(cache.len(), 1);
    assert!(matches!(cache.lookup(&third), FrameHit::Exact(_)));
    assert!(!matches!(cache.lookup(&first), FrameHit::Exact(_)));
}

// ARQ-145: the key is the hash of the content of the medium, the proxy level
// and the index of the frame in that level.
#[test]
fn arq_145_the_key_is_the_content_hash_the_level_and_the_index() {
    let key = FrameKey::new(content(7), ProxyLevel::P720, 12);

    assert_eq!(key.content(), content(7));
    assert_eq!(key.level(), ProxyLevel::P720);
    assert_eq!(key.index(), 12);
    assert_eq!(key, FrameKey::new(content(7), ProxyLevel::P720, 12));
    assert_ne!(key, FrameKey::new(content(8), ProxyLevel::P720, 12));
    assert_ne!(key, FrameKey::new(content(7), ProxyLevel::Original, 12));
    assert_ne!(key, FrameKey::new(content(7), ProxyLevel::P720, 13));
    // The key a request resolves to is the one of the three parts: the hash
    // of the content of the medium, not the id of the resource.
    let request = FrameRequest::new(medium("clip", 7), ProxyLevel::P720, 12, at(480));
    assert_eq!(request.key(), key);
}

// ARQ-145: the level is part of the key: a frame of the original is not the
// frame of the light copy of the same index.
#[test]
fn arq_145_a_frame_of_the_original_is_not_the_frame_of_the_proxy() {
    let mut cache = FrameCache::new(Bytes(1024));
    let medium = medium("clip", 1);
    let original = FrameRequest::new(medium.clone(), ProxyLevel::Original, 4, at(160));
    let proxy = FrameRequest::new(medium, ProxyLevel::P540, 4, at(160));

    cache.put(&original, frame(at(160), 2));

    assert_eq!(cache.len(), 1);
    assert!(matches!(cache.lookup(&original), FrameHit::Exact(_)));
    assert_eq!(cache.lookup(&proxy), FrameHit::Empty);
}

// ARQ-145: the same file with another resource id inside the project gives
// the same cached frame: the key is the hash of the content, not the id.
#[test]
fn arq_145_the_same_file_with_another_resource_id_gives_the_same_frame() {
    let mut cache = FrameCache::new(Bytes(1024));
    let first = FrameRequest::new(medium("clip-1", 9), ProxyLevel::Original, 3, at(120));
    let second = FrameRequest::new(medium("clip-2", 9), ProxyLevel::Original, 3, at(120));

    cache.put(&first, frame(at(120), 2));

    assert!(
        matches!(cache.lookup(&second), FrameHit::Exact(_)),
        "the same file gives the same cached frame in any project"
    );
    assert_eq!(cache.len(), 1);
}

// ARQ-145: a trim leaves the cache at or below the budget given and keeps
// the most recently used frames.
#[test]
fn arq_145_the_trim_leaves_the_cache_at_or_below_the_budget() {
    let budget = frame_bytes(2);
    let mut cache = FrameCache::new(Bytes(64));
    let first = FrameRequest::new(medium("clip", 1), ProxyLevel::Original, 0, at(0));
    let second = FrameRequest::new(medium("clip", 1), ProxyLevel::Original, 1, at(40));
    let third = FrameRequest::new(medium("clip", 1), ProxyLevel::Original, 2, at(80));

    cache.put(&first, frame(at(0), 2));
    cache.put(&second, frame(at(40), 2));
    cache.put(&third, frame(at(80), 2));
    // The first frame becomes the most recently used one.
    assert!(matches!(cache.lookup(&first), FrameHit::Exact(_)));

    cache.trim(budget);

    assert!(cache.bytes() <= budget);
    assert_eq!(cache.len(), 1);
    assert!(
        matches!(cache.lookup(&first), FrameHit::Exact(_)),
        "the most recently used frame stays in the cache"
    );
}

// ARQ-145: a trim with a budget bigger than the size of the cache changes
// nothing and does not fail.
#[test]
fn arq_145_a_trim_with_a_bigger_budget_changes_nothing() {
    let mut cache = FrameCache::new(Bytes(1024));
    let first = FrameRequest::new(medium("clip", 1), ProxyLevel::Original, 0, at(0));
    let second = FrameRequest::new(medium("clip", 1), ProxyLevel::Original, 1, at(40));

    cache.put(&first, frame(at(0), 2));
    cache.put(&second, frame(at(40), 2));
    let before = cache.bytes();

    cache.trim(Bytes(4096));

    assert_eq!(cache.bytes(), before);
    assert_eq!(cache.len(), 2);
}

// ARQ-145: the default budget of the cache is the lesser of one gigabyte and
// 15 % of the RAM of the machine.
#[test]
fn arq_145_the_default_budget_is_the_lesser_of_a_gigabyte_and_a_fifteenth_of_the_ram() {
    let gibibyte = 1024u64 * 1024 * 1024;

    assert_eq!(default_budget(Bytes(8 * gibibyte)), Bytes(gibibyte));
    assert_eq!(
        default_budget(Bytes(4 * gibibyte)),
        Bytes(4 * gibibyte * 15 / 100)
    );
    assert_eq!(default_budget(Bytes(0)), Bytes(0));
}

// ARQ-145: the nearest frame is chosen by the distance in time and, at the
// same distance, the earlier one wins.
#[test]
fn arq_145_the_nearest_frame_wins_the_tie_with_the_earlier_one() {
    let mut cache = FrameCache::new(Bytes(1024));
    let medium = medium("clip", 1);
    let earlier = FrameRequest::new(medium.clone(), ProxyLevel::Original, 1, at(100));
    let later = FrameRequest::new(medium.clone(), ProxyLevel::Original, 2, at(300));
    cache.put(&earlier, frame(at(100), 2));
    cache.put(&later, frame(at(300), 2));
    // The frame asked for is at 200 ms: the two cached frames are 100 ms away.
    let request = FrameRequest::new(medium, ProxyLevel::Original, 5, at(200));

    match cache.lookup(&request) {
        FrameHit::Nearest(found) => {
            assert_eq!(found.pts, at(100), "the earlier frame wins the tie");
        }
        other => panic!("the cache answers with the nearest frame: {other:?}"),
    }
}

// ARQ-145: a frame that is not cached is answered with the nearest frame
// that is, and the exact one is left to the decode.
#[test]
fn arq_145_a_missing_frame_answers_with_the_nearest_one() {
    let mut cache = FrameCache::new(Bytes(1024));
    let near = FrameRequest::new(medium("clip", 1), ProxyLevel::Original, 4, at(400));
    let other = FrameRequest::new(medium("other", 2), ProxyLevel::Original, 1, at(410));
    cache.put(&near, frame(at(400), 2));
    cache.put(&other, frame(at(410), 2));
    let missing = FrameRequest::new(medium("clip", 1), ProxyLevel::Original, 5, at(420));

    match cache.lookup(&missing) {
        FrameHit::Nearest(found) => {
            assert_eq!(
                found.pts,
                at(400),
                "the nearest frame of the medium asked for"
            );
        }
        other => panic!("the cache answers with the nearest frame: {other:?}"),
    }
}

// ARQ-145: the prefetch follows the direction the head of the scrub moves.
#[test]
fn arq_145_the_prefetch_follows_the_direction_of_the_head() {
    let cache = FrameCache::new(Bytes(1024));
    let head = FrameRequest::new(medium("clip", 1), ProxyLevel::Original, 10, at(400));

    let forward = cache.prefetch(&head, HeadDirection::Forward, 3);
    assert_eq!(indices(&forward), vec![11, 12, 13]);

    let backward = cache.prefetch(&head, HeadDirection::Backward, 3);
    assert_eq!(indices(&backward), vec![9, 8, 7]);
}

// ARQ-145: the prefetch skips the frames the cache holds and stops at the
// first frame of the medium.
#[test]
fn arq_145_the_prefetch_skips_the_cached_frames_and_stops_at_the_first_one() {
    let mut cache = FrameCache::new(Bytes(1024));
    let medium = medium("clip", 1);
    let head = FrameRequest::new(medium.clone(), ProxyLevel::Original, 2, at(80));
    let cached = FrameRequest::new(medium.clone(), ProxyLevel::Original, 1, at(40));
    cache.put(&cached, frame(at(40), 2));

    let backward = cache.prefetch(&head, HeadDirection::Backward, 3);
    assert_eq!(indices(&backward), vec![0]);

    let forward = cache.prefetch(&head, HeadDirection::Forward, 2);
    assert_eq!(indices(&forward), vec![3, 4]);
}
