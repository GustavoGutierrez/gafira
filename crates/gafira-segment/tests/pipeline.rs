//! ARQ-130: the pipeline isolates a centred subject on a flat background.

use std::sync::Arc;

use gafira_doc::contract::{CancelToken, NoProgress};
use gafira_raster::{ImageView, PixelData};
use gafira_segment::{ClassicSegmenter, SegHints, SegZone, Segmenter};

fn seg() -> ClassicSegmenter {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    ClassicSegmenter::new(Arc::new(pool))
}

/// Grey background, red square covering the middle `frac` of each side.
fn scene(w: u32, h: u32, frac: f32) -> Vec<u8> {
    let mut v = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let fx = (x as f32 + 0.5) / w as f32;
            let fy = (y as f32 + 0.5) / h as f32;
            let inside = (fx - 0.5).abs() < frac / 2.0 && (fy - 0.5).abs() < frac / 2.0;
            v.extend_from_slice(if inside {
                &[200, 40, 40, 255]
            } else {
                &[120, 120, 120, 255]
            });
        }
    }
    v
}

// ARQ-130: with no seeds the subject gets alpha >= 128 and the background < 128.
#[test]
fn arq_130_isolates_a_centred_subject_without_hints() {
    for (w, h) in [(64, 48), (200, 150), (800, 600), (4000, 3000)] {
        let data = scene(w, h, 0.4);
        let hints = SegHints {
            zone: SegZone::Foreground,
            ..SegHints::default()
        };
        let view = ImageView::new(w, h, PixelData::Rgba8(&data)).unwrap();
        let out = seg()
            .segment(view, &hints, &NoProgress, &CancelToken::new())
            .unwrap();
        let at = |x: u32, y: u32| out.mask.data()[(y * w + x) as usize];
        assert!(
            at(w / 2, h / 2) >= 128,
            "{w}x{h} centre {}",
            at(w / 2, h / 2)
        );
        assert!(at(2, 2) < 128 && at(w - 3, h - 3) < 128, "{w}x{h} corners");
        let inside = (0..w * h)
            .filter(|i| out.mask.data()[*i as usize] >= 128)
            .count() as f32;
        let truth = (w as f32 * 0.4) * (h as f32 * 0.4);
        assert!(
            (inside / truth - 1.0).abs() < 0.1,
            "{w}x{h}: {inside} vs {truth}"
        );
    }
}

/// Opaque `w` x `h` pixels from a colour function.
fn paint(w: u32, h: u32, colour: impl Fn(u32, u32) -> [u8; 3]) -> Vec<u8> {
    (0..w * h)
        .flat_map(|i| {
            let [r, g, b] = colour(i % w, i / w);
            [r, g, b, 255]
        })
        .collect()
}

fn foreground_share(w: u32, h: u32, data: &[u8], inside: impl Fn(u32, u32) -> bool) -> (f64, f64) {
    let view = ImageView::new(w, h, PixelData::Rgba8(data)).unwrap();
    let hints = SegHints {
        zone: SegZone::Foreground,
        ..SegHints::default()
    };
    let out = seg()
        .segment(view, &hints, &NoProgress, &CancelToken::new())
        .unwrap();
    let (mut hit, mut wanted, mut extra, mut rest) = (0.0, 0.0, 0.0, 0.0);
    for (i, a) in out.mask.data().iter().enumerate() {
        let (x, y) = (i as u32 % w, i as u32 / w);
        if inside(x, y) {
            wanted += 1.0;
            hit += f64::from(u8::from(*a >= 128));
        } else {
            rest += 1.0;
            extra += f64::from(u8::from(*a >= 128));
        }
    }
    (hit / wanted, extra / rest)
}

// ARQ-133: on a gradient that spans the subject's colour, the band keeps the
// background on its side. A free band loses that side and gives a mask of 100 %.
// Mutant: band pin off.
#[test]
fn arq_133_a_gradient_does_not_swallow_the_image() {
    let (w, h) = (1200, 900);
    let inside = |x: u32, y: u32| {
        let (dx, dy) = (f64::from(x) - 600.0, f64::from(y) - 450.0);
        dx * dx + dy * dy <= 260.0 * 260.0
    };
    let data = paint(w, h, |x, y| {
        if inside(x, y) {
            [150, 96, 60]
        } else {
            let v = (70 + x * 120 / w) as u8;
            [v, v, v]
        }
    });
    let (hit, extra) = foreground_share(w, h, &data, inside);
    assert!(hit > 0.9, "subject found: {hit}");
    assert!(extra < 0.1, "background kept: {extra}");
}

// ARQ-133: a subject touching the border loses at most what falls in pinned
// superpixels; the rest of it does not vanish, whether it touches one stretch of
// the border, a corner, or a whole side.
// Mutant: subject colours of the band train the background model.
#[test]
fn arq_133_a_subject_touching_the_border_does_not_vanish() {
    let (w, h) = (1200u32, 900u32);
    let disk = |cx: f64, cy: f64, r: f64| {
        move |x: u32, y: u32| {
            let (dx, dy) = (f64::from(x) - cx, f64::from(y) - cy);
            dx * dx + dy * dy <= r * r
        }
    };
    let whole_side = |_: u32, y: u32| y >= 620;
    type Inside<'a> = &'a dyn Fn(u32, u32) -> bool;
    let shapes: [(&str, Inside<'_>); 4] = [
        ("left edge", &disk(0.0, 450.0, 300.0)),
        ("corner", &disk(1200.0, 900.0, 480.0)),
        ("top edge", &disk(600.0, 0.0, 240.0)),
        ("whole side", &whole_side),
    ];
    for (name, inside) in shapes {
        let data = paint(w, h, |x, y| {
            if inside(x, y) {
                [190, 70, 60]
            } else {
                [70, 110, 150]
            }
        });
        let (hit, extra) = foreground_share(w, h, &data, inside);
        assert!(hit > 0.75, "{name}: kept {hit}");
        assert!(extra < 0.05, "{name}: extra {extra}");
    }
}
