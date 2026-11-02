//! Textura: Nitidez, Claridad and Viñeta (FD-33).

use gafira_doc::contract::{Describe, Descriptor, ParamSchema, ParamValues};
use gafira_doc::msg::MessageKey;
use gafira_geom::IRect;

use super::blur::{blur_shifted, half_width};
use super::pointwise::{
    check_io, check_scale, check_tile, map_straight, slider, smoothstep, window_of,
};
use super::{Cost, Effect};
use crate::transfer::Transfer;
use crate::{EffectCx, EffectError, ImageView, ImageViewMut, PixelData, unpremultiply_u8};

/// The texture group, in linear light. The blurs read the input image, not each other's output.
#[derive(Debug)]
pub struct Texture {
    descriptor: Descriptor,
    schema: ParamSchema,
}

impl Texture {
    /// Builds the effect.
    #[must_use]
    pub fn new() -> Self {
        Self {
            descriptor: Descriptor::new(
                "effect.adjust.texture",
                MessageKey("glossary.effect_texture"),
                1,
            ),
            schema: ParamSchema::new(vec![
                slider("sharpness", "effect.adjust.sharpness", 0.0, 100.0, 0.0),
                slider("clarity", "effect.adjust.clarity", 0.0, 100.0, 0.0),
                slider("vignette", "effect.adjust.vignette", -100.0, 100.0, 0.0),
            ]),
        }
    }
}

impl Default for Texture {
    fn default() -> Self {
        Self::new()
    }
}

impl Describe for Texture {
    fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }
}

/// Work units per pixel besides the blurs: the formulas and the transfer curves.
const BASE_UNITS: u64 = 8;

/// The sliders as factors, and the blur radii in device pixels.
struct Settings {
    sharpness: f64,
    clarity: f64,
    vignette: f64,
    sharpness_radius: f64,
    clarity_radius: f64,
}

impl Settings {
    /// Reads `p` with the scale of `cx`. A non-finite scale or value counts as 0: only
    /// `run_cpu` rejects them, before it gets here.
    fn new(cx: &EffectCx, p: &ParamValues) -> Self {
        let number = |id: &str| p.float(id).filter(|v| v.is_finite()).unwrap_or(0.0) / 100.0;
        let scale = if cx.scale.is_finite() {
            f64::from(cx.scale).max(0.0)
        } else {
            0.0
        };
        Self {
            sharpness: number("sharpness"),
            clarity: number("clarity"),
            vignette: number("vignette"),
            sharpness_radius: SHARPNESS_RADIUS * scale,
            clarity_radius: CLARITY_RADIUS * scale,
        }
    }

    fn sharpness_taps(&self) -> usize {
        if self.sharpness == 0.0 {
            0
        } else {
            half_width(self.sharpness_radius)
        }
    }

    fn clarity_taps(&self) -> usize {
        if self.clarity == 0.0 {
            0
        } else {
            half_width(self.clarity_radius)
        }
    }
}

/// Radius of the Nitidez blur and of the Claridad blur, in logical px (FD-33).
const SHARPNESS_RADIUS: f64 = 2.0;
const CLARITY_RADIUS: f64 = 40.0;

fn luma(c: [f64; 3]) -> f64 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

/// Blurred premultiplied linear planes: either the three colour planes and alpha, or the
/// luminance plane and alpha.
struct Planes {
    colour: Vec<Vec<f32>>,
    alpha: Vec<f32>,
}

impl Planes {
    /// The premultiplied linear planes of `src`: alpha, then each channel times alpha.
    /// A pixel with alpha 0 has no colour.
    fn linear(src: &ImageView<'_>, w: usize, h: usize) -> (Vec<Vec<f32>>, Vec<f32>) {
        let mut colour = vec![vec![0.0f32; w * h]; 3];
        let mut alpha = vec![0.0f32; w * h];
        match src.data() {
            PixelData::Rgba8(s) => {
                let t = Transfer::new();
                for (i, px) in s.chunks_exact(4).enumerate() {
                    let a = px[3];
                    if a == 0 {
                        continue;
                    }
                    alpha[i] = f32::from(a) / 255.0;
                    for k in 0..3 {
                        let straight = t.decode[usize::from(unpremultiply_u8(px[k], a))];
                        colour[k][i] = (straight * f64::from(a) / 255.0) as f32;
                    }
                }
            }
            PixelData::RgbaF32(s) => {
                for (i, px) in s.chunks_exact(4).enumerate() {
                    if px[3] == 0.0 {
                        continue;
                    }
                    alpha[i] = px[3];
                    for k in 0..3 {
                        colour[k][i] = px[k];
                    }
                }
            }
            PixelData::Alpha8(_) => {}
        }
        (colour, alpha)
    }

    fn blurred(src: &ImageView<'_>, w: usize, h: usize, radius: f64) -> Self {
        let (colour, alpha) = Self::linear(src, w, h);
        Self {
            colour: colour
                .iter()
                .map(|p| blur_shifted(p, w, h, 0, 0, radius))
                .collect(),
            alpha: blur_shifted(&alpha, w, h, 0, 0, radius),
        }
    }

