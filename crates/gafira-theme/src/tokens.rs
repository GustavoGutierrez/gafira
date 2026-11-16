//! `gafira.tokens.json`: loading, reference resolution and validation (02 §8.2).
//!
//! The values live only in the JSON (03 §3). This module knows the token
//! *names*, because they are the contract with `gafira-ui`, and the rules of
//! 02 §8.2; it holds no colour, size or duration of its own.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use serde_json::{Map, Value};

use crate::color::{Rgb8, contrast_ratio};
use crate::error::ThemeError;
use crate::fonts;

#[cfg(test)]
mod tests;

/// The tokens file compiled into the binary.
pub const TOKENS_JSON: &str = include_str!("../../../assets/theme/gafira.tokens.json");

/// The newest `schema` this build reads.
pub const SUPPORTED_SCHEMA: u64 = 1;

/// The colour tokens every mode declares (03 §3); `gafira-ui` reads these names.
pub const REQUIRED_MODE_TOKENS: [&str; 45] = [
    "bg",
    "surface",
    "surface.raised",
    "surface.sunken",
    "workspace",
    "border",
    "border.strong",
    "text",
    "text.secondary",
    "text.muted",
    "text.disabled",
    "primary.bg",
    "primary.bg.hover",
    "primary.bg.pressed",
    "primary.fg",
    "secondary.bg",
    "secondary.bg.hover",
    "secondary.bg.pressed",
    "secondary.fg",
    "accent",
    "accent.fg",
    "focus.ring",
    "focus.halo",
    "selection.bg",
    "link",
    "error.fg",
    "error.bg",
    "error.border",
    "success.fg",
    "success.bg",
    "success.border",
    "warning.fg",
    "warning.bg",
    "danger.bg",
    "danger.bg.hover",
    "danger.fg",
    "topbar.bg",
    "topbar.fg",
    "topbar.fg.muted",
    "topbar.hover",
    "canvas.selection",
    "canvas.selection.halo",
    "canvas.guide",
    "canvas.bleed",
    "canvas.safe",
];

/// The type scale steps of 03 §3.
pub const REQUIRED_TEXT_STYLES: [&str; 10] = [
    "display", "h1", "h2", "h3", "body-lg", "body", "body-sm", "caption", "ui-label", "mono-num",
];

const MODES: [&str; 2] = ["light", "dark"];

/// The validated design tokens, as neutral types (no egui).
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeTokens {
    pub schema: u64,
    pub name: String,
    pub brand: Brand,
    pub light: ModeTokens,
    pub dark: ModeTokens,
    pub splash: SplashTokens,
    pub typography: TypeTokens,
    pub space: SpaceTokens,
    pub radius: RadiusTokens,
    pub size: SizeTokens,
    pub border: BorderTokens,
    /// Elevation level (`0`..) to shadow.
    pub shadow: BTreeMap<u8, Shadow>,
    pub motion: MotionTokens,
    /// Layer name to z order.
    pub z: BTreeMap<String, i64>,
    /// Every declared contrast pair, with its computed ratio.
    pub contrast: Vec<CheckedPair>,
    /// The colour pairs that must never be used together (fg, bg).
    pub rejected: Vec<(Rgb8, Rgb8)>,
}

/// The brand palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Brand {
    pub ink: Rgb8,
    pub coral: Rgb8,
    pub canvas: Rgb8,
    pub mint: Rgb8,
    pub mist: Rgb8,
}

/// The resolved colours of one mode, by token name (`"surface.raised"`).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ModeTokens {
    colors: BTreeMap<String, Rgb8>,
}

impl ModeTokens {
    #[must_use]
    pub fn get(&self, token: &str) -> Option<Rgb8> {
        self.colors.get(token).copied()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.colors.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.colors.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, Rgb8)> {
        self.colors.iter().map(|(k, v)| (k.as_str(), *v))
    }
}

