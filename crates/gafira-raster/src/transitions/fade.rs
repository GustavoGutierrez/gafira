//! Fade (Fundido): from to a colour and from the colour to `to` (ARQ-118).

use gafira_color::{Color, ColorSpaceId, Rgba};
use gafira_doc::contract::{Describe, Descriptor, ParamDef, ParamKind, ParamSchema, ParamValues};
use gafira_doc::msg::MessageKey;

use super::guard::{Plan, copy, map_pixels, plan};
use super::mix::{weighted_f32, weighted_u8};
use super::{HandleReq, Transition};
use crate::{EffectCx, EffectError, ImageView, ImageViewMut, premultiply_u8};

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
        _cx: &EffectCx,
        p: &ParamValues,
        progress: f32,
        from: ImageView<'_>,
        to: ImageView<'_>,
        mut dst: ImageViewMut<'_>,
    ) -> Result<(), EffectError> {
        match plan(&self.schema, p, progress, &from, &to, &dst)? {
            Plan::Done => {}
            Plan::CopyFrom => copy(&mut dst, &from),
            Plan::CopyTo => copy(&mut dst, &to),
            Plan::Mix { p, values } => {
                let c = values
                    .color("color")
                    .unwrap_or(Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)));
                let (c8, cf) = (colour_u8(c), colour_f32(c));
                // `from` goes to the colour with 2p; the colour goes to `to` with 2p - 1.
                // At exactly one half the second branch gives the colour alone.
                let first = p < 0.5;
                let t = if first { 2.0 * p } else { 2.0 * p - 1.0 };
                map_pixels(
                    &from,
                    &to,
                    &mut dst,
                    |_, a, b| {
                        if first {
                            weighted_u8(&[(a, 1.0 - t), (c8, t)])
                        } else {
                            weighted_u8(&[(c8, 1.0 - t), (b, t)])
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

/// The colour as premultiplied sRGB bytes. Its alpha is kept, clipped to `[0, 1]`.
fn colour_u8(c: Color) -> [u8; 4] {
    let Color::Srgb(s) = c.convert(ColorSpaceId::Srgb) else {
        return [0, 0, 0, 255];
    };
    let q = |v: f32| (f64::from(v).clamp(0.0, 1.0) * 255.0 + 0.5).floor() as u8;
    let a = q(s.a);
    [
        premultiply_u8(q(s.r), a),
        premultiply_u8(q(s.g), a),
        premultiply_u8(q(s.b), a),
        a,
    ]
}

/// The colour as premultiplied linear sRGB. Its alpha is kept, clipped to `[0, 1]`.
fn colour_f32(c: Color) -> [f32; 4] {
    let l = c.to_linear_srgb();
    let a = l.a.clamp(0.0, 1.0);
    [l.r * a, l.g * a, l.b * a, a]
}
