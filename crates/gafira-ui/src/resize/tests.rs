use std::f64::consts::FRAC_PI_2;

use super::*;

fn origin() -> Placement {
    Placement::new(100.0, 100.0, 200.0, 100.0)
}

fn h(x: i8, y: i8) -> Handle {
    Handle { x, y }
}

fn boxed(p: &Placement) -> (f64, f64, f64, f64) {
    (p.x, p.y, p.w, p.h)
}

// S45-01: each corner keeps 2:1, the side that grows more drives and the
// opposite corner stays.
#[test]
fn s45_01_each_corner_keeps_the_proportions() {
    let o = origin();
    let at = |handle, x: f32, y: f32| boxed(&resized(&o, handle, Vec2::new(x, y), 1.0, true));
    assert_eq!(at(h(1, 1), 100.0, 10.0), (100.0, 100.0, 300.0, 150.0));
    assert_eq!(at(h(-1, -1), -10.0, -50.0), (0.0, 50.0, 300.0, 150.0));
    assert_eq!(
        at(h(1, -1), 20.0, 0.0).min_eq(),
        (100.0, 90.0, 220.0, 110.0)
    );
    assert_eq!(at(h(-1, 1), 0.0, 50.0), (0.0, 100.0, 300.0, 150.0));
    assert_eq!(
        at(h(1, 1), 0.0, 0.0),
        (100.0, 100.0, 200.0, 100.0),
        "no drag"
    );
    // At 200 % the same drag is twice the screen px.
    let zoomed = resized(&o, h(1, 1), Vec2::new(200.0, 20.0), 2.0, true);
    assert_eq!(boxed(&zoomed), (100.0, 100.0, 300.0, 150.0));
}

/// Compares within 1e-9, so a corner that sums floats still matches.
trait MinEq {
    fn min_eq(self) -> Self;
}

impl MinEq for (f64, f64, f64, f64) {
    fn min_eq(self) -> Self {
        let r = |v: f64| (v * 1e6).round() / 1e6;
        (r(self.0), r(self.1), r(self.2), r(self.3))
    }
}

// S45-02: Shift frees a corner; an edge changes one side with or without.
#[test]
fn s45_02_shift_frees_a_corner_and_edges_change_one_side() {
    let o = origin();
    let free = resized(&o, h(1, 1), Vec2::new(100.0, 10.0), 1.0, false);
    assert_eq!(boxed(&free), (100.0, 100.0, 300.0, 110.0));
    for keep in [true, false] {
        let edge = resized(&o, h(1, 0), Vec2::new(100.0, 50.0), 1.0, keep);
        assert_eq!(boxed(&edge), (100.0, 100.0, 300.0, 100.0));
        let top = resized(&o, h(0, -1), Vec2::new(30.0, -20.0), 1.0, keep);
        assert_eq!(boxed(&top), (100.0, 80.0, 200.0, 120.0));
    }
}

// S45-03: dragging past the opposite corner stops at the smallest box of
// the same proportions (no side under 1 px), anchored where it was.
#[test]
fn s45_03_past_the_opposite_corner_the_proportions_hold() {
    let o = origin();
    let tiny = resized(&o, h(1, 1), Vec2::new(-500.0, -500.0), 1.0, true);
    assert_eq!(boxed(&tiny), (100.0, 100.0, 2.0, 1.0));
    let tiny = resized(&o, h(-1, -1), Vec2::new(900.0, 900.0), 1.0, true);
    assert_eq!(boxed(&tiny), (298.0, 199.0, 2.0, 1.0));
    assert_eq!(
        scaled(&o, h(1, 1), 2.0).map(|p| boxed(&p)),
        Some((100.0, 100.0, 400.0, 200.0))
    );
    assert_eq!(
        scaled(&o, h(-1, -1), 0.5).map(|p| boxed(&p)),
        Some((200.0, 150.0, 100.0, 50.0))
    );
    assert_eq!(scaled(&o, h(1, 1), 0.004), None, "under 1 px");
}

