//! Outlines of the glyphs of a layout (FD-43).

use gafira_pathops::{BezPath, FillRule, Path, PathEl};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{FontRef, GlyphId, MetadataProvider};

use crate::{TextError, TextLayout};

/// The outline of one cluster, in the coordinates of the layout.
#[derive(Clone, Debug, PartialEq)]
pub struct ClusterOutline {
    /// Index of the line.
    pub line: usize,
    /// Index of the cluster in the line.
    pub cluster: usize,
    /// The contours of all its glyphs, in px from the top left of the box, y down, filled with
    /// the non-zero rule. Empty for a cluster with no contour, such as a space.
    pub path: Path,
}

/// Draws a glyph into a path, flipped to y down and moved to its pen position.
struct Pen {
    path: BezPath,
    origin: (f64, f64),
}

impl Pen {
    fn at(&self, x: f32, y: f32) -> (f64, f64) {
        (self.origin.0 + f64::from(x), self.origin.1 - f64::from(y))
    }
}

impl OutlinePen for Pen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path.move_to(self.at(x, y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.path.line_to(self.at(x, y));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.path.quad_to(self.at(cx0, cy0), self.at(x, y));
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.path
            .curve_to(self.at(cx0, cy0), self.at(cx1, cy1), self.at(x, y));
    }

    fn close(&mut self) {
        self.path.close_path();
    }
}

/// The outlines of the clusters of a layout: one for each cluster, in reading order of the lines
/// and visual order of the clusters.
///
/// Each glyph is drawn from the font that shaped its cluster (`Cluster::font`, a position of
/// `fonts`; the first face of a collection), at the size of the cluster and at the place the
/// layout gives it: the left edge of the cluster plus the offset of the glyph, and the baseline
/// of the line. The layout is only read, so an effect built on these paths (a stroke from
/// `gafira-pathops`, say) cannot change it (FD-43). A glyph the font has no outline for gives
/// nothing. A font that cannot be read, or a position outside `fonts`, gives
/// `Err(InvalidFont)`; a text with no glyph reads no font.
pub fn outline_text(
    layout: &TextLayout,
    fonts: &[&[u8]],
) -> Result<Vec<ClusterOutline>, TextError> {
    let mut opened: Vec<Option<FontRef<'_>>> = vec![None; fonts.len()];
    let mut out = Vec::new();
    for (line_index, line) in layout.lines.iter().enumerate() {
        for (cluster_index, cluster) in line.clusters.iter().enumerate() {
            let mut path = BezPath::new();
            if !cluster.glyphs.is_empty() {
                let index = cluster.font;
                let invalid = TextError::InvalidFont { index };
                let bytes = fonts.get(index).ok_or(invalid)?;
                if opened[index].is_none() {
                    opened[index] = Some(FontRef::new(bytes).map_err(|_| invalid)?);
                }
                let font = opened[index].as_ref().ok_or(invalid)?;
                let glyphs = font.outline_glyphs();
                for (id, (dx, dy)) in cluster.glyphs.iter().zip(&cluster.glyph_offsets) {
                    let Some(glyph) = glyphs.get(GlyphId::new(*id)) else {
                        continue;
                    };
                    let mut pen = Pen {
                        path: BezPath::new(),
                        origin: (
                            f64::from(cluster.x) + f64::from(*dx),
                            f64::from(line.baseline) + f64::from(*dy),
                        ),
                    };
                    let settings =
                        DrawSettings::unhinted(Size::new(cluster.size_px), LocationRef::default());
                    glyph.draw(settings, &mut pen).map_err(|_| invalid)?;
                    path.extend(pen.path);
                }
            }
            if !path.elements().iter().all(finite) {
                return Err(TextError::NonFinite { field: "output" });
            }
            out.push(ClusterOutline {
                line: line_index,
                cluster: cluster_index,
                path: Path::new(path, FillRule::NonZero),
            });
        }
    }
    Ok(out)
}

fn finite(el: &PathEl) -> bool {
    match *el {
        PathEl::MoveTo(p) | PathEl::LineTo(p) => p.is_finite(),
        PathEl::QuadTo(a, b) => a.is_finite() && b.is_finite(),
        PathEl::CurveTo(a, b, c) => a.is_finite() && b.is_finite() && c.is_finite(),
        PathEl::ClosePath => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // FD-43 (a): no embedded font has cubic outlines, so the pen is driven by hand.
    #[test]
    fn fd_43_a_a_cubic_curve_of_a_font_stays_a_cubic_curve_moved_and_flipped() {
        let mut pen = Pen {
            path: BezPath::new(),
            origin: (10.0, 100.0),
        };
        pen.move_to(0.0, 0.0);
        pen.curve_to(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
        pen.close();
        let els = pen.path.elements();
        let point = |x: f64, y: f64| gafira_pathops::Point::new(x, y);
        assert_eq!(
            els[1],
            PathEl::CurveTo(point(11.0, 98.0), point(13.0, 96.0), point(15.0, 94.0))
        );
    }

    // S13-05, ARQ-110: one NaN in any point of a curve makes the element non-finite, one
    // point at a time.
    #[test]
    fn arq_110_a_nan_in_any_point_of_a_curve_is_not_finite() {
        let ok = gafira_pathops::Point::new(1.0, 2.0);
        let nan = gafira_pathops::Point::new(f64::NAN, 2.0);
        assert!(finite(&PathEl::QuadTo(ok, ok)));
        assert!(!finite(&PathEl::QuadTo(nan, ok)));
        assert!(!finite(&PathEl::QuadTo(ok, nan)));
        assert!(finite(&PathEl::CurveTo(ok, ok, ok)));
        assert!(!finite(&PathEl::CurveTo(nan, ok, ok)));
        assert!(!finite(&PathEl::CurveTo(ok, nan, ok)));
        assert!(!finite(&PathEl::CurveTo(ok, ok, nan)));
    }
}
