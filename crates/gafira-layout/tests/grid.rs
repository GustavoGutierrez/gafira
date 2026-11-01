//! FD-07 (a, c, d, e), ARQ-110, ARQ-111 for the visible grid lines.

use gafira_layout::{
    GridLine, GridSettings, GridStyle, LayoutError, Length, Rect, Unit, grid_lines,
};
use proptest::prelude::*;

const BAD: [f64; 3] = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];

fn grid(step: f64, unit: Unit, subdivisions: u8) -> GridSettings {
    GridSettings {
        visible: true,
        step: Length::new(step, unit),
        subdivisions,
        style: GridStyle::Lines,
        snap: true,
    }
}

fn px(step: f64, subdivisions: u8) -> GridSettings {
    grid(step, Unit::Px, subdivisions)
}

fn view(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
    Rect::new(x0, y0, x1, y1)
}

fn xs(lines: &[GridLine]) -> Vec<f64> {
    lines.iter().map(|l| l.position).collect()
}

fn vertical(g: &GridSettings, v: Rect, zoom: f64) -> Vec<GridLine> {
    grid_lines(g, v, zoom, 96.0).unwrap().vertical
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 0.01
}

/// The view that a 3840 x 2160 screen shows at `zoom`, from `(x, y)`.
fn screen(x: f64, y: f64, zoom: f64) -> Rect {
    view(x, y, x + 3840.0 / zoom, y + 2160.0 / zoom)
}

#[test]
fn fd_07_d_ten_px_step_with_five_subdivisions() {
    let g = px(10.0, 5);
    let v = view(0.0, 0.0, 200.0, 100.0);
    // 24.9 %: neither level fits (2.49 and 0.5 px); the smallest 10 * 2^k >= 6 px is 40.
    let at = vertical(&g, v, 0.249);
    assert_eq!(xs(&at), [0.0, 40.0, 80.0, 120.0, 160.0, 200.0]);
    assert!(at.iter().all(|l| l.major));
    // 100 %: every 10 px, not every 2 px.
    let at = vertical(&g, v, 1.0);
    assert_eq!(at.len(), 21);
    assert!(near(at[1].position, 10.0) && at.iter().all(|l| l.major));
    // 500 %: every 2 px, the 10 px lines are the major ones.
    let at = vertical(&g, view(0.0, 0.0, 20.0, 20.0), 5.0);
    assert_eq!(at.len(), 11);
    for (i, l) in at.iter().enumerate() {
        assert!(near(l.position, 2.0 * i as f64));
        assert_eq!(l.major, i % 5 == 0);
    }
    // the horizontal lines come from the y range
    let both = grid_lines(&g, view(0.0, 0.0, 100.0, 20.0), 1.0, 96.0).unwrap();
    assert_eq!((both.vertical.len(), both.horizontal.len()), (11, 3));
}

#[test]
fn fd_07_minor_grid_needs_25_percent_zoom_and_6_px_spacing() {
    let v = view(0.0, 0.0, 2000.0, 100.0);
    let spacing = |g: &GridSettings, zoom: f64| {
        let l = xs(&vertical(g, v, zoom));
        l[1] - l[0]
    };
    // minor of 12.5 screen px: shown at 25 %, hidden below it
    assert_eq!(spacing(&px(100.0, 2), 0.25), 50.0);
    assert_eq!(spacing(&px(100.0, 2), 0.2499), 100.0);
    // minor of 10 screen px at 20 %: the zoom rule hides it though it would fit
    assert_eq!(spacing(&px(500.0, 10), 0.2), 500.0);
    // exactly 6 screen px is shown; a hair less is not
    assert_eq!(spacing(&px(12.0, 2), 1.0), 6.0);
    assert_eq!(spacing(&px(11.9, 2), 1.0), 11.9);
    assert_eq!(spacing(&px(12.0, 1), 0.5), 12.0);
    // major of exactly 6 screen px is shown without falling back to 2^k
    assert_eq!(spacing(&px(6.0, 1), 1.0), 6.0);
    assert_eq!(spacing(&px(5.9, 1), 1.0), 11.8);
    // a doubled step that lands exactly on 6 px is the smallest one that fits
    assert_eq!(spacing(&px(3.0, 1), 1.0), 6.0);
    // one subdivision: major and minor are the same lines, listed once
    let lines = vertical(&px(10.0, 1), view(0.0, 0.0, 30.0, 1.0), 1.0);
    assert_eq!(xs(&lines), [0.0, 10.0, 20.0, 30.0]);
}

