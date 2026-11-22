//! An image made to look as its document says (spec 085): its adjustments,
//! those of its zones, its filter and its shadow, in that order.

mod cast;
mod filters;

use gafira_doc::contract::{ParamValue, ParamValues};
use gafira_doc::{AssetRef, ImageAdjust, ImageLook};

use crate::effects::{
    ColorAdjust, DETECTED_SLOTS, DetectedColors, Effect, Light, Texture, WhiteBalance,
};
use crate::{
    AlphaMask, EffectCx, EffectError, ImageBuf, ImageView, ImageViewMut, PixelBuf, PixelData,
    PixelDataMut, PixelFormat,
};

pub use cast::cast;
pub use filters::{FILTER_GROUPS, Filter, FilterGroup, filter};

/// `image` (8-bit) as `look` says; `mask` gives the mask of a zone, and a zone
/// whose mask is lost changes nothing. `scale` is how much smaller than the
/// whole one it is (ARQ-120).
///
/// # Errors
///
/// [`EffectError`] for another format or a number out of range.
pub fn apply_look(
    look: &ImageLook,
    image: &ImageBuf,
    mask: &dyn Fn(AssetRef) -> Option<AlphaMask>,
    scale: f32,
) -> Result<ImageBuf, EffectError> {
    let PixelBuf::Rgba8(bytes) = image.data() else {
        return Err(unsupported(image));
    };
    let size = (image.width(), image.height());
    let mut pixels = adjusted(&look.adjust, bytes.clone(), size, scale)?;
    for zone in &look.zones {
        if zone.adjust.is_plain() {
            continue;
        }
        let Some(mask) = mask(zone.mask) else {
            continue;
        };
        let changed = adjusted(&zone.adjust, pixels.clone(), size, scale)?;
        blend(&mut pixels, &changed, size, &mask, zone.inverted);
    }
    if let Some(chosen) = &look.filter
        && let Some(known) = filter(&chosen.id)
    {
        let share = chosen.intensity / 100.0;
        pixels = adjusted(&at_share(&known.first, share), pixels, size, scale)?;
        pixels = adjusted(&at_share(&known.then, share), pixels, size, scale)?;
    }
    let out = ImageBuf::new(size.0, size.1, PixelBuf::Rgba8(pixels))?;
    match &look.shadow {
        Some(shadow) => cast(&out, shadow),
        None => Ok(out),
    }
}

/// The mask an 8-bit picture stands for: its light where it is opaque, its
/// opacity where it is white.
#[must_use]
pub fn mask_of(picture: &ImageBuf) -> Option<AlphaMask> {
    let PixelBuf::Rgba8(bytes) = picture.data() else {
        return None;
    };
    let data = bytes.chunks_exact(4).map(|px| px[0]).collect();
    AlphaMask::new(picture.width(), picture.height(), data).ok()
}

/// `image` (8-bit) no longer than `most`, each pixel the mean of those it
/// covers, and how much smaller it is; none when it is already so.
#[must_use]
pub fn reduced(image: &ImageBuf, most: u32) -> Option<(ImageBuf, f32)> {
    let PixelBuf::Rgba8(bytes) = image.data() else {
        return None;
    };
    let (w, h) = (image.width(), image.height());
    let k = w
        .max(h)
        .checked_div(most)
        .map(|_| w.max(h).div_ceil(most))?;
    if k <= 1 {
        return None;
    }
    let (out_w, out_h) = (w.div_ceil(k), h.div_ceil(k));
    let (wide, k_wide) = (w as usize, k as usize);
    let mut sums = vec![[0_u32; 5]; out_w as usize * out_h as usize];
    for (y, row) in bytes.chunks_exact(wide * 4).enumerate() {
        for (x, px) in row.chunks_exact(4).enumerate() {
            let sum = &mut sums[y / k_wide * out_w as usize + x / k_wide];
            for (sum, v) in sum.iter_mut().zip(px) {
                *sum += u32::from(*v);
            }
            sum[4] += 1;
        }
    }
    let data = sums
        .iter()
        .flat_map(|[r, g, b, a, n]| [r, g, b, a].map(|v| ((v + n / 2) / n.max(&1)) as u8))
        .collect();
    let made = ImageBuf::new(out_w, out_h, PixelBuf::Rgba8(data)).ok()?;
    Some((made, 1.0 / k as f32))
}

fn unsupported(image: &ImageBuf) -> EffectError {
    EffectError::UnsupportedFormat {
        format: match image.data() {
            PixelBuf::Rgba8(_) => PixelFormat::Rgba8,
            PixelBuf::RgbaF32(_) => PixelFormat::RgbaF32,
            PixelBuf::Alpha8(_) => PixelFormat::Alpha8,
        },
    }
}

/// `adjust` at a share of its strength: every slider starts at 0.
fn at_share(adjust: &ImageAdjust, share: f64) -> ImageAdjust {
    ImageAdjust {
        temperature: adjust.temperature * share,
        tint: adjust.tint * share,
        brightness: adjust.brightness * share,
        contrast: adjust.contrast * share,
        highlights: adjust.highlights * share,
        shadows: adjust.shadows * share,
        whites: adjust.whites * share,
        blacks: adjust.blacks * share,
        hue: adjust.hue * share,
        saturation: adjust.saturation * share,
        vibrance: adjust.vibrance * share,
        invert: adjust.invert && share > 0.0,
        sharpness: adjust.sharpness * share,
        clarity: adjust.clarity * share,
        vignette: adjust.vignette * share,
        colors: Vec::new(),
    }
}

