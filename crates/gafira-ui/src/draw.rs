//! Dibujo (spec 079): freehand drawing with brushes. A stroke is drawn by
//! hand and, when the pointer lets go, becomes one vector element: the
//! outline of the stroke as a filled path, so its colour and transparency
//! change like any shape's. The eraser removes whole strokes.
//!
//! The brushes are a catalogue ([`BRUSHES`]): each one is a tip (how wide,
//! how its ends taper, how speed and direction shape it, how rough its
//! edge is). Adding a brush is adding an entry and its name; its look in
//! the panel is drawn by the brush itself.

use egui::{Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use gafira_doc::msg::{Message, MessageKey};
use gafira_doc::{BlendMode, NodeId, PageId, PathCommand, ShapePath};

use crate::style::Theme;
use crate::text::Texts;

/// The ends of a stroke.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cap {
    /// Rounded, like ink.
    Round,
    /// Cut straight across, like a felt chisel.
    Flat,
}

/// What makes a brush's line its own; each brush starts with its own and
/// the person can change them (spec 079).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Style {
    /// How much the hand's shake is smoothed, `0..=1`.
    pub(crate) smoothing: f32,
    /// How thin the stroke starts, `0..=1` (0 = full width at once).
    pub(crate) taper_start: f32,
    /// How thin the stroke ends, `0..=1`.
    pub(crate) taper_end: f32,
    /// How much a fast stroke thins, `0..=1`.
    pub(crate) speed: f32,
    /// The angle of a flat nib in degrees, for calligraphy; `None` for a
    /// round tip.
    pub(crate) nib: Option<f32>,
    /// How rough the edge is, `0..=1` (pencil, crayon).
    pub(crate) roughness: f32,
}

/// A brush of the catalogue.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Brush {
    /// Kept with the strokes it draws (`ShapeNode::drawn_with`).
    pub(crate) id: &'static str,
    /// The key of its name.
    pub(crate) key: &'static str,
    /// Its colour on first use (0xRRGGBB).
    pub(crate) colour: u32,
    /// Its width in page px on first use, and the range the slider offers.
    pub(crate) size: f32,
    pub(crate) sizes: (f32, f32),
    /// Its opacity on first use, `0..=1`.
    pub(crate) opacity: f32,
    pub(crate) cap: Cap,
    /// How the stroke mixes with what is under it (the highlighter
    /// multiplies, so text under it stays readable).
    pub(crate) blend: BlendMode,
    pub(crate) style: Style,
}

const fn style(
    smoothing: f32,
    taper: (f32, f32),
    speed: f32,
    nib: Option<f32>,
    roughness: f32,
) -> Style {
    Style {
        smoothing,
        taper_start: taper.0,
        taper_end: taper.1,
        speed,
        nib,
        roughness,
    }
}

/// The brushes, in the panel's order.
pub(crate) const BRUSHES: [Brush; 16] = [
    Brush {
        id: "pen",
        key: "draw.brush.pen",
        colour: 0x1B4FA0,
        size: 4.0,
        sizes: (1.0, 40.0),
        opacity: 1.0,
        cap: Cap::Round,
        blend: BlendMode::Normal,
        style: style(0.5, (0.0, 0.0), 0.0, None, 0.0),
    },
    Brush {
        id: "marker",
        key: "draw.brush.marker",
        colour: 0xE84545,
        size: 10.0,
        sizes: (2.0, 80.0),
        opacity: 1.0,
        cap: Cap::Round,
        blend: BlendMode::Normal,
        style: style(0.55, (0.0, 0.0), 0.0, None, 0.0),
    },
    Brush {
        id: "highlighter",
        key: "draw.brush.highlighter",
        colour: 0xFFE14D,
        size: 26.0,
        sizes: (6.0, 120.0),
        opacity: 0.6,
        cap: Cap::Flat,
        blend: BlendMode::Multiply,
        style: style(0.7, (0.0, 0.0), 0.0, None, 0.0),
    },
    Brush {
        id: "fineliner",
        key: "draw.brush.fineliner",
        colour: 0x18243B,
        size: 2.0,
        sizes: (0.5, 12.0),
        opacity: 1.0,
        cap: Cap::Round,
        blend: BlendMode::Normal,
        style: style(0.35, (0.0, 0.0), 0.0, None, 0.0),
    },
    Brush {
        id: "pencil",
        key: "draw.brush.pencil",
        colour: 0x46526B,
        size: 3.0,
        sizes: (1.0, 20.0),
        opacity: 0.85,
        cap: Cap::Round,
        blend: BlendMode::Normal,
        style: style(0.3, (0.3, 0.5), 0.2, None, 0.45),
    },
    Brush {
        id: "brush",
        key: "draw.brush.brush",
        colour: 0x7B61FF,
        size: 16.0,
        sizes: (2.0, 100.0),
        opacity: 1.0,
        cap: Cap::Round,
        blend: BlendMode::Normal,
        style: style(0.6, (0.6, 0.8), 0.6, None, 0.0),
    },
    Brush {
        id: "calligraphy",
        key: "draw.brush.calligraphy",
        colour: 0x18243B,
        size: 14.0,
        sizes: (2.0, 80.0),
        opacity: 1.0,
        cap: Cap::Flat,
        blend: BlendMode::Normal,
        style: style(0.6, (0.0, 0.0), 0.0, Some(45.0), 0.0),
    },
    Brush {
        id: "crayon",
        key: "draw.brush.crayon",
        colour: 0xFF6B57,
        size: 12.0,
        sizes: (3.0, 60.0),
        opacity: 0.9,
        cap: Cap::Round,
        blend: BlendMode::Normal,
        style: style(0.45, (0.1, 0.2), 0.0, None, 0.9),
    },
    Brush {
        id: "neon",
        key: "draw.brush.neon",
        colour: 0x3FBFA9,
        size: 8.0,
        sizes: (2.0, 60.0),
        opacity: 1.0,
        cap: Cap::Round,
        blend: BlendMode::Screen,
        style: style(0.75, (0.2, 0.2), 0.0, None, 0.0),
    },
    Brush {
        id: "fountain",
        key: "draw.brush.fountain",
        colour: 0x17324D,
        size: 5.0,
        sizes: (1.0, 30.0),
        opacity: 1.0,
        cap: Cap::Round,
        blend: BlendMode::Normal,
        style: style(0.55, (0.2, 0.4), 0.5, None, 0.0),
    },
    Brush {
        id: "brush_pen",
        key: "draw.brush.brush_pen",
        colour: 0xE0457B,
        size: 10.0,
        sizes: (2.0, 60.0),
        opacity: 1.0,
        cap: Cap::Round,
        blend: BlendMode::Normal,
        style: style(0.6, (0.5, 0.7), 0.5, None, 0.0),
    },
    Brush {
        id: "ink",
        key: "draw.brush.ink",
        colour: 0x111111,
        size: 12.0,
        sizes: (2.0, 80.0),
        opacity: 1.0,
        cap: Cap::Round,
        blend: BlendMode::Normal,
        style: style(0.5, (0.3, 0.6), 0.8, None, 0.1),
    },
    Brush {
        id: "watercolor",
        key: "draw.brush.watercolor",
        colour: 0x2563EB,
        size: 30.0,
        sizes: (6.0, 120.0),
        opacity: 0.35,
        cap: Cap::Round,
        blend: BlendMode::Multiply,
        style: style(0.8, (0.4, 0.4), 0.3, None, 0.25),
    },
    Brush {
        id: "chalk",
        key: "draw.brush.chalk",
        colour: 0x46526B,
        size: 10.0,
        sizes: (3.0, 60.0),
        opacity: 0.85,
        cap: Cap::Round,
        blend: BlendMode::Normal,
        style: style(0.4, (0.05, 0.1), 0.0, None, 1.0),
    },
    Brush {
        id: "charcoal",
        key: "draw.brush.charcoal",
        colour: 0x2B2B2B,
        size: 16.0,
        sizes: (4.0, 80.0),
        opacity: 0.8,
        cap: Cap::Round,
        blend: BlendMode::Normal,
        style: style(0.4, (0.2, 0.3), 0.2, None, 0.8),
    },
    Brush {
        id: "dry_brush",
        key: "draw.brush.dry_brush",
        colour: 0x17695A,
        size: 22.0,
        sizes: (4.0, 100.0),
        opacity: 0.9,
        cap: Cap::Flat,
        blend: BlendMode::Normal,
        style: style(0.5, (0.1, 0.5), 0.3, None, 1.0),
    },
];

