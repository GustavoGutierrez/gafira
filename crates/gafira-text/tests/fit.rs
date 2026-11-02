//! FD-46: fit to the box. A search on the grid of 0.25 pt for the largest size at which the
//! text fits, with the minimum and the maximum as the ends.

use gafira_geom::Size;
use gafira_testkit::fonts;
use gafira_text::{
    FitBox, FitResult, FitRun, TextAlign, TextError, TextStyle, fit_box, layout_box,
};

const INTER: &[u8] = fonts::INTER.bytes;
const ARABIC: &[u8] = fonts::NOTO_SANS_ARABIC.bytes;
const DPI: f64 = 96.0;
const WORDS: [&str; 7] = ["gafira", "text", "box", "fit", "lorem", "ipsum", "dolor"];

fn lorem(chars: usize) -> String {
    let mut text = String::new();
    let mut i = 0;
    while text.len() < chars {
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(WORDS[i * 3 % WORDS.len()]);
        i += 1;
    }
    text.truncate(chars);
    text
}

fn style(align: TextAlign) -> TextStyle {
    TextStyle {
        size_pt: 12.0,
        align,
        ..TextStyle::default()
    }
}

fn range(min_size: f64, max_size: f64) -> FitBox {
    FitBox { min_size, max_size }
}

fn fit_in(
    text: &str,
    fonts: &[&[u8]],
    (w, h): (f64, f64),
    fit: FitBox,
) -> Result<FitResult, TextError> {
    let runs = [FitRun {
        text,
        size_pt: 16.0,
    }];
    fit_box(
        &runs,
        fonts,
        &style(TextAlign::Left),
        DPI,
        Size::new(w, h),
        &fit,
    )
}

fn fit(text: &str, wh: (f64, f64), fit: FitBox) -> FitResult {
    fit_in(text, &[INTER], wh, fit).expect("valid input")
}

/// Whether the text at `size` fits the box, by the plain layout.
fn fits(text: &str, fonts: &[&[u8]], (w, h): (f64, f64), size: f64) -> bool {
    let s = TextStyle {
        size_pt: size,
        ..style(TextAlign::Left)
    };
    let out = layout_box(text, fonts, &s, DPI, Size::new(w, h)).expect("valid input");
    f64::from(out.height) <= h && !out.overflow
}

/// The next size of the grid above `r`: the next multiple of 0.25, or `max`.
fn next_on_grid(r: f64, max: f64) -> Option<f64> {
    let up = ((r / 0.25).floor() + 1.0) * 0.25;
    let next = up.min(max);
    (next > r).then_some(next)
}

fn on_grid(r: f64, fit: &FitBox) -> bool {
    r == fit.min_size || r == fit.max_size || (r / 0.25).fract() == 0.0
}

struct Case {
    text: String,
    fonts: Vec<&'static [u8]>,
    wh: (f64, f64),
    fit: FitBox,
}

fn case(text: &str, wh: (f64, f64), min: f64, max: f64) -> Case {
    Case {
        text: text.to_string(),
        fonts: vec![INTER, ARABIC],
        wh,
        fit: range(min, max),
    }
}

fn cases() -> Vec<Case> {
    vec![
        case(&lorem(120), (200.0, 150.0), 6.0, 400.0),
        case(&lorem(500), (300.0, 200.0), 6.0, 400.0),
        case(&lorem(30), (120.0, 60.0), 6.1, 90.3),
        case("Hello", (500.0, 500.0), 1.0, 1000.0),
        case(
            "abc \u{627}\u{628}\u{62c} def \u{627}\u{628}",
            (150.0, 80.0),
            6.0,
            200.0,
        ),
        case("a b c\nd e f\n\ng h", (100.0, 100.0), 6.0, 400.0),
        case(&lorem(200), (80.0, 400.0), 8.0, 40.0),
        // A word that wraps by cluster: the width decides, not the height.
        case("WWWW", (100.0, 1e6), 6.0, 400.0),
        case("Hello world", (1e6, 40.0), 6.0, 400.0),
        case(&lorem(60), (47.3, 91.7), 7.3, 63.9),
        // Always fits: the maximum. Never fits: the minimum.
        case("a", (400.0, 400.0), 6.0, 40.0),
        case(&lorem(500), (50.0, 20.0), 6.0, 12.0),
    ]
}

