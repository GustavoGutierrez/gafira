//! Slide (Deslizar): `to` enters over a still `from` (ARQ-118).

use gafira_doc::contract::{
    Describe, Descriptor, ParamChoice, ParamDef, ParamKind, ParamSchema, ParamValues,
};
use gafira_doc::msg::MessageKey;

use super::guard::{Plan, copy, plan, render};
use super::{HandleReq, Transition};
use crate::{EffectCx, EffectError, ImageView, ImageViewMut};

/// The directions of the `direction` parameter, named by the way the moving edge travels.
const DIRECTIONS: &[ParamChoice] = &[
    ParamChoice {
        id: "left",
        label: MessageKey("transition.direction.left"),
    },
    ParamChoice {
        id: "right",
        label: MessageKey("transition.direction.right"),
    },
    ParamChoice {
        id: "up",
        label: MessageKey("transition.direction.up"),
    },
    ParamChoice {
        id: "down",
        label: MessageKey("transition.direction.down"),
    },
];

/// The sliding transition.
#[derive(Debug)]
pub struct Slide {
    descriptor: Descriptor,
    schema: ParamSchema,
}

impl Slide {
    /// Builds the transition.
    #[must_use]
    pub fn new() -> Self {
        Self {
            descriptor: Descriptor::new(
                "transition.slide",
                MessageKey("glossary.transition_slide"),
                1,
            ),
            schema: ParamSchema::new(vec![ParamDef {
                id: "direction",
                label: MessageKey("transition.slide.direction"),
                kind: ParamKind::Choice {
                    options: DIRECTIONS,
                    default: "left",
                },
                animatable: false,
            }]),
        }
    }
}

impl Default for Slide {
    fn default() -> Self {
        Self::new()
    }
}

impl Describe for Slide {
    fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }
}

