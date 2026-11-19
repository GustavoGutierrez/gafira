//! The shapes of the Formas panel (specs 053 and 054, D-95): which shapes
//! there are, how the panel groups them, and the path each one draws in the
//! unit box. Pure, so it is tested without a window.
//!
//! Every path is built with `kurbo` in its own coordinates, each closed
//! subpath turned the same way so the non-zero rule fills their union (a
//! hole is turned the other way), and then scaled so its exact bounds are
//! the unit box. The aspect it had before scaling is the one a new shape
//! takes on the page.

use gafira_doc::{LineCap, PathCommand, ShapeGeometry, ShapePath};
use kurbo::{Affine, BezPath, Circle, Ellipse, PathEl, Point, RoundedRect, Shape};

/// A shape of the Formas panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Piece {
    /// The lines (spec 058): circles at both ends, hollow diamonds, filled
    /// diamonds, no caps, and a square start with an arrow end.
    LineCircles,
    LineDiamonds,
    LineDiamondsFilled,
    LinePlain,
    LineArrow,
    Square,
    RoundedSquare,
    Circle,
    Triangle,
    RightTriangle,
    Pentagon,
    Hexagon,
    HexagonFlat,
    Octagon,
    Decagon,
    Star4,
    Star5,
    Star6,
    Star8,
    Seal,
    ArrowRight,
    ArrowLeft,
    ArrowUp,
    ArrowDown,
    ArrowLeftRight,
    Preparation,
    Terminator,
    Decision,
    Data,
    Document,
    SpeechSquare,
    SpeechRound,
    SpeechRounded,
    Cloud,
    CloudFlat,
    Heart,
    HeartWide,
    Ribbon,
    Pennant,
    Bookmark,
    Drop,
    DropWide,
    Gear8,
    Gear12,
    Asterisk,
    Cross,
    Flower6,
    Flower8,
}

impl Piece {
    /// Every shape, in the order of the panel.
    #[cfg(test)]
    pub(crate) const ALL: [Self; 48] = [
        Self::LineCircles,
        Self::LineDiamonds,
        Self::LineDiamondsFilled,
        Self::LinePlain,
        Self::LineArrow,
        Self::Square,
        Self::RoundedSquare,
        Self::Circle,
        Self::Triangle,
        Self::RightTriangle,
        Self::Pentagon,
        Self::Hexagon,
        Self::HexagonFlat,
        Self::Octagon,
        Self::Decagon,
        Self::Star4,
        Self::Star5,
        Self::Star6,
        Self::Star8,
        Self::Seal,
        Self::ArrowRight,
        Self::ArrowLeft,
        Self::ArrowUp,
        Self::ArrowDown,
        Self::ArrowLeftRight,
        Self::Preparation,
        Self::Terminator,
        Self::Decision,
        Self::Data,
        Self::Document,
        Self::SpeechSquare,
        Self::SpeechRound,
        Self::SpeechRounded,
        Self::Cloud,
        Self::CloudFlat,
        Self::Heart,
        Self::HeartWide,
        Self::Ribbon,
        Self::Pennant,
        Self::Bookmark,
        Self::Drop,
        Self::DropWide,
        Self::Gear8,
        Self::Gear12,
        Self::Asterisk,
        Self::Cross,
        Self::Flower6,
        Self::Flower8,
    ];

    /// Its geometry in the document: a rectangle, an ellipse or a path.
    pub(crate) fn geometry(self) -> ShapeGeometry {
        match self {
            Self::Square | Self::RoundedSquare => ShapeGeometry::Rect,
            Self::Circle => ShapeGeometry::Ellipse,
            _ => built(self).map_or(ShapeGeometry::Rect, |(path, _)| ShapeGeometry::Path(path)),
        }
    }

    /// The caps of a line, start then end; `None` for a shape that is not
    /// a line (spec 058).
    pub(crate) fn line_caps(self) -> Option<(LineCap, LineCap)> {
        Some(match self {
            Self::LineCircles => (LineCap::Circle, LineCap::Circle),
            Self::LineDiamonds => (LineCap::Diamond, LineCap::Diamond),
            Self::LineDiamondsFilled => (LineCap::DiamondFilled, LineCap::DiamondFilled),
            Self::LinePlain => (LineCap::None, LineCap::None),
            Self::LineArrow => (LineCap::Square, LineCap::Arrow),
            _ => return None,
        })
    }

