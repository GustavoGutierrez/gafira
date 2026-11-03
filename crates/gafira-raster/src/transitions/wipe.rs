//! Wipe (Borrado): `to` opens along a direction, with a soft edge (ARQ-118).

use gafira_doc::contract::{
    Describe, Descriptor, ParamChoice, ParamDef, ParamKind, ParamSchema, ParamValues,
};
use gafira_doc::msg::MessageKey;

use super::guard::{Plan, copy, map_pixels, plan};
use super::mix::{weighted_f32, weighted_u8};
use super::{HandleReq, Transition};
use crate::transfer::Transfer;
use crate::{EffectCx, EffectError, ImageView, ImageViewMut};

/// The directions of the `direction` parameter, named by the way the moving edge travels.
const DIRECTIONS: &[ParamChoice] = &[
    ParamChoice {
        id: "right",
        label: MessageKey("transition.direction.right"),
    },
    ParamChoice {
        id: "left",
        label: MessageKey("transition.direction.left"),
    },
    ParamChoice {
        id: "down",
        label: MessageKey("transition.direction.down"),
    },
    ParamChoice {
        id: "up",
        label: MessageKey("transition.direction.up"),
    },
    ParamChoice {
        id: "circular",
        label: MessageKey("transition.direction.circular"),
    },
];

/// The wipe.
#[derive(Debug)]
pub struct Wipe {
    descriptor: Descriptor,
    schema: ParamSchema,
}

impl Wipe {
    /// Builds the transition.
    #[must_use]
    pub fn new() -> Self {
        Self {
            descriptor: Descriptor::new(
                "transition.wipe",
                MessageKey("glossary.transition_wipe"),
                1,
            ),
            schema: ParamSchema::new(vec![
                ParamDef {
                    id: "direction",
                    label: MessageKey("transition.wipe.direction"),
                    kind: ParamKind::Choice {
                        options: DIRECTIONS,
                        default: "right",
                    },
                    animatable: false,
                },
                ParamDef {
                    id: "softness",
                    label: MessageKey("transition.wipe.softness"),
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 1.0,
                        default: 0.0,
                    },
                    animatable: false,
                },
            ]),
        }
    }
}

impl Default for Wipe {
    fn default() -> Self {
        Self::new()
    }
}

impl Describe for Wipe {
    fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }
}

