//! ARQ-130: seed strokes and zones of `SegHints`.

use std::sync::Arc;

use gafira_doc::contract::{CancelToken, NoProgress};
use gafira_raster::{AlphaMask, ImageView, PixelData};
use gafira_segment::{ClassicSegmenter, SeedStroke, SegHints, SegZone, Segmenter};

const W: u32 = 200;
const H: u32 = 150;

fn seg() -> ClassicSegmenter {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    ClassicSegmenter::new(Arc::new(pool))
}

/// Grey background with red rectangles `[x0, x1) x [y0, y1)` in pixels.
fn scene(rects: &[(u32, u32, u32, u32)]) -> Vec<u8> {
    let mut v = Vec::new();
    for y in 0..H {
        for x in 0..W {
            let inside = rects
                .iter()
                .any(|&(x0, y0, x1, y1)| x >= x0 && x < x1 && y >= y0 && y < y1);
            v.extend_from_slice(if inside {
                &[200, 40, 40, 255]
            } else {
                &[120, 120, 120, 255]
            });
        }
    }
    v
}

fn run(data: &[u8], hints: &SegHints) -> AlphaMask {
    let view = ImageView::new(W, H, PixelData::Rgba8(data)).unwrap();
    seg()
        .segment(view, hints, &NoProgress, &CancelToken::new())
        .unwrap()
        .mask
}

fn at(m: &AlphaMask, x: u32, y: u32) -> u8 {
    m.data()[(y * W + x) as usize]
}

fn dot(x: f32, y: f32) -> SeedStroke {
    SeedStroke {
        points: vec![[x, y]],
        radius: 4.0,
    }
}

const CENTRE: (u32, u32, u32, u32) = (70, 45, 130, 105);

// ARQ-130: under a foreground seed alpha >= 243, under a background seed <= 12.
#[test]
fn arq_130_seed_strokes_set_the_alpha_under_them() {
    let data = scene(&[CENTRE]);
    let hints = SegHints {
        zone: SegZone::Foreground,
        foreground: vec![dot(30.0, 75.0)],
        background: vec![dot(100.0, 75.0)],
    };
    let m = run(&data, &hints);
    assert!(
        at(&m, 30, 75) >= 243,
        "foreground seed on grey: {}",
        at(&m, 30, 75)
    );
    assert!(
        at(&m, 100, 75) <= 12,
        "background seed on the subject: {}",
        at(&m, 100, 75)
    );
}

// ARQ-130: the guarantee holds for the thinnest stroke too, a radius-0 point
// that covers one pixel of the full-size image.
#[test]
fn arq_130_a_single_pixel_seed_still_decides_its_pixel() {
    let data = scene(&[CENTRE]);
    let point = |x: f32, y: f32| SeedStroke {
        points: vec![[x, y]],
        radius: 0.0,
    };
    let hints = SegHints {
        zone: SegZone::Foreground,
        foreground: vec![point(30.2, 75.7)],
        background: vec![point(100.4, 75.4)],
    };
    let m = run(&data, &hints);
    assert!(
        at(&m, 30, 75) >= 243,
        "foreground point: {}",
        at(&m, 30, 75)
    );
    assert!(
        at(&m, 100, 75) <= 12,
        "background point: {}",
        at(&m, 100, 75)
    );
}

// ARQ-130: a seed steers the cut beyond the pixels under it: a blue decoy is
// foreground without hints, and a background dot in its middle takes pixels
// of it that lie outside the dot.
#[test]
fn arq_130_a_background_seed_steers_the_cut() {
    let mut data = scene(&[CENTRE]);
    for y in 60..90 {
        for x in 20..50 {
            let i = ((y * W + x) * 4) as usize;
            data[i..i + 4].copy_from_slice(&[30, 60, 200, 255]);
        }
    }
    let plain = run(
        &data,
        &SegHints {
            zone: SegZone::Foreground,
            ..SegHints::default()
        },
    );
    assert!(
        at(&plain, 45, 75) >= 128 && at(&plain, 48, 88) >= 128,
        "decoy is foreground"
    );
    let hints = SegHints {
        zone: SegZone::Foreground,
        background: vec![SeedStroke {
            points: vec![[35.0, 75.0]],
            radius: 6.0,
        }],
        ..SegHints::default()
    };
    let seeded = run(&data, &hints);
    assert!(
        at(&seeded, 45, 75) < 128,
        "outside the dot: {}",
        at(&seeded, 45, 75)
    );
    assert!(at(&seeded, 48, 88) < 128, "corner: {}", at(&seeded, 48, 88));
    assert!(at(&seeded, 100, 75) >= 128, "subject kept");
}

// ARQ-130: Background is the complement of Foreground, 255 - alpha, +-1; All is solid.
#[test]
fn arq_130_background_is_the_complement_and_all_is_solid() {
    let data = scene(&[CENTRE]);
    let fg = run(
        &data,
        &SegHints {
            zone: SegZone::Foreground,
            ..SegHints::default()
        },
    );
    let bg = run(
        &data,
        &SegHints {
            zone: SegZone::Background,
            ..SegHints::default()
        },
    );
    for (a, b) in fg.data().iter().zip(bg.data()) {
        assert!((i16::from(255 - a) - i16::from(*b)).abs() <= 1);
    }
    assert!(fg.data().iter().any(|&a| a >= 128), "there is a foreground");
    let all = run(
        &data,
        &SegHints {
            zone: SegZone::All,
            ..SegHints::default()
        },
    );
    assert!(all.data().iter().all(|&a| a == 255));
}

// ARQ-130 / D-23: with two objects Elemento keeps the central one; Primer plano keeps both.
#[test]
fn arq_130_element_picks_the_object_at_the_centre() {
    let side = (10, 55, 40, 95);
    let data = scene(&[CENTRE, side]);
    let fg = run(
        &data,
        &SegHints {
            zone: SegZone::Foreground,
            ..SegHints::default()
        },
    );
    assert!(
        at(&fg, 100, 75) >= 128 && at(&fg, 25, 75) >= 128,
        "both objects"
    );
    let el = run(
        &data,
        &SegHints {
            zone: SegZone::Element,
            ..SegHints::default()
        },
    );
    assert!(at(&el, 100, 75) >= 128, "centre kept");
    assert!(
        at(&el, 25, 75) < 128,
        "side object dropped: {}",
        at(&el, 25, 75)
    );
}
