//! Grunge shapes of the Formas panel (spec 109): dry brush strokes,
//! splatters and claw marks, drawn by seeded noise so every one is the
//! same on every run. Each is one filled path under the non-zero rule: the
//! body, the bristle streaks as holes and the loose specks as more pieces.

use std::f64::consts::{PI, TAU};
use std::sync::OnceLock;

use kurbo::{BezPath, Point, Vec2};

/// How many dry brush strokes, splatters and claw marks there are.
pub(crate) const STROKES: usize = 24;
pub(crate) const SPLATTERS: usize = 8;
pub(crate) const CLAWS: usize = 8;

/// The shapes the owner drew and traced (`assets/shapes/grunge/`): two
/// brush strokes, five splatters and three claw marks, each with the turn,
/// in degrees, it comes in at.
pub(crate) const TRACED: usize = 10;
const TRACED_SVGS: [(&str, f64); TRACED] = [
    (
        include_str!("../../../assets/shapes/grunge/brush-stroke-curved.svg"),
        0.0,
    ),
    // Wider than the panel takes: a little slanted.
    (
        include_str!("../../../assets/shapes/grunge/brush-stroke-wide.svg"),
        -8.0,
    ),
    (
        include_str!("../../../assets/shapes/grunge/splatter-heavy.svg"),
        0.0,
    ),
    (
        include_str!("../../../assets/shapes/grunge/splatter.svg"),
        0.0,
    ),
    (
        include_str!("../../../assets/shapes/grunge/claw-marks.svg"),
        0.0,
    ),
    (
        include_str!("../../../assets/shapes/grunge/claw-marks-with-claws.svg"),
        0.0,
    ),
    (
        include_str!("../../../assets/shapes/grunge/claw-marks-thin.svg"),
        0.0,
    ),
    (
        include_str!("../../../assets/shapes/grunge/splatter-burst.svg"),
        0.0,
    ),
    (
        include_str!("../../../assets/shapes/grunge/splatter-round.svg"),
        0.0,
    ),
    (
        include_str!("../../../assets/shapes/grunge/splatter-drips.svg"),
        0.0,
    ),
];

/// A small, fast generator of numbers in `[0, 1)` from a seed.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03)
    }

    fn next(&mut self) -> f64 {
        // splitmix64.
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }

    /// A number in `[a, b)`.
    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.next()
    }
}

/// Smooth noise in `[-1, 1]` along one axis: random values at whole
/// numbers, eased between them.
struct Noise(Vec<f64>);

impl Noise {
    fn new(rng: &mut Rng, len: usize) -> Self {
        Self((0..=len).map(|_| rng.range(-1.0, 1.0)).collect())
    }

    /// The noise at `x`, wrapping past its end.
    fn at(&self, x: f64) -> f64 {
        let n = self.0.len() - 1;
        let x = x.rem_euclid(n as f64);
        let i = x.floor() as usize;
        let f = x - i as f64;
        let f = f * f * (3.0 - 2.0 * f);
        let a = self.0[i % n];
        let b = self.0[(i + 1) % n];
        a + (b - a) * f
    }
}

/// A closed polygon through `points`.
fn polygon(points: &[Point]) -> BezPath {
    let mut path = BezPath::new();
    if let Some((first, rest)) = points.split_first() {
        path.move_to(*first);
        for p in rest {
            path.line_to(*p);
        }
        path.close_path();
    }
    path
}

/// `path` turned the way the non-zero rule fills (`true`) or cuts.
fn turned(path: BezPath, fill: bool) -> BezPath {
    use kurbo::Shape as _;
    if (path.area() > 0.0) == fill {
        path
    } else {
        path.reverse_subpaths()
    }
}

/// A small irregular blob about `centre` of radius `r`.
fn speck(rng: &mut Rng, centre: Point, r: f64, stretch: Vec2) -> BezPath {
    let n = 9;
    let along = if stretch.hypot() > 0.0 {
        stretch.normalize()
    } else {
        Vec2::new(1.0, 0.0)
    };
    let across = Vec2::new(-along.y, along.x);
    let long = 1.0 + stretch.hypot();
    let points: Vec<Point> = (0..n)
        .map(|k| {
            let a = TAU * k as f64 / n as f64;
            let rr = r * rng.range(0.7, 1.15);
            centre + along * (a.cos() * rr * long) + across * (a.sin() * rr)
        })
        .collect();
    turned(polygon(&points), true)
}

