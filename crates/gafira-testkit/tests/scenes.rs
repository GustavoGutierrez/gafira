//! ARQ-132: the synthetic scenes, the IoU and the fresh-set warning.

use gafira_testkit::scenes::{
    FIXED_COUNT, FRESH_COUNT, Scene, fixed_recipe, fixed_scene, fresh_report, fresh_scene, iou,
    meets_gate, reference_scene,
};

/// A mask of `n` bytes with the first `on` set.
fn first(n: usize, on: usize) -> Vec<u8> {
    (0..n).map(|i| if i < on { 255 } else { 0 }).collect()
}

// ARQ-132: IoU 0.90 passes and 0.8999 fails; the threshold includes equality.
#[test]
fn arq_132_iou_090_passes_and_08999_fails() {
    // Mutant: `>=` becomes `>`.
    let truth = first(100, 100);
    assert!(meets_gate(&first(100, 90), &truth), "90/100");
    assert!(!meets_gate(&first(100, 89), &truth), "89/100");
    let truth = first(10_000, 10_000);
    assert!(!meets_gate(&first(10_000, 8_999), &truth), "8999/10000");
    assert!(meets_gate(&first(10_000, 9_000), &truth), "9000/10000");
    assert!((iou(&first(100, 90), &first(100, 100)) - 0.9).abs() < 1e-12);
}

// ARQ-132: with both masks empty the IoU is 1; with one empty it is 0.
#[test]
fn arq_132_empty_masks_have_iou_one() {
    assert_eq!(iou(&[0; 16], &[0; 16]), 1.0);
    assert!(meets_gate(&[0; 16], &[0; 16]));
    assert_eq!(iou(&[0; 16], &first(16, 4)), 0.0);
    assert_eq!(iou(&first(16, 4), &[0; 16]), 0.0);
}

// ARQ-132: masks are binarised at alpha >= 128, on both sides.
#[test]
fn arq_132_iou_binarises_at_alpha_128() {
    assert_eq!(iou(&[127, 0], &[255, 0]), 0.0);
    assert_eq!(iou(&[128, 0], &[255, 0]), 1.0);
    assert_eq!(iou(&[255, 0], &[128, 0]), 1.0);
    assert_eq!(iou(&[255, 0], &[127, 0]), 0.0);
}

// ARQ-132: 12 fresh scenes; a segmenter that fails on all reports 0/12 and
// the run does not fail.
#[test]
fn arq_132_fresh_report_counts_and_never_fails() {
    let none = fresh_report(&[0.0; FRESH_COUNT]);
    assert_eq!((none.passed, none.total), (0, 12));
    let mut ious = [0.5; FRESH_COUNT];
    ious[0] = 0.9;
    ious[1] = 0.8999;
    ious[2] = 1.0;
    let some = fresh_report(&ious);
    assert_eq!((some.passed, some.total), (2, 12));
    assert_eq!(some.ious.len(), 12);
}

// ARQ-132: the recipes include a low-contrast scene and textured ones.
#[test]
fn arq_132_recipes_cover_low_contrast_and_texture() {
    let recipes: Vec<_> = (0..FIXED_COUNT).map(fixed_recipe).collect();
    assert!(recipes.iter().any(|r| r.low_contrast), "low contrast");
    assert!(
        recipes.iter().filter(|r| r.textured).count() >= 3,
        "textured"
    );
    let mut names: Vec<_> = recipes.iter().map(|r| r.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), FIXED_COUNT, "names are distinct");
}

/// Subject between 2 % and 50 % of the area, and the exact mean colours at
/// least ΔE76 6 apart; low contrast within 6 and 10; textured above 6.
fn check_properties(s: &Scene, w: u32, h: u32) {
    assert_eq!((s.width, s.height), (w, h), "{}", s.name);
    assert_eq!(s.rgba.len(), (w * h * 4) as usize, "{}", s.name);
    assert_eq!(s.truth.len(), (w * h) as usize, "{}", s.name);
    let area = s.area_fraction();
    assert!((0.02..=0.50).contains(&area), "{}: area {area}", s.name);
    let de = s.mean_delta_e76();
    assert!(de >= 6.0, "{}: dE76 {de}", s.name);
    if s.low_contrast {
        assert!(de <= 10.0, "{}: low contrast dE76 {de}", s.name);
    }
    if s.textured {
        assert!(de > 6.0, "{}: textured dE76 {de}", s.name);
    }
    assert!(s.rgba.chunks(4).all(|p| p[3] == 255), "{}: opaque", s.name);
}