/// The Startup screen tokens (D-09, 03 §7.10), in logical px.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplashTokens {
    /// The app mode whose tokens the splash texts use: always `light` today.
    pub mode: String,
    pub panel: Rgb8,
    pub progress_track: Rgb8,
    pub progress_fill: Rgb8,
    pub window: (u32, u32),
    pub panel_height: u32,
    pub panel_padding: u32,
    pub logo_width: u32,
    pub progress_height: u32,
    pub image_min_visible_height: u32,
}

/// Font families and the type scale.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeTokens {
    /// The headings family (`type.family.display`).
    pub display: String,
    /// The interface family (`type.family.ui`).
    pub ui: String,
    /// Scale step (`"h1"`, `"body"`...) to style.
    pub scale: BTreeMap<String, TextStyle>,
}

/// One step of the type scale, in logical px.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextStyle {
    /// The resolved family name, for example `"Inter"`.
    pub family: String,
    pub size: u32,
    pub line: u32,
    pub weight: u32,
    /// OpenType features, for example `"tnum"`.
    pub features: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpaceTokens {
    pub unit: u32,
    pub scale: Vec<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RadiusTokens {
    pub sm: u32,
    pub md: u32,
    pub lg: u32,
    pub pill: u32,
}

/// Small, medium and large variants of a size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SizeSteps {
    pub sm: u32,
    pub md: u32,
    pub lg: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SizeTokens {
    pub control: SizeSteps,
    pub icon: SizeSteps,
    pub target_min: u32,
    pub panel_left: u32,
    pub panel_right: u32,
    pub rail: u32,
    pub topbar: u32,
    pub bottombar: u32,
    pub timeline_min: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BorderTokens {
    pub width: u32,
    pub focus: u32,
    pub focus_offset: u32,
}

/// A drop shadow in logical px, or none.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shadow {
    None,
    Drop {
        x: f32,
        y: f32,
        blur: f32,
        color: Rgb8,
        alpha: f32,
    },
}

/// The three UI transition durations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Durations {
    pub fast: Duration,
    pub base: Duration,
    pub slow: Duration,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MotionTokens {
    pub fast: Duration,
    pub base: Duration,
    pub slow: Duration,
    /// Name to cubic Bézier control points `[x1, y1, x2, y2]`.
    pub easing: BTreeMap<String, [f32; 4]>,
    /// The durations under reduced motion: all zero (ARQ-64).
    pub reduced: Durations,
}

/// A declared contrast pair after validation.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedPair {
    /// `light`, `dark` or `splash`.
    pub group: String,
    pub fg: String,
    pub bg: String,
    pub ratio: f64,
    /// `None` for an exempt pair (minimum 0).
    pub min: Option<f64>,
}

impl ThemeTokens {
    /// The embedded tokens file, validated.
    ///
    /// # Errors
    /// Never for a build whose tests pass; see [`ThemeTokens::from_json`].
    pub fn embedded() -> Result<Self, ThemeError> {
        Self::from_json(TOKENS_JSON)
    }

    /// Parses and validates a tokens file (02 §8.2, steps 1-5).
    ///
    /// # Errors
    /// The first rule the file breaks, as a [`ThemeError`].
    pub fn from_json(text: &str) -> Result<Self, ThemeError> {
        let root: Value = serde_json::from_str(text).map_err(|e| ThemeError::Json {
            detail: e.to_string(),
        })?;
        let root = object(&root, "")?;
        let schema = schema(root)?;
        let name = string(field(root, "", "name")?, "name")?.to_string();

        let colors = resolve_colors(&collect_colors(root)?)?;
        let modes = modes(&colors)?;
        let [light, dark] = modes;
        let brand = Brand {
            ink: color_at(&colors, "brand.ink")?,
            coral: color_at(&colors, "brand.coral")?,
            canvas: color_at(&colors, "brand.canvas")?,
            mint: color_at(&colors, "brand.mint")?,
            mist: color_at(&colors, "brand.mist")?,
        };
        let splash = splash(root, &colors)?;
        let typography = typography(root)?;
        let (contrast, rejected) = contrast(root, &colors)?;

        Ok(Self {
            schema,
            name,
            brand,
            light,
            dark,
            splash,
            typography,
            space: space(root)?,
            radius: radius(root)?,
            size: size(root)?,
            border: border(root)?,
            shadow: shadows(root)?,
            motion: motion(root)?,
            z: z_order(root)?,
            contrast,
            rejected,
        })
    }

