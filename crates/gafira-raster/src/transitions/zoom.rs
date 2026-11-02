//! Zoom: a dissolve with both frames scaled about the centre (ARQ-118).

use gafira_doc::contract::{
    Describe, Descriptor, ParamChoice, ParamDef, ParamKind, ParamSchema, ParamValues,
};
use gafira_doc::msg::MessageKey;

use super::guard::{Frames, Plan, copy, plan, render};
use super::mix::{weighted_f32, weighted_u8};
use super::{HandleReq, Transition};
use crate::transfer::Transfer;
use crate::{EffectCx, EffectError, ImageView, ImageViewMut};

/// The directions of the `direction` parameter: zoom in or zoom out.
const DIRECTIONS: &[ParamChoice] = &[
    ParamChoice {
        id: "in",
        label: MessageKey("transition.direction.in"),
    },
    ParamChoice {
        id: "out",
        label: MessageKey("transition.direction.out"),
    },
];

/// The four pixels and weights that a bilinear sample at the screen pixel `(x, y)` reads from an
/// image `w` by `h` scaled by `scale` about its centre `(w / 2, h / 2)`. Outside the image the
/// edge pixel repeats.
fn taps(w: usize, h: usize, x: usize, y: usize, scale: f64) -> [(usize, usize, f64); 4] {
    let (cx, cy) = (w as f64 / 2.0, h as f64 / 2.0);
    let sx = cx + ((x as f64 + 0.5) - cx) / scale - 0.5;
    let sy = cy + ((y as f64 + 0.5) - cy) / scale - 0.5;
    let (x_floor, y_floor) = (sx.floor(), sy.floor());
    let (tx, ty) = (sx - x_floor, sy - y_floor);
    let clamp = |v: f64, n: usize| (v.max(0.0) as usize).min(n - 1);
    let (x0, x1) = (clamp(x_floor, w), clamp(x_floor + 1.0, w));
    let (y0, y1) = (clamp(y_floor, h), clamp(y_floor + 1.0, h));
    [
        (x0, y0, (1.0 - tx) * (1.0 - ty)),
        (x1, y0, tx * (1.0 - ty)),
        (x0, y1, (1.0 - tx) * ty),
        (x1, y1, tx * ty),
    ]
}

/// The eight samples of one output pixel: four taps of each frame, weighted `1 - p` and `p`.
fn samples<T: Copy>(
    f: &Frames<'_, T>,
    x: usize,
    y: usize,
    p: f64,
    scales: (f64, f64),
) -> Vec<([T; 4], f64)> {
    let mut out = Vec::with_capacity(8);
    for (use_to, scale, weight) in [(false, scales.0, 1.0 - p), (true, scales.1, p)] {
        for (tx, ty, w) in taps(f.w, f.h, x, y, scale) {
            out.push((f.px(use_to, tx, ty), weight * w));
        }
    }
    out
}

/// The zoom.
#[derive(Debug)]
pub struct Zoom {
    descriptor: Descriptor,
    schema: ParamSchema,
}

impl Zoom {
    /// Builds the transition.
    #[must_use]
    pub fn new() -> Self {
        Self {
            descriptor: Descriptor::new(
                "transition.zoom",
                MessageKey("glossary.transition_zoom"),
                1,
            ),
            schema: ParamSchema::new(vec![ParamDef {
                id: "direction",
                label: MessageKey("transition.zoom.direction"),
                kind: ParamKind::Choice {
                    options: DIRECTIONS,
                    default: "in",
                },
                animatable: false,
            }]),
        }
    }
}

impl Default for Zoom {
    fn default() -> Self {
        Self::new()
    }
}

impl Describe for Zoom {
    fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }
}

impl Transition for Zoom {
    fn params(&self) -> &ParamSchema {
        &self.schema
    }

    fn handles(&self) -> HandleReq {
        HandleReq::Both
    }

