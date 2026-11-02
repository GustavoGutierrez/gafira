//! Filters: presets of effect parameters with an intensity (FD-34, D-40, D-41).

use gafira_color::{Color, LinearRgba};
use gafira_doc::contract::{ParamError, ParamSchema, ParamValue, ParamValues};
use gafira_doc::msg;
use gafira_doc::msg::{Message, MessageKey, ToMessage};

use super::Effect;
use super::pointwise::{check_io, check_scale};
use crate::{
    BufferError, EffectCx, EffectError, ImageView, ImageViewMut, PixelData, PixelDataMut,
    PixelFormat,
};

/// One effect of a preset and the parameters the preset gives it.
#[derive(Clone, Debug, PartialEq)]
pub struct FilterStep {
    /// The id of the effect (`Describe::descriptor().id`).
    pub effect: String,
    /// The values the preset names; the rest stay at their defaults.
    pub params: ParamValues,
}

/// A filter: a stable id, a name and the effects to apply in order. Data, not code.
#[derive(Clone, Debug, PartialEq)]
pub struct FilterPreset {
    /// Stable id.
    pub id: String,
    /// Catalog key of the name.
    pub name: MessageKey,
    /// The effects, applied in this order.
    pub steps: Vec<FilterStep>,
}

/// Why a filter could not run.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterError {
    /// The intensity, a parameter of the preset, a buffer or an effect was rejected.
    Effect(EffectError),
    /// Step `step` of the preset names an effect that was not given.
    UnknownEffect {
        /// Index of the step.
        step: usize,
    },
}

impl From<BufferError> for FilterError {
    fn from(e: BufferError) -> Self {
        Self::Effect(e.into())
    }
}

impl From<EffectError> for FilterError {
    fn from(e: EffectError) -> Self {
        Self::Effect(e)
    }
}

impl ToMessage for FilterError {
    fn to_message(&self) -> Message {
        match self {
            Self::Effect(e) => e.to_message(),
            Self::UnknownEffect { step } => msg!(
                "error.raster.unknown_effect",
                step = Count(u64::try_from(*step).unwrap_or(u64::MAX))
            ),
        }
    }
}

/// The intensity must be finite and in `[0, 100]` (ARQ-110).
fn check_intensity(intensity: f64) -> Result<(), FilterError> {
    if !intensity.is_finite() {
        return Err(EffectError::Buffer(BufferError::NonFinite {
            field: "intensity",
            index: 0,
        })
        .into());
    }
    if !(0.0..=100.0).contains(&intensity) {
        return Err(EffectError::Param(ParamError::OutOfRange { param: "intensity" }).into());
    }
    Ok(())
}

/// Linear-light blend with premultiplied alpha (02 3.4, Q-27, Q-31), `t` in `(0, 1)`. The
/// result keeps the space both ends share and is linear sRGB otherwise. When the resulting
/// alpha is not positive the channels blend straight. Nothing is clamped.
fn lerp_color(from: Color, to: Color, t: f64) -> Color {
    let (a, b) = (from.to_linear_srgb(), to.to_linear_srgb());
    let (wa, wb) = (f64::from(a.a), f64::from(b.a));
    let alpha = wa + (wb - wa) * t;
    let blend = |ca: f32, cb: f32| {
        let (ca, cb) = (f64::from(ca), f64::from(cb));
        let value = if alpha <= 0.0 {
            ca + (cb - ca) * t
        } else {
            let (pa, pb) = (ca * wa, cb * wb);
            (pa + (pb - pa) * t) / alpha
        };
        value as f32
    };
    let linear = Color::LinearSrgb(LinearRgba::new(
        blend(a.r, b.r),
        blend(a.g, b.g),
        blend(a.b, b.b),
        alpha as f32,
    ));
    if from.space() == to.space() {
        linear.convert(from.space())
    } else {
        linear
    }
}

/// One parameter between its default `d` and the preset's `p`, at `intensity` in `[0, 100]`.
fn mix_value(d: &ParamValue, p: &ParamValue, intensity: f64) -> ParamValue {
    // The ends are the defaults and the preset's values themselves, not a computation that
    // could drift (a -0.0, an `i64` beyond 2^53).
    if intensity == 0.0 {
        return d.clone();
    }
    if intensity == 100.0 {
        return p.clone();
    }
    let t = intensity / 100.0;
    match (d, p) {
        (ParamValue::Float(d), ParamValue::Float(p)) => ParamValue::Float(d * (1.0 - t) + p * t),
        (ParamValue::Int(d), ParamValue::Int(p)) => {
            // `round` sends a half away from 0 (D-41); `as` cuts at the ends of `i64`.
            ParamValue::Int(((*d as f64) * (1.0 - t) + (*p as f64) * t).round() as i64)
        }
        (ParamValue::Color(d), ParamValue::Color(p)) => ParamValue::Color(lerp_color(*d, *p, t)),
        // A Bool or a Choice jumps to the preset's value above 0 (D-41).
        (_, p) => p.clone(),
    }
}