/// The brush with `id`.
#[cfg(test)]
fn brush(id: &str) -> Option<&'static Brush> {
    BRUSHES.iter().find(|b| b.id == id)
}

/// A point of a stroke as the hand drew it, in page px.
pub(crate) type Point = [f64; 2];

/// A deterministic noise in `-1..=1` for the point `i` of a stroke (so the
/// same stroke always has the same rough edge).
fn noise(i: usize) -> f64 {
    let mut x = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03;
    x ^= x >> 29;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 32;
    (x % 20_001) as f64 / 10_000.0 - 1.0
}

/// The hand's points smoothed: each one pulled towards the line so far by
/// `smoothing`; the last one kept, so the stroke ends where the hand did.
fn smoothed(points: &[Point], smoothing: f32) -> Vec<Point> {
    let keep = 1.0 - 0.85 * f64::from(smoothing.clamp(0.0, 1.0));
    let mut out: Vec<Point> = Vec::with_capacity(points.len());
    for (i, p) in points.iter().enumerate() {
        match out.last() {
            Some(q) if i + 1 < points.len() => {
                out.push([q[0] + (p[0] - q[0]) * keep, q[1] + (p[1] - q[1]) * keep]);
            }
            _ => out.push(*p),
        }
    }
    out
}

fn dist(a: Point, b: Point) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// The two edges of a stroke, or a dot.
pub(crate) enum Sides {
    /// The left and right edges, point by point, and the centres of the
    /// two ends.
    Edges {
        left: Vec<Point>,
        right: Vec<Point>,
        ends: (Point, Point),
    },
    /// A click without moving: a disc.
    Dot { centre: Point, radius: f64 },
}