/// The shape of one kind of centre line: where a stroke starts, its two
/// controls and where it ends, in a box about 1 high.
#[derive(Clone, Copy)]
enum Line {
    /// Along x.
    Straight,
    /// One bend.
    Bow(f64),
    /// Two bends.
    Wave(f64),
}

/// A point on a cubic from `p` with its tangent.
fn cubic(p: [Point; 4], t: f64) -> (Point, Vec2) {
    let u = 1.0 - t;
    let at = p[0].to_vec2() * (u * u * u)
        + p[1].to_vec2() * (3.0 * u * u * t)
        + p[2].to_vec2() * (3.0 * u * t * t)
        + p[3].to_vec2() * (t * t * t);
    let d = (p[1] - p[0]) * (3.0 * u * u)
        + (p[2] - p[1]) * (6.0 * u * t)
        + (p[3] - p[2]) * (3.0 * t * t);
    (at.to_point(), d)
}

/// The settings of one dry brush stroke.
struct Stroke {
    seed: u64,
    line: Line,
    /// Its length over its width.
    long: f64,
    /// Turn, in degrees, from lying along x.
    angle: f64,
    /// How frayed its dry end is, `[0, 1]`.
    dry: f64,
}

/// The two dozen strokes: horizontal, vertical, diagonal and curved, long
/// and short, more and less dry.
fn stroke_settings(index: usize) -> Stroke {
    const TABLE: [(f64, f64, f64, f64); STROKES] = [
        // (bend kind and amount, length over width, angle, dryness)
        (0.0, 4.2, 0.0, 0.6),
        (0.35, 3.6, -62.0, 0.8),
        (-0.25, 4.6, 0.0, 0.5),
        (0.0, 3.4, 90.0, 0.7),
        (0.5, 3.0, -35.0, 0.9),
        (1.25, 4.8, 0.0, 0.6),
        (0.0, 3.2, 35.0, 0.75),
        (-0.4, 3.8, -110.0, 0.7),
        (0.2, 5.4, 0.0, 0.85),
        (1.3, 3.6, 70.0, 0.55),
        (0.0, 2.6, -20.0, 0.9),
        (-0.55, 3.2, 20.0, 0.65),
        (0.3, 4.0, 180.0, 0.7),
        (0.0, 4.6, 90.0, 0.5),
        (1.2, 4.2, -15.0, 0.8),
        (0.45, 2.8, 55.0, 0.95),
        (-0.3, 5.0, 8.0, 0.6),
        (0.0, 3.0, -45.0, 0.6),
        (0.6, 3.4, -80.0, 0.75),
        (1.35, 4.4, 30.0, 0.7),
        (-0.2, 3.6, 120.0, 0.85),
        (0.0, 5.2, 0.0, 0.9),
        (0.4, 2.6, -55.0, 0.6),
        (-0.5, 4.2, 160.0, 0.8),
    ];
    let (bend, long, angle, dry) = TABLE[index % STROKES];
    let line = if bend == 0.0 {
        Line::Straight
    } else if bend.abs() > 1.0 {
        Line::Wave(bend.signum() * (bend.abs() - 1.0))
    } else {
        Line::Bow(bend)
    };
    Stroke {
        seed: 1000 + index as u64,
        line,
        long: long * 0.7,
        angle,
        dry,
    }
}

/// The grid a stroke is painted on, along and across it, in cells.
const ALONG: usize = 420;
const ACROSS: usize = 130;

