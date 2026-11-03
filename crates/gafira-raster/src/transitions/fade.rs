//! Fade (Fundido): from to a colour and from the colour to `to` (ARQ-118).

use gafira_color::{Color, ColorSpaceId, Rgba};
use gafira_doc::contract::{Describe, Descriptor, ParamDef, ParamKind, ParamSchema, ParamValues};
use gafira_doc::msg::MessageKey;

use super::guard::{Plan, copy, map_pixels, plan};
use super::mix::{weighted_f32, weighted_u8};
use super::{HandleReq, Transition};
use crate::transfer::Transfer;
use crate::{
    BufferError, EffectCx, EffectError, ImageView, ImageViewMut, PixelFormat, premultiply_u8,
};

/// The cross fade.
#[derive(Debug)]
pub struct Fade {
    descriptor: Descriptor,
    schema: ParamSchema,
}

impl Fade {
    /// Builds the transition.
    #[must_use]
    pub fn new() -> Self {
        Self {
            descriptor: Descriptor::new(
                "transition.fade",
                MessageKey("glossary.transition_fade"),
                1,
            ),
            schema: ParamSchema::new(vec![ParamDef {
                id: "color",
                label: MessageKey("transition.fade.color"),
                kind: ParamKind::Color {
                    default: Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)),
                },
                animatable: false,
            }]),
        }
    }
}

impl Default for Fade {
    fn default() -> Self {
        Self::new()
    }
}

impl Describe for Fade {
    fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }
}

impl Transition for Fade {
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
                let c = values
                    .color("color")
                    .unwrap_or(Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)));
                // Only the conversion this format uses is built, and it must not overflow.
                let (c8, cf) = match from.data().format() {
                    PixelFormat::RgbaF32 => ([0; 4], colour_f32(c)?),
                    _ => (colour_u8(c)?, [0.0; 4]),
                };
                // `from` goes to the colour with 2p; the colour goes to `to` with 2p - 1.
                // At exactly one half the second branch gives the colour alone.
                let tr = Transfer::new();
                let first = p < 0.5;
                let t = if first { 2.0 * p } else { 2.0 * p - 1.0 };
                map_pixels(
                    &from,
                    &to,
                    &mut dst,
                    |_, a, b| {
                        if first {
                            weighted_u8(&tr, &[(a, 1.0 - t), (c8, t)])
                        } else {
                            weighted_u8(&tr, &[(c8, 1.0 - t), (b, t)])
                        }
                    },
                    |_, a, b| {
                        if first {
                            weighted_f32(&[(a, 1.0 - t), (cf, t)])
                        } else {
                            weighted_f32(&[(cf, 1.0 - t), (b, t)])
                        }
                    },
                );
            }
        }
        Ok(())
    }
}

/// The converted colour overflowed `f32`: the channel is named.
fn overflow(index: usize) -> EffectError {
    BufferError::NonFinite {
        field: "color",
        index,
    }
    .into()
}

/// The colour as premultiplied sRGB bytes. Its alpha is kept, clipped to `[0, 1]`.
fn colour_u8(c: Color) -> Result<[u8; 4], EffectError> {
    let Color::Srgb(s) = c.convert(ColorSpaceId::Srgb) else {
        return Ok([0, 0, 0, 255]);
    };
    if let Some(i) = [s.r, s.g, s.b].iter().position(|v| !v.is_finite()) {
        return Err(overflow(i));
    }
    let q = |v: f32| (f64::from(v).clamp(0.0, 1.0) * 255.0 + 0.5).floor() as u8;
    let a = q(s.a);
    Ok([
        premultiply_u8(q(s.r), a),
        premultiply_u8(q(s.g), a),
        premultiply_u8(q(s.b), a),
        a,
    ])
}

/// The colour as premultiplied linear sRGB. Its alpha is kept, clipped to `[0, 1]`.
fn colour_f32(c: Color) -> Result<[f32; 4], EffectError> {
    let l = c.to_linear_srgb();
    if let Some(i) = [l.r, l.g, l.b].iter().position(|v| !v.is_finite()) {
        return Err(overflow(i));
    }
    let a = l.a.clamp(0.0, 1.0);
    Ok([l.r * a, l.g * a, l.b * a, a])
}
