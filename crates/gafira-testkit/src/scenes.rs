//! Synthetic scenes with an exact mask for the segmentation gate (ARQ-132).
//!
//! A scene is an opaque `Rgba8` image and the exact mask of its subject
//! (`255` inside, `0` outside). The fixed set has 16 recipes: flat, gradient and
//! textured backgrounds; centred and displaced subjects; convex, concave and
//! holed shapes; and two flat low-contrast scenes (ΔE76 between 6 and 10). The
//! fresh set runs the same generator with another seed over 12 of the recipes.
//!
//! Everything is integer or `+ - * / sqrt` arithmetic and a fixed generator, so
//! a scene is the same on every run and platform. Edges are hard: the exact mask
//! is the shape, with no antialiasing.

use gafira_color::{Lab, LinearRgba, delta_e76};

/// Scenes in the fixed set (the blocking gate).
pub const FIXED_COUNT: usize = 16;
/// Scenes in the fresh set (a warning, D-45).
pub const FRESH_COUNT: usize = 12;
/// The IoU a scene of the fixed set must reach (ARQ-132).
pub const GATE_IOU: f64 = 0.90;
/// Seed of the fixed set.
pub const FIXED_SEED: u64 = 0x6AF1_2A01_D20F_0132;
/// Seed of the fresh set: never used to tune a parameter (D-45).
pub const FRESH_SEED: u64 = 0x51DE_C0DE_9B3A_77E5;

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
    /// Exact mask, one byte per pixel: 255 in the subject, 0 outside.
    pub truth: Vec<u8>,
    /// Flat subject and flat background at ΔE76 between 6 and 10.
    pub low_contrast: bool,
    /// The background is textured.
    pub textured: bool,
}

/// What a recipe says about its scenes, without generating pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Recipe {
    /// Distinct name of the recipe.
    pub name: &'static str,
    /// See [`Scene::low_contrast`].
    pub low_contrast: bool,
    /// See [`Scene::textured`].
    pub textured: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Bg {
    Flat,
    Gradient,
    Textured,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Ellipse,
    RoundRect,
    Crescent,
    Cross,
    Elbow,
    Ring,
    RectRing,
}

struct Def {
    name: &'static str,
    bg: Bg,
    shape: Shape,
    offset: bool,
    low: bool,
}

const fn def(name: &'static str, bg: Bg, shape: Shape, offset: bool, low: bool) -> Def {
    Def {
        name,
        bg,
        shape,
        offset,
        low,
    }
}

const DEFS: [Def; FIXED_COUNT] = [
    def(
        "flat-ellipse-centred",
        Bg::Flat,
        Shape::Ellipse,
        false,
        false,
    ),
    def(
        "flat-roundrect-offset",
        Bg::Flat,
        Shape::RoundRect,
        true,
        false,
    ),
    def("flat-c-centred", Bg::Flat, Shape::Crescent, false, false),
    def("flat-ring-centred", Bg::Flat, Shape::Ring, false, false),
    def(
        "flat-ellipse-low-contrast",
        Bg::Flat,
        Shape::Ellipse,
        false,
        true,
    ),
    def(
        "flat-cross-offset-low-contrast",
        Bg::Flat,
        Shape::Cross,
        true,
        true,
    ),
    def(
        "gradient-ellipse-centred",
        Bg::Gradient,
        Shape::Ellipse,
        false,
        false,
    ),
    def(
        "gradient-roundrect-offset",
        Bg::Gradient,
        Shape::RoundRect,
        true,
        false,
    ),
    def(
        "gradient-c-offset",
        Bg::Gradient,
        Shape::Crescent,
        true,
        false,
    ),
    def(
        "gradient-rectring-centred",
        Bg::Gradient,
        Shape::RectRing,
        false,
        false,
    ),
    def(
        "textured-ellipse-centred",
        Bg::Textured,
        Shape::Ellipse,
        false,
        false,
    ),
    def(
        "textured-roundrect-offset",
        Bg::Textured,
        Shape::RoundRect,
        true,
        false,
    ),
    def(
        "textured-cross-centred",
        Bg::Textured,
        Shape::Cross,
        false,
        false,
    ),
    def(
        "textured-ring-offset",
        Bg::Textured,
        Shape::Ring,
        true,
        false,
    ),
    def(
        "textured-l-centred",
        Bg::Textured,
        Shape::Elbow,
        false,
        false,
    ),
    def(
        "flat-rectring-offset",
        Bg::Flat,
        Shape::RectRing,
        true,
        false,
    ),
];

/// The recipe of fresh scene `index`: 12 of the 16, spread over the list.
fn fresh_def_index(index: usize) -> usize {
    index * FIXED_COUNT / FRESH_COUNT
}

/// Recipe `index` of the fixed set.
///
/// # Panics
/// If `index >= FIXED_COUNT`.
#[must_use]
pub fn fixed_recipe(index: usize) -> Recipe {
    let d = &DEFS[index];
    Recipe {
        name: d.name,
        low_contrast: d.low,
        textured: d.bg == Bg::Textured,
    }
}

