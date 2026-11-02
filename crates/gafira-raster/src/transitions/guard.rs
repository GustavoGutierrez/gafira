//! The checks every transition runs before it writes (P15, ARQ-110, ARQ-118).

use gafira_doc::contract::{ParamSchema, ParamValues};

use crate::{
    BufferError, EffectCx, EffectError, ImageView, ImageViewMut, PixelData, PixelDataMut,
    PixelFormat,
};

/// What `run_cpu` does after the checks.
pub(crate) enum Plan {
    /// Size 0: nothing to write.
    Done,
    /// `progress` is 0: copy `from`.
    CopyFrom,
    /// `progress` is 1: copy `to`.
    CopyTo,
    /// Mix with `p` strictly between 0 and 1 and the resolved parameters.
    Mix { p: f64, values: ParamValues },
}

/// Runs the checks of P15 in order, then decides what to do. Nothing is written here.
pub(crate) fn plan(
    cx: &EffectCx,
    schema: &ParamSchema,
    values: &ParamValues,
    progress: f32,
    from: &ImageView<'_>,
    to: &ImageView<'_>,
    dst: &ImageViewMut<'_>,
) -> Result<Plan, EffectError> {
    // 1. `cx.scale` is finite. No transition reads it, but ARQ-110 checks every number.
    if !cx.scale.is_finite() {
        return Err(BufferError::NonFinite {
            field: "cx.scale",
            index: 0,
        }
        .into());
    }
    // 2. progress is finite.
    if !progress.is_finite() {
        return Err(BufferError::NonFinite {
            field: "progress",
            index: 0,
        }
        .into());
    }
    // 3. The parameters fit the schema.
    let resolved = schema.resolve(values)?;
    // 4. Formats: Rgba8 or RgbaF32, the same on all three.
    let format = from.data().format();
    if format == PixelFormat::Alpha8 {
        return Err(EffectError::UnsupportedFormat { format });
    }
    for (field, actual) in [("to", to.data().format()), ("dst", dst.data().format())] {
        if actual != format {
            return Err(EffectError::FormatMismatch {
                field,
                expected: format,
                actual,
            });
        }
    }
    // 5. Dimensions.
    let size = (from.width(), from.height());
    for (field, actual) in [
        ("to", (to.width(), to.height())),
        ("dst", (dst.width(), dst.height())),
    ] {
        if actual != size {
            return Err(EffectError::SizeMismatch {
                field,
                expected: size,
                actual,
            });
        }
    }
    // 6. The f32 pixels that will be read are finite.
    for (field, view) in [("from", from), ("to", to)] {
        if let PixelData::RgbaF32(px) = view.data()
            && let Some(index) = px.iter().position(|v| !v.is_finite())
        {
            return Err(BufferError::NonFinite { field, index }.into());
        }
    }
    // 7. Size 0 is Ok.
    if size.0 == 0 || size.1 == 0 {
        return Ok(Plan::Done);
    }
    // 8. Clip; the ends are copies.
    let p = f64::from(progress.clamp(0.0, 1.0));
    Ok(if p == 0.0 {
        Plan::CopyFrom
    } else if p == 1.0 {
        Plan::CopyTo
    } else {
        Plan::Mix {
            p,
            values: resolved,
        }
    })
}

/// Copies `src` into `dst`; the checks made the formats and sizes equal.
pub(crate) fn copy(dst: &mut ImageViewMut<'_>, src: &ImageView<'_>) {
    match (dst.data_mut(), src.data()) {
        (PixelDataMut::Rgba8(d), PixelData::Rgba8(s)) => d.copy_from_slice(s),
        (PixelDataMut::RgbaF32(d), PixelData::RgbaF32(s)) => d.copy_from_slice(s),
        _ => {}
    }
}

/// Writes `f(i, from[i], to[i])` for every pixel, in the format of the buffers.
pub(crate) fn map_pixels(
    from: &ImageView<'_>,
    to: &ImageView<'_>,
    dst: &mut ImageViewMut<'_>,
    f_u8: impl Fn(usize, [u8; 4], [u8; 4]) -> [u8; 4],
    f_f32: impl Fn(usize, [f32; 4], [f32; 4]) -> [f32; 4],
) {
    match (from.data(), to.data(), dst.data_mut()) {
        (PixelData::Rgba8(a), PixelData::Rgba8(b), PixelDataMut::Rgba8(d)) => {
            for (i, ((a, b), d)) in a
                .chunks_exact(4)
                .zip(b.chunks_exact(4))
                .zip(d.chunks_exact_mut(4))
                .enumerate()
            {
                d.copy_from_slice(&f_u8(i, [a[0], a[1], a[2], a[3]], [b[0], b[1], b[2], b[3]]));
            }
        }
        (PixelData::RgbaF32(a), PixelData::RgbaF32(b), PixelDataMut::RgbaF32(d)) => {
            for (i, ((a, b), d)) in a
                .chunks_exact(4)
                .zip(b.chunks_exact(4))
                .zip(d.chunks_exact_mut(4))
                .enumerate()
            {
                d.copy_from_slice(&f_f32(
                    i,
                    [a[0], a[1], a[2], a[3]],
                    [b[0], b[1], b[2], b[3]],
                ));
            }
        }
        _ => {}
    }
}

/// The two input frames with random access, for transitions that move pixels.
pub(crate) struct Frames<'a, T> {
    from: &'a [T],
    to: &'a [T],
    /// Width in pixels.
    pub(crate) w: usize,
    /// Height in pixels.
    pub(crate) h: usize,
}

impl<T: Copy> Frames<'_, T> {
    /// The pixel at `(x, y)` of `to` if `use_to`, else of `from`. The caller keeps it in range.
    pub(crate) fn px(&self, use_to: bool, x: usize, y: usize) -> [T; 4] {
        let s = if use_to { self.to } else { self.from };
        let i = (y * self.w + x) * 4;
        [s[i], s[i + 1], s[i + 2], s[i + 3]]
    }
}

/// Writes `f(x, y, frames)` for every pixel, in the format of the buffers.
pub(crate) fn render(
    from: &ImageView<'_>,
    to: &ImageView<'_>,
    dst: &mut ImageViewMut<'_>,
    f_u8: impl Fn(usize, usize, &Frames<'_, u8>) -> [u8; 4],
    f_f32: impl Fn(usize, usize, &Frames<'_, f32>) -> [f32; 4],
) {
    let (w, h) = (from.width() as usize, from.height() as usize);
    match (from.data(), to.data(), dst.data_mut()) {
        (PixelData::Rgba8(a), PixelData::Rgba8(b), PixelDataMut::Rgba8(d)) => {
            let frames = Frames {
                from: a,
                to: b,
                w,
                h,
            };
            for (i, px) in d.chunks_exact_mut(4).enumerate() {
                px.copy_from_slice(&f_u8(i % w, i / w, &frames));
            }
        }
        (PixelData::RgbaF32(a), PixelData::RgbaF32(b), PixelDataMut::RgbaF32(d)) => {
            let frames = Frames {
                from: a,
                to: b,
                w,
                h,
            };
            for (i, px) in d.chunks_exact_mut(4).enumerate() {
                px.copy_from_slice(&f_f32(i % w, i / w, &frames));
            }
        }
        _ => {}
    }
}
