//! The path simplification.
//!
//! Simplifying a path removes the parts of it that carry no shape, within a
//! tolerance that bounds how far the result may move from the original.

use kurbo::simplify::{SimplifyOptions, simplify_bezpath};
use kurbo::{BezPath, PathEl, Point};

use crate::{Path, PathOpError, validate};

/// The smallest tolerance the simplification accepts, per unit of the largest
/// absolute coordinate of the path.
///
/// Below it the simplification can run without end, on curved paths and, a
/// little further down, on straight ones. The bound is empirical: no hang was
/// observed at or above it.
const MINIMUM_TOLERANCE_PER_UNIT: f64 = 1e-12;

impl Path {
    /// The path with its redundant parts removed.
    ///
    /// The simplified path stays within `tolerance` of the original. The
    /// guarantee is about shape, not about the segment count: straight segments
    /// may be replaced by fitted curves, and the result may hold more segments
    /// than the original. Every subpath of the original that holds a segment appears
    /// in the result, in the same order, with the same fill rule; a subpath made
    /// only of a move holds no shape and is dropped. A segment drawn after a
    /// close, with no new move, belongs to a new subpath that starts at the
    /// start point of the subpath just closed. A closed path of straight
    /// segments whose every vertex lies farther than `tolerance` from the line
    /// through its neighbours comes back equal in value. A subpath whose every
    /// segment has zero length is kept as it is.
    ///
    /// # Errors
    ///
    /// A path with a coordinate whose absolute value is above
    /// [`crate::COORDINATE_LIMIT`] (`1e9`) is rejected with
    /// [`PathOpError::CoordinateOutOfRange`]. A tolerance below `1e-12 * m`,
    /// where `m` is the largest absolute coordinate of the path, is rejected
    /// with [`PathOpError::ToleranceTooFine`]; a tolerance equal to the minimum
    /// is accepted, and an empty path has no minimum.
    pub fn simplify(&self, tolerance: f64) -> Result<Path, PathOpError> {
        validate::tolerance(tolerance)?;
        self.validate_finite()?;
        self.validate_in_range()?;
        validate::not_too_fine(
            tolerance,
            MINIMUM_TOLERANCE_PER_UNIT,
            self.max_abs_coordinate(),
        )?;

        let options = SimplifyOptions::default();
        let mut simplified: Vec<PathEl> = Vec::new();
        for subpath in subpaths(self.as_bez_path().elements()) {
            if is_zero_length(&subpath) {
                // The simplifier drops a subpath that covers no distance; it is
                // still a subpath of the input, so it is kept as it is.
                simplified.extend_from_slice(&subpath);
            } else {
                simplified.extend(simplify_bezpath(subpath, tolerance, &options));
            }
        }

        Ok(Path::new(BezPath::from_vec(simplified), self.fill_rule()))
    }
}

/// The subpaths of the elements, each starting with a move.
///
/// A segment drawn after a close, with no new move, starts a new subpath at the
/// start point of the subpath just closed (the SVG convention): the move is
/// written out. A subpath made only of a move holds no shape and is dropped.
fn subpaths(elements: &[PathEl]) -> Vec<Vec<PathEl>> {
    let mut subpaths: Vec<Vec<PathEl>> = Vec::new();
    let mut start = Point::ORIGIN;
    let mut closed = false;
    for el in elements {
        match *el {
            PathEl::MoveTo(p) => {
                start = p;
                closed = false;
                subpaths.push(vec![*el]);
            }
            PathEl::ClosePath => {
                closed = true;
                if let Some(current) = subpaths.last_mut() {
                    current.push(*el);
                }
            }
            _ => {
                if closed || subpaths.is_empty() {
                    closed = false;
                    subpaths.push(vec![PathEl::MoveTo(start)]);
                }
                if let Some(current) = subpaths.last_mut() {
                    current.push(*el);
                }
            }
        }
    }
    subpaths.retain(|subpath| subpath.iter().any(|el| !matches!(el, PathEl::MoveTo(_))));
    subpaths
}

/// Whether the subpath starts with a move, holds a segment, and every point of
/// it is the start point: all its segments have zero length.
fn is_zero_length(subpath: &[PathEl]) -> bool {
    let Some(PathEl::MoveTo(start)) = subpath.first() else {
        return false;
    };
    let mut has_segment = false;
    for el in subpath {
        let points: &[Point] = match el {
            PathEl::MoveTo(p) => std::slice::from_ref(p),
            PathEl::LineTo(p) => {
                has_segment = true;
                std::slice::from_ref(p)
            }
            PathEl::QuadTo(a, b) => {
                has_segment = true;
                &[*a, *b]
            }
            PathEl::CurveTo(a, b, c) => {
                has_segment = true;
                &[*a, *b, *c]
            }
            PathEl::ClosePath => &[],
        };
        if points.iter().any(|p| p != start) {
            return false;
        }
    }
    has_segment
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FillRule;
    use kurbo::{BezPath, Rect, Shape};

    #[test]
    fn the_simplification_keeps_the_subpaths() {
        let mut bez = Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.0);
        let mut second = BezPath::new();
        second.move_to((20.0, 20.0));
        second.line_to((30.0, 20.0));
        second.line_to((30.0, 30.0));
        second.close_path();
        bez.extend(second.elements().iter().copied());
        let path = Path::new(bez, FillRule::NonZero);

        let simplified = path.simplify(1e-3).unwrap();

        assert_eq!(simplified.subpath_count(), 2);
    }
}