/// Scene `index` of the fixed set at `width` x `height`.
///
/// # Panics
/// If `index >= FIXED_COUNT` or a side is 0.
#[must_use]
pub fn fixed_scene(index: usize, width: u32, height: u32) -> Scene {
    make(&DEFS[index], FIXED_SEED, index as u64, width, height)
}

/// Scene `index` of the fresh set at `width` x `height`: the same generator,
/// another seed.
///
/// # Panics
/// If `index >= FRESH_COUNT` or a side is 0.
#[must_use]
pub fn fresh_scene(index: usize, width: u32, height: u32) -> Scene {
    assert!(index < FRESH_COUNT, "fresh scene {index}");
    make(
        &DEFS[fresh_def_index(index)],
        FRESH_SEED,
        index as u64,
        width,
        height,
    )
}

/// The reference input of `segment` (ARQ-112): flat background, centred
/// subject, 1024x768.
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

    /// An integer in `lo..=hi`.
    fn int(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next() % (hi - lo + 1) as u64) as i64
    }

    /// A number in `[lo, hi)` in steps of 1/1024.
    fn frac(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * ((self.next() >> 54) as f64 / 1024.0)
    }

    fn sign(&mut self) -> f64 {
        if self.next() & 1 == 0 { -1.0 } else { 1.0 }
    }
}

fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Where the shape is, in pixels.
struct Geometry {
    shape: Shape,
    cx: f64,
    cy: f64,
    r: f64,
}

impl Geometry {
    fn new(d: &Def, rng: &mut Rng, w: f64, h: f64) -> Self {
        let s = w.min(h);
        let r = if d.offset {
            s * rng.frac(0.15, 0.20)
        } else {
            s * rng.frac(0.17, 0.26)
        };
        let (mut cx, mut cy) = (w / 2.0, h / 2.0);
        if d.offset {
            cx += w * rng.frac(0.17, 0.22) * rng.sign();
            cy += h * rng.frac(0.12, 0.18) * rng.sign();
        }
        // Whatever the aspect, the shape stays inside the image.
        let reach = 1.35 * r;
        cx = cx.clamp(reach + 2.0, (w - reach - 2.0).max(reach + 2.0));
        cy = cy.clamp(reach + 2.0, (h - reach - 2.0).max(reach + 2.0));
        Self {
            shape: d.shape,
            cx,
            cy,
            r,
        }
    }

    /// True when the pixel centre `(px, py)` is in the subject.
    fn contains(&self, px: f64, py: f64) -> bool {
        let (dx, dy, r) = (px - self.cx, py - self.cy, self.r);
        let ellipse = |a: f64, b: f64| (dx / a) * (dx / a) + (dy / b) * (dy / b) <= 1.0;
        let rect = |a: f64, b: f64| dx.abs() <= a && dy.abs() <= b;
        match self.shape {
            Shape::Ellipse => ellipse(1.25 * r, r),
            Shape::RoundRect => {
                let (a, b, c) = (1.3 * r, 0.9 * r, 0.3 * r);
                let (qx, qy) = ((dx.abs() - (a - c)).max(0.0), (dy.abs() - (b - c)).max(0.0));
                dx.abs() <= a && dy.abs() <= b && (qx * qx + qy * qy).sqrt() <= c
            }
            Shape::Crescent => {
                let outer = 1.2 * r;
                let d2 = dx * dx + dy * dy;
                let in_ring = d2 <= outer * outer && d2 >= (0.55 * outer) * (0.55 * outer);
                in_ring && !(dx > 0.0 && dy.abs() < 0.28 * outer)
            }
            Shape::Cross => rect(1.3 * r, 0.42 * r) || rect(0.42 * r, 1.3 * r),
            Shape::Elbow => {
                let bar = dx >= -1.2 * r && dx <= -0.3 * r && dy.abs() <= 1.2 * r;
                let foot = dx >= -1.2 * r && dx <= 1.2 * r && dy >= 0.3 * r && dy <= 1.2 * r;
                bar || foot
            }
            Shape::Ring => ellipse(1.3 * r, r) && !ellipse(0.65 * r, 0.5 * r),
            Shape::RectRing => rect(1.3 * r, 0.95 * r) && !rect(0.72 * r, 0.52 * r),
        }
    }
}

type Rgb = [u8; 3];

/// How the background colours a pixel.
enum Paint {
    Flat(Rgb),
    Gradient(Rgb, Rgb),
    Textured {
        base: Rgb,
        amp: i64,
        cell: u32,
        seed: u64,
    },
}