macro_rules! fixed_scene_tests {
    ($($name:ident: $i:expr),* $(,)?) => {$(
        // ARQ-132: area and colour distance of the exact mask, scene by scene.
        #[test]
        fn $name() {
            check_properties(&fixed_scene($i, 4000, 3000), 4000, 3000);
        }
    )*};
}

fixed_scene_tests! {
    arq_132_fixed_scene_00_area_and_contrast: 0,
    arq_132_fixed_scene_01_area_and_contrast: 1,
    arq_132_fixed_scene_02_area_and_contrast: 2,
    arq_132_fixed_scene_03_area_and_contrast: 3,
    arq_132_fixed_scene_04_area_and_contrast: 4,
    arq_132_fixed_scene_05_area_and_contrast: 5,
    arq_132_fixed_scene_06_area_and_contrast: 6,
    arq_132_fixed_scene_07_area_and_contrast: 7,
    arq_132_fixed_scene_08_area_and_contrast: 8,
    arq_132_fixed_scene_09_area_and_contrast: 9,
    arq_132_fixed_scene_10_area_and_contrast: 10,
    arq_132_fixed_scene_11_area_and_contrast: 11,
    arq_132_fixed_scene_12_area_and_contrast: 12,
    arq_132_fixed_scene_13_area_and_contrast: 13,
    arq_132_fixed_scene_14_area_and_contrast: 14,
    arq_132_fixed_scene_15_area_and_contrast: 15,
}

// ARQ-132: the fresh scenes come from the same generator, so they keep the rules.
#[test]
fn arq_132_fresh_scenes_keep_the_generator_rules() {
    for i in 0..FRESH_COUNT {
        check_properties(&fresh_scene(i, 800, 600), 800, 600);
    }
}

// ARQ-132: the generator is deterministic, the seeds differ, and scenes differ.
#[test]
fn arq_132_scenes_are_deterministic_and_the_seeds_differ() {
    for i in 0..FIXED_COUNT {
        assert_eq!(
            fixed_scene(i, 160, 120),
            fixed_scene(i, 160, 120),
            "fixed {i}"
        );
    }
    for i in 0..FRESH_COUNT {
        assert_eq!(
            fresh_scene(i, 160, 120),
            fresh_scene(i, 160, 120),
            "fresh {i}"
        );
    }
    // Same recipe, other seed: the fresh scene 0 is not the fixed scene 0.
    let (a, b) = (fixed_scene(0, 160, 120), fresh_scene(0, 160, 120));
    assert_eq!(a.name, b.name);
    assert_ne!(a.rgba, b.rgba, "other seed, other pixels");
    assert_ne!(fixed_scene(0, 160, 120).rgba, fixed_scene(1, 160, 120).rgba);
}

// ARQ-112 / ARQ-132: the reference is the flat background, centred subject, 1024x768.
#[test]
fn arq_112_reference_is_flat_and_centred_at_1024x768() {
    let s = reference_scene();
    assert_eq!((s.width, s.height), (1024, 768));
    assert!(!s.textured);
    let bg: Vec<&[u8]> = s
        .rgba
        .chunks(4)
        .zip(&s.truth)
        .filter(|(_, t)| **t == 0)
        .map(|(p, _)| p)
        .collect();
    assert!(
        !bg.is_empty() && bg.iter().all(|p| *p == bg[0]),
        "flat background"
    );
    let (mut sx, mut sy, mut n) = (0u64, 0u64, 0u64);
    for (i, t) in s.truth.iter().enumerate() {
        if *t == 255 {
            sx += (i % 1024) as u64;
            sy += (i / 1024) as u64;
            n += 1;
        }
    }
    let (cx, cy) = (sx as f64 / n as f64, sy as f64 / n as f64);
    assert!(
        (cx - 512.0).abs() < 8.0 && (cy - 384.0).abs() < 8.0,
        "centre {cx},{cy}"
    );
}

// D-44 / ARQ-132: the exact mean colours are averaged in linear light. A subject
// of one black and one white pixel has the mean of linear 0.5 (L 76.07), which
// grey 188 matches; averaging the encoded bytes would give grey 127.5 (L 53).
#[test]
fn d_44_scene_means_are_averaged_in_linear_light() {
    let scene = Scene {
        name: "hand",
        width: 2,
        height: 2,
        rgba: vec![
            0, 0, 0, 255, 255, 255, 255, 255, 188, 188, 188, 255, 188, 188, 188, 255,
        ],
        truth: vec![255, 255, 0, 0],
        low_contrast: false,
        textured: false,
    };
    let de = scene.mean_delta_e76();
    assert!(de < 0.5, "dE76 {de}");
}