/// `pixels` with the adjustments that change something, in the order of FD-33.
fn adjusted(
    a: &ImageAdjust,
    mut pixels: Vec<u8>,
    size: (u32, u32),
    scale: f32,
) -> Result<Vec<u8>, EffectError> {
    let white = [("temperature", a.temperature), ("tint", a.tint)];
    let light = [
        ("brightness", a.brightness),
        ("contrast", a.contrast),
        ("highlights", a.highlights),
        ("shadows", a.shadows),
        ("whites", a.whites),
        ("blacks", a.blacks),
    ];
    let color = [
        ("hue", a.hue),
        ("saturation", a.saturation),
        ("vibrance", a.vibrance),
    ];
    let texture = [
        ("sharpness", a.sharpness),
        ("clarity", a.clarity),
        ("vignette", a.vignette),
    ];
    if changes(&white) {
        pixels = run(&WhiteBalance::new(), &numbers(&white), &pixels, size, scale)?;
    }
    if changes(&light) {
        pixels = run(&Light::new(), &numbers(&light), &pixels, size, scale)?;
    }
    if changes(&color) || a.invert {
        let mut values = numbers(&color);
        values.set("invert", ParamValue::Bool(a.invert));
        pixels = run(&ColorAdjust::new(), &values, &pixels, size, scale)?;
    }
    if a.colors.iter().any(|c| c.hue != 0.0 || c.saturation != 0.0) {
        pixels = run(&DetectedColors::new(), &shifts(a), &pixels, size, scale)?;
    }
    if changes(&texture) {
        pixels = run(&Texture::new(), &numbers(&texture), &pixels, size, scale)?;
    }
    Ok(pixels)
}

fn changes(list: &[(&'static str, f64)]) -> bool {
    list.iter().any(|(_, v)| *v != 0.0)
}

fn numbers(list: &[(&'static str, f64)]) -> ParamValues {
    let mut values = ParamValues::new();
    for (id, v) in list {
        values.set(*id, ParamValue::Float(*v));
    }
    values
}

/// The parameters of the colours moved.
fn shifts(a: &ImageAdjust) -> ParamValues {
    let mut values = ParamValues::new();
    for (def, shift) in DetectedColors::new()
        .params()
        .params()
        .chunks(4)
        .zip(a.colors.iter().take(DETECTED_SLOTS))
    {
        // Four per sample: its colour, its hue, its saturation, its tolerance.
        if let [color, hue, saturation, _] = def {
            values.set(color.id, ParamValue::Color(shift.color));
            values.set(hue.id, ParamValue::Float(shift.hue));
            values.set(saturation.id, ParamValue::Float(shift.saturation));
        }
    }
    values
}

fn run(
    effect: &dyn Effect,
    values: &ParamValues,
    pixels: &[u8],
    (w, h): (u32, u32),
    scale: f32,
) -> Result<Vec<u8>, EffectError> {
    let mut out = vec![0; pixels.len()];
    effect.run_cpu(
        &EffectCx::whole(scale),
        values,
        ImageView::new(w, h, PixelData::Rgba8(pixels))?,
        ImageViewMut::new(w, h, PixelDataMut::Rgba8(&mut out))?,
    )?;
    Ok(out)
}

/// `changed` over `pixels` as far as `mask`, stretched to them, says.
fn blend(pixels: &mut [u8], changed: &[u8], (w, h): (u32, u32), mask: &AlphaMask, inverted: bool) {
    let (mw, mh) = (mask.width() as usize, mask.height() as usize);
    if mw == 0 || mh == 0 || w == 0 {
        return;
    }
    let columns: Vec<(usize, usize, f32)> = (0..w).map(|x| between(x, w, mw)).collect();
    let data = mask.data();
    let row = w as usize * 4;
    for (y, (to, from)) in pixels
        .chunks_exact_mut(row)
        .zip(changed.chunks_exact(row))
        .enumerate()
    {
        // A row of the image: the cast is exact.
        let (top, bottom, down) = between(y as u32, h, mh);
        let (top, bottom) = (&data[top * mw..][..mw], &data[bottom * mw..][..mw]);
        for ((to, from), (left, right, across)) in to
            .chunks_exact_mut(4)
            .zip(from.chunks_exact(4))
            .zip(&columns)
        {
            let mix = |a: f32, b: f32, t: f32| a + (b - a) * t;
            let at = |line: &[u8]| mix(f32::from(line[*left]), f32::from(line[*right]), *across);
            let mut share = mix(at(top), at(bottom), down) / 255.0;
            if inverted {
                share = 1.0 - share;
            }
            if share <= 0.0 {
                continue;
            }
            for (to, from) in to.iter_mut().zip(from) {
                // Between two bytes: it is one.
                *to = (mix(f32::from(*to), f32::from(*from), share) + 0.5) as u8;
            }
        }
    }
}

/// The two samples of a line of `n` around the pixel `at` of `len`, and how
/// far it is between them.
fn between(at: u32, len: u32, n: usize) -> (usize, usize, f32) {
    let last = n - 1;
    let x = ((f64::from(at) + 0.5) / f64::from(len) * n as f64 - 0.5).clamp(0.0, last as f64);
    let low = (x.floor() as usize).min(last);
    (low, (low + 1).min(last), (x - low as f64) as f32)
}