/// The edges of a stroke through `points` (page px), `size` px wide, with
/// `style`.
pub(crate) fn sides(points: &[Point], size: f64, style: &Style) -> Sides {
    let half = (size / 2.0).max(0.25);
    let smooth = smoothed(points, style.smoothing);
    // Points closer than a fraction of the width add nothing but bumps.
    let spacing = (size * 0.12).clamp(0.4, 4.0);
    let mut line: Vec<(Point, f64)> = Vec::new();
    for (i, p) in smooth.iter().enumerate() {
        // How fast the hand went here: the distance between the raw points.
        let speed = if i > 0 {
            dist(points[i], points[i - 1]) / size.max(1.0)
        } else {
            0.0
        };
        match line.last() {
            Some((q, _)) if dist(*q, *p) < spacing && i + 1 < smooth.len() => {}
            _ => line.push((*p, speed)),
        }
    }
    if line.len() < 2 || total_length(&line) < spacing {
        let centre = line.first().map_or([0.0, 0.0], |(p, _)| *p);
        return Sides::Dot {
            centre,
            radius: half,
        };
    }
    let total = total_length(&line);
    let taper_len = (total / 2.0).min(size * 10.0);
    let mut along = 0.0;
    let mut widths = Vec::with_capacity(line.len());
    for (i, (p, speed)) in line.iter().enumerate() {
        if i > 0 {
            along += dist(line[i - 1].0, *p);
        }
        let mut w = half;
        let ease = |t: f64| {
            let t = t.clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };
        if style.taper_start > 0.0 {
            let t = along / (taper_len * f64::from(style.taper_start)).max(1e-9);
            w *= 0.12 + 0.88 * ease(t);
        }
        if style.taper_end > 0.0 {
            let t = (total - along) / (taper_len * f64::from(style.taper_end)).max(1e-9);
            w *= 0.12 + 0.88 * ease(t);
        }
        if style.speed > 0.0 {
            w *= 1.0 - f64::from(style.speed) * (speed * 0.35).clamp(0.0, 0.65);
        }
        if let Some(nib) = style.nib {
            let (a, b) = (
                line[i.saturating_sub(1)].0,
                line[(i + 1).min(line.len() - 1)].0,
            );
            let heading = (b[1] - a[1]).atan2(b[0] - a[0]);
            let across = (heading - f64::from(nib).to_radians()).sin().abs();
            w *= 0.18 + 0.82 * across;
        }
        if style.roughness > 0.0 {
            w *= 1.0 + f64::from(style.roughness) * 0.3 * noise(i);
        }
        widths.push(w.max(0.2));
    }
    // A light blur of the widths keeps the edge fluid.
    let widths: Vec<f64> = (0..widths.len())
        .map(|i| {
            let a = widths[i.saturating_sub(1)];
            let c = widths[(i + 1).min(widths.len() - 1)];
            (a + 2.0 * widths[i] + c) / 4.0
        })
        .collect();
    let n = line.len();
    let normal = |i: usize| {
        let (a, b) = (line[i.saturating_sub(1)].0, line[(i + 1).min(n - 1)].0);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len = dx.hypot(dy).max(1e-9);
        [-dy / len, dx / len]
    };
    let mut left = Vec::with_capacity(n);
    let mut right = Vec::with_capacity(n);
    for i in 0..n {
        let (p, w, m) = (line[i].0, widths[i], normal(i));
        // Calligraphy keeps the nib's angle: its edge points along the nib.
        let m = match style.nib {
            Some(nib) => {
                let a = f64::from(nib).to_radians();
                [a.cos(), a.sin()]
            }
            None => m,
        };
        let jitter = if style.roughness > 0.0 {
            f64::from(style.roughness) * 0.15 * w * noise(i + 7919)
        } else {
            0.0
        };
        left.push([p[0] + m[0] * (w + jitter), p[1] + m[1] * (w + jitter)]);
        right.push([p[0] - m[0] * (w - jitter), p[1] - m[1] * (w - jitter)]);
    }
    Sides::Edges {
        left,
        right,
        ends: (line[0].0, line[n - 1].0),
    }
}

/// The outline of a stroke through `points` (page px), `size` px wide,
/// with `cap` and `style`: a closed polygon, filled to draw it.
pub(crate) fn outline(points: &[Point], size: f64, cap: Cap, style: &Style) -> Vec<Point> {
    match sides(points, size, style) {
        Sides::Dot { centre, radius } => (0..24)
            .map(|k| {
                let a = f64::from(k) / 24.0 * std::f64::consts::TAU;
                [centre[0] + radius * a.cos(), centre[1] + radius * a.sin()]
            })
            .collect(),
        Sides::Edges { left, right, ends } => {
            let n = left.len();
            let mut out = left.clone();
            cap_points(&mut out, ends.1, left[n - 1], cap);
            out.extend(right.iter().rev());
            cap_points(&mut out, ends.0, right[0], cap);
            out
        }
    }
}

/// The points of an end of a stroke at `centre`, going from `from` half a
/// turn round the outside.
fn cap_points(out: &mut Vec<Point>, centre: Point, from: Point, cap: Cap) {
    if cap == Cap::Flat {
        return;
    }
    let r = dist(centre, from);
    let start = (from[1] - centre[1]).atan2(from[0] - centre[0]);
    // Half a turn, the long way round the outside of the stroke.
    for k in 1..8 {
        let a = start - std::f64::consts::PI * f64::from(k) / 8.0;
        out.push([centre[0] + r * a.cos(), centre[1] + r * a.sin()]);
    }
}

fn total_length(line: &[(Point, f64)]) -> f64 {
    line.windows(2).map(|w| dist(w[0].0, w[1].0)).sum()
}

