//! ARQ-130: the pipeline isolates a centred subject on a flat background.

use std::sync::Arc;

use gafira_doc::contract::{CancelToken, NoProgress};
use gafira_raster::{ImageView, PixelData};
use gafira_segment::{ClassicSegmenter, SegHints, Segmenter};

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

// ARQ-130: with no hints the subject gets alpha >= 128 and the background < 128.
#[test]
fn arq_130_isolates_a_centred_subject_without_hints() {
    for (w, h) in [(64, 48), (200, 150), (800, 600), (4000, 3000)] {
        let data = scene(w, h, 0.4);
        let view = ImageView::new(w, h, PixelData::Rgba8(&data)).unwrap();
        let out = seg()
            .segment(view, &SegHints::default(), &NoProgress, &CancelToken::new())
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