    /// The tokens of `dark` (`true`) or light mode.
    #[must_use]
    pub fn mode(&self, dark: bool) -> &ModeTokens {
        if dark { &self.dark } else { &self.light }
    }

    /// Every declared contrast pair and its ratio (DIS-10).
    #[must_use]
    pub fn contrast_report(&self) -> &[CheckedPair] {
        &self.contrast
    }
}

// ---- JSON access with token paths ----------------------------------------

fn join(parent: &str, key: &str) -> String {
    if parent.is_empty() {
        key.to_string()
    } else {
        format!("{parent}.{key}")
    }
}

fn object<'a>(v: &'a Value, path: &str) -> Result<&'a Map<String, Value>, ThemeError> {
    v.as_object().ok_or_else(|| bad(path))
}

fn bad(path: &str) -> ThemeError {
    ThemeError::BadValue {
        path: path.to_string(),
    }
}

fn field<'a>(
    obj: &'a Map<String, Value>,
    parent: &str,
    key: &str,
) -> Result<&'a Value, ThemeError> {
    obj.get(key).ok_or_else(|| ThemeError::MissingToken {
        path: join(parent, key),
    })
}

fn section<'a>(
    root: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a Map<String, Value>, ThemeError> {
    object(field(root, "", key)?, key)
}

fn string<'a>(v: &'a Value, path: &str) -> Result<&'a str, ThemeError> {
    v.as_str().ok_or_else(|| bad(path))
}

fn uint(obj: &Map<String, Value>, parent: &str, key: &str) -> Result<u32, ThemeError> {
    let path = join(parent, key);
    let v = field(obj, parent, key)?;
    v.as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| bad(&path))
}

fn number(v: &Value, path: &str) -> Result<f64, ThemeError> {
    v.as_f64()
        .filter(|n| n.is_finite())
        .ok_or_else(|| bad(path))
}

fn millis(obj: &Map<String, Value>, parent: &str, key: &str) -> Result<Duration, ThemeError> {
    Ok(Duration::from_millis(u64::from(uint(obj, parent, key)?)))
}

// ---- step 1: schema --------------------------------------------------------

fn schema(root: &Map<String, Value>) -> Result<u64, ThemeError> {
    let found = field(root, "", "schema")?
        .as_u64()
        .ok_or_else(|| bad("schema"))?;
    if found > SUPPORTED_SCHEMA {
        Err(ThemeError::SchemaTooNew {
            found,
            supported: SUPPORTED_SCHEMA,
        })
    } else if found == 0 {
        Err(ThemeError::SchemaUnknown { found })
    } else {
        Ok(found)
    }
}

// ---- step 2: colours and references ----------------------------------------

/// A colour as written: a value or a reference to another colour's path.
enum RawColor {
    Value(Rgb8),
    Ref(String),
}

fn raw_color(path: &str, v: &Value) -> Result<RawColor, ThemeError> {
    let text = string(v, path)?;
    if let Some(target) = text.strip_prefix('{').and_then(|t| t.strip_suffix('}')) {
        return Ok(RawColor::Ref(target.to_string()));
    }
    Rgb8::parse_hex(text)
        .map(RawColor::Value)
        .ok_or_else(|| ThemeError::BadColor {
            path: path.to_string(),
            value: text.to_string(),
        })
}