/// Dry brush stroke `index`, painted bristle by bristle on a grid lying
/// along x and traced into a path: a loaded, blunt start, strands that run
/// out of paint one by one toward a dry end that opens into bristles,
/// streaks where the bristles skipped, and flecks shaken off. The path is
/// then bent along the stroke's line and turned.
fn stroke(index: usize) -> BezPath {
    let s = stroke_settings(index);
    let mut rng = Rng::new(s.seed);
    let mut grid = vec![false; ALONG * ACROSS];
    // The stroke fills the middle 100 of the 130 cells across; u in [0, 1]
    // runs over cells 8..408 along.
    let (u0, u_len) = (8.0, 400.0);
    let (v_mid, v_half) = (ACROSS as f64 / 2.0, 50.0);
    let paint = |grid: &mut [bool], i: isize, j: isize| {
        if (0..ALONG as isize).contains(&i) && (0..ACROSS as isize).contains(&j) {
            grid[j as usize * ALONG + i as usize] = true;
        }
    };
    let edge = [Noise::new(&mut rng, 24), Noise::new(&mut rng, 24)];
    let ink = Noise::new(&mut rng, 32);
    let skips = Noise::new(&mut rng, 256);
    let bristles = 72;
    for b in 0..bristles {
        let lane = -1.0 + 2.0 * (b as f64 + 0.5) / bristles as f64 + rng.range(-0.01, 0.01);
        let thick = 2.0 / bristles as f64 * rng.range(0.9, 2.1);
        // A loaded start, slightly rounded; the paint of each bristle runs
        // out at its own place, the outer ones first.
        let start =
            rng.range(0.0, 0.03) + 0.07 * lane * lane + 0.02 * ink.at(b as f64 * 0.9 + 11.0);
        let outer = lane.abs().powi(3);
        let end = (0.97
            - s.dry * (0.35 * (0.5 + 0.5 * ink.at(b as f64 * 0.45)) + rng.range(0.0, 0.22))
            - 0.18 * outer * s.dry)
            .clamp(start + 0.25, 1.0);
        // Where the bristle starts to skip.
        let dry_from = end - (0.2 + 0.25 * s.dry);
        let wobble = rng.range(0.0, 1000.0);
        let steps = (u_len * (end - start)).ceil() as usize;
        for k in 0..=steps {
            let u = start + (end - start) * k as f64 / steps.max(1) as f64;
            // Skips grow toward the end of the paint.
            if u > dry_from {
                let gap = ((u - dry_from) / (end - dry_from)).clamp(0.0, 1.0);
                if skips.at(u * 110.0 + b as f64 * 3.7) < -0.9 + 1.5 * gap * s.dry {
                    continue;
                }
            }
            // The pressure of the hand: the stroke swells and narrows.
            let press = 0.9 + 0.1 * ink.at(u * 5.0 + 20.0);
            let v = lane * press + 0.012 * edge[0].at(u * 6.0 + wobble);
            // The rough outer edge of the whole stroke, rounder at the start.
            let round = ((u + 0.01) / 0.06).clamp(0.0, 1.0).sqrt();
            let rough = 0.05 * edge[usize::from(v < 0.0)].at(u * 10.0 + 3.0)
                + 0.025 * skips.at(u * 160.0 + 50.0 * f64::from(u8::from(v < 0.0)));
            if v.abs() > (1.0 + rough) * (0.75 + 0.25 * round) {
                continue;
            }
            // Each bristle thins to a point where its paint ends.
            let tip = ((end - u) / 0.06).clamp(0.15, 1.0);
            let wide = thick * tip;
            let i = (u0 + u * u_len).round() as isize;
            let lo = (v_mid + (v - wide / 2.0) * v_half).round() as isize;
            let hi = (v_mid + (v + wide / 2.0) * v_half).round() as isize;
            for j in lo..=hi {
                paint(&mut grid, i, j);
            }
        }
    }
    // Flecks shaken off the brush, most by the dry end and the edges.
    let flecks = 8 + (rng.next() * 10.0) as usize;
    for _ in 0..flecks {
        let u = rng.range(0.3, 1.02);
        let side = if rng.next() < 0.5 { -1.0 } else { 1.0 };
        let v = side * rng.range(1.02, 1.22);
        let (ci, cj) = (u0 + u * u_len, v_mid + v * v_half);
        let r = rng.range(0.6, 2.2);
        let stretch = rng.range(1.0, 3.0);
        let reach = (r * stretch).ceil() as isize + 1;
        for di in -reach..=reach {
            for dj in -reach..=reach {
                let (x, y) = (di as f64 / stretch, dj as f64);
                if x * x + y * y <= r * r {
                    paint(&mut grid, ci as isize + di, cj as isize + dj);
                }
            }
        }
    }
    // Traced, simplified, bent along the line and turned.
    let len = s.long;
    let controls = match s.line {
        Line::Straight => [
            Point::new(len / 3.0, rng.range(-0.06, 0.06)),
            Point::new(2.0 * len / 3.0, rng.range(-0.06, 0.06)),
        ],
        Line::Bow(b) => [
            Point::new(len / 3.0, b * len * 0.45),
            Point::new(2.0 * len / 3.0, b * len * 0.45),
        ],
        Line::Wave(b) => [
            Point::new(len / 3.0, b * len * 0.5),
            Point::new(2.0 * len / 3.0, -b * len * 0.5),
        ],
    };
    let curve = [
        Point::new(0.0, 0.0),
        controls[0],
        controls[1],
        Point::new(len, 0.0),
    ];
    let half = 0.5;
    let bend = |p: Point| {
        let u = (p.x - u0) / u_len;
        let v = (p.y - v_mid) / v_half;
        let t = u.clamp(0.0, 1.0);
        let (at, d) = cubic(curve, t);
        let along = d.normalize();
        let normal = Vec2::new(-along.y, along.x);
        // Past either end, straight on along the tangent.
        at + along * ((u - t) * len) + normal * (v * half)
    };
    let mut tolerance = 0.7;
    loop {
        let mut path = BezPath::new();
        let mut count = 0;
        for ring in trace(&grid, ALONG, ACROSS) {
            let ring = smoothed(&simplified(&ring, tolerance));
            if ring.len() < 3 {
                continue;
            }
            count += ring.len() + 1;
            let points: Vec<Point> = ring.into_iter().map(bend).collect();
            path.extend(polygon(&points));
        }
        if count < 3900 || tolerance > 6.0 {
            path.apply_affine(kurbo::Affine::rotate(s.angle.to_radians()));
            // Filled one way like every shape; the holes stay the other.
            return if kurbo::Shape::area(&path) < 0.0 {
                path.reverse_subpaths()
            } else {
                path
            };
        }
        tolerance *= 1.4;
    }
}

