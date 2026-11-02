//! Synthetic scenes with an exact mask for the segmentation gate (ARQ-132).
//!
//! A scene is an opaque `Rgba8` image and the exact mask of its subject (`255`
//! inside, `0` outside). The fixed set is the table of 16 scenes in 02 §4.17, the
//! XP5 definitions: hard edges, gradients and textures at ΔE76 25. The fresh set
//! is 12 other recipes over the whole range of ARQ-132: gradients and textures
//! down to ΔE76 7, subjects from 2 % to 50 %, hard and soft edges. Its colours,
//! gradients, textures and placement come from another seed; it is never used to
//! tune a parameter (D-45).
//!
//! Measures are in units of the image height, so a scene is the same picture at
//! every size. A fixed generator makes a scene the same on every run. The
//! geometry and the texture use only `+ - * / sqrt` and table-free polynomials;
//! the colour conversions are those of the rest of the crate.

use gafira_color::{Lab, LinearRgba, delta_e76};

/// Scenes in the fixed set (the blocking gate).
pub const FIXED_COUNT: usize = 16;
/// Scenes in the fresh set (a warning, D-45).
pub const FRESH_COUNT: usize = 12;
/// The IoU a scene of the fixed set must reach (ARQ-132).
pub const GATE_IOU: f64 = 0.90;
/// Seed of the noise of the fixed set.
pub const FIXED_SEED: u64 = 0x6AF1_2A01_D20F_0132;
/// Seed of the fresh set: never used to tune a parameter (D-45).
pub const FRESH_SEED: u64 = 0x51DE_C0DE_9B3A_77E5;

/// Width of the ramp of a soft edge, in pixels (ARQ-132).
const RAMP: f64 = 16.0;
/// Lab of the fixed set's background and start of its subject (02 §4.17).
const BASE: [f64; 3] = [55.0, 8.0, -12.0];
/// Direction of the subject's colour in Lab.
const DIRECTION: [f64; 3] = [0.6, 0.5, -0.6];
/// Direction of scene 14, a change of hue only.
const HUE_ONLY: [f64; 3] = [0.0, 0.7, 0.7];
/// The side of the render the subject colour is calibrated on: the same
/// picture at any size, cheap enough to run for every scene.
const CALIBRATION: (u32, u32) = (640, 480);

/// One scene: an opaque `Rgba8` image and the exact mask of its subject.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scene {
    /// The recipe's name.
    pub name: &'static str,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `Rgba8`, row-major, alpha 255.
    pub rgba: Vec<u8>,
    /// Exact mask, one byte per pixel: 255 in the subject, 0 outside. On a soft
    /// edge, the pixels with at least half the subject's colour.
    pub truth: Vec<u8>,
}

/// The background of a recipe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Background {
    /// One colour.
    Flat,
    /// A smooth change of `L` and `a` across the image.
    Gradient,
    /// A wave of `L` and per-pixel noise on each channel.
    Textured,
}

/// Where the subject sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    /// Near the middle of the image.
    Centred,
    /// Away from it.
    Offset,
}

/// A piece of a shape, in units of the image height (width 4/3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Part {
    /// `(cx, cy, r)`.
    Disk(f64, f64, f64),
    /// `(cx, cy, rx, ry)`.
    Ellipse(f64, f64, f64, f64),
    /// `(x0, x1, y0, y1)`.
    Rect(f64, f64, f64, f64),
    /// `(cx, cy, half width, half height, corner radius)`.
    RoundRect(f64, f64, f64, f64, f64),
    /// Five points, `r <= r0 (1 + 0.35 cos 5θ)`: `(cx, cy, r0)`.
    Star(f64, f64, f64),
    /// `(cx, cy, rx, ry)`, the corners on the axes.
    Diamond(f64, f64, f64, f64),
}

/// What a recipe says about its scenes, without generating pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Recipe {
    /// Distinct name of the recipe.
    pub name: &'static str,
    /// Kind of background.
    pub background: Background,
    /// Position of the subject.
    pub position: Position,
    /// The subject is the union of these parts ...
    pub add: &'static [Part],
    /// ... less the union of these. A fresh scene moves and scales the parts a
    /// little with its seed.
    pub cut: &'static [Part],
    /// Target ΔE76 between the exact mean colours.
    pub target_delta_e: f64,
    /// Flat subject and flat background at ΔE76 between 6 and 10.
    pub low_contrast: bool,
    /// The shape is concave.
    pub concave: bool,
    /// The shape has a hole.
    pub holed: bool,
    /// A ramp of 16 px between subject and background.
    pub soft_edge: bool,
}