impl Transition for Wipe {
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
                let dir = values.choice("direction").unwrap_or("right");
                let softness = values.float("softness").unwrap_or(0.0);
                let (w, h) = (f64::from(from.width()), f64::from(from.height()));
                let width = from.width() as usize;
                // Circular u is |(a, b)| / sqrt(w^2 + h^2) with a = 2 cx - w and b = 2 cy - h,
                // both exact. One quotient of squares and one sqrt round once, so an exact
                // u == p stays exact; two `hypot` results would each round on their own.
                let diagonal_sq = w * w + h * h;
                // The weight of `to` at pixel i: 1 where `u < p`, softened by `softness`.
                let mask = |i: usize| -> f64 {
                    let (cx, cy) = ((i % width) as f64 + 0.5, (i / width) as f64 + 0.5);
                    let u = match dir {
                        "left" => 1.0 - cx / w,
                        "down" => cy / h,
                        "up" => 1.0 - cy / h,
                        "circular" => {
                            let (a, b) = (2.0 * cx - w, 2.0 * cy - h);
                            ((a * a + b * b) / diagonal_sq).sqrt()
                        }
                        _ => cx / w,
                    };
                    if softness == 0.0 {
                        if u < p { 1.0 } else { 0.0 }
                    } else {
                        // clamp((p (1 + s) - u) / s, 0, 1) is clamp(p + (p - u) / s, 0, 1). The
                        // quotient only matters while |p - u| <= s (beyond it the mask has
                        // saturated since 0 <= p <= 1), so it is never huge, never infinite,
                        // and there is no cancellation between `1 + s` and `p (1 + s)`.
                        let gap = p - u;
                        if gap.abs() > softness {
                            if gap > 0.0 { 1.0 } else { 0.0 }
                        } else {
                            (p + gap / softness).clamp(0.0, 1.0)
                        }
                    }
                };
                let tr = Transfer::new();
                map_pixels(
                    &from,
                    &to,
                    &mut dst,
                    |i, a, b| {
                        let m = mask(i);
                        weighted_u8(&tr, &[(a, 1.0 - m), (b, m)])
                    },
                    |i, a, b| {
                        let m = mask(i);
                        weighted_f32(&tr, &[(a, 1.0 - m), (b, m)])
                    },
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use gafira_doc::contract::{ParamError, ParamKind, ParamValue};

    use super::super::tests::{SENTINEL_F32, SENTINEL_U8, bits};
    use super::*;
    use crate::{PixelData, PixelDataMut};

    fn params(direction: &str, softness: f64) -> ParamValues {
        let mut p = ParamValues::new();
        p.set("direction", ParamValue::Choice(direction.to_owned()));
        p.set("softness", ParamValue::Float(softness));
        p
    }

    /// `from` all 0, `to` all 1, opaque, `f32`: the output channel is the mask `m`.
    fn mask(p: &ParamValues, progress: f32, w: u32, h: u32) -> Vec<f32> {
        let n = (w * h) as usize;
        let (f, g) = ([0.0f32, 0.0, 0.0, 1.0].repeat(n), [1.0f32; 4].repeat(n));
        let mut d = vec![f32::from_bits(SENTINEL_F32); n * 4];
        Wipe::new()
            .run_cpu(
                &EffectCx::whole(1.0),
                p,
                progress,
                ImageView::new(w, h, PixelData::RgbaF32(&f)).unwrap(),
                ImageView::new(w, h, PixelData::RgbaF32(&g)).unwrap(),
                ImageViewMut::new(w, h, PixelDataMut::RgbaF32(&mut d)).unwrap(),
            )
            .unwrap();
        for px in d.chunks_exact(4) {
            assert_eq!(px[3], 1.0, "opaque stays opaque");
            assert!(px[0] == px[1] && px[1] == px[2]);
        }
        d.chunks_exact(4).map(|px| px[0]).collect()
    }

    /// The mask of ARQ-118, written again here from the text of the criterion.
    fn reference(direction: &str, softness: f64, p: f64, w: u32, h: u32) -> Vec<f32> {
        let (wf, hf) = (f64::from(w), f64::from(h));
        let mut out = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let (cx, cy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
                let u = match direction {
                    "right" => cx / wf,
                    "left" => 1.0 - cx / wf,
                    "down" => cy / hf,
                    "up" => 1.0 - cy / hf,
                    _ => {
                        ((cx - wf / 2.0).powi(2) + (cy - hf / 2.0).powi(2)).sqrt()
                            / ((wf * wf + hf * hf).sqrt() / 2.0)
                    }
                };
                let m = if softness == 0.0 {
                    if u < p { 1.0 } else { 0.0 }
                } else {
                    ((p * (1.0 + softness) - u) / softness).clamp(0.0, 1.0)
                };
                out.push(m as f32);
            }
        }
        out
    }

    // 02 §4.13 acceptance, C12: id, name, handles and the two parameters of the table.
    #[test]
    fn wipe_has_the_identity_and_schema_of_the_table() {
        let t = Wipe::new();
        assert_eq!(t.descriptor().id, "transition.wipe");
        assert_eq!(t.descriptor().name.0, "glossary.transition_wipe");
        assert_eq!(t.descriptor().version, 1);
        assert_eq!(t.handles(), HandleReq::Both);
        assert!(t.params().validate_definition().is_ok());
        let defs = t.params().params();
        assert_eq!(defs.len(), 2);
        assert_eq!(
            (defs[0].id, defs[0].label.0, defs[0].animatable),
            ("direction", "transition.wipe.direction", false)
        );
        let ParamKind::Choice { options, default } = &defs[0].kind else {
            panic!("kind")
        };
        let ids: Vec<_> = options.iter().map(|o| o.id).collect();
        assert_eq!(ids, ["right", "left", "down", "up", "circular"]);
        assert_eq!(*default, "right");
        for o in *options {
            assert_eq!(o.label.0, format!("transition.direction.{}", o.id));
        }
        assert_eq!(
            (defs[1].id, defs[1].label.0, defs[1].animatable),
            ("softness", "transition.wipe.softness", false)
        );
        assert_eq!(
            defs[1].kind,
            ParamKind::Float {
                min: 0.0,
                max: 1.0,
                default: 0.0
            }
        );
    }