/// The outlines of the painted cells of `grid`, `w` × `h`, along the cell
/// sides, each turned so the paint is on its left: the outer outlines one
/// way and the holes the other, as the non-zero rule wants them. Outlines
/// of fewer than three cells are left out.
fn trace(grid: &[bool], w: usize, h: usize) -> Vec<Vec<Point>> {
    use std::collections::BTreeMap;
    let filled = |i: isize, j: isize| {
        (0..w as isize).contains(&i)
            && (0..h as isize).contains(&j)
            && grid[j as usize * w + i as usize]
    };
    // Directed sides between paint and no paint, from corner to corner.
    let mut next: BTreeMap<(isize, isize), Vec<(isize, isize)>> = BTreeMap::new();
    for j in 0..h as isize {
        for i in 0..w as isize {
            if !filled(i, j) {
                continue;
            }
            // Corners: (i, j) top-left ... going round the cell.
            if !filled(i, j - 1) {
                next.entry((i + 1, j)).or_default().push((i, j));
            }
            if !filled(i - 1, j) {
                next.entry((i, j)).or_default().push((i, j + 1));
            }
            if !filled(i, j + 1) {
                next.entry((i, j + 1)).or_default().push((i + 1, j + 1));
            }
            if !filled(i + 1, j) {
                next.entry((i + 1, j + 1)).or_default().push((i + 1, j));
            }
        }
    }
    let mut rings = Vec::new();
    while let Some((&start, _)) = next.iter().find(|(_, to)| !to.is_empty()) {
        let mut ring = vec![start];
        let mut at = start;
        let mut came: Option<(isize, isize)> = None;
        loop {
            let Some(outs) = next.get_mut(&at) else {
                break;
            };
            if outs.is_empty() {
                break;
            }
            // At a corner two cells share, keep turning the same way.
            let pick = match came {
                Some(from) if outs.len() > 1 => {
                    let d = (at.0 - from.0, at.1 - from.1);
                    let left = (d.1, -d.0);
                    outs.iter()
                        .position(|o| (o.0 - at.0, o.1 - at.1) == left)
                        .unwrap_or(0)
                }
                _ => 0,
            };
            let to = outs.swap_remove(pick);
            if to == start {
                break;
            }
            came = Some(at);
            at = to;
            ring.push(at);
        }
        if ring.len() >= 8 {
            rings.push(
                ring.into_iter()
                    .map(|(x, y)| Point::new(x as f64, y as f64))
                    .collect(),
            );
        }
    }
    rings
}

/// `ring` with its corners cut (Chaikin): each side keeps its middle half,
/// so the steps of the grid round off.
fn smoothed(ring: &[Point]) -> Vec<Point> {
    let n = ring.len();
    (0..n)
        .flat_map(|k| {
            let (a, b) = (ring[k], ring[(k + 1) % n]);
            [a.lerp(b, 0.25), a.lerp(b, 0.75)]
        })
        .collect()
}

