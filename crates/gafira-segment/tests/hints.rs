//! ARQ-130: seed strokes and zones of `SegHints`.

use std::sync::Arc;

use gafira_doc::contract::{CancelToken, NoProgress};
use gafira_raster::{AlphaMask, ImageView, PixelData};
use gafira_segment::{ClassicSegmenter, SeedStroke, SegHints, SegZone, Segmenter};
use gafira_testkit::progress::{ProgressLog, check_progress_contract};

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

// ARQ-130: in Primer plano and Elemento, under a foreground seed alpha >= 243,
// under a background seed <= 12.
#[test]
fn arq_130_seed_strokes_set_the_alpha_under_them() {
    let data = scene(&[CENTRE]);
    for zone in [SegZone::Foreground, SegZone::Element] {
        let hints = SegHints {
            zone,
            foreground: vec![dot(30.0, 75.0)],
            background: vec![dot(100.0, 75.0)],
        };
        let m = run(&data, &hints);
        assert!(
            at(&m, 30, 75) >= 243,
            "{zone:?} foreground seed on grey: {}",
            at(&m, 30, 75)
        );
        assert!(
            at(&m, 100, 75) <= 12,
            "{zone:?} background seed on the subject: {}",
            at(&m, 100, 75)
        );
    }
}

// ARQ-130: a pixel under seeds of both kinds gets >= 243 (foreground wins), and
// two adjacent pixels with seeds of different kinds each keep their bound.
#[test]
fn arq_130_foreground_wins_an_overlap_and_neighbours_keep_their_bounds() {
    let data = scene(&[CENTRE]);
    for zone in [SegZone::Foreground, SegZone::Element] {
        let both = SegHints {
            zone,
            foreground: vec![dot(30.0, 75.0)],
            background: vec![dot(30.0, 75.0)],
        };
        let m = run(&data, &both);
        assert!(
            at(&m, 30, 75) >= 243,
            "{zone:?} overlap: {}",
            at(&m, 30, 75)
        );
        let point = |x: f32, y: f32| SeedStroke {
            points: vec![[x, y]],
            radius: 0.0,
        };
        // Pixels 30 and 31 of row 75, on the grey; then the other way round, on the subject.
        for (fx, bx, row_x) in [(30.5, 31.5, 30), (31.5, 30.5, 30), (99.5, 100.5, 99)] {
            let pair = SegHints {
                zone,
                foreground: vec![point(fx, 75.5)],
                background: vec![point(bx, 75.5)],
            };
            let m = run(&data, &pair);
            let (f, b) = if fx < bx {
                (row_x, row_x + 1)
            } else {
                (row_x + 1, row_x)
            };
            assert!(
                at(&m, f, 75) >= 243,
                "{zone:?} fg pixel {f}: {}",
                at(&m, f, 75)
            );
            assert!(
                at(&m, b, 75) <= 12,
                "{zone:?} bg pixel {b}: {}",
                at(&m, b, 75)
            );
        }
    }
}

// ARQ-130: Todo ignores the seeds: solid 255, with or without them, also under a background seed.
#[test]
fn arq_130_all_ignores_the_seeds() {
    let data = scene(&[CENTRE]);
    let hints = SegHints {
        zone: SegZone::All,
        foreground: vec![dot(30.0, 75.0)],
        background: vec![dot(100.0, 75.0)],
    };
    let m = run(&data, &hints);
    assert!(m.data().iter().all(|&a| a == 255));
    assert_eq!(at(&m, 100, 75), 255);
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

// ARQ-130: Fondo is the exact complement of Primer plano with the same image and
// seeds: 255 - alpha for every pixel, no tolerance; All is solid.
#[test]
fn arq_130_background_is_the_exact_complement_and_all_is_solid() {
    let data = scene(&[CENTRE]);
    let seeds = |zone| SegHints {
        zone,
        foreground: vec![dot(30.0, 75.0)],
        background: vec![dot(100.0, 75.0)],
    };
    for hints in [
        SegHints {
            zone: SegZone::Foreground,
            ..SegHints::default()
        },
        seeds(SegZone::Foreground),
    ] {
        let fg = run(&data, &hints);
        let bg = run(
            &data,
            &SegHints {
                zone: SegZone::Background,
                ..hints.clone()
            },
        );
        for (i, (a, b)) in fg.data().iter().zip(bg.data()).enumerate() {
            assert_eq!(255 - a, *b, "pixel {i}");
        }
        assert!(fg.data().iter().any(|&a| a >= 128), "there is a foreground");
        if !hints.foreground.is_empty() {
            assert!(at(&bg, 30, 75) <= 12, "under a foreground seed");
            assert!(at(&bg, 100, 75) >= 243, "under a background seed");
        }
    }
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

fn paint(data: &mut [u8], (x0, y0, x1, y1): (u32, u32, u32, u32), colour: [u8; 3]) {
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * W + x) * 4) as usize;
            data[i..i + 3].copy_from_slice(&colour);
        }
    }
}