    /// Its width over its height: the box a new shape takes.
    pub(crate) fn aspect(self) -> f64 {
        built(self).map_or(1.0, |(_, aspect)| aspect)
    }

    /// Its corner radius for a side of `side` px (S53-01).
    pub(crate) fn corner_radius(self, side: f64) -> f64 {
        match self {
            Self::RoundedSquare => side / 8.0,
            _ => 0.0,
        }
    }

    /// The catalog key of its name.
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::LineCircles => "pieces.line_circles",
            Self::LineDiamonds => "pieces.line_diamonds",
            Self::LineDiamondsFilled => "pieces.line_diamonds_filled",
            Self::LinePlain => "pieces.line_plain",
            Self::LineArrow => "pieces.line_arrow",
            Self::Square => "pieces.square",
            Self::RoundedSquare => "pieces.rounded_square",
            Self::Circle => "pieces.circle",
            Self::Triangle => "pieces.triangle",
            Self::RightTriangle => "pieces.right_triangle",
            Self::Pentagon => "pieces.pentagon",
            Self::Hexagon => "pieces.hexagon",
            Self::HexagonFlat => "pieces.hexagon_flat",
            Self::Octagon => "pieces.octagon",
            Self::Decagon => "pieces.decagon",
            Self::Star4 => "pieces.star4",
            Self::Star5 => "pieces.star5",
            Self::Star6 => "pieces.star6",
            Self::Star8 => "pieces.star8",
            Self::Seal => "pieces.seal",
            Self::ArrowRight => "pieces.arrow_right",
            Self::ArrowLeft => "pieces.arrow_left",
            Self::ArrowUp => "pieces.arrow_up",
            Self::ArrowDown => "pieces.arrow_down",
            Self::ArrowLeftRight => "pieces.arrow_left_right",
            Self::Preparation => "pieces.preparation",
            Self::Terminator => "pieces.terminator",
            Self::Decision => "pieces.decision",
            Self::Data => "pieces.data",
            Self::Document => "pieces.document",
            Self::SpeechSquare => "pieces.speech_square",
            Self::SpeechRound => "pieces.speech_round",
            Self::SpeechRounded => "pieces.speech_rounded",
            Self::Cloud => "pieces.cloud",
            Self::CloudFlat => "pieces.cloud_flat",
            Self::Heart => "pieces.heart",
            Self::HeartWide => "pieces.heart_wide",
            Self::Ribbon => "pieces.ribbon",
            Self::Pennant => "pieces.pennant",
            Self::Bookmark => "pieces.bookmark",
            Self::Drop => "pieces.drop",
            Self::DropWide => "pieces.drop_wide",
            Self::Gear8 => "pieces.gear8",
            Self::Gear12 => "pieces.gear12",
            Self::Asterisk => "pieces.asterisk",
            Self::Cross => "pieces.cross",
            Self::Flower6 => "pieces.flower6",
            Self::Flower8 => "pieces.flower8",
        }
    }
}

/// The sections of the Formas panel, in the order of the reference (spec
/// 054), each with the catalog key of its title and its shapes.
pub(crate) const PIECE_SECTIONS: &[(&str, &[Piece])] = &[
    (
        "pieces.lines",
        &[
            Piece::LineCircles,
            Piece::LineDiamonds,
            Piece::LineDiamondsFilled,
            Piece::LinePlain,
            Piece::LineArrow,
        ],
    ),
    (
        "pieces.basic",
        &[
            Piece::Square,
            Piece::RoundedSquare,
            Piece::Circle,
            Piece::Triangle,
            Piece::RightTriangle,
        ],
    ),
    (
        "pieces.polygons",
        &[
            Piece::Pentagon,
            Piece::Hexagon,
            Piece::HexagonFlat,
            Piece::Octagon,
            Piece::Decagon,
        ],
    ),
    (
        "pieces.stars",
        &[
            Piece::Star4,
            Piece::Star5,
            Piece::Star6,
            Piece::Star8,
            Piece::Seal,
        ],
    ),
    (
        "pieces.arrows",
        &[
            Piece::ArrowRight,
            Piece::ArrowLeft,
            Piece::ArrowUp,
            Piece::ArrowDown,
            Piece::ArrowLeftRight,
        ],
    ),
    (
        "pieces.flowchart",
        &[
            Piece::Preparation,
            Piece::Terminator,
            Piece::Decision,
            Piece::Data,
            Piece::Document,
        ],
    ),
    (
        "pieces.speech",
        &[
            Piece::SpeechSquare,
            Piece::SpeechRound,
            Piece::SpeechRounded,
        ],
    ),
    ("pieces.clouds", &[Piece::Cloud, Piece::CloudFlat]),
    ("pieces.hearts", &[Piece::Heart, Piece::HeartWide]),
    (
        "pieces.banners",
        &[Piece::Ribbon, Piece::Pennant, Piece::Bookmark],
    ),
    ("pieces.drops", &[Piece::Drop, Piece::DropWide]),
    ("pieces.gears", &[Piece::Gear8, Piece::Gear12]),
    ("pieces.asterisks", &[Piece::Asterisk, Piece::Cross]),
    ("pieces.organic", &[Piece::Flower6, Piece::Flower8]),
];