/// `ring` with the corners that lie within `tolerance` of the line of
/// their neighbours dropped (Ramer–Douglas–Peucker on a closed ring).
fn simplified(ring: &[Point], tolerance: f64) -> Vec<Point> {
    fn reduce(points: &[Point], tolerance: f64, out: &mut Vec<Point>) {
        let (first, last) = (points[0], points[points.len() - 1]);
        let line = last - first;
        let len = line.hypot();
        let mut far = (0, 0.0);
        for (k, p) in points.iter().enumerate().take(points.len() - 1).skip(1) {
            let d = if len > 0.0 {
                ((*p - first).cross(line) / len).abs()
            } else {
                (*p - first).hypot()
            };
            if d > far.1 {
                far = (k, d);
            }
        }
        if far.1 > tolerance {
            reduce(&points[..=far.0], tolerance, out);
            reduce(&points[far.0..], tolerance, out);
        } else {
            out.push(first);
        }
    }
    if ring.len() < 4 {
        return ring.to_vec();
    }
    // Split at the point furthest from the first, so both halves are open.
    let far = ring
        .iter()
        .enumerate()
        .max_by(|a, b| {
            (*a.1 - ring[0])
                .hypot2()
                .total_cmp(&(*b.1 - ring[0]).hypot2())
        })
        .map_or(ring.len() / 2, |(k, _)| k)
        .max(1);
    let mut closed = ring.to_vec();
    closed.push(ring[0]);
    let mut out = Vec::new();
    reduce(&closed[..=far], tolerance, &mut out);
    reduce(&closed[far..], tolerance, &mut out);
    out
}

/// Splatter `index`: a wet blob with spikes running out of it, drops at
/// their tips and spray around.
fn splatter(index: usize) -> BezPath {
    let mut rng = Rng::new(5000 + index as u64);
    let r = 1.0;
    let rays = 7 + (rng.next() * 9.0) as usize;
    let rim = Noise::new(&mut rng, 12);
    let spikes: Vec<(f64, f64, f64)> = (0..rays)
        .map(|_| {
            (
                rng.range(0.0, TAU),
                rng.range(0.35, 1.3),
                rng.range(0.06, 0.16),
            )
        })
        .collect();
    let n = 260;
    let outline: Vec<Point> = (0..n)
        .map(|k| {
            let a = TAU * k as f64 / n as f64;
            let mut radius = r * (1.0 + 0.18 * rim.at(a / TAU * 12.0));
            for &(at, long, wide) in &spikes {
                let d = ((a - at + PI).rem_euclid(TAU) - PI).abs();
                let bump = (1.0 - d / wide).max(0.0);
                radius += r * long * bump.powi(3);
            }
            Point::new(a.cos() * radius, a.sin() * radius)
        })
        .collect();
    let mut path = turned(polygon(&outline), true);
    // A drop at the end of most spikes.
    for &(at, long, _) in &spikes {
        if rng.next() < 0.75 {
            let far = r * (1.0 + long + rng.range(0.1, 0.35));
            let centre = Point::new(at.cos() * far, at.sin() * far);
            let toward = Vec2::new(at.cos(), at.sin());
            let size = rng.range(0.05, 0.13);
            let stretch = toward * rng.range(0.0, 0.8);
            path.extend(speck(&mut rng, centre, size, stretch));
        }
    }
    // Spray: small drops further out.
    let spray = 14 + (rng.next() * 14.0) as usize;
    for _ in 0..spray {
        let a = rng.range(0.0, TAU);
        let far = r * rng.range(1.5, 2.5);
        let centre = Point::new(a.cos() * far, a.sin() * far);
        let size = rng.range(0.015, 0.06);
        let toward = Vec2::new(a.cos(), a.sin());
        let stretch = toward * rng.range(0.0, 1.2);
        path.extend(speck(&mut rng, centre, size, stretch));
    }
    if index % 2 == 1 {
        // A splash thrown one way: stretched along a slant.
        let k = 1.0 + 0.35 * (index as f64 / SPLATTERS as f64);
        path.apply_affine(
            kurbo::Affine::rotate(0.6 * index as f64)
                * kurbo::Affine::scale_non_uniform(k, 1.0 / k),
        );
    }
    path
}

