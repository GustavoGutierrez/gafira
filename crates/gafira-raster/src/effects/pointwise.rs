//! What the colour-adjustment effects of FD-33 share.

use gafira_doc::contract::{ParamDef, ParamKind};
use gafira_doc::msg::MessageKey;

/// A float slider: id, label key, range and default. All animatable.
pub(super) fn slider(
    id: &'static str,
    label: &'static str,
    min: f64,
    max: f64,
    default: f64,
) -> ParamDef {
    ParamDef {
        id,
        label: MessageKey(label),
        kind: ParamKind::Float { min, max, default },
        animatable: true,
    }
}

/// Smoothstep from `a` to `b` (FD-33 `w(a, b, x)`). With `b <= a` there is no ramp: a step at `a`.
pub(super) fn smoothstep(a: f64, b: f64, x: f64) -> f64 {
    if b <= a {
        return if x < a { 0.0 } else { 1.0 };
    }
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

use crate::transfer::Transfer;
use crate::{
    BufferError, EffectCx, EffectError, ImageView, ImageViewMut, PixelData, PixelDataMut,
    PixelFormat, premultiply_u8, unpremultiply_u8,
};

/// The checks every colour effect makes before it writes: `cx.scale` finite, format, size, and
/// every `f32` pixel finite. Returns the width and height. The parameters are checked by the
/// caller, between `check_scale` and this.
pub(super) fn check_io(
    src: &ImageView<'_>,
    dst: &ImageViewMut<'_>,
) -> Result<(usize, usize), EffectError> {
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
    let size = (src.width(), src.height());
    if (dst.width(), dst.height()) != size {
        return Err(EffectError::SizeMismatch {
            field: "dst",
            expected: size,
            actual: (dst.width(), dst.height()),
        });
    }
    if let PixelData::RgbaF32(px) = src.data()
        && let Some(index) = px.iter().position(|v| !v.is_finite())
    {
        return Err(BufferError::NonFinite {
            field: "src",
            index,
        }
        .into());
    }
    Ok((size.0 as usize, size.1 as usize))
}

/// `cx.scale` must be finite (ARQ-110).
pub(super) fn check_scale(cx: &EffectCx) -> Result<(), EffectError> {
    if cx.scale.is_finite() {
        Ok(())
    } else {
        Err(BufferError::NonFinite {
            field: "cx.scale",
            index: 0,
        }
        .into())
    }
}

/// `(x, y, image_width, image_height)` of `src` in its image: `(0, 0, w, h)` without a window.
pub(super) fn window_of(cx: &EffectCx, w: usize, h: usize) -> (u64, u64, u64, u64) {
    match cx.tile {
        Some(t) => (
            u64::from(t.x),
            u64::from(t.y),
            u64::from(t.image_width),
            u64::from(t.image_height),
        ),
        None => (0, 0, w as u64, h as u64),
    }
}

/// The window of `cx` must hold `src`: `x + w <= image_width` and `y + h <= image_height`.
/// For an effect whose output depends on the position or the size of the image (P16). Call
/// after `check_io`.
pub(super) fn check_tile(cx: &EffectCx, w: usize, h: usize) -> Result<(), EffectError> {
    let (x, y, iw, ih) = window_of(cx, w, h);
    let (right, bottom) = (x + w as u64, y + h as u64);
    if right <= iw && bottom <= ih {
        return Ok(());
    }
    let cut = |v: u64| u32::try_from(v).unwrap_or(u32::MAX);
    Err(EffectError::SizeMismatch {
        field: "cx.tile",
        expected: (cut(iw), cut(ih)),
        actual: (cut(right), cut(bottom)),
    })
}

/// Runs `f` on the straight colour in linear light of every pixel with alpha other than 0 and
/// writes the result premultiplied, with the same alpha. `f(i, c)` gets the pixel index and
/// returns the new colour, or `None` to leave the pixel as it was, byte for byte. A pixel with
/// alpha 0 is copied. `Rgba8` pixels are sRGB bytes, `RgbaF32` pixels are linear light; both
/// premultiplied. An `f32` result that is not finite is `Err` and `dst` is not written.
///
/// Call after `check_io`.
pub(super) fn map_straight<F>(
    src: ImageView<'_>,
    mut dst: ImageViewMut<'_>,
    mut f: F,
) -> Result<(), EffectError>
where
    F: FnMut(usize, [f64; 3]) -> Option<[f64; 3]>,
{
    match (src.data(), dst.data_mut()) {
        (PixelData::Rgba8(s), PixelDataMut::Rgba8(d)) => {
            let t = Transfer::new();
            for (i, (px, out)) in s.chunks_exact(4).zip(d.chunks_exact_mut(4)).enumerate() {
                let a = px[3];
                if a == 0 {
                    out.copy_from_slice(px);
                    continue;
                }
                let c = [0, 1, 2].map(|k| t.decode[usize::from(unpremultiply_u8(px[k], a))]);
                match f(i, c) {
                    None => out.copy_from_slice(px),
                    Some(n) => {
                        let e = t.encode(n);
                        for k in 0..3 {
                            out[k] = premultiply_u8(e[k], a);
                        }
                        out[3] = a;
                    }
                }
            }
            Ok(())
        }
        (PixelData::RgbaF32(s), PixelDataMut::RgbaF32(d)) => {
            // Built apart so an overflowing result leaves `dst` as it was.
            let mut result = s.to_vec();
            for (i, (px, out)) in s
                .chunks_exact(4)
                .zip(result.chunks_exact_mut(4))
                .enumerate()
            {
                let a = f64::from(px[3]);
                if a == 0.0 {
                    continue;
                }
                let c = [0, 1, 2].map(|k| f64::from(px[k]) / a);
                if let Some(n) = f(i, c) {
                    for k in 0..3 {
                        out[k] = (n[k] * a) as f32;
                    }
                }
            }
            if let Some(index) = result.iter().position(|v| !v.is_finite()) {
                return Err(BufferError::NonFinite {
                    field: "result",
                    index,
                }
                .into());
            }
            d.copy_from_slice(&result);
            Ok(())
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // FD-33 `w(a, b, x)`: smoothstep from `a` to `b`; with equal edges it is a step at `a`.
    #[test]
    fn fd_33_smoothstep_values_and_the_step_of_equal_edges() {
        assert_eq!(smoothstep(0.0, 1.0, -1.0), 0.0);
        assert_eq!(smoothstep(0.0, 1.0, 0.0), 0.0);
        assert_eq!(smoothstep(0.0, 1.0, 0.5), 0.5);
        assert_eq!(smoothstep(0.0, 1.0, 1.0), 1.0);
        assert_eq!(smoothstep(0.0, 1.0, 7.0), 1.0);
        assert!((smoothstep(2.0, 4.0, 2.5) - 0.15625).abs() < 1e-12);
        assert_eq!(smoothstep(3.0, 3.0, 2.999), 0.0);
        assert_eq!(smoothstep(3.0, 3.0, 3.0), 1.0);
        assert_eq!(smoothstep(0.0, 0.0, 0.0), 1.0);
        assert_eq!(smoothstep(0.0, 0.0, 12.0), 1.0);
    }
}
