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
