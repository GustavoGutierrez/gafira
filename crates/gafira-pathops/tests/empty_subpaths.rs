//! A subpath that holds no segment (a move followed directly by a close, a
//! close after a close, lone moves) is empty: it covers nothing and is never
//! open. Every operation reads a path with such subpaths the way it reads the
//! same path without them, and no operation panics on them.

use gafira_pathops::{BezPath, Cap, Containment, FillRule, Join, Path, StrokeStyle};
use kurbo::{PathEl, Point, Rect, Shape};

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn square(x: f64, y: f64, side: f64) -> Vec<PathEl> {
    vec![
        PathEl::MoveTo(p(x, y)),
        PathEl::LineTo(p(x + side, y)),
        PathEl::LineTo(p(x + side, y + side)),
        PathEl::LineTo(p(x, y + side)),
        PathEl::ClosePath,
    ]
}

fn empty(x: f64, y: f64) -> Vec<PathEl> {
    vec![PathEl::MoveTo(p(x, y)), PathEl::ClosePath]
}

fn path(parts: &[Vec<PathEl>]) -> Path {
    let bez: BezPath = parts.iter().flatten().copied().collect();
    Path::new(bez, FillRule::NonZero)
}

/// The path with empty subpaths, and the same path without them.
fn cases() -> Vec<(&'static str, Path, Path)> {
    let tail = vec![
        PathEl::LineTo(p(6.0, 6.0)),
        PathEl::LineTo(p(9.0, 5.0)),
        PathEl::ClosePath,
    ];
    let reduced_tail = vec![
        PathEl::MoveTo(p(5.0, 5.0)),
        PathEl::LineTo(p(6.0, 6.0)),
        PathEl::LineTo(p(9.0, 5.0)),
        PathEl::ClosePath,
    ];
    let square_tail = vec![
        PathEl::MoveTo(p(0.0, 0.0)),
        PathEl::LineTo(p(4.0, 7.0)),
        PathEl::LineTo(p(7.0, 3.0)),
        PathEl::ClosePath,
    ];
    vec![
        ("M Z", path(&[empty(5.0, 5.0)]), path(&[])),
        (
            "M Z M Z",
            path(&[empty(5.0, 5.0), empty(6.0, 6.0)]),
            path(&[]),
        ),
        (
            "square M Z",
            path(&[square(0.0, 0.0, 10.0), empty(5.0, 5.0)]),
            path(&[square(0.0, 0.0, 10.0)]),
        ),
        (
            "M Z square",
            path(&[empty(5.0, 5.0), square(0.0, 0.0, 10.0)]),
            path(&[square(0.0, 0.0, 10.0)]),
        ),
        (
            "square M Z square M Z",
            path(&[
                square(0.0, 0.0, 10.0),
                empty(5.0, 5.0),
                square(20.0, 20.0, 5.0),
                empty(1.0, 1.0),
            ]),
            path(&[square(0.0, 0.0, 10.0), square(20.0, 20.0, 5.0)]),
        ),
        (
            "square Z Z",
            path(&[square(0.0, 0.0, 10.0), vec![PathEl::ClosePath]]),
            path(&[square(0.0, 0.0, 10.0)]),
        ),
        (
            "M Z tail",
            path(&[empty(5.0, 5.0), tail]),
            path(&[reduced_tail]),
        ),
        (
            "square Z Z tail",
            path(&[
                square(0.0, 0.0, 10.0),
                vec![PathEl::ClosePath],
                vec![
                    PathEl::LineTo(p(4.0, 7.0)),
                    PathEl::LineTo(p(7.0, 3.0)),
                    PathEl::ClosePath,
                ],
            ]),
            path(&[square(0.0, 0.0, 10.0), square_tail]),
        ),
    ]
}

fn style() -> StrokeStyle {
    StrokeStyle {
        width: 2.0,
        cap: Cap::Round,
        join: Join::Round,
    }
}

fn operand() -> Path {
    Path::new(
        Rect::new(2.0, 2.0, 12.0, 12.0).to_path(0.0),
        FillRule::NonZero,
    )
}

/// Runs every operation on the result of a simplification and on the sources.
fn use_everywhere(label: &str, path: &Path) {
    let q = p(3.0, 3.0);
    path.validate_closed().unwrap();
    path.contains(q, 1e-3).unwrap();
    path.distance_to_perimeter(q, 1e-3).unwrap();
    path.union(&operand(), 1e-3).unwrap();
    path.stroke_outline(&style(), 1e-3).unwrap();
    path.simplify(1e-3).unwrap();
    let _ = label;
}

#[test]
fn a_simplification_never_starts_with_a_close_and_holds_no_empty_subpath() {
    for (name, input, reduced) in cases() {
        let simplified = input.simplify(1e-3).unwrap();

        assert!(
            !matches!(
                simplified.as_bez_path().elements().first(),
                Some(PathEl::ClosePath)
            ),
            "{name}: the output starts with a close"
        );
        assert_eq!(
            simplified,
            reduced.simplify(1e-3).unwrap(),
            "{name}: differs from the path without its empty subpaths"
        );
        use_everywhere(name, &simplified);
    }
}

#[test]
fn a_path_with_empty_subpaths_is_closed_and_closing_it_keeps_it_closed() {
    for (name, input, _) in cases() {
        assert!(input.validate_closed().is_ok(), "{name}");
        let closed = input.close_subpaths();
        assert!(closed.validate_closed().is_ok(), "{name}");
        assert_eq!(closed.subpath_count(), input.subpath_count(), "{name}");
    }
}

