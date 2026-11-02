//! The checks every transition runs before it writes (P15, ARQ-110, ARQ-118).

use gafira_doc::contract::{ParamSchema, ParamValues};

use crate::{
    BufferError, EffectError, ImageView, ImageViewMut, PixelData, PixelDataMut, PixelFormat,
};

/// What `run_cpu` does after the checks.
pub(crate) enum Plan {
    /// Size 0: nothing to write.
    Done,
    /// `progress` is 0: copy `from`.
    CopyFrom,
    /// `progress` is 1: copy `to`.
    CopyTo,
    /// Mix with `p` strictly between 0 and 1.
    Mix { p: f64 },
}

/// Runs the checks of P15 in order, then decides what to do. Nothing is written here.
pub(crate) fn plan(
    schema: &ParamSchema,
    values: &ParamValues,
    progress: f32,
    from: &ImageView<'_>,
    to: &ImageView<'_>,
    dst: &ImageViewMut<'_>,
) -> Result<Plan, EffectError> {
    // 1. progress is finite.
    if !progress.is_finite() {
        return Err(BufferError::NonFinite {
            field: "progress",
            index: 0,
        }
        .into());
    }
    // 2. The parameters fit the schema.
    schema.resolve(values)?;
    // 3. Formats: Rgba8 or RgbaF32, the same on all three.
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
    // 4. Dimensions.
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
    // 5. The f32 pixels that will be read are finite.
    for (field, view) in [("from", from), ("to", to)] {
        if let PixelData::RgbaF32(px) = view.data()
            && let Some(index) = px.iter().position(|v| !v.is_finite())
        {
            return Err(BufferError::NonFinite { field, index }.into());
        }
    }
    // 6. Size 0 is Ok.
    if size.0 == 0 || size.1 == 0 {
        return Ok(Plan::Done);
    }
    // 7. Clip; the ends are copies.
    let p = f64::from(progress.clamp(0.0, 1.0));
    Ok(if p == 0.0 {
        Plan::CopyFrom
    } else if p == 1.0 {
        Plan::CopyTo
    } else {
        Plan::Mix { p }
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
