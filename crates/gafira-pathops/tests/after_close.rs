//! A segment drawn after a close, with no new move, starts a new subpath at the
//! start point of the subpath just closed. Every operation reads such a path
//! the way it reads the same path with that move written out.

use gafira_pathops::{BezPath, Cap, FillRule, Join, Path, StrokeStyle};
use kurbo::{ParamCurve, ParamCurveNearest, PathEl, Point};
use proptest::prelude::*;

/// The path with the implicit move written out: a move to the start point of
/// the closed subpath before each segment that follows a close.
fn explicit(path: &Path) -> Path {
    let mut bez = BezPath::new();
    let mut start = Point::ORIGIN;
    let mut closed = false;
    for el in path.as_bez_path().elements() {
        match *el {
            PathEl::MoveTo(p) => {
                start = p;
                closed = false;
            }
            PathEl::ClosePath => closed = true,
            _ => {
                if closed {
                    bez.move_to(start);
                    closed = false;
                }
            }
        }
        bez.push(*el);
    }
    Path::new(bez, path.fill_rule())
}

/// A closed polygon, then more segments with or without a move, closed or not.
#[derive(Clone, Debug)]
struct Shape {
    first: Vec<(f64, f64)>,
    with_move: Option<(f64, f64)>,
    tail: Vec<(f64, f64)>,
    close_tail: bool,
}

impl Shape {
    fn path(&self, rule: FillRule) -> Path {
        let mut bez = BezPath::new();
        bez.move_to(self.first[0]);
        for p in &self.first[1..] {
            bez.line_to(*p);
        }
        bez.close_path();
        if let Some(p) = self.with_move {
            bez.move_to(p);
        }
        for p in &self.tail {
            bez.line_to(*p);
        }
        if self.close_tail {
            bez.close_path();
        }
        Path::new(bez, rule)
    }
}

fn point() -> impl Strategy<Value = (f64, f64)> {
    (0.0..100.0f64, 0.0..100.0f64)
}

fn shape_strategy(close_tail: bool) -> impl Strategy<Value = Shape> {
    (
        prop::collection::vec(point(), 3..6),
        prop::option::of(point()),
        prop::collection::vec(point(), 2..5),
    )
        .prop_map(move |(first, with_move, tail)| Shape {
            first,
            with_move,
            tail,
            close_tail,
        })
}

fn distance(path: &Path, point: Point) -> f64 {
    path.as_bez_path()
        .segments()
        .map(|s| s.eval(s.nearest(point, 1e-9).t).distance(point))
        .fold(f64::INFINITY, f64::min)
}

fn style() -> StrokeStyle {
    StrokeStyle {
        width: 2.0,
        cap: Cap::Butt,
        join: Join::Miter,
    }
}

proptest! {
    /// The simplification of a path equals that of the same path with the
    /// moves written out, and stays within the tolerance of it.
    #[test]
    fn the_simplification_reads_the_implicit_move(
        shape in shape_strategy(false),
        close_tail in any::<bool>(),
    ) {
        let shape = Shape { close_tail, ..shape };
        let path = shape.path(FillRule::NonZero);
        let written = explicit(&path);

        let simplified = path.simplify(1e-2).unwrap();

        prop_assert_eq!(&simplified, &written.simplify(1e-2).unwrap());
        for segment in written.as_bez_path().segments() {
            for step in 0..=8 {
                let q = segment.eval(f64::from(step) / 8.0);
                prop_assert!(distance(&simplified, q) <= 1e-2 * 1.01);
            }
        }
    }

    /// The stroke outline and the distance read the implicit move too.
    #[test]
    fn the_stroke_and_the_distance_read_the_implicit_move(
        shape in shape_strategy(false),
        probe in point(),
    ) {
        let path = shape.path(FillRule::NonZero);
        let written = explicit(&path);
        let q = Point::new(probe.0, probe.1);

        prop_assert_eq!(
            path.stroke_outline(&style(), 1e-3).unwrap(),
            written.stroke_outline(&style(), 1e-3).unwrap()
        );
        prop_assert_eq!(
            path.distance_to_perimeter(q, 1e-3).unwrap(),
            written.distance_to_perimeter(q, 1e-3).unwrap()
        );
    }

    /// Closing the open subpaths leaves no open subpath, tails included, and
    /// the count of subpaths stays.
    #[test]
    fn closing_the_subpaths_closes_the_tail(shape in shape_strategy(false)) {
        let path = shape.path(FillRule::NonZero);

        let closed = path.close_subpaths();

        prop_assert!(closed.validate_closed().is_ok());
        prop_assert_eq!(closed.subpath_count(), path.subpath_count());
    }
}

fn triangle_then(tail: impl FnOnce(&mut BezPath)) -> Path {
    let mut bez = BezPath::new();
    bez.move_to((0.0, 0.0));
    bez.line_to((10.0, 0.0));
    bez.line_to((10.0, 10.0));
    bez.close_path();
    tail(&mut bez);
    Path::new(bez, FillRule::NonZero)
}

/// A segment drawn after a close is simplified from the start of the closed
/// subpath, not from the last point drawn.
#[test]
fn a_tail_after_a_close_is_simplified_from_the_start_of_the_closed_subpath() {
    let path = triangle_then(|bez| {
        bez.line_to((5.0, 20.0));
        bez.line_to((0.0, 20.0));
    });

    let simplified = path.simplify(1e-3).unwrap();

    assert_eq!(simplified.subpath_count(), 2);
    assert!(distance(&simplified, Point::new(2.5, 10.0)) <= 1e-3);
    assert!(distance(&simplified, Point::new(10.0, 5.0)) <= 1e-3);
}

/// A zero-length tail after a close stays a zero-length subpath at the start
/// of the closed subpath.
#[test]
fn a_zero_length_tail_after_a_close_stays_at_the_start_of_the_closed_subpath() {
    let path = triangle_then(|bez| bez.line_to((0.0, 0.0)));

    let simplified = path.simplify(1e-3).unwrap();

    let mut expected = BezPath::new();
    expected.move_to((0.0, 0.0));
    expected.line_to((10.0, 0.0));
    expected.line_to((10.0, 10.0));
    expected.close_path();
    expected.move_to((0.0, 0.0));
    expected.line_to((0.0, 0.0));
    assert_eq!(simplified.as_bez_path(), &expected);
}

/// A subpath made only of a move is dropped by the simplification.
#[test]
fn a_subpath_made_only_of_a_move_is_dropped() {
    let mut moves = BezPath::new();
    moves.move_to((1.0, 1.0));
    moves.move_to((4.0, 4.0));
    let moves = Path::new(moves, FillRule::NonZero);

    let mut mixed = BezPath::new();
    mixed.move_to((1.0, 1.0));
    mixed.move_to((0.0, 0.0));
    mixed.line_to((10.0, 0.0));
    mixed.line_to((10.0, 10.0));
    mixed.close_path();
    let mixed = Path::new(mixed, FillRule::NonZero);

    assert!(
        moves
            .simplify(1e-3)
            .unwrap()
            .as_bez_path()
            .elements()
            .is_empty()
    );
    let simplified = mixed.simplify(1e-3).unwrap();
    assert_eq!(simplified.subpath_count(), 1);
    assert!(matches!(
        simplified.as_bez_path().elements().first(),
        Some(PathEl::MoveTo(p)) if *p == Point::new(0.0, 0.0)
    ));
}