// S45-04: a turned element keeps its proportions in its own axes: the
// opposite corner stays on the page and the turn is kept.
#[test]
fn s45_04_a_turned_element_keeps_its_proportions_in_its_axes() {
    let mut o = origin();
    o.rotation_deg = 90.0;
    // Local x points down the page at 90°: a page drag of (0, 100) grows w.
    let to = resized(&o, h(1, 1), Vec2::new(0.0, 100.0), 1.0, true);
    assert!(
        (to.w - 300.0).abs() < 1e-9 && (to.h - 150.0).abs() < 1e-9,
        "{to:?}"
    );
    assert_eq!(to.rotation_deg, 90.0);
    let before = o.to_transform() * Point::new(0.0, 0.0);
    let after = to.to_transform() * Point::new(0.0, 0.0);
    assert!((before - after).hypot() < 1e-9, "{before:?} {after:?}");
}

// S45-05: the cursor of each handle is the double-headed arrow nearest its
// direction on the screen: corners diagonal whatever the proportions.
#[test]
fn s45_05_each_handle_shows_its_resize_cursor() {
    use CursorIcon::{ResizeHorizontal, ResizeNeSw, ResizeNwSe, ResizeVertical};
    let flat = Affine::scale(0.5);
    for (handle, icon) in [
        (h(1, -1), ResizeNeSw),
        (h(-1, 1), ResizeNeSw),
        (h(-1, -1), ResizeNwSe),
        (h(1, 1), ResizeNwSe),
        (h(1, 0), ResizeHorizontal),
        (h(-1, 0), ResizeHorizontal),
        (h(0, -1), ResizeVertical),
        (h(0, 1), ResizeVertical),
    ] {
        assert_eq!(cursor(flat, handle), icon, "{handle:?}");
    }
    let quarter = Affine::rotate(FRAC_PI_2);
    assert_eq!(cursor(quarter, h(1, 0)), ResizeVertical);
    assert_eq!(cursor(quarter, h(1, -1)), ResizeNwSe);
    let degrees = |d: f64| Affine::rotate(d.to_radians());
    assert_eq!(cursor(degrees(20.0), h(1, 0)), ResizeHorizontal);
    assert_eq!(cursor(degrees(30.0), h(1, 0)), ResizeNwSe);
    assert_eq!(cursor(Affine::FLIP_X, h(1, -1)), ResizeNwSe);
    // Where the element sits on the screen does not turn its cursor.
    let placed = Affine::translate((500.0, 300.0)) * flat;
    assert_eq!(cursor(placed, h(1, 0)), ResizeHorizontal);
    assert_eq!(cursor(placed, h(-1, -1)), ResizeNwSe);
    assert!(is_corner(h(1, -1)) && !is_corner(h(1, 0)) && !is_corner(h(0, 1)));
}

// S45-06: the label reads like the Inspector, with the change signed.
#[test]
fn s45_06_the_size_label_text() {
    assert_eq!(number(320.0), "320");
    assert_eq!(number(180.46), "180.5");
    assert_eq!(number(-0.04), "0");
    assert_eq!(size_text(320.0, 180.0), "320 × 180");
    assert_eq!(delta_text(20.0, -11.0), "+20 × −11");
    assert_eq!(delta_text(0.0, 0.5), "0 × +0.5");
}

// S45-07: the label sits 16 px from the pointer outward, and flips to the
// other side on an axis where it would leave the view.
#[test]
fn s45_07_the_size_label_sits_outside_and_flips_at_the_view_edge() {
    let view = Rect::from_min_size(Pos2::ZERO, Vec2::splat(1000.0));
    let size = Vec2::new(80.0, 20.0);
    let at = |x: f32, y: f32, out: Vec2| label_rect(Pos2::new(x, y), out, size, view).min;
    assert_eq!(
        at(500.0, 500.0, Vec2::new(1.0, 1.0)),
        Pos2::new(516.0, 516.0)
    );
    assert_eq!(
        at(500.0, 500.0, Vec2::new(-1.0, -1.0)),
        Pos2::new(404.0, 464.0)
    );
    assert_eq!(
        at(500.0, 500.0, Vec2::new(1.0, 0.0)),
        Pos2::new(516.0, 516.0)
    );
    assert_eq!(
        at(950.0, 500.0, Vec2::new(1.0, 1.0)),
        Pos2::new(854.0, 516.0)
    );
    assert_eq!(
        at(500.0, 990.0, Vec2::new(0.0, 1.0)),
        Pos2::new(516.0, 954.0)
    );
    assert_eq!(at(10.0, 10.0, Vec2::new(-1.0, -1.0)), Pos2::new(26.0, 26.0));
}