/// A stroke's outline as an element: its box in page px `(x, y, w, h)`
/// and its path in the unit box, drawn with curves through the midpoints
/// so it stays fluid. `None` for an empty outline.
pub(crate) fn to_path(outline: &[Point]) -> Option<((f64, f64, f64, f64), ShapePath)> {
    if outline.len() < 3 {
        return None;
    }
    // A path holds at most 4096 orders: a long stroke keeps every k-th point.
    let step = outline.len().div_ceil(ShapePath::MAX_COMMANDS - 8).max(1);
    let points: Vec<Point> = outline.iter().step_by(step).copied().collect();
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for p in &points {
        x0 = x0.min(p[0]);
        y0 = y0.min(p[1]);
        x1 = x1.max(p[0]);
        y1 = y1.max(p[1]);
    }
    let (w, h) = ((x1 - x0).max(1.0), (y1 - y0).max(1.0));
    let unit = |p: Point| [(p[0] - x0) / w, (p[1] - y0) / h];
    let mid = |a: Point, b: Point| unit([(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0]);
    let n = points.len();
    let mut commands = Vec::with_capacity(n + 2);
    commands.push(PathCommand::MoveTo(mid(points[n - 1], points[0])));
    for i in 0..n {
        let next = points[(i + 1) % n];
        commands.push(PathCommand::QuadTo(unit(points[i]), mid(points[i], next)));
    }
    commands.push(PathCommand::Close);
    Some(((x0, y0, w, h), ShapePath { commands }))
}

/// Whether the point `at` (page px) touches the polygon `outline`, or
/// passes within `reach` px of its edge: what the eraser asks of a stroke.
pub(crate) fn touches(outline: &[Point], at: Point, reach: f64) -> bool {
    let n = outline.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut near = false;
    for i in 0..n {
        let (a, b) = (outline[i], outline[(i + 1) % n]);
        if (a[1] > at[1]) != (b[1] > at[1]) {
            let x = a[0] + (at[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
            if at[0] < x {
                inside = !inside;
            }
        }
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len2 = dx * dx + dy * dy;
        let t = if len2 > 0.0 {
            (((at[0] - a[0]) * dx + (at[1] - a[1]) * dy) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        if dist(at, [a[0] + t * dx, a[1] + t * dy]) <= reach {
            near = true;
        }
    }
    inside || near
}

/// The polygon of a path in the unit box, each point placed by `at`
/// (page px, with the element's turn and flips): the curves flattened,
/// for the eraser.
pub(crate) fn flatten(path: &ShapePath, at: impl Fn([f64; 2]) -> Point) -> Vec<Point> {
    let mut out = Vec::new();
    let mut last = [0.0, 0.0];
    for command in &path.commands {
        match *command {
            PathCommand::MoveTo(p) | PathCommand::LineTo(p) => {
                last = at(p);
                out.push(last);
            }
            PathCommand::QuadTo(c, p) => {
                let (c, p) = (at(c), at(p));
                for k in 1..=4 {
                    let t = f64::from(k) / 4.0;
                    let u = 1.0 - t;
                    out.push([
                        u * u * last[0] + 2.0 * u * t * c[0] + t * t * p[0],
                        u * u * last[1] + 2.0 * u * t * c[1] + t * t * p[1],
                    ]);
                }
                last = p;
            }
            PathCommand::CubicTo(a, b, p) => {
                let (a, b, p) = (at(a), at(b), at(p));
                for k in 1..=6 {
                    let t = f64::from(k) / 6.0;
                    let u = 1.0 - t;
                    out.push([
                        u * u * u * last[0]
                            + 3.0 * u * u * t * a[0]
                            + 3.0 * u * t * t * b[0]
                            + t * t * t * p[0],
                        u * u * u * last[1]
                            + 3.0 * u * u * t * a[1]
                            + 3.0 * u * t * t * b[1]
                            + t * t * t * p[1],
                    ]);
                }
                last = p;
            }
            PathCommand::Close => {}
        }
    }
    out
}

/// A sample stroke for a brush's card in the panel: a wave across the box
/// `(w, h)`, in its own px.
pub(crate) fn sample(w: f64, h: f64) -> Vec<Point> {
    (0..=40)
        .map(|k| {
            let t = f64::from(k) / 40.0;
            [
                w * (0.1 + 0.8 * t),
                h * (0.5 - 0.28 * (t * std::f64::consts::TAU * 0.9).sin()),
            ]
        })
        .collect()
}

/// What the drawing tool does with the pointer (spec 079).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tool {
    /// Select and move, as outside Dibujo.
    Select,
    /// Draw with the brush of [`BRUSHES`] at this index.
    Brush(usize),
    /// Remove whole strokes.
    Eraser,
}

/// A brush as the person left it: colour, width, opacity and style.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Setting {
    pub(crate) colour: Color32,
    pub(crate) size: f32,
    pub(crate) opacity: f32,
    pub(crate) style: Style,
}

impl Setting {
    fn of(brush: &Brush) -> Self {
        let c = brush.colour;
        Self {
            colour: Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8),
            size: brush.size,
            opacity: brush.opacity,
            style: brush.style,
        }
    }
}

/// A stroke being drawn: its page and the hand's points in page px.
pub(crate) struct Live {
    pub(crate) page: PageId,
    pub(crate) points: Vec<Point>,
}

/// The state of Dibujo: the tool, each brush's setting, the stroke being
/// drawn and the strokes the eraser has gone over.
pub(crate) struct DrawState {
    pub(crate) tool: Tool,
    pub(crate) settings: Vec<Setting>,
    pub(crate) live: Option<Live>,
    pub(crate) erasing: Option<(PageId, Vec<NodeId>)>,
}

impl Default for DrawState {
    fn default() -> Self {
        Self {
            tool: Tool::Brush(0),
            settings: BRUSHES.iter().map(Setting::of).collect(),
            live: None,
            erasing: None,
        }
    }
}

/// The colours offered under the brushes: Gafira's and the usual ones.
const SWATCHES: [u32; 12] = [
    0x18243B, 0xFFFFFF, 0xE84545, 0xFF6B57, 0xF4A21B, 0xFFE14D, 0x3FBFA9, 0x17695A, 0x1B4FA0,
    0x2563EB, 0x7B61FF, 0xE0457B,
];

fn t(texts: &Texts, key: &'static str) -> String {
    texts.t(&Message {
        key: MessageKey(key),
        args: Vec::new(),
    })
}

/// The mesh of a stroke through `points` (page px) mapped to the screen by
/// `screen`: its edges as a strip and its round ends as fans, for the
/// stroke being drawn and the brushes' cards.
pub(crate) fn mesh(
    points: &[Point],
    (size, cap, style): (f64, Cap, &Style),
    screen: impl Fn(Point) -> Pos2,
    colour: Color32,
) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    let fan = |mesh: &mut egui::Mesh, centre: Point, rim: &[Point]| {
        let base = mesh.vertices.len() as u32;
        mesh.colored_vertex(screen(centre), colour);
        for p in rim {
            mesh.colored_vertex(screen(*p), colour);
        }
        for k in 1..rim.len() as u32 {
            mesh.add_triangle(base, base + k, base + k + 1);
        }
    };
    match sides(points, size, style) {
        Sides::Dot { centre, radius } => {
            let rim: Vec<Point> = (0..=24)
                .map(|k| {
                    let a = f64::from(k) / 24.0 * std::f64::consts::TAU;
                    [centre[0] + radius * a.cos(), centre[1] + radius * a.sin()]
                })
                .collect();
            fan(&mut mesh, centre, &rim);
        }
        Sides::Edges { left, right, ends } => {
            let base = mesh.vertices.len() as u32;
            for (l, r) in left.iter().zip(&right) {
                mesh.colored_vertex(screen(*l), colour);
                mesh.colored_vertex(screen(*r), colour);
            }
            for k in 0..left.len().saturating_sub(1) as u32 {
                let i = base + 2 * k;
                mesh.add_triangle(i, i + 1, i + 2);
                mesh.add_triangle(i + 1, i + 3, i + 2);
            }
            if cap == Cap::Round {
                let n = left.len();
                for (centre, from) in [(ends.1, left[n - 1]), (ends.0, right[0])] {
                    let mut rim = vec![from];
                    cap_points(&mut rim, centre, from, Cap::Round);
                    let r = dist(centre, from);
                    let last = rim.last().copied().unwrap_or(from);
                    let a = (last[1] - centre[1]).atan2(last[0] - centre[0])
                        - std::f64::consts::PI / 8.0;
                    rim.push([centre[0] + r * a.cos(), centre[1] + r * a.sin()]);
                    fan(&mut mesh, centre, &rim);
                }
            }
        }
    }
    mesh
}

/// The Dibujo toolbox: «Seleccionar», the brushes and the eraser as tools,
/// then the colour,
/// the width and transparency of the brush in hand, and its style.
pub(crate) fn panel(ui: &mut egui::Ui, theme: &Theme, texts: &Texts, state: &mut DrawState) {
    egui::ScrollArea::vertical()
        .id_salt("draw.panel")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            heading(ui, theme, texts, "draw.brushes");
            let gap = 8.0;
            let columns: f32 = if ui.available_width() >= 330.0 {
                4.0
            } else {
                3.0
            };
            let side = ((ui.available_width() - gap * (columns - 1.0)) / columns).floor();
            let tools: Vec<Tool> = std::iter::once(Tool::Select)
                .chain((0..BRUSHES.len()).map(Tool::Brush))
                .chain([Tool::Eraser])
                .collect();
            for row in tools.chunks(columns as usize) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for tool in row {
                        if brush_card(ui, theme, texts, state, *tool, side) {
                            state.tool = *tool;
                        }
                    }
                });
                ui.add_space(gap);
            }
            let Tool::Brush(index) = state.tool else {
                return;
            };
            let brush = BRUSHES[index];
            let setting = &mut state.settings[index];
            ui.add_space(8.0);
            heading(ui, theme, texts, "draw.colour");
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(6.0);
                for c in SWATCHES {
                    let colour = Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8);
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
                    let on = setting.colour == colour;
                    ui.painter().circle_filled(rect.center(), 11.0, colour);
                    let ring = if on {
                        Stroke::new(2.0, theme.c("accent"))
                    } else {
                        Stroke::new(1.0, theme.c("border"))
                    };
                    ui.painter()
                        .circle_stroke(rect.center(), if on { 12.0 } else { 11.0 }, ring);
                    if response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if response.clicked() {
                        setting.colour = colour;
                    }
                }
                egui::color_picker::color_edit_button_srgba(
                    ui,
                    &mut setting.colour,
                    egui::color_picker::Alpha::Opaque,
                )
                .on_hover_text(t(texts, "draw.custom_colour"));
            });
            ui.add_space(12.0);
            let (lo, hi) = brush.sizes;
            number_row(
                ui,
                theme,
                texts,
                "draw.size",
                &mut setting.size,
                lo..=hi,
                1.0,
            );
            let mut percent = (setting.opacity * 100.0).round();
            number_row(
                ui,
                theme,
                texts,
                "draw.opacity",
                &mut percent,
                5.0..=100.0,
                1.0,
            );
            setting.opacity = percent / 100.0;
            ui.add_space(10.0);
            let title = RichText::new(t(texts, "draw.style"))
                .font(theme.font("ui-label"))
                .color(theme.c("text"));
            egui::CollapsingHeader::new(title)
                .id_salt("draw.style")
                .default_open(false)
                .show(ui, |ui| {
                    let style = &mut state.settings[index].style;
                    let knob = |ui: &mut egui::Ui, key, value: &mut f32| {
                        let mut percent = (*value * 100.0).round();
                        number_row(ui, theme, texts, key, &mut percent, 0.0..=100.0, 1.0);
                        *value = percent / 100.0;
                    };
                    knob(ui, "draw.knob.smoothing", &mut style.smoothing);
                    knob(ui, "draw.knob.taper_start", &mut style.taper_start);
                    knob(ui, "draw.knob.taper_end", &mut style.taper_end);
                    knob(ui, "draw.knob.speed", &mut style.speed);
                    knob(ui, "draw.knob.texture", &mut style.roughness);
                    if let Some(nib) = style.nib.as_mut() {
                        number_row(ui, theme, texts, "draw.knob.nib", nib, 0.0..=180.0, 1.0);
                    }
                    ui.add_space(4.0);
                    let reset = egui::Button::new(
                        RichText::new(t(texts, "draw.reset")).font(theme.font("body-sm")),
                    );
                    if ui.add(reset).clicked() {
                        state.settings[index] = Setting::of(&brush);
                    }
                });
        });
}