use Background::{Flat, Gradient, Textured};
use Part::{Diamond, Disk, Ellipse, Rect, RoundRect, Star};
use Position::{Centred, Offset};

#[derive(Clone, Copy)]
enum Class {
    Convex,
    Concave,
    Holed,
}

/// A recipe from the columns of its table: name, background and position; add,
/// cut and class; target, low contrast and soft edge.
const fn row(
    (name, background, position): (&'static str, Background, Position),
    (add, cut, class): (&'static [Part], &'static [Part], Class),
    (target_delta_e, low_contrast, soft_edge): (f64, bool, bool),
) -> Recipe {
    Recipe {
        name,
        background,
        position,
        add,
        cut,
        target_delta_e,
        low_contrast,
        concave: matches!(class, Class::Concave),
        holed: matches!(class, Class::Holed),
        soft_edge,
    }
}

/// The 16 rows of the table in 02 §4.17, in order: scene 01 is index 0.
const FIXED: [Recipe; FIXED_COUNT] = [
    row(
        ("01-flat-ellipse", Flat, Centred),
        (ELLIPSE, &[], Class::Convex),
        (40.0, false, false),
    ),
    row(
        ("02-flat-roundrect", Flat, Offset),
        (
            &[RoundRect(0.35, 0.62, 0.2, 0.15, 0.05)],
            &[],
            Class::Convex,
        ),
        (30.0, false, false),
    ),
    row(
        ("03-gradient-disk", Gradient, Centred),
        (&[Disk(0.667, 0.5, 0.32)], &[], Class::Convex),
        (25.0, false, false),
    ),
    row(
        ("04-gradient-star", Gradient, Offset),
        (&[Star(0.95, 0.4, 0.22)], &[], Class::Concave),
        (25.0, false, false),
    ),
    row(
        ("05-texture-disk", Textured, Centred),
        (&[Disk(0.667, 0.5, 0.28)], &[], Class::Convex),
        (25.0, false, false),
    ),
    row(
        ("06-texture-ellipse", Textured, Offset),
        (&[Ellipse(1.0, 0.6, 0.25, 0.17)], &[], Class::Convex),
        (25.0, false, false),
    ),
    row(
        ("07-flat-crescent", Flat, Centred),
        (CRESCENT, BITE, Class::Concave),
        (30.0, false, false),
    ),
    row(
        ("08-gradient-l", Gradient, Centred),
        (
            &[Rect(0.35, 0.62, 0.2, 0.8), Rect(0.35, 1.0, 0.58, 0.8)],
            &[],
            Class::Concave,
        ),
        (25.0, false, false),
    ),
    row(
        ("09-flat-ring", Flat, Centred),
        (
            &[Disk(0.667, 0.5, 0.34)],
            &[Disk(0.667, 0.5, 0.17)],
            Class::Holed,
        ),
        (30.0, false, false),
    ),
    row(
        ("10-texture-two-holes", Textured, Centred),
        (
            &[Rect(0.3, 1.0, 0.2, 0.8)],
            &[Disk(0.5, 0.5, 0.1), Disk(0.8, 0.5, 0.1)],
            Class::Holed,
        ),
        (25.0, false, false),
    ),
    row(
        ("11-low-ellipse", Flat, Centred),
        (ELLIPSE, &[], Class::Convex),
        (6.0, true, false),
    ),
    row(
        ("12-low-star", Flat, Offset),
        (&[Star(0.9, 0.55, 0.22)], &[], Class::Concave),
        (8.0, true, false),
    ),
    row(
        ("13-flat-ring-offset", Flat, Offset),
        (
            &[Disk(0.5, 0.5, 0.26)],
            &[Disk(0.5, 0.5, 0.13)],
            Class::Holed,
        ),
        (10.0, false, false),
    ),
    row(
        ("14-low-crescent-hue", Flat, Centred),
        (CRESCENT, BITE, Class::Concave),
        (6.0, true, false),
    ),
    row(
        ("15-flat-small-disk", Flat, Centred),
        (&[Disk(0.667, 0.5, 0.10)], &[], Class::Convex),
        (30.0, false, false),
    ),
    row(
        ("16-flat-large-ellipse", Flat, Centred),
        (&[Ellipse(0.667, 0.5, 0.53, 0.36)], &[], Class::Convex),
        (30.0, false, false),
    ),
];

const ELLIPSE: &[Part] = &[Ellipse(0.667, 0.5, 0.34, 0.25)];
const CRESCENT: &[Part] = &[Disk(0.667, 0.5, 0.33)];
const BITE: &[Part] = &[Disk(0.8, 0.5, 0.27)];

/// The 12 recipes of the fresh set: other shapes, the whole range of ARQ-132.
const FRESH: [Recipe; FRESH_COUNT] = [
    row(
        ("fresh-cross-soft", Flat, Offset),
        (
            &[Rect(0.75, 1.15, 0.40, 0.60), Rect(0.85, 1.05, 0.25, 0.75)],
            &[],
            Class::Concave,
        ),
        (18.0, false, true),
    ),
    row(
        ("fresh-gradient-capsule", Gradient, Centred),
        (
            &[RoundRect(0.667, 0.5, 0.4, 0.15, 0.15)],
            &[],
            Class::Convex,
        ),
        (7.4, false, false),
    ),
    row(
        ("fresh-texture-frame-soft", Textured, Offset),
        (
            &[Rect(0.35, 0.95, 0.25, 0.85)],
            &[Rect(0.5, 0.8, 0.4, 0.7)],
            Class::Holed,
        ),
        (7.4, false, true),
    ),
    row(
        ("fresh-small-diamond", Flat, Offset),
        (&[Diamond(1.05, 0.3, 0.13, 0.13)], &[], Class::Convex),
        (14.0, false, false),
    ),
    row(
        ("fresh-large-rect-soft", Flat, Centred),
        (&[RoundRect(0.66, 0.5, 0.5, 0.31, 0.03)], &[], Class::Convex),
        (20.0, false, true),
    ),
    row(
        ("fresh-gradient-horseshoe-soft", Gradient, Centred),
        (
            &[Disk(0.667, 0.5, 0.33)],
            &[Disk(0.667, 0.5, 0.18), Rect(0.60, 0.74, 0.5, 0.9)],
            Class::Concave,
        ),
        (11.0, false, true),
    ),
    row(
        ("fresh-texture-large-cross", Textured, Centred),
        (
            &[Rect(0.2, 1.13, 0.33, 0.67), Rect(0.5, 0.83, 0.07, 0.93)],
            &[],
            Class::Concave,
        ),
        (10.0, false, false),
    ),
    row(
        ("fresh-low-frame", Flat, Offset),
        (
            &[Rect(0.72, 1.2, 0.2, 0.8)],
            &[Rect(0.86, 1.06, 0.4, 0.6)],
            Class::Holed,
        ),
        (7.0, true, false),
    ),
    row(
        ("fresh-gradient-diamond-soft", Gradient, Offset),
        (&[Diamond(0.4, 0.62, 0.17, 0.17)], &[], Class::Convex),
        (9.0, false, true),
    ),
    row(
        ("fresh-texture-keyhole-soft", Textured, Offset),
        (
            &[Disk(0.95, 0.42, 0.22), Rect(0.88, 1.02, 0.42, 0.85)],
            &[],
            Class::Concave,
        ),
        (14.0, false, true),
    ),
    row(
        ("fresh-low-horseshoe", Flat, Offset),
        (
            &[Disk(0.5, 0.55, 0.3)],
            &[Disk(0.5, 0.55, 0.16), Rect(0.44, 0.56, 0.1, 0.55)],
            Class::Concave,
        ),
        (9.0, true, false),
    ),
    row(
        ("fresh-gradient-large-frame", Gradient, Centred),
        (
            &[Rect(0.3, 1.1, 0.12, 0.88)],
            &[Rect(0.55, 0.85, 0.35, 0.65)],
            Class::Holed,
        ),
        (16.0, false, false),
    ),
];

/// Recipe `index` of the fixed set; index 0 is scene 01 of the table.
///
/// # Panics
/// If `index >= FIXED_COUNT`.
#[must_use]
pub fn fixed_recipe(index: usize) -> Recipe {
    FIXED[index]
}

/// Recipe `index` of the fresh set.
///
/// # Panics
/// If `index >= FRESH_COUNT`.
#[must_use]
pub fn fresh_recipe(index: usize) -> Recipe {
    FRESH[index]
}

/// Scene `index` of the fixed set at `width` x `height`.
///
/// # Panics
/// If `index >= FIXED_COUNT` or a side is 0.
#[must_use]
pub fn fixed_scene(index: usize, width: u32, height: u32) -> Scene {
    let recipe = &FIXED[index];
    let direction = if index == 13 { HUE_ONLY } else { DIRECTION };
    make(recipe, &Look::fixed(direction, index as u64), width, height)
}

/// Scene `index` of the fresh set at `width` x `height`: another recipe, and its
/// colours, gradient, texture and placement drawn from the fresh seed.
///
/// # Panics
/// If `index >= FRESH_COUNT` or a side is 0.
#[must_use]
pub fn fresh_scene(index: usize, width: u32, height: u32) -> Scene {
    make(&FRESH[index], &Look::fresh(index as u64), width, height)
}

/// The reference input of `segment` (ARQ-112): scene 01, 1024x768.
#[must_use]
pub fn reference_scene() -> Scene {
    fixed_scene(0, 1024, 768)
}

/// A fixed generator (SplitMix64): the same seed gives the same stream.
struct Rng(u64);

impl Rng {
    fn new(seed: u64, index: u64) -> Self {
        Self(seed ^ index.wrapping_add(1).wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix(self.0)
    }

    /// A number in `[lo, hi)` in steps of 1/1024.
    fn frac(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * ((self.next() >> 54) as f64 / 1024.0)
    }
}

fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Everything about a scene that its recipe leaves open.
struct Look {
    /// Lab of the flat background and of the subject before its displacement.
    base: [f64; 3],
    direction: [f64; 3],
    /// Amplitudes of the gradient on `L` and on `a`.
    gradient: [f64; 2],
    /// `L + amp sin(fu u) sin(fv v)`: `[amp, fu, fv]`.
    wave: [f64; 3],
    /// Per-pixel noise on each 8-bit channel, plus or minus this.
    noise: f64,
    noise_seed: u64,
    /// Parts moved by `[dx, dy]` and scaled by `k` around the image centre.
    shift: [f64; 2],
    scale: f64,
}

impl Look {
    /// The fixed set: the constants of 02 §4.17.
    fn fixed(direction: [f64; 3], index: u64) -> Self {
        Self {
            base: BASE,
            direction,
            gradient: [20.0, 8.0],
            wave: [6.0, 60.0, 50.0],
            noise: 10.0,
            noise_seed: FIXED_SEED ^ index,
            shift: [0.0, 0.0],
            scale: 1.0,
        }
    }

    /// A fresh scene: all of it from the fresh seed.
    fn fresh(index: u64) -> Self {
        let mut rng = Rng::new(FRESH_SEED, index);
        let base = [
            rng.frac(42.0, 68.0),
            rng.frac(-14.0, 18.0),
            rng.frac(-20.0, 20.0),
        ];
        let d = [
            rng.frac(-1.0, 1.0),
            rng.frac(-1.0, 1.0),
            rng.frac(-1.0, 1.0),
        ];
        let norm = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(0.1);
        Self {
            base,
            direction: d.map(|v| v / norm),
            gradient: [rng.frac(14.0, 30.0), rng.frac(-12.0, 12.0)],
            wave: [
                rng.frac(3.0, 10.0),
                rng.frac(24.0, 90.0),
                rng.frac(24.0, 90.0),
            ],
            noise: rng.frac(6.0, 14.0),
            noise_seed: rng.next(),
            shift: [rng.frac(-0.02, 0.02), rng.frac(-0.02, 0.02)],
            scale: rng.frac(0.98, 1.02),
        }
    }
}

/// Sine by a polynomial, the same on every platform.
fn sine(x: f64) -> f64 {
    let pi = std::f64::consts::PI;
    let mut t = x - (x / (2.0 * pi)).round() * 2.0 * pi;
    if t > pi / 2.0 {
        t = pi - t;
    } else if t < -pi / 2.0 {
        t = -pi - t;
    }
    let t2 = t * t;
    t * (1.0
        - t2 / 6.0 * (1.0 - t2 / 20.0 * (1.0 - t2 / 42.0 * (1.0 - t2 / 72.0 * (1.0 - t2 / 110.0)))))
}

impl Part {
    fn contains(self, x: f64, y: f64) -> bool {
        match self {
            Disk(cx, cy, r) => (x - cx) * (x - cx) + (y - cy) * (y - cy) <= r * r,
            Ellipse(cx, cy, rx, ry) => {
                let (a, b) = ((x - cx) / rx, (y - cy) / ry);
                a * a + b * b <= 1.0
            }
            Rect(x0, x1, y0, y1) => x >= x0 && x <= x1 && y >= y0 && y <= y1,
            RoundRect(cx, cy, hw, hh, c) => {
                let (qx, qy) = ((x - cx).abs() - (hw - c), (y - cy).abs() - (hh - c));
                let (ox, oy) = (qx.max(0.0), qy.max(0.0));
                (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) <= c
            }
            Star(cx, cy, r0) => {
                let (dx, dy) = (x - cx, y - cy);
                let r = (dx * dx + dy * dy).sqrt();
                if r == 0.0 {
                    return true;
                }
                // cos 5θ from cos θ, the Chebyshev polynomial T5.
                let c = dx / r;
                let c2 = c * c;
                r <= r0 * (1.0 + 0.35 * c * (16.0 * c2 * c2 - 20.0 * c2 + 5.0))
            }
            Diamond(cx, cy, rx, ry) => (x - cx).abs() / rx + (y - cy).abs() / ry <= 1.0,
        }
    }
}

impl Recipe {
    fn contains(&self, x: f64, y: f64) -> bool {
        self.add.iter().any(|p| p.contains(x, y)) && !self.cut.iter().any(|p| p.contains(x, y))
    }
}

/// Lab to linear sRGB, unclamped.
fn lab_to_linear(lab: [f64; 3]) -> [f64; 3] {
    let fy = (lab[0] + 16.0) / 116.0;
    let (fx, fz) = (fy + lab[1] / 500.0, fy - lab[2] / 200.0);
    let inverse = |t: f64| {
        if t > 0.206_897 {
            t * t * t
        } else {
            (t - 16.0 / 116.0) / 7.787
        }
    };
    let (x, y, z) = (inverse(fx) * 0.95047, inverse(fy), inverse(fz) * 1.08883);
    [
        3.240_454_2 * x - 1.537_138_5 * y - 0.498_531_4 * z,
        -0.969_266 * x + 1.876_010_8 * y + 0.041_556 * z,
        0.055_643_4 * x - 0.204_025_9 * y + 1.057_225_2 * z,
    ]
}

/// The linear value at which each 8-bit level starts: encoding is a search.
struct Encoder([f64; 255]);

impl Encoder {
    fn new() -> Self {
        Self(std::array::from_fn(|i| {
            srgb_to_linear_value((i as f64 + 0.5) / 255.0)
        }))
    }

    fn byte(&self, linear: f64) -> u8 {
        self.0.partition_point(|t| *t <= linear) as u8
    }

    fn colour(&self, lab: [f64; 3]) -> Rgb {
        lab_to_linear(lab).map(|c| self.byte(c))
    }
}

/// For each pixel, the weight of the subject's colour on a soft edge: a linear
/// ramp of [`RAMP`] pixels over the distance to the shape's outline. A pixel of
/// the subject is above one half and one outside it below.
fn ramp_weights(truth: &[u8], w: usize, h: usize) -> Vec<f32> {
    const REACH: usize = 9;
    // Distance along the row to the nearest pixel of each side, capped.
    let mut row_distance = [vec![REACH as u8; w * h], vec![REACH as u8; w * h]];
    for y in 0..h {
        let row = &truth[y * w..(y + 1) * w];
        for (side, near) in row_distance.iter_mut().enumerate() {
            let mine = |x: usize| usize::from(row[x] == 255) == side;
            let near = &mut near[y * w..(y + 1) * w];
            let mut d = REACH;
            for (x, slot) in near.iter_mut().enumerate() {
                d = if mine(x) { 0 } else { (d + 1).min(REACH) };
                *slot = d as u8;
            }
            d = REACH;
            for (x, slot) in near.iter_mut().enumerate().rev() {
                d = if mine(x) { 0 } else { (d + 1).min(REACH) };
                *slot = (*slot).min(d as u8);
            }
        }
    }
    let mut out = vec![0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let inside = truth[i] == 255;
            let other = &row_distance[usize::from(!inside)];
            let mut best = (REACH * REACH) as u64;
            for dy in y.saturating_sub(REACH)..(y + REACH + 1).min(h) {
                let (a, b) = (dy.abs_diff(y) as u64, u64::from(other[dy * w + x]));
                best = best.min(a * a + b * b);
            }
            let distance = (best as f64).sqrt() - 0.5;
            let t = if inside {
                0.5 + distance / RAMP
            } else {
                0.5 - distance / RAMP
            };
            out[i] = t.clamp(0.0, 1.0) as f32;
        }
    }
    out
}

/// The picture, with the subject at `s` from the base along its direction.
fn render(r: &Recipe, look: &Look, (width, height): (u32, u32), s: f64) -> (Vec<u8>, Vec<u8>) {
    let (w, h) = (width as usize, height as usize);
    let unit = f64::from(height);
    let aspect = f64::from(width) / unit;
    let us: Vec<f64> = (0..w).map(|x| (x as f64 + 0.5) / unit).collect();
    let vs: Vec<f64> = (0..h).map(|y| (y as f64 + 0.5) / unit).collect();
    let [wave, fu, fv] = look.wave;
    let wave_u: Vec<f64> = us.iter().map(|u| sine(fu * u)).collect();
    let wave_v: Vec<f64> = vs.iter().map(|v| sine(fv * v)).collect();
    let [dx, dy] = look.shift;
    let (mid_x, mid_y) = (0.667, 0.5);
    let mut truth = vec![0u8; w * h];
    for (y, row) in truth.chunks_mut(w).enumerate() {
        let v = (vs[y] - dy - mid_y) / look.scale + mid_y;
        for (x, t) in row.iter_mut().enumerate() {
            let u = (us[x] - dx - mid_x) / look.scale + mid_x;
            if r.contains(u, v) {
                *t = 255;
            }
        }
    }
    let encoder = Encoder::new();
    let subject = encoder.colour([0, 1, 2].map(|k| look.base[k] + s * look.direction[k]));
    let flat = encoder.colour(look.base);
    let ramp = r.soft_edge.then(|| ramp_weights(&truth, w, h));
    let mut rgba = vec![255u8; w * h * 4];
    for (y, row) in rgba.chunks_mut(w * 4).enumerate() {
        for (x, px) in row.chunks_mut(4).enumerate() {
            let i = y * w + x;
            let mut background = [0.0; 3];
            match r.background {
                Flat => background = flat.map(f64::from),
                Gradient => {
                    let mut lab = look.base;
                    lab[0] += look.gradient[0] * (us[x] / aspect - 0.5);
                    lab[1] += look.gradient[1] * (vs[y] - 0.5);
                    background = encoder.colour(lab).map(f64::from);
                }
                Textured => {
                    let mut lab = look.base;
                    lab[0] += wave * wave_u[x] * wave_v[y];
                    let c = encoder.colour(lab);
                    for k in 0..3 {
                        let key = look.noise_seed
                            ^ (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                            ^ (y as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
                            ^ (k as u64 + 1).wrapping_mul(0x94D0_49BB_1331_11EB);
                        let n = (mix(key) >> 11) as f64 / (1u64 << 53) as f64;
                        background[k] = f64::from(c[k]) + (n - 0.5) * 2.0 * look.noise;
                    }
                }
            }
            let t = ramp
                .as_ref()
                .map_or(f64::from(u8::from(truth[i] == 255)), |r| f64::from(r[i]));
            for k in 0..3 {
                let c = background[k] * (1.0 - t) + f64::from(subject[k]) * t;
                px[k] = c.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    (rgba, truth)
}

/// The displacement that gives the recipe's target ΔE76 or just above, measured
/// on a small hard-edged render: the mean colours do not depend on the size.
fn calibrate(r: &Recipe, look: &Look) -> f64 {
    let (w, h) = CALIBRATION;
    let hard = Recipe {
        soft_edge: false,
        ..*r
    };
    // A flat scene is exact at any size; the others keep a small margin.
    let margin = if r.background == Flat { 0.02 } else { 0.1 };
    let goal = r.target_delta_e + margin;
    let measure = |s: f64| {
        let (rgba, truth) = render(&hard, look, (w, h), s);
        let scene = Scene {
            name: r.name,
            width: w,
            height: h,
            rgba,
            truth,
        };
        scene.mean_delta_e76()
    };
    let mut s = r.target_delta_e;
    for _ in 0..14 {
        s *= goal / measure(s).max(0.01);
    }
    while measure(s) < goal {
        s *= 1.01;
    }
    s
}

fn make(r: &Recipe, look: &Look, width: u32, height: u32) -> Scene {
    assert!(width > 0 && height > 0, "an empty scene");
    let s = calibrate(r, look);
    let (rgba, truth) = render(r, look, (width, height), s);
    Scene {
        name: r.name,
        width,
        height,
        rgba,
        truth,
    }
}

type Rgb = [u8; 3];

fn srgb_to_linear_value(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn srgb_to_linear(byte: u8) -> f64 {
    srgb_to_linear_value(f64::from(byte) / 255.0)
}

/// Lab of a linear-light colour given as `f64` channels.
fn lab_of_linear(rgb: [f64; 3]) -> Lab {
    Lab::from_linear_srgb(LinearRgba::new(
        rgb[0] as f32,
        rgb[1] as f32,
        rgb[2] as f32,
        1.0,
    ))
}

impl Scene {
    /// Share of the image the subject covers.
    #[must_use]
    pub fn area_fraction(&self) -> f64 {
        let inside = self.truth.iter().filter(|&&t| t == 255).count();
        inside as f64 / self.truth.len() as f64
    }

    /// ΔE76 between the exact mean colours of subject and background, both
    /// averaged in linear light (D-44).
    #[must_use]
    pub fn mean_delta_e76(&self) -> f64 {
        let lut: Vec<f64> = (0..=255u8).map(srgb_to_linear).collect();
        let mut sums = [[0.0f64; 3]; 2];
        let mut counts = [0u64; 2];
        for (p, t) in self.rgba.chunks(4).zip(&self.truth) {
            let g = usize::from(*t == 255);
            for i in 0..3 {
                sums[g][i] += lut[usize::from(p[i])];
            }
            counts[g] += 1;
        }
        let mean = |g: usize| sums[g].map(|v| v / counts[g].max(1) as f64);
        delta_e76(lab_of_linear(mean(1)), lab_of_linear(mean(0)))
    }
}

/// Counts of a mask against the exact one, binarised at alpha >= 128.
fn overlap(mask: &[u8], truth: &[u8]) -> (u64, u64) {
    let (mut inter, mut union) = (0u64, 0u64);
    for (m, t) in mask.iter().zip(truth) {
        let (m, t) = (*m >= 128, *t >= 128);
        inter += u64::from(m && t);
        union += u64::from(m || t);
    }
    (inter, union)
}

/// Intersection over union of two masks binarised at alpha >= 128; with both
/// empty it is 1 (ARQ-132).
#[must_use]
pub fn iou(mask: &[u8], truth: &[u8]) -> f64 {
    let (inter, union) = overlap(mask, truth);
    if union == 0 {
        1.0
    } else {
        inter as f64 / union as f64
    }
}

/// True when the IoU reaches [`GATE_IOU`], equality included (ARQ-132). It is
/// computed in integers: `inter / union >= 9 / 10`.
#[must_use]
pub fn meets_gate(mask: &[u8], truth: &[u8]) -> bool {
    let (inter, union) = overlap(mask, truth);
    inter * 10 >= union * 9
}

/// The warning on the fresh set (D-45): how many scenes reach the gate IoU.
#[derive(Clone, Debug, PartialEq)]
pub struct FreshReport {
    /// Scenes at or above the gate IoU.
    pub passed: usize,
    /// Scenes run.
    pub total: usize,
    /// The IoU of each scene, in order.
    pub ious: Vec<f64>,
}

/// Counts the IoUs that reach the gate. It never fails the run: `0/12` is a
/// valid report (ARQ-132).
#[must_use]
pub fn fresh_report(ious: &[f64]) -> FreshReport {
    FreshReport {
        passed: ious.iter().filter(|&&v| v >= GATE_IOU).count(),
        total: ious.len(),
        ious: ious.to_vec(),
    }
}