#[test]
fn fd_46_a_the_result_fits_and_the_next_size_of_the_grid_does_not() {
    let mut at_max = 0;
    let mut at_min = 0;
    for c in cases() {
        let fonts: Vec<&[u8]> = c.fonts.clone();
        let out = fit_in(&c.text, &fonts, c.wh, c.fit).expect("valid input");
        let r = out.size_pt;
        assert!(on_grid(r, &c.fit), "{r} is on the grid of {:?}", c.fit);
        if r == c.fit.max_size {
            at_max += 1;
            assert!(
                fits(&c.text, &fonts, c.wh, r),
                "{:?} fits at its maximum",
                c.text
            );
        } else {
            let next = next_on_grid(r, c.fit.max_size).expect("below the maximum");
            assert!(
                !fits(&c.text, &fonts, c.wh, next),
                "{:?}: {next} fits",
                c.text
            );
            if r == c.fit.min_size {
                at_min += 1;
            } else {
                assert!(
                    fits(&c.text, &fonts, c.wh, r),
                    "{:?}: {r} does not fit",
                    c.text
                );
            }
        }
        assert_eq!(out.sizes_pt, vec![r]);
        assert_eq!(out.overflow, !fits(&c.text, &fonts, c.wh, r));
        // The layout is the one at that size.
        let s = TextStyle {
            size_pt: r,
            ..style(TextAlign::Left)
        };
        let boxed =
            layout_box(&c.text, &fonts, &s, DPI, Size::new(c.wh.0, c.wh.1)).expect("valid input");
        assert_eq!(out.layout, boxed);
    }
    assert!(
        at_max >= 1 && at_min >= 1,
        "{at_max} at the maximum, {at_min} at the minimum"
    );
}

#[test]
fn fd_46_a_a_height_equal_to_the_box_fits() {
    let text = lorem(80);
    let s = TextStyle {
        size_pt: 20.0,
        ..style(TextAlign::Left)
    };
    let height = f64::from(
        layout_box(&text, &[INTER], &s, DPI, Size::new(150.0, 1e6))
            .expect("valid input")
            .height,
    );
    let at = |h: f64| fit(&text, (150.0, h), range(6.0, 20.0)).size_pt;
    assert_eq!(at(height), 20.0, "equality fits");
    assert!(at(height - 1e-4) < 20.0, "a hair less does not");
}

#[test]
fn fd_46_a_white_space_is_a_text_with_a_height_and_grows_to_the_box() {
    // Three spaces are not empty text: the line has a height, so the search runs.
    let wh = (400.0, 200.0);
    let out = fit("   ", wh, range(6.0, 400.0));
    assert!(out.size_pt > 16.0, "grew from 16 pt, got {}", out.size_pt);
    assert!(!out.overflow);
    assert!(fits("   ", &[INTER], wh, out.size_pt));
    let next = next_on_grid(out.size_pt, 400.0).expect("not at the maximum");
    assert!(
        !fits("   ", &[INTER], wh, next),
        "the next size of the grid does not fit"
    );
}

#[test]
fn fd_46_a_a_cluster_wider_than_the_box_does_not_fit() {
    // 100 px wide, as tall as it gets: only the width stops the growth.
    let out = fit("WWWW", (100.0, 1e6), range(6.0, 400.0));
    let widest = out
        .layout
        .lines
        .iter()
        .flat_map(|l| &l.clusters)
        .map(|c| c.advance)
        .fold(0.0, f32::max);
    assert!(widest <= 100.0 && !out.overflow, "widest {widest}");
    assert!(out.size_pt < 400.0);
    let s = TextStyle {
        size_pt: next_on_grid(out.size_pt, 400.0).expect("below the maximum"),
        ..style(TextAlign::Left)
    };
    let bigger = layout_box("WWWW", &[INTER], &s, DPI, Size::new(100.0, 1e6)).expect("valid");
    assert!(bigger.overflow, "the next size overflows");
}