/// Claw mark `index`: three or four long slashes side by side, as a claw
/// tears paper or skin: thin and sharp at both ends, widest a little past
/// the middle, with angular torn teeth along both edges.
fn claw(index: usize) -> BezPath {
    let mut rng = Rng::new(9000 + index as u64);
    let count = [4, 3, 4, 4, 3, 4, 3, 4][index % CLAWS];
    let bend = [0.12, 0.0, -0.1, 0.2, 0.08, -0.16, 0.25, 0.04][index % CLAWS];
    let angle: f64 = [-35.0, -50.0, -28.0, -42.0, -60.0, -32.0, -45.0, -22.0][index % CLAWS];
    let wide: f64 = [0.15, 0.11, 0.19, 0.13, 0.17, 0.12, 0.16, 0.2][index % CLAWS];
    let spacing = (wide * 3.4).max(0.45);
    let mut path = BezPath::new();
    for c in 0..count {
        let offset = (c as f64 - (count - 1) as f64 / 2.0) * spacing;
        // The middle slashes are the longest; each starts a little apart.
        let middle = 1.0 - (offset / (spacing * count as f64)).abs();
        let long = 3.2 * (0.72 + 0.32 * middle) * rng.range(0.92, 1.05);
        let shift = rng.range(-0.2, 0.2) + 0.35 * (1.0 - middle);
        let width = wide * rng.range(0.8, 1.15);
        let curve = [
            Point::new(shift, offset),
            Point::new(shift + long / 3.0, offset + bend * long * 0.5),
            Point::new(shift + 2.0 * long / 3.0, offset + bend * long * 0.5),
            Point::new(shift + long, offset + bend * long * 0.15),
        ];
        // Each edge torn in teeth: points at uneven steps, in and out.
        let edge = |rng: &mut Rng, sign: f64| {
            let mut points = Vec::new();
            let mut t: f64 = 0.0;
            let mut out = true;
            while t < 1.0 {
                let (p, d) = cubic(curve, t);
                let normal = Vec2::new(-d.y, d.x).normalize();
                // A long thin start, the widest part at 0.6, a sharp end.
                let body = if t < 0.6 {
                    (t / 0.6).powf(1.6)
                } else {
                    ((1.0 - t) / 0.4).powf(0.8)
                } * width;
                let tooth = if out {
                    rng.range(0.9, 1.2) + if rng.next() < 0.12 { 0.3 } else { 0.0 }
                } else {
                    rng.range(0.55, 0.85)
                };
                // Now and then a deeper tear.
                let tear = if rng.next() < 0.08 { 0.35 } else { 1.0 };
                points.push(p + normal * (sign * body * tooth * tear));
                out = !out;
                t += rng.range(0.015, 0.05);
            }
            points
        };
        let (tail, _) = cubic(curve, 0.0);
        let (tip, _) = cubic(curve, 1.0);
        let mut outline = vec![tail];
        outline.extend(edge(&mut rng, 1.0));
        outline.push(tip);
        let mut other = edge(&mut rng, -1.0);
        other.reverse();
        outline.extend(other);
        path.extend(turned(polygon(&outline), true));
        // A loose shard torn off beside a few slashes.
        if rng.next() < 0.4 {
            let t = rng.range(0.4, 0.8);
            let (p, d) = cubic(curve, t);
            let along = d.normalize();
            let normal = Vec2::new(-along.y, along.x);
            let side = if rng.next() < 0.5 { -1.0 } else { 1.0 };
            let at = p + normal * (side * width * 1.6);
            let len = rng.range(0.08, 0.18);
            let shard = [
                at - along * len,
                at + normal * (side * width * 0.25),
                at + along * len,
                at - normal * (side * width * 0.1),
            ];
            path.extend(turned(polygon(&shard), true));
        }
    }
    path.apply_affine(kurbo::Affine::rotate(angle.to_radians()));
    path
}

