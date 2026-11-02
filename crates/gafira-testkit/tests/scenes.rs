//! ARQ-132: the synthetic scenes, the IoU and the fresh-set warning.

use std::sync::OnceLock;

use gafira_testkit::scenes::{
    Background, FIXED_COUNT, FIXED_SEED, FRESH_COUNT, FRESH_SEED, Part, Position, Recipe, Scene,
    fixed_recipe, fixed_scene, fresh_recipe, fresh_report, fresh_scene, iou, meets_gate,
    reference_scene,
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

/// What the tests read of a scene at 4000x3000, kept once for all of them.
struct Facts {
    name: &'static str,
    area: f64,
    delta_e: f64,
    /// Horizontal pairs of background pixels with different colours, over all.
    differing_pairs: f64,
    /// Distinct colours of the subject and of the background.
    colours: (usize, usize),
    /// A pixel strictly between the subject's colour and the background's.
    between: bool,
    truth: Vec<u8>,
}

fn facts_of(s: &Scene) -> Facts {
    let (w, h) = (s.width as usize, s.height as usize);
    assert_eq!(s.rgba.len(), w * h * 4, "{}", s.name);
    assert_eq!(s.truth.len(), w * h, "{}", s.name);
    assert!(s.rgba.chunks(4).all(|p| p[3] == 255), "{}: opaque", s.name);
    let px = |i: usize| [s.rgba[i * 4], s.rgba[i * 4 + 1], s.rgba[i * 4 + 2]];
    let (mut pairs, mut differ) = (0u64, 0u64);
    for y in 0..h {
        for x in 0..w - 1 {
            let i = y * w + x;
            if s.truth[i] == 0 && s.truth[i + 1] == 0 {
                pairs += 1;
                differ += u64::from(px(i) != px(i + 1));
            }
        }
    }
    // Distinct colours, stopping early: only 1 or more than 1 matters to most.
    let mut seen = [Vec::new(), Vec::new()];
    for i in 0..w * h {
        let group = &mut seen[usize::from(s.truth[i] == 255)];
        if group.len() < 8 && !group.contains(&px(i)) {
            group.push(px(i));
        }
    }
    // Colour of the subject's deepest pixel against a far background pixel.
    let far_bg = px(s.truth.iter().position(|t| *t == 0).unwrap());
    let subject = {
        let mut best = (0u64, 0usize);
        let mut run = 0u64;
        for i in 0..w * h {
            run = if s.truth[i] == 255 && i % w != 0 {
                run + 1
            } else {
                0
            };
            if run > best.0 {
                best = (run, i - (run / 2) as usize);
            }
        }
        px(best.1)
    };
    let between = (0..w * h).any(|i| {
        let c = px(i);
        (0..3).any(|k| {
            let (lo, hi) = (subject[k].min(far_bg[k]), subject[k].max(far_bg[k]));
            hi - lo >= 4 && c[k] > lo.saturating_add(1) && c[k].saturating_add(1) < hi
        })
    });
    Facts {
        name: s.name,
        area: s.area_fraction(),
        delta_e: s.mean_delta_e76(),
        differing_pairs: differ as f64 / pairs.max(1) as f64,
        colours: (seen[1].len(), seen[0].len()),
        between,
        truth: s.truth.clone(),
    }
}

fn fixed_facts() -> &'static [Facts] {
    static FACTS: OnceLock<Vec<Facts>> = OnceLock::new();
    FACTS.get_or_init(|| {
        (0..FIXED_COUNT)
            .map(|i| facts_of(&fixed_scene(i, 4000, 3000)))
            .collect()
    })
}

fn fresh_facts() -> &'static [Facts] {
    static FACTS: OnceLock<Vec<Facts>> = OnceLock::new();
    FACTS.get_or_init(|| {
        (0..FRESH_COUNT)
            .map(|i| facts_of(&fresh_scene(i, 4000, 3000)))
            .collect()
    })
}

fn fixed_recipes() -> Vec<Recipe> {
    (0..FIXED_COUNT).map(fixed_recipe).collect()
}

fn fresh_recipes() -> Vec<Recipe> {
    (0..FRESH_COUNT).map(fresh_recipe).collect()
}