#[test]
fn fd_46_a_the_same_input_gives_the_same_result() {
    for c in cases() {
        let first = fit_in(&c.text, &[INTER, ARABIC], c.wh, c.fit).expect("valid input");
        for _ in 0..2 {
            let again = fit_in(&c.text, &[INTER, ARABIC], c.wh, c.fit).expect("valid input");
            assert_eq!(again, first);
        }
    }
}

#[test]
fn fd_46_b_empty_text_does_not_change_and_does_not_overflow() {
    for (wh, fit_range) in [
        ((0.0, 0.0), range(6.0, 400.0)),
        ((300.0, 10.0), range(30.0, 40.0)),
        ((300.0, 300.0), range(1.0, 5.0)),
    ] {
        let out = fit("", wh, fit_range);
        assert_eq!(out.size_pt, 16.0, "the size of the run stays");
        assert_eq!(out.sizes_pt, vec![16.0]);
        assert!(!out.overflow);
        assert_eq!(out.layout.lines.len(), 1);
    }
    // No runs at all: the size of the style stays.
    let none = fit_box(
        &[],
        &[INTER],
        &style(TextAlign::Left),
        DPI,
        Size::new(10.0, 10.0),
        &range(6.0, 400.0),
    )
    .expect("valid input");
    assert_eq!(none.size_pt, 12.0);
    assert!(none.sizes_pt.is_empty() && !none.overflow);
    // The font stack is still checked.
    let runs = [FitRun {
        text: "",
        size_pt: 16.0,
    }];
    let no_font = fit_box(
        &runs,
        &[],
        &style(TextAlign::Left),
        DPI,
        Size::new(10.0, 10.0),
        &range(6.0, 400.0),
    );
    assert_eq!(no_font.err(), Some(TextError::NoFont));
}

#[test]
fn fd_46_c_the_result_stays_between_the_minimum_and_the_maximum() {
    for c in cases() {
        let out = fit_in(&c.text, &[INTER, ARABIC], c.wh, c.fit).expect("valid input");
        assert!(
            (c.fit.min_size..=c.fit.max_size).contains(&out.size_pt),
            "{} in {:?}",
            out.size_pt,
            c.fit
        );
    }
    // Text that always fits gives the maximum, with no overflow.
    let out = fit("a", (400.0, 400.0), range(6.0, 40.0));
    assert_eq!((out.size_pt, out.overflow), (40.0, false));
    // Text that does not fit even at the minimum gives the minimum, with overflow.
    let long = lorem(500);
    let out = fit(&long, (50.0, 20.0), range(6.0, 12.0));
    assert_eq!((out.size_pt, out.overflow), (6.0, true));
    assert!(!fits(&long, &[INTER], (50.0, 20.0), 6.0));
    // The minimum does not fit by its width alone (a cluster wider than the box).
    let out = fit("WWWW", (3.0, 1e6), range(6.0, 12.0));
    assert_eq!((out.size_pt, out.overflow), (6.0, true));
    // The text that fits at the minimum only.
    let tight = fit(&lorem(200), (120.0, 90.0), range(6.0, 400.0));
    assert!(tight.size_pt > 6.0 && !tight.overflow);
}

#[test]
fn fd_46_e_equal_minimum_and_maximum_give_that_size() {
    for size in [1.0, 6.1, 13.0, 1000.0] {
        let out = fit("Hi", (50.0, 50.0), range(size, size));
        assert_eq!(out.size_pt, size);
        assert_eq!(out.overflow, !fits("Hi", &[INTER], (50.0, 50.0), size));
    }
    assert!(fit("Hi", (50.0, 50.0), range(1000.0, 1000.0)).overflow);
    assert!(!fit("Hi", (5000.0, 5000.0), range(30.0, 30.0)).overflow);
}

