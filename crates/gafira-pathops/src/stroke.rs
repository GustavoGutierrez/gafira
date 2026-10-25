//! The stroke outlining.
//!
//! A stroke covers an area around a path. The crate returns that area as a
//! path, so a caller can fill it, intersect it or measure it.

use kurbo::{BezPath, Cap, Join, Stroke, StrokeOpts};

use crate::{FillRule, NonFiniteValue, Path, PathOpError};

/// The visual style of a stroke.
///
/// The style carries the width of the stroke and the way its ends and corners
/// are shaped. Dashes are out of scope for this crate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeStyle {
    /// The width of the stroke, in the unit of the path.
    ///
    /// It must be a finite positive number.
    pub width: f64,
    /// How the ends of an open subpath are shaped.
    pub cap: Cap,
    /// How the corners between two segments are shaped.
    pub join: Join,
}

impl Path {
    /// The outline of the area this path covers when stroked.
    ///
    /// The outline is always closed: a stroke encloses an area, even when the
    /// stroked path is open. The tolerance bounds the deviation between the
    /// outline and the exact stroke.
    pub fn stroke_outline(&self, style: &StrokeStyle, tolerance: f64) -> Result<Path, PathOpError> {
        validate_tolerance(tolerance)?;
        validate_width(style.width)?;
        self.validate_finite()?;

        let kurbo_style = Stroke::new(style.width)
            .with_caps(style.cap)
            .with_join(style.join);

        let outline = kurbo::stroke(
            self.as_bez_path().elements().iter().copied(),
            &kurbo_style,
            &StrokeOpts::default(),
            tolerance,
        );

        Ok(Path::new(close_all(outline), FillRule::NonZero))
    }
}

/// Closes every subpath of the outline.
fn close_all(bez: BezPath) -> BezPath {
    let mut closed = BezPath::new();
    let mut open = false;
    for el in bez.elements() {
        match el {
            kurbo::PathEl::MoveTo(p) => {
                if open {
                    closed.close_path();
                }
                open = false;
                closed.move_to(*p);
            }
            kurbo::PathEl::ClosePath => {
                closed.close_path();
                open = false;
            }
            other => {
                open = true;
                closed.push(*other);
            }
        }
    }
    if open {
        closed.close_path();
    }
    closed
}

/// Checks that the tolerance is a finite positive number.
fn validate_tolerance(tolerance: f64) -> Result<(), PathOpError> {
    if tolerance.is_finite() && tolerance > 0.0 {
        Ok(())
    } else if let Some(value) = NonFiniteValue::classify(tolerance) {
        Err(PathOpError::InvalidTolerance { value })
    } else {
        Err(PathOpError::InvalidTolerance {
            value: NonFiniteValue::Nan,
        })
    }
}

/// Checks that the stroke width is a finite positive number.
fn validate_width(width: f64) -> Result<(), PathOpError> {
    if width.is_finite() && width > 0.0 {
        Ok(())
    } else if let Some(value) = NonFiniteValue::classify(width) {
        Err(PathOpError::InvalidStrokeWidth { value })
    } else {
        Err(PathOpError::InvalidStrokeWidth {
            value: NonFiniteValue::Nan,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::{Rect, Shape};

    #[test]
    fn a_valid_width_passes_the_check() {
        assert_eq!(validate_width(4.0), Ok(()));
    }

    #[test]
    fn an_invalid_width_is_rejected() {
        assert!(matches!(
            validate_width(0.0),
            Err(PathOpError::InvalidStrokeWidth { .. })
        ));
        assert!(matches!(
            validate_width(-1.0),
            Err(PathOpError::InvalidStrokeWidth { .. })
        ));
    }

    #[test]
    fn the_outline_of_an_open_line_is_closed() {
        let mut bez = BezPath::new();
        bez.move_to((0.0, 0.0));
        bez.line_to((10.0, 0.0));
        let path = Path::new(bez, FillRule::NonZero);

        let outline = path
            .stroke_outline(
                &StrokeStyle {
                    width: 2.0,
                    cap: Cap::Butt,
                    join: Join::Miter,
                },
                1e-3,
            )
            .unwrap();

        assert!(outline.is_subpath_closed(0));
    }

    #[test]
    fn the_outline_of_a_rectangle_is_closed() {
        let path = Path::new(
            Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.0),
            FillRule::NonZero,
        );

        let outline = path
            .stroke_outline(
                &StrokeStyle {
                    width: 2.0,
                    cap: Cap::Round,
                    join: Join::Round,
                },
                1e-3,
            )
            .unwrap();

        // A stroked closed shape has an outer boundary and an inner one.
        assert_eq!(outline.subpath_count(), 2);
        assert!(outline.is_subpath_closed(0));
        assert!(outline.is_subpath_closed(1));
    }
}