/// The path of `piece` in the unit box and its aspect; `None` for the
/// shapes the document draws without a path (rectangle and ellipse).
pub(crate) fn built(piece: Piece) -> Option<(ShapePath, f64)> {
    let path = match piece {
        Piece::Square | Piece::RoundedSquare | Piece::Circle => return None,
        Piece::LineCircles
        | Piece::LineDiamonds
        | Piece::LineDiamondsFilled
        | Piece::LinePlain
        | Piece::LineArrow => return None,
        Piece::Triangle => poly(&[(0.5, 0.0), (1.0, 1.0), (0.0, 1.0)]),
        Piece::RightTriangle => poly(&[(0.0, 0.0), (1.0, 1.0), (0.0, 1.0)]),
        Piece::Pentagon => poly(&regular(5, -90.0)),
        Piece::Hexagon => poly(&regular(6, -90.0)),
        Piece::HexagonFlat => poly(&regular(6, 0.0)),
        Piece::Octagon => poly(&regular(8, -67.5)),
        Piece::Decagon => poly(&regular(10, -90.0)),
        Piece::Star4 => poly(&star(4, 0.38)),
        Piece::Star5 => poly(&star(5, 0.45)),
        Piece::Star6 => poly(&star(6, 0.58)),
        Piece::Star8 => poly(&star(8, 0.7)),
        Piece::Seal => poly(&star(16, 0.85)),
        Piece::ArrowRight => poly(&ARROW),
        Piece::ArrowLeft => poly(&mapped(&ARROW, |(x, y)| (1.0 - x, y))),
        Piece::ArrowUp => poly(&mapped(&ARROW, |(x, y)| (y, 1.0 - x))),
        Piece::ArrowDown => poly(&mapped(&ARROW, |(x, y)| (y, x))),
        Piece::ArrowLeftRight => poly(&[
            (0.0, 0.5),
            (0.45, 0.0),
            (0.45, 0.3),
            (1.15, 0.3),
            (1.15, 0.0),
            (1.6, 0.5),
            (1.15, 1.0),
            (1.15, 0.7),
            (0.45, 0.7),
            (0.45, 1.0),
        ]),
        Piece::Preparation => poly(&[
            (0.0, 0.5),
            (0.3, 0.0),
            (1.7, 0.0),
            (2.0, 0.5),
            (1.7, 1.0),
            (0.3, 1.0),
        ]),
        Piece::Terminator => RoundedRect::new(0.0, 0.0, 2.0, 1.0, 0.5).to_path(TOLERANCE),
        Piece::Decision => poly(&[(0.75, 0.0), (1.5, 0.5), (0.75, 1.0), (0.0, 0.5)]),
        Piece::Data => poly(&[(0.35, 0.0), (1.7, 0.0), (1.35, 1.0), (0.0, 1.0)]),
        Piece::Document => {
            let mut path = BezPath::new();
            path.move_to((0.0, 0.0));
            path.line_to((1.6, 0.0));
            path.line_to((1.6, 0.85));
            path.curve_to((1.2, 0.6), (0.45, 1.2), (0.0, 0.9));
            path.close_path();
            path
        }
        Piece::SpeechSquare => poly(&[
            (0.0, 0.0),
            (1.2, 0.0),
            (1.2, 0.8),
            (0.5, 0.8),
            (0.2, 1.05),
            (0.28, 0.8),
            (0.0, 0.8),
        ]),
        Piece::SpeechRound => union(vec![
            Ellipse::new((0.6, 0.42), (0.6, 0.42), 0.0).to_path(TOLERANCE),
            poly(&[(0.25, 0.6), (0.1, 1.0), (0.55, 0.78)]),
        ]),
        Piece::SpeechRounded => union(vec![
            RoundedRect::new(0.0, 0.0, 1.2, 0.8, 0.18).to_path(TOLERANCE),
            poly(&[(0.25, 0.7), (0.15, 1.05), (0.55, 0.75)]),
        ]),
        Piece::Cloud => union(vec![
            circle(0.38, 0.55, 0.26),
            circle(0.68, 0.4, 0.33),
            circle(1.0, 0.52, 0.26),
            circle(1.2, 0.66, 0.18),
            RoundedRect::new(0.15, 0.55, 1.3, 0.84, 0.14).to_path(TOLERANCE),
        ]),
        Piece::CloudFlat => union(vec![
            circle(0.42, 0.58, 0.25),
            circle(0.78, 0.42, 0.32),
            circle(1.1, 0.58, 0.23),
            poly(&[(0.17, 0.58), (1.33, 0.58), (1.33, 0.83), (0.17, 0.83)]),
        ]),
        Piece::Heart => heart(1.0),
        Piece::HeartWide => heart(1.3),
        Piece::Ribbon => poly(&[
            (0.0, 0.0),
            (1.6, 0.0),
            (1.35, 0.5),
            (1.6, 1.0),
            (0.0, 1.0),
            (0.25, 0.5),
        ]),
        Piece::Pennant => poly(&[(0.0, 0.0), (1.0, 0.0), (1.0, 0.75), (0.5, 1.0), (0.0, 0.75)]),
        Piece::Bookmark => poly(&[(0.0, 0.0), (0.7, 0.0), (0.7, 1.0), (0.35, 0.72), (0.0, 1.0)]),
        Piece::Drop => drop(1.0),
        Piece::DropWide => drop(1.3),
        Piece::Gear8 => gear(8),
        Piece::Gear12 => gear(12),
        Piece::Asterisk => union(
            [0.0, 60.0, 120.0]
                .into_iter()
                .map(|deg| {
                    poly(&turned(
                        &[(-1.0, -0.13), (1.0, -0.13), (1.0, 0.13), (-1.0, 0.13)],
                        deg,
                    ))
                })
                .collect(),
        ),
        Piece::Cross => poly(&[
            (0.35, 0.0),
            (0.65, 0.0),
            (0.65, 0.35),
            (1.0, 0.35),
            (1.0, 0.65),
            (0.65, 0.65),
            (0.65, 1.0),
            (0.35, 1.0),
            (0.35, 0.65),
            (0.0, 0.65),
            (0.0, 0.35),
            (0.35, 0.35),
        ]),
        Piece::Flower6 => flower(6),
        Piece::Flower8 => flower(8),
    };
    Some(finished(path))
}

