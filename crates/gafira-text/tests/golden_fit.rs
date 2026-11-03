//! ARQ-14: `FitBox` gives the same size on the three platforms with the embedded fonts.
//!
//! The expected sizes are literals, measured once. CI runs this file on Linux, macOS and
//! Windows, so a platform that shapes or rounds differently fails here by name. Each case also
//! checks the rule of FD-46 (a) by plain layouts, so a wrong literal cannot be kept by mistake.

use gafira_geom::Size;
use gafira_testkit::fonts;
use gafira_text::{FitBox, FitRun, TextStyle, fit_box};

const INTER: &[u8] = fonts::INTER.bytes;
const MANROPE: &[u8] = fonts::MANROPE.bytes;
const ARABIC: &[u8] = fonts::NOTO_SANS_ARABIC.bytes;

const LOREM: &str = "Gafira lays text out in boxes: the size that fits is a derived value, found on a grid of a quarter of a point, and it must not depend on the machine that runs it.";

struct Golden {
    name: &'static str,
    runs: &'static [(&'static str, f64)],
    fonts: &'static [&'static [u8]],
    wh: (f64, f64),
    range: (f64, f64),
    dpi: f64,
    expected: (f64, bool),
}

const GOLDEN: &[Golden] = &[
    Golden {
        name: "inter, a paragraph",
        runs: &[(LOREM, 16.0)],
        fonts: &[INTER],
        wh: (320.0, 180.0),
        range: (6.0, 400.0),
        dpi: 96.0,
        expected: (18.5, false),
    },
    Golden {
        name: "manrope, a short title",
        runs: &[("Fit to the box", 16.0)],
        fonts: &[MANROPE],
        wh: (500.0, 120.0),
        range: (6.0, 400.0),
        dpi: 96.0,
        expected: (62.5, false),
    },
    Golden {
        name: "inter and arabic, mixed direction",
        runs: &[(
            "abc \u{627}\u{644}\u{639}\u{631}\u{628}\u{64a}\u{629} def \u{627}\u{628}\u{62c} ghi",
            16.0,
        )],
        fonts: &[INTER, ARABIC],
        wh: (260.0, 140.0),
        range: (6.0, 400.0),
        dpi: 96.0,
        expected: (24.75, false),
    },
    Golden {
        name: "two sizes, one to two",
        runs: &[("Small ", 10.0), ("and large text in a box", 20.0)],
        fonts: &[INTER],
        wh: (240.0, 130.0),
        range: (6.0, 400.0),
        dpi: 96.0,
        expected: (30.75, false),
    },
    Golden {
        name: "forced breaks, odd range, other dpi",
        runs: &[("one two\nthree\n\nfour five six", 16.0)],
        fonts: &[INTER],
        wh: (211.5, 97.25),
        range: (6.1, 90.3),
        dpi: 144.0,
        expected: (10.0, false),
    },
    Golden {
        name: "does not fit at the minimum",
        runs: &[(LOREM, 16.0)],
        fonts: &[INTER],
        wh: (60.0, 30.0),
        range: (12.0, 24.0),
        dpi: 96.0,
        expected: (12.0, true),
    },
    Golden {
        name: "always fits",
        runs: &[("Hi", 16.0)],
        fonts: &[INTER],
        wh: (900.0, 900.0),
        range: (6.0, 48.0),
        dpi: 72.0,
        expected: (48.0, false),
    },
];

/// Fits `runs` in the box with the range `min` to `max`.
fn fit_in(g: &Golden, min: f64, max: f64) -> gafira_text::FitResult {
    let runs: Vec<FitRun<'_>> = g
        .runs
        .iter()
        .map(|(text, size_pt)| FitRun {
            text,
            size_pt: *size_pt,
        })
        .collect();
    let style = TextStyle {
        size_pt: 12.0,
        ..TextStyle::default()
    };
    let fit = FitBox {
        min_size: min,
        max_size: max,
    };
    fit_box(
        &runs,
        g.fonts,
        &style,
        g.dpi,
        Size::new(g.wh.0, g.wh.1),
        &fit,
    )
    .expect("valid input")
}

#[test]
fn arq_14_fit_box_gives_the_same_size_on_every_platform() {
    for g in GOLDEN {
        let out = fit_in(g, g.range.0, g.range.1);
        assert_eq!(
            (out.size_pt, out.overflow),
            g.expected,
            "{}: {} pt",
            g.name,
            out.size_pt
        );
    }
}

#[test]
fn arq_14_each_golden_size_fits_and_the_next_one_does_not() {
    for g in GOLDEN {
        let (r, overflow) = g.expected;
        // The size itself: fits, or is the minimum that does not.
        let at = fit_in(g, r, r);
        assert_eq!(at.overflow, overflow, "{}: {r} pt", g.name);
        // The next size of the grid does not fit, unless it is past the maximum.
        if r < g.range.1 {
            let next = (((r / 0.25).floor() + 1.0) * 0.25).min(g.range.1);
            assert!(fit_in(g, next, next).overflow, "{}: {next} pt fits", g.name);
        }
    }
}