fn heading(ui: &mut egui::Ui, theme: &Theme, texts: &Texts, key: &'static str) {
    ui.label(
        RichText::new(t(texts, key))
            .font(theme.font("ui-label"))
            .color(theme.c("text")),
    );
    ui.add_space(6.0);
}

/// A label with a slider and its number, on one line.
fn number_row(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    key: &'static str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    step: f64,
) {
    ui.label(
        RichText::new(t(texts, key))
            .font(theme.font("body-sm"))
            .color(theme.c("text.secondary")),
    );
    ui.horizontal(|ui| {
        let width = ui.available_width() - 64.0;
        ui.spacing_mut().slider_width = width.max(60.0);
        ui.add(
            egui::Slider::new(value, range.clone())
                .show_value(false)
                .step_by(step),
        );
        ui.add(
            egui::DragValue::new(value)
                .range(range)
                .speed(step)
                .max_decimals(1),
        );
    });
    ui.add_space(4.0);
}

/// One card of the brushes: the brush drawing its own sample stroke in its
/// colour, or the eraser; lit when in hand. `true` when clicked.
fn brush_card(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    state: &DrawState,
    tool: Tool,
    side: f32,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(side, side * 0.78), Sense::click());
    let on = state.tool == tool;
    let fill = if on {
        theme.c("selection.bg")
    } else if response.hovered() {
        theme.c("secondary.bg")
    } else {
        theme.c("surface")
    };
    ui.painter().rect_filled(rect, 10.0, fill);
    let stroke = if on {
        Stroke::new(1.5, theme.c("accent"))
    } else {
        Stroke::new(1.0, theme.c("border"))
    };
    ui.painter()
        .rect_stroke(rect, 10.0, stroke, egui::StrokeKind::Inside);
    let art = Rect::from_min_max(
        rect.min + Vec2::new(8.0, 8.0),
        rect.max - Vec2::new(8.0, 24.0),
    );
    let name = match tool {
        Tool::Brush(index) => {
            let brush = BRUSHES[index];
            let setting = state.settings[index];
            let size = f64::from(setting.size).clamp(2.0, f64::from(art.height()) * 0.45);
            let points = sample(f64::from(art.width()), f64::from(art.height()));
            let colour = setting.colour.gamma_multiply(setting.opacity.max(0.35));
            let mesh = mesh(
                &points,
                (size, brush.cap, &setting.style),
                |p| art.min + Vec2::new(p[0] as f32, p[1] as f32),
                colour,
            );
            ui.painter().add(mesh);
            t(texts, brush.key)
        }
        Tool::Select => {
            // The arrow of the pointer.
            let c = art.center() + Vec2::new(-4.0, -2.0);
            let ink = theme.c("text");
            let head = vec![
                c + Vec2::new(-8.0, -14.0),
                c + Vec2::new(10.0, 4.0),
                c + Vec2::new(-8.0, 6.0),
            ];
            ui.painter()
                .add(egui::Shape::convex_polygon(head, ink, Stroke::NONE));
            let stem = vec![
                c + Vec2::new(-1.0, 3.0),
                c + Vec2::new(3.0, 1.0),
                c + Vec2::new(9.0, 13.0),
                c + Vec2::new(5.0, 15.0),
            ];
            ui.painter()
                .add(egui::Shape::convex_polygon(stem, ink, Stroke::NONE));
            t(texts, "draw.select")
        }
        Tool::Eraser => {
            // A rubber on a slant over the trail it leaves.
            let c = art.center();
            let trail = Stroke::new(1.5, theme.c("text.muted"));
            ui.painter().line_segment(
                [c + Vec2::new(-22.0, 16.0), c + Vec2::new(-4.0, 16.0)],
                trail,
            );
            ui.painter().line_segment(
                [c + Vec2::new(-16.0, 20.0), c + Vec2::new(-8.0, 20.0)],
                trail,
            );
            paint_eraser(ui.painter(), c + Vec2::new(4.0, -2.0), 1.0);
            t(texts, "draw.eraser")
        }
    };
    // One line, cut with «…» when the card is narrow; the tooltip has it
    // whole.
    let mut job = egui::text::LayoutJob::simple_singleline(
        name.clone(),
        theme.font("caption"),
        theme.c("text"),
    );
    job.wrap = egui::text::TextWrapping::truncate_at_width(rect.width() - 8.0);
    let galley = ui.painter().layout_job(job);
    let at = egui::pos2(
        rect.center().x - galley.size().x / 2.0,
        rect.bottom() - 12.0 - galley.size().y / 2.0,
    );
    ui.painter().galley(at, galley, theme.c("text"));
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let hint = match tool {
        Tool::Select => "draw.select_hint",
        Tool::Eraser => "draw.eraser_hint",
        Tool::Brush(_) => "draw.brush_hint",
    };
    response
        .on_hover_text(format!("{name}\n{}", t(texts, hint)))
        .clicked()
}