/// How closely a curve shape follows its exact outline, in the shape's
/// own units (about 1 for a whole shape).
const TOLERANCE: f64 = 1e-3;

/// The right arrow; the other arrows are it, mirrored or turned.
const ARROW: [(f64, f64); 7] = [
    (0.0, 0.3),
    (0.55, 0.3),
    (0.55, 0.0),
    (1.0, 0.5),
    (0.55, 1.0),
    (0.55, 0.7),
    (0.0, 0.7),
];

/// A closed polygon through `points`.
fn poly(points: &[(f64, f64)]) -> BezPath {
    let mut path = BezPath::new();
    for (i, p) in points.iter().enumerate() {
        if i == 0 {
            path.move_to(*p);
        } else {
            path.line_to(*p);
        }
    }
    path.close_path();
    path
}

/// `points`, each moved by `f`.
fn mapped(points: &[(f64, f64)], f: impl Fn((f64, f64)) -> (f64, f64)) -> Vec<(f64, f64)> {
    points.iter().copied().map(f).collect()
}

/// `points` turned `degrees` about the origin.
fn turned(points: &[(f64, f64)], degrees: f64) -> Vec<(f64, f64)> {
    let (sin, cos) = degrees.to_radians().sin_cos();
    mapped(points, |(x, y)| (x * cos - y * sin, x * sin + y * cos))
}