    fn blurred_luma(src: &ImageView<'_>, w: usize, h: usize, radius: f64) -> Self {
        let (colour, alpha) = Self::linear(src, w, h);
        let y: Vec<f32> = (0..w * h)
            .map(|i| {
                luma([
                    f64::from(colour[0][i]),
                    f64::from(colour[1][i]),
                    f64::from(colour[2][i]),
                ]) as f32
            })
            .collect();
        Self {
            colour: vec![blur_shifted(&y, w, h, 0, 0, radius)],
            alpha: blur_shifted(&alpha, w, h, 0, 0, radius),
        }
    }

    /// The straight blurred colour at pixel `i`, or `None` where no alpha reaches it.
    fn straight(&self, i: usize) -> Option<[f64; 3]> {
        let a = f64::from(self.alpha[i]);
        (a > 0.0).then(|| [0, 1, 2].map(|k| f64::from(self.colour[k][i]) / a))
    }

    /// The straight blurred luminance at pixel `i`.
    fn luma(&self, i: usize) -> Option<f64> {
        let a = f64::from(self.alpha[i]);
        (a > 0.0).then(|| f64::from(self.colour[0][i]) / a)
    }
}

impl Effect for Texture {
    fn params(&self) -> &ParamSchema {
        &self.schema
    }

    fn roi(&self, cx: &EffectCx, p: &ParamValues, out: IRect) -> IRect {
        if out.is_empty() {
            return out;
        }
        let g = Settings::new(cx, p);
        let taps = g.sharpness_taps().max(g.clarity_taps()) as i32;
        // A vignette reads nothing but its own pixel; `cx.tile` places it in its image.
        out.inflate(taps, taps)
    }

    fn cost_hint(&self, cx: &EffectCx, p: &ParamValues, px: u64) -> Cost {
        let g = Settings::new(cx, p);
        // Per active blur: four planes (Nitidez) or two (Claridad), two passes of `2k + 1` taps.
        let blur = |taps: usize, planes: u64| planes * 2 * (2 * taps as u64 + 1);
        let work = BASE_UNITS
            + if g.sharpness > 0.0 {
                blur(g.sharpness_taps(), 4)
            } else {
                0
            }
            + if g.clarity > 0.0 {
                blur(g.clarity_taps(), 2)
            } else {
                0
            };
        Cost {
            units: px.saturating_mul(work),
        }
    }

    fn run_cpu(
        &self,
        cx: &EffectCx,
        p: &ParamValues,
        src: ImageView<'_>,
        dst: ImageViewMut<'_>,
    ) -> Result<(), EffectError> {
        check_scale(cx)?;
        let values = self.schema.resolve(p)?;
        check_io(&src, &dst)?;
        let (w, h) = (src.width() as usize, src.height() as usize);
        check_tile(cx, w, h)?;
        if w == 0 || h == 0 {
            return Ok(());
        }
        let g = Settings::new(cx, &values);
        let sharp = (g.sharpness != 0.0).then(|| Planes::blurred(&src, w, h, g.sharpness_radius));
        let clear = (g.clarity != 0.0).then(|| Planes::blurred_luma(&src, w, h, g.clarity_radius));
        // Viñeta centres on the image, not on `src` (P16).
        let (x0, y0, image_w, image_h) = window_of(cx, w, h);
        let (image_w, image_h) = (image_w as f64, image_h as f64);
        let half_diagonal = image_w.hypot(image_h) / 2.0;
        map_straight(src, dst, |i, c| {
            let mut out = c;
            if let Some(b) = &sharp
                && let Some(blur) = b.straight(i)
            {
                let e = smoothstep(0.02, 0.1, (luma(c) - luma(blur)).abs());
                for ch in 0..3 {
                    out[ch] += 1.5 * g.sharpness * e * (c[ch] - blur[ch]);
                }
            }
            if let Some(b) = &clear
                && let Some(y) = b.luma(i)
            {
                let delta = 0.5 * g.clarity * (luma(c) - y);
                out = out.map(|x| x + delta);
            }
            if g.vignette != 0.0 {
                let x = (x0 + (i % w) as u64) as f64 + 0.5 - image_w / 2.0;
                let y = (y0 + (i / w) as u64) as f64 + 0.5 - image_h / 2.0;
                let d = x.hypot(y) / half_diagonal;
                let f = (g.vignette * smoothstep(0.5, 1.0, d)).exp2();
                out = out.map(|v| v * f);
            }
            (out != c).then_some(out)
        })
    }
}

#[cfg(test)]
mod tests {
    use gafira_doc::contract::ParamKind;

    use super::super::testing::*;
    use super::*;
    use crate::TileWindow;

    fn tex() -> Texture {
        Texture::new()
    }