#[test]
fn fd_46_e_a_minimum_over_the_maximum_or_outside_1_to_1000_is_an_error() {
    let call = |min: f64, max: f64| fit_in("Hi", &[INTER], (50.0, 50.0), range(min, max)).err();
    let range_err = |field| Some(TextError::OutOfRange { field });
    assert_eq!(call(10.0, 9.99), range_err("fit.min_size"));
    assert_eq!(call(0.99, 10.0), range_err("fit.min_size"));
    assert_eq!(call(1000.01, 1001.0), range_err("fit.min_size"));
    assert_eq!(call(5.0, 1000.01), range_err("fit.max_size"));
    assert_eq!(
        call(1.0, 0.5),
        range_err("fit.max_size"),
        "the range comes first"
    );
    for (min, max) in [(1.0, 1.0), (1.0, 1000.0), (1000.0, 1000.0)] {
        assert_eq!(call(min, max), None, "{min} to {max}");
    }
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            call(bad, 10.0),
            Some(TextError::NonFinite {
                field: "fit.min_size"
            })
        );
        assert_eq!(
            call(5.0, bad),
            Some(TextError::NonFinite {
                field: "fit.max_size"
            })
        );
    }
    // The size of a run is a number of the call too.
    let run = |size: f64| {
        let runs = [FitRun {
            text: "Hi",
            size_pt: size,
        }];
        fit_box(
            &runs,
            &[INTER],
            &style(TextAlign::Left),
            DPI,
            Size::new(50.0, 50.0),
            &range(6.0, 40.0),
        )
        .err()
    };
    assert_eq!(
        run(f64::NAN),
        Some(TextError::NonFinite {
            field: "runs.size_pt"
        })
    );
    assert_eq!(run(0.99), range_err("runs.size_pt"));
    assert_eq!(run(1000.01), range_err("runs.size_pt"));
    assert_eq!(run(1.0), None);
    assert_eq!(run(1000.0), None);
}

fn two_runs(sizes: (f64, f64), wh: (f64, f64), fit: FitBox) -> FitResult {
    let runs = [
        FitRun {
            text: "aa ",
            size_pt: sizes.0,
        },
        FitRun {
            text: "aa bb cc dd",
            size_pt: sizes.1,
        },
    ];
    fit_box(
        &runs,
        &[INTER],
        &style(TextAlign::Left),
        DPI,
        Size::new(wh.0, wh.1),
        &fit,
    )
    .expect("valid input")
}

#[test]
fn fd_46_f_runs_of_10_and_20_keep_their_ratio_of_one_to_two() {
    for wh in [(120.0, 100.0), (300.0, 60.0), (90.0, 400.0), (40.0, 40.0)] {
        let out = two_runs((10.0, 20.0), wh, range(6.0, 400.0));
        assert_eq!(out.sizes_pt.len(), 2);
        assert_eq!(
            out.sizes_pt[1],
            2.0 * out.sizes_pt[0],
            "{wh:?}: {:?}",
            out.sizes_pt
        );
        assert_eq!(out.size_pt, out.sizes_pt[1], "the largest run decides");
        assert!(on_grid(out.size_pt, &range(6.0, 400.0)));
    }
    // The text is laid out at those sizes: the glyphs of the larger run are twice as wide.
    let out = two_runs((10.0, 20.0), (300.0, 200.0), range(6.0, 400.0));
    let clusters: Vec<_> = out.layout.lines.iter().flat_map(|l| &l.clusters).collect();
    let small = clusters
        .iter()
        .find(|c| c.range.start == 0)
        .expect("first a");
    let large = clusters
        .iter()
        .find(|c| c.range.start == 3)
        .expect("second a");
    assert!(
        (f64::from(large.advance) - 2.0 * f64::from(small.advance)).abs() <= 0.01,
        "{} and {}",
        small.advance,
        large.advance
    );
}