/// An eraser centred on `centre`, on a slant, `scale` times its 36 × 16
/// px: a pink rubber end (down and left), a blue sleeve with a white band,
/// rounded and outlined.
fn paint_eraser(painter: &egui::Painter, centre: Pos2, scale: f32) {
    let u = Vec2::new(1.0, -1.0).normalized();
    let v = Vec2::new(1.0, 1.0).normalized();
    let (len, wid) = (36.0 * scale, 16.0 * scale);
    let part = |a: f32, b: f32| {
        let from = centre - u * (len / 2.0) + u * (a * len);
        let to = centre - u * (len / 2.0) + u * (b * len);
        vec![
            from - v * (wid / 2.0),
            to - v * (wid / 2.0),
            to + v * (wid / 2.0),
            from + v * (wid / 2.0),
        ]
    };
    let none = Stroke::NONE;
    let pink = Color32::from_rgb(0xF4, 0x9A, 0xB0);
    let blue = Color32::from_rgb(0x3D, 0x6F, 0xD8);
    painter.add(egui::Shape::convex_polygon(part(0.0, 0.42), pink, none));
    painter.add(egui::Shape::convex_polygon(part(0.42, 1.0), blue, none));
    painter.add(egui::Shape::convex_polygon(
        part(0.58, 0.68),
        Color32::WHITE,
        none,
    ));
    // Rounded rubber end and outline.
    let end = centre - u * (len / 2.0);
    painter.add(egui::Shape::convex_polygon(
        vec![
            end - v * (wid / 2.0),
            end - u * (2.0 * scale) - v * (wid / 2.0 - 2.0 * scale),
            end - u * (2.0 * scale) + v * (wid / 2.0 - 2.0 * scale),
            end + v * (wid / 2.0),
        ],
        pink,
        none,
    ));
    let outline: Vec<Pos2> = {
        let o = part(0.0, 1.0);
        vec![o[0], o[1], o[2], o[3]]
    };
    painter.add(egui::Shape::closed_line(
        outline,
        Stroke::new(1.2, Color32::from_rgb(0x18, 0x24, 0x3B)),
    ));
}