/// The corners of a regular polygon of `n` sides on the unit circle, the
/// first at `start` degrees (−90 is straight up).
fn regular(n: usize, start: f64) -> Vec<(f64, f64)> {
    (0..n)
        .map(|i| {
            let a = (start + 360.0 * i as f64 / n as f64).to_radians();
            (a.cos(), a.sin())
        })
        .collect()
}

/// A star of `n` points, its inner corners at `inner` of the outer radius,
/// a point straight up.
fn star(n: usize, inner: f64) -> Vec<(f64, f64)> {
    (0..2 * n)
        .map(|i| {
            let r = if i % 2 == 0 { 1.0 } else { inner };
            let a = (-90.0 + 180.0 * i as f64 / n as f64).to_radians();
            (r * a.cos(), r * a.sin())
        })
        .collect()
}

/// A circle of radius `r` about `(x, y)`.
fn circle(x: f64, y: f64, r: f64) -> BezPath {
    Circle::new((x, y), r).to_path(TOLERANCE)
}

/// A heart `wide` times as wide as it is high.
fn heart(wide: f64) -> BezPath {
    let at = |x: f64, y: f64| (x * wide, y);
    let mut path = BezPath::new();
    path.move_to(at(0.5, 0.28));
    path.curve_to(at(0.5, 0.0), at(0.0, -0.02), at(0.0, 0.3));
    path.curve_to(at(0.0, 0.62), at(0.45, 0.78), at(0.5, 1.0));
    path.curve_to(at(0.55, 0.78), at(1.0, 0.62), at(1.0, 0.3));
    path.curve_to(at(1.0, -0.02), at(0.5, 0.0), at(0.5, 0.28));
    path.close_path();
    path
}

/// A teardrop with its point up, `wide` times as wide.
fn drop(wide: f64) -> BezPath {
    let at = |x: f64, y: f64| (x * wide, y);
    let mut path = BezPath::new();
    path.move_to(at(0.5, 0.0));
    path.curve_to(at(0.6, 0.15), at(1.0, 0.5), at(1.0, 0.72));
    path.curve_to(at(1.0, 0.88), at(0.78, 1.0), at(0.5, 1.0));
    path.curve_to(at(0.22, 1.0), at(0.0, 0.88), at(0.0, 0.72));
    path.curve_to(at(0.0, 0.5), at(0.4, 0.15), at(0.5, 0.0));
    path.close_path();
    path
}

/// A gear of `n` teeth with a round hole in the middle.
fn gear(n: usize) -> BezPath {
    let (outer, root) = (1.0, 0.78);
    let step = 360.0 / n as f64;
    let mut points = Vec::new();
    for i in 0..n {
        let a = -90.0 + step * i as f64;
        for (r, d) in [(root, -0.3), (outer, -0.2), (outer, 0.2), (root, 0.3)] {
            let t = (a + d * step).to_radians();
            points.push((r * t.cos(), r * t.sin()));
        }
    }
    let ring = oriented(poly(&points), true);
    let hole = oriented(circle(0.0, 0.0, 0.36), false);
    let mut path = ring;
    path.extend(hole);
    path
}

/// A flower of `n` round petals about a round heart.
fn flower(n: usize) -> BezPath {
    let mut parts: Vec<BezPath> = regular(n, -90.0)
        .into_iter()
        .map(|(x, y)| circle(0.58 * x, 0.58 * y, 0.42))
        .collect();
    parts.push(circle(0.0, 0.0, 0.5));
    union(parts)
}

/// `path` turned so its signed area is positive (or negative): with every
/// part turned the same way the non-zero rule fills their union, and a part
/// turned the other way is a hole.
fn oriented(path: BezPath, positive: bool) -> BezPath {
    if (path.area() > 0.0) == positive {
        path
    } else {
        path.reverse_subpaths()
    }
}

/// The union of `parts` under the non-zero rule: all turned the same way.
fn union(parts: Vec<BezPath>) -> BezPath {
    let mut out = BezPath::new();
    for part in parts {
        out.extend(oriented(part, true));
    }
    out
}