/// Every colour-valued token, by full path: `brand.*`, `modes.<mode>.*` and the
/// string tokens of `splash` other than `mode`.
fn collect_colors(root: &Map<String, Value>) -> Result<BTreeMap<String, RawColor>, ThemeError> {
    let mut out = BTreeMap::new();
    for (key, v) in section(root, "brand")? {
        let path = join("brand", key);
        out.insert(path.clone(), raw_color(&path, v)?);
    }
    let modes = section(root, "modes")?;
    for mode in MODES {
        let parent = join("modes", mode);
        for (key, v) in object(field(modes, "modes", mode)?, &parent)? {
            let path = join(&parent, key);
            out.insert(path.clone(), raw_color(&path, v)?);
        }
    }
    for (key, v) in section(root, "splash")? {
        if key != "mode" && v.is_string() {
            let path = join("splash", key);
            out.insert(path.clone(), raw_color(&path, v)?);
        }
    }
    Ok(out)
}

fn resolve_colors(raw: &BTreeMap<String, RawColor>) -> Result<BTreeMap<String, Rgb8>, ThemeError> {
    let mut done = BTreeMap::new();
    for path in raw.keys() {
        let mut chain: Vec<&str> = Vec::new();
        let mut current = path.as_str();
        let value = loop {
            if let Some(v) = done.get(current) {
                break *v;
            }
            if chain.contains(&current) {
                return Err(ThemeError::ReferenceCycle {
                    path: current.to_string(),
                });
            }
            chain.push(current);
            match raw.get(current) {
                Some(RawColor::Value(v)) => break *v,
                Some(RawColor::Ref(target)) => {
                    if !raw.contains_key(target) {
                        return Err(ThemeError::UnresolvedReference {
                            path: current.to_string(),
                            target: target.clone(),
                        });
                    }
                    current = target;
                }
                // Unreachable: `current` is `path` or a target checked above.
                None => {
                    return Err(ThemeError::MissingToken {
                        path: current.to_string(),
                    });
                }
            }
        };
        for p in chain {
            done.insert(p.to_string(), value);
        }
    }
    Ok(done)
}

fn color_at(colors: &BTreeMap<String, Rgb8>, path: &str) -> Result<Rgb8, ThemeError> {
    colors
        .get(path)
        .copied()
        .ok_or_else(|| ThemeError::MissingToken {
            path: path.to_string(),
        })
}

// ---- step 3: the two modes -------------------------------------------------

fn mode_colors(colors: &BTreeMap<String, Rgb8>, mode: &str) -> ModeTokens {
    let prefix = format!("modes.{mode}.");
    ModeTokens {
        colors: colors
            .iter()
            .filter_map(|(k, v)| k.strip_prefix(&prefix).map(|t| (t.to_string(), *v)))
            .collect(),
    }
}

fn modes(colors: &BTreeMap<String, Rgb8>) -> Result<[ModeTokens; 2], ThemeError> {
    let [light, dark] = MODES.map(|m| mode_colors(colors, m));
    let names = |m: &ModeTokens| m.colors.keys().cloned().collect::<BTreeSet<_>>();
    let (l, d) = (names(&light), names(&dark));
    if let Some(token) = l.difference(&d).next() {
        return Err(ThemeError::ModeMismatch {
            token: token.clone(),
            missing_in: "dark".into(),
        });
    }
    if let Some(token) = d.difference(&l).next() {
        return Err(ThemeError::ModeMismatch {
            token: token.clone(),
            missing_in: "light".into(),
        });
    }
    if let Some(token) = REQUIRED_MODE_TOKENS.iter().find(|t| !l.contains(**t)) {
        return Err(ThemeError::MissingToken {
            path: format!("modes.light.{token}"),
        });
    }
    Ok([light, dark])
}

// ---- other sections ----------------------------------------------------------

fn splash(
    root: &Map<String, Value>,
    colors: &BTreeMap<String, Rgb8>,
) -> Result<SplashTokens, ThemeError> {
    let s = section(root, "splash")?;
    let mode = string(field(s, "splash", "mode")?, "splash.mode")?;
    if !MODES.contains(&mode) {
        return Err(bad("splash.mode"));
    }
    let window = field(s, "splash", "window")?
        .as_array()
        .filter(|a| a.len() == 2)
        .ok_or_else(|| bad("splash.window"))?;
    let dim = |i: usize| {
        window[i]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| bad("splash.window"))
    };
    Ok(SplashTokens {
        mode: mode.to_string(),
        panel: color_at(colors, "splash.panel")?,
        progress_track: color_at(colors, "splash.progress.track")?,
        progress_fill: color_at(colors, "splash.progress.fill")?,
        window: (dim(0)?, dim(1)?),
        panel_height: uint(s, "splash", "panel.height")?,
        panel_padding: uint(s, "splash", "panel.padding")?,
        logo_width: uint(s, "splash", "logo.width")?,
        progress_height: uint(s, "splash", "progress.height")?,
        image_min_visible_height: uint(s, "splash", "image.min_visible_height")?,
    })
}