    // ARQ-110: a bad `direction` or `softness` is Err naming it, before anything is written.
    #[test]
    fn arq_110_wipe_rejects_bad_parameters_before_writing() {
        let cases: [(ParamValues, ParamError); 7] = [
            (
                params("diagonal", 0.0),
                ParamError::OutOfRange { param: "direction" },
            ),
            (
                params("right", -0.1),
                ParamError::OutOfRange { param: "softness" },
            ),
            (
                params("right", 1.000_001),
                ParamError::OutOfRange { param: "softness" },
            ),
            (
                params("right", f64::NAN),
                ParamError::NonFinite { param: "softness" },
            ),
            (
                params("right", f64::INFINITY),
                ParamError::NonFinite { param: "softness" },
            ),
            (
                params("right", f64::NEG_INFINITY),
                ParamError::NonFinite { param: "softness" },
            ),
            (
                {
                    let mut p = ParamValues::new();
                    p.set("softness", ParamValue::Int(1));
                    p
                },
                ParamError::WrongType { param: "softness" },
            ),
        ];
        for (p, want) in cases {
            for progress in [0.0f32, 0.4, 1.0] {
                let (f, g) = (vec![1u8; 16], vec![2u8; 16]);
                let mut d = vec![SENTINEL_U8; 16];
                let err = Wipe::new()
                    .run_cpu(
                        &EffectCx::whole(1.0),
                        &p,
                        progress,
                        ImageView::new(2, 2, PixelData::Rgba8(&f)).unwrap(),
                        ImageView::new(2, 2, PixelData::Rgba8(&g)).unwrap(),
                        ImageViewMut::new(2, 2, PixelDataMut::Rgba8(&mut d)).unwrap(),
                    )
                    .unwrap_err();
                assert_eq!(err, EffectError::Param(want));
                assert!(d.iter().all(|b| *b == SENTINEL_U8));
            }
        }
        // The valid ends of the range are accepted.
        for s in [0.0, 1.0, 0.5] {
            assert_eq!(mask(&params("right", s), 0.5, 2, 1).len(), 2);
        }
    }

    // ARQ-118 acceptance: s = 0, p = 0.5 on 4x2.
    #[test]
    fn arq_118_wipe_acceptance_rows_on_4_by_2() {
        let expect = |d: &str, to: [bool; 8]| {
            let m = mask(&params(d, 0.0), 0.5, 4, 2);
            let got: Vec<bool> = m.iter().map(|v| *v == 1.0).collect();
            assert_eq!(got, to, "{d}: {m:?}");
            assert!(m.iter().all(|v| *v == 0.0 || *v == 1.0));
        };
        let (t, f) = (true, false);
        expect("right", [t, t, f, f, t, t, f, f]);
        expect("left", [f, f, t, t, f, f, t, t]);
        expect("down", [t, t, t, t, f, f, f, f]);
        expect("up", [f, f, f, f, t, t, t, t]);
    }

    // ARQ-118: `u` is the centre of the pixel, and `u < p` is strict at s = 0 (no 0/0 there).
    #[test]
    fn arq_118_wipe_edge_is_strict_and_at_the_pixel_centre() {
        // 4 columns: u = 0.125, 0.375, 0.625, 0.875, exact in binary.
        let m = mask(&params("right", 0.0), 0.375, 4, 1);
        assert_eq!(m, [1.0, 0.0, 0.0, 0.0]);
        let m = mask(&params("right", 0.0), 0.125, 4, 1);
        assert_eq!(m, [0.0, 0.0, 0.0, 0.0]);
        let m = mask(&params("right", 0.0), 0.875, 4, 1);
        assert_eq!(m, [1.0, 1.0, 1.0, 0.0]);
        // Rows, mirrored directions, and an odd side.
        assert_eq!(
            mask(&params("left", 0.0), 0.375, 4, 1),
            [0.0, 0.0, 0.0, 1.0]
        );
        assert_eq!(
            mask(&params("down", 0.0), 0.375, 1, 4),
            [1.0, 0.0, 0.0, 0.0]
        );
        assert_eq!(mask(&params("up", 0.0), 0.375, 1, 4), [0.0, 0.0, 0.0, 1.0]);
        let m = mask(&params("right", 0.0), 0.5, 3, 1); // u = 1/6, 1/2, 5/6: the middle one is not < 0.5
        assert_eq!(m, [1.0, 0.0, 0.0]);
    }

