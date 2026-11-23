//! The shapes of the modules and of the eyes, in units of one module.

use gafira_doc::{QrBody, QrEyeBall, QrEyeFrame};
use kurbo::{BezPath, Point, Rect, Vec2};

use crate::Matrix;

/// Where the handles of a quarter of a circle of radius 1 lie.
const KAPPA: f64 = 0.552_284_749_830_793_4;

/// How a corner is cut.
#[derive(Clone, Copy, PartialEq)]
enum Corner {
    Sharp,
    Round(f64),
    Bevel(f64),
}

/// East, south, west and north, clockwise with y down.
const WAYS: [Vec2; 4] = [
    Vec2::new(1.0, 0.0),
    Vec2::new(0.0, 1.0),
    Vec2::new(-1.0, 0.0),
    Vec2::new(0.0, -1.0),
];

/// Adds the corner at `at`, reached along `into` and left along `out`.
fn corner(path: &mut BezPath, first: bool, at: Point, (into, out): (Vec2, Vec2), cut: Corner) {
    let mut to = |p: Point| {
        if first {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    };
    match cut {
        Corner::Sharp => to(at),
        Corner::Round(r) => {
            let (a, b) = (at - into * r, at + out * r);
            to(a);
            let reach = r * KAPPA;
            path.curve_to(a + into * reach, b - out * reach, b);
        }
        Corner::Bevel(r) => {
            to(at - into * r);
            path.line_to(at + out * r);
        }
    }
}

/// `rect` with its corners cut: top left, top right, bottom right, bottom
/// left; clockwise.
fn rect_path(path: &mut BezPath, rect: Rect, cuts: [Corner; 4]) {
    let at = [
        Point::new(rect.x0, rect.y0),
        Point::new(rect.x1, rect.y0),
        Point::new(rect.x1, rect.y1),
        Point::new(rect.x0, rect.y1),
    ];
    for i in 0..4 {
        corner(path, i == 0, at[i], (WAYS[(i + 3) % 4], WAYS[i]), cuts[i]);
    }
    path.close_path();
}

fn circle(path: &mut BezPath, centre: Point, radius: f64) {
    let rect = Rect::from_center_size(centre, (2.0 * radius, 2.0 * radius));
    rect_path(path, rect, [Corner::Round(radius); 4]);
}

fn polygon(path: &mut BezPath, points: impl IntoIterator<Item = Point>) {
    for (i, p) in points.into_iter().enumerate() {
        if i == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path.close_path();
}

/// The outlines of the dark modules that `keep` holds, as `body` draws them.
pub(crate) fn body_path(matrix: &Matrix, body: QrBody, keep: &dyn Fn(i32, i32) -> bool) -> BezPath {
    // At most 177: the casts are exact.
    let n = matrix.size() as i32;
    let dark = |x: i32, y: i32| matrix.dark(x, y) && keep(x, y);
    let mut path = BezPath::new();
    match body {
        QrBody::Square | QrBody::Fluid | QrBody::FluidCorners | QrBody::Bevel | QrBody::Leaf => {
            contours(&mut path, n, &dark, body)
        }
        QrBody::BarsHorizontal => bars(&mut path, n, &dark, |from, to, line| {
            Rect::new(from, line + 0.08, to, line + 0.92)
        }),
        QrBody::BarsVertical => bars(&mut path, n, &|x, y| dark(y, x), |from, to, line| {
            Rect::new(line + 0.08, from, line + 0.92, to)
        }),
        QrBody::Separate
        | QrBody::Dot
        | QrBody::SmallDot
        | QrBody::Diamond
        | QrBody::Star
        | QrBody::Cross => {
            for y in 0..n {
                for x in (0..n).filter(|x| dark(*x, y)) {
                    let centre = Point::new(f64::from(x) + 0.5, f64::from(y) + 0.5);
                    module(&mut path, centre, body);
                }
            }
        }
    }
    path
}

/// A module on its own, around `c`.
fn module(path: &mut BezPath, c: Point, body: QrBody) {
    match body {
        QrBody::Dot => circle(path, c, 0.48),
        QrBody::SmallDot => circle(path, c, 0.36),
        QrBody::Diamond => {
            let r = 0.5;
            let tips = [(0.0, -r), (r, 0.0), (0.0, r), (-r, 0.0)];
            polygon(path, tips.map(|v| c + Vec2::from(v)));
        }
        QrBody::Star => {
            let tips = (0..10).map(|i| {
                let angle = std::f64::consts::PI * (f64::from(i) / 5.0 - 0.5);
                let r = if i % 2 == 0 { 0.5 } else { 0.28 };
                c + Vec2::from_angle(angle) * r
            });
            polygon(path, tips);
        }
        QrBody::Cross => {
            let (a, b) = (0.22, 0.48);
            let turn = [
                (-a, -b),
                (a, -b),
                (a, -a),
                (b, -a),
                (b, a),
                (a, a),
                (a, b),
                (-a, b),
                (-a, a),
                (-b, a),
                (-b, -a),
                (-a, -a),
            ];
            polygon(path, turn.map(|v| c + Vec2::from(v)));
        }
        _ => {
            let rect = Rect::from_center_size(c, (0.84, 0.84));
            rect_path(path, rect, [Corner::Sharp; 4]);
        }
    }
}

/// A rounded bar for each run of dark modules along a line.
fn bars(
    path: &mut BezPath,
    n: i32,
    dark: &dyn Fn(i32, i32) -> bool,
    rect: impl Fn(f64, f64, f64) -> Rect,
) {
    for line in 0..n {
        let mut from = None;
        for at in 0..=n {
            match (dark(at, line), from) {
                (true, None) => from = Some(at),
                (false, Some(start)) => {
                    let bar = rect(
                        f64::from(start) + 0.04,
                        f64::from(at) - 0.04,
                        f64::from(line),
                    );
                    rect_path(path, bar, [Corner::Round(0.42); 4]);
                    from = None;
                }
                _ => {}
            }
        }
    }
}

/// The edges of the dark region: outlines clockwise, holes against them.
/// Modules that only touch at a corner stay apart.
fn contours(path: &mut BezPath, n: i32, dark: &dyn Fn(i32, i32) -> bool, body: QrBody) {
    let stride = n as usize + 1;
    let index = |x: i32, y: i32| y as usize * stride + x as usize;
    // Which ways an edge leaves each vertex, one bit a way.
    let mut leaves = vec![0_u8; stride * stride];
    for y in 0..n {
        for x in (0..n).filter(|x| dark(*x, y)) {
            for (way, neighbour, from) in [
                (0, (x, y - 1), (x, y)),
                (1, (x + 1, y), (x + 1, y)),
                (2, (x, y + 1), (x + 1, y + 1)),
                (3, (x - 1, y), (x, y + 1)),
            ] {
                if !dark(neighbour.0, neighbour.1) {
                    leaves[index(from.0, from.1)] |= 1 << way;
                }
            }
        }
    }
    const STEP: [(i32, i32); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];
    let mut turns: Vec<((i32, i32), usize, usize)> = Vec::new();
    for y in 0..=n {
        for x in 0..=n {
            while leaves[index(x, y)] != 0 {
                turns.clear();
                let start = (x, y);
                let mut way = leaves[index(x, y)].trailing_zeros() as usize;
                let first = way;
                leaves[index(x, y)] &= !(1 << way);
                let mut at = (x + STEP[way].0, y + STEP[way].1);
                loop {
                    let out = leaves[index(at.0, at.1)];
                    // A right turn first: it stays on the same module.
                    let next = [1, 0, 3]
                        .into_iter()
                        .map(|turn| (way + turn) % 4)
                        .find(|next| out & (1 << next) != 0);
                    let Some(next) = next else {
                        // Back at the start, whose edge is already taken.
                        debug_assert_eq!(at, start);
                        if way != first {
                            turns.push((at, way, first));
                        }
                        break;
                    };
                    // The start can be a vertex crossed twice: the edge it
                    // left by comes before a left turn or going straight.
                    if at == start && (way + 1) % 4 == first {
                        turns.push((at, way, first));
                        break;
                    }
                    if next != way {
                        turns.push((at, way, next));
                    }
                    leaves[index(at.0, at.1)] &= !(1 << next);
                    way = next;
                    at = (at.0 + STEP[way].0, at.1 + STEP[way].1);
                }
                for (i, (vertex, into, out)) in turns.iter().enumerate() {
                    let cut = cut_of(body, *into, *out);
                    let point = Point::new(f64::from(vertex.0), f64::from(vertex.1));
                    corner(path, i == 0, point, (WAYS[*into], WAYS[*out]), cut);
                }
                path.close_path();
            }
        }
    }
}

/// How `body` cuts the corner from the way `into` to the way `out`.
fn cut_of(body: QrBody, into: usize, out: usize) -> Corner {
    let outward = out == (into + 1) % 4;
    match (body, outward) {
        (QrBody::Fluid | QrBody::FluidCorners, true) => Corner::Round(0.5),
        (QrBody::FluidCorners, false) => Corner::Round(0.3),
        (QrBody::Bevel, true) => Corner::Bevel(0.35),
        // Top left and bottom right.
        (QrBody::Leaf, true) if into == 3 || into == 1 => Corner::Round(0.5),
        _ => Corner::Sharp,
    }
}

/// Which eye: top left, top right or bottom left.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Eye {
    TopLeft,
    TopRight,
    BottomLeft,
}

/// `cuts` of the top left eye turned to face the middle from `eye`.
fn facing(cuts: [Corner; 4], eye: Eye) -> [Corner; 4] {
    let [tl, tr, br, bl] = cuts;
    match eye {
        Eye::TopLeft => cuts,
        Eye::TopRight => [tr, tl, bl, br],
        Eye::BottomLeft => [bl, br, tr, tl],
    }
}

/// The ring of an eye whose corner is at `at`.
pub(crate) fn frame_path(path: &mut BezPath, at: Point, frame: QrEyeFrame, eye: Eye) {
    use Corner::{Bevel, Round, Sharp};
    if frame == QrEyeFrame::Dotted {
        for y in 0..7 {
            for x in 0..7 {
                if x == 0 || y == 0 || x == 6 || y == 6 {
                    let c = at + Vec2::new(f64::from(x) + 0.5, f64::from(y) + 0.5);
                    circle(path, c, 0.5);
                }
            }
        }
        return;
    }
    let (outer, inner) = match frame {
        QrEyeFrame::Square | QrEyeFrame::Dotted => ([Sharp; 4], [Sharp; 4]),
        QrEyeFrame::Rounded => ([Round(2.0); 4], [Round(1.2); 4]),
        QrEyeFrame::Circle => ([Round(3.5); 4], [Round(2.5); 4]),
        QrEyeFrame::Leaf => (
            [Round(2.6), Sharp, Round(2.6), Sharp],
            [Round(1.8), Sharp, Round(1.8), Sharp],
        ),
        QrEyeFrame::Drop => (
            [Round(2.8), Round(2.8), Sharp, Round(2.8)],
            [Round(1.9), Round(1.9), Sharp, Round(1.9)],
        ),
        QrEyeFrame::Bevel => ([Bevel(1.2); 4], [Bevel(0.7); 4]),
        QrEyeFrame::Octagon => ([Bevel(2.05); 4], [Bevel(1.46); 4]),
    };
    let around = Rect::from_origin_size(at, (7.0, 7.0));
    rect_path(path, around, facing(outer, eye));
    let mut hole = BezPath::new();
    rect_path(&mut hole, around.inset(-1.0), facing(inner, eye));
    path.extend(hole.reverse_subpaths());
}

/// The centre of an eye whose corner is at `at`.
pub(crate) fn ball_path(path: &mut BezPath, at: Point, ball: QrEyeBall, eye: Eye) {
    use Corner::{Round, Sharp};
    let rect = Rect::from_origin_size(at + Vec2::new(2.0, 2.0), (3.0, 3.0));
    let c = rect.center();
    match ball {
        QrEyeBall::Square => rect_path(path, rect, [Sharp; 4]),
        QrEyeBall::Rounded => rect_path(path, rect, [Round(0.9); 4]),
        QrEyeBall::Circle => circle(path, c, 1.5),
        QrEyeBall::Leaf => {
            let cuts = [Round(1.3), Sharp, Round(1.3), Sharp];
            rect_path(path, rect, facing(cuts, eye));
        }
        QrEyeBall::Diamond => {
            let r = 1.75;
            let tips = [(0.0, -r), (r, 0.0), (0.0, r), (-r, 0.0)];
            polygon(path, tips.map(|v| c + Vec2::from(v)));
        }
        QrEyeBall::Bars => {
            for i in 0..3 {
                let x = rect.x0 + f64::from(i);
                let bar = Rect::new(x + 0.04, rect.y0, x + 0.96, rect.y1);
                rect_path(path, bar, [Round(0.46); 4]);
            }
        }
        QrEyeBall::Cushion => {
            // The sides bend inwards.
            let corners = [
                Point::new(rect.x0, rect.y0),
                Point::new(rect.x1, rect.y0),
                Point::new(rect.x1, rect.y1),
                Point::new(rect.x0, rect.y1),
            ];
            path.move_to(corners[0]);
            for i in 0..4 {
                let (a, b) = (corners[i], corners[(i + 1) % 4]);
                path.quad_to(a.midpoint(b).lerp(c, 0.3), b);
            }
            path.close_path();
        }
        QrEyeBall::Flower => {
            // Four petals that overlap, so the middle stays wide enough to
            // be found: the arc of each one outside the others.
            let (off, r): (f64, f64) = (0.65, 0.85);
            let notch = (r * r - off * off).sqrt();
            let start = f64::atan2(off, -notch);
            let end = f64::atan2(-notch, off) + std::f64::consts::TAU;
            for turn in 0..4 {
                let angle = std::f64::consts::FRAC_PI_2 * f64::from(turn);
                let centre = c + Vec2::new(-off, -off).rotate_scale(Vec2::from_angle(angle));
                let from = centre + Vec2::from_angle(start + angle) * r;
                if turn == 0 {
                    path.move_to(from);
                }
                let arc = kurbo::Arc::new(centre, (r, r), start + angle, end - start, 0.0);
                path.extend(arc.append_iter(0.001));
            }
            path.close_path();
        }
    }
}
