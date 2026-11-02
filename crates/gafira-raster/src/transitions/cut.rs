//! Cut (Corte): `from` below 0.5, `to` from 0.5 (ARQ-118).

use gafira_doc::contract::{Describe, Descriptor, ParamSchema, ParamValues};
use gafira_doc::msg::MessageKey;

use super::guard::{Plan, copy, plan};
use super::{HandleReq, Transition};
use crate::{EffectCx, EffectError, ImageView, ImageViewMut};

/// The instantaneous transition.
#[derive(Debug)]
pub struct Cut {
    descriptor: Descriptor,
    schema: ParamSchema,
}

impl Cut {
    /// Builds the transition.
    #[must_use]
    pub fn new() -> Self {
        Self {
            descriptor: Descriptor::new("transition.cut", MessageKey("glossary.transition_cut"), 1),
            schema: ParamSchema::new(Vec::new()),
        }
    }
}

impl Default for Cut {
    fn default() -> Self {
        Self::new()
    }
}

impl Describe for Cut {
    fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }
}

impl Transition for Cut {
    fn params(&self) -> &ParamSchema {
        &self.schema
    }

    fn handles(&self) -> HandleReq {
        // Instantaneous: no extra frames are shown (P15 item 5).
        HandleReq::None
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
            // `from` below one half, `to` from one half.
            Plan::Mix { p } => copy(&mut dst, if p < 0.5 { &from } else { &to }),
        }
        Ok(())
    }
}
