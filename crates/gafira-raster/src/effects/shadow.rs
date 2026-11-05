//! Paralela, the parallel drop shadow (FD-35, ARQ-120).

use gafira_color::{Color, ColorSpaceId};
use gafira_doc::BlendMode;
use gafira_doc::contract::{Describe, Descriptor, ParamDef, ParamKind, ParamSchema, ParamValues};
use gafira_doc::msg::MessageKey;
use gafira_geom::IRect;

use super::blur::{blur_shifted, half_width};
use super::{Cost, Effect};
use crate::{
    BufferError, EffectCx, EffectError, ImageView, ImageViewMut, PixelData, PixelDataMut,
    PixelFormat, blend_f32, blend_u8, premultiply_u8,
};

/// Work units per pixel besides the two blur passes: alpha plane, tint and composite.
const COMPOSITE_UNITS: u64 = 8;

const DEFAULT_COLOR: Color = Color::Srgb(gafira_color::Rgba {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
});

/// A float parameter: id, label key, range and default. All animatable.
fn float(id: &'static str, label: &'static str, min: f64, max: f64, default: f64) -> ParamDef {
    ParamDef {
        id,
        label: MessageKey(label),
        kind: ParamKind::Float { min, max, default },
        animatable: true,
    }
}

/// The shadow of an element: its alpha moved, blurred and tinted, with the element over it.
#[derive(Debug)]
pub struct ParallelShadow {
    descriptor: Descriptor,
    schema: ParamSchema,
}

impl ParallelShadow {
    /// Builds the effect.
    #[must_use]
    pub fn new() -> Self {
        Self {
            descriptor: Descriptor::new(
                "effect.shadow.parallel",
                MessageKey("glossary.effect_shadow_parallel"),
                1,
            ),
            schema: ParamSchema::new(vec![
                float("offset_x", "effect.shadow.offset_x", -500.0, 500.0, 4.0),
                float("offset_y", "effect.shadow.offset_y", -500.0, 500.0, 4.0),
                float("blur", "effect.shadow.blur", 0.0, 100.0, 8.0),
                float("opacity", "effect.shadow.opacity", 0.0, 100.0, 50.0),
                ParamDef {
                    id: "color",
                    label: MessageKey("effect.shadow.color"),
                    kind: ParamKind::Color {
                        default: DEFAULT_COLOR,
                    },
                    animatable: true,
                },
            ]),
        }
    }
}

impl Default for ParallelShadow {
    fn default() -> Self {
        Self::new()
    }
}

impl Describe for ParallelShadow {
    fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }
}

/// The geometry of the shadow in device pixels.
struct Geometry {
    /// Offset in whole device pixels, half away from zero.
    dx: i64,
    dy: i64,
    /// CSS radius in device pixels.
    radius: f64,
}

/// A number of `p` that is finite, or 0. `roi` and `cost_hint` take what they are given.
fn number(p: &ParamValues, id: &str) -> f64 {
    p.float(id).filter(|v| v.is_finite()).unwrap_or(0.0)
}

impl Geometry {
    /// Reads `p` with the scale of `cx`. A non-finite scale counts as 0, with no meaning: only
    /// `run_cpu` rejects it, before it gets here.
    fn new(cx: &EffectCx, p: &ParamValues) -> Self {
        let scale = if cx.scale.is_finite() {
            f64::from(cx.scale)
        } else {
            0.0
        };
        // `as` saturates, so even f32::MAX times 500 stays an integer.
        let device = |id: &str| (number(p, id) * scale).round().clamp(-4.0e9, 4.0e9) as i64;
        Self {
            dx: device("offset_x"),
            dy: device("offset_y"),
            radius: (number(p, "blur") * scale).max(0.0),
        }
    }

    /// Taps of the blur on each side.
    fn taps(&self) -> usize {
        half_width(self.radius)
    }
}

