//! The box layout and its input and output types.

use gafira_geom::Size;

use crate::TextError;

/// Text style numbers (FD-41).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    /// Font size in points, in `[1, 1000]`.
    pub size_pt: f64,
    /// Letter spacing in thousandths of an em, in `[-100, 500]`.
    pub letter_spacing: f64,
    /// Line height multiplier, in `[0.5, 4.0]`.
    pub line_height: f64,
}

/// The laid out text.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextLayout {}

/// Lays `text` out in a box of `bounds` px.
///
/// `fonts` is the ordered font stack, as file bytes. All numbers are checked
/// before any work: a NaN or an infinity (also a computed one) gives
/// `Err(NonFinite)` and a style value outside its range gives `Err(OutOfRange)`,
/// each naming the field (ARQ-110, FD-41). `style.letter_spacing` and
/// `style.line_height` are checked here and applied from the next delivery.
pub fn layout_box(
    _text: &str,
    _fonts: &[&[u8]],
    style: &TextStyle,
    dpi: f64,
    bounds: Size,
) -> Result<TextLayout, TextError> {
    let _size_px = check_input(style, dpi, bounds)?;
    Ok(TextLayout::default())
}

const SIZE_PT: (f64, f64) = (1.0, 1000.0);
const LETTER_SPACING: (f64, f64) = (-100.0, 500.0);
const LINE_HEIGHT: (f64, f64) = (0.5, 4.0);

fn finite(field: &'static str, value: f64) -> Result<f64, TextError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(TextError::NonFinite { field })
    }
}

fn in_range(field: &'static str, value: f64, (lo, hi): (f64, f64)) -> Result<(), TextError> {
    if (lo..=hi).contains(&value) {
        Ok(())
    } else {
        Err(TextError::OutOfRange { field })
    }
}

/// Non-finite checks of every input first, then the ranges, then the computed size.
/// Returns the font size in px.
fn check_input(style: &TextStyle, dpi: f64, bounds: Size) -> Result<f32, TextError> {
    let size_pt = finite("style.size_pt", style.size_pt)?;
    let spacing = finite("style.letter_spacing", style.letter_spacing)?;
    let line_height = finite("style.line_height", style.line_height)?;
    finite("dpi", dpi)?;
    finite("bounds.width", bounds.width())?;
    finite("bounds.height", bounds.height())?;
    in_range("style.size_pt", size_pt, SIZE_PT)?;
    in_range("style.letter_spacing", spacing, LETTER_SPACING)?;
    in_range("style.line_height", line_height, LINE_HEIGHT)?;
    let size_px = finite("size_px", size_pt * dpi / 72.0)? as f32;
    if size_px.is_finite() {
        Ok(size_px)
    } else {
        Err(TextError::NonFinite { field: "size_px" })
    }
}