    fn run_cpu(
        &self,
        cx: &EffectCx,
        p: &ParamValues,
        progress: f32,
        from: ImageView<'_>,
        to: ImageView<'_>,
        mut dst: ImageViewMut<'_>,
    ) -> Result<(), EffectError> {
        match plan(cx, &self.schema, p, progress, &from, &to, &dst)? {
            Plan::Done => {}
            Plan::CopyFrom => copy(&mut dst, &from),
            Plan::CopyTo => copy(&mut dst, &to),
            Plan::Mix { p, values } => {
                // `in`: `from` 1 + p, `to` (1 + p) / 2. `out`, the mirror: 1 - p / 2 and 2 - p.
                let scales = if values.choice("direction") == Some("out") {
                    (1.0 - p / 2.0, 2.0 - p)
                } else {
                    (1.0 + p, (1.0 + p) / 2.0)
                };
                let tr = Transfer::new();
                render(
                    &from,
                    &to,
                    &mut dst,
                    |x, y, f| weighted_u8(&tr, &samples(f, x, y, p, scales)),
                    |x, y, f| weighted_f32(&tr, &samples(f, x, y, p, scales)),
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use gafira_doc::contract::{ParamError, ParamKind, ParamValue};

    use super::super::mix::{MIX_SPACE, weighted_u8_in};
    use super::super::tests::{SENTINEL_F32, SENTINEL_U8};
    use super::*;
    use crate::transfer::Transfer;
    use crate::{PixelData, PixelDataMut};

    fn dir(d: &str) -> ParamValues {
        let mut p = ParamValues::new();
        p.set("direction", ParamValue::Choice(d.to_owned()));
        p
    }

    fn zoom_f32(d: &str, progress: f32, from: &[f32], to: &[f32], size: (u32, u32)) -> Vec<f32> {
        let mut dst = vec![f32::from_bits(SENTINEL_F32); from.len()];
        Zoom::new()
            .run_cpu(
                &EffectCx { scale: 1.0 },
                &dir(d),
                progress,
                ImageView::new(size.0, size.1, PixelData::RgbaF32(from)).unwrap(),
                ImageView::new(size.0, size.1, PixelData::RgbaF32(to)).unwrap(),
                ImageViewMut::new(size.0, size.1, PixelDataMut::RgbaF32(&mut dst)).unwrap(),
            )
            .unwrap();
        dst
    }

    fn zoom_u8(d: &str, progress: f32, from: &[u8], to: &[u8], size: (u32, u32)) -> Vec<u8> {
        let mut dst = vec![SENTINEL_U8; from.len()];
        Zoom::new()
            .run_cpu(
                &EffectCx { scale: 1.0 },
                &dir(d),
                progress,
                ImageView::new(size.0, size.1, PixelData::Rgba8(from)).unwrap(),
                ImageView::new(size.0, size.1, PixelData::Rgba8(to)).unwrap(),
                ImageViewMut::new(size.0, size.1, PixelDataMut::Rgba8(&mut dst)).unwrap(),
            )
            .unwrap();
        dst
    }

    /// Opaque grey values as f32 pixels.
    fn grey(values: &[f32]) -> Vec<f32> {
        values.iter().flat_map(|v| [*v, *v, *v, 1.0]).collect()
    }

    fn channel(v: &[f32], c: usize) -> Vec<f32> {
        v.chunks_exact(4).map(|p| p[c]).collect()
    }

    fn near(a: &[f32], b: &[f32], tol: f32) {
        assert!(
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() <= tol),
            "{a:?} vs {b:?}"
        );
    }

    /// A pseudo-random premultiplied frame.
    fn random_frame(n: usize, mut seed: u32, opaque: bool) -> Vec<f32> {
        let mut next = || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            f64::from(seed >> 8) as f32 / 16_777_216.0
        };
        (0..n)
            .flat_map(|_| {
                let a = if opaque { 1.0 } else { next() };
                [next() * a, next() * a, next() * a, a]
            })
            .collect()
    }

    fn random_frame_u8(n: usize, seed: u32, opaque: bool) -> Vec<u8> {
        random_frame(n, seed, opaque)
            .chunks_exact(4)
            .flat_map(|p| {
                let a = (p[3] * 255.0).round() as u8;
                let c = |v: f32| ((v / p[3].max(1e-9)).min(1.0) * 255.0).round() as u8;
                [
                    crate::premultiply_u8(c(p[0]), a),
                    crate::premultiply_u8(c(p[1]), a),
                    crate::premultiply_u8(c(p[2]), a),
                    a,
                ]
            })
            .collect()
    }

    /// The taps of one output pixel as the criterion describes them, written again here:
    /// for `image` scaled by `s` about the centre, the four pixels and weights.
    fn taps(w: usize, h: usize, x: usize, y: usize, s: f64) -> [(usize, usize, f64); 4] {
        let (cx, cy) = (w as f64 / 2.0, h as f64 / 2.0);
        let sx = cx + ((x as f64 + 0.5) - cx) / s - 0.5;
        let sy = cy + ((y as f64 + 0.5) - cy) / s - 0.5;
        let (fx0, fy0) = (sx.floor(), sy.floor());
        let (tx, ty) = (sx - fx0, sy - fy0);
        let cl = |v: f64, n: usize| (v.max(0.0) as usize).min(n - 1);
        let (x0, x1) = (cl(fx0, w), cl(fx0 + 1.0, w));
        let (y0, y1) = (cl(fy0, h), cl(fy0 + 1.0, h));
        [
            (x0, y0, (1.0 - tx) * (1.0 - ty)),
            (x1, y0, tx * (1.0 - ty)),
            (x0, y1, (1.0 - tx) * ty),
            (x1, y1, tx * ty),
        ]
    }

    fn scales(d: &str, p: f64) -> (f64, f64) {
        if d == "in" {
            (1.0 + p, (1.0 + p) / 2.0)
        } else {
            (1.0 - p / 2.0, 2.0 - p)
        }
    }

    // 02 §4.13 acceptance, C12: id, name, handles and the `direction` parameter of the table.
    #[test]
    fn zoom_has_the_identity_and_schema_of_the_table() {
        let t = Zoom::new();
        assert_eq!(t.descriptor().id, "transition.zoom");
        assert_eq!(t.descriptor().name.0, "glossary.transition_zoom");
        assert_eq!(t.descriptor().version, 1);
        assert_eq!(t.handles(), HandleReq::Both);
        assert!(t.params().validate_definition().is_ok());
        let defs = t.params().params();
        assert_eq!(defs.len(), 1);
        assert_eq!(
            (defs[0].id, defs[0].label.0, defs[0].animatable),
            ("direction", "transition.zoom.direction", false)
        );
        let ParamKind::Choice { options, default } = &defs[0].kind else {
            panic!("kind")
        };
        let ids: Vec<_> = options.iter().map(|o| o.id).collect();
        assert_eq!(ids, ["in", "out"]);
        assert_eq!(*default, "in");
        for o in *options {
            assert_eq!(o.label.0, format!("transition.direction.{}", o.id));
        }
    }

    // ARQ-110: `diagonal` is Err naming `direction`, and nothing is written.
    #[test]
    fn arq_110_zoom_rejects_an_unknown_direction() {
        let (f, g) = (vec![1u8; 16], vec![2u8; 16]);
        for progress in [0.0f32, 0.3, 1.0] {
            let mut d = vec![SENTINEL_U8; 16];
            let err = Zoom::new()
                .run_cpu(
                    &EffectCx { scale: 1.0 },
                    &dir("diagonal"),
                    progress,
                    ImageView::new(2, 2, PixelData::Rgba8(&f)).unwrap(),
                    ImageView::new(2, 2, PixelData::Rgba8(&g)).unwrap(),
                    ImageViewMut::new(2, 2, PixelDataMut::Rgba8(&mut d)).unwrap(),
                )
                .unwrap_err();
            assert_eq!(
                err,
                EffectError::Param(ParamError::OutOfRange { param: "direction" })
            );
            assert!(d.iter().all(|b| *b == SENTINEL_U8));
        }
    }

    // ARQ-118 worked by hand: 4x1, `from` a ramp 0..3, `to` black, p = 0.5, centre at 2.
    // in: `from` at 1.5 samples at 0.5, 1.1667, 1.8333, 2.5, and weighs 0.5.
    // out: `from` at 0.75 samples at -0.5 (clamped), 0.8333, 2.1667, 3.5 (clamped).
    #[test]
    fn arq_118_zoom_ramp_values_for_in_and_out() {
        let ramp = grey(&[0.0, 1.0, 2.0, 3.0]);
        let black = grey(&[0.0; 4]);
        let got = zoom_f32("in", 0.5, &ramp, &black, (4, 1));
        near(&channel(&got, 0), &[0.25, 0.583_333, 0.916_667, 1.25], 1e-5);
        near(&channel(&got, 3), &[1.0; 4], 1e-6);
        let got = zoom_f32("out", 0.5, &ramp, &black, (4, 1));
        near(&channel(&got, 0), &[0.0, 0.416_667, 1.083_333, 1.5], 1e-5);
        near(&channel(&got, 3), &[1.0; 4], 1e-6);
        // `to` scales too: in, `to` at (1 + p) / 2 = 0.75 gives the clamped values of the out case.
        let got = zoom_f32("in", 0.5, &black, &ramp, (4, 1));
        near(&channel(&got, 0), &[0.0, 0.416_667, 1.083_333, 1.5], 1e-5);
        // out, `to` at 2 - p = 1.5 gives the values of the in case.
        let got = zoom_f32("out", 0.5, &black, &ramp, (4, 1));
        near(&channel(&got, 0), &[0.25, 0.583_333, 0.916_667, 1.25], 1e-5);
    }

    // ARQ-118: the pivot is the continuous centre (W / 2, H / 2), so an odd side keeps its middle pixel.
    #[test]
    fn arq_118_zoom_pivot_is_the_continuous_centre() {
        let ramp = grey(&[0.0, 1.0, 2.0]);
        let black = grey(&[0.0; 3]);
        // 3x1, centre 1.5: sample x -> 1 + (x - 1) / 1.5; the middle pixel stays at 1.
        let got = zoom_f32("in", 0.5, &ramp, &black, (3, 1));
        near(&channel(&got, 0), &[0.166_667, 0.5, 0.833_333], 1e-5);
        // Vertical: the same on a column.
        let got = zoom_f32("in", 0.5, &ramp, &black, (1, 3));
        near(&channel(&got, 0), &[0.166_667, 0.5, 0.833_333], 1e-5);
    }

    // ARQ-118: the filter works on premultiplied channels.
    #[test]
    fn arq_118_zoom_filters_premultiplied_values() {
        // 2x1: opaque red next to nothing; `to` is nothing. in, p = 0.5, `from` at 1.5.
        let from = [1.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0];
        let to = [0.0f32; 8];
        let got = zoom_f32("in", 0.5, &from, &to, (2, 1));
        // x = 0 samples at 0.16667: premultiplied red 0.8333 and alpha 0.8333, half of each.
        near(&got[..4], &[0.416_667, 0.0, 0.0, 0.416_667], 1e-5);
        // Straight-alpha filtering would give red 0.8333 * 0.8333 / 2 = 0.3472 instead.
        assert!(got[0] > 0.4);
    }

    // ARQ-118: outside the image the edge pixel repeats; two images of one colour stay that
    // colour and opaque images stay opaque at every p, in both directions and both formats.
    #[test]
    fn arq_118_zoom_constant_images_stay_constant_and_opaque_images_opaque() {
        for d in ["in", "out"] {
            for p in [0.05f32, 0.3, 0.5, 0.8, 0.97] {
                for (w, h) in [(5u32, 3u32), (4, 4), (1, 1), (7, 1)] {
                    let n = (w * h) as usize;
                    for c in [
                        [0.2f32, 0.4, 0.1, 1.0],
                        [0.0, 0.0, 0.0, 1.0],
                        [1.0, 1.0, 1.0, 1.0],
                        [0.1, 0.05, 0.0, 0.25],
                        [0.0; 4],
                    ] {
                        let img = c.repeat(n);
                        let got = zoom_f32(d, p, &img, &img, (w, h));
                        for px in got.chunks_exact(4) {
                            for k in 0..4 {
                                assert!(
                                    (px[k] - c[k]).abs() < 1e-6,
                                    "{d} p={p} {w}x{h} {c:?}: {px:?}"
                                );
                            }
                        }
                    }
                    for c in [
                        [51u8, 102, 153, 255],
                        [0, 0, 0, 255],
                        [255, 255, 255, 255],
                        [10, 20, 30, 128],
                        [0; 4],
                        [200, 100, 50, 255],
                    ] {
                        let img = c.repeat(n);
                        let got = zoom_u8(d, p, &img, &img, (w, h));
                        for px in got.chunks_exact(4) {
                            assert_eq!(px, c, "{d} p={p} {w}x{h}");
                        }
                    }
                    // Opaque frames of any content stay opaque.
                    let (a, b) = (random_frame(n, 3, true), random_frame(n, 9, true));
                    assert!(
                        zoom_f32(d, p, &a, &b, (w, h))
                            .chunks_exact(4)
                            .all(|px| (px[3] - 1.0).abs() < 1e-6)
                    );
                    let (a, b) = (random_frame_u8(n, 3, true), random_frame_u8(n, 9, true));
                    assert!(
                        zoom_u8(d, p, &a, &b, (w, h))
                            .chunks_exact(4)
                            .all(|px| px[3] == 255),
                        "{d} p={p}"
                    );
                }
            }
        }
        // Every opaque grey byte survives a zoom of itself.
        for v in 0..=255u8 {
            let img = [v, v, v, 255].repeat(6);
            assert_eq!(zoom_u8("in", 0.37, &img, &img, (3, 2)), img, "grey {v}");
        }
    }

    // ARQ-118: the whole result against the criterion, tap by tap, for random frames: f32 as
    // stored, bytes through the mix with the taps written here.
    #[test]
    fn arq_118_zoom_matches_the_criterion_tap_by_tap() {
        for d in ["in", "out"] {
            for p in [0.1f32, 0.4, 0.5, 0.75, 0.93] {
                for (w, h) in [(5usize, 3usize), (4, 4), (1, 6), (6, 1), (3, 3)] {
                    let n = w * h;
                    let (a, b) = (random_frame(n, 5, false), random_frame(n, 11, false));
                    let got = zoom_f32(d, p, &a, &b, (w as u32, h as u32));
                    let (sf, st) = scales(d, f64::from(p));
                    for y in 0..h {
                        for x in 0..w {
                            let mut want = [0.0f64; 4];
                            for (img, s, wt) in
                                [(&a, sf, 1.0 - f64::from(p)), (&b, st, f64::from(p))]
                            {
                                for (tx, ty, tw) in taps(w, h, x, y, s) {
                                    for k in 0..4 {
                                        want[k] += wt * tw * f64::from(img[(ty * w + tx) * 4 + k]);
                                    }
                                }
                            }
                            let g = &got[(y * w + x) * 4..][..4];
                            for k in 0..4 {
                                assert!(
                                    (f64::from(g[k]) - want[k]).abs() < 1e-5,
                                    "{d} p={p} {w}x{h} ({x},{y}) ch{k}: {g:?} vs {want:?}"
                                );
                            }
                        }
                    }
                    let (a8, b8) = (random_frame_u8(n, 5, false), random_frame_u8(n, 11, false));
                    let got = zoom_u8(d, p, &a8, &b8, (w as u32, h as u32));
                    let t = Transfer::new();
                    for y in 0..h {
                        for x in 0..w {
                            let mut samples = Vec::new();
                            for (img, s, wt) in
                                [(&a8, sf, 1.0 - f64::from(p)), (&b8, st, f64::from(p))]
                            {
                                for (tx, ty, tw) in taps(w, h, x, y, s) {
                                    let i = (ty * w + tx) * 4;
                                    samples.push((
                                        [img[i], img[i + 1], img[i + 2], img[i + 3]],
                                        wt * tw,
                                    ));
                                }
                            }
                            let want = weighted_u8_in(&t, MIX_SPACE, &samples);
                            assert_eq!(
                                &got[(y * w + x) * 4..][..4],
                                &want,
                                "{d} p={p} {w}x{h} ({x},{y})"
                            );
                        }
                    }
                }
            }
        }
    }

    // ARQ-118: `out` with (a, b, p) matches `in` with (b, a, 1 - p) within 1/255.
    #[test]
    fn arq_118_zoom_out_mirrors_zoom_in() {
        for p in [0.1f32, 0.3, 0.5, 0.8] {
            for (w, h) in [(6u32, 4u32), (5, 5), (1, 3)] {
                let n = (w * h) as usize;
                let (a, b) = (random_frame_u8(n, 21, false), random_frame_u8(n, 22, false));
                let out = zoom_u8("out", p, &a, &b, (w, h));
                let inn = zoom_u8("in", 1.0 - p, &b, &a, (w, h));
                assert!(
                    out.iter().zip(&inn).all(|(x, y)| x.abs_diff(*y) <= 1),
                    "u8 p={p} {w}x{h}"
                );
                let (a, b) = (random_frame(n, 21, false), random_frame(n, 22, false));
                let out = zoom_f32("out", p, &a, &b, (w, h));
                let inn = zoom_f32("in", 1.0 - p, &b, &a, (w, h));
                near(&out, &inn, 1e-5);
            }
        }
    }
}