impl Transition for Slide {
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
                let dir = values.choice("direction").unwrap_or("left");
                let (w, h) = (from.width() as usize, from.height() as usize);
                let side = if matches!(dir, "up" | "down") { h } else { w };
                // d = round((1 - p) * side), in f64, halves away from zero.
                let d = (((1.0 - p) * side as f64).round() as usize).min(side);
                // Where the screen pixel comes from: `Some((x, y))` of `to`, or `from` as it is.
                let source = |x: usize, y: usize| match dir {
                    "left" if x >= d => Some((x - d, y)),
                    "right" if x < w - d => Some((x + d, y)),
                    "up" if y >= d => Some((x, y - d)),
                    "down" if y < h - d => Some((x, y + d)),
                    _ => None,
                };
                render(
                    &from,
                    &to,
                    &mut dst,
                    |x, y, f| match source(x, y) {
                        Some((sx, sy)) => f.px(true, sx, sy),
                        None => f.px(false, x, y),
                    },
                    |x, y, f| match source(x, y) {
                        Some((sx, sy)) => f.px(true, sx, sy),
                        None => f.px(false, x, y),
                    },
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use gafira_doc::contract::{ParamError, ParamValue};

    use super::super::tests::{SENTINEL_F32, SENTINEL_U8, bits, frame_f32};
    use super::*;
    use crate::{PixelData, PixelDataMut};

    fn dir(d: &str) -> ParamValues {
        let mut p = ParamValues::new();
        p.set("direction", ParamValue::Choice(d.to_owned()));
        p
    }

    /// Pixel (x, y) of a frame whose bytes are all distinct: the pixel index and the frame id.
    fn unique(w: u32, h: u32, id: u8) -> Vec<u8> {
        (0..w * h)
            .flat_map(|i| [i as u8, id, (i >> 8) as u8, 255])
            .collect()
    }

    fn slide_u8(d: &str, progress: f32, from: &[u8], to: &[u8], size: (u32, u32)) -> Vec<u8> {
        let mut dst = vec![SENTINEL_U8; from.len()];
        Slide::new()
            .run_cpu(
                &EffectCx::whole(1.0),
                &dir(d),
                progress,
                ImageView::new(size.0, size.1, PixelData::Rgba8(from)).unwrap(),
                ImageView::new(size.0, size.1, PixelData::Rgba8(to)).unwrap(),
                ImageViewMut::new(size.0, size.1, PixelDataMut::Rgba8(&mut dst)).unwrap(),
            )
            .unwrap();
        dst
    }

    fn px(v: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * w + x) * 4) as usize;
        [v[i], v[i + 1], v[i + 2], v[i + 3]]
    }

    // 02 §4.13 acceptance, C12: id, name, handles and the `direction` parameter of the table.
    #[test]
    fn slide_has_the_identity_and_schema_of_the_table() {
        let t = Slide::new();
        assert_eq!(t.descriptor().id, "transition.slide");
        assert_eq!(t.descriptor().name.0, "glossary.transition_slide");
        assert_eq!(t.descriptor().version, 1);
        assert_eq!(t.handles(), HandleReq::Both);
        assert!(t.params().validate_definition().is_ok());
        let defs = t.params().params();
        assert_eq!(defs.len(), 1);
        assert_eq!(
            (defs[0].id, defs[0].label.0, defs[0].animatable),
            ("direction", "transition.slide.direction", false)
        );
        let ParamKind::Choice { options, default } = &defs[0].kind else {
            panic!("kind")
        };
        let ids: Vec<_> = options.iter().map(|o| o.id).collect();
        assert_eq!(ids, ["left", "right", "up", "down"]);
        assert_eq!(*default, "left");
        for o in *options {
            assert_eq!(o.label.0, format!("transition.direction.{}", o.id));
        }
    }

    // ARQ-118: `diagonal` is Err naming `direction`, and nothing is written.
    #[test]
    fn arq_110_slide_rejects_an_unknown_direction() {
        let (f, g) = (unique(2, 2, 1), unique(2, 2, 2));
        for progress in [0.0f32, 0.3, 1.0] {
            let mut d = vec![SENTINEL_U8; 16];
            let err = Slide::new()
                .run_cpu(
                    &EffectCx::whole(1.0),
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

    // ARQ-118 acceptance: 10x4 at 0.25. Horizontal d = 8, vertical d = 3.
    #[test]
    fn arq_118_slide_acceptance_rows_on_10_by_4() {
        let (f, g) = (unique(10, 4, 1), unique(10, 4, 2));
        // left: columns 8 and 9 are columns 0 and 1 of `to`; the rest is `from`.
        let out = slide_u8("left", 0.25, &f, &g, (10, 4));
        for y in 0..4 {
            for x in 0..10 {
                let want = if x >= 8 {
                    px(&g, 10, x - 8, y)
                } else {
                    px(&f, 10, x, y)
                };
                assert_eq!(px(&out, 10, x, y), want, "left ({x},{y})");
            }
        }
        // right: columns 0 and 1 are columns 8 and 9 of `to`.
        let out = slide_u8("right", 0.25, &f, &g, (10, 4));
        for y in 0..4 {
            for x in 0..10 {
                let want = if x < 2 {
                    px(&g, 10, x + 8, y)
                } else {
                    px(&f, 10, x, y)
                };
                assert_eq!(px(&out, 10, x, y), want, "right ({x},{y})");
            }
        }
        // up: with d = 3, row 3 is row 0 of `to`.
        let out = slide_u8("up", 0.25, &f, &g, (10, 4));
        for y in 0..4 {
            for x in 0..10 {
                let want = if y >= 3 {
                    px(&g, 10, x, y - 3)
                } else {
                    px(&f, 10, x, y)
                };
                assert_eq!(px(&out, 10, x, y), want, "up ({x},{y})");
            }
        }
        // down: row 0 is row 3 of `to`.
        let out = slide_u8("down", 0.25, &f, &g, (10, 4));
        for y in 0..4 {
            for x in 0..10 {
                let want = if y < 1 {
                    px(&g, 10, x, y + 3)
                } else {
                    px(&f, 10, x, y)
                };
                assert_eq!(px(&out, 10, x, y), want, "down ({x},{y})");
            }
        }
    }

    // ARQ-118: d = round((1 - p) * side) in f64, halves away from zero.
    #[test]
    fn arq_118_slide_offset_rounds_half_away_from_zero() {
        let (f, g) = (unique(10, 1, 1), unique(10, 1, 2));
        // (progress, expected d) on a side of 10; `left` shows `to` from column d.
        let cases: [(f32, usize); 8] = [
            (0.75, 3),    // 2.5 rounds to 3, not to the even 2
            (0.25, 8),    // 7.5 to 8
            (0.74, 3),    // 2.6 up
            (0.76, 2),    // 2.4 down
            (0.95, 1),    // 0.5 to 1
            (0.9999, 0),  // everything is `to`
            (0.0001, 10), // everything is `from`
            (0.5, 5),
        ];
        for (p, d) in cases {
            let out = slide_u8("left", p, &f, &g, (10, 1));
            let first_to = (0..10)
                .find(|x| px(&out, 10, *x, 0) == px(&g, 10, 0, 0))
                .unwrap_or(10);
            assert_eq!(first_to as usize, d, "p={p}");
            for x in 0..10u32 {
                let want = if (x as usize) >= d {
                    px(&g, 10, x - d as u32, 0)
                } else {
                    px(&f, 10, x, 0)
                };
                assert_eq!(px(&out, 10, x, 0), want, "p={p} x={x}");
            }
        }
        // A side of 1: d = 1 below one half is all `from`, d = 0 above is all `to`.
        let (f1, g1) = (unique(1, 1, 1), unique(1, 1, 2));
        assert_eq!(slide_u8("left", 0.4, &f1, &g1, (1, 1)), f1);
        assert_eq!(slide_u8("left", 0.6, &f1, &g1, (1, 1)), g1);
        assert_eq!(slide_u8("up", 0.4, &f1, &g1, (1, 1)), f1);
        assert_eq!(slide_u8("up", 0.6, &f1, &g1, (1, 1)), g1);
    }

    // ARQ-118: Slide replaces pixels: a transparent `to` clears `from` where it enters, and a
    // translucent one does not blend with it. `from` never moves.
    #[test]
    fn arq_118_slide_replaces_and_does_not_composite() {
        let f = [200u8, 100, 50, 255].repeat(4);
        let g = [9u8, 8, 7, 0].repeat(4);
        for d in ["left", "right"] {
            let out = slide_u8(d, 0.5, &f, &g, (4, 1));
            let cleared: Vec<_> = (0..4).map(|x| px(&out, 4, x, 0)).collect();
            let n_to = cleared.iter().filter(|p| **p == [9, 8, 7, 0]).count();
            let n_from = cleared
                .iter()
                .filter(|p| **p == [200, 100, 50, 255])
                .count();
            assert_eq!((n_to, n_from), (2, 2), "{d}: {cleared:?}");
        }
        let h = [30u8, 20, 10, 64].repeat(4);
        let out = slide_u8("up", 0.5, &f.repeat(1), &h, (1, 4));
        assert!(
            px(&out, 1, 0, 3) == [30, 20, 10, 64] && px(&out, 1, 0, 0) == [200, 100, 50, 255],
            "{out:?}"
        );
    }

    // Both formats: f32 frames keep awkward values bit for bit (no conversion on a copy).
    #[test]
    fn arq_118_slide_copies_f32_pixels_bit_for_bit() {
        let (f, g) = (frame_f32(6, 1.0), frame_f32(6, 4.0));
        let mut d = vec![f32::from_bits(SENTINEL_F32); 24];
        Slide::new()
            .run_cpu(
                &EffectCx::whole(1.0),
                &dir("left"),
                0.5,
                ImageView::new(3, 2, PixelData::RgbaF32(&f)).unwrap(),
                ImageView::new(3, 2, PixelData::RgbaF32(&g)).unwrap(),
                ImageViewMut::new(3, 2, PixelDataMut::RgbaF32(&mut d)).unwrap(),
            )
            .unwrap();
        // 0.5 * 3 = 1.5 -> d = 2: columns 0, 1 from `from`, column 2 is column 0 of `to`.
        for y in 0..2usize {
            for x in 0..3usize {
                let i = (y * 3 + x) * 4;
                let src: &[f32] = if x >= 2 {
                    &g[(y * 3 + x - 2) * 4..][..4]
                } else {
                    &f[i..i + 4]
                };
                assert_eq!(bits(&d[i..i + 4]), bits(src), "({x},{y})");
            }
        }
    }

    // ARQ-118: d is computed in f64 ("en f64"); in f32 these three would give 5, 10 and 5.
    #[test]
    fn arq_118_slide_offset_is_computed_in_f64() {
        // (side, progress, expected d): (1 - p) * side is 4.4999999, 9.4999999 and 4.5000000.
        let cases: [(u32, f32, u32); 4] = [(5, 0.1, 4), (10, 0.05, 9), (10, 0.55, 4), (7, 0.3, 5)];
        for (side, p, d) in cases {
            let (f, g) = (unique(side, 1, 1), unique(side, 1, 2));
            let out = slide_u8("left", p, &f, &g, (side, 1));
            for x in 0..side {
                let want = if x >= d {
                    px(&g, side, x - d, 0)
                } else {
                    px(&f, side, x, 0)
                };
                assert_eq!(px(&out, side, x, 0), want, "side={side} p={p} d={d} x={x}");
            }
            // The same on the vertical axis.
            let (f, g) = (unique(1, side, 1), unique(1, side, 2));
            let out = slide_u8("up", p, &f, &g, (1, side));
            for y in 0..side {
                let want = if y >= d {
                    px(&g, 1, 0, y - d)
                } else {
                    px(&f, 1, 0, y)
                };
                assert_eq!(px(&out, 1, 0, y), want, "up side={side} p={p} d={d} y={y}");
            }
        }
    }
}