/// `path` scaled so its exact bounds are the unit box, as a document path,
/// with the width over the height it had.
fn finished(mut path: BezPath) -> (ShapePath, f64) {
    let b = path.bounding_box();
    let (w, h) = (b.width(), b.height());
    path.apply_affine(
        Affine::scale_non_uniform(1.0 / w, 1.0 / h) * Affine::translate((-b.x0, -b.y0)),
    );
    let at = |p: Point| [p.x, p.y];
    let commands = path
        .elements()
        .iter()
        .map(|el| match *el {
            PathEl::MoveTo(p) => PathCommand::MoveTo(at(p)),
            PathEl::LineTo(p) => PathCommand::LineTo(at(p)),
            PathEl::QuadTo(c, p) => PathCommand::QuadTo(at(c), at(p)),
            PathEl::CurveTo(c1, c2, p) => PathCommand::CubicTo(at(c1), at(c2), at(p)),
            PathEl::ClosePath => PathCommand::Close,
        })
        .collect();
    (ShapePath::new(commands), w / h)
}

/// `path` as a `kurbo` path of `w` × `h`, from the top-left corner.
pub(crate) fn bez(path: &ShapePath, w: f64, h: f64) -> kurbo::BezPath {
    let at = |[x, y]: [f64; 2]| kurbo::Point::new(x * w, y * h);
    let mut out = kurbo::BezPath::new();
    for command in &path.commands {
        match *command {
            PathCommand::MoveTo(p) => out.move_to(at(p)),
            PathCommand::LineTo(p) => out.line_to(at(p)),
            PathCommand::QuadTo(c, p) => out.quad_to(at(c), at(p)),
            PathCommand::CubicTo(c1, c2, p) => out.curve_to(at(c1), at(c2), at(p)),
            PathCommand::Close => out.close_path(),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    // S54-05: every shape drawn by a path is a valid path of the document
    // whose exact bounds are the unit box, with area; the sections follow
    // the reference, none is empty, and every shape is in one of them.
    #[test]
    fn s54_05_library_paths_are_valid_and_fill_the_box() {
        for piece in Piece::ALL {
            let geometry = piece.geometry();
            let ShapeGeometry::Path(path) = geometry else {
                assert!(
                    matches!(piece, Piece::Square | Piece::RoundedSquare | Piece::Circle)
                        || piece.line_caps().is_some(),
                    "{piece:?} draws a path"
                );
                continue;
            };
            assert!(
                matches!(path.commands.first(), Some(PathCommand::MoveTo(_))),
                "{piece:?}"
            );
            assert!(path.commands.len() <= ShapePath::MAX_COMMANDS, "{piece:?}");
            assert!(
                path.coordinates()
                    .all(|v| (ShapePath::MIN_COORDINATE..=ShapePath::MAX_COORDINATE).contains(&v)),
                "{piece:?}"
            );
            let shape = bez(&path, 1.0, 1.0);
            let b = shape.bounding_box();
            for (got, want) in [(b.x0, 0.0), (b.y0, 0.0), (b.x1, 1.0), (b.y1, 1.0)] {
                assert!((got - want).abs() < 1e-9, "{piece:?}: {b:?}");
            }
            assert!(shape.area().abs() > 0.1, "{piece:?} has area");
            assert!((0.25..=4.0).contains(&piece.aspect()), "{piece:?}");
        }
        let titles: Vec<&str> = PIECE_SECTIONS.iter().map(|(title, _)| *title).collect();
        assert_eq!(
            titles[..5],
            [
                "pieces.lines",
                "pieces.basic",
                "pieces.polygons",
                "pieces.stars",
                "pieces.arrows"
            ]
        );
        assert!(PIECE_SECTIONS.iter().all(|(_, pieces)| !pieces.is_empty()));
        let listed: Vec<Piece> = PIECE_SECTIONS
            .iter()
            .flat_map(|(_, p)| p.iter().copied())
            .collect();
        assert_eq!(listed, Piece::ALL, "every shape once, in order");
        let keys: BTreeSet<&str> = Piece::ALL.iter().map(|p| p.key()).collect();
        assert_eq!(keys.len(), Piece::ALL.len());
    }
}