#[test]
fn fd_46_f_the_largest_run_decides_and_every_run_scales_in_proportion() {
    let runs = [
        FitRun {
            text: "ab ",
            size_pt: 5.0,
        },
        FitRun {
            text: "cd ",
            size_pt: 20.0,
        },
        FitRun {
            text: "ef gh",
            size_pt: 10.0,
        },
    ];
    let out = fit_box(
        &runs,
        &[INTER],
        &style(TextAlign::Left),
        DPI,
        Size::new(200.0, 60.0),
        &range(6.0, 400.0),
    )
    .expect("valid input");
    let r = out.size_pt;
    assert_eq!(out.sizes_pt, vec![r / 4.0, r, r / 2.0]);
    // A run that scales below 1 pt is still laid out.
    let small = two_runs((1.0, 1000.0), (50.0, 50.0), range(1.0, 1000.0));
    assert!(small.sizes_pt[0] < 1.0 && small.layout.height.is_finite());
    assert!(small.sizes_pt[0] > 0.0);
}

/// The widths of the clusters of a text at fixed sizes: the box is the whole of it and the range
/// of sizes is one value, so the sizes are the ones given.
fn at_sizes(runs: &[FitRun<'_>], t: f64, uppercase: bool, wh: (f64, f64), size: f64) -> FitResult {
    let s = TextStyle {
        letter_spacing: t,
        uppercase,
        ..style(TextAlign::Left)
    };
    fit_box(
        runs,
        &[INTER],
        &s,
        DPI,
        Size::new(wh.0, wh.1),
        &range(size, size),
    )
    .expect("valid input")
}

#[test]
fn fd_46_f_letter_spacing_is_in_em_of_each_run() {
    // 0.1 em after each cluster but the last, of the run of the cluster: 1.33 px at 10 pt,
    // 2.67 px at 20 pt (96 DPI). The run of the last cluster may be the smaller one.
    for sizes in [(10.0, 20.0), (20.0, 10.0)] {
        let runs = [
            FitRun {
                text: "aaa",
                size_pt: sizes.0,
            },
            FitRun {
                text: "aaa",
                size_pt: sizes.1,
            },
        ];
        let plain = at_sizes(&runs, 0.0, false, (1e6, 1e6), 20.0);
        let spaced = at_sizes(&runs, 100.0, false, (1e6, 1e6), 20.0);
        let (p, s) = (
            &plain.layout.lines[0].clusters,
            &spaced.layout.lines[0].clusters,
        );
        assert_eq!(p.len(), 6);
        for (i, (a, b)) in p.iter().zip(s).enumerate() {
            let size = if i < 3 { sizes.0 } else { sizes.1 };
            let extra = if i == 5 {
                0.0
            } else {
                0.1 * size * 96.0 / 72.0
            };
            assert!(
                (f64::from(b.advance - a.advance) - extra).abs() <= 0.01,
                "{sizes:?} cluster {i}: {} to {}",
                a.advance,
                b.advance
            );
        }
    }
}

#[test]
fn fd_46_f_a_run_with_no_text_does_not_set_the_largest_size() {
    // An empty run, however large, has nothing to fit: "Hello" gets the size it gets alone.
    let wh = (300.0, 120.0);
    let alone = fit_box(
        &[FitRun {
            text: "Hello",
            size_pt: 10.0,
        }],
        &[INTER],
        &style(TextAlign::Left),
        DPI,
        Size::new(wh.0, wh.1),
        &range(6.0, 400.0),
    )
    .expect("valid input");
    let with_empty = fit_box(
        &[
            FitRun {
                text: "",
                size_pt: 100.0,
            },
            FitRun {
                text: "Hello",
                size_pt: 10.0,
            },
        ],
        &[INTER],
        &style(TextAlign::Left),
        DPI,
        Size::new(wh.0, wh.1),
        &range(6.0, 400.0),
    )
    .expect("valid input");
    assert_eq!(with_empty.size_pt, alone.size_pt);
    assert_eq!(with_empty.sizes_pt[1], alone.size_pt);
    assert_eq!(with_empty.layout, alone.layout);
    // The proportion of an empty run can be huge and is still only a number.
    let huge = fit_box(
        &[
            FitRun {
                text: "",
                size_pt: 1000.0,
            },
            FitRun {
                text: "Hello",
                size_pt: 1.0,
            },
        ],
        &[INTER],
        &style(TextAlign::Left),
        DPI,
        Size::new(wh.0, wh.1),
        &range(6.0, 1000.0),
    )
    .expect("valid input");
    assert!(huge.layout.height.is_finite());
}

#[test]
fn fd_46_f_no_line_passes_the_box_when_the_sizes_and_the_spacing_differ() {
    // FD-46 purpose: no overflow. The spacing after the last cluster of a line is the one of
    // its run, which may be the smaller, so the break must not count the larger one.
    let (a, b) = ("aaa bbb ccc ddd eee ", "fff ggg hhh iii jjj");
    let text = format!("{a}{b}");
    for (sizes, t) in [
        ((20.0, 10.0), 500.0),
        ((10.0, 20.0), 500.0),
        ((20.0, 10.0), -100.0),
        ((10.0, 20.0), -100.0),
    ] {
        let runs = [
            FitRun {
                text: a,
                size_pt: sizes.0,
            },
            FitRun {
                text: b,
                size_pt: sizes.1,
            },
        ];
        for step in 0..200 {
            let width = 60.0 + f64::from(step) * 1.37;
            let out = at_sizes(&runs, t, false, (width, 1e6), 20.0);
            assert!(!out.overflow, "{sizes:?} t {t} at {width}");
            for line in &out.layout.lines {
                let ink: f64 = line
                    .clusters
                    .iter()
                    .rev()
                    .skip_while(|c| text[c.range.clone()].trim().is_empty())
                    .map(|c| f64::from(c.advance))
                    .sum();
                assert!(
                    ink <= width + 0.01,
                    "{sizes:?} t {t}: a line of {ink} px in a box of {width}"
                );
            }
        }
    }
}

#[test]
fn fd_46_f_runs_follow_the_upper_case_expansion_and_the_forced_breaks() {
    let reference = |text: &str, size: f64| {
        let s = TextStyle {
            size_pt: size,
            ..style(TextAlign::Left)
        };
        layout_box(text, &[INTER], &s, DPI, Size::new(1e6, 1e6))
            .expect("valid input")
            .lines[0]
            .clusters[0]
            .advance
    };
    // The ligature U+FB01 (3 bytes) becomes "FI" (2 bytes), so the byte ranges of the input and
    // of the upper case text differ. First run small, then large; and the other way round.
    for (first, second, expected) in [
        (("\u{fb01}", 10.0), ("a", 20.0), [10.0, 10.0, 20.0]),
        (("a", 20.0), ("\u{fb01}", 10.0), [20.0, 10.0, 10.0]),
    ] {
        let runs = [
            FitRun {
                text: first.0,
                size_pt: first.1,
            },
            FitRun {
                text: second.0,
                size_pt: second.1,
            },
        ];
        let out = at_sizes(&runs, 0.0, true, (1e6, 1e6), 20.0);
        let clusters = &out.layout.lines[0].clusters;
        let letters = if first.0 == "a" {
            ["A", "F", "I"]
        } else {
            ["F", "I", "A"]
        };
        assert_eq!(clusters.len(), 3);
        for ((c, letter), size) in clusters.iter().zip(letters).zip(expected) {
            assert_eq!(c.advance, reference(letter, size), "{letter} at {size}");
        }
    }
    // A run that crosses a forced break keeps its size in the next paragraph.
    let runs = [
        FitRun {
            text: "ab\nc",
            size_pt: 10.0,
        },
        FitRun {
            text: "d",
            size_pt: 20.0,
        },
    ];
    let out = at_sizes(&runs, 0.0, false, (1e6, 1e6), 20.0);
    let c = |line: usize, i: usize| out.layout.lines[line].clusters[i].advance;
    assert_eq!(c(0, 0), reference("a", 10.0));
    assert_eq!(c(1, 0), reference("c", 10.0));
    assert_eq!(c(1, 1), reference("d", 20.0));
}

/// The height of `text` at `size` in a column of `width`.
fn height_at(text: &str, width: f64, size: f64) -> f64 {
    let s = TextStyle {
        size_pt: size,
        ..style(TextAlign::Left)
    };
    f64::from(
        layout_box(text, &[INTER], &s, DPI, Size::new(width, 1e6))
            .expect("valid input")
            .height,
    )
}

#[test]
fn fd_46_c_the_multiple_of_a_quarter_below_a_maximum_that_is_not_one_is_on_the_grid() {
    // The grid is 6, 6.25, ..., 20.25 and 20.3: the box holds 20.25 and not 20.3.
    let text = lorem(80);
    let h = height_at(&text, 150.0, 20.25);
    assert!(height_at(&text, 150.0, 20.3) > h);
    let out = fit(&text, (150.0, h), range(6.0, 20.3));
    assert_eq!((out.size_pt, out.overflow), (20.25, false));
    // And the first multiple above a minimum that is not one: 6.25 after 6.1.
    let h = height_at(&text, 150.0, 6.25);
    let out = fit(&text, (150.0, h), range(6.1, 20.3));
    assert_eq!((out.size_pt, out.overflow), (6.25, false));
    let out = fit(&text, (150.0, h - 1e-4), range(6.1, 20.3));
    assert_eq!(
        (out.size_pt, out.overflow),
        (6.1, false),
        "6.1 fits, 6.25 does not"
    );
}

#[test]
fn fd_46_c_a_range_with_no_multiple_of_a_quarter_inside_has_two_sizes() {
    let text = lorem(80);
    let (min, max) = (6.05, 6.2);
    let h_min = height_at(&text, 150.0, min);
    let h_max = height_at(&text, 150.0, max);
    assert!(h_max > h_min);
    // Both fit: the maximum. Only the minimum fits: the minimum, with no overflow.
    let out = fit(&text, (150.0, h_max), range(min, max));
    assert_eq!((out.size_pt, out.overflow), (max, false));
    let out = fit(&text, (150.0, h_min), range(min, max));
    assert_eq!((out.size_pt, out.overflow), (min, false));
    // Neither: the minimum, with overflow.
    let out = fit(&text, (150.0, h_min - 1e-4), range(min, max));
    assert_eq!((out.size_pt, out.overflow), (min, true));
}

#[test]
fn arq_110_a_size_whose_layout_is_not_finite_is_too_big_and_the_minimum_is_an_error() {
    let call = |dpi: f64, min: f64, base: f64| {
        let runs = [FitRun {
            text: "ab\ncd\nef\ngh",
            size_pt: 16.0,
        }];
        fit_box(
            &runs,
            &[INTER],
            &TextStyle {
                size_pt: base,
                ..style(TextAlign::Left)
            },
            dpi,
            Size::new(1e300, 1e300),
            &range(min, 1000.0),
        )
    };
    // At 1000 pt and 5.2e36 DPI the height is infinite in f32; smaller sizes are finite.
    let out = call(5.2e36, 1.0, 12.0).expect("a finite size is found");
    assert!(out.size_pt < 1000.0 && !out.overflow && out.layout.height.is_finite());
    // Not even the minimum has a finite layout.
    assert_eq!(
        call(7.2e39, 1.0, 1.0).err(),
        Some(TextError::NonFinite { field: "output" })
    );
}