/// The path of traced shape `index`: the `d` of its SVG, read under the
/// even-odd rule and turned for the non-zero one.
fn traced(index: usize) -> BezPath {
    let (svg, turn) = TRACED_SVGS[index % TRACED];
    let path = svg
        .split(" d=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .and_then(|d| BezPath::from_svg(d).ok())
        .unwrap_or_default();
    let mut path = even_odd_as_non_zero(&path);
    path.apply_affine(kurbo::Affine::rotate(turn.to_radians()));
    path
}

/// `path`, filled under the even-odd rule, as the same shape under the
/// non-zero one: each outline turned one way when an even number of the
/// others hold it and the other way when an odd number do.
fn even_odd_as_non_zero(path: &BezPath) -> BezPath {
    use kurbo::Shape as _;
    let mut pieces: Vec<BezPath> = Vec::new();
    for el in path.elements() {
        if matches!(el, kurbo::PathEl::MoveTo(_)) || pieces.is_empty() {
            pieces.push(BezPath::new());
        }
        if let Some(last) = pieces.last_mut() {
            last.push(*el);
        }
    }
    let first = |piece: &BezPath| match piece.elements().first() {
        Some(kurbo::PathEl::MoveTo(p)) => Some(*p),
        _ => None,
    };
    let mut out = BezPath::new();
    for (k, piece) in pieces.iter().enumerate() {
        let Some(at) = first(piece) else {
            continue;
        };
        // Only a larger outline holds it: a drop touching a blob does not.
        let own = piece.area().abs();
        let depth = pieces
            .iter()
            .enumerate()
            .filter(|(j, other)| *j != k && other.area().abs() > own && other.winding(at) != 0)
            .count();
        out.extend(turned(piece.clone(), depth % 2 == 0));
    }
    out
}

/// Every grunge path, made once.
fn all() -> &'static [BezPath] {
    static PATHS: OnceLock<Vec<BezPath>> = OnceLock::new();
    PATHS.get_or_init(|| {
        (0..STROKES)
            .map(stroke)
            .chain((0..TRACED).map(traced))
            .chain((0..SPLATTERS).map(splatter))
            .chain((0..CLAWS).map(claw))
            .collect()
    })
}

/// Dry brush stroke `index`.
pub(crate) fn stroke_path(index: usize) -> BezPath {
    all()[index.min(STROKES - 1)].clone()
}

/// Traced shape `index`.
pub(crate) fn traced_path(index: usize) -> BezPath {
    all()[STROKES + index.min(TRACED - 1)].clone()
}

/// Splatter `index`.
pub(crate) fn splatter_path(index: usize) -> BezPath {
    all()[STROKES + TRACED + index.min(SPLATTERS - 1)].clone()
}

/// Claw mark `index`.
pub(crate) fn claw_path(index: usize) -> BezPath {
    all()[STROKES + TRACED + SPLATTERS + index.min(CLAWS - 1)].clone()
}

#[cfg(test)]
mod tests {
    use kurbo::Shape as _;

    use super::*;

    // S109-02: the shapes are the same on every run, finite, with area,
    // holes cut inside the strokes and more than one piece each.
    #[test]
    fn s109_02_grunge_shapes_are_fixed_and_filled() {
        assert_eq!(stroke(3), stroke(3), "the same every time");
        // S109-03: the owner's traced shapes are read whole: many pieces,
        // filled, the holes kept.
        for i in 0..TRACED {
            let path = traced(i);
            let pieces = path
                .elements()
                .iter()
                .filter(|e| matches!(e, kurbo::PathEl::MoveTo(_)))
                .count();
            assert!(pieces > 3, "traced {i}: {pieces} pieces");
            assert!(path.area() > 0.0, "traced {i} filled");
        }
        for path in all() {
            let b = path.bounding_box();
            assert!(b.width().is_finite() && b.height().is_finite());
            assert!(b.width() > 0.0 && b.height() > 0.0);
            assert!(path.area() > 0.0, "filled");
            let pieces = path
                .elements()
                .iter()
                .filter(|e| matches!(e, kurbo::PathEl::MoveTo(_)))
                .count();
            assert!(pieces > 1, "with flecks, drops or slashes");
        }
        // Every stroke has bristle streaks: pieces turned against the fill.
        for i in 0..STROKES {
            let path = stroke(i);
            let mut piece = BezPath::new();
            let mut holes = 0;
            for el in path.elements() {
                if matches!(el, kurbo::PathEl::MoveTo(_)) && !piece.elements().is_empty() {
                    holes += usize::from(piece.area() < 0.0);
                    piece = BezPath::new();
                }
                piece.push(*el);
            }
            assert!(holes >= 1, "stroke {i}: {holes} streaks");
        }
    }
}
