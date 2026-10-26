//! Oracles for the generated documents: colour coverage and px page-side bands.
//!
//! Both sample `document()` with a deterministic runner and a fixed number of
//! draws, so the verdict does not depend on `PROPTEST_CASES`.

use std::sync::OnceLock;

use gafira_color::Color;
use gafira_doc::{Document, NodeKind};
use gafira_geom::{Length, Unit};
use gafira_testkit::document;
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;

const COLOR_DRAWS: usize = 2_000;
const SIDE_DRAWS: usize = 20_000;
/// A band is reached for sure, not by chance of the random mode.
const MIN_PER_BAND: usize = 10;

fn draws(n: usize) -> impl Iterator<Item = Document> {
    let strategy = document();
    let mut runner = TestRunner::deterministic();
    (0..n).map(move |_| strategy.new_tree(&mut runner).expect("tree").current())
}

#[derive(Default)]
struct Spaces {
    srgb: usize,
    linear: usize,
    p3: usize,
    negative: [usize; 3],
}

fn fills() -> Spaces {
    let mut seen = Spaces::default();
    for doc in draws(COLOR_DRAWS) {
        for page in doc.pages() {
            for node in page.nodes.values() {
                let NodeKind::Shape(shape) = &node.kind else {
                    continue;
                };
                let Some(fill) = shape.fill else { continue };
                let (space, [r, g, b]) = match fill {
                    Color::Srgb(c) => (&mut seen.srgb, [c.r, c.g, c.b]),
                    Color::LinearSrgb(c) => (&mut seen.linear, [c.r, c.g, c.b]),
                    Color::DisplayP3(c) => (&mut seen.p3, [c.r, c.g, c.b]),
                };
                *space += 1;
                for (count, channel) in seen.negative.iter_mut().zip([r, g, b]) {
                    if channel < 0.0 {
                        *count += 1;
                    }
                }
            }
        }
    }
    seen
}

#[test]
fn every_colour_space_is_generated() {
    let seen = fills();
    assert!(seen.srgb > 0, "no sRGB fill");
    assert!(seen.linear > 0, "no linear sRGB fill");
    assert!(seen.p3 > 0, "no Display-P3 fill");
}

#[test]
fn negative_channels_are_generated() {
    let seen = fills();
    for (name, count) in ["red", "green", "blue"].into_iter().zip(seen.negative) {
        assert!(count > 0, "no fill with a negative {name} channel");
    }
}

#[derive(Default, Debug)]
struct Bands {
    lower: usize,
    upper: usize,
    above_lower: usize,
    below_upper: usize,
    random: usize,
}

/// The px-unit sides of the pages of the deterministic draws, counted per band.
fn bands() -> &'static Bands {
    static BANDS: OnceLock<Bands> = OnceLock::new();
    BANDS.get_or_init(|| {
        let mut bands = Bands::default();
        for doc in draws(SIDE_DRAWS) {
            for page in doc.pages() {
                let sides: [Length; 2] = [page.size.width, page.size.height];
                for side in sides.into_iter().filter(|s| s.unit() == Unit::Px) {
                    let v = side.value();
                    if v == 1.0 {
                        bands.lower += 1;
                    } else if v == 16_384.0 {
                        bands.upper += 1;
                    } else if v > 1.0 && v < 2.0 {
                        bands.above_lower += 1;
                    } else if v > 16_383.0 && v < 16_384.0 {
                        bands.below_upper += 1;
                    } else if v > 2.0 && v < 16_383.0 && v != 100.0 {
                        bands.random += 1;
                    }
                }
            }
        }
        bands
    })
}

#[test]
fn px_pages_reach_both_exact_limits() {
    let b = bands();
    assert!(b.lower >= MIN_PER_BAND, "px sides of 1.0: {}", b.lower);
    assert!(b.upper >= MIN_PER_BAND, "px sides of 16384.0: {}", b.upper);
}

#[test]
fn px_pages_reach_the_bands_next_to_the_limits() {
    let b = bands();
    assert!(
        b.above_lower >= MIN_PER_BAND,
        "px sides in (1, 2): {}",
        b.above_lower
    );
    assert!(
        b.below_upper >= MIN_PER_BAND,
        "px sides in (16383, 16384): {}",
        b.below_upper
    );
}

#[test]
fn px_pages_reach_the_random_band() {
    let b = bands();
    assert!(
        b.random >= MIN_PER_BAND,
        "px sides in (2, 16383): {}",
        b.random
    );
}
