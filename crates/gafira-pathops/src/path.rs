//! The path model: a Bézier path plus the rule that describes its interior.

use kurbo::{BezPath, PathEl, Point};

use crate::{FillRule, NonFiniteValue, PathOpError};

/// A path: a sequence of Bézier subpaths, plus the rule that decides which
/// points the path covers.
///
/// The path is a thin wrapper over [`BezPath`], the geometry vocabulary the
/// workspace already speaks (`gafira-geom` re-exports the same `kurbo` types).
/// The wrapper adds the fill rule and the validation the crate owns.
#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    bez: BezPath,
    fill_rule: FillRule,
}

/// A subpath of a path, as the crate sees it.
struct Subpath {
    /// The index of the subpath in the path.
    index: usize,
    /// Whether the subpath is closed.
    closed: bool,
}

impl Path {
    /// Builds a path from a Bézier path and the rule that describes its
    /// interior.
    ///
    /// The path is not validated here: validation happens in the operations
    /// that need a well-formed path, so a caller can build a path and inspect
    /// it before using it.
    #[must_use]
    pub fn new(bez: BezPath, fill_rule: FillRule) -> Self {
        Self { bez, fill_rule }
    }

    /// The Bézier path this path wraps.
    #[must_use]
    pub fn as_bez_path(&self) -> &BezPath {
        &self.bez
    }

    /// The rule that decides which points this path covers.
    #[must_use]
    pub fn fill_rule(&self) -> FillRule {
        self.fill_rule
    }

    /// Whether the path holds no drawing element.
    ///
    /// A path holding only a move or a close is empty: it covers no area.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bez.is_empty()
    }

    /// The number of subpaths in the path.
    #[must_use]
    pub fn subpath_count(&self) -> usize {
        self.subpaths().len()
    }

    /// Whether the subpath at `index` is closed.
    ///
    /// Returns `false` when the index is out of range.
    #[must_use]
    pub fn is_subpath_closed(&self, index: usize) -> bool {
        self.subpaths()
            .into_iter()
            .find(|s| s.index == index)
            .is_some_and(|s| s.closed)
    }

    /// Closes every open subpath of the path.
    ///
    /// An open subpath has no interior, so it has no defined meaning under a
    /// fill rule. Closing is an explicit act: the crate never closes a subpath
    /// behind the caller's back inside another operation.
    #[must_use]
    pub fn close_subpaths(&self) -> Path {
        let mut bez = BezPath::new();
        let mut start: Option<Point> = None;
        let mut closed = true;

        for el in self.bez.elements() {
            match el {
                PathEl::MoveTo(p) => {
                    if !closed && start.is_some() {
                        bez.close_path();
                    }
                    start = Some(*p);
                    closed = true;
                    bez.move_to(*p);
                }
                PathEl::LineTo(p) => {
                    closed = false;
                    bez.line_to(*p);
                }
                PathEl::QuadTo(p1, p2) => {
                    closed = false;
                    bez.quad_to(*p1, *p2);
                }
                PathEl::CurveTo(p1, p2, p3) => {
                    closed = false;
                    bez.curve_to(*p1, *p2, *p3);
                }
                PathEl::ClosePath => {
                    closed = true;
                    bez.close_path();
                }
            }
        }
        if !closed && start.is_some() {
            bez.close_path();
        }

        Path {
            bez,
            fill_rule: self.fill_rule,
        }
    }

    /// Checks that every coordinate of the path is finite.
    ///
    /// The check runs before any geometric computation, so a non-finite
    /// coordinate is reported as such instead of surfacing as a misleading
    /// lower-level error.
    pub fn validate_finite(&self) -> Result<(), PathOpError> {
        for el in self.bez.elements() {
            let check = |p: Point| -> Result<(), PathOpError> {
                if let Some(value) = NonFiniteValue::classify(p.x) {
                    return Err(PathOpError::NonFiniteCoordinate { value });
                }
                if let Some(value) = NonFiniteValue::classify(p.y) {
                    return Err(PathOpError::NonFiniteCoordinate { value });
                }
                Ok(())
            };
            match el {
                PathEl::MoveTo(p) | PathEl::LineTo(p) => check(*p)?,
                PathEl::QuadTo(p1, p2) => {
                    check(*p1)?;
                    check(*p2)?;
                }
                PathEl::CurveTo(p1, p2, p3) => {
                    check(*p1)?;
                    check(*p2)?;
                    check(*p3)?;
                }
                PathEl::ClosePath => {}
            }
        }
        Ok(())
    }

    /// Checks that every subpath of the path is closed.
    ///
    /// The error names the index of the first open subpath.
    pub fn validate_closed(&self) -> Result<(), PathOpError> {
        for subpath in self.subpaths() {
            if !subpath.closed {
                return Err(PathOpError::OpenSubpath {
                    subpath: subpath.index,
                });
            }
        }
        Ok(())
    }

    /// The subpaths of the path, in path order.
    fn subpaths(&self) -> Vec<Subpath> {
        let mut subpaths = Vec::new();
        let mut index = 0usize;
        let mut open: Option<usize> = None;

        for el in self.bez.elements() {
            match el {
                PathEl::MoveTo(_) => {
                    if let Some(start) = open.take() {
                        subpaths.push(Subpath {
                            index: start,
                            closed: false,
                        });
                    }
                    open = Some(index);
                    index += 1;
                }
                PathEl::ClosePath => {
                    if let Some(start) = open.take() {
                        subpaths.push(Subpath {
                            index: start,
                            closed: true,
                        });
                    }
                }
                PathEl::LineTo(_) | PathEl::QuadTo(..) | PathEl::CurveTo(..) => {}
            }
        }
        if let Some(start) = open {
            subpaths.push(Subpath {
                index: start,
                closed: false,
            });
        }
        subpaths
    }
}