// ARQ-130 / D-23: Elemento drops the other object even when it is larger than the
// central one, in the same colour or in another; Primer plano keeps both.
#[test]
fn arq_130_element_keeps_the_central_object_even_when_the_other_is_larger() {
    let small_centre = (90, 60, 110, 90);
    let big_side = (10, 20, 60, 130);
    for side_colour in [[200, 40, 40], [30, 60, 200]] {
        let mut data = scene(&[small_centre]);
        paint(&mut data, big_side, side_colour);
        let fg = run(
            &data,
            &SegHints {
                zone: SegZone::Foreground,
                ..SegHints::default()
            },
        );
        assert!(at(&fg, 100, 75) >= 128 && at(&fg, 35, 75) >= 128, "both");
        let el = run(
            &data,
            &SegHints {
                zone: SegZone::Element,
                ..SegHints::default()
            },
        );
        assert!(at(&el, 100, 75) >= 128, "centre kept {side_colour:?}");
        assert!(
            at(&el, 35, 75) < 128,
            "larger side object dropped {side_colour:?}: {}",
            at(&el, 35, 75)
        );
    }
}

/// A `w` x `h` image whose long axis carries a red band at `[1900, 2100)`, a blue
/// decoy at `[3886, 3914)` and grey elsewhere; `tall` swaps the axes.
fn strip(tall: bool) -> (u32, u32, Vec<u8>) {
    let (w, h) = if tall { (33, 4000) } else { (4000, 33) };
    let mut v = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let l = if tall { y } else { x };
            v.extend_from_slice(match l {
                1900..2100 => &[200, 40, 40, 255],
                3886..3914 => &[30, 60, 200, 255],
                _ => &[120, 120, 120, 255],
            });
        }
    }
    (w, h, v)
}

// ARQ-130 / FD-38: a seed lands on the right superpixels of the reduced copy
// on a very elongated image, in both orientations (33x4000 and 4000x33).
#[test]
fn arq_130_a_seed_steers_the_cut_on_elongated_images_in_both_orientations() {
    for tall in [true, false] {
        let (w, h, data) = strip(tall);
        let px = |m: &AlphaMask, l: u32| {
            let (x, y) = if tall { (16, l) } else { (l, 16) };
            m.data()[(y * w + x) as usize]
        };
        let view = ImageView::new(w, h, PixelData::Rgba8(&data)).unwrap();
        let go = |hints: &SegHints| {
            seg()
                .segment(view, hints, &NoProgress, &CancelToken::new())
                .unwrap()
                .mask
        };
        let plain = go(&SegHints {
            zone: SegZone::Foreground,
            ..SegHints::default()
        });
        assert!(
            px(&plain, 3890) >= 128 && px(&plain, 3910) >= 128,
            "tall={tall}: the decoy is foreground: {} {}",
            px(&plain, 3890),
            px(&plain, 3910)
        );
        let l = 3900.0;
        let centre = if tall { [16.0, l] } else { [l, 16.0] };
        let seeded = go(&SegHints {
            zone: SegZone::Foreground,
            background: vec![SeedStroke {
                points: vec![centre],
                radius: 2.0,
            }],
            ..SegHints::default()
        });
        assert!(
            px(&seeded, 3890) < 128 && px(&seeded, 3910) < 128,
            "tall={tall}: beyond the dot: {} {}",
            px(&seeded, 3890),
            px(&seeded, 3910)
        );
        assert!(px(&seeded, 2000) >= 128, "tall={tall}: subject kept");
    }
}

// ARQ-110: a finite stroke never turns into a non-finite intermediate: a stroke
// across the image, or a brush the size of the float range, is applied.
#[test]
fn arq_110_extreme_finite_strokes_are_applied() {
    let (w, h) = (64u32, 48u32);
    let mut data = Vec::new();
    for i in 0..(w * h) {
        let on = if (i % w) < 32 { 90 } else { 160 };
        data.extend_from_slice(&[on, 100, 100, 255]);
    }
    let view = ImageView::new(w, h, PixelData::Rgba8(&data)).unwrap();
    let go = |stroke: SeedStroke| {
        seg()
            .segment(
                view,
                &SegHints {
                    zone: SegZone::Foreground,
                    foreground: vec![stroke],
                    background: vec![],
                },
                &NoProgress,
                &CancelToken::new(),
            )
            .unwrap()
            .mask
    };
    let m = f32::MAX;
    for (a, b) in [
        ([-1e19, 10.0], [1e19, 10.0]),
        ([-1e20, 10.0], [1e20, 10.0]),
        ([m, 10.0], [-m, 10.0]),
        ([-1.3e19, 10.0], [2.7e19, 10.0]),
        ([3e38, 10.0], [-1.7e38, 10.0]),
    ] {
        let mask = go(SeedStroke {
            points: vec![a, b],
            radius: 1.0,
        });
        for x in 0..w {
            assert!(
                mask.data()[(10 * w + x) as usize] >= 243,
                "{a:?} {b:?} x={x}"
            );
        }
    }
    for (a, b) in [
        ([10.0, -1e19], [10.0, 1e19]),
        ([10.0, m], [10.0, -m]),
        ([10.0, -1.3e19], [10.0, 2.7e19]),
    ] {
        let mask = go(SeedStroke {
            points: vec![a, b],
            radius: 1.0,
        });
        for y in 0..h {
            assert!(
                mask.data()[(y * w + 10) as usize] >= 243,
                "{a:?} {b:?} y={y}"
            );
        }
    }
    for radius in [1e30, m] {
        let mask = go(SeedStroke {
            points: vec![[3.0, 3.0]],
            radius,
        });
        assert!(mask.data().iter().all(|&a| a >= 243), "radius {radius}");
    }
    // Far outside the image: nothing to cover, and no error.
    let _ = go(SeedStroke {
        points: vec![[1e30, 0.0]],
        radius: 0.0,
    });
}