/// Subject between 2 % and 50 % of the area, and the exact mean colours at
/// least ΔE76 6 apart; low contrast within 6 and 10; textured above 6.
fn check_properties(f: &Facts, r: &Recipe) {
    assert!(
        (0.02..=0.50).contains(&f.area),
        "{}: area {}",
        f.name,
        f.area
    );
    assert!(f.delta_e >= 6.0, "{}: dE76 {}", f.name, f.delta_e);
    if r.low_contrast {
        assert!(f.delta_e <= 10.0, "{}: dE76 {}", f.name, f.delta_e);
    }
    if r.background == Background::Textured {
        assert!(f.delta_e > 6.0, "{}: textured dE76 {}", f.name, f.delta_e);
    }
}

// ARQ-132: area and colour distance of the exact mask, scene by scene, in both
// sets at 4000x3000.
#[test]
fn arq_132_both_sets_keep_area_and_contrast_at_4000x3000() {
    for (f, r) in fixed_facts().iter().zip(fixed_recipes()) {
        check_properties(f, &r);
    }
    for (f, r) in fresh_facts().iter().zip(fresh_recipes()) {
        check_properties(f, &r);
    }
}

// ARQ-132, table of the fixed set: every scene is its row, in background, shape,
// position and target ΔE76. Scene 13 is the row that is not low contrast at 10.
// Mutants H1 and H2: a row loses its gradient, its shape or its offset.
#[test]
fn arq_132_the_fixed_set_is_the_table_of_the_spec() {
    use Background::{Flat, Gradient, Textured};
    use Part::{Disk, Ellipse, Rect, RoundRect, Star};
    use Position::{Centred, Offset};
    type Row = (Background, &'static [Part], &'static [Part], Position, f64);
    const ELLIPSE: &[Part] = &[Ellipse(0.667, 0.5, 0.34, 0.25)];
    const CRESCENT: &[Part] = &[Disk(0.667, 0.5, 0.33)];
    const BITE: &[Part] = &[Disk(0.8, 0.5, 0.27)];
    const RING: &[Part] = &[Disk(0.667, 0.5, 0.34)];
    const RING_CUT: &[Part] = &[Disk(0.667, 0.5, 0.17)];
    let rows: [Row; FIXED_COUNT] = [
        (Flat, ELLIPSE, &[], Centred, 40.0),
        (
            Flat,
            &[RoundRect(0.35, 0.62, 0.2, 0.15, 0.05)],
            &[],
            Offset,
            30.0,
        ),
        (Gradient, &[Disk(0.667, 0.5, 0.32)], &[], Centred, 25.0),
        (Gradient, &[Star(0.95, 0.4, 0.22)], &[], Offset, 25.0),
        (Textured, &[Disk(0.667, 0.5, 0.28)], &[], Centred, 25.0),
        (
            Textured,
            &[Ellipse(1.0, 0.6, 0.25, 0.17)],
            &[],
            Offset,
            25.0,
        ),
        (Flat, CRESCENT, BITE, Centred, 30.0),
        (
            Gradient,
            &[Rect(0.35, 0.62, 0.2, 0.8), Rect(0.35, 1.0, 0.58, 0.8)],
            &[],
            Centred,
            25.0,
        ),
        (Flat, RING, RING_CUT, Centred, 30.0),
        (
            Textured,
            &[Rect(0.3, 1.0, 0.2, 0.8)],
            &[Disk(0.5, 0.5, 0.1), Disk(0.8, 0.5, 0.1)],
            Centred,
            25.0,
        ),
        (Flat, ELLIPSE, &[], Centred, 6.0),
        (Flat, &[Star(0.9, 0.55, 0.22)], &[], Offset, 8.0),
        (
            Flat,
            &[Disk(0.5, 0.5, 0.26)],
            &[Disk(0.5, 0.5, 0.13)],
            Offset,
            10.0,
        ),
        (Flat, CRESCENT, BITE, Centred, 6.0),
        (Flat, &[Disk(0.667, 0.5, 0.10)], &[], Centred, 30.0),
        (Flat, &[Ellipse(0.667, 0.5, 0.53, 0.36)], &[], Centred, 30.0),
    ];
    for (i, (r, row)) in fixed_recipes().iter().zip(rows).enumerate() {
        assert_eq!(
            (r.background, r.add, r.cut, r.position, r.target_delta_e),
            row,
            "scene {:02}",
            i + 1
        );
        assert!(!r.soft_edge, "scene {:02}: hard edges", i + 1);
    }
    // The dE76 of a scene is its target or just above (13: 10.54 once quantised).
    for (i, (f, r)) in fixed_facts().iter().zip(fixed_recipes()).enumerate() {
        let d = f.delta_e - r.target_delta_e;
        assert!(
            (0.0..=1.2).contains(&d),
            "scene {:02}: {}",
            i + 1,
            f.delta_e
        );
    }
    let lows: Vec<usize> = (0..FIXED_COUNT)
        .filter(|&i| fixed_recipe(i).low_contrast)
        .map(|i| i + 1)
        .collect();
    assert_eq!(lows, [11, 12, 14], "low-contrast scenes");
    assert!((fixed_facts()[14].area - 0.024).abs() <= 0.001, "scene 15");
    assert!((fixed_facts()[15].area - 0.45).abs() <= 0.001, "scene 16");
}

