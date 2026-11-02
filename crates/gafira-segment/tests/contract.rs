//! ARQ-130 shape and sizes, ARQ-110 inputs of `ClassicSegmenter::segment`.

use gafira_doc::contract::{CancelToken, NoProgress};
use gafira_doc::msg::MessageKey;
use gafira_raster::{BufferError, ImageView, PixelData, PixelFormat};
use gafira_segment::{ClassicSegmenter, SeedStroke, SegError, SegHints, SegZone, Segmenter};
use gafira_testkit::progress::ProgressLog;

fn seg() -> ClassicSegmenter {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    ClassicSegmenter::new(std::sync::Arc::new(pool))
}

fn rgba8(w: u32, h: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(w as usize * h as usize * 4);
    for i in 0..(w as usize * h as usize) {
        let on = (i % 7) as u8 * 30;
        v.extend_from_slice(&[on, 255 - on, 90, 255]);
    }
    v
}

fn run(w: u32, h: u32, zone: SegZone) -> gafira_segment::Segmentation {
    let data = rgba8(w, h);
    let view = ImageView::new(w, h, PixelData::Rgba8(&data)).unwrap();
    seg()
        .segment(view, &with_zone(zone), &NoProgress, &CancelToken::new())
        .unwrap()
}

// ARQ-130: the mask has the input's width and height.
#[test]
fn arq_130_mask_has_the_input_dimensions() {
    for (w, h) in [
        (1, 1),
        (1, 9),
        (9, 1),
        (800, 600),
        (2048, 1536),
        (4000, 3000),
    ] {
        for zone in ZONES {
            let mask = run(w, h, zone).mask;
            assert_eq!((mask.width(), mask.height()), (w, h), "{w}x{h} {zone:?}");
            assert_eq!(mask.data().len(), w as usize * h as usize);
        }
    }
}

// ARQ-130: width or height 0 is Err.
#[test]
fn arq_130_zero_size_is_err() {
    for (w, h) in [(0, 5), (5, 0), (0, 0)] {
        let view = ImageView::new(w, h, PixelData::Rgba8(&[])).unwrap();
        let r = seg().segment(view, &SegHints::default(), &NoProgress, &CancelToken::new());
        assert_eq!(
            r.unwrap_err(),
            SegError::EmptyImage {
                width: w,
                height: h
            }
        );
    }
}

const ZONES: [SegZone; 4] = [
    SegZone::All,
    SegZone::Foreground,
    SegZone::Background,
    SegZone::Element,
];

fn with_zone(zone: SegZone) -> SegHints {
    SegHints {
        zone,
        foreground: Vec::new(),
        background: Vec::new(),
    }
}

// ARQ-130: an Alpha8 image carries no colour: Err in every zone.
#[test]
fn arq_130_alpha8_is_unsupported() {
    let a = [0u8; 4];
    for zone in ZONES {
        let view = ImageView::new(2, 2, PixelData::Alpha8(&a)).unwrap();
        let r = seg().segment(view, &with_zone(zone), &NoProgress, &CancelToken::new());
        assert_eq!(
            r.unwrap_err(),
            SegError::UnsupportedFormat {
                format: PixelFormat::Alpha8
            },
            "{zone:?}"
        );
    }
}

// ARQ-130: a SegHints written with only the zone and the strokes compiles
// (the central priority is the zone Element, not a field).
#[test]
fn arq_130_hints_are_zone_and_strokes_only() {
    let hints = SegHints {
        zone: SegZone::Element,
        foreground: vec![],
        background: vec![],
    };
    assert_eq!(hints.zone, SegZone::Element);
}

/// 800x600 grey scene: a subject of 220 on a background of 40, its border a
/// linear ramp 16 px wide.
fn ramp_scene() -> Vec<u8> {
    let (x0, y0, x1, y1) = (250i32, 150i32, 550i32, 450i32);
    let mut v = Vec::new();
    for y in 0..600i32 {
        for x in 0..800i32 {
            let dx = (x0 - x).max(x - (x1 - 1)).max(0);
            let dy = (y0 - y).max(y - (y1 - 1)).max(0);
            let d = dx.max(dy).min(16);
            let g = (220 - 180 * d / 16) as u8;
            v.extend_from_slice(&[g, g, g, 255]);
        }
    }
    v
}

// ARQ-130 / FD-38 (e): a ramp edge gives a graded alpha, no separate matting pass.
#[test]
fn fd_38_a_ramp_edge_gives_graded_alpha() {
    let data = ramp_scene();
    let view = ImageView::new(800, 600, PixelData::Rgba8(&data)).unwrap();
    let m = seg()
        .segment(
            view,
            &with_zone(SegZone::Foreground),
            &NoProgress,
            &CancelToken::new(),
        )
        .unwrap()
        .mask;
    assert!(m.data()[300 * 800 + 400] >= 243, "subject");
    assert!(m.data()[10 * 800 + 10] <= 12, "background");
    let graded = m.data().iter().filter(|&&a| a > 0 && a < 255).count();
    assert!(graded > 0, "graded pixels: {graded}");
}

