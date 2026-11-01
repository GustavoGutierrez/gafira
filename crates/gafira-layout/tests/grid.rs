//! FD-07 (a, c, d, e), ARQ-110, ARQ-111 for the visible grid lines.

use gafira_layout::grid::grid_lines;
use gafira_layout::{GridLine, GridSettings, GridStyle, LayoutError, Length, Rect, Unit};
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
    // FD-07 (a), exact oracle in integers: both view edges lie on lines, so every line from
    // `j0 * step` to `(j0 + m) * step` must be there, the edges included.
    #[test]
    fn fd_07_a_every_line_between_two_edge_lines_is_kept(
        step in 6u32..=500, sub in 1u8..=10, zoom in prop::sample::select(vec![1u32, 2, 4, 8, 16, 32]),
        j0 in -20i64..=20, m in 0u32..=640,
    ) {
        let (s, d, z) = (i64::from(step), i64::from(sub), i64::from(zoom));
        let m = i64::from(m) % (3840 / (s * z) + 1);
        let at = vertical(&px(f64::from(step), sub), view((j0 * s) as f64, 0.0, ((j0 + m) * s) as f64, 1.0), f64::from(zoom));
        // the minor grid shows when its screen spacing is at least 6 px
        let per = if s * z >= 6 * d { d } else { 1 };
        prop_assert_eq!(at.len() as i64, m * per + 1);
        for (i, l) in at.iter().enumerate() {
            let k = j0 * per + i as i64;
            prop_assert!((l.position - (k * s) as f64 / per as f64).abs() <= 1e-9);
            prop_assert_eq!(l.major, k % per == 0);
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
}

#[test]
fn fd_07_c_the_densest_cases_stay_under_the_limit() {
    // (step px, subdivisions, zoom, vertical, horizontal): the fallback, a hidden minor grid, the 6 px spacing
    for (step, sub, zoom, want) in [
        (1.0, 1, 0.05, (601, 338)),
        (1.0, 10, 1.0, (481, 271)),
        (1.0, 10, 32.0, (121, 68)),
        (6.0, 1, 1.0, (641, 361)),
    ] {
        let g = grid_lines(&px(step, sub), screen(0.0, 0.0, zoom), zoom, 96.0).unwrap();
        assert_eq!(
            (g.vertical.len(), g.horizontal.len()),
            want,
            "{step} {sub} {zoom}"
        );
    }
}

#[test]
fn fd_07_lines_at_negative_positions_are_major_on_whole_multiples_of_the_step() {
    // step 10, 5 subdivisions: every 2 px, and only -10, 0 and 10 are major
    let at = vertical(&px(10.0, 5), view(-10.0, 0.0, 10.0, 10.0), 5.0);
    assert_eq!(
        xs(&at),
        [-10.0, -8.0, -6.0, -4.0, -2.0, 0.0, 2.0, 4.0, 6.0, 8.0, 10.0]
    );
    assert_eq!(at.iter().filter(|l| l.major).count(), 3);
    assert!([0, 5, 10].iter().all(|&i| at[i].major));
    // any unit: 1 in at 96 dpi
    let at = vertical(&grid(1.0, Unit::In, 1), view(-96.0, 0.0, 96.0, 10.0), 1.0);
    assert_eq!(xs(&at), [-96.0, 0.0, 96.0]);
    // only 3 x 2^k fits: the same rule with the doubled step, all major
    let at = vertical(&px(3.0, 1), view(-12.0, 0.0, 12.0, 1.0), 1.0);
    assert_eq!(xs(&at), [-12.0, -6.0, 0.0, 6.0, 12.0]);
    assert!(at.iter().all(|l| l.major));
}

#[test]
fn fd_07_e_ranges_include_both_ends_and_reject_just_outside() {
    let v = view(0.0, 0.0, 50.0, 50.0);
    let run = |g: &GridSettings, zoom: f64| grid_lines(g, v, zoom, 96.0);
    let out = |field| Err(LayoutError::OutOfRange { field });
    for ok in [1.0, 500.0] {
        assert!(run(&px(ok, 1), 1.0).is_ok());
    }
    assert!(run(&grid(500.0, Unit::In, 1), 1.0).is_ok());
    assert_eq!(run(&px(1.0f64.next_down(), 1), 1.0), out("step"));
    assert_eq!(run(&px(500.0f64.next_up(), 1), 1.0), out("step"));
    assert_eq!(run(&grid(0.99, Unit::Mm, 1), 1.0), out("step"));
    for ok in [1, 10] {
        assert!(run(&px(10.0, ok), 1.0).is_ok());
    }
    assert_eq!(run(&px(10.0, 0), 1.0), out("subdivisions"));
    assert_eq!(run(&px(10.0, 11), 1.0), out("subdivisions"));
    for ok in [0.05, 32.0] {
        assert!(run(&px(10.0, 1), ok).is_ok());
    }
    assert_eq!(run(&px(10.0, 1), 0.05f64.next_down()), out("zoom"));
    assert_eq!(run(&px(10.0, 1), 32.0f64.next_up()), out("zoom"));
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

#[test]
fn fd_07_view_screen_size_is_limited_per_axis_with_a_tolerance_of_a_millionth() {
    let out = |field| Err(LayoutError::OutOfRange { field });
    let at = |x: f64, y: f64, w: f64, h: f64, zoom: f64| {
        grid_lines(&px(6.0, 1), view(x, y, x + w, y + h), zoom, 96.0)
    };
    // the limit itself is valid, at zoom 1 and scaled by the zoom
    assert!(at(0.0, 0.0, 3840.0, 2160.0, 1.0).is_ok());
    assert!(at(0.0, 0.0, 1920.0, 1080.0, 2.0).is_ok());
    assert!(at(0.0, 0.0, 7680.0, 4320.0, 0.5).is_ok());
    // just above on one axis; width and height are not swapped
    assert_eq!(at(0.0, 0.0, 1920.01, 1080.0, 2.0), out("view"));
    assert_eq!(at(0.0, 0.0, 1920.0, 1080.01, 2.0), out("view"));
    assert_eq!(at(0.0, 0.0, 2160.0, 3840.0, 1.0), out("view"));
    // epsilon = 1e-6 px on screen, per axis: 9e-7 is inside, 1.1e-6 is not
    assert!(at(0.0, 0.0, 3_840.000_000_9, 2160.0, 1.0).is_ok());
    assert!(at(0.0, 0.0, 3840.0, 2_160.000_000_9, 1.0).is_ok());
    assert_eq!(at(0.0, 0.0, 3_840.000_001_1, 2160.0, 1.0), out("view"));
    assert_eq!(at(0.0, 0.0, 3840.0, 2_160.000_001_1, 1.0), out("view"));
    // the limit plus epsilon is valid, the next f64 above it is not
    let (w, h) = (3840.0 + 1e-6, 2160.0 + 1e-6);
    assert!(at(0.0, 0.0, w, 2160.0, 1.0).is_ok() && at(0.0, 0.0, 3840.0, h, 1.0).is_ok());
    assert_eq!(at(0.0, 0.0, w.next_up(), 2160.0, 1.0), out("view"));
    assert_eq!(at(0.0, 0.0, 3840.0, h.next_up(), 1.0), out("view"));
    // a view computed as screen / zoom rounds: 3840 / 0.9 * 0.9 is above 3840
    for corner in [0.0, 1000.5] {
        assert!(at(corner, corner, 3840.0 / 0.9, 2160.0 / 0.9, 0.9).is_ok());
    }
    // the tolerance adds no line: 641 x 361 at the limit and at 9e-7 above it
    for extra in [0.0, 0.000_000_9] {
        let g = at(0.0, 0.0, 3840.0 + extra, 2160.0 + extra, 1.0).unwrap();
        assert_eq!((g.vertical.len(), g.horizontal.len()), (641, 361));
    }
}

#[test]
fn fd_07_dpi_is_checked_after_non_finite_and_before_every_range_rule() {
    let v = view(0.0, 0.0, 50.0, 50.0);
    for dpi in [0.0, -96.0] {
        for (g, v, zoom) in [
            (px(10.0, 1), v, 33.0),
            (px(10.0, 1), view(0.0, 0.0, 100_000.0, 1.0), 1.0),
            (px(0.5, 1), v, 1.0),
            (px(10.0, 0), v, 1.0),
            (px(10.0, 1), v, 1.0),
        ] {
            assert_eq!(
                grid_lines(&g, v, zoom, dpi),
                Err(LayoutError::OutOfRange { field: "dpi" })
            );
        }
    }
    assert_eq!(
        grid_lines(&px(f64::NAN, 1), v, 1.0, 0.0),
        Err(LayoutError::NonFinite { field: "step" })
    );
}

#[test]
fn fd_07_view_is_checked_after_the_zoom_range_and_a_product_that_overflows_is_non_finite() {
    let big = view(0.0, 0.0, 1000.0, 1000.0);
    assert_eq!(
        grid_lines(&px(10.0, 1), big, 33.0, 96.0),
        Err(LayoutError::OutOfRange { field: "zoom" })
    );
    assert_eq!(
        grid_lines(&px(10.0, 1), view(0.0, 0.0, f64::MAX, 1.0), 32.0, 96.0),
        Err(LayoutError::NonFinite { field: "view" })
    );
}

#[test]
fn fd_07_a_major_line_on_a_view_edge_is_kept_for_a_non_integer_step_with_subdivisions() {
    // Step 4.8 px, 3 subdivisions, zoom 4: 11 * 4.8 = 52.8 is a major line on the edge.
    let at = vertical(&px(4.8, 3), view(0.0, 0.0, 52.8, 10.0), 4.0);
    assert_eq!(at.len(), 34);
    let last = at[33];
    assert!(last.major && last.position == 52.8, "{last:?}");
}

proptest! {
    /// Views from 0 to `m * step`, with the minor grid showing: oracle built apart from the code.
    #[test]
    fn fd_07_a_majors_sit_exactly_on_multiples_of_the_step(
        tenths in 20_u32..=1200,
        subdivisions in 1_u8..=10,
        m_raw in 1_u32..=60,
    ) {
        let step = f64::from(tenths) / 10.0;
        let m = 1 + m_raw % ((120.0 / step) as u32).max(1);
        let n = u32::from(subdivisions);
        // Zoom 32: the minor spacing is step * 32 / n >= 6 px.
        let at = vertical(&px(step, subdivisions), view(0.0, 0.0, f64::from(m) * step, 5.0), 32.0);
        prop_assert_eq!(at.len() as u32, m * n + 1);
        for (i, line) in at.iter().enumerate() {
            let i = i as u32;
            prop_assert_eq!(line.major, i.is_multiple_of(n));
            if i.is_multiple_of(n) {
                prop_assert_eq!(line.position, f64::from(i / n) * step);
            } else {
                prop_assert!((line.position - f64::from(i) * step / f64::from(n)).abs() <= 1e-9);
            }
        }
    }
}