fn typography(root: &Map<String, Value>) -> Result<TypeTokens, ThemeError> {
    let t = section(root, "type")?;
    let families = object(field(t, "type", "family")?, "type.family")?;
    let family = |role: &str| -> Result<String, ThemeError> {
        let path = join("type.family", role);
        let name = string(field(families, "type.family", role)?, &path)?;
        fonts::font(name)
            .map(|f| f.family.to_string())
            .ok_or_else(|| ThemeError::UnknownFamily {
                family: name.to_string(),
            })
    };
    let display = family("display")?;
    let ui = family("ui")?;
    let scale_obj = object(field(t, "type", "scale")?, "type.scale")?;
    if let Some(step) = REQUIRED_TEXT_STYLES
        .iter()
        .find(|s| !scale_obj.contains_key(**s))
    {
        return Err(ThemeError::MissingToken {
            path: format!("type.scale.{step}"),
        });
    }
    let mut scale = BTreeMap::new();
    for (step, v) in scale_obj {
        let parent = join("type.scale", step);
        let o = object(v, &parent)?;
        let role_path = join(&parent, "family");
        let role = string(field(o, &parent, "family")?, &role_path)?;
        let features = match o.get("features") {
            None => Vec::new(),
            Some(f) => {
                let path = join(&parent, "features");
                f.as_array()
                    .ok_or_else(|| bad(&path))?
                    .iter()
                    .map(|x| string(x, &path).map(str::to_string))
                    .collect::<Result<_, _>>()?
            }
        };
        let style = TextStyle {
            family: family(role)?,
            size: uint(o, &parent, "size")?,
            line: uint(o, &parent, "line")?,
            weight: uint(o, &parent, "weight")?,
            features,
        };
        scale.insert(step.clone(), style);
    }
    Ok(TypeTokens { display, ui, scale })
}

fn space(root: &Map<String, Value>) -> Result<SpaceTokens, ThemeError> {
    let s = section(root, "space")?;
    let scale = field(s, "space", "scale")?
        .as_array()
        .ok_or_else(|| bad("space.scale"))?
        .iter()
        .map(|v| {
            v.as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| bad("space.scale"))
        })
        .collect::<Result<_, _>>()?;
    Ok(SpaceTokens {
        unit: uint(s, "space", "unit")?,
        scale,
    })
}

fn radius(root: &Map<String, Value>) -> Result<RadiusTokens, ThemeError> {
    let r = section(root, "radius")?;
    Ok(RadiusTokens {
        sm: uint(r, "radius", "sm")?,
        md: uint(r, "radius", "md")?,
        lg: uint(r, "radius", "lg")?,
        pill: uint(r, "radius", "pill")?,
    })
}

fn steps(obj: &Map<String, Value>, parent: &str, key: &str) -> Result<SizeSteps, ThemeError> {
    let path = join(parent, key);
    let o = object(field(obj, parent, key)?, &path)?;
    Ok(SizeSteps {
        sm: uint(o, &path, "sm")?,
        md: uint(o, &path, "md")?,
        lg: uint(o, &path, "lg")?,
    })
}

fn size(root: &Map<String, Value>) -> Result<SizeTokens, ThemeError> {
    let s = section(root, "size")?;
    Ok(SizeTokens {
        control: steps(s, "size", "control")?,
        icon: steps(s, "size", "icon")?,
        target_min: uint(s, "size", "target.min")?,
        panel_left: uint(s, "size", "panel.left")?,
        panel_right: uint(s, "size", "panel.right")?,
        rail: uint(s, "size", "rail")?,
        topbar: uint(s, "size", "topbar")?,
        bottombar: uint(s, "size", "bottombar")?,
        timeline_min: uint(s, "size", "timeline.min")?,
    })
}