/// How a brush looks as the cursor: the length and widths of its tip, the
/// width of its body, and whether the body takes the ink's colour.
struct Glyph {
    tip: f32,
    tip_end: f32,
    body: f32,
    inked_body: bool,
    /// A flat (chisel) tip, square at the point.
    chisel: bool,
}

fn glyph(id: &str) -> Glyph {
    let g = |tip, tip_end, body, inked_body, chisel| Glyph {
        tip,
        tip_end,
        body,
        inked_body,
        chisel,
    };
    match id {
        "marker" | "neon" => g(6.0, 5.0, 8.0, false, false),
        "highlighter" => g(6.0, 7.0, 9.0, true, true),
        "fineliner" => g(8.0, 2.0, 5.0, false, false),
        "pencil" => g(9.0, 6.0, 6.0, true, false),
        "brush" => g(11.0, 6.0, 4.0, false, false),
        "calligraphy" => g(9.0, 5.0, 4.0, false, true),
        "crayon" | "chalk" | "charcoal" => g(7.0, 7.0, 7.0, true, false),
        "fountain" => g(10.0, 3.0, 5.0, false, true),
        "brush_pen" | "ink" | "watercolor" | "dry_brush" => g(11.0, 6.0, 4.0, false, false),
        _ => g(8.0, 4.0, 6.0, false, false),
    }
}