// ARQ-130: the colour is read as stored, over black: the same RGB with another
// alpha gives the same mask, bit for bit (Rgba8 and RgbaF32).
#[test]
fn arq_130_input_alpha_does_not_enter_the_mask() {
    let data = ramp_scene();
    let (w, h) = (800u32, 600u32);
    let mut faded = data.clone();
    for (i, px) in faded.chunks_exact_mut(4).enumerate() {
        px[3] = [0, 1, 77, 255][i % 4];
    }
    let mask = |d: &[u8]| {
        let view = ImageView::new(w, h, PixelData::Rgba8(d)).unwrap();
        seg()
            .segment(
                view,
                &with_zone(SegZone::Foreground),
                &NoProgress,
                &CancelToken::new(),
            )
            .unwrap()
            .mask
    };
    assert_eq!(mask(&data), mask(&faded));
    // The same for floats (a small image keeps the test short).
    let f = |alpha: f32| -> Vec<f32> {
        let mut v = Vec::new();
        for i in 0..(64 * 48) {
            let on = if (i % 64) % 40 < 20 { 0.8 } else { 0.1 };
            v.extend_from_slice(&[on, on * 0.5, 0.2, alpha]);
        }
        v
    };
    let float_mask = |px: &[f32]| {
        let view = ImageView::new(64, 48, PixelData::RgbaF32(px)).unwrap();
        seg()
            .segment(
                view,
                &with_zone(SegZone::Foreground),
                &NoProgress,
                &CancelToken::new(),
            )
            .unwrap()
            .mask
    };
    assert_eq!(float_mask(&f(1.0)), float_mask(&f(0.0)));
}

// ARQ-110: a non-finite pixel is Err naming the field and the element, in every
// zone and in every element of the pixel, alpha included.
#[test]
fn arq_110_non_finite_pixel_is_err() {
    for zone in ZONES {
        for index in 0..3 * 2 * 4 {
            for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                let mut px = vec![0.5f32; 3 * 2 * 4];
                px[index] = bad;
                let view = ImageView::new(3, 2, PixelData::RgbaF32(&px)).unwrap();
                let log = ProgressLog::new();
                let r = seg().segment(view, &with_zone(zone), &log, &CancelToken::new());
                assert_eq!(
                    r.unwrap_err(),
                    SegError::Buffer(BufferError::NonFinite {
                        field: "pixels",
                        index
                    }),
                    "{zone:?} {index} {bad}"
                );
                assert!(log.fractions().is_empty(), "no report before the checks");
            }
        }
    }
}

// ARQ-110: every numeric hint is checked before any work, in every zone (also
// All, where it takes no part) and also with a cancelled token.
#[test]
fn arq_110_non_finite_hint_is_err_before_any_work() {
    let data = rgba8(4, 4);
    let view = ImageView::new(4, 4, PixelData::Rgba8(&data)).unwrap();
    for zone in ZONES {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let cases: [(&str, SeedStroke, bool); 6] = [
                (
                    "hints.foreground.points",
                    SeedStroke {
                        points: vec![[1.0, 1.0], [bad, 1.0]],
                        radius: 1.0,
                    },
                    true,
                ),
                (
                    "hints.foreground.points",
                    SeedStroke {
                        points: vec![[1.0, bad]],
                        radius: 1.0,
                    },
                    true,
                ),
                (
                    "hints.foreground.radius",
                    SeedStroke {
                        points: vec![[1.0, 1.0]],
                        radius: bad,
                    },
                    true,
                ),
                (
                    "hints.background.points",
                    SeedStroke {
                        points: vec![[bad, 1.0]],
                        radius: 1.0,
                    },
                    false,
                ),
                (
                    "hints.background.points",
                    SeedStroke {
                        points: vec![[1.0, 1.0], [1.0, bad]],
                        radius: 1.0,
                    },
                    false,
                ),
                (
                    "hints.background.radius",
                    SeedStroke {
                        points: vec![[1.0, 1.0]],
                        radius: bad,
                    },
                    false,
                ),
            ];
            for (field, stroke, foreground) in cases {
                let ok = SeedStroke {
                    points: vec![[0.0, 0.0]],
                    radius: 1.0,
                };
                let mut hints = with_zone(zone);
                let list = if foreground {
                    &mut hints.foreground
                } else {
                    &mut hints.background
                };
                list.push(ok);
                list.push(stroke);
                let token = CancelToken::new();
                token.cancel();
                let log = ProgressLog::new();
                let r = seg().segment(view, &hints, &log, &token);
                assert_eq!(
                    r.unwrap_err(),
                    SegError::Buffer(BufferError::NonFinite { field, index: 1 }),
                    "{zone:?} {field} {bad}"
                );
                assert!(log.fractions().is_empty());
            }
        }
    }
}

// ARQ-130: the requirements state the working size of D-45.
#[test]
fn arq_130_requirements_state_the_working_size() {
    let r = seg().requirements();
    assert_eq!(r.max_work_side, 2048);
    assert!(!r.gpu);
}

// ARQ-130 / 02 §4.7: the requirements state an estimated RAM, from the working
// size: 16 bytes per pixel of a 2048 x 2048 copy, plus the guided filter's eight
// `f32` buffers of 282 rows (a 250-row band and its 32-row margin) of 2048.
#[test]
fn arq_130_requirements_estimate_ram_from_the_working_size() {
    let r = seg().requirements();
    let side = u64::from(r.max_work_side);
    assert_eq!(r.ram_bytes, side * side * 16 + 8 * 4 * 282 * side);
    assert_eq!(r.ram_bytes, 85_590_016);
}

// ARQ-130 / 02 §4.7: a `Segmenter` is a `Describe`, so a registry can hold it.
#[test]
fn arq_130_segmenter_describes_itself() {
    let s = seg();
    let dynamic: &dyn Segmenter = &s;
    let d = dynamic.descriptor();
    assert_eq!(d.id, "segment.classic");
    assert_eq!(d.name, MessageKey("segment.classic"));
    assert_eq!(d.version, 1);
}