    /// Straight linear colour and alpha of every pixel, from the test images.
    type Px = ([f64; 3], f64);

    fn from_u8(b: &[u8]) -> Vec<Px> {
        b.chunks(4)
            .map(|p| {
                let a = f64::from(p[3]);
                if p[3] == 0 {
                    return ([0.0; 3], 0.0);
                }
                let s = |c: u8| lin((f64::from(c) * 255.0 / a).round().min(255.0));
                ([s(p[0]), s(p[1]), s(p[2])], a / 255.0)
            })
            .collect()
    }

    fn from_f32(b: &[f32]) -> Vec<Px> {
        b.chunks(4)
            .map(|p| {
                let a = f64::from(p[3]);
                if a == 0.0 {
                    return ([0.0; 3], 0.0);
                }
                ([0, 1, 2].map(|k| f64::from(p[k]) / a), a)
            })
            .collect()
    }

    /// Gaussian blur of a plane written again: CSS radius, sigma = radius / 2, taps cut at
    /// ceil(1.5 radius), weights summing to 1, zero outside the image.
    fn blur_ref(plane: &[f64], w: usize, h: usize, radius: f64) -> Vec<f64> {
        let sigma = radius / 2.0;
        let k = (1.5 * radius).ceil() as i64;
        let raw: Vec<f64> = (-k..=k)
            .map(|d| (-((d * d) as f64) / (2.0 * sigma * sigma)).exp())
            .collect();
        let total: f64 = raw.iter().sum();
        let ker: Vec<f64> = raw.iter().map(|x| x / total).collect();
        let at = |p: &[f64], x: i64, y: i64| {
            if x < 0 || y < 0 || x >= w as i64 || y >= h as i64 {
                0.0
            } else {
                p[y as usize * w + x as usize]
            }
        };
        let mut rows = vec![0.0; w * h];
        for y in 0..h {
            for x in 0..w {
                rows[y * w + x] = (-k..=k)
                    .map(|d| ker[(d + k) as usize] * at(plane, x as i64 + d, y as i64))
                    .sum();
            }
        }
        let mut out = vec![0.0; w * h];
        for y in 0..h {
            for x in 0..w {
                out[y * w + x] = (-k..=k)
                    .map(|d| ker[(d + k) as usize] * at(&rows, x as i64, y as i64 + d))
                    .sum();
            }
        }
        out
    }

    fn lum(c: [f64; 3]) -> f64 {
        0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
    }

    /// The Textura formulas of FD-33 on straight linear pixels. `k` is the three sliders
    /// divided by 100; radii are 2 px and 40 px times `scale`.
    fn reference(px: &[Px], w: usize, h: usize, scale: f64, k: [f64; 3]) -> Vec<[f64; 3]> {
        let [ks, kc, kv] = k;
        let alpha: Vec<f64> = px.iter().map(|p| p.1).collect();
        let plane = |f: &dyn Fn(&Px) -> f64| -> Vec<f64> { px.iter().map(f).collect() };
        let pre = |ch: usize| plane(&|p: &Px| p.0[ch] * p.1);
        let (r2, r40) = (2.0 * scale, 40.0 * scale);
        let g2: Vec<Vec<f64>> = (0..3).map(|ch| blur_ref(&pre(ch), w, h, r2)).collect();
        let a2 = blur_ref(&alpha, w, h, r2);
        let y40 = blur_ref(&plane(&|p: &Px| lum(p.0) * p.1), w, h, r40);
        let a40 = blur_ref(&alpha, w, h, r40);
        let mut out = Vec::new();
        for (i, p) in px.iter().enumerate() {
            let mut c = p.0;
            if p.1 > 0.0 {
                if ks != 0.0 {
                    let g = [0, 1, 2].map(|ch| g2[ch][i] / a2[i]);
                    let e = smooth(0.02, 0.1, (lum(p.0) - lum(g)).abs());
                    for ch in 0..3 {
                        c[ch] += 1.5 * ks * e * (p.0[ch] - g[ch]);
                    }
                }
                if kc != 0.0 {
                    let d = 0.5 * kc * (lum(p.0) - y40[i] / a40[i]);
                    c = c.map(|x| x + d);
                }
                if kv != 0.0 {
                    let (x, y) = ((i % w) as f64 + 0.5, (i / w) as f64 + 0.5);
                    let d = ((x - w as f64 / 2.0).hypot(y - h as f64 / 2.0))
                        / ((w as f64).hypot(h as f64) / 2.0);
                    let f = (kv * smooth(0.5, 1.0, d)).exp2();
                    c = c.map(|x| x * f);
                }
            }
            out.push(c);
        }
        out
    }

    fn params_of(k: [f64; 3]) -> ParamValues {
        pv(&[
            ("sharpness", k[0] * 100.0),
            ("clarity", k[1] * 100.0),
            ("vignette", k[2] * 100.0),
        ])
    }