fn border(root: &Map<String, Value>) -> Result<BorderTokens, ThemeError> {
    let b = section(root, "border")?;
    Ok(BorderTokens {
        width: uint(b, "border", "width")?,
        focus: uint(b, "border", "focus")?,
        focus_offset: uint(b, "border", "focus.offset")?,
    })
}

fn shadows(root: &Map<String, Value>) -> Result<BTreeMap<u8, Shadow>, ThemeError> {
    let mut out = BTreeMap::new();
    for (key, v) in section(root, "shadow")? {
        let path = join("shadow", key);
        let level: u8 = key.parse().map_err(|_| bad(&path))?;
        let text = string(v, &path)?;
        let shadow = parse_shadow(text).ok_or_else(|| ThemeError::BadShadow {
            path: path.clone(),
            value: text.to_string(),
        })?;
        out.insert(level, shadow);
    }
    Ok(out)
}

/// `none` or `x y blur rgba(r,g,b,a)`, lengths in px (`0` may omit the unit).
fn parse_shadow(text: &str) -> Option<Shadow> {
    if text == "none" {
        return Some(Shadow::None);
    }
    let (lengths, color) = text.split_once("rgba(")?;
    let lengths: Vec<f32> = lengths
        .split_whitespace()
        .map(|l| {
            l.strip_suffix("px")
                .unwrap_or(l)
                .parse::<f32>()
                .ok()
                .filter(|n| n.is_finite())
        })
        .collect::<Option<_>>()?;
    let [x, y, blur] = lengths[..] else {
        return None;
    };
    let parts: Vec<&str> = color.strip_suffix(')')?.split(',').map(str::trim).collect();
    let [r, g, b, a] = parts[..] else { return None };
    let alpha: f32 = a.parse().ok().filter(|a: &f32| (0.0..=1.0).contains(a))?;
    if blur < 0.0 {
        return None;
    }
    Some(Shadow::Drop {
        x,
        y,
        blur,
        color: Rgb8::new(r.parse().ok()?, g.parse().ok()?, b.parse().ok()?),
        alpha,
    })
}

fn motion(root: &Map<String, Value>) -> Result<MotionTokens, ThemeError> {
    let m = section(root, "motion")?;
    let easing_obj = object(field(m, "motion", "easing")?, "motion.easing")?;
    let mut easing = BTreeMap::new();
    for (name, v) in easing_obj {
        let path = join("motion.easing", name);
        let points = v
            .as_array()
            .filter(|a| a.len() == 4)
            .ok_or_else(|| bad(&path))?;
        let mut curve = [0.0f32; 4];
        for (slot, p) in curve.iter_mut().zip(points) {
            // The control points are small fractions; f32 is the egui type.
            *slot = number(p, &path)? as f32;
        }
        easing.insert(name.clone(), curve);
    }
    let reduced_obj = object(field(m, "motion", "reduced")?, "motion.reduced")?;
    let reduced = Durations {
        fast: millis(reduced_obj, "motion.reduced", "fast")?,
        base: millis(reduced_obj, "motion.reduced", "base")?,
        slow: millis(reduced_obj, "motion.reduced", "slow")?,
    };
    for (name, d) in [
        ("fast", reduced.fast),
        ("base", reduced.base),
        ("slow", reduced.slow),
    ] {
        if d != Duration::ZERO {
            return Err(ThemeError::ReducedMotionNotZero { name: name.into() });
        }
    }
    Ok(MotionTokens {
        fast: millis(m, "motion", "fast")?,
        base: millis(m, "motion", "base")?,
        slow: millis(m, "motion", "slow")?,
        easing,
        reduced,
    })
}

