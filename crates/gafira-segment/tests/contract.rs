//! ARQ-130 shape and sizes, ARQ-110 inputs of `ClassicSegmenter::segment`.

use gafira_doc::contract::{CancelToken, NoProgress};
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

fn run(w: u32, h: u32) -> gafira_segment::Segmentation {
    let data = rgba8(w, h);
    let view = ImageView::new(w, h, PixelData::Rgba8(&data)).unwrap();
    seg()
        .segment(view, &SegHints::default(), &NoProgress, &CancelToken::new())
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
        let mask = run(w, h).mask;
        assert_eq!((mask.width(), mask.height()), (w, h), "{w}x{h}");
        assert_eq!(mask.data().len(), w as usize * h as usize);
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

// ARQ-130: an Alpha8 image carries no colour.
#[test]
fn arq_130_alpha8_is_unsupported() {
    let a = [0u8; 4];
    let view = ImageView::new(2, 2, PixelData::Alpha8(&a)).unwrap();
    let r = seg().segment(view, &SegHints::default(), &NoProgress, &CancelToken::new());
    assert_eq!(
        r.unwrap_err(),
        SegError::UnsupportedFormat {
            format: PixelFormat::Alpha8
        }
    );
}

// ARQ-110: a non-finite pixel is Err naming the field and the element.
#[test]
fn arq_110_non_finite_pixel_is_err() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut px = vec![0.5f32; 3 * 2 * 4];
        px[9] = bad;
        let view = ImageView::new(3, 2, PixelData::RgbaF32(&px)).unwrap();
        let log = ProgressLog::new();
        let r = seg().segment(view, &SegHints::default(), &log, &CancelToken::new());
        assert_eq!(
            r.unwrap_err(),
            SegError::Buffer(BufferError::NonFinite {
                field: "pixels",
                index: 9
            })
        );
        assert!(log.fractions().is_empty(), "no report before the checks");
    }
}

// ARQ-110: every numeric hint is checked before any work, also with a cancelled token.
#[test]
fn arq_110_non_finite_hint_is_err_before_any_work() {
    let data = rgba8(4, 4);
    let view = ImageView::new(4, 4, PixelData::Rgba8(&data)).unwrap();
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let cases: [(&str, SeedStroke, bool); 4] = [
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
            let mut hints = SegHints {
                zone: SegZone::Foreground,
                ..SegHints::default()
            };
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
                "{field} {bad}"
            );
            assert!(log.fractions().is_empty());
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
