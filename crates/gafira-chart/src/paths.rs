//! The outlines the charts share.

use std::f64::consts::{FRAC_PI_2, TAU};

use kurbo::{
    BezPath, Cap, Circle, CircleSegment, Join, Point, Rect, RoundedRectRadii, Shape as _, Stroke,
    StrokeOpts, Vec2,
};

/// How far a path may be from the curve it stands for, in px.
pub(crate) const TOLERANCE: f64 = 0.05;

/// A mark of the series `index`, about as large as a circle of `radius`:
/// each series has a shape besides its colour (DIS-33).
pub(crate) fn marker(index: usize, at: Point, radius: f64) -> BezPath {
    let polygon = |points: &[(f64, f64)]| {
        let mut path = BezPath::new();
        for (i, (x, y)) in points.iter().enumerate() {
            let p = at + Vec2::new(*x, *y) * radius;
            if i == 0 {
                path.move_to(p);
            } else {
                path.line_to(p);
            }
        }
        path.close_path();
        path
    };
    match index % 4 {
        0 => Circle::new(at, radius).to_path(TOLERANCE),
        1 => Rect::from_center_size(at, (radius * 1.8, radius * 1.8)).to_path(TOLERANCE),
        2 => polygon(&[(0.0, -1.1), (1.05, 0.8), (-1.05, 0.8)]),
        _ => polygon(&[(0.0, -1.2), (1.2, 0.0), (0.0, 1.2), (-1.2, 0.0)]),
    }
}

/// `path` as a line of `width`, its outline.
pub(crate) fn stroke(path: &BezPath, width: f64) -> BezPath {
    let style = Stroke::new(width)
        .with_caps(Cap::Round)
        .with_join(Join::Round);
    kurbo::stroke(path, &style, &StrokeOpts::default(), TOLERANCE)
}

/// A part of a ring from `start`, clockwise from the top, both in turns.
pub(crate) fn slice(centre: Point, outer: f64, inner: f64, start: f64, turns: f64) -> BezPath {
    let turns = turns.min(1.0);
    if turns >= 1.0 - 1e-9 {
        let mut path = Circle::new(centre, outer).to_path(TOLERANCE);
        if inner > 0.0 {
            path.extend(
                Circle::new(centre, inner)
                    .to_path(TOLERANCE)
                    .reverse_subpaths()
                    .elements()
                    .iter()
                    .copied(),
            );
        }
        return path;
    }
    CircleSegment::new(centre, outer, inner, start * TAU - FRAC_PI_2, turns * TAU)
        .to_path(TOLERANCE)
}

/// The point at `turns` clockwise from the top, `radius` away.
pub(crate) fn polar(centre: Point, radius: f64, turns: f64) -> Point {
    centre + Vec2::from_angle(turns * TAU - FRAC_PI_2) * radius
}

/// The side of a bar its value ends at.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum End {
    None,
    Top,
    Bottom,
    Left,
    Right,
}

/// A bar with its end as round as `round`, 0 to 1.
pub(crate) fn bar(rect: Rect, end: End, round: f64) -> BezPath {
    let (across, along) = match end {
        End::Top | End::Bottom => (rect.width(), rect.height()),
        _ => (rect.height(), rect.width()),
    };
    let r = (round * across / 2.0).min(along).max(0.0);
    let radii = match end {
        End::None => RoundedRectRadii::new(0.0, 0.0, 0.0, 0.0),
        End::Top => RoundedRectRadii::new(r, r, 0.0, 0.0),
        End::Right => RoundedRectRadii::new(0.0, r, r, 0.0),
        End::Bottom => RoundedRectRadii::new(0.0, 0.0, r, r),
        End::Left => RoundedRectRadii::new(r, 0.0, 0.0, r),
    };
    if r <= 0.0 {
        return rect.to_path(TOLERANCE);
    }
    rect.to_rounded_rect(radii).to_path(TOLERANCE)
}

/// Adds the way through `points` to `path`, from where it is or from the
/// first one: straight, or smooth without going past a point.
pub(crate) fn through(path: &mut BezPath, points: &[Point], smooth: bool) {
    let Some(first) = points.first() else { return };
    if path.elements().is_empty() {
        path.move_to(*first);
    } else {
        path.line_to(*first);
    }
    if !smooth || points.len() < 3 {
        for p in &points[1..] {
            path.line_to(*p);
        }
        return;
    }
    // Tangents that keep each piece between its ends (Fritsch and Carlson).
    let slopes: Vec<f64> = points
        .windows(2)
        .map(|p| {
            let dx = p[1].x - p[0].x;
            if dx == 0.0 {
                0.0
            } else {
                (p[1].y - p[0].y) / dx
            }
        })
        .collect();
    let last = slopes.len();
    let tangent = |i: usize| -> f64 {
        if i == 0 {
            return slopes[0];
        }
        if i == last {
            return slopes[last - 1];
        }
        let (a, b) = (slopes[i - 1], slopes[i]);
        if a * b <= 0.0 {
            return 0.0;
        }
        let middle = f64::midpoint(a, b);
        middle.signum() * middle.abs().min(3.0 * a.abs().min(b.abs()))
    };
    for i in 0..last {
        let (a, b) = (points[i], points[i + 1]);
        let third = (b.x - a.x) / 3.0;
        path.curve_to(
            Point::new(a.x + third, a.y + tangent(i) * third),
            Point::new(b.x - third, b.y - tangent(i + 1) * third),
            b,
        );
    }
}