// ARQ-132: a low-contrast scene has one colour in the subject and one in the
// background, in both sets, at a dE76 from 6 to 10.
// Mutant H3: a low-contrast recipe on a gradient.
#[test]
fn arq_132_low_contrast_is_flat_on_both_sides() {
    let sets = [
        (fixed_recipes(), fixed_facts()),
        (fresh_recipes(), fresh_facts()),
    ];
    for (recipes, facts) in &sets {
        for (r, f) in recipes.iter().zip(facts.iter()) {
            if r.low_contrast {
                assert_eq!(r.background, Background::Flat, "{}", f.name);
                assert_eq!(f.colours, (1, 1), "{}", f.name);
            }
        }
    }
}

// ARQ-132: each set has at least one scene of each class.
// Mutants H1 and H2: no gradient; no concave shape, no hole, no offset.
#[test]
fn arq_132_each_set_has_every_class() {
    for recipes in [fixed_recipes(), fresh_recipes()] {
        let any = |f: &dyn Fn(&Recipe) -> bool| recipes.iter().any(f);
        for b in [Background::Flat, Background::Gradient, Background::Textured] {
            assert!(any(&|r| r.background == b), "background {b:?}");
        }
        assert!(any(&|r| r.position == Position::Centred), "centred");
        assert!(any(&|r| r.position == Position::Offset), "offset");
        assert!(any(&|r| r.concave), "concave");
        assert!(any(&|r| r.holed), "holed");
        assert!(any(&|r| r.low_contrast), "low contrast");
    }
    let mut names: Vec<_> = fixed_recipes().iter().map(|r| r.name).collect();
    names.extend(fresh_recipes().iter().map(|r| r.name));
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), FIXED_COUNT + FRESH_COUNT, "names are distinct");
}

// ARQ-132: a concave or holed recipe has that shape in its exact mask: a hole
// is background enclosed by the subject, and a concave shape has two points
// whose midpoint is outside it. Mutant H2 at mask level.
#[test]
fn arq_132_shape_classes_hold_in_the_masks() {
    let probe = |r: &Recipe, s: &Scene| {
        let (w, h) = (s.width as usize, s.height as usize);
        // Background flood from the image border, 4-connected.
        let mut outside = vec![false; w * h];
        let mut todo: Vec<usize> = (0..w * h)
            .filter(|&i| i % w == 0 || i % w == w - 1 || i / w == 0 || i / w == h - 1)
            .filter(|&i| s.truth[i] == 0)
            .collect();
        while let Some(i) = todo.pop() {
            if std::mem::replace(&mut outside[i], true) {
                continue;
            }
            let (x, y) = (i % w, i / w);
            let around = [
                (x > 0).then(|| i - 1),
                (x + 1 < w).then(|| i + 1),
                (y > 0).then(|| i - w),
                (y + 1 < h).then(|| i + w),
            ];
            todo.extend(around.into_iter().flatten().filter(|&n| s.truth[n] == 0));
        }
        let holed = (0..w * h).any(|i| s.truth[i] == 0 && !outside[i]);
        assert_eq!(holed, r.holed, "{}: hole", r.name);
        if r.concave {
            // Two points of the subject whose midpoint is not in it.
            let points: Vec<(usize, usize)> = (0..h)
                .step_by(16)
                .flat_map(|y| (0..w).step_by(16).map(move |x| (x, y)))
                .filter(|&(x, y)| s.truth[y * w + x] == 255)
                .collect();
            let left = points.iter().any(|a| {
                points
                    .iter()
                    .any(|b| s.truth[(a.1 + b.1) / 2 * w + (a.0 + b.0) / 2] == 0)
            });
            assert!(left, "{}: concave", r.name);
        }
    };
    for i in 0..FIXED_COUNT {
        probe(&fixed_recipe(i), &fixed_scene(i, 800, 600));
    }
    for i in 0..FRESH_COUNT {
        probe(&fresh_recipe(i), &fresh_scene(i, 800, 600));
    }
}