/// The parameters of an effect at `intensity` (0 to 100): each one between its default and the
/// preset's value (FD-34, D-40, D-41).
///
/// # Errors
///
/// [`FilterError::Effect`] for an intensity that is NaN, infinite or outside `[0, 100]`, and for
/// a preset value the schema rejects.
pub fn mix_params(
    schema: &ParamSchema,
    preset: &ParamValues,
    intensity: f64,
) -> Result<ParamValues, FilterError> {
    check_intensity(intensity)?;
    schema.check_values(preset).map_err(EffectError::Param)?;
    let defaults = schema.defaults();
    let mut mixed = ParamValues::new();
    for def in schema.params() {
        let Some(d) = defaults.get(def.id) else {
            continue;
        };
        let value = match preset.get(def.id) {
            Some(p) => mix_value(d, p, intensity),
            None => d.clone(),
        };
        mixed.set(def.id, value);
    }
    Ok(mixed)
}

/// The effect a step names.
fn find<'a>(
    effects: &[&'a dyn Effect],
    step: &FilterStep,
    index: usize,
) -> Result<&'a dyn Effect, FilterError> {
    effects
        .iter()
        .find(|e| e.descriptor().id == step.effect)
        .copied()
        .ok_or(FilterError::UnknownEffect { step: index })
}

/// A buffer the chain writes between two effects.
enum Scratch {
    Bytes(Vec<u8>),
    Floats(Vec<f32>),
}

impl Scratch {
    fn new(format: PixelFormat, len: usize) -> Self {
        match format {
            PixelFormat::RgbaF32 => Self::Floats(vec![0.0; len]),
            _ => Self::Bytes(vec![0; len]),
        }
    }

    fn view(&self, w: u32, h: u32) -> Result<ImageView<'_>, BufferError> {
        match self {
            Self::Bytes(v) => ImageView::new(w, h, PixelData::Rgba8(v)),
            Self::Floats(v) => ImageView::new(w, h, PixelData::RgbaF32(v)),
        }
    }

    fn view_mut(&mut self, w: u32, h: u32) -> Result<ImageViewMut<'_>, BufferError> {
        match self {
            Self::Bytes(v) => ImageViewMut::new(w, h, PixelDataMut::Rgba8(v)),
            Self::Floats(v) => ImageViewMut::new(w, h, PixelDataMut::RgbaF32(v)),
        }
    }
}

impl FilterPreset {
    /// The parameters of every step at `intensity`, after finding each effect.
    ///
    /// # Errors
    ///
    /// [`FilterError`] for an unknown effect, a bad intensity or a bad preset value.
    pub fn resolve(
        &self,
        effects: &[&dyn Effect],
        intensity: f64,
    ) -> Result<Vec<ParamValues>, FilterError> {
        check_intensity(intensity)?;
        self.steps
            .iter()
            .enumerate()
            .map(|(i, step)| {
                let effect = find(effects, step, i)?;
                mix_params(effect.params(), &step.params, intensity)
            })
            .collect()
    }

    /// Applies the filter at `intensity`: every effect in order. Nothing is written to `dst`
    /// when it returns `Err`.
    ///
    /// # Errors
    ///
    /// [`FilterError`].
    pub fn apply(
        &self,
        effects: &[&dyn Effect],
        cx: &EffectCx,
        intensity: f64,
        src: ImageView<'_>,
        dst: ImageViewMut<'_>,
    ) -> Result<(), FilterError> {
        check_scale(cx)?;
        let values = self.resolve(effects, intensity)?;
        let (w, h) = check_io(&src, &dst)?;
        let (width, height) = (src.width(), src.height());
        let format = src.data().format();
        let mut dst = dst;
        if values.is_empty() {
            match (src.data(), dst.data_mut()) {
                (PixelData::Rgba8(s), PixelDataMut::Rgba8(d)) => d.copy_from_slice(s),
                (PixelData::RgbaF32(s), PixelDataMut::RgbaF32(d)) => d.copy_from_slice(s),
                _ => {}
            }
            return Ok(());
        }
        let last = values.len() - 1;
        let mut dst = Some(dst);
        let mut previous: Option<Scratch> = None;
        for (i, (step, params)) in self.steps.iter().zip(&values).enumerate() {
            let effect = find(effects, step, i)?;
            let input = match &previous {
                Some(buffer) => buffer.view(width, height).map_err(EffectError::from)?,
                None => src,
            };
            if i == last {
                if let Some(out) = dst.take() {
                    effect.run_cpu(cx, params, input, out)?;
                }
            } else {
                let mut next = Scratch::new(format, w * h * 4);
                effect.run_cpu(
                    cx,
                    params,
                    input,
                    next.view_mut(width, height).map_err(EffectError::from)?,
                )?;
                previous = Some(next);
            }
        }
        Ok(())
    }
}

/// The preset of the first cut (D-40): Contraste and Saturación. It is not shown to the person.
#[must_use]
pub fn test_preset() -> FilterPreset {
    let step = |effect: &str, id: &str, value: f64| {
        let mut params = ParamValues::new();
        params.set(id, ParamValue::Float(value));
        FilterStep {
            effect: effect.to_string(),
            params,
        }
    };
    FilterPreset {
        id: "filter.test".to_string(),
        name: MessageKey("filter.test"),
        steps: vec![
            step("effect.adjust.light", "contrast", 40.0),
            step("effect.adjust.color", "saturation", 60.0),
        ],
    }
}

#[cfg(test)]
mod tests {
    use gafira_color::Rgba;
    use gafira_doc::contract::{ParamChoice, ParamDef, ParamKind};

    use super::super::testing::*;
    use super::super::{ColorAdjust, Light};
    use super::*;
    use crate::build_mips;

