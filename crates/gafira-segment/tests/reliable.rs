//! ARQ-131: `reliable` through the public contract.

use std::sync::Arc;

use gafira_doc::contract::{CancelToken, NoProgress};
use gafira_raster::{ImageView, PixelData};
use gafira_segment::{ClassicSegmenter, SegHints, SegZone, Segmentation, Segmenter};
use gafira_testkit::scenes::{fixed_scene, reference_scene};

fn seg() -> ClassicSegmenter {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    ClassicSegmenter::new(Arc::new(pool))
}

fn run(w: u32, h: u32, data: &[u8], zone: SegZone) -> Segmentation {
    let view = ImageView::new(w, h, PixelData::Rgba8(data)).unwrap();
    let hints = SegHints {
        zone,
        ..SegHints::default()
    };
    seg()
        .segment(view, &hints, &NoProgress, &CancelToken::new())
        .unwrap()
}

// ARQ-131: a scene of the fixed set gives a reliable result.
#[test]
fn arq_131_a_fixed_scene_is_reliable() {
    let s = reference_scene();
    let out = run(s.width, s.height, &s.rgba, SegZone::Foreground);
    assert!(out.reliable);
    assert_eq!(out.mask.data().len(), (s.width * s.height) as usize);
}

// ARQ-131: a uniform image, per-pixel noise with a fixed seed, a 1x1 image and
// a subject of 0.5 % are `Ok`, not reliable, and keep the input's mask size.
#[test]
fn arq_131_uniform_noise_tiny_subject_and_1x1_are_unreliable() {
    let (w, h) = (200u32, 150u32);
    let uniform: Vec<u8> = (0..w * h).flat_map(|_| [120, 130, 140, 255]).collect();
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    let noise: Vec<u8> = (0..w * h)
        .flat_map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            [state as u8, (state >> 8) as u8, (state >> 16) as u8, 255]
        })
        .collect();
    // A red square of 0.5 % of the area (150 of 30000 pixels: 12 x 12).
    let tiny: Vec<u8> = (0..w * h)
        .flat_map(|i| {
            let (x, y) = (i % w, i / w);
            if (94..106).contains(&x) && (69..81).contains(&y) {
                [220, 20, 20, 255]
            } else {
                [120, 120, 120, 255]
            }
        })
        .collect();
    for (name, data) in [("uniform", uniform), ("noise", noise), ("tiny", tiny)] {
        let out = run(w, h, &data, SegZone::Foreground);
        assert!(!out.reliable, "{name}");
        assert_eq!(out.mask.data().len(), (w * h) as usize, "{name}");
    }
    let out = run(1, 1, &[9, 9, 9, 255], SegZone::Foreground);
    assert!(!out.reliable);
    assert_eq!(out.mask.data().len(), 1);
}

// ARQ-131 / FD-32: `All` is a solid mask, the foreground covers 100 %, so it is
// not reliable; the result is `Ok` and keeps its mask.
#[test]
fn arq_131_all_zone_is_solid_and_not_reliable() {
    let s = reference_scene();
    let out = run(s.width, s.height, &s.rgba, SegZone::All);
    assert!(!out.reliable);
    assert!(out.mask.data().iter().all(|&a| a == 255));
}

// ARQ-131: the input alpha weights the colours, so a transparent copy of a good
// scene has no mean colour and is not reliable, while its mask is the same.
#[test]
fn arq_131_input_alpha_weights_reliability_not_the_mask() {
    let s = fixed_scene(0, 400, 300);
    let clear: Vec<u8> = s
        .rgba
        .chunks(4)
        .flat_map(|p| [p[0], p[1], p[2], 0])
        .collect();
    let opaque = run(400, 300, &s.rgba, SegZone::Foreground);
    let transparent = run(400, 300, &clear, SegZone::Foreground);
    assert!(opaque.reliable);
    assert!(!transparent.reliable);
    assert_eq!(opaque.mask, transparent.mask);
}
