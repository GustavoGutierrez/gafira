//! FD-07 (a, c), ARQ-110: the grid tests that walk the most lines.
//!
//! Apart from `grid` so that, if the line range rule breaks, the quick tests there fail
//! first: these loop over a range that is orders of magnitude too wide.

use gafira_layout::grid::grid_lines;
use gafira_layout::{GridSettings, GridStyle, Length, Rect, Unit};
use proptest::prelude::*;

fn grid(step: f64, unit: Unit, subdivisions: u8) -> GridSettings {
    GridSettings {
        visible: true,
        step: Length::new(step, unit),
        subdivisions,
        style: GridStyle::Lines,
        snap: true,
    }
}

fn view(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
    Rect::new(x0, y0, x1, y1)
}

/// The view that a 3840 x 2160 screen shows at `zoom`, from `(x, y)`.
fn screen(x: f64, y: f64, zoom: f64) -> Rect {
    view(x, y, x + 3840.0 / zoom, y + 2160.0 / zoom)
}
proptest! {

    // FD-07 (c) and ARQ-110: at most 2 000 lines, strictly ascending, finite, inside the view.
    #[test]
    fn fd_07_c_at_most_2000_lines_at_any_zoom_on_a_4k_screen(
        step in 1.0..=500.0f64, unit in prop::sample::select(Unit::ALL.to_vec()),
        dpi in 1.0..=10_000.0f64, zoom in 0.05..=32.0f64, sub in 1u8..=10,
        x in -16_384.0..=16_384.0f64, y in -16_384.0..=16_384.0f64,
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
fn fd_07_a_mm_steps_keep_a_line_on_either_view_edge() {
    // the report's case: 1 mm at 150 dpi doubles to 11.81 px; its line k = 13 ends the view
    let unit = 2.0 * Length::new(1.0, Unit::Mm).to_px(150.0).unwrap();
    let r = view(0.0, 0.0, 13.0 * unit, 1.0);
    let at = grid_lines(&grid(1.0, Unit::Mm, 1), r, 1.0, 150.0)
        .unwrap()
        .vertical;
    assert_eq!(at.len(), 14);
    for dpi in [150.0, 300.0, 600.0] {
        for mm in 2..=20 {
            let step = Length::new(f64::from(mm), Unit::Mm).to_px(dpi).unwrap();
            for (k0, k1) in (0..=13).flat_map(|a| (0..=13).map(move |b| (a, b))) {
                let (lo, hi) = (-f64::from(k0) * step, f64::from(k1) * step);
                if hi - lo > 3840.0 {
                    continue;
                }
                let at = grid_lines(
                    &grid(f64::from(mm), Unit::Mm, 1),
                    view(lo, 0.0, hi, 1.0),
                    1.0,
                    dpi,
                );
                let at = at.unwrap().vertical;
                assert_eq!(
                    at.len(),
                    (k0 + k1 + 1) as usize,
                    "{mm} mm, {dpi} dpi, {k0}..{k1}"
                );
                assert!(at[0].position == lo && at[at.len() - 1].position == hi);
            }
        }
    }
}