    const OPTIONS: &[ParamChoice] = &[
        ParamChoice {
            id: "a",
            label: MessageKey("test.a"),
        },
        ParamChoice {
            id: "b",
            label: MessageKey("test.b"),
        },
    ];

    fn def(id: &'static str, kind: ParamKind) -> ParamDef {
        ParamDef {
            id,
            label: MessageKey("test.param"),
            kind,
            animatable: true,
        }
    }

    fn black() -> Color {
        Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0))
    }

    /// A schema with every kind of parameter and defaults away from the preset's values.
    fn schema() -> ParamSchema {
        ParamSchema::new(vec![
            def(
                "f",
                ParamKind::Float {
                    min: 0.0,
                    max: 100.0,
                    default: 0.0,
                },
            ),
            def(
                "g",
                ParamKind::Float {
                    min: -10.0,
                    max: 10.0,
                    default: 2.0,
                },
            ),
            def(
                "n",
                ParamKind::Int {
                    min: -100,
                    max: 100,
                    default: 0,
                },
            ),
            def(
                "m",
                ParamKind::Int {
                    min: 0,
                    max: 20,
                    default: 10,
                },
            ),
            def("b", ParamKind::Bool { default: false }),
            def(
                "c",
                ParamKind::Choice {
                    options: OPTIONS,
                    default: "a",
                },
            ),
            def("col", ParamKind::Color { default: black() }),
            def(
                "z",
                ParamKind::Float {
                    min: -1.0,
                    max: 1.0,
                    default: 0.0,
                },
            ),
            def(
                "big",
                ParamKind::Int {
                    min: i64::MIN,
                    max: i64::MAX,
                    default: 0,
                },
            ),
        ])
    }

    fn preset_values(items: &[(&str, ParamValue)]) -> ParamValues {
        let mut v = ParamValues::new();
        for (id, x) in items {
            v.set(*id, x.clone());
        }
        v
    }

    fn mix(items: &[(&str, ParamValue)], i: f64) -> ParamValues {
        mix_params(&schema(), &preset_values(items), i).unwrap()
    }

    fn effects() -> (Light, ColorAdjust) {
        (Light::new(), ColorAdjust::new())
    }

    fn step(effect: &str, items: &[(&str, f64)]) -> FilterStep {
        FilterStep {
            effect: effect.to_string(),
            params: pv(items),
        }
    }

    fn preset(steps: Vec<FilterStep>) -> FilterPreset {
        FilterPreset {
            id: "filter.t".to_string(),
            name: MessageKey("filter.t"),
            steps,
        }
    }

    fn apply8(
        p: &FilterPreset,
        scale: f32,
        i: f64,
        src: &[u8],
        size: (u32, u32),
    ) -> (Result<(), FilterError>, Vec<u8>) {
        let (l, c) = effects();
        let mut dst = vec![SENTINEL; src.len()];
        let r = p.apply(
            &[&l, &c],
            &cx(scale),
            i,
            ImageView::new(size.0, size.1, PixelData::Rgba8(src)).unwrap(),
            ImageViewMut::new(size.0, size.1, PixelDataMut::Rgba8(&mut dst)).unwrap(),
        );
        (r, dst)
    }

    fn applyf(
        p: &FilterPreset,
        scale: f32,
        i: f64,
        src: &[f32],
        size: (u32, u32),
    ) -> (Result<(), FilterError>, Vec<f32>) {
        let (l, c) = effects();
        let mut dst = vec![f32::from_bits(SENTINEL_F32); src.len()];
        let r = p.apply(
            &[&l, &c],
            &cx(scale),
            i,
            ImageView::new(size.0, size.1, PixelData::RgbaF32(src)).unwrap(),
            ImageViewMut::new(size.0, size.1, PixelDataMut::RgbaF32(&mut dst)).unwrap(),
        );
        (r, dst)
    }

    const LIGHT: &str = "effect.adjust.light";
    const COLOR: &str = "effect.adjust.color";

    // FD-34 (b): a Float is d (1 - i/100) + p i/100: d 0 and p 40 at 25 give 10.
    #[test]
    fn fd_34_b_float_intensity_formula() {
        let f = |p: f64, i: f64| mix(&[("f", ParamValue::Float(p))], i).float("f").unwrap();
        assert!((f(40.0, 25.0) - 10.0).abs() < 1e-12);
        assert!((f(40.0, 50.0) - 20.0).abs() < 1e-12);
        assert!((f(40.0, 1.0) - 0.4).abs() < 1e-12);
        // The default is not zero: g from 2 to -6 at 25 is 2 * 0.75 - 6 * 0.25 = 0.
        let g = mix(&[("g", ParamValue::Float(-6.0))], 25.0)
            .float("g")
            .unwrap();
        assert!(g.abs() < 1e-12, "{g}");
        // A parameter the preset does not name stays at its default, at every intensity.
        for i in [0.0, 30.0, 100.0] {
            let v = mix(&[("f", ParamValue::Float(40.0))], i);
            assert_eq!(v.float("g"), Some(2.0));
            assert_eq!(v.int("m"), Some(10));
            assert_eq!(v.bool("b"), Some(false));
            assert_eq!(v.choice("c"), Some("a"));
            assert_eq!(v.float("z"), Some(0.0));
        }
    }

    // FD-34 (b): 0 is the defaults, 100 is the preset's values, bit for bit (also -0.0).
    #[test]
    fn fd_34_b_the_ends_are_exact() {
        let items = [
            ("f", ParamValue::Float(40.0)),
            ("g", ParamValue::Float(-0.1 + 0.2)),
            ("z", ParamValue::Float(-0.0)),
            ("n", ParamValue::Int(100)),
            ("big", ParamValue::Int(i64::MAX - 1)),
            ("m", ParamValue::Int(1)),
            ("b", ParamValue::Bool(true)),
            ("c", ParamValue::Choice("b".to_string())),
            (
                "col",
                ParamValue::Color(Color::Srgb(Rgba::new(1.0, 0.25, 0.1, 0.5))),
            ),
        ];
        let top = mix(&items, 100.0);
        let named = preset_values(&items);
        for (id, v) in named.iter() {
            match (v, top.get(id).unwrap()) {
                (ParamValue::Float(a), ParamValue::Float(b)) => {
                    assert_eq!(a.to_bits(), b.to_bits(), "{id}");
                }
                (a, b) => assert_eq!(a, b, "{id}"),
            }
        }
        let zero = mix(&items, 0.0);
        assert_eq!(zero, schema().defaults());
        let Some(ParamValue::Float(z)) = zero.get("z") else {
            panic!()
        };
        assert_eq!(z.to_bits(), 0.0f64.to_bits());
    }

    // FD-34 (b), D-41: an Int mixes like a Float, rounds, and a half goes away from 0.
    #[test]
    fn fd_34_b_int_rounds_half_away_from_zero() {
        let n = |p: i64, i: f64| mix(&[("n", ParamValue::Int(p))], i).int("n").unwrap();
        assert_eq!(n(5, 50.0), 3);
        assert_eq!(n(-5, 50.0), -3);
        assert_eq!(n(4, 50.0), 2);
        assert_eq!(n(5, 10.0), 1);
        assert_eq!(n(-5, 10.0), -1);
        assert_eq!(n(5, 25.0), 1);
        assert_eq!(n(5, 30.0), 2);
        assert_eq!(n(-5, 30.0), -2);
        // The default counts: m from 10 to 1 at 50 is 5.5, which rounds to 6.
        let m = mix(&[("m", ParamValue::Int(1))], 50.0).int("m").unwrap();
        assert_eq!(m, 6);
        let m = mix(&[("m", ParamValue::Int(0))], 25.0).int("m").unwrap();
        assert_eq!(m, 8);
    }

    // FD-34 (b), D-41: a Color blends in linear light with premultiplied alpha (02 3.4).
    #[test]
    fn fd_34_b_color_blends_in_linear_light() {
        let opaque = |r: f32, g: f32, b: f32| Color::Srgb(Rgba::new(r, g, b, 1.0));
        let white = ParamValue::Color(opaque(1.0, 1.0, 1.0));
        let c = |v: &ParamValues| v.color("col").unwrap().to_linear_srgb();
        let mid = c(&mix(&[("col", white.clone())], 50.0));
        for ch in [mid.r, mid.g, mid.b] {
            assert!((ch - 0.5).abs() < 1e-6, "{mid:?}");
        }
        assert!((mid.a - 1.0).abs() < 1e-6);
        let quarter = c(&mix(&[("col", white)], 25.0));
        assert!((quarter.r - 0.25).abs() < 1e-6, "{quarter:?}");
        // From transparent black to opaque white at 50 is white at alpha 0.5 (premultiplied).
        let mut p = ParamSchema::new(vec![def(
            "col",
            ParamKind::Color {
                default: Color::LinearSrgb(LinearRgba::new(0.0, 0.0, 0.0, 0.0)),
            },
        )]);
        let to = preset_values(&[(
            "col",
            ParamValue::Color(Color::LinearSrgb(LinearRgba::new(1.0, 1.0, 1.0, 1.0))),
        )]);
        let v = mix_params(&p, &to, 50.0).unwrap();
        let l = v.color("col").unwrap().to_linear_srgb();
        assert!(
            (l.r - 1.0).abs() < 1e-6 && (l.a - 0.5).abs() < 1e-6,
            "{l:?}"
        );
        // Another space for the preset: the result is in linear sRGB.
        p = ParamSchema::new(vec![def("col", ParamKind::Color { default: black() })]);
        let p3 = preset_values(&[(
            "col",
            ParamValue::Color(Color::LinearSrgb(LinearRgba::new(1.0, 1.0, 1.0, 1.0))),
        )]);
        let v = mix_params(&p, &p3, 50.0).unwrap();
        assert!(matches!(v.color("col"), Some(Color::LinearSrgb(_))));
    }

    // FD-34 (b), 02 3.4: premultiplied weights when the alphas differ; straight when the result
    // has no alpha (Q-31); the space of both ends when they share it, linear sRGB otherwise.
    #[test]
    fn fd_34_b_color_alpha_and_space_follow_the_interpolation_rule() {
        let lin = |r, g, b, a| Color::LinearSrgb(LinearRgba::new(r, g, b, a));
        let one = |d: Color, p: Color, i: f64| {
            let s = ParamSchema::new(vec![def("col", ParamKind::Color { default: d })]);
            mix_params(&s, &preset_values(&[("col", ParamValue::Color(p))]), i)
                .unwrap()
                .color("col")
                .unwrap()
        };
        // Opaque red to half-transparent blue at 50: alpha 0.75, red 2/3, blue 1/3.
        let m = one(lin(1.0, 0.0, 0.0, 1.0), lin(0.0, 0.0, 1.0, 0.5), 50.0).to_linear_srgb();
        assert!((m.a - 0.75).abs() < 1e-6, "{m:?}");
        assert!(
            (m.r - 2.0 / 3.0).abs() < 1e-6 && (m.b - 1.0 / 3.0).abs() < 1e-6,
            "{m:?}"
        );
        // Both ends transparent: the colours blend straight, with no division by zero.
        let m = one(lin(0.2, 0.4, 0.6, 0.0), lin(1.0, 0.0, 0.0, 0.0), 50.0).to_linear_srgb();
        assert_eq!(m.a, 0.0);
        assert!(
            (m.r - 0.6).abs() < 1e-6 && (m.g - 0.2).abs() < 1e-6 && (m.b - 0.3).abs() < 1e-6,
            "{m:?}"
        );
        // Same space in, same space out; different spaces give linear sRGB.
        let s = |v: f32| Color::Srgb(Rgba::new(v, v, v, 1.0));
        assert!(matches!(one(s(0.0), s(1.0), 50.0), Color::Srgb(_)));
        assert!(matches!(
            one(lin(0.0, 0.0, 0.0, 1.0), lin(1.0, 1.0, 1.0, 1.0), 50.0),
            Color::LinearSrgb(_)
        ));
        assert!(matches!(
            one(s(0.0), lin(1.0, 1.0, 1.0, 1.0), 50.0),
            Color::LinearSrgb(_)
        ));
    }

    // FD-34 (b), D-41: a Bool and a Choice are the default at 0 and the preset's above 0.
    #[test]
    fn fd_34_b_bool_and_choice_jump_above_zero() {
        let items = [
            ("b", ParamValue::Bool(true)),
            ("c", ParamValue::Choice("b".to_string())),
        ];
        for (i, want_on) in [(0.0, false), (0.01, true), (50.0, true), (100.0, true)] {
            let v = mix(&items, i);
            assert_eq!(v.bool("b"), Some(want_on), "{i}");
            assert_eq!(v.choice("c"), Some(if want_on { "b" } else { "a" }), "{i}");
        }
        // A preset that names the default value leaves it there at every intensity.
        let same = [
            ("b", ParamValue::Bool(false)),
            ("c", ParamValue::Choice("a".to_string())),
        ];
        for i in [0.0, 0.01, 100.0] {
            let v = mix(&same, i);
            assert_eq!(v.bool("b"), Some(false));
            assert_eq!(v.choice("c"), Some("a"));
        }
    }

    // FD-34 (b), ARQ-110: NaN, infinities and out of range intensities are Err, the ends pass.
    #[test]
    fn fd_34_b_bad_intensity_is_err_and_the_ends_pass() {
        let p = preset_values(&[("f", ParamValue::Float(40.0))]);
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                mix_params(&schema(), &p, bad),
                Err(FilterError::Effect(EffectError::Buffer(
                    BufferError::NonFinite {
                        field: "intensity",
                        index: 0
                    }
                ))),
                "{bad}"
            );
        }
        for bad in [-0.01, 100.01, -1e300, 1e300] {
            assert_eq!(
                mix_params(&schema(), &p, bad),
                Err(FilterError::Effect(EffectError::Param(
                    ParamError::OutOfRange { param: "intensity" }
                ))),
                "{bad}"
            );
        }
        mix_params(&schema(), &p, 0.0).unwrap();
        mix_params(&schema(), &p, 100.0).unwrap();
        mix_params(&schema(), &p, -0.0).unwrap();
    }

    // FD-34 (b): a preset value outside its range is Err, naming the parameter.
    #[test]
    fn fd_34_b_a_preset_value_out_of_range_is_err() {
        for (id, v, want) in [
            (
                "f",
                ParamValue::Float(100.5),
                ParamError::OutOfRange { param: "f" },
            ),
            (
                "f",
                ParamValue::Float(f64::NAN),
                ParamError::NonFinite { param: "f" },
            ),
            (
                "n",
                ParamValue::Int(101),
                ParamError::OutOfRange { param: "n" },
            ),
            (
                "c",
                ParamValue::Choice("z".to_string()),
                ParamError::OutOfRange { param: "c" },
            ),
            (
                "b",
                ParamValue::Float(1.0),
                ParamError::WrongType { param: "b" },
            ),
        ] {
            for i in [0.0, 50.0, 100.0] {
                assert_eq!(
                    mix_params(&schema(), &preset_values(&[(id, v.clone())]), i),
                    Err(FilterError::Effect(EffectError::Param(want))),
                    "{id} at {i}"
                );
            }
        }
    }

    /// 2 x 2 blocks of constant colour, `bw` by `bh` blocks, opaque.
    fn blocks(bw: usize, bh: usize, seed: u32) -> (Vec<u8>, usize, usize) {
        let (w, h) = (bw * 2, bh * 2);
        let mut v = vec![0u8; w * h * 4];
        for by in 0..bh {
            for bx in 0..bw {
                let x = ((by * bw + bx) as u32 ^ seed).wrapping_mul(2_654_435_761);
                let c = [(x >> 8) as u8, (x >> 16) as u8, (x >> 24) as u8, 255];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    v[((by * 2 + dy) * w + bx * 2 + dx) * 4..][..4].copy_from_slice(&c);
                }
            }
        }
        (v, w, h)
    }

    // FD-34 (a): with the test preset (Contraste and Saturación) and 2 x 2 blocks, the thumbnail
    // on mip level 1 at scale 0.5 equals level 1 of the full-size result, <= 1/255 per channel.
    #[test]
    fn fd_34_a_the_thumbnail_matches_level_one_of_the_full_result() {
        let (l, c) = effects();
        let preset = test_preset();
        for (bw, bh, seed) in [(8usize, 6usize, 1u32), (5, 7, 9), (1, 1, 3), (16, 3, 4)] {
            let (src, w, h) = blocks(bw, bh, seed);
            let size = (w as u32, h as u32);
            for i in [100.0, 60.0, 25.0] {
                let mut full = vec![SENTINEL; src.len()];
                preset
                    .apply(
                        &[&l, &c],
                        &cx(1.0),
                        i,
                        ImageView::new(size.0, size.1, PixelData::Rgba8(&src)).unwrap(),
                        ImageViewMut::new(size.0, size.1, PixelDataMut::Rgba8(&mut full)).unwrap(),
                    )
                    .unwrap();
                assert!(full.iter().any(|b| *b != SENTINEL), "nothing written");
                assert_ne!(full, src, "the preset changed nothing at {i}");
                let src_view = ImageView::new(size.0, size.1, PixelData::Rgba8(&src)).unwrap();
                let small = build_mips(src_view).unwrap().remove(1);
                let full_view = ImageView::new(size.0, size.1, PixelData::Rgba8(&full)).unwrap();
                let want = build_mips(full_view).unwrap().remove(1);
                let crate::PixelBuf::Rgba8(small_px) = small.data() else {
                    panic!()
                };
                let (tw, th) = (small.width(), small.height());
                let mut thumb = vec![SENTINEL; small_px.len()];
                preset
                    .apply(
                        &[&l, &c],
                        &cx(0.5),
                        i,
                        small.view(),
                        ImageViewMut::new(tw, th, PixelDataMut::Rgba8(&mut thumb)).unwrap(),
                    )
                    .unwrap();
                let crate::PixelBuf::Rgba8(want_px) = want.data() else {
                    panic!()
                };
                for (k, (a, b)) in thumb.iter().zip(want_px).enumerate() {
                    assert!(a.abs_diff(*b) <= 1, "{bw}x{bh} i {i} byte {k}: {a} vs {b}");
                }
            }
        }
    }

    // FD-34 (a), `RgbaF32`: the same, <= 1/255.
    #[test]
    fn fd_34_a_the_thumbnail_matches_in_rgba_f32() {
        let (l, c) = effects();
        let preset = test_preset();
        let (bw, bh) = (6usize, 5usize);
        let (w, h) = (bw * 2, bh * 2);
        let mut src = vec![0f32; w * h * 4];
        for by in 0..bh {
            for bx in 0..bw {
                let t = (by * bw + bx) as f32 / (bw * bh) as f32;
                let px = [0.1 + 0.8 * t, 0.7 - 0.5 * t, 0.2 + 0.3 * (1.0 - t), 1.0];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    src[((by * 2 + dy) * w + bx * 2 + dx) * 4..][..4].copy_from_slice(&px);
                }
            }
        }
        let mut full = vec![f32::from_bits(SENTINEL_F32); src.len()];
        preset
            .apply(
                &[&l, &c],
                &cx(1.0),
                100.0,
                ImageView::new(w as u32, h as u32, PixelData::RgbaF32(&src)).unwrap(),
                ImageViewMut::new(w as u32, h as u32, PixelDataMut::RgbaF32(&mut full)).unwrap(),
            )
            .unwrap();
        let small =
            build_mips(ImageView::new(w as u32, h as u32, PixelData::RgbaF32(&src)).unwrap())
                .unwrap()
                .remove(1);
        let want =
            build_mips(ImageView::new(w as u32, h as u32, PixelData::RgbaF32(&full)).unwrap())
                .unwrap()
                .remove(1);
        let (tw, th) = (small.width(), small.height());
        let crate::PixelBuf::RgbaF32(small_px) = small.data() else {
            panic!()
        };
        let mut thumb = vec![f32::from_bits(SENTINEL_F32); small_px.len()];
        preset
            .apply(
                &[&l, &c],
                &cx(0.5),
                100.0,
                small.view(),
                ImageViewMut::new(tw, th, PixelDataMut::RgbaF32(&mut thumb)).unwrap(),
            )
            .unwrap();
        let crate::PixelBuf::RgbaF32(want_px) = want.data() else {
            panic!()
        };
        assert_ne!(thumb, *small_px);
        assert!(thumb.iter().all(|v| v.is_finite()), "nothing written");
        for (k, (a, b)) in thumb.iter().zip(want_px).enumerate() {
            assert!((a - b).abs() <= 1.0 / 255.0, "value {k}: {a} vs {b}");
        }
    }

    // FD-34: the test preset uses Contraste and Saturación, and is valid for their schemas.
    #[test]
    fn fd_34_the_test_preset_is_contrast_and_saturation() {
        let p = test_preset();
        let (l, c) = effects();
        let ids: Vec<&str> = p.steps.iter().map(|s| s.effect.as_str()).collect();
        assert_eq!(ids, [LIGHT, COLOR]);
        assert!(!p.id.is_empty());
        assert!(
            p.steps[0]
                .params
                .float("contrast")
                .is_some_and(|v| v != 0.0)
        );
        assert!(
            p.steps[1]
                .params
                .float("saturation")
                .is_some_and(|v| v != 0.0)
        );
        let resolved = p.resolve(&[&l, &c], 100.0).unwrap();
        assert_eq!(resolved.len(), 2);
        assert_eq!(
            resolved[0].float("contrast"),
            p.steps[0].params.float("contrast")
        );
    }

    // FD-34 (b): intensity 0 is no filter, byte for byte; both formats.
    #[test]
    fn fd_34_b_intensity_zero_is_no_filter_byte_for_byte() {
        let p = test_preset();
        let s8 = img_u8(9, 7, 2);
        let (r, out) = apply8(&p, 1.0, 0.0, &s8, (9, 7));
        r.unwrap();
        assert_eq!(out, s8);
        let sf = img_f32(9, 7, 2);
        let (r, out) = applyf(&p, 1.0, 0.0, &sf, (9, 7));
        r.unwrap();
        assert_eq!(bits(&out), bits(&sf));
        // And -0.0.
        let (r, out) = apply8(&p, 1.0, -0.0, &s8, (9, 7));
        r.unwrap();
        assert_eq!(out, s8);
    }

    // FD-34 (b): intensity 100 runs the effects with the preset's parameters, in order, and a
    // parameter the preset does not name stays at its default.
    #[test]
    fn fd_34_b_intensity_100_runs_the_preset_in_order() {
        let (l, c) = effects();
        let a = preset(vec![
            step(LIGHT, &[("brightness", 60.0)]),
            step(LIGHT, &[("contrast", -80.0)]),
            step(COLOR, &[("hue", 40.0)]),
        ]);
        let b = preset(vec![
            step(LIGHT, &[("contrast", -80.0)]),
            step(LIGHT, &[("brightness", 60.0)]),
            step(COLOR, &[("hue", 40.0)]),
        ]);
        let sf: Vec<f32> = img_f32(8, 6, 3)
            .chunks(4)
            .flat_map(|p| [p[0] * 0.5, p[1] * 0.5, p[2] * 0.5, p[3].max(0.0)])
            .collect();
        let (ra, out_a) = applyf(&a, 1.0, 100.0, &sf, (8, 6));
        ra.unwrap();
        // By hand.
        let one = runf(&l, &cx(1.0), &pv(&[("brightness", 60.0)]), &sf, (8, 6)).unwrap();
        let two = runf(&l, &cx(1.0), &pv(&[("contrast", -80.0)]), &one, (8, 6)).unwrap();
        let three = runf(&c, &cx(1.0), &pv(&[("hue", 40.0)]), &two, (8, 6)).unwrap();
        assert_eq!(bits(&out_a), bits(&three));
        let (rb, out_b) = applyf(&b, 1.0, 100.0, &sf, (8, 6));
        rb.unwrap();
        assert_ne!(bits(&out_a), bits(&out_b), "the order matters");
        // The parameters: named ones as given, the others at their defaults.
        let r = a.resolve(&[&l, &c], 100.0).unwrap();
        assert_eq!(r[0].float("brightness"), Some(60.0));
        assert_eq!(r[0].float("contrast"), Some(0.0));
        assert_eq!(r[2].bool("invert"), Some(false));
        assert_eq!(r[2].float("saturation"), Some(0.0));
        // At 50 the sliders are halfway.
        let r = a.resolve(&[&l, &c], 50.0).unwrap();
        assert_eq!(r[0].float("brightness"), Some(30.0));
        assert_eq!(r[2].float("hue"), Some(20.0));
    }

    // FD-34: a preset with a wrong value, an unknown effect or a bad intensity is Err and
    // `dst` is untouched; zero steps copy.
    #[test]
    fn fd_34_errors_leave_dst_untouched() {
        let s8 = img_u8(4, 3, 1);
        let bad_value = preset(vec![step(LIGHT, &[("brightness", 101.0)])]);
        let (r, dst) = apply8(&bad_value, 1.0, 100.0, &s8, (4, 3));
        assert_eq!(
            r,
            Err(FilterError::Effect(EffectError::Param(
                ParamError::OutOfRange {
                    param: "brightness"
                }
            )))
        );
        assert!(dst.iter().all(|b| *b == SENTINEL));
        let unknown = preset(vec![step(LIGHT, &[]), step("effect.nope", &[])]);
        let (r, dst) = apply8(&unknown, 1.0, 100.0, &s8, (4, 3));
        assert_eq!(r, Err(FilterError::UnknownEffect { step: 1 }));
        assert!(dst.iter().all(|b| *b == SENTINEL));
        for bad in [f64::NAN, f64::INFINITY, -0.01, 100.01] {
            let (r, dst) = apply8(&test_preset(), 1.0, bad, &s8, (4, 3));
            assert!(matches!(r, Err(FilterError::Effect(_))), "{bad}");
            assert!(dst.iter().all(|b| *b == SENTINEL));
        }
        // A bad intensity is Err with no steps too.
        let (r, dst) = apply8(&preset(vec![]), 1.0, 100.5, &s8, (4, 3));
        assert!(r.is_err());
        assert!(dst.iter().all(|b| *b == SENTINEL));
        // No steps: a copy.
        let (r, dst) = apply8(&preset(vec![]), 1.0, 70.0, &s8, (4, 3));
        r.unwrap();
        assert_eq!(dst, s8);
    }

    // ARQ-110: a non-finite `cx.scale` or pixel, an overflow and bad buffers are Err, `dst`
    // untouched, also with steps after the one that fails.
    #[test]
    fn arq_110_filter_rejects_bad_inputs_before_writing() {
        let p = preset(vec![
            step(LIGHT, &[("brightness", 100.0)]),
            step(COLOR, &[("hue", 30.0)]),
        ]);
        let s8 = img_u8(4, 3, 1);
        let (r, dst) = apply8(&p, f32::NAN, 100.0, &s8, (4, 3));
        assert!(matches!(
            r,
            Err(FilterError::Effect(EffectError::Buffer(_)))
        ));
        assert!(dst.iter().all(|b| *b == SENTINEL));
        let (r, dst) = apply8(&preset(vec![]), f32::INFINITY, 100.0, &s8, (4, 3));
        assert!(matches!(
            r,
            Err(FilterError::Effect(EffectError::Buffer(_)))
        ));
        assert!(dst.iter().all(|b| *b == SENTINEL));
        let mut sf = img_f32(4, 3, 1);
        sf[7] = f32::NAN;
        let (r, dst) = applyf(&p, 1.0, 100.0, &sf, (4, 3));
        assert!(matches!(
            r,
            Err(FilterError::Effect(EffectError::Buffer(_)))
        ));
        assert!(dst.iter().all(|b| b.to_bits() == SENTINEL_F32));
        let big = [2.5e38f32, 0.5, 0.5, 1.0];
        let (r, dst) = applyf(&p, 1.0, 100.0, &big, (1, 1));
        assert!(
            matches!(r, Err(FilterError::Effect(EffectError::Buffer(_)))),
            "{r:?}"
        );
        assert!(dst.iter().all(|b| b.to_bits() == SENTINEL_F32));
        // Formats and sizes, with and without steps.
        for pr in [p.clone(), preset(vec![])] {
            let a8 = [1u8, 2, 3, 4];
            let mut d8 = [SENTINEL; 4];
            let r = pr.apply(
                &[&Light::new(), &ColorAdjust::new()],
                &cx(1.0),
                100.0,
                ImageView::new(2, 2, PixelData::Alpha8(&a8)).unwrap(),
                ImageViewMut::new(2, 2, PixelDataMut::Alpha8(&mut d8)).unwrap(),
            );
            assert_eq!(
                r,
                Err(FilterError::Effect(EffectError::UnsupportedFormat {
                    format: PixelFormat::Alpha8
                }))
            );
            let mut df = vec![f32::from_bits(SENTINEL_F32); 16];
            let r = pr.apply(
                &[&Light::new(), &ColorAdjust::new()],
                &cx(1.0),
                100.0,
                ImageView::new(2, 2, PixelData::Rgba8(&s8[..16])).unwrap(),
                ImageViewMut::new(2, 2, PixelDataMut::RgbaF32(&mut df)).unwrap(),
            );
            assert!(matches!(
                r,
                Err(FilterError::Effect(EffectError::FormatMismatch {
                    field: "dst",
                    ..
                }))
            ));
            let mut small = vec![SENTINEL; 8];
            let r = pr.apply(
                &[&Light::new(), &ColorAdjust::new()],
                &cx(1.0),
                100.0,
                ImageView::new(2, 2, PixelData::Rgba8(&s8[..16])).unwrap(),
                ImageViewMut::new(2, 1, PixelDataMut::Rgba8(&mut small)).unwrap(),
            );
            assert!(matches!(
                r,
                Err(FilterError::Effect(EffectError::SizeMismatch {
                    field: "dst",
                    ..
                }))
            ));
            assert!(small.iter().all(|b| *b == SENTINEL));
        }
    }

    // Size 0 is Ok.
    #[test]
    fn filter_empty_images_are_ok() {
        let (r, d) = apply8(&test_preset(), 1.0, 50.0, &[], (0, 0));
        r.unwrap();
        assert!(d.is_empty());
        let (r, _) = applyf(&test_preset(), 1.0, 50.0, &[], (3, 0));
        r.unwrap();
    }

    // ARQ-111
    #[test]
    fn arq_111_filter_is_deterministic() {
        let s8 = img_u8(10, 8, 6);
        let a = apply8(&test_preset(), 1.0, 63.0, &s8, (10, 8));
        let b = apply8(&test_preset(), 1.0, 63.0, &s8, (10, 8));
        assert_eq!(a.1, b.1);
        assert!(a.1.iter().any(|b| *b != SENTINEL), "nothing written");
        assert_ne!(a.1, s8);
    }

    // ADR 0012: the unknown-effect error has a key and delegates the rest.
    #[test]
    fn filter_error_messages() {
        let m = FilterError::UnknownEffect { step: 3 }.to_message();
        assert_eq!(m.key.0, "error.raster.unknown_effect");
        assert_eq!(m.args, vec![("step", gafira_doc::msg::MsgArg::Count(3))]);
        let inner = EffectError::UnsupportedFormat {
            format: PixelFormat::Alpha8,
        };
        assert_eq!(FilterError::from(inner).to_message(), inner.to_message());
    }
}