#[test]
fn the_queries_read_empty_subpaths_as_absent() {
    let points = [p(3.0, 3.0), p(5.0, 5.0), p(-4.0, 1.0), p(22.0, 22.0)];
    for (name, input, reduced) in cases() {
        for q in points {
            assert_eq!(
                input.contains(q, 1e-3),
                reduced.contains(q, 1e-3),
                "{name} contains {q:?}"
            );
            assert_eq!(
                input.distance_to_perimeter(q, 1e-3),
                reduced.distance_to_perimeter(q, 1e-3),
                "{name} distance {q:?}"
            );
        }
        assert_eq!(input.is_empty(), reduced.is_empty(), "{name}");
    }
}

#[test]
fn the_boolean_operations_read_empty_subpaths_as_absent() {
    let other = operand();
    for (name, input, reduced) in cases() {
        assert_eq!(
            input.union(&other, 1e-3),
            reduced.union(&other, 1e-3),
            "{name} union"
        );
        assert_eq!(
            other.intersection(&input, 1e-3),
            other.intersection(&reduced, 1e-3),
            "{name} intersection"
        );
        assert_eq!(
            input.difference(&other, 1e-3),
            reduced.difference(&other, 1e-3),
            "{name} difference"
        );
        assert_eq!(
            other.xor(&input, 1e-3),
            other.xor(&reduced, 1e-3),
            "{name} xor"
        );
    }
}

#[test]
fn the_stroke_reads_empty_subpaths_as_absent() {
    for (name, input, reduced) in cases() {
        let stroked = input.stroke_outline(&style(), 1e-3).unwrap();
        let expected = reduced.stroke_outline(&style(), 1e-3).unwrap();
        assert_eq!(stroked.is_empty(), expected.is_empty(), "{name}");
        for q in [p(0.5, 0.5), p(5.0, 5.0), p(10.5, 3.0)] {
            assert_eq!(
                stroked.contains(q, 1e-3),
                expected.contains(q, 1e-3),
                "{name} {q:?}"
            );
        }
    }
}

#[test]
fn a_path_of_empty_subpaths_covers_nothing() {
    let only = path(&[empty(5.0, 5.0), empty(6.0, 6.0)]);

    assert!(only.is_empty());
    assert_eq!(only.contains(p(5.0, 5.0), 1e-3), Ok(Containment::Outside));
}

mod generated {
    use super::*;
    use proptest::prelude::*;

    /// One piece of a path: a square, an empty subpath, or a close after a
    /// close.
    #[derive(Clone, Debug)]
    enum Piece {
        Square(f64, f64),
        Empty(f64, f64),
        Close,
    }

    fn pieces() -> impl Strategy<Value = Vec<Piece>> {
        let piece = prop_oneof![
            (0.0..60.0f64, 0.0..60.0f64).prop_map(|(x, y)| Piece::Square(x, y)),
            (0.0..60.0f64, 0.0..60.0f64).prop_map(|(x, y)| Piece::Empty(x, y)),
            Just(Piece::Close),
        ];
        // The first piece is never a bare close.
        (
            prop_oneof![
                (0.0..60.0f64, 0.0..60.0f64).prop_map(|(x, y)| Piece::Square(x, y)),
                (0.0..60.0f64, 0.0..60.0f64).prop_map(|(x, y)| Piece::Empty(x, y)),
            ],
            prop::collection::vec(piece, 0..5),
        )
            .prop_map(|(first, rest)| std::iter::once(first).chain(rest).collect())
    }

    fn build(pieces: &[Piece], keep_empty: bool) -> Path {
        let mut parts: Vec<Vec<PathEl>> = Vec::new();
        for piece in pieces {
            match *piece {
                Piece::Square(x, y) => parts.push(square(x, y, 8.0)),
                Piece::Empty(x, y) if keep_empty => parts.push(empty(x, y)),
                Piece::Close if keep_empty => parts.push(vec![PathEl::ClosePath]),
                Piece::Empty(..) | Piece::Close => {}
            }
        }
        path(&parts)
    }

    proptest! {
        /// Every operation reads empty subpaths, closes after closes included,
        /// as absent.
        #[test]
        fn every_operation_reads_empty_subpaths_as_absent(
            pieces in pieces(),
            probe in (0.0..70.0f64, 0.0..70.0f64),
        ) {
            let input = build(&pieces, true);
            let reduced = build(&pieces, false);
            let q = p(probe.0, probe.1);
            let other = operand();

            prop_assert!(input.validate_closed().is_ok());
            prop_assert_eq!(input.is_empty(), reduced.is_empty());
            prop_assert_eq!(input.contains(q, 1e-3), reduced.contains(q, 1e-3));
            prop_assert_eq!(
                input.distance_to_perimeter(q, 1e-3),
                reduced.distance_to_perimeter(q, 1e-3)
            );
            prop_assert_eq!(input.union(&other, 1e-3), reduced.union(&other, 1e-3));
            prop_assert_eq!(other.xor(&input, 1e-3), other.xor(&reduced, 1e-3));
            let simplified = input.simplify(1e-3).unwrap();
            prop_assert_eq!(&simplified, &reduced.simplify(1e-3).unwrap());
            prop_assert!(!matches!(
                simplified.as_bez_path().elements().first(),
                Some(PathEl::ClosePath)
            ));
            use_everywhere("generated", &simplified);
            prop_assert!(input.close_subpaths().validate_closed().is_ok());
        }
    }
}