impl Paint {
    fn at(&self, x: u32, y: u32, w: u32, h: u32) -> Rgb {
        match *self {
            Paint::Flat(c) => c,
            Paint::Gradient(c0, c1) => {
                // t in [0, 1024), the mean of x/w and y/h, exact in integers.
                let t = ((2 * u64::from(x) + 1) * u64::from(h)
                    + (2 * u64::from(y) + 1) * u64::from(w))
                    * 1024
                    / (4 * u64::from(w) * u64::from(h));
                [0, 1, 2].map(|i| {
                    ((u64::from(c0[i]) * (1024 - t) + u64::from(c1[i]) * t + 512) / 1024) as u8
                })
            }
            Paint::Textured {
                base,
                amp,
                cell,
                seed,
            } => {
                let key = seed ^ (u64::from(x / cell) << 32) ^ u64::from(y / cell);
                let h0 = mix(key);
                let lum = (h0 % (2 * amp as u64 + 1)) as i64 - amp;
                [0, 1, 2].map(|i| {
                    let chroma =
                        (mix(h0 ^ (i as u64 + 1)) % (2 * (amp as u64 / 3) + 1)) as i64 - amp / 3;
                    (i64::from(base[i]) + lum + chroma).clamp(0, 255) as u8
                })
            }
        }
    }
}

fn srgb_to_linear(byte: u8) -> f64 {
    let c = f64::from(byte) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
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

fn lab_of(c: Rgb) -> Lab {
    lab_of_linear(c.map(srgb_to_linear))
}

fn make(d: &Def, seed: u64, index: u64, width: u32, height: u32) -> Scene {
    assert!(width > 0 && height > 0, "an empty scene");
    let (w, h) = (width as usize, height as usize);
    let mut rng = Rng::new(seed, index);
    let geo = Geometry::new(d, &mut rng, f64::from(width), f64::from(height));
    let mut truth = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            if geo.contains(x as f64 + 0.5, y as f64 + 0.5) {
                truth[y * w + x] = 255;
            }
        }
    }
    let side = f64::from(width.min(height));
    let paint = match d.bg {
        Bg::Flat => Paint::Flat(random_colour(&mut rng, 70, 200)),
        Bg::Gradient => {
            let c0 = random_colour(&mut rng, 70, 190);
            let c1 = [0, 1, 2].map(|i| {
                (i64::from(c0[i]) + rng.int(25, 50) * if rng.next() & 1 == 0 { -1 } else { 1 })
                    .clamp(0, 255) as u8
            });
            Paint::Gradient(c0, c1)
        }
        Bg::Textured => Paint::Textured {
            base: random_colour(&mut rng, 80, 180),
            amp: rng.int(10, 22),
            cell: ((side / 250.0) as u32).max(3),
            seed: rng.next(),
        },
    };
    let mut rgba = vec![255u8; w * h * 4];
    let mut sum = [0.0f64; 3];
    let mut count = 0u64;
    // Linear value of every byte, so the mean pass does no `powf`.
    let lut: Vec<f64> = (0..=255u8).map(srgb_to_linear).collect();
    for y in 0..h {
        for x in 0..w {
            let c = paint.at(x as u32, y as u32, width, height);
            let o = (y * w + x) * 4;
            rgba[o..o + 3].copy_from_slice(&c);
            if truth[y * w + x] == 0 {
                for i in 0..3 {
                    sum[i] += lut[usize::from(c[i])];
                }
                count += 1;
            }
        }
    }
    let mean = sum.map(|v| v / count.max(1) as f64);
    let subject = pick_subject(d, &paint, mean, &mut rng);
    for (i, t) in truth.iter().enumerate() {
        if *t == 255 {
            rgba[i * 4..i * 4 + 3].copy_from_slice(&subject);
        }
    }
    Scene {
        name: d.name,
        width,
        height,
        rgba,
        truth,
        low_contrast: d.low,
        textured: d.bg == Bg::Textured,
    }
}

fn random_colour(rng: &mut Rng, lo: i64, hi: i64) -> Rgb {
    [0, 1, 2].map(|_| rng.int(lo, hi) as u8)
}

/// A flat subject colour whose distance to the background's exact mean is in
/// the recipe's range (found by sampling, so the constraint holds by
/// construction).
fn pick_subject(d: &Def, paint: &Paint, mean: [f64; 3], rng: &mut Rng) -> Rgb {
    let mean_lab = lab_of_linear(mean);
    let (lo, hi, apart) = match (d.low, d.bg) {
        (true, _) => (6.6, 9.4, 0.0),
        (false, Bg::Flat) => (35.0, 80.0, 0.0),
        (false, Bg::Gradient) => (35.0, 80.0, 25.0),
        (false, Bg::Textured) => (22.0, 55.0, 0.0),
    };
    // A low-contrast subject stays close to the background colour.
    let near = match paint {
        Paint::Flat(c) if d.low => Some(*c),
        _ => None,
    };
    let ends: Vec<Rgb> = match paint {
        Paint::Gradient(a, b) => vec![*a, *b],
        _ => Vec::new(),
    };
    for _ in 0..200_000 {
        let c = match near {
            Some(b) => [0, 1, 2].map(|i| (i64::from(b[i]) + rng.int(-14, 14)).clamp(0, 255) as u8),
            None => random_colour(rng, 15, 240),
        };
        let de = delta_e76(lab_of(c), mean_lab);
        if (lo..=hi).contains(&de)
            && ends
                .iter()
                .all(|&e| delta_e76(lab_of(c), lab_of(e)) >= apart)
        {
            return c;
        }
    }
    unreachable!("no subject colour for {}", d.name)
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