#[test]
fn fd_07_lines_cover_a_view_that_starts_off_the_page() {
    let lines = vertical(&px(10.0, 1), view(-25.0, 0.0, 12.0, 1.0), 1.0);
    assert_eq!(xs(&lines), [-20.0, -10.0, 0.0, 10.0]);
}

proptest! {
    // FD-07 (a): lines fall on k x step converted with the DPI, in any unit.
    #[test]
    fn fd_07_a_lines_fall_on_multiples_of_the_converted_step(
        step in 1.0..=500.0f64, unit in prop::sample::select(Unit::ALL.to_vec()),
        dpi in 1.0..=10_000.0f64, zoom in 0.05..=32.0f64, hi in 100.0..3000.0f64,
    ) {
        let g = grid(step, unit, 1);
        let step_px = g.step.to_px(dpi).unwrap();
        prop_assume!(step_px * zoom >= 6.0 && (hi / step_px).fract() > 1e-6 && hi / step_px < 1e5);
        let lines = grid_lines(&g, view(0.0, 0.0, hi, 1.0), zoom, dpi).unwrap().vertical;
        prop_assert_eq!(lines.len(), (hi / step_px).floor() as usize + 1);
        for (k, l) in lines.iter().enumerate() {
            prop_assert!((l.position - k as f64 * step_px).abs() <= 0.01);
            prop_assert!(l.major);
        }
    }

    // FD-07: the fallback is the smallest step x 2^k with a screen spacing >= 6 px.
    #[test]
    fn fd_07_fallback_is_the_smallest_power_of_two_multiple(
        step in 1.0..=50.0f64, unit in prop::sample::select(Unit::ALL.to_vec()),
        dpi in 1.0..=96.0f64, zoom in 0.05..=0.5f64, sub in 1u8..=10,
    ) {
        let g = grid(step, unit, sub);
        let step_px = g.step.to_px(dpi).unwrap();
        prop_assume!(step_px * zoom < 6.0);
        let lines = grid_lines(&g, view(0.0, 0.0, 30.0 / zoom, 1.0), zoom, dpi).unwrap().vertical;
        prop_assert!(lines.len() >= 2 && lines.iter().all(|l| l.major));
        let s = lines[1].position - lines[0].position;
        prop_assert!(s * zoom >= 6.0 - 1e-9);
        prop_assert!(s / 2.0 * zoom < 6.0);
        let k = (s / step_px).log2();
        prop_assert!((k - k.round()).abs() <= 1e-9 && k.round() >= 1.0);
    }

    // FD-07 (c) and ARQ-110: at most 2 000 lines, strictly ascending, finite, inside the view.
    #[test]
    fn fd_07_c_at_most_2000_lines_at_any_zoom_on_a_4k_screen(
        step in 1.0..=500.0f64, unit in prop::sample::select(Unit::ALL.to_vec()),
        dpi in 1.0..=10_000.0f64, zoom in 0.05..=32.0f64, sub in 1u8..=10,
        x in -5000.0..5000.0f64, y in -5000.0..5000.0f64,
    ) {
        let v = screen(x, y, zoom);
        let g = grid_lines(&grid(step, unit, sub), v, zoom, dpi).unwrap();
        prop_assert!(g.vertical.len() + g.horizontal.len() <= 2000);
        for (lines, lo, hi) in [(&g.vertical, v.x0, v.x1), (&g.horizontal, v.y0, v.y1)] {
            prop_assert!(lines.windows(2).all(|p| p[0].position < p[1].position));
            prop_assert!(lines.iter().all(|l| l.position.is_finite() && l.position >= lo && l.position <= hi));
        }
    }
}