    // ARQ-118: with softness the mask is clamp((p (1 + s) - u) / s, 0, 1).
    #[test]
    fn arq_118_wipe_soft_edge_values() {
        let near = |a: &[f32], b: &[f32]| {
            assert!(
                a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-6),
                "{a:?} vs {b:?}"
            );
        };
        near(
            &mask(&params("right", 1.0), 0.5, 4, 1),
            &[0.875, 0.625, 0.375, 0.125],
        );
        near(
            &mask(&params("right", 0.5), 0.5, 4, 1),
            &[1.0, 0.75, 0.25, 0.0],
        );
        near(
            &mask(&params("left", 1.0), 0.5, 4, 1),
            &[0.125, 0.375, 0.625, 0.875],
        );
        near(&mask(&params("down", 1.0), 0.25, 1, 2), &[0.25, 0.0]);
    }

    // ARQ-118: every direction and a range of p and s against the mask written from the text.
    #[test]
    fn arq_118_wipe_matches_the_criterion_for_every_direction() {
        for d in ["right", "left", "down", "up", "circular"] {
            for s in [0.0, 0.2, 1.0, 0.5] {
                for p in [0.05f32, 0.3, 0.5, 0.71, 0.95] {
                    for (w, h) in [(7u32, 5u32), (4, 9), (1, 6), (6, 1)] {
                        let got = mask(&params(d, s), p, w, h);
                        let want = reference(d, s, f64::from(p), w, h);
                        assert!(
                            got.iter().zip(&want).all(|(a, b)| (a - b).abs() < 1e-6),
                            "{d} s={s} p={p} {w}x{h}: {got:?} vs {want:?}"
                        );
                    }
                }
            }
        }
    }

    // ARQ-118: the circular wipe measures from the centre to half the diagonal.
    #[test]
    fn arq_118_wipe_circular_centre_and_half_diagonal() {
        // 4x4: centre (2, 2), half diagonal sqrt(32) / 2. Corner pixel (0, 0): u = 0.75.
        let at = |p: f32| mask(&params("circular", 0.0), p, 4, 4);
        let m = at(0.7);
        assert_eq!(m[0], 0.0, "corner is still `from` at 0.7");
        assert_eq!(m[5], 1.0, "(1, 1) has u = 0.25");
        assert_eq!(at(0.8)[0], 1.0, "corner is `to` at 0.8");
        // Non-square: 8x2, centre (4, 1); pixel (0, 0) is at distance sqrt(3.5^2 + 0.5^2).
        let d = (3.5f64 * 3.5 + 0.25).sqrt() / (68.0f64.sqrt() / 2.0);
        let m = mask(&params("circular", 0.0), (d - 0.001) as f32, 8, 2);
        assert_eq!(m[0], 0.0);
        let m = mask(&params("circular", 0.0), (d + 0.001) as f32, 8, 2);
        assert_eq!(m[0], 1.0);
        // No pixel of `from` is left at p near 1, corners included.
        for s in [0.0, 0.3, 1.0] {
            let m = mask(&params("circular", s), 0.9999, 9, 5);
            assert!(m.iter().all(|v| *v == 1.0), "s={s}: {m:?}");
        }
    }

    // ARQ-118: softness defaults to 0 and direction to `right`.
    #[test]
    fn arq_118_wipe_defaults_are_right_and_hard() {
        let none = mask(&ParamValues::new(), 0.375, 4, 1);
        assert_eq!(none, [1.0, 0.0, 0.0, 0.0]);
    }

    // ARQ-25, ARQ-118: the ends are byte-exact in every direction, also with softness 1.
    #[test]
    fn arq_118_wipe_ends_are_exact_with_any_softness() {
        let (f, g) = (
            super::super::tests::frame_u8(6, 3),
            super::super::tests::frame_u8(6, 80),
        );
        let (ff, gf) = (
            super::super::tests::frame_f32(6, 1.0),
            super::super::tests::frame_f32(6, 4.0),
        );
        for d in ["right", "left", "down", "up", "circular"] {
            for s in [0.0, 1.0] {
                for (progress, is_to) in [(0.0f32, false), (-0.5, false), (1.0, true), (1.5, true)]
                {
                    let mut du = vec![SENTINEL_U8; 24];
                    Wipe::new()
                        .run_cpu(
                            &EffectCx::whole(1.0),
                            &params(d, s),
                            progress,
                            ImageView::new(3, 2, PixelData::Rgba8(&f)).unwrap(),
                            ImageView::new(3, 2, PixelData::Rgba8(&g)).unwrap(),
                            ImageViewMut::new(3, 2, PixelDataMut::Rgba8(&mut du)).unwrap(),
                        )
                        .unwrap();
                    assert_eq!(
                        du,
                        if is_to { g.clone() } else { f.clone() },
                        "{d} s={s} p={progress}"
                    );
                    let mut df = vec![f32::from_bits(SENTINEL_F32); 24];
                    Wipe::new()
                        .run_cpu(
                            &EffectCx::whole(1.0),
                            &params(d, s),
                            progress,
                            ImageView::new(3, 2, PixelData::RgbaF32(&ff)).unwrap(),
                            ImageView::new(3, 2, PixelData::RgbaF32(&gf)).unwrap(),
                            ImageViewMut::new(3, 2, PixelDataMut::RgbaF32(&mut df)).unwrap(),
                        )
                        .unwrap();
                    assert_eq!(
                        bits(&df),
                        bits(if is_to { &gf } else { &ff }),
                        "{d} s={s} p={progress}"
                    );
                }
            }
        }
    }

    // ARQ-118: with softness 1 the result stays within 1/255 of its end at p = 0.0001 and 0.9999.
    #[test]
    fn arq_118_wipe_soft_stays_within_1_255_of_the_ends_near_them() {
        let (f, g) = (
            super::super::tests::frame_u8(20, 3),
            super::super::tests::frame_u8(20, 80),
        );
        for d in ["right", "left", "down", "up", "circular"] {
            for (progress, end) in [(0.0001f32, &f), (0.9999, &g)] {
                let mut du = vec![0u8; 80];
                Wipe::new()
                    .run_cpu(
                        &EffectCx::whole(1.0),
                        &params(d, 1.0),
                        progress,
                        ImageView::new(5, 4, PixelData::Rgba8(&f)).unwrap(),
                        ImageView::new(5, 4, PixelData::Rgba8(&g)).unwrap(),
                        ImageViewMut::new(5, 4, PixelDataMut::Rgba8(&mut du)).unwrap(),
                    )
                    .unwrap();
                for (a, b) in du.iter().zip(end.iter()) {
                    assert!(a.abs_diff(*b) <= 1, "{d} p={progress}: {a} vs {b}");
                }
            }
        }
    }

    // ARQ-118, P15 item 8: at s = 0 each pixel is `from` or `to` exactly, ghost colour and all;
    // a soft pixel mixes premultiplied, so a transparent `from` gives `to` its own alpha share.
    #[test]
    fn arq_118_wipe_hard_edge_copies_pixels_and_soft_edge_mixes_premultiplied() {
        let f = [200u8, 10, 5, 0].repeat(4); // transparent with colour
        let g = [255u8, 255, 255, 255].repeat(4);
        let mut d = vec![0u8; 16];
        Wipe::new()
            .run_cpu(
                &EffectCx::whole(1.0),
                &params("right", 0.0),
                0.5,
                ImageView::new(4, 1, PixelData::Rgba8(&f)).unwrap(),
                ImageView::new(4, 1, PixelData::Rgba8(&g)).unwrap(),
                ImageViewMut::new(4, 1, PixelDataMut::Rgba8(&mut d)).unwrap(),
            )
            .unwrap();
        assert_eq!(&d[..8], &g[..8]);
        assert_eq!(&d[8..], &f[8..]);
        // Softness 1 at p = 0.5, x = 3: m = 0.125, so alpha is about 32 and the colour is white.
        let mut d = vec![0u8; 16];
        Wipe::new()
            .run_cpu(
                &EffectCx::whole(1.0),
                &params("right", 1.0),
                0.5,
                ImageView::new(4, 1, PixelData::Rgba8(&f)).unwrap(),
                ImageView::new(4, 1, PixelData::Rgba8(&g)).unwrap(),
                ImageViewMut::new(4, 1, PixelDataMut::Rgba8(&mut d)).unwrap(),
            )
            .unwrap();
        assert_eq!(d[15], 32);
        assert!(d[12] <= d[15], "premultiplied: {:?}", &d[12..]);
    }

    /// Pixel values of `mask` at the given softness for one row.
    fn row(direction: &str, softness: f64, p: f32, w: u32, h: u32) -> Vec<f32> {
        mask(&params(direction, softness), p, w, h)
    }

    // ARQ-118: at tiny softness the mask is p exactly where u == p, 1 where u is below p by more
    // than the softness allows, 0 where it is above. Hand-worked on 8 columns: u = (x + 0.5) / 8,
    // so x = 3 has u = 0.4375 = p, x = 2 has u = 0.3125, x = 4 has u = 0.5625. Down to the
    // subnormals, where dividing first would go through infinity.
    #[test]
    fn arq_118_wipe_tiny_softness_keeps_the_edge_pixel_at_p() {
        let softnesses = [
            1.0e-2, 1.0e-4, 1.0e-6, 1.0e-8, 1.0e-10, 1.0e-12, 1.0e-14, 3.0e-16, 1.0e-16, 1.0e-20,
            1.0e-50, 1.0e-150, 1.0e-300, 1.0e-308, 1.0e-310, 5.0e-324,
        ];
        for s in softnesses {
            let got = row("right", s, 0.4375, 8, 1);
            assert_eq!(
                got,
                [1.0, 1.0, 1.0, 0.4375, 0.0, 0.0, 0.0, 0.0],
                "right s={s}"
            );
            // Mirrored: left has u = 1 - (x + 0.5) / 8, so x = 4 has u = 0.4375.
            let got = row("left", s, 0.4375, 8, 1);
            assert_eq!(
                got,
                [0.0, 0.0, 0.0, 0.0, 0.4375, 1.0, 1.0, 1.0],
                "left s={s}"
            );
            let got = row("down", s, 0.4375, 1, 8);
            assert_eq!(
                got,
                [1.0, 1.0, 1.0, 0.4375, 0.0, 0.0, 0.0, 0.0],
                "down s={s}"
            );
            let got = row("up", s, 0.4375, 1, 8);
            assert_eq!(got, [0.0, 0.0, 0.0, 0.0, 0.4375, 1.0, 1.0, 1.0], "up s={s}");
        }
        // Another p and side: 2 columns have u = 0.25 and 0.75.
        for s in softnesses {
            assert_eq!(row("right", s, 0.25, 2, 1), [0.25, 0.0], "s={s}");
            assert_eq!(row("right", s, 0.75, 2, 1), [1.0, 0.75], "s={s}");
        }
        // Circular: 2x2 has all four u equal to 0.5 (the corner distance over half the diagonal).
        for s in softnesses {
            assert_eq!(row("circular", s, 0.5, 2, 2), [0.5; 4], "circular s={s}");
        }
    }

    // ARQ-118: the mask against exact integer arithmetic. With p = P / 2^30, u = U / 2^30 and
    // s = 2^-k, clamp(p + (p - u) / s, 0, 1) is the integer clamp(P + (P - U) 2^k, 0, 2^30) / 2^30,
    // which an f32 holds exactly after one rounding. Every power-of-two softness, every pixel.
    #[test]
    fn arq_118_wipe_mask_equals_exact_integer_arithmetic_for_dyadic_softness() {
        const D: i128 = 1 << 30;
        for k in 0..=60u32 {
            let s = 0.5f64.powi(k as i32);
            for p_num in [1i128, 5, 8, 9, 15, 16, 17, 31] {
                // p = p_num / 32 is an f32 exactly.
                let p = (p_num as f32) / 32.0;
                let big_p = p_num * (D / 32);
                for w in [4u32, 8, 16, 32] {
                    let got = row("right", s, p, w, 1);
                    for x in 0..w {
                        let big_u = i128::from(2 * x + 1) * (D / i128::from(2 * w));
                        let m = (big_p + (big_p - big_u) * (1i128 << k)).clamp(0, D);
                        let want = (m as f64 / D as f64) as f32;
                        assert_eq!(
                            got[x as usize].to_bits(),
                            want.to_bits(),
                            "k={k} p={p} w={w} x={x}: {} vs {want}",
                            got[x as usize]
                        );
                    }
                }
            }
        }
    }

    /// Pixels of a w x h image whose circular `u` is exactly `p_num / den`, by integer arithmetic:
    /// u^2 = (a^2 + b^2) / (w^2 + h^2) with a = 2x + 1 - w and b = 2y + 1 - h, so u = p exactly
    /// when (a^2 + b^2) den^2 = p_num^2 (w^2 + h^2).
    fn circular_edge_pixels(w: u32, h: u32, p_num: i128, den: i128) -> Vec<usize> {
        let mut out = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let a = i128::from(2 * x + 1) - i128::from(w);
                let b = i128::from(2 * y + 1) - i128::from(h);
                let d = i128::from(w) * i128::from(w) + i128::from(h) * i128::from(h);
                if (a * a + b * b) * den * den == p_num * p_num * d {
                    out.push((y * w + x) as usize);
                }
            }
        }
        out
    }

    // ARQ-118, ARQ-110, ARQ-111: on a circular Wipe a pixel with u exactly p keeps the rule of the
    // criterion: 0 when hard (strict u < p) and p for any softness above 0. Hand-worked: 8x8
    // pixel (0, 0) has u = 3.5 sqrt 2 / (4 sqrt 2) = 7/8, so p = 0.875 gives 0 hard and 0.875 soft.
    #[test]
    fn arq_118_wipe_circular_exact_edge_pixels_keep_the_rule() {
        let hand = row("circular", 0.0, 0.875, 8, 8);
        assert_eq!(hand[0], 0.0, "hard 8x8 p 0.875 pixel (0,0)");
        for s in [1.0e-16, 1.0e-14, 1.0e-12, 1.0e-10, 1.0e-3] {
            assert_eq!(row("circular", s, 0.875, 8, 8)[0], 0.875, "8x8 s={s}");
        }
        let mut found = 0;
        for n in [4u32, 8, 16, 32, 64] {
            for j in 1..64i128 {
                let p = (j as f32) / 64.0;
                let edge = circular_edge_pixels(n, n, j, 64);
                if edge.is_empty() {
                    continue;
                }
                let hard = row("circular", 0.0, p, n, n);
                let soft = row("circular", 1.0e-16, p, n, n);
                for &i in &edge {
                    found += 1;
                    assert_eq!(hard[i], 0.0, "hard n={n} p={p} i={i}");
                    assert_eq!(soft[i], p, "soft n={n} p={p} i={i}");
                }
            }
        }
        assert!(found >= 88, "the scan finds the exact-edge pixels: {found}");
    }

    // ARQ-118: the same on a real frame size, 1080x1080 at p = 3/8 (the report's example).
    #[test]
    fn arq_118_wipe_circular_exact_edge_on_1080_square() {
        let edge = circular_edge_pixels(1080, 1080, 3, 8);
        assert!(!edge.is_empty());
        let hard = row("circular", 0.0, 0.375, 1080, 1080);
        let soft = row("circular", 1.0e-16, 0.375, 1080, 1080);
        for &i in &edge {
            assert_eq!(hard[i], 0.0, "hard i={i}");
            assert_eq!(soft[i], 0.375, "soft i={i}");
        }
    }
}