fn z_order(root: &Map<String, Value>) -> Result<BTreeMap<String, i64>, ThemeError> {
    section(root, "z")?
        .iter()
        .map(|(k, v)| {
            v.as_i64()
                .map(|n| (k.clone(), n))
                .ok_or_else(|| bad(&join("z", k)))
        })
        .collect()
}

// ---- step 4: contrast --------------------------------------------------------

type Contrast = (Vec<CheckedPair>, Vec<(Rgb8, Rgb8)>);

fn contrast(
    root: &Map<String, Value>,
    colors: &BTreeMap<String, Rgb8>,
) -> Result<Contrast, ThemeError> {
    let c = section(root, "contrast")?;
    let mins = object(field(c, "contrast", "min")?, "contrast.min")?;
    let mut usage = BTreeMap::new();
    for (name, v) in mins {
        let min = number(v, &join("contrast.min", name))?;
        // A minimum of 0 marks an exempt usage (WCAG 1.4.3, disabled text).
        usage.insert(name.as_str(), (min > 0.0).then_some(min));
    }
    let text_min =
        usage
            .get("text")
            .copied()
            .flatten()
            .ok_or_else(|| ThemeError::MissingToken {
                path: "contrast.min.text".into(),
            })?;

    let rejected = rejected_pairs(c, text_min)?;

    let mut checked = Vec::new();
    for (group, list) in object(field(c, "contrast", "pairs")?, "contrast.pairs")? {
        let parent = join("contrast.pairs", group);
        let prefix = match group.as_str() {
            "light" | "dark" => join("modes", group),
            "splash" => "splash".to_string(),
            _ => return Err(bad(&parent)),
        };
        for item in list.as_array().ok_or_else(|| bad(&parent))? {
            let triple = item
                .as_array()
                .filter(|a| a.len() == 3)
                .and_then(|a| Some((a[0].as_str()?, a[1].as_str()?, a[2].as_str()?)))
                .ok_or_else(|| bad(&parent))?;
            let (fg, bg, use_name) = triple;
            let min = *usage
                .get(use_name)
                .ok_or_else(|| ThemeError::UnknownUsage {
                    group: group.clone(),
                    usage: use_name.to_string(),
                })?;
            let fg_color = color_at(colors, &join(&prefix, fg))?;
            let bg_color = color_at(colors, &join(&prefix, bg))?;
            if rejected.contains(&(fg_color, bg_color)) {
                return Err(ThemeError::RejectedPairDeclared {
                    group: group.clone(),
                    fg: fg.to_string(),
                    bg: bg.to_string(),
                });
            }
            let ratio = contrast_ratio(fg_color, bg_color);
            if let Some(min) = min
                && ratio < min
            {
                return Err(ThemeError::Contrast {
                    group: group.clone(),
                    fg: fg.to_string(),
                    bg: bg.to_string(),
                    ratio,
                    min,
                });
            }
            checked.push(CheckedPair {
                group: group.clone(),
                fg: fg.to_string(),
                bg: bg.to_string(),
                ratio,
                min,
            });
        }
    }
    Ok((checked, rejected))
}

fn rejected_pairs(c: &Map<String, Value>, text_min: f64) -> Result<Vec<(Rgb8, Rgb8)>, ThemeError> {
    let list = field(c, "contrast", "rejected")?
        .as_array()
        .ok_or_else(|| bad("contrast.rejected"))?;
    let mut out = Vec::new();
    for item in list {
        let pair = item
            .as_array()
            .filter(|a| a.len() == 2)
            .ok_or_else(|| bad("contrast.rejected"))?;
        let parse = |v: &Value| match raw_color("contrast.rejected", v)? {
            RawColor::Value(c) => Ok(c),
            RawColor::Ref(_) => Err(bad("contrast.rejected")),
        };
        let (fg, bg) = (parse(&pair[0])?, parse(&pair[1])?);
        if contrast_ratio(fg, bg) >= text_min {
            return Err(ThemeError::RejectedPairPasses {
                fg: fg.to_string(),
                bg: bg.to_string(),
            });
        }
        out.push((fg, bg));
    }
    Ok(out)
}
