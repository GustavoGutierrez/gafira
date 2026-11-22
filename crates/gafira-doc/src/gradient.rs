//! Gradient fills of shapes (spec 063, D-101): linear in any direction or
//! radial, with two to sixteen stops, and the colour at a point of the box.

use gafira_color::{Color, ColorSpaceId, Rgba};
use serde::{Deserialize, Serialize};

/// How a gradient runs across the box of its shape.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GradientKind {
    /// Along a direction through the centre: 0° left to right, 90° top to
    /// bottom, 180° right to left, 270° bottom to top. The ends are where
    /// the corners project, so the first and last colours reach them.
    Linear { angle_deg: f64 },
    /// From the centre of the box to its farthest corner.
    Radial,
}

/// A colour of a gradient and where it sits, 0 the start, 1 the end.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GradientStop {
    pub offset: f32,
    pub color: Color,
}

/// A gradient fill (spec 063).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gradient {
    pub kind: GradientKind,
    pub stops: Vec<GradientStop>,
}

/// What is wrong with a gradient (spec 063).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradientProblem {
    /// Fewer than two or more than sixteen stops.
    StopCount { count: usize },
    /// A stop out of 0–1, not finite, or before the one ahead of it.
    StopOffset { index: usize },
    /// A colour or the angle is not finite.
    NotFinite,
}

impl Gradient {
    /// The fewest and the most stops a gradient holds.
    pub const MIN_STOPS: usize = 2;
    pub const MAX_STOPS: usize = 16;

    /// What is wrong with it, if anything.
    #[must_use]
    pub fn problem(&self) -> Option<GradientProblem> {
        let count = self.stops.len();
        if !(Self::MIN_STOPS..=Self::MAX_STOPS).contains(&count) {
            return Some(GradientProblem::StopCount { count });
        }
        let mut last = 0.0_f32;
        for (index, stop) in self.stops.iter().enumerate() {
            if !stop.offset.is_finite() || !(0.0..=1.0).contains(&stop.offset) || stop.offset < last
            {
                return Some(GradientProblem::StopOffset { index });
            }
            last = stop.offset;
            if !stop.color.is_finite() {
                return Some(GradientProblem::NotFinite);
            }
        }
        match self.kind {
            GradientKind::Linear { angle_deg } if !angle_deg.is_finite() => {
                Some(GradientProblem::NotFinite)
            }
            _ => None,
        }
    }

    /// Where `(x, y)` of a `w` × `h` box falls along the gradient, 0 to 1.
    #[must_use]
    pub fn position(&self, x: f64, y: f64, w: f64, h: f64) -> f64 {
        let (cx, cy) = (w / 2.0, h / 2.0);
        let (dx, dy) = (x - cx, y - cy);
        let t = match self.kind {
            GradientKind::Linear { angle_deg } => {
                let (sin, cos) = angle_deg.to_radians().sin_cos();
                // Half the length over which the corners project.
                let half = (cx * cos).abs() + (cy * sin).abs();
                if half <= f64::EPSILON {
                    0.5
                } else {
                    0.5 + (dx * cos + dy * sin) / (2.0 * half)
                }
            }
            GradientKind::Radial => {
                let radius = cx.hypot(cy);
                if radius <= f64::EPSILON {
                    0.0
                } else {
                    dx.hypot(dy) / radius
                }
            }
        };
        t.clamp(0.0, 1.0)
    }

    /// The colour at `(x, y)` of a `w` × `h` box, in sRGB, mixed channel
    /// by channel between the two stops around it.
    #[must_use]
    pub fn color_at(&self, x: f64, y: f64, w: f64, h: f64) -> Rgba {
        let t = self.position(x, y, w, h) as f32;
        let Some(first) = self.stops.first() else {
            return Rgba::new(0.0, 0.0, 0.0, 0.0);
        };
        if t <= first.offset {
            return srgb(first.color);
        }
        for pair in self.stops.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if t <= b.offset {
                let span = b.offset - a.offset;
                let k = if span <= f32::EPSILON {
                    1.0
                } else {
                    (t - a.offset) / span
                };
                let (ca, cb) = (srgb(a.color), srgb(b.color));
                let mix = |p: f32, q: f32| p + (q - p) * k;
                return Rgba::new(
                    mix(ca.r, cb.r),
                    mix(ca.g, cb.g),
                    mix(ca.b, cb.b),
                    mix(ca.a, cb.a),
                );
            }
        }
        self.stops
            .last()
            .map_or(Rgba::new(0.0, 0.0, 0.0, 0.0), |s| srgb(s.color))
    }

    /// The colour of stop `index` in sRGB.
    #[must_use]
    pub fn stop_rgba(&self, index: usize) -> Rgba {
        self.stops
            .get(index)
            .map_or(Rgba::new(0.0, 0.0, 0.0, 0.0), |s| srgb(s.color))
    }
}

/// `color` in sRGB.
fn srgb(color: Color) -> Rgba {
    match color.convert(ColorSpaceId::Srgb) {
        Color::Srgb(c) => c,
        _ => Rgba::new(0.0, 0.0, 0.0, 0.0),
    }
}