/// The cursor of Dibujo at `at` (spec 079): the tool in hand drawn on a
/// slant, its point on the pointer and its ink in the brush's colour; the
/// eraser as a pink block. The system cursor is hidden meanwhile.
pub(crate) fn paint_cursor(ctx: &egui::Context, at: Pos2, tool: Tool, state: &DrawState) {
    ctx.set_cursor_icon(egui::CursorIcon::None);
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("draw.cursor"),
    ));
    // Along the tool (up and right) and across it.
    let u = Vec2::new(1.0, -1.0).normalized();
    let v = Vec2::new(1.0, 1.0).normalized();
    let quad = |a: f32, b: f32, wa: f32, wb: f32| {
        vec![
            at + u * a - v * (wa / 2.0),
            at + u * b - v * (wb / 2.0),
            at + u * b + v * (wb / 2.0),
            at + u * a + v * (wa / 2.0),
        ]
    };
    let ink_line = Stroke::new(1.0, Color32::from_rgb(0x18, 0x24, 0x3B));
    let poly = |points: Vec<Pos2>, fill: Color32| {
        painter.add(egui::Shape::convex_polygon(points, fill, ink_line));
    };
    match tool {
        Tool::Eraser => {
            // Its rubber end on the pointer.
            paint_eraser(&painter, at + u * (18.0 * 0.62), 0.62);
        }
        Tool::Brush(index) => {
            let brush = BRUSHES[index];
            let ink = state.settings[index].colour;
            let g = glyph(brush.id);
            let body = if g.inked_body { ink } else { Color32::WHITE };
            let tip_start = if g.chisel { g.tip_end * 0.6 } else { 0.0 };
            poly(quad(0.0, g.tip, tip_start, g.tip_end), ink);
            poly(quad(g.tip, g.tip + 18.0, g.body, g.body), body);
            // The cap at the end, in the ink's colour.
            poly(quad(g.tip + 18.0, g.tip + 22.0, g.body, g.body), ink);
            if matches!(
                brush.id,
                "brush" | "brush_pen" | "ink" | "watercolor" | "dry_brush"
            ) {
                // The metal band between bristles and handle.
                poly(
                    quad(g.tip, g.tip + 4.0, g.body + 1.0, g.body + 1.0),
                    Color32::from_gray(170),
                );
            }
        }
        Tool::Select => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wave() -> Vec<Point> {
        (0..200)
            .map(|k| {
                let t = f64::from(k);
                [t * 3.0, 100.0 + 30.0 * (t / 15.0).sin()]
            })
            .collect()
    }

    // Spec 079: every brush turns a hand stroke into one closed outline
    // that becomes a valid path in the unit box around it, and its width
    // shows: the marker's outline is wider than the pen's.
    #[test]
    fn every_brush_makes_one_path() {
        for brush in BRUSHES {
            let outline = outline(&wave(), f64::from(brush.size), brush.cap, &brush.style);
            let ((x, y, w, h), path) = to_path(&outline).expect(brush.id);
            assert!(
                w >= 1.0 && h >= 1.0 && x.is_finite() && y.is_finite(),
                "{}",
                brush.id
            );
            assert!(
                path.commands.len() <= ShapePath::MAX_COMMANDS,
                "{}",
                brush.id
            );
            assert!(matches!(path.commands[0], PathCommand::MoveTo(_)));
            for command in &path.commands {
                if let PathCommand::QuadTo(c, p) = command {
                    for v in c.iter().chain(p) {
                        assert!((-0.001..=1.001).contains(v), "{} {v}", brush.id);
                    }
                }
            }
        }
        let height = |id: &str| {
            let b = brush(id).expect(id);
            let o = outline(
                &[[0.0, 0.0], [200.0, 0.0]],
                f64::from(b.size),
                b.cap,
                &b.style,
            );
            let (lo, hi) = o.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
                (lo.min(p[1]), hi.max(p[1]))
            });
            hi - lo
        };
        assert!(height("marker") > height("pen"));
        assert!((height("pen") - 4.0).abs() < 0.5, "{}", height("pen"));
    }

    // A click without moving leaves a dot as wide as the brush.
    #[test]
    fn a_click_is_a_dot() {
        let pen = brush("pen").expect("pen");
        let dot = outline(&[[50.0, 50.0]], 10.0, pen.cap, &pen.style);
        assert!(
            dot.iter()
                .all(|p| (dist(*p, [50.0, 50.0]) - 5.0).abs() < 1e-6)
        );
        assert!(to_path(&dot).is_some());
    }

    // Spec 079: tapering thins the ends; calligraphy is thin along its nib
    // and wide across it.
    #[test]
    fn styles_shape_the_line() {
        let line: Vec<Point> = (0..=100).map(|k| [f64::from(k) * 4.0, 0.0]).collect();
        let flat = style(0.0, (0.0, 0.0), 0.0, None, 0.0);
        let tapered = style(0.0, (1.0, 1.0), 0.0, None, 0.0);
        let width_at = |o: &[Point], x: f64| {
            let ys: Vec<f64> = o
                .iter()
                .filter(|p| (p[0] - x).abs() < 3.0)
                .map(|p| p[1])
                .collect();
            ys.iter().copied().fold(f64::MIN, f64::max)
                - ys.iter().copied().fold(f64::MAX, f64::min)
        };
        let a = outline(&line, 20.0, Cap::Flat, &flat);
        let b = outline(&line, 20.0, Cap::Flat, &tapered);
        assert!(width_at(&b, 8.0) < width_at(&a, 8.0) * 0.6);
        assert!((width_at(&b, 200.0) - width_at(&a, 200.0)).abs() < 1.0);
        let nib = style(0.0, (0.0, 0.0), 0.0, Some(0.0), 0.0);
        let along: Vec<Point> = (0..=50).map(|k| [f64::from(k) * 4.0, 0.0]).collect();
        let across: Vec<Point> = (0..=50).map(|k| [0.0, f64::from(k) * 4.0]).collect();
        let thin = outline(&along, 20.0, Cap::Flat, &nib);
        let wide = outline(&across, 20.0, Cap::Flat, &nib);
        let spread = |o: &[Point], axis: usize| {
            let (lo, hi) = o.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
                (lo.min(p[axis]), hi.max(p[axis]))
            });
            hi - lo
        };
        assert!(spread(&wide, 0) > spread(&thin, 1) * 3.0);
    }

    // Spec 079: the eraser touches a stroke inside it or near its edge,
    // not away from it; a path flattens back to its polygon.
    #[test]
    fn the_eraser_finds_strokes() {
        let pen = brush("marker").expect("marker");
        let o = outline(&wave(), 10.0, pen.cap, &pen.style);
        assert!(touches(&o, [0.0, 100.0], 2.0));
        assert!(touches(
            &o,
            [3.0 * 50.0, 100.0 + 30.0 * (50.0_f64 / 15.0).sin() + 8.0],
            4.0
        ));
        assert!(!touches(&o, [300.0, 300.0], 4.0));
        let ((x, y, w, h), path) = to_path(&o).expect("path");
        let back = flatten(&path, |p| [x + p[0] * w, y + p[1] * h]);
        assert!(touches(&back, [0.0, 100.0], 2.0));
        assert!(!touches(&back, [300.0, 300.0], 4.0));
    }
}

#[cfg(test)]
mod texts {
    use gafira_doc::msg::MessageKey;
    use gafira_i18n::CatalogTranslator;
    use gafira_ports::i18n::{Locale, Translator};

    use super::BRUSHES;

    // Spec 079: every word of the Dibujo panel reads in both languages.
    #[test]
    fn every_word_is_written() {
        let translator = CatalogTranslator::new();
        let mut keys: Vec<&str> = BRUSHES.iter().map(|b| b.key).collect();
        keys.extend([
            "draw.select",
            "draw.select_hint",
            "draw.brushes",
            "draw.eraser",
            "draw.eraser_hint",
            "draw.colour",
            "draw.custom_colour",
            "draw.size",
            "draw.opacity",
            "draw.style",
            "draw.reset",
            "draw.brush_hint",
            "draw.knob.smoothing",
            "draw.knob.taper_start",
            "draw.knob.taper_end",
            "draw.knob.speed",
            "draw.knob.texture",
            "draw.knob.nib",
        ]);
        for key in keys {
            for locale in [Locale::Es, Locale::En] {
                assert!(
                    translator.has(locale, MessageKey(key)),
                    "{key} ({locale:?})"
                );
            }
        }
    }
}