    /// A smooth f32 image with a hard edge and a varying alpha.
    fn edge_f32(w: usize, h: usize) -> Vec<f32> {
        let mut v = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let a = if (x + y) % 11 == 0 { 0.6 } else { 1.0 };
                let base = if x < w / 2 { 0.15 } else { 0.7 };
                let ramp = 0.1 * y as f64 / h as f64;
                let c = [base + ramp, base * 0.8 + ramp, base * 0.5];
                v.extend([
                    (c[0] * a) as f32,
                    (c[1] * a) as f32,
                    (c[2] * a) as f32,
                    a as f32,
                ]);
            }
        }
        v
    }

    fn assert_close(got: &[f32], want: &[[f64; 3]], src: &[f32], tol: f64, what: &str) {
        for (i, (g, w)) in got.chunks(4).zip(want).enumerate() {
            let a = f64::from(src[i * 4 + 3]);
            for ch in 0..3 {
                let expect = if a == 0.0 {
                    f64::from(src[i * 4 + ch])
                } else {
                    w[ch] * a
                };
                assert!(
                    (f64::from(g[ch]) - expect).abs() < tol * expect.abs().max(1.0),
                    "{what} px {i} ch {ch}: {} vs {expect}",
                    g[ch]
                );
            }
            assert_eq!(g[3].to_bits(), src[i * 4 + 3].to_bits());
        }
    }

    // Passes from the start (data only); proved by mutants on the ranges, defaults and ids.
    #[test]
    fn fd_33_texture_schema_has_three_sliders() {
        let e = tex();
        assert_eq!(e.descriptor().id, "effect.adjust.texture");
        e.params().validate_definition().unwrap();
        let ids: Vec<_> = e.params().params().iter().map(|d| d.id).collect();
        assert_eq!(ids, ["sharpness", "clarity", "vignette"]);
        let range = |id: &str| match e.params().get(id).unwrap().kind {
            ParamKind::Float { min, max, default } => (min, max, default),
            _ => panic!(),
        };
        assert_eq!(range("sharpness"), (0.0, 100.0, 0.0));
        assert_eq!(range("clarity"), (0.0, 100.0, 0.0));
        assert_eq!(range("vignette"), (-100.0, 100.0, 0.0));
        assert!(e.params().params().iter().all(|d| d.animatable));
        assert!(e.gpu().is_none());
    }

    // FD-33 (a)
    #[test]
    fn fd_33_a_texture_defaults_are_identity() {
        assert_defaults_are_identity(&tex());
    }

    // FD-33 (f): Nitidez and Claridad leave a flat image as it is, at the edges too (the blur is
    // divided by its own alpha, so the border does not darken).
    #[test]
    fn fd_33_f_sharpness_and_clarity_do_not_change_a_flat_image() {
        let sets = [
            pv(&[("sharpness", 100.0)]),
            pv(&[("clarity", 100.0)]),
            pv(&[("sharpness", 100.0), ("clarity", 100.0)]),
        ];
        for (w, h) in [(1usize, 1usize), (2, 3), (30, 20), (70, 5)] {
            for px in [
                [120u8, 60, 200, 255],
                [90, 45, 30, 128],
                [255, 255, 255, 255],
                [0, 0, 0, 255],
            ] {
                let src: Vec<u8> = px.iter().copied().cycle().take(w * h * 4).collect();
                for p in &sets {
                    for scale in [1.0f32, 0.25] {
                        let out = run8(&tex(), &cx(scale), p, &src, (w as u32, h as u32)).unwrap();
                        assert_eq!(out, src, "{w}x{h} {px:?} {p:?} scale {scale}");
                    }
                }
            }
            let srcf: Vec<f32> = [0.3f32, 0.2, 0.1, 0.5]
                .iter()
                .copied()
                .cycle()
                .take(w * h * 4)
                .collect();
            for p in &sets {
                let out = runf(&tex(), &cx(1.0), p, &srcf, (w as u32, h as u32)).unwrap();
                for (a, b) in srcf.iter().zip(&out) {
                    assert!((a - b).abs() < 1e-5, "{w}x{h}: {a} vs {b}");
                }
            }
        }
    }

    // FD-33 (f): Viñeta does not change the centre, nor anything inside half the half-diagonal.
    #[test]
    fn fd_33_f_vignette_does_not_change_the_centre() {
        for (w, h) in [(21usize, 21usize), (1, 1), (40, 24)] {
            let src = img_u8(w, h, 4);
            for v in [-100.0, 100.0] {
                let out = run8(
                    &tex(),
                    &cx(1.0),
                    &pv(&[("vignette", v)]),
                    &src,
                    (w as u32, h as u32),
                )
                .unwrap();
                let mut changed = 0;
                for i in 0..w * h {
                    let (x, y) = ((i % w) as f64 + 0.5, (i / w) as f64 + 0.5);
                    let d = (x - w as f64 / 2.0).hypot(y - h as f64 / 2.0)
                        / ((w as f64).hypot(h as f64) / 2.0);
                    if d <= 0.5 {
                        assert_eq!(out[i * 4..][..4], src[i * 4..][..4], "{w}x{h} px {i} d {d}");
                    } else if out[i * 4..][..4] != src[i * 4..][..4] {
                        changed += 1;
                    }
                }
                if w * h > 100 {
                    assert!(changed > 10, "{w}x{h} v {v}: {changed}");
                }
            }
        }
    }

    // FD-33: Viñeta c x 2^(k w(0.5, 1, d)), d from the pixel centre to half the diagonal.
    #[test]
    fn fd_33_texture_vignette_matches_the_formula() {
        let (w, h) = (31usize, 17usize);
        let src: Vec<f32> = [0.4f32, 0.3, 0.2, 1.0]
            .iter()
            .copied()
            .cycle()
            .take(w * h * 4)
            .collect();
        for kv in [1.0, -1.0, 0.5, -0.3] {
            let out = runf(
                &tex(),
                &cx(1.0),
                &params_of([0.0, 0.0, kv]),
                &src,
                (w as u32, h as u32),
            )
            .unwrap();
            for i in 0..w * h {
                let (x, y) = ((i % w) as f64 + 0.5, (i / w) as f64 + 0.5);
                let d = (x - w as f64 / 2.0).hypot(y - h as f64 / 2.0)
                    / ((w as f64).hypot(h as f64) / 2.0);
                let f = (kv * smooth(0.5, 1.0, d)).exp2();
                for ch in 0..3 {
                    let want = f64::from(src[i * 4 + ch]) * f;
                    assert!(
                        (f64::from(out[i * 4 + ch]) - want).abs() < 1e-5,
                        "kv {kv} px {i} d {d}: {} vs {want}",
                        out[i * 4 + ch]
                    );
                }
            }
        }
        // A negative value darkens the corners, a positive one brightens them.
        let dark = runf(
            &tex(),
            &cx(1.0),
            &pv(&[("vignette", -100.0)]),
            &src,
            (w as u32, h as u32),
        )
        .unwrap();
        assert!(dark[0] < src[0]);
        let bright = runf(
            &tex(),
            &cx(1.0),
            &pv(&[("vignette", 100.0)]),
            &src,
            (w as u32, h as u32),
        )
        .unwrap();
        assert!(bright[0] > src[0]);
    }

    // FD-33: Nitidez and Claridad against the formulas with a gaussian written again, f32.
    #[test]
    fn fd_33_texture_sharpness_and_clarity_match_the_reference_in_f32() {
        let (w, h) = (24usize, 18usize);
        let srcs = [edge_f32(w, h), img_f32(w, h, 21)];
        let cases: [(f32, [f64; 3]); 7] = [
            (1.0, [1.0, 0.0, 0.0]),
            (1.0, [0.4, 0.0, 0.0]),
            (2.0, [1.0, 0.0, 0.0]),
            (1.0, [0.0, 1.0, 0.0]),
            (0.25, [0.0, 0.7, 0.0]),
            (0.5, [0.0, 1.0, 0.0]),
            (1.0, [0.6, 0.8, -0.5]),
        ];
        for src in &srcs {
            let px = from_f32(src);
            for (scale, k) in cases {
                let out =
                    runf(&tex(), &cx(scale), &params_of(k), src, (w as u32, h as u32)).unwrap();
                let want = reference(&px, w, h, f64::from(scale), k);
                assert_close(&out, &want, src, 3e-5, &format!("scale {scale} {k:?}"));
            }
        }
    }

    // FD-33, 8 bits: the same reference within 1/255 on premultiplied pixels with alpha 0.
    #[test]
    fn fd_33_texture_matches_the_reference_in_8_bits() {
        let (w, h) = (20usize, 14usize);
        let mut src = img_u8(w, h, 77);
        // A hard vertical edge so the mask e is not zero.
        for (i, px) in src.chunks_mut(4).enumerate() {
            if px[3] == 255 {
                let v = if i % w < w / 2 { 30 } else { 220 };
                px[..3].copy_from_slice(&[v, v / 2 + 10, 90]);
            }
        }
        let px = from_u8(&src);
        for (scale, k) in [
            (1.0f32, [1.0, 0.0, 0.0]),
            (1.0, [0.0, 1.0, 0.0]),
            (2.0, [0.5, 0.5, -0.8]),
            (0.25, [0.0, 0.5, 0.3]),
        ] {
            let out = run8(
                &tex(),
                &cx(scale),
                &params_of(k),
                &src,
                (w as u32, h as u32),
            )
            .unwrap();
            let want = reference(&px, w, h, f64::from(scale), k);
            for (i, (o, wc)) in out.chunks(4).zip(&want).enumerate() {
                let a = f64::from(src[i * 4 + 3]);
                if src[i * 4 + 3] == 0 {
                    assert_eq!(o, &src[i * 4..][..4]);
                    continue;
                }
                for ch in 0..3 {
                    let e = (f64::from(enc(wc[ch])) * a / 255.0).round();
                    assert!(
                        (f64::from(o[ch]) - e).abs() <= 1.0,
                        "scale {scale} {k:?} px {i} ch {ch}: {} vs {e}",
                        o[ch]
                    );
                }
                assert_eq!(o[3], src[i * 4 + 3]);
            }
        }
    }

    // FD-33: Nitidez makes an edge steeper: the bright side gets brighter, the dark side darker.
    #[test]
    fn fd_33_texture_sharpness_steepens_an_edge() {
        let (w, h) = (16usize, 4usize);
        let mut src = Vec::new();
        for _ in 0..h {
            for x in 0..w {
                let v = if x < w / 2 { 60u8 } else { 190 };
                src.extend([v, v, v, 255]);
            }
        }
        let out = run8(
            &tex(),
            &cx(1.0),
            &pv(&[("sharpness", 100.0)]),
            &src,
            (w as u32, h as u32),
        )
        .unwrap();
        let (dark_edge, bright_edge) = (out[(w / 2 - 1) * 4], out[(w / 2) * 4]);
        assert!(
            dark_edge < 60 && bright_edge > 190,
            "{dark_edge} {bright_edge}"
        );
        assert_eq!(out[0], 60);
        assert_eq!(out[(w - 1) * 4], 190);
    }

    // FD-33 (f)
    #[test]
    fn fd_33_f_texture_keeps_alpha() {
        assert_alpha_is_kept(&tex(), 1.0, &[]);
        assert_alpha_is_kept(&tex(), 0.25, &[]);
    }

    // FD-33 (h), ARQ-110
    #[test]
    fn fd_33_h_texture_bad_parameters_are_err_and_dst_is_untouched() {
        assert_bad_parameters_are_err_and_ends_pass(&tex());
    }

    #[test]
    fn arq_110_texture_non_finite_scale_and_pixels_are_err() {
        assert_non_finite_scale_and_pixels_are_err(&tex());
    }

    // ARQ-110: a result that overflows `f32` is Err before anything is written.
    #[test]
    fn arq_110_texture_overflowing_result_is_err_and_dst_is_untouched() {
        let src = [
            0.5f32, 0.5, 0.5, 1.0, 3.0e38, 0.5, 0.5, 1.0, 0.5, 0.5, 0.5, 1.0,
        ];
        let (r, dst) = runf_keep(&tex(), &cx(1.0), &pv(&[("sharpness", 100.0)]), &src, (3, 1));
        assert!(matches!(r, Err(EffectError::Buffer(_))), "{r:?}");
        assert!(dst.iter().all(|b| b.to_bits() == SENTINEL_F32));
    }

    #[test]
    fn texture_rejects_other_formats_and_sizes() {
        assert_format_and_size_errors(&tex());
    }

    #[test]
    fn texture_empty_images_are_ok() {
        assert_empty_images_are_ok(&tex());
    }

    // ARQ-111
    #[test]
    fn arq_111_texture_is_deterministic() {
        assert_deterministic(&tex(), &params_of([0.5, 0.7, -0.4]));
    }

    // ARQ-120: the input a tile needs is the output grown by the largest blur in use, at the
    // scale of the context; a vignette alone needs nothing more.
    #[test]
    fn arq_120_texture_roi_grows_by_the_largest_active_kernel() {
        let out = IRect::new(10, 20, 50, 60);
        let roi = |scale: f32, k: [f64; 3]| tex().roi(&cx(scale), &params_of(k), out);
        // radius 2 px gives ceil(3 scale) taps; radius 40 px gives ceil(60 scale).
        for (scale, s_taps, c_taps) in [(1.0, 3, 60), (2.0, 6, 120), (0.5, 2, 30), (0.25, 1, 15)] {
            assert_eq!(
                roi(scale, [1.0, 0.0, 0.0]),
                out.inflate(s_taps, s_taps),
                "sharp {scale}"
            );
            assert_eq!(
                roi(scale, [0.0, 1.0, 0.0]),
                out.inflate(c_taps, c_taps),
                "clarity {scale}"
            );
            assert_eq!(
                roi(scale, [1.0, 1.0, 0.0]),
                out.inflate(c_taps, c_taps),
                "both {scale}"
            );
        }
        assert_eq!(roi(1.0, [0.0, 0.0, 1.0]), out);
        assert_eq!(roi(1.0, [0.0, 0.0, 0.0]), out);
        assert_eq!(roi(1.0, [0.0, 0.0, -0.5]), out);
    }

    // ARQ-120: running a tile cut with `roi` gives the whole-image pixels, <= 1/255.
    #[test]
    fn arq_120_texture_tiles_with_roi_match_the_whole_image() {
        let (w, h) = (64usize, 48usize);
        let mut src = img_u8(w, h, 5);
        for (i, px) in src.chunks_mut(4).enumerate() {
            if px[3] == 255 && i % w > w / 3 {
                px[..3].copy_from_slice(&[240, 200, 30]);
            }
        }
        for (scale, k) in [
            (1.0f32, [1.0, 0.0, 0.0]),
            (0.25, [0.6, 1.0, 0.0]),
            (0.5, [0.0, 1.0, 0.0]),
        ] {
            let p = params_of(k);
            let full = run8(&tex(), &cx(scale), &p, &src, (w as u32, h as u32)).unwrap();
            assert!(full.iter().any(|b| *b != SENTINEL), "nothing was written");
            assert_ne!(full, src, "the effect changed nothing");
            let out = IRect::new(20, 12, 44, 34);
            let r = tex().roi(&cx(scale), &p, out);
            let (x0, y0) = (r.x0.max(0) as usize, r.y0.max(0) as usize);
            let (x1, y1) = ((r.x1 as usize).min(w), (r.y1 as usize).min(h));
            let (tw, th) = (x1 - x0, y1 - y0);
            let mut crop = Vec::new();
            for y in y0..y1 {
                crop.extend_from_slice(&src[(y * w + x0) * 4..(y * w + x1) * 4]);
            }
            let tile = run8(&tex(), &cx(scale), &p, &crop, (tw as u32, th as u32)).unwrap();
            for y in 12..34usize {
                for x in 20..44usize {
                    let a = &full[(y * w + x) * 4..][..4];
                    let b = &tile[((y - y0) * tw + (x - x0)) * 4..][..4];
                    for ch in 0..4 {
                        assert!(
                            a[ch].abs_diff(b[ch]) <= 1,
                            "scale {scale} ({x},{y}) {a:?} {b:?}"
                        );
                    }
                }
            }
        }
    }

    fn window(image: (usize, usize), at: (usize, usize)) -> EffectCx {
        EffectCx {
            scale: 1.0,
            tile: Some(TileWindow {
                image_width: image.0 as u32,
                image_height: image.1 as u32,
                x: at.0 as u32,
                y: at.1 as u32,
            }),
        }
    }

    /// The pixels of `rect` (x0, y0, x1, y1) of a `w`-wide image with `n` values per pixel.
    fn crop<T: Copy>(src: &[T], w: usize, n: usize, rect: (usize, usize, usize, usize)) -> Vec<T> {
        let mut v = Vec::new();
        for y in rect.1..rect.3 {
            v.extend_from_slice(&src[(y * w + rect.0) * n..(y * w + rect.2) * n]);
        }
        v
    }

    // ARQ-120, P16: a tile cut with `roi` and placed in its image by `cx.tile` gives the
    // whole-image pixels, <= 1/255, with Viñeta active: in the middle, at the top-left corner
    // and at the bottom-right one, in both formats.
    #[test]
    fn arq_120_texture_vignette_tiles_with_a_window_match_the_whole_image() {
        let (w, h) = (64usize, 48usize);
        let s8 = img_u8(w, h, 5);
        let sf = img_f32(w, h, 5);
        let size = (w as u32, h as u32);
        for (k, out) in [
            ([0.5, 0.0, 0.8], (20usize, 12usize, 44usize, 34usize)),
            ([0.0, 0.0, -1.0], (0, 0, 16, 16)),
            ([0.0, 0.0, 1.0], (48, 32, 64, 48)),
            ([0.0, 0.7, -0.8], (30, 5, 60, 20)),
        ] {
            let p = params_of(k);
            let r = tex().roi(
                &cx(1.0),
                &p,
                IRect::new(out.0 as i32, out.1 as i32, out.2 as i32, out.3 as i32),
            );
            let rect = (
                r.x0.max(0) as usize,
                r.y0.max(0) as usize,
                (r.x1 as usize).min(w),
                (r.y1 as usize).min(h),
            );
            let (tw, th) = (rect.2 - rect.0, rect.3 - rect.1);
            let c = window((w, h), (rect.0, rect.1));
            let tsize = (tw as u32, th as u32);

            let full = run8(&tex(), &cx(1.0), &p, &s8, size).unwrap();
            assert_ne!(full, s8, "the effect changed nothing {k:?}");
            let tile = run8(&tex(), &c, &p, &crop(&s8, w, 4, rect), tsize).unwrap();
            let fullf = runf(&tex(), &cx(1.0), &p, &sf, size).unwrap();
            let tilef = runf(&tex(), &c, &p, &crop(&sf, w, 4, rect), tsize).unwrap();
            for y in out.1..out.3 {
                for x in out.0..out.2 {
                    let (a, b) = (
                        &full[(y * w + x) * 4..][..4],
                        &tile[((y - rect.1) * tw + (x - rect.0)) * 4..][..4],
                    );
                    for ch in 0..4 {
                        assert!(a[ch].abs_diff(b[ch]) <= 1, "{k:?} ({x},{y}) {a:?} {b:?}");
                    }
                    let (a, b) = (
                        &fullf[(y * w + x) * 4..][..4],
                        &tilef[((y - rect.1) * tw + (x - rect.0)) * 4..][..4],
                    );
                    for ch in 0..4 {
                        assert!(
                            (a[ch] - b[ch]).abs() <= 1.0 / 255.0,
                            "{k:?} ({x},{y}) {a:?} {b:?}"
                        );
                    }
                }
            }
        }
    }

    // P16: a window at the origin with the size of `src` is the same as no window, bit for bit.
    #[test]
    fn arq_120_texture_window_at_the_origin_equals_none_bit_for_bit() {
        let (w, h) = (23usize, 17usize);
        let size = (w as u32, h as u32);
        let s8 = img_u8(w, h, 9);
        let sf = img_f32(w, h, 9);
        for k in [[0.0, 0.0, 1.0], [0.4, 0.6, -0.7], [0.0; 3]] {
            let p = params_of(k);
            let c = window((w, h), (0, 0));
            assert_eq!(
                run8(&tex(), &c, &p, &s8, size).unwrap(),
                run8(&tex(), &cx(1.0), &p, &s8, size).unwrap(),
                "{k:?}"
            );
            assert_eq!(
                bits(&runf(&tex(), &c, &p, &sf, size).unwrap()),
                bits(&runf(&tex(), &cx(1.0), &p, &sf, size).unwrap()),
                "{k:?}"
            );
        }
    }

    // P16: a window that does not hold `src` is `Err` naming `cx.tile`, before anything is
    // written and before the empty-image return; the sum is made in `u64` and saturates.
    #[test]
    fn arq_120_texture_window_that_does_not_fit_is_err_and_dst_is_untouched() {
        let s8 = img_u8(4, 3, 1);
        let sf = img_f32(4, 3, 1);
        let p = params_of([0.0, 0.0, 0.5]);
        let tile = |iw: u32, ih: u32, x: u32, y: u32| EffectCx {
            scale: 1.0,
            tile: Some(TileWindow {
                image_width: iw,
                image_height: ih,
                x,
                y,
            }),
        };
        let err = |actual: (u32, u32), expected: (u32, u32)| {
            Err(EffectError::SizeMismatch {
                field: "cx.tile",
                expected,
                actual,
            })
        };
        // (window, expected Err) for a 4x3 `src`.
        let cases = [
            (tile(10, 10, 7, 0), err((11, 3), (10, 10))),
            (tile(10, 10, 0, 8), err((4, 11), (10, 10))),
            (tile(3, 3, 0, 0), err((4, 3), (3, 3))),
            (tile(4, 2, 0, 0), err((4, 3), (4, 2))),
            (tile(0, 0, 0, 0), err((4, 3), (0, 0))),
            (
                tile(u32::MAX, u32::MAX, u32::MAX, u32::MAX),
                err((u32::MAX, u32::MAX), (u32::MAX, u32::MAX)),
            ),
        ];
        for (c, want) in cases {
            let (r, dst) = run8_keep(&tex(), &c, &p, &s8, (4, 3));
            assert_eq!(r, want, "{c:?}");
            assert!(dst.iter().all(|b| *b == SENTINEL));
            let (r, dst) = runf_keep(&tex(), &c, &p, &sf, (4, 3));
            assert_eq!(r, want, "{c:?}");
            assert!(dst.iter().all(|b| b.to_bits() == SENTINEL_F32));
        }
        // The limit itself fits; an empty `src` outside its image is still `Err`.
        assert!(run8(&tex(), &tile(4, 3, 0, 0), &p, &s8, (4, 3)).is_ok());
        assert!(run8(&tex(), &tile(10, 10, 6, 7), &p, &s8, (4, 3)).is_ok());
        let (r, _) = run8_keep(&tex(), &tile(3, 3, 5, 0), &p, &[], (0, 3));
        assert_eq!(r, err((5, 3), (3, 3)));
    }

    // Passes from the start; proved by a mutant on the cost.
    #[test]
    fn texture_cost_grows_with_pixels_and_with_the_blur() {
        let e = tex();
        let c = |scale: f32, k: [f64; 3], px| e.cost_hint(&cx(scale), &params_of(k), px).units;
        assert_eq!(c(1.0, [0.0; 3], 0), 0);
        assert!(c(1.0, [0.0; 3], 1000) > 0);
        assert_eq!(
            c(1.0, [1.0, 0.0, 0.0], 2000),
            2 * c(1.0, [1.0, 0.0, 0.0], 1000)
        );
        assert!(c(1.0, [1.0, 0.0, 0.0], 1000) > c(1.0, [0.0, 0.0, 1.0], 1000));
        assert!(c(1.0, [0.0, 1.0, 0.0], 1000) > c(1.0, [1.0, 0.0, 0.0], 1000));
        assert!(c(2.0, [0.0, 1.0, 0.0], 1000) > c(1.0, [0.0, 1.0, 0.0], 1000));
    }
}
