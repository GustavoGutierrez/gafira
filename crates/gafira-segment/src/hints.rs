//! What the person tells the segmenter: a zone and seed strokes (ARQ-130).

use gafira_raster::BufferError;

/// Which part of the image the mask selects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SegZone {
    /// The whole image: no segmentation is needed.
    #[default]
    All,
    /// The detected subject.
    Foreground,
    /// The complement of [`SegZone::Foreground`]: `255 - alpha`.
    Background,
    /// The detected object nearest the centre of the image (D-23).
    Element,
}

/// A brush stroke over the image, in pixels of the input image.
#[derive(Clone, Debug, PartialEq)]
pub struct SeedStroke {
    /// Points of the stroke as `[x, y]`; consecutive points are joined.
    pub points: Vec<[f32; 2]>,
    /// Brush radius in pixels. A negative radius covers nothing.
    pub radius: f32,
}

/// Half the diagonal of a pixel: a point inside a pixel reaches its centre.
const HALF_DIAGONAL: f32 = 0.708;

impl SeedStroke {
    /// Calls `f(x, y)` for every pixel of a `w` x `h` image the stroke covers,
    /// with the stroke scaled by `scale` first. A pixel is covered when its
    /// centre lies within the radius of the polyline (a single point is a disc).
    /// A negative radius covers nothing.
    pub(crate) fn for_covered(
        &self,
        scale: f32,
        (w, h): (usize, usize),
        mut f: impl FnMut(usize, usize),
    ) {
        if self.radius < 0.0 {
            return;
        }
        let reach = self.radius * scale + HALF_DIAGONAL;
        let n = self.points.len();
        for i in 0..n {
            let a = self.points[i].map(|v| v * scale);
            let b = self.points.get(i + 1).map_or(a, |p| p.map(|v| v * scale));
            let lo = |p: f32, q: f32, limit: usize| {
                ((p.min(q) - reach).floor() as i64).clamp(0, limit as i64 - 1) as usize
            };
            let hi = |p: f32, q: f32, limit: usize| {
                ((p.max(q) + reach).ceil() as i64).clamp(0, limit as i64 - 1) as usize
            };
            let (x0, x1) = (lo(a[0], b[0], w), hi(a[0], b[0], w));
            let (y0, y1) = (lo(a[1], b[1], h), hi(a[1], b[1], h));
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let len2 = dx * dx + dy * dy;
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let (px, py) = (x as f32 + 0.5 - a[0], y as f32 + 0.5 - a[1]);
                    let t = if len2 > 0.0 {
                        ((px * dx + py * dy) / len2).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let (ex, ey) = (px - t * dx, py - t * dy);
                    if ex * ex + ey * ey <= reach * reach {
                        f(x, y);
                    }
                }
            }
        }
    }
}

/// Zone, seed strokes and the central priority of «Elemento» (ARQ-130, D-23).
///
/// Plain data: it does not validate (ARQ-110); `Segmenter::segment` does.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SegHints {
    /// The part of the image the mask selects. [`SegZone::Element`] carries
    /// the central priority hint.
    pub zone: SegZone,
    /// Strokes over the subject.
    pub foreground: Vec<SeedStroke>,
    /// Strokes over the background.
    pub background: Vec<SeedStroke>,
}

impl SegHints {
    /// Every number is finite, checked before any work (ARQ-110).
    pub(crate) fn check_finite(&self) -> Result<(), BufferError> {
        let lists = [
            (
                &self.foreground,
                "hints.foreground.points",
                "hints.foreground.radius",
            ),
            (
                &self.background,
                "hints.background.points",
                "hints.background.radius",
            ),
        ];
        for (strokes, points_field, radius_field) in lists {
            for (index, stroke) in strokes.iter().enumerate() {
                if stroke.points.iter().flatten().any(|v| !v.is_finite()) {
                    return Err(BufferError::NonFinite {
                        field: points_field,
                        index,
                    });
                }
                if !stroke.radius.is_finite() {
                    return Err(BufferError::NonFinite {
                        field: radius_field,
                        index,
                    });
                }
            }
        }
        Ok(())
    }
}
