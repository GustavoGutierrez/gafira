//! Effects of one input (02 §4.4, ARQ-120, FD-35).

mod auto_adjust;
mod blur;
mod color_adjust;
mod detected;
mod filter;
mod light;
mod pointwise;
mod shadow;
#[cfg(test)]
mod testing;
mod texture;
mod white_balance;

use gafira_doc::contract::{Describe, ParamSchema, ParamValues};
use gafira_geom::IRect;

pub use auto_adjust::{AutoAdjust, auto_adjust};
pub(crate) use blur::blur_plane;
pub use color_adjust::ColorAdjust;
pub use detected::{DETECTED_SLOTS, DetectedColors};
pub use filter::{
    DEFAULT_INTENSITY, FilterError, FilterPreset, FilterStep, mix_params, test_preset,
};
pub use light::Light;
pub use shadow::ParallelShadow;
pub use texture::Texture;
pub use white_balance::{TemperatureTint, WhiteBalance, eyedropper};

use crate::transitions::UniformBlock;
use crate::{EffectCx, EffectError, ImageView, ImageViewMut};

/// Abstract work units for planning and budgets: pixels and kernel size, never time (ARQ-02).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cost {
    /// Work units. The scale is private to the effect; only comparisons are meaningful.
    pub units: u64,
}

/// The optional WGSL variant of an effect. No implementation in this cut.
pub trait GpuEffect: Send + Sync {
    /// The WGSL source.
    fn wgsl(&self) -> &'static str;
    /// Uniform bytes for resolved parameters (ARQ-110: `p` is resolved).
    fn bind(&self, p: &ParamValues) -> UniformBlock;
}

/// An effect with one input (02 §4.4).
pub trait Effect: Describe + Send + Sync {
    /// The parameters: types, ranges, defaults, animatable.
    fn params(&self) -> &ParamSchema;
    /// The input region needed to compute `out`. `p` is resolved and `cx.scale` finite (ARQ-110).
    fn roi(&self, cx: &EffectCx, p: &ParamValues, out: IRect) -> IRect;
    /// Work for planning and budgets, without time. `p` is resolved and `cx.scale` finite (ARQ-110).
    fn cost_hint(&self, cx: &EffectCx, p: &ParamValues, px: u64) -> Cost;
    /// Runs on `src`, a whole image or a tile described by `cx.tile`.
    ///
    /// # Errors
    ///
    /// [`EffectError`], before anything is written to `dst`.
    fn run_cpu(
        &self,
        cx: &EffectCx,
        p: &ParamValues,
        src: ImageView<'_>,
        dst: ImageViewMut<'_>,
    ) -> Result<(), EffectError>;
    /// The WGSL variant, if any.
    fn gpu(&self) -> Option<&dyn GpuEffect> {
        None
    }
}
