//! What the person tells the segmenter: a zone and seed strokes (ARQ-130).

use gafira_doc::contract::Cancelled;
use gafira_raster::BufferError;

use crate::meter::CHUNK_ROWS;

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
const HALF_DIAGONAL: f64 = 0.708;

/// One piece of a stroke's work: a segment of the polyline and a run of at most
/// [`CHUNK_ROWS`] rows of its bounding box.
struct Band {
    a: [f64; 2],
    b: [f64; 2],
    reach: f64,
    x: (usize, usize),
    y: (usize, usize),
}

impl Band {
    fn area(&self) -> u64 {
        ((self.x.1 - self.x.0 + 1) * (self.y.1 - self.y.0 + 1)) as u64
    }
}

/// Pixel range `[lo, hi]` of `limit` that `p..q` widened by `reach` touches.
fn range(p: f64, q: f64, reach: f64, limit: usize) -> (usize, usize) {
    let lo = ((p.min(q) - reach).floor() as i64).clamp(0, limit as i64 - 1) as usize;
    let hi = ((p.max(q) + reach).ceil() as i64).clamp(0, limit as i64 - 1) as usize;
    (lo, hi)
}

impl SeedStroke {
    /// The bands of the stroke on a `w` x `h` image, the stroke scaled by
    /// `scale` (per axis). A negative radius has none.
    fn bands(&self, scale: (f64, f64), (w, h): (usize, usize)) -> impl Iterator<Item = Band> + '_ {
        // The brush shrinks with the smaller scale: never more than was drawn.
        let reach = f64::from(self.radius) * scale.0.min(scale.1) + HALF_DIAGONAL;
        let scaled = move |p: [f32; 2]| [f64::from(p[0]) * scale.0, f64::from(p[1]) * scale.1];
        let n = if self.radius < 0.0 {
            0
        } else {
            self.points.len()
        };
        (0..n).flat_map(move |i| {
            let a = scaled(self.points[i]);
            let b = self.points.get(i + 1).map_or(a, |&p| scaled(p));
            let x = range(a[0], b[0], reach, w);
            let (y0, y1) = range(a[1], b[1], reach, h);
            (y0..=y1).step_by(CHUNK_ROWS).map(move |top| Band {
                a,
                b,
                reach,
                x,
                y: (top, (top + CHUNK_ROWS - 1).min(y1)),
            })
        })
    }

    /// Work units of the stroke: the pixels its bands look at.
    pub(crate) fn cost(&self, scale: (f64, f64), size: (usize, usize)) -> u64 {
        self.bands(scale, size).map(|b| b.area()).sum()
    }

    /// Calls `f(x, y)` for every pixel of a `w` x `h` image the stroke covers,
    /// with the stroke scaled by `scale` (per axis) first. A pixel is covered
    /// when its centre lies within the radius of the polyline (a single point is
    /// a disc). A negative radius covers nothing. `before(units)` is called
    /// ahead of each band of at most [`CHUNK_ROWS`] rows, and stops the work
    /// with its error. The geometry is `f64`, so every intermediate of a finite
    /// `f32` stroke is finite (ARQ-110); beyond about 1e16 pixels the position
    /// is only as exact as an `f64`.
    ///
    /// # Errors
    /// What `before` returns.
    pub(crate) fn for_covered(
        &self,
        scale: (f64, f64),
        size: (usize, usize),
        mut before: impl FnMut(u64) -> Result<(), Cancelled>,
        mut f: impl FnMut(usize, usize),
    ) -> Result<(), Cancelled> {
        for band in self.bands(scale, size) {
            before(band.area())?;
            let Band { a, b, reach, x, y } = band;
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let len2 = dx * dx + dy * dy;
            for py in y.0..=y.1 {
                for px in x.0..=x.1 {
                    let (qx, qy) = (px as f64 + 0.5 - a[0], py as f64 + 0.5 - a[1]);
                    let t = if len2 > 0.0 {
                        ((qx * dx + qy * dy) / len2).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let (ex, ey) = (qx - t * dx, qy - t * dy);
                    if ex * ex + ey * ey <= reach * reach {
                        f(px, py);
                    }
                }
            }
        }
        Ok(())
    }
}

/// Zone, seed strokes and the central priority of «Elemento» (ARQ-130, D-23).
///
/// Plain data: it does not validate (ARQ-110); `Segmenter::segment` does.
/// The default is the zone [`SegZone::All`] with no strokes, which does not
/// segment: it gives a solid mask (ARQ-130). Ask for a subject with
/// [`SegZone::Foreground`] or [`SegZone::Element`].
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
    /// Work units of all strokes at `scale` on a `size` image (progress plan).
    pub(crate) fn stroke_cost(&self, scale: (f64, f64), size: (usize, usize)) -> u64 {
        self.foreground
            .iter()
            .chain(&self.background)
            .map(|s| s.cost(scale, size))
            .sum()
    }

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