/// An `i64` as `i32`, cut at the ends.
fn to_i32(v: i64) -> i32 {
    v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

impl Effect for ParallelShadow {
    fn params(&self) -> &ParamSchema {
        &self.schema
    }

    fn roi(&self, cx: &EffectCx, p: &ParamValues, out: IRect) -> IRect {
        if out.is_empty() {
            return out;
        }
        let g = Geometry::new(cx, p);
        let k = g.taps() as i64;
        // The shadow at pixel q is the blur of the element at q - offset, +- k.
        let (x0, x1) = (i64::from(out.x0) - g.dx - k, i64::from(out.x1) - g.dx + k);
        let (y0, y1) = (i64::from(out.y0) - g.dy - k, i64::from(out.y1) - g.dy + k);
        IRect::new(
            to_i32(x0.min(i64::from(out.x0))),
            to_i32(y0.min(i64::from(out.y0))),
            to_i32(x1.max(i64::from(out.x1))),
            to_i32(y1.max(i64::from(out.y1))),
        )
    }

    fn cost_hint(&self, cx: &EffectCx, p: &ParamValues, px: u64) -> Cost {
        let taps = 2 * Geometry::new(cx, p).taps() as u64 + 1;
        Cost {
            units: px.saturating_mul(2 * taps + COMPOSITE_UNITS),
        }
    }

    fn run_cpu(
        &self,
        cx: &EffectCx,
        p: &ParamValues,
        src: ImageView<'_>,
        mut dst: ImageViewMut<'_>,
    ) -> Result<(), EffectError> {
        // 1. `cx.scale` is finite.
        if !cx.scale.is_finite() {
            return Err(BufferError::NonFinite {
                field: "cx.scale",
                index: 0,
            }
            .into());
        }
        // 2. The parameters fit the schema (this also rejects NaN and infinities).
        let values = self.schema.resolve(p)?;
        // 3. Formats: Rgba8 or RgbaF32, the same on both.
        let format = src.data().format();
        if format == PixelFormat::Alpha8 {
            return Err(EffectError::UnsupportedFormat { format });
        }
        if dst.data().format() != format {
            return Err(EffectError::FormatMismatch {
                field: "dst",
                expected: format,
                actual: dst.data().format(),
            });
        }
        // 4. Dimensions.
        let size = (src.width(), src.height());
        if (dst.width(), dst.height()) != size {
            return Err(EffectError::SizeMismatch {
                field: "dst",
                expected: size,
                actual: (dst.width(), dst.height()),
            });
        }
        // 5. The f32 pixels that will be read are finite.
        if let PixelData::RgbaF32(px) = src.data()
            && let Some(index) = px.iter().position(|v| !v.is_finite())
        {
            return Err(BufferError::NonFinite {
                field: "src",
                index,
            }
            .into());
        }
        // 6. The colour in the format of the pixels, without overflow.
        let color = values.color("color").unwrap_or(DEFAULT_COLOR);
        let tint = Tint::new(color, format)?;
        // 7. Size 0 is Ok.
        let (w, h) = (size.0 as usize, size.1 as usize);
        if w == 0 || h == 0 {
            return Ok(());
        }
        let g = Geometry::new(cx, &values);
        let opacity = number(&values, "opacity");
        match (src.data(), dst.data_mut()) {
            (PixelData::Rgba8(s), PixelDataMut::Rgba8(d)) => {
                // Alpha in 0..=255 as exact f32 integers: with no blur they come back unchanged.
                let alpha: Vec<f32> = s.chunks_exact(4).map(|px| f32::from(px[3])).collect();
                let blurred = blur_shifted(&alpha, w, h, -g.dx, -g.dy, g.radius);
                for ((px, out), cover) in s.chunks_exact(4).zip(d.chunks_exact_mut(4)).zip(&blurred)
                {
                    let sa = (f64::from(*cover) * opacity * tint.alpha / 100.0 + 0.5)
                        .floor()
                        .clamp(0.0, 255.0) as u8;
                    let element = [px[0], px[1], px[2], px[3]];
                    let shadow = [
                        premultiply_u8(tint.bytes[0], sa),
                        premultiply_u8(tint.bytes[1], sa),
                        premultiply_u8(tint.bytes[2], sa),
                        sa,
                    ];
                    out.copy_from_slice(&if sa == 0 {
                        element
                    } else {
                        blend_u8(BlendMode::Normal, shadow, element)
                    });
                }
                Ok(())
            }
            (PixelData::RgbaF32(s), PixelDataMut::RgbaF32(d)) => {
                let alpha: Vec<f32> = s.chunks_exact(4).map(|px| px[3].clamp(0.0, 1.0)).collect();
                let blurred = blur_shifted(&alpha, w, h, -g.dx, -g.dy, g.radius);
                // Built apart so an overflowing result leaves `dst` as it was.
                let mut result = vec![0.0f32; s.len()];
                for ((px, out), cover) in s
                    .chunks_exact(4)
                    .zip(result.chunks_exact_mut(4))
                    .zip(&blurred)
                {
                    let sa =
                        (f64::from(*cover) * opacity * tint.alpha / 100.0).clamp(0.0, 1.0) as f32;
                    let element = [px[0], px[1], px[2], px[3]];
                    let shadow = [
                        tint.linear[0] * sa,
                        tint.linear[1] * sa,
                        tint.linear[2] * sa,
                        sa,
                    ];
                    out.copy_from_slice(&if sa == 0.0 {
                        element
                    } else {
                        blend_f32(BlendMode::Normal, shadow, element)?
                    });
                }
                d.copy_from_slice(&result);
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

/// The colour parameter in what each format needs: straight bytes and straight linear light,
/// and its alpha, which scales the shadow.
struct Tint {
    bytes: [u8; 3],
    linear: [f32; 3],
    alpha: f64,
}

impl Tint {
    fn new(c: Color, format: PixelFormat) -> Result<Self, EffectError> {
        let overflow = |index: usize| -> EffectError {
            BufferError::NonFinite {
                field: "color",
                index,
            }
            .into()
        };
        let check = |v: [f32; 3]| match v.iter().position(|x| !x.is_finite()) {
            Some(i) => Err(overflow(i)),
            None => Ok(()),
        };
        if format == PixelFormat::RgbaF32 {
            let l = c.to_linear_srgb();
            check([l.r, l.g, l.b])?;
            Ok(Self {
                bytes: [0; 3],
                linear: [l.r, l.g, l.b],
                alpha: f64::from(l.a).clamp(0.0, 1.0),
            })
        } else {
            let Color::Srgb(s) = c.convert(ColorSpaceId::Srgb) else {
                return Err(overflow(0));
            };
            check([s.r, s.g, s.b])?;
            let q = |v: f32| (f64::from(v).clamp(0.0, 1.0) * 255.0 + 0.5).floor() as u8;
            Ok(Self {
                bytes: [q(s.r), q(s.g), q(s.b)],
                linear: [0.0; 3],
                alpha: f64::from(s.a).clamp(0.0, 1.0),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use gafira_color::{Color, Rgba};
    use gafira_doc::contract::{ParamError, ParamValue};
    use proptest::prelude::*;

    use super::*;
    use crate::{BufferError, PixelData, PixelDataMut, PixelFormat, blend_u8, premultiply_u8};

    const SENTINEL: u8 = 0xAB;

    fn cx(scale: f32) -> EffectCx {
        EffectCx::whole(scale)
    }

    fn red() -> ParamValue {
        ParamValue::Color(Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0)))
    }

    /// Parameter values from `(id, number)` pairs; the rest take their defaults.
    fn pv(items: &[(&str, f64)]) -> ParamValues {
        let mut v = ParamValues::new();
        for (id, n) in items {
            v.set(*id, ParamValue::Float(*n));
        }
        v
    }

    /// Premultiplied pixels: random alpha with zeros, colour never above alpha, and
    /// transparent pixels that keep a colour.
    fn img_u8(w: usize, h: usize, seed: u32) -> Vec<u8> {
        let mut v = Vec::with_capacity(w * h * 4);
        for i in 0..w * h {
            let x = (i as u32 ^ seed)
                .wrapping_mul(2_654_435_761)
                .wrapping_add(seed);
            let a = match x >> 29 {
                0 | 1 => 0,
                2 => 255,
                _ => (x >> 8) as u8,
            };
            let c = |s: u32| ((x >> s) as u8).min(a);
            v.extend([c(3), c(11), c(17), a]);
            if a == 0 && x & 1 == 1 {
                let n = v.len();
                v[n - 4] = 7;
            }
        }
        v
    }

    fn img_f32(w: usize, h: usize, seed: u32) -> Vec<f32> {
        img_u8(w, h, seed)
            .chunks(4)
            .flat_map(|p| {
                let a = f32::from(p[3]) / 255.0;
                [
                    f32::from(p[0]) / 255.0 * 1.2,
                    f32::from(p[1]) / 255.0,
                    -0.0 + f32::from(p[2]) / 300.0,
                    a,
                ]
            })
            .collect()
    }

    fn bits(v: &[f32]) -> Vec<u32> {
        v.iter().map(|x| x.to_bits()).collect()
    }

    fn run8(
        c: &EffectCx,
        p: &ParamValues,
        src: &[u8],
        size: (u32, u32),
    ) -> Result<Vec<u8>, EffectError> {
        let mut dst = vec![SENTINEL; src.len()];
        ParallelShadow::new().run_cpu(
            c,
            p,
            ImageView::new(size.0, size.1, PixelData::Rgba8(src)).unwrap(),
            ImageViewMut::new(size.0, size.1, PixelDataMut::Rgba8(&mut dst)).unwrap(),
        )?;
        Ok(dst)
    }

    fn runf(
        c: &EffectCx,
        p: &ParamValues,
        src: &[f32],
        size: (u32, u32),
    ) -> Result<Vec<f32>, EffectError> {
        let mut dst = vec![f32::from_bits(0x7FC0_1234); src.len()];
        ParallelShadow::new().run_cpu(
            c,
            p,
            ImageView::new(size.0, size.1, PixelData::RgbaF32(src)).unwrap(),
            ImageViewMut::new(size.0, size.1, PixelDataMut::RgbaF32(&mut dst)).unwrap(),
        )?;
        Ok(dst)
    }

    fn resolved(p: &ParamValues) -> ParamValues {
        ParallelShadow::new().params().resolve(p).unwrap()
    }

    // FD-35, 02 §4.4 step 3: the five parameters, their ranges, and a valid definition.
    #[test]
    fn fd_35_schema_has_the_parameters_ranges_and_a_valid_definition() {
        use gafira_doc::contract::ParamKind;
        let e = ParallelShadow::new();
        assert_eq!(e.descriptor().id, "effect.shadow.parallel");
        e.params().validate_definition().unwrap();
        let float = |id: &str| match e.params().get(id).map(|d| &d.kind) {
            Some(ParamKind::Float { min, max, .. }) => (*min, *max),
            other => panic!("{id}: {other:?}"),
        };
        assert_eq!(float("offset_x"), (-500.0, 500.0));
        assert_eq!(float("offset_y"), (-500.0, 500.0));
        assert_eq!(float("blur"), (0.0, 100.0));
        assert_eq!(float("opacity"), (0.0, 100.0));
        assert!(matches!(
            e.params().get("color").map(|d| &d.kind),
            Some(ParamKind::Color { .. })
        ));
        assert_eq!(e.params().params().len(), 5);
        assert!(e.gpu().is_none());
    }

    // FD-35 (d): the extremes pass; just beyond them is Err naming the parameter.
    #[test]
    fn fd_35_d_extremes_pass_and_just_outside_is_err() {
        let src = img_u8(4, 3, 1);
        for ok in [
            [
                ("offset_x", 500.0),
                ("offset_y", -500.0),
                ("blur", 0.0),
                ("opacity", 0.0),
            ],
            [
                ("offset_x", -500.0),
                ("offset_y", 500.0),
                ("blur", 100.0),
                ("opacity", 100.0),
            ],
        ] {
            run8(&cx(1.0), &pv(&ok), &src, (4, 3)).unwrap();
            let f = img_f32(4, 3, 1);
            runf(&cx(1.0), &pv(&ok), &f, (4, 3)).unwrap();
        }
        for (id, bad) in [
            ("offset_x", 500.01),
            ("offset_x", -500.01),
            ("offset_y", 500.01),
            ("offset_y", -500.01),
            ("blur", 100.01),
            ("blur", -0.01),
            ("opacity", 100.01),
            ("opacity", -0.01),
        ] {
            assert_eq!(
                run8(&cx(1.0), &pv(&[(id, bad)]), &src, (4, 3)),
                Err(EffectError::Param(ParamError::OutOfRange {
                    param: ParallelShadow::new().params().get(id).unwrap().id
                })),
                "{id} {bad}"
            );
        }
    }

    // FD-35 (d): opacity 0 leaves the image as it was, bit for bit, in both formats, and the
    // destination is replaced, not composited onto.
    #[test]
    fn fd_35_d_opacity_zero_does_not_change_the_image() {
        let p = pv(&[
            ("opacity", 0.0),
            ("blur", 20.0),
            ("offset_x", 5.0),
            ("offset_y", -3.0),
        ]);
        let src = img_u8(12, 9, 4);
        assert_eq!(run8(&cx(1.0), &p, &src, (12, 9)).unwrap(), src);
        let f = img_f32(12, 9, 4);
        assert_eq!(bits(&runf(&cx(1.0), &p, &f, (12, 9)).unwrap()), bits(&f));
    }

    // FD-35 (a): with blur 0, where the element is transparent the shadow alpha is the
    // element's alpha moved by the offset, times the opacity, rounded to nearest. The shadow
    // is tinted with the colour parameter, premultiplied.
    #[test]
    fn fd_35_a_blur_zero_shadow_is_the_moved_alpha_times_opacity() {
        let (w, h) = (9usize, 7usize);
        let src = img_u8(w, h, 9);
        for opacity in [100.0, 50.0, 30.0, 1.0] {
            let mut p = pv(&[
                ("offset_x", 3.0),
                ("offset_y", -2.0),
                ("blur", 0.0),
                ("opacity", opacity),
            ]);
            p.set("color", red());
            let out = run8(&cx(1.0), &p, &src, (w as u32, h as u32)).unwrap();
            for y in 0..h as i64 {
                for x in 0..w as i64 {
                    let i = (y as usize * w + x as usize) * 4;
                    if src[i + 3] != 0 {
                        continue;
                    }
                    let (qx, qy) = (x - 3, y + 2);
                    let qa = if (0..w as i64).contains(&qx) && (0..h as i64).contains(&qy) {
                        u32::from(src[(qy as usize * w + qx as usize) * 4 + 3])
                    } else {
                        0
                    };
                    // round half up of qa * opacity / 100, with the opacity in tenths of percent
                    let sa = ((qa * (opacity * 10.0) as u32 * 2 + 1000) / 2000) as u8;
                    let want = [premultiply_u8(255, sa), 0, 0, sa];
                    if sa == 0 {
                        // No shadow here: the pixel stays as it is, only its alpha is known.
                        assert_eq!(out[i + 3], 0, "opacity {opacity} ({x},{y})");
                    } else {
                        assert_eq!(&out[i..i + 4], &want, "opacity {opacity} ({x},{y})");
                    }
                }
            }
        }
        // The same in f32: linear red, alpha exact to rounding.
        let f = img_f32(w, h, 9);
        let mut p = pv(&[
            ("offset_x", 3.0),
            ("offset_y", -2.0),
            ("blur", 0.0),
            ("opacity", 40.0),
        ]);
        p.set("color", red());
        let out = runf(&cx(1.0), &p, &f, (w as u32, h as u32)).unwrap();
        for y in 0..h as i64 {
            for x in 0..w as i64 {
                let i = (y as usize * w + x as usize) * 4;
                if f[i + 3] != 0.0 {
                    continue;
                }
                let (qx, qy) = (x - 3, y + 2);
                let qa = if (0..w as i64).contains(&qx) && (0..h as i64).contains(&qy) {
                    f[(qy as usize * w + qx as usize) * 4 + 3]
                } else {
                    0.0
                };
                let want = [qa * 0.4, 0.0, 0.0, qa * 0.4];
                // Without a shadow the pixel stays as it is; only its alpha is known.
                let channels = if qa == 0.0 { 3..4 } else { 0..4 };
                for c in channels {
                    assert!((out[i + c] - want[c]).abs() <= 1e-6, "({x},{y}) c{c}");
                }
            }
        }
    }

    // FD-35: a positive offset moves the shadow right and down (y grows downward).
    #[test]
    fn fd_35_offset_signs_move_the_shadow_right_and_down() {
        let (w, h) = (9usize, 9usize);
        let mut src = vec![0u8; w * h * 4];
        src[(4 * w + 4) * 4..][..4].copy_from_slice(&[0, 0, 0, 255]);
        for (dx, dy) in [(2.0, 1.0), (-3.0, 2.0), (1.0, -4.0), (-2.0, -1.0)] {
            let p = pv(&[
                ("offset_x", dx),
                ("offset_y", dy),
                ("blur", 0.0),
                ("opacity", 100.0),
            ]);
            let out = run8(&cx(1.0), &p, &src, (9, 9)).unwrap();
            let at = |x: i32, y: i32| out[(y as usize * w + x as usize) * 4 + 3];
            assert_eq!(at(4, 4), 255);
            assert_eq!(at(4 + dx as i32, 4 + dy as i32), 255, "offset ({dx},{dy})");
            let alpha: u32 = out.chunks(4).map(|p| u32::from(p[3])).sum();
            assert_eq!(alpha, 510, "offset ({dx},{dy})");
        }
    }

    // FD-35 (e): every output pixel of the 8-bit path is the ARQ-119 Normal blend in 8 bits, with
    // the shadow pixel as the backdrop and the element as the source, bit for bit; a shadow of
    // alpha 0 leaves the element as it is.
    #[test]
    fn fd_35_the_8_bit_composite_equals_the_normal_blend() {
        let vals: Vec<u8> = (0..=255)
            .step_by(17)
            .chain([1, 2, 126, 127, 128, 253, 254, 255])
            .collect();
        let tints = [
            (1.0, 0.0, 0.0),
            (0.8, 0.25, 0.1),
            (0.5, 0.5, 0.5),
            (0.0, 0.0, 0.0),
        ];
        for opacity in [100.0, 60.0, 25.0] {
            for &(r, g, b) in &tints {
                let mut p = pv(&[
                    ("blur", 0.0),
                    ("opacity", opacity),
                    ("offset_x", 0.0),
                    ("offset_y", 0.0),
                ]);
                p.set(
                    "color",
                    ParamValue::Color(Color::Srgb(Rgba::new(r, g, b, 1.0))),
                );
                let q = |v: f32| (f64::from(v) * 255.0 + 0.5).floor() as u8;
                let bytes = [q(r), q(g), q(b)];
                for &ea in &vals {
                    for &c in &vals {
                        for &e in &vals {
                            // An element is a premultiplied pixel: no channel above its alpha.
                            let element = [e.min(ea), c.min(ea), (e / 3).min(ea), ea];
                            let got = run8(&cx(1.0), &p, &element, (1, 1)).unwrap();
                            let sa = (f64::from(ea) * opacity / 100.0 + 0.5).floor() as u8;
                            let shadow = [
                                premultiply_u8(bytes[0], sa),
                                premultiply_u8(bytes[1], sa),
                                premultiply_u8(bytes[2], sa),
                                sa,
                            ];
                            let want = if sa == 0 {
                                element
                            } else {
                                blend_u8(BlendMode::Normal, shadow, element)
                            };
                            assert_eq!(
                                got,
                                want.to_vec(),
                                "{shadow:?} under {element:?} (opacity {opacity})"
                            );
                        }
                    }
                }
            }
        }
    }

    fn srgb_to_linear(v: f64) -> f64 {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }

    fn linear_to_srgb(v: f64) -> f64 {
        if v <= 0.003_130_8 {
            12.92 * v
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        }
    }

    /// The `RgbaF32` input of an 8-bit image (FD-35 f): unpremultiplied with ARQ-117, decoded
    /// to linear light and premultiplied again.
    fn to_linear_f32(px: &[u8]) -> Vec<f32> {
        px.chunks_exact(4)
            .flat_map(|p| {
                let a = f64::from(p[3]) / 255.0;
                let c = |i: usize| {
                    let straight = crate::unpremultiply_u8(p[i], p[3]);
                    (srgb_to_linear(f64::from(straight) / 255.0) * a) as f32
                };
                [c(0), c(1), c(2), a as f32]
            })
            .collect()
    }

    /// The `RgbaF32` output in 8-bit units, as the reference of ARQ-119: straight linear light
    /// encoded to sRGB, premultiplied, times 255; alpha does not go through the curve.
    fn to_units(px: &[f32]) -> Vec<f64> {
        px.chunks_exact(4)
            .flat_map(|p| {
                let a = f64::from(p[3]);
                let c = |i: usize| {
                    if a == 0.0 {
                        0.0
                    } else {
                        255.0 * a * linear_to_srgb((f64::from(p[i]) / a).clamp(0.0, 1.0))
                    }
                };
                [c(0), c(1), c(2), 255.0 * a]
            })
            .collect()
    }

    // FD-35 (f): the same image in Rgba8 and RgbaF32, with the same parameters, differs by at
    // most 2/255 per channel wherever the 8-bit shadow alpha is at least 16; below 16 there is
    // no bound (D-56). Includes blur, low element alpha and a translucent tint.
    #[test]
    fn fd_35_f_rgba8_and_rgba_f32_agree_within_2_255_from_shadow_alpha_16() {
        let bump = |peak: f64| -> Vec<u8> {
            bump_f32(96, 80, peak)
                .iter()
                .map(|v| (f64::from(*v) * 255.0).round() as u8)
                .collect()
        };
        // Alpha 0 is black (ARQ-117): the f32 input cannot keep a stray colour there.
        let clean = |mut v: Vec<u8>| {
            for p in v.chunks_exact_mut(4).filter(|p| p[3] == 0) {
                p.fill(0);
            }
            v
        };
        let mut images: Vec<(String, Vec<u8>, (u32, u32))> = vec![
            ("translucent bump".into(), bump(200.0 / 255.0), (96, 80)),
            ("opaque bump".into(), bump(1.0), (96, 80)),
            ("low alpha bump".into(), bump(12.0 / 255.0), (96, 80)),
        ];
        for seed in 1..=12u32 {
            images.push((
                format!("random {seed}"),
                clean(img_u8(40, 30, seed.wrapping_mul(0x9E37_79B9))),
                (40, 30),
            ));
        }
        let colors = [
            Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)),
            Color::Srgb(Rgba::new(0.8, 0.25, 0.1, 1.0)),
            Color::Srgb(Rgba::new(0.8, 0.25, 0.1, 0.4)),
            Color::Srgb(Rgba::new(0.5, 0.5, 0.5, 0.5)),
        ];
        let (mut worst, mut worst_at, mut compared) = (0.0f64, String::new(), 0usize);
        for (name, img, size) in &images {
            let (w, h) = (size.0 as usize, size.1 as usize);
            let alpha: Vec<f32> = img.chunks_exact(4).map(|px| f32::from(px[3])).collect();
            for blur in [0.0, 1.0, 3.0, 12.0] {
                for opacity in [100.0, 55.0, 10.0, 3.0, 1.0] {
                    for color in colors {
                        let mut p = pv(&[
                            ("blur", blur),
                            ("opacity", opacity),
                            ("offset_x", 3.0),
                            ("offset_y", -2.0),
                        ]);
                        p.set("color", ParamValue::Color(color));
                        let o8 = run8(&cx(1.0), &p, img, *size).unwrap();
                        let of = runf(&cx(1.0), &p, &to_linear_f32(img), *size).unwrap();
                        let units = to_units(&of);
                        // The 8-bit shadow alpha of each pixel, as the effect forms it.
                        let v = resolved(&p);
                        let g = Geometry::new(&cx(1.0), &v);
                        let cover = blur_shifted(&alpha, w, h, -g.dx, -g.dy, g.radius);
                        let tint_a = match color {
                            Color::Srgb(c) => f64::from(c.a),
                            _ => 1.0,
                        };
                        for (i, c) in cover.iter().enumerate() {
                            let sa = (f64::from(*c) * opacity / 100.0 * tint_a + 0.5).floor();
                            if sa < 16.0 {
                                continue;
                            }
                            compared += 1;
                            for ch in 0..4 {
                                let d = (f64::from(o8[i * 4 + ch]) - units[i * 4 + ch]).abs();
                                if d > worst {
                                    worst = d;
                                    worst_at = format!(
                                        "{name} blur {blur} opacity {opacity} {color:?} px {i} ch {ch}"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        eprintln!("worst at shadow alpha >= 16: {worst} / 255, {worst_at}");
        assert!(compared > 10_000, "too few pixels compared: {compared}");
        assert!(worst <= 2.0, "{worst} / 255 at {worst_at}");
    }

    // FD-35: the colour parameter tints the shadow, and its own alpha scales the shadow. 8-bit
    // pixels hold the sRGB bytes; f32 pixels hold linear light.
    #[test]
    fn fd_35_the_colour_tints_the_shadow_and_its_alpha_scales_it() {
        let mut p = pv(&[
            ("blur", 0.0),
            ("opacity", 100.0),
            ("offset_x", 1.0),
            ("offset_y", 0.0),
        ]);
        p.set(
            "color",
            ParamValue::Color(Color::Srgb(Rgba::new(0.5, 0.5, 0.5, 0.5))),
        );
        let mut src = vec![0u8; 3 * 4];
        src[0..4].copy_from_slice(&[0, 0, 0, 255]);
        let out = run8(&cx(1.0), &p, &src, (3, 1)).unwrap();
        // Alpha 255 * 0.5 = 127.5, to 128; the grey byte 127.5 to 128, premultiplied by 128.
        let sa = 128;
        assert_eq!(
            &out[4..8],
            &[
                premultiply_u8(128, sa),
                premultiply_u8(128, sa),
                premultiply_u8(128, sa),
                sa
            ]
        );
        let mut f = vec![0.0f32; 3 * 4];
        f[0..4].copy_from_slice(&[0.0, 0.0, 0.0, 1.0]);
        let out = runf(&cx(1.0), &p, &f, (3, 1)).unwrap();
        // Linear light of sRGB 0.5 is 0.2140, premultiplied by the alpha 0.5.
        for (g, w) in out[4..8]
            .iter()
            .zip([0.2140 * 0.5, 0.2140 * 0.5, 0.2140 * 0.5, 0.5])
        {
            assert!((g - w).abs() <= 1e-3, "{out:?}");
        }
    }

    // FD-35: the element is painted over its shadow, not under: a translucent element shows
    // the shadow through it, an opaque shadow does not cover the element.
    #[test]
    fn fd_35_the_element_is_composited_over_the_shadow() {
        let mut p = pv(&[
            ("blur", 0.0),
            ("opacity", 100.0),
            ("offset_x", 0.0),
            ("offset_y", 0.0),
        ]);
        p.set("color", red());
        // A half-transparent black element over its own red shadow, which has the same alpha.
        let src = [0u8, 0, 0, 128];
        let out = run8(&cx(1.0), &p, &src, (1, 1)).unwrap();
        // The red of the shadow shows through the element, in linear light (FD-35 e).
        assert_eq!(out, vec![117, 0, 0, 192]);
        let f = [0.0f32, 0.0, 0.0, 0.5];
        let out = runf(&cx(1.0), &p, &f, (1, 1)).unwrap();
        for (g, w) in out.iter().zip([0.25f32, 0.0, 0.0, 0.75]) {
            assert!((g - w).abs() <= 1e-6, "{out:?}");
        }
    }

    // ARQ-120: the radius and the offset are in logical px and are multiplied by cx.scale.
    #[test]
    fn arq_120_radius_and_offset_are_multiplied_by_the_scale() {
        let (w, h) = (48usize, 40usize);
        let mut src = vec![0u8; w * h * 4];
        for y in 14..22 {
            for x in 16..26 {
                src[(y * w + x) * 4 + 3] = 255;
            }
        }
        let (ws, hs) = (w as u32, h as u32);
        let at2 = run8(
            &cx(2.0),
            &pv(&[("blur", 8.0), ("offset_x", 3.0), ("offset_y", -2.0)]),
            &src,
            (ws, hs),
        )
        .unwrap();
        let at1 = run8(
            &cx(1.0),
            &pv(&[("blur", 16.0), ("offset_x", 6.0), ("offset_y", -4.0)]),
            &src,
            (ws, hs),
        )
        .unwrap();
        assert_eq!(at2, at1);
        let plain = run8(
            &cx(1.0),
            &pv(&[("blur", 8.0), ("offset_x", 3.0), ("offset_y", -2.0)]),
            &src,
            (ws, hs),
        )
        .unwrap();
        assert_ne!(at2, plain);
    }

    // ARQ-120: a smooth image and its half at scale 0.5 agree after scaling back, for a radius
    // of 16 px or more and offsets that are whole pixels at that scale (even). This test is the
    // alpha of an opaque-colour bump, in `Rgba8`, within 1/255 of the 2x2 mean over the whole
    // image; the tinted `RgbaF32` and translucent `Rgba8` cases are the next two tests.
    #[test]
    fn arq_120_a_half_size_image_at_half_scale_matches_after_rescaling() {
        let half = (40usize, 32usize);
        // A smooth bump of alpha: raised cosine over an ellipse.
        let mut small = vec![0u8; half.0 * half.1 * 4];
        for y in 0..half.1 {
            for x in 0..half.0 {
                let (dx, dy) = ((x as f64 - 19.5) / 12.0, (y as f64 - 15.5) / 9.0);
                let d = (dx * dx + dy * dy).sqrt().min(1.0);
                let a = 255.0 * 0.5 * (1.0 + (std::f64::consts::PI * d).cos());
                small[(y * half.0 + x) * 4 + 3] = a.round() as u8;
            }
        }
        let mut big = vec![0u8; small.len() * 4];
        for y in 0..half.1 * 2 {
            for x in 0..half.0 * 2 {
                let s = ((y / 2) * half.0 + x / 2) * 4;
                big[(y * half.0 * 2 + x) * 4..][..4].copy_from_slice(&small[s..s + 4]);
            }
        }
        for (blur, ox, oy) in [(16.0, 0.0, 0.0), (24.0, 4.0, -2.0), (32.0, -6.0, 8.0)] {
            let p = pv(&[
                ("blur", blur),
                ("opacity", 100.0),
                ("offset_x", ox),
                ("offset_y", oy),
            ]);
            let full = run8(&cx(1.0), &p, &big, (half.0 as u32 * 2, half.1 as u32 * 2)).unwrap();
            let lo = run8(&cx(0.5), &p, &small, (half.0 as u32, half.1 as u32)).unwrap();
            // The shadow reaches outside the bump: pixels the element leaves empty get alpha.
            let spread = (0..half.0 * half.1)
                .filter(|&i| small[i * 4 + 3] == 0 && lo[i * 4 + 3] > 0)
                .count();
            assert!(spread > 50, "no shadow, blur {blur}: {spread}");
            for y in 0..half.1 {
                for x in 0..half.0 {
                    let mean: u32 = [(0, 0), (1, 0), (0, 1), (1, 1)]
                        .iter()
                        .map(|(a, b)| {
                            u32::from(full[((2 * y + b) * half.0 * 2 + 2 * x + a) * 4 + 3])
                        })
                        .sum();
                    let got = u32::from(lo[(y * half.0 + x) * 4 + 3]);
                    assert!(
                        (mean as f64 / 4.0 - got as f64).abs() <= 1.0,
                        "blur {blur} ({x},{y}): {} vs {got}",
                        mean as f64 / 4.0
                    );
                }
            }
        }
    }

    /// A smooth premultiplied bump: alpha a raised cosine over an ellipse scaled by `peak`,
    /// straight colour a gentle gradient. Returns straight-colour premultiplied `f32` pixels.
    fn bump_f32(w: usize, h: usize, peak: f64) -> Vec<f32> {
        let mut v = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
                let (dx, dy) = (
                    (x as f64 - (w as f64 - 1.0) / 2.0) / (w as f64 * 0.3),
                    (y as f64 - (h as f64 - 1.0) / 2.0) / (h as f64 * 0.28),
                );
                let d = (dx * dx + dy * dy).sqrt().min(1.0);
                let a = peak * 0.5 * (1.0 + (std::f64::consts::PI * d).cos());
                let c = [
                    0.2 + 0.6 * x as f64 / w as f64,
                    0.7 - 0.4 * y as f64 / h as f64,
                    0.3 + 0.2 * (x + y) as f64 / (w + h) as f64,
                ];
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

    /// Whether the pixel is farther than 3 sigma of the half-size run plus its offset from
    /// every edge of the half-size image (ARQ-120).
    fn inside(x: usize, y: usize, half: (usize, usize), blur: f64, off: (f64, f64)) -> bool {
        let margin = (0.75 * blur).ceil() + (off.0.abs().max(off.1.abs()) / 2.0).ceil() + 1.0;
        let m = margin as usize;
        x >= m && y >= m && x + m < half.0 && y + m < half.1
    }

    fn tinted() -> ParamValue {
        ParamValue::Color(Color::Srgb(Rgba::new(0.8, 0.25, 0.1, 1.0)))
    }

    // ARQ-120 (W7 of RS4): the smooth tinted image in `RgbaF32`, its half and the full size made
    // of the half repeated 2x2, within 1/255 per channel at 3 sigma plus the offset from the edge.
    // Passes from the start; proved by a mutant on the tint (see apply-progress).
    #[test]
    fn arq_120_half_size_tinted_rgba_f32_matches_within_1_255() {
        let half = (96usize, 80usize);
        let small = bump_f32(half.0, half.1, 1.0);
        let mut big = vec![0f32; small.len() * 4];
        for y in 0..half.1 * 2 {
            for x in 0..half.0 * 2 {
                let s = ((y / 2) * half.0 + x / 2) * 4;
                big[(y * half.0 * 2 + x) * 4..][..4].copy_from_slice(&small[s..s + 4]);
            }
        }
        for (blur, ox, oy) in [(16.0, 0.0, 0.0), (24.0, 4.0, -2.0), (32.0, -6.0, 8.0)] {
            let mut p = pv(&[
                ("blur", blur),
                ("opacity", 80.0),
                ("offset_x", ox),
                ("offset_y", oy),
            ]);
            p.set("color", tinted());
            let full = runf(&cx(1.0), &p, &big, (half.0 as u32 * 2, half.1 as u32 * 2)).unwrap();
            let lo = runf(&cx(0.5), &p, &small, (half.0 as u32, half.1 as u32)).unwrap();
            let mut checked = 0;
            for y in 0..half.1 {
                for x in 0..half.0 {
                    if !inside(x, y, half, blur, (ox, oy)) {
                        continue;
                    }
                    checked += 1;
                    for ch in 0..4 {
                        let mean: f32 = [(0, 0), (1, 0), (0, 1), (1, 1)]
                            .iter()
                            .map(|(a, b)| full[((2 * y + b) * half.0 * 2 + 2 * x + a) * 4 + ch])
                            .sum::<f32>()
                            / 4.0;
                        let got = lo[(y * half.0 + x) * 4 + ch];
                        assert!(
                            (mean - got).abs() <= 1.0 / 255.0,
                            "blur {blur} ({x},{y}) ch {ch}: {mean} vs {got}"
                        );
                    }
                }
            }
            assert!(checked > 500, "too few pixels compared: {checked}");
        }
    }

    // ARQ-120 (W7 of RS4): the translucent premultiplied image with a tinted shadow in `Rgba8`,
    // within 2/255 per channel (8-bit rounding), same distance from the edge. Passes from the
    // start; proved by a mutant on the shadow alpha (see apply-progress).
    #[test]
    fn arq_120_half_size_translucent_tinted_rgba8_matches_within_2_255() {
        let half = (96usize, 80usize);
        let small: Vec<u8> = bump_f32(half.0, half.1, 200.0 / 255.0)
            .iter()
            .map(|v| (f64::from(*v) * 255.0).round() as u8)
            .collect();
        let mut big = vec![0u8; small.len() * 4];
        for y in 0..half.1 * 2 {
            for x in 0..half.0 * 2 {
                let s = ((y / 2) * half.0 + x / 2) * 4;
                big[(y * half.0 * 2 + x) * 4..][..4].copy_from_slice(&small[s..s + 4]);
            }
        }
        for (blur, ox, oy) in [(16.0, 0.0, 0.0), (24.0, 4.0, -2.0), (32.0, -6.0, 8.0)] {
            let mut p = pv(&[
                ("blur", blur),
                ("opacity", 80.0),
                ("offset_x", ox),
                ("offset_y", oy),
            ]);
            p.set("color", tinted());
            let full = run8(&cx(1.0), &p, &big, (half.0 as u32 * 2, half.1 as u32 * 2)).unwrap();
            let lo = run8(&cx(0.5), &p, &small, (half.0 as u32, half.1 as u32)).unwrap();
            let mut checked = 0;
            for y in 0..half.1 {
                for x in 0..half.0 {
                    if !inside(x, y, half, blur, (ox, oy)) {
                        continue;
                    }
                    checked += 1;
                    for ch in 0..4 {
                        let mean: u32 = [(0, 0), (1, 0), (0, 1), (1, 1)]
                            .iter()
                            .map(|(a, b)| {
                                u32::from(full[((2 * y + b) * half.0 * 2 + 2 * x + a) * 4 + ch])
                            })
                            .sum();
                        let got = f64::from(lo[(y * half.0 + x) * 4 + ch]);
                        assert!(
                            (f64::from(mean) / 4.0 - got).abs() <= 2.0,
                            "blur {blur} ({x},{y}) ch {ch}: {} vs {got}",
                            f64::from(mean) / 4.0
                        );
                    }
                }
            }
            assert!(checked > 500, "too few pixels compared: {checked}");
        }
    }

    // ARQ-120: the region of interest is the output grown by the kernel on each side, with the
    // scale of the context it receives, joined with the output itself and the moved shadow.
    #[test]
    fn arq_120_roi_grows_the_output_by_the_scaled_kernel() {
        let e = ParallelShadow::new();
        let out = IRect::new(10, 20, 50, 60);
        let p = resolved(&pv(&[("blur", 10.0), ("offset_x", 0.0), ("offset_y", 0.0)]));
        // blur 10 gives sigma 5 and 15 taps at scale 1; the table is taps = ceil(3 sigma).
        for (scale, taps) in [(1.0, 15), (2.0, 30), (0.5, 8), (4.0, 60)] {
            assert_eq!(
                e.roi(&cx(scale), &p, out),
                out.inflate(taps, taps),
                "scale {scale}"
            );
        }
        // ARQ-120: at scale 2 and radius r it holds out grown by ceil(2 r).
        assert!(e.roi(&cx(2.0), &p, out).contains(10 - 20, 20 - 20));
        let none = resolved(&pv(&[("blur", 0.0), ("offset_x", 0.0), ("offset_y", 0.0)]));
        assert_eq!(e.roi(&cx(1.0), &none, out), out);
        // The offset moves the shadow's source the other way, and the output stays inside.
        let moved = resolved(&pv(&[("blur", 0.0), ("offset_x", 4.0), ("offset_y", -6.0)]));
        assert_eq!(e.roi(&cx(1.0), &moved, out), IRect::new(6, 20, 50, 66));
        assert_eq!(e.roi(&cx(2.0), &moved, out), IRect::new(2, 20, 50, 72));
        let blurred = resolved(&pv(&[
            ("blur", 4.0),
            ("offset_x", -10.0),
            ("offset_y", 0.0),
        ]));
        assert_eq!(
            e.roi(&cx(1.0), &blurred, out),
            IRect::new(10, 14, 50 + 10 + 6, 66)
        );
    }

    // ARQ-120: a tile computed with its roi equals the whole image, at scale 1, 0.5 and 2.
    #[test]
    fn arq_120_a_tile_with_its_roi_matches_the_whole_image() {
        let e = ParallelShadow::new();
        let (w, h) = (64usize, 52usize);
        let src_f = img_f32(w, h, 21);
        let src_8 = img_u8(w, h, 21);
        let mut p = pv(&[
            ("offset_x", 9.0),
            ("offset_y", -6.0),
            ("blur", 7.0),
            ("opacity", 80.0),
        ]);
        p.set(
            "color",
            ParamValue::Color(Color::Srgb(Rgba::new(0.2, 0.4, 0.9, 0.9))),
        );
        let values = resolved(&p);
        let out = IRect::new(21, 17, 44, 36);
        for scale in [1.0f32, 0.5, 2.0] {
            let c = cx(scale);
            let roi = e
                .roi(&c, &values, out)
                .intersect(IRect::new(0, 0, w as i32, h as i32));
            let (rx, ry) = (roi.x0 as usize, roi.y0 as usize);
            let (cw, ch) = (roi.width() as usize, roi.height() as usize);
            let crop = |s: &[f32]| -> Vec<f32> {
                (ry..ry + ch)
                    .flat_map(|y| s[(y * w + rx) * 4..(y * w + rx + cw) * 4].to_vec())
                    .collect()
            };
            let whole = runf(&c, &p, &src_f, (w as u32, h as u32)).unwrap();
            assert!(whole.iter().all(|v| v.is_finite()), "scale {scale}");
            let tile = runf(&c, &p, &crop(&src_f), (cw as u32, ch as u32)).unwrap();
            let crop8 = |s: &[u8]| -> Vec<u8> {
                (ry..ry + ch)
                    .flat_map(|y| s[(y * w + rx) * 4..(y * w + rx + cw) * 4].to_vec())
                    .collect()
            };
            let whole8 = run8(&c, &p, &src_8, (w as u32, h as u32)).unwrap();
            let tile8 = run8(&c, &p, &crop8(&src_8), (cw as u32, ch as u32)).unwrap();
            for y in out.y0 as usize..out.y1 as usize {
                for x in out.x0 as usize..out.x1 as usize {
                    let (a, b) = ((y * w + x) * 4, ((y - ry) * cw + (x - rx)) * 4);
                    assert_eq!(
                        bits(&whole[a..a + 4]),
                        bits(&tile[b..b + 4]),
                        "f32 scale {scale} ({x},{y})"
                    );
                    for c in 0..4 {
                        assert!(
                            whole8[a + c].abs_diff(tile8[b + c]) <= 1,
                            "u8 scale {scale} ({x},{y})"
                        );
                    }
                }
            }
        }
    }

    fn bad_numbers() -> [f32; 3] {
        [f32::NAN, f32::INFINITY, f32::NEG_INFINITY]
    }

    // ARQ-110: NaN and both infinities in each numeric parameter give Err naming it, and the
    // destination is left as it was.
    #[test]
    fn arq_110_non_finite_parameter_is_err_naming_it_and_writes_nothing() {
        let src = img_u8(5, 4, 2);
        for id in ["offset_x", "offset_y", "blur", "opacity"] {
            for bad in bad_numbers() {
                let mut dst = vec![SENTINEL; src.len()];
                let r = ParallelShadow::new().run_cpu(
                    &cx(1.0),
                    &pv(&[(id, f64::from(bad))]),
                    ImageView::new(5, 4, PixelData::Rgba8(&src)).unwrap(),
                    ImageViewMut::new(5, 4, PixelDataMut::Rgba8(&mut dst)).unwrap(),
                );
                let name = ParallelShadow::new().params().get(id).unwrap().id;
                assert_eq!(
                    r,
                    Err(EffectError::Param(ParamError::NonFinite { param: name }))
                );
                assert!(dst.iter().all(|&b| b == SENTINEL), "{id} {bad}");
            }
        }
        for bad in bad_numbers() {
            for ch in 0..4 {
                let mut c = [0.1f32, 0.2, 0.3, 1.0];
                c[ch] = bad;
                let mut p = ParamValues::new();
                p.set(
                    "color",
                    ParamValue::Color(Color::Srgb(Rgba::new(c[0], c[1], c[2], c[3]))),
                );
                assert_eq!(
                    run8(&cx(1.0), &p, &src, (5, 4)),
                    Err(EffectError::Param(ParamError::NonFinite { param: "color" }))
                );
            }
        }
    }

    // ARQ-110: a non-finite cx.scale is the first thing checked, before the parameters, the
    // formats and the sizes; nothing is written.
    #[test]
    fn arq_110_non_finite_scale_is_checked_first_and_writes_nothing() {
        let src = img_u8(5, 4, 2);
        let a8 = vec![1u8; 20];
        for bad in bad_numbers() {
            let want = EffectError::Buffer(BufferError::NonFinite {
                field: "cx.scale",
                index: 0,
            });
            assert_eq!(run8(&cx(bad), &pv(&[]), &src, (5, 4)), Err(want));
            // With a bad parameter too.
            assert_eq!(
                run8(&cx(bad), &pv(&[("blur", f64::NAN)]), &src, (5, 4)),
                Err(want)
            );
            assert_eq!(
                run8(&cx(bad), &pv(&[("blur", 500.0)]), &src, (5, 4)),
                Err(want)
            );
            // With an unsupported format and a wrong size.
            let mut dst = vec![SENTINEL; 8];
            let r = ParallelShadow::new().run_cpu(
                &cx(bad),
                &pv(&[]),
                ImageView::new(5, 4, PixelData::Alpha8(&a8)).unwrap(),
                ImageViewMut::new(2, 1, PixelDataMut::Rgba8(&mut dst)).unwrap(),
            );
            assert_eq!(r, Err(want));
            assert!(dst.iter().all(|&b| b == SENTINEL));
        }
    }

    // ARQ-110, 02 §7.3: parameters come before formats, formats before sizes, sizes before
    // pixels; Alpha8 is not supported; size 0 is Ok.
    #[test]
    fn arq_110_checks_run_in_order_params_formats_sizes_pixels() {
        let a8 = vec![1u8; 20];
        let src = img_u8(5, 4, 2);
        let nan = pv(&[("blur", f64::NAN)]);
        let param = Err(EffectError::Param(ParamError::NonFinite { param: "blur" }));
        let mut dst = vec![0u8; 20 * 4];
        let r = ParallelShadow::new().run_cpu(
            &cx(1.0),
            &nan,
            ImageView::new(5, 4, PixelData::Alpha8(&a8)).unwrap(),
            ImageViewMut::new(5, 4, PixelDataMut::Rgba8(&mut dst)).unwrap(),
        );
        assert_eq!(r, param);
        let r = ParallelShadow::new().run_cpu(
            &cx(1.0),
            &pv(&[]),
            ImageView::new(5, 4, PixelData::Alpha8(&a8)).unwrap(),
            ImageViewMut::new(5, 4, PixelDataMut::Rgba8(&mut dst)).unwrap(),
        );
        assert_eq!(
            r,
            Err(EffectError::UnsupportedFormat {
                format: PixelFormat::Alpha8
            })
        );
        // dst of another format and another size: the format is reported.
        let mut df = vec![0.0f32; 4];
        let r = ParallelShadow::new().run_cpu(
            &cx(1.0),
            &pv(&[]),
            ImageView::new(5, 4, PixelData::Rgba8(&src)).unwrap(),
            ImageViewMut::new(1, 1, PixelDataMut::RgbaF32(&mut df)).unwrap(),
        );
        assert_eq!(
            r,
            Err(EffectError::FormatMismatch {
                field: "dst",
                expected: PixelFormat::Rgba8,
                actual: PixelFormat::RgbaF32
            })
        );
        let mut small = vec![SENTINEL; 4];
        let r = ParallelShadow::new().run_cpu(
            &cx(1.0),
            &pv(&[]),
            ImageView::new(5, 4, PixelData::Rgba8(&src)).unwrap(),
            ImageViewMut::new(1, 1, PixelDataMut::Rgba8(&mut small)).unwrap(),
        );
        assert_eq!(
            r,
            Err(EffectError::SizeMismatch {
                field: "dst",
                expected: (5, 4),
                actual: (1, 1)
            })
        );
        assert_eq!(small, vec![SENTINEL; 4]);
        // Size 0 is Ok and writes nothing.
        let r = ParallelShadow::new().run_cpu(
            &cx(1.0),
            &pv(&[]),
            ImageView::new(0, 4, PixelData::Rgba8(&[])).unwrap(),
            ImageViewMut::new(0, 4, PixelDataMut::Rgba8(&mut [])).unwrap(),
        );
        assert_eq!(r, Ok(()));
    }

    // ARQ-110: a non-finite f32 pixel of the source is Err naming `src` and the element.
    #[test]
    fn arq_110_non_finite_source_pixel_is_err_and_writes_nothing() {
        for bad in bad_numbers() {
            for at in [0usize, 5, 11] {
                let mut f = img_f32(3, 3, 5);
                f[at] = bad;
                let mut dst = vec![0.5f32; f.len()];
                let r = ParallelShadow::new().run_cpu(
                    &cx(1.0),
                    &pv(&[]),
                    ImageView::new(3, 3, PixelData::RgbaF32(&f)).unwrap(),
                    ImageViewMut::new(3, 3, PixelDataMut::RgbaF32(&mut dst)).unwrap(),
                );
                assert_eq!(
                    r,
                    Err(EffectError::Buffer(BufferError::NonFinite {
                        field: "src",
                        index: at
                    }))
                );
                assert!(dst.iter().all(|&v| v == 0.5));
            }
        }
    }

    // ARQ-110: an id the schema does not know is never read, even holding NaN.
    #[test]
    fn arq_110_unknown_ids_are_ignored() {
        let src = img_u8(7, 6, 3);
        let base = pv(&[("blur", 6.0), ("offset_x", 2.0)]);
        assert!(run8(&cx(1.0), &base, &src, (7, 6)).unwrap() != vec![SENTINEL; src.len()]);
        let mut with = base.clone();
        with.set("not_mine", ParamValue::Float(f64::NAN));
        with.set("also_not", ParamValue::Float(f64::INFINITY));
        assert_eq!(
            run8(&cx(1.0), &with, &src, (7, 6)).unwrap(),
            run8(&cx(1.0), &base, &src, (7, 6)).unwrap()
        );
    }

    // ARQ-110: roi and cost_hint do not panic at the extremes of resolved values, of i32 and u64,
    // nor with cx.scale at f32::MAX; nor with values that were never resolved, NaN included,
    // nor with a non-finite scale. No value is pinned for those.
    #[test]
    fn arq_110_roi_and_cost_hint_do_not_panic_at_the_extremes() {
        let e = ParallelShadow::new();
        let outs = [
            IRect::new(i32::MIN, i32::MIN, i32::MAX, i32::MAX),
            IRect::new(i32::MAX - 1, i32::MAX - 1, i32::MAX, i32::MAX),
            IRect::new(i32::MIN, i32::MIN, i32::MIN + 1, i32::MIN + 1),
            IRect::new(5, 5, 5, 5),
            IRect::new(9, 9, 2, 2),
        ];
        let mut sets = Vec::new();
        for (ox, oy, b, o) in [(-500.0, -500.0, 0.0, 0.0), (500.0, 500.0, 100.0, 100.0)] {
            sets.push(resolved(&pv(&[
                ("offset_x", ox),
                ("offset_y", oy),
                ("blur", b),
                ("opacity", o),
            ])));
        }
        sets.push(ParamValues::new());
        sets.push(pv(&[
            ("blur", f64::NAN),
            ("offset_x", f64::INFINITY),
            ("offset_y", f64::NEG_INFINITY),
        ]));
        sets.push(pv(&[
            ("blur", 1e300),
            ("offset_x", 1e300),
            ("opacity", -1e300),
        ]));
        let mut wrong = ParamValues::new();
        wrong.set("blur", ParamValue::Bool(true));
        wrong.set("offset_x", ParamValue::Choice("x".into()));
        sets.push(wrong);
        let scales = [
            1.0,
            f32::MAX,
            f32::MIN,
            f32::MIN_POSITIVE,
            0.0,
            -2.0,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ];
        for p in &sets {
            for s in scales {
                for out in outs {
                    let _ = e.roi(&cx(s), p, out);
                }
                for px in [0, 1, u64::MAX / 2, u64::MAX] {
                    let _ = e.cost_hint(&cx(s), p, px);
                }
            }
        }
        // And the run itself, with the largest values and scale, on a small image.
        let p = pv(&[("offset_x", 500.0), ("offset_y", -500.0), ("blur", 100.0)]);
        run8(&cx(f32::MAX), &p, &img_u8(6, 5, 1), (6, 5)).unwrap();
        runf(&cx(f32::MAX), &p, &img_f32(6, 5, 1), (6, 5)).unwrap();
    }

    // ARQ-02, ARQ-110: with the same parameters and cx, twice the pixels never costs less, and a
    // wider kernel costs more; the pixel count does not overflow.
    #[test]
    fn arq_110_cost_hint_never_falls_with_twice_the_pixels() {
        let e = ParallelShadow::new();
        for blur in [0.0, 1.0, 10.0, 100.0] {
            let p = resolved(&pv(&[("blur", blur)]));
            for s in [0.5f32, 1.0, 2.0, 1e30] {
                for px in [
                    0u64,
                    1,
                    7,
                    1 << 20,
                    1 << 40,
                    u64::MAX / 2,
                    u64::MAX / 2 + 1,
                    u64::MAX,
                ] {
                    let one = e.cost_hint(&cx(s), &p, px);
                    let two = e.cost_hint(&cx(s), &p, px.saturating_mul(2));
                    assert!(two >= one, "blur {blur} scale {s} px {px}");
                }
            }
        }
        let cost =
            |blur: f64, s: f32| e.cost_hint(&cx(s), &resolved(&pv(&[("blur", blur)])), 1_000_000);
        assert!(cost(100.0, 1.0) > cost(10.0, 1.0));
        assert!(cost(10.0, 1.0) > cost(0.0, 1.0));
        assert!(cost(10.0, 2.0) > cost(10.0, 1.0));
    }

    // ARQ-111: two runs give the same bytes.
    #[test]
    fn arq_111_two_runs_give_the_same_bytes() {
        let p = pv(&[
            ("blur", 9.0),
            ("offset_x", -4.0),
            ("offset_y", 3.0),
            ("opacity", 65.0),
        ]);
        let s8 = img_u8(23, 17, 8);
        assert_eq!(
            run8(&cx(1.5), &p, &s8, (23, 17)),
            run8(&cx(1.5), &p, &s8, (23, 17))
        );
        assert!(run8(&cx(1.5), &p, &s8, (23, 17)).unwrap() != vec![SENTINEL; s8.len()]);
        let sf = img_f32(23, 17, 8);
        let (a, b) = (
            runf(&cx(1.5), &p, &sf, (23, 17)).unwrap(),
            runf(&cx(1.5), &p, &sf, (23, 17)).unwrap(),
        );
        assert_eq!(bits(&a), bits(&b));
    }

    // ARQ-110, FD-35: a blur whose square underflows is blur 0, in both formats: no Err, no
    // lost shadow.
    #[test]
    fn arq_110_a_blur_too_small_to_square_is_the_same_as_blur_zero() {
        let base = [("offset_x", 3.0), ("offset_y", -2.0), ("opacity", 70.0)];
        let with = |blur: f64| {
            let mut v = base.to_vec();
            v.push(("blur", blur));
            pv(&v)
        };
        let s8 = img_u8(9, 7, 6);
        let sf = img_f32(9, 7, 6);
        for tiny in [1e-170, 5e-324] {
            assert_eq!(
                run8(&cx(1.0), &with(tiny), &s8, (9, 7)),
                run8(&cx(1.0), &with(0.0), &s8, (9, 7)),
                "u8 blur {tiny}"
            );
            let (got, want) = (
                runf(&cx(1.0), &with(tiny), &sf, (9, 7)),
                runf(&cx(1.0), &with(0.0), &sf, (9, 7)),
            );
            assert!(got.is_ok(), "f32 blur {tiny}: {got:?}");
            assert_eq!(got.map(|v| bits(&v)), want.map(|v| bits(&v)), "f32 {tiny}");
        }
    }

    // ARQ-110: an overflow in the composite is Err and leaves dst as it was.
    #[test]
    fn arq_110_composite_overflow_is_err_and_dst_is_untouched() {
        let src = [0.5f32, 0.0, 0.0, 0.5, 3.3e38, 0.0, 0.0, 0.5];
        let mut p = pv(&[
            ("blur", 0.0),
            ("opacity", 100.0),
            ("offset_x", 0.0),
            ("offset_y", 0.0),
        ]);
        p.set(
            "color",
            ParamValue::Color(Color::Srgb(Rgba::new(1e16, 0.0, 0.0, 1.0))),
        );
        let sentinel = f32::from_bits(0x7FC0_1234);
        let mut dst = vec![sentinel; 8];
        let r = ParallelShadow::new().run_cpu(
            &cx(1.0),
            &p,
            ImageView::new(2, 1, PixelData::RgbaF32(&src)).unwrap(),
            ImageViewMut::new(2, 1, PixelDataMut::RgbaF32(&mut dst)).unwrap(),
        );
        assert!(r.is_err(), "{r:?}");
        assert_eq!(bits(&dst), bits(&[sentinel; 8]));
    }

    // FD-35: offsets round to whole device pixels, half away from zero, also negative.
    #[test]
    fn fd_35_offsets_round_half_away_from_zero() {
        let mut src = vec![0u8; 9 * 4];
        src[4 * 4 + 3] = 255; // pixel 4 of 9
        let at = |ox: f64, scale: f32| {
            let p = pv(&[
                ("offset_x", ox),
                ("offset_y", 0.0),
                ("blur", 0.0),
                ("opacity", 100.0),
            ]);
            let out = run8(&cx(scale), &p, &src, (9, 1)).unwrap();
            let on: Vec<usize> = (0..9).filter(|&x| out[x * 4 + 3] == 255).collect();
            // The element stays at 4; the shadow is the other pixel, or none when it overlaps.
            on.into_iter().find(|&x| x != 4).map_or(4, |x| x)
        };
        for (ox, scale, want) in [
            (-0.5, 1.0, 3),
            (-1.5, 1.0, 2),
            (0.5, 1.0, 5),
            (1.5, 1.0, 6),
            (2.5, 1.0, 7),
            (-2.5, 1.0, 1),
            (1.0, 0.5, 5),
            (-1.0, 0.5, 3),
            (3.0, 0.5, 6),
            (-3.0, 0.5, 2),
            (0.4, 1.0, 4),
            (-0.4, 1.0, 4),
        ] {
            assert_eq!(at(ox, scale), want, "offset {ox} scale {scale}");
        }
    }

    // FD-35 (a): the shadow alpha is round-half-up of alpha x opacity / 100 exactly, also at the
    // ties where opacity / 100 is not exact in binary.
    #[test]
    fn fd_35_a_opacity_ties_round_up_exactly() {
        for (a, opacity) in [
            (50u32, 29u32),
            (90, 35),
            (170, 35),
            (150, 41),
            (50, 57),
            (150, 57),
            (25, 58),
            (150, 69),
            (45, 70),
            (85, 70),
            (165, 70),
            (175, 70),
            (75, 82),
        ] {
            assert_eq!((a * opacity) % 100, 50, "{a} {opacity} is a tie");
            let mut src = vec![0u8; 8];
            src[3] = a as u8;
            let p = pv(&[
                ("offset_x", 1.0),
                ("offset_y", 0.0),
                ("blur", 0.0),
                ("opacity", f64::from(opacity)),
            ]);
            let out = run8(&cx(1.0), &p, &src, (2, 1)).unwrap();
            assert_eq!(
                u32::from(out[7]),
                (a * opacity) / 100 + 1,
                "alpha {a} opacity {opacity}"
            );
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(48))]

        // ARQ-110: with finite input inside the domain, every number of the output is finite.
        #[test]
        fn arq_110_finite_input_gives_finite_output(
            px in proptest::collection::vec(-1.0e6f32..1.0e6, 4 * 12),
            ox in -500.0f64..=500.0,
            oy in -500.0f64..=500.0,
            blur in 0.0f64..=100.0,
            opacity in 0.0f64..=100.0,
            scale in 0.01f32..8.0,
        ) {
            let p = pv(&[("offset_x", ox), ("offset_y", oy), ("blur", blur), ("opacity", opacity)]);
            if let Ok(out) = runf(&cx(scale), &p, &px, (4, 3)) {
                prop_assert!(out.iter().all(|v| v.is_finite()));
            }
        }
    }
}