// ARQ-132: the fresh set covers the whole range: a small and a large subject, a
// gradient and a texture at dE76 from 7 to 8, hard and soft edges.
#[test]
fn arq_132_fresh_scenes_cover_the_whole_range() {
    let facts = fresh_facts();
    let recipes = fresh_recipes();
    assert!(facts.iter().any(|f| f.area <= 0.05), "small subject");
    assert!(facts.iter().any(|f| f.area >= 0.40), "large subject");
    for b in [Background::Gradient, Background::Textured] {
        assert!(
            facts
                .iter()
                .zip(&recipes)
                .any(|(f, r)| r.background == b && (7.0..=8.0).contains(&f.delta_e)),
            "{b:?} at dE76 7 to 8: {:?}",
            facts.iter().map(|f| f.delta_e).collect::<Vec<_>>()
        );
    }
    assert!(recipes.iter().any(|r| !r.soft_edge), "hard edge");
    assert!(recipes.iter().any(|r| r.soft_edge), "soft edge");
    for (f, r) in facts.iter().zip(&recipes) {
        if r.soft_edge {
            assert!(f.between, "{}: soft edge without a ramp", f.name);
        }
    }
}

// ARQ-132: no exact mask of a fresh scene reaches IoU 0.90 with one of the fixed
// set: the recipes are not the same shapes with other colours.
#[test]
fn arq_132_no_fresh_mask_is_a_fixed_mask() {
    for f in fresh_facts() {
        for g in fixed_facts() {
            let v = iou(&f.truth, &g.truth);
            assert!(
                !meets_gate(&f.truth, &g.truth),
                "{} and {}: {v}",
                f.name,
                g.name
            );
        }
    }
}

// ARQ-132: texture is per pixel in both sets: at least half of the horizontal
// pairs of background pixels differ. A texture of 12 px cells does not pass.
#[test]
fn arq_132_texture_is_per_pixel() {
    let sets = [
        (fixed_recipes(), fixed_facts()),
        (fresh_recipes(), fresh_facts()),
    ];
    for (recipes, facts) in &sets {
        for (r, f) in recipes.iter().zip(facts.iter()) {
            if r.background == Background::Textured {
                assert!(
                    f.differing_pairs >= 0.5,
                    "{}: {}",
                    f.name,
                    f.differing_pairs
                );
            }
        }
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
    // The noise of the fresh set is not the noise of the fixed set.
    assert_ne!(FIXED_SEED, FRESH_SEED);
    assert_ne!(fixed_scene(0, 160, 120).rgba, fixed_scene(1, 160, 120).rgba);
}

// ARQ-112 / ARQ-132: the reference is scene 01, flat and centred, at 1024x768.
#[test]
fn arq_112_reference_is_flat_and_centred_at_1024x768() {
    let s = reference_scene();
    assert_eq!((s.width, s.height), (1024, 768));
    assert_eq!(fixed_recipe(0).background, Background::Flat);
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
        (cx - 0.667 * 768.0).abs() < 2.0 && (cy - 384.0).abs() < 2.0,
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
    };
    let de = scene.mean_delta_e76();
    assert!(de < 0.5, "dE76 {de}");
}