#[test]
fn fd_07_c_a_small_step_at_a_high_zoom_stays_under_the_limit() {
    let g = grid_lines(&px(1.0, 10), screen(0.0, 0.0, 32.0), 32.0, 96.0).unwrap();
    assert!(g.vertical.len() + g.horizontal.len() <= 2000);
    assert!(g.vertical.len() > 100);
    // the densest case: 6 px spacing on both axes
    let g = grid_lines(&px(12.0, 2), screen(0.0, 0.0, 1.0), 1.0, 96.0).unwrap();
    assert_eq!((g.vertical.len(), g.horizontal.len()), (641, 361));
}

#[test]
fn fd_07_e_ranges_include_both_ends_and_reject_just_outside() {
    let v = view(0.0, 0.0, 100.0, 100.0);
    let run = |g: &GridSettings, zoom: f64| grid_lines(g, v, zoom, 96.0);
    let out = |field| Err(LayoutError::OutOfRange { field });
    for ok in [1.0, 500.0] {
        assert!(run(&px(ok, 1), 1.0).is_ok());
    }
    assert!(run(&grid(500.0, Unit::In, 1), 1.0).is_ok());
    assert_eq!(run(&px(0.99, 1), 1.0), out("step"));
    assert_eq!(run(&px(500.01, 1), 1.0), out("step"));
    assert_eq!(run(&grid(0.99, Unit::Mm, 1), 1.0), out("step"));
    for ok in [1, 10] {
        assert!(run(&px(10.0, ok), 1.0).is_ok());
    }
    assert_eq!(run(&px(10.0, 0), 1.0), out("subdivisions"));
    assert_eq!(run(&px(10.0, 11), 1.0), out("subdivisions"));
    for ok in [0.05, 32.0] {
        assert!(run(&px(10.0, 1), ok).is_ok());
    }
    assert_eq!(run(&px(10.0, 1), 0.049_9), out("zoom"));
    assert_eq!(run(&px(10.0, 1), 32.000_1), out("zoom"));
    assert_eq!(grid_lines(&px(10.0, 1), v, 1.0, 0.0), out("dpi"));
}

#[test]
fn arq_110_every_float_input_is_checked() {
    let g = px(10.0, 2);
    let v = view(0.0, 0.0, 100.0, 100.0);
    for bad in BAD {
        for (g, v, zoom, dpi, field) in [
            (px(bad, 2), v, 1.0, 96.0, "step"),
            (g, v, bad, 96.0, "zoom"),
            (g, v, 1.0, bad, "dpi"),
            (g, view(bad, 0.0, 100.0, 100.0), 1.0, 96.0, "view"),
            (g, view(0.0, bad, 100.0, 100.0), 1.0, 96.0, "view"),
            (g, view(0.0, 0.0, bad, 100.0), 1.0, 96.0, "view"),
            (g, view(0.0, 0.0, 100.0, bad), 1.0, 96.0, "view"),
        ] {
            assert_eq!(
                grid_lines(&g, v, zoom, dpi),
                Err(LayoutError::NonFinite { field }),
                "{field} = {bad}"
            );
        }
    }
    // intermediates: a step that overflows when converted to px
    assert!(matches!(
        grid_lines(&grid(500.0, Unit::In, 1), v, 1.0, f64::MAX),
        Err(LayoutError::NonFinite { .. })
    ));
    assert_eq!(
        grid_lines(&g, view(5.0, 0.0, 1.0, 1.0), 1.0, 96.0),
        Err(LayoutError::InvalidBox { field: "view" })
    );
}

#[test]
fn arq_111_the_same_input_gives_the_same_bytes_with_axes_that_differ() {
    let g = px(7.5, 3);
    let v = view(-33.0, 4.0, 190.0, 61.0);
    let a = grid_lines(&g, v, 2.0, 96.0).unwrap();
    let b = grid_lines(&g, v, 2.0, 96.0).unwrap();
    for (x, y) in a
        .vertical
        .iter()
        .chain(&a.horizontal)
        .zip(b.vertical.iter().chain(&b.horizontal))
    {
        assert_eq!(x.position.to_bits(), y.position.to_bits());
        assert_eq!(x.major, y.major);
    }
    assert_eq!(a.vertical.len(), b.vertical.len());
    assert_ne!(a.vertical.len(), a.horizontal.len());
    assert!(a.vertical[0].position < 0.0 && a.horizontal[0].position > 0.0);
    assert!(a.vertical.windows(2).all(|p| p[0].position < p[1].position));
}