// FD-38 (c) / ARQ-112: a stroke is worked in chunks of rows, and the token is
// read and progress reported at each chunk, not once per stroke: a long brush
// over a tall image adds one report per chunk, and cancelling at any report ends
// the run, the ones inside the stroke included.
#[test]
fn fd_38_a_stroke_reports_progress_and_checks_the_token_per_chunk() {
    let (w, h) = (128u32, 1100u32);
    let mut data = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let inside = (40..90).contains(&x) && (400..700).contains(&y);
            data.extend_from_slice(if inside {
                &[200, 40, 40, 255]
            } else {
                &[120, 120, 120, 255]
            });
        }
    }
    let view = ImageView::new(w, h, PixelData::Rgba8(&data)).unwrap();
    let stroke = SeedStroke {
        points: vec![[64.0, 20.0], [64.0, 1080.0]],
        radius: 6.0,
    };
    let run = |token: &CancelToken, progress: &dyn gafira_doc::contract::Progress, seeds: bool| {
        let hints = SegHints {
            zone: SegZone::Foreground,
            foreground: if seeds { vec![stroke.clone()] } else { vec![] },
            background: vec![],
        };
        seg().segment(view, &hints, progress, token)
    };
    let count = |seeds: bool| {
        let log = ProgressLog::new();
        run(&CancelToken::new(), &log, seeds).unwrap();
        log.fractions().len()
    };
    // Two stamps (the pins at work size and the final one), 17 chunks of 64 rows each
    // (rows 14 to 1086).
    assert!(
        count(true) >= count(false) + 2 * 17,
        "{} against {}",
        count(true),
        count(false)
    );
    let verdict =
        check_progress_contract(1.0, |token, progress| match run(token, progress, true) {
            Ok(_) => Ok(false),
            Err(gafira_segment::SegError::Cancelled(_)) => Ok(true),
            Err(e) => Err(e),
        })
        .unwrap();
    assert_eq!(verdict, Ok(()));
}

// ARQ-110: an extreme stroke is applied along its whole line, not only near its
// origin or from where it was cut: at 1e19 one step of an `f64` is 2048 pixels,
// so the stroke is cut to the image with the cut coordinate exact. Run once as a
// foreground and once as a background stroke: no pixel can already be both.
#[test]
fn arq_110_extreme_strokes_are_applied_along_their_whole_line() {
    for (w, h) in [
        (2049u32, 9u32),
        (9, 2049),
        (100, 100),
        (33, 4000),
        (4000, 33),
    ] {
        let mut data = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let inside = (x as f32 / w as f32 - 0.5).abs() < 0.2
                    && (y as f32 / h as f32 - 0.5).abs() < 0.2;
                data.extend_from_slice(if inside {
                    &[200, 40, 40, 255]
                } else {
                    &[120, 120, 120, 255]
                });
            }
        }
        let view = ImageView::new(w, h, PixelData::Rgba8(&data)).unwrap();
        let go = |foreground: Vec<SeedStroke>, background: Vec<SeedStroke>| {
            seg()
                .segment(
                    view,
                    &SegHints {
                        zone: SegZone::Foreground,
                        foreground,
                        background,
                    },
                    &NoProgress,
                    &CancelToken::new(),
                )
                .unwrap()
                .mask
        };
        for (a, b) in [(-1.3e19f32, 2.7e19f32), (3e38, -1.7e38)] {
            for along_x in [true, false] {
                let points = if along_x {
                    vec![[a, 1.0], [b, 1.0]]
                } else {
                    vec![[1.0, a], [1.0, b]]
                };
                let stroke = SeedStroke {
                    points,
                    radius: 1.0,
                };
                let fg = go(vec![stroke.clone()], vec![]);
                let bg = go(vec![], vec![stroke]);
                for l in 0..(if along_x { w } else { h }) {
                    let (x, y) = if along_x { (l, 1) } else { (1, l) };
                    let i = (y * w + x) as usize;
                    let what = format!("{w}x{h} {a} {b} along_x={along_x} at {x},{y}");
                    assert!(fg.data()[i] >= 243, "{what}: fg {}", fg.data()[i]);
                    assert!(bg.data()[i] <= 12, "{what}: bg {}", bg.data()[i]);
                }
            }
        }
    }
}
