//! Dissolve (Disolvencia): `from × (1 − p) + to × p` (ARQ-118).

use gafira_doc::contract::{Describe, Descriptor, ParamSchema, ParamValues};
use gafira_doc::msg::MessageKey;

use super::guard::{Plan, copy, map_pixels, plan};
use super::mix::{weighted_f32, weighted_u8};
use super::{HandleReq, Transition};
use crate::{EffectCx, EffectError, ImageView, ImageViewMut};

/// The cross dissolve.
#[derive(Debug)]
pub struct Dissolve {
    descriptor: Descriptor,
    schema: ParamSchema,
}

impl Dissolve {
    /// Builds the transition.
    #[must_use]
    pub fn new() -> Self {
        Self {
            descriptor: Descriptor::new(
                "transition.dissolve",
                MessageKey("glossary.transition_dissolve"),
                1,
            ),
            schema: ParamSchema::new(Vec::new()),
        }
    }
}

impl Default for Dissolve {
    fn default() -> Self {
        Self::new()
    }
}

impl Describe for Dissolve {
    fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }
}

impl Transition for Dissolve {
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
            Plan::Mix { p, .. } => map_pixels(
                &from,
                &to,
                &mut dst,
                |_, a, b| weighted_u8(&[(a, 1.0 - p), (b, p)]),
                |_, a, b| weighted_f32(&[(a, 1.0 - p), (b, p)]),
            ),
        }
        Ok(())
    }
}
