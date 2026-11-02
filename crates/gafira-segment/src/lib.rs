//! Layer-2 engine crate `gafira-segment`: the `Segmenter` contract and the
//! `ClassicSegmenter` (02 §4.7, ARQ-130). Pure domain code: no I/O, no clocks.

#![forbid(unsafe_code)]

mod error;
mod hints;

use gafira_doc::contract::{CancelToken, Progress};
use gafira_raster::{AlphaMask, BufferError, ImageView, PixelData};

pub use error::SegError;
pub use hints::{SeedStroke, SegHints, SegZone};

/// What a segmenter needs to run (02 §4.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SegReq {
    /// True when the segmenter needs a GPU.
    pub gpu: bool,
    /// Longest side of the working copy, in pixels (D-45).
    pub max_work_side: u32,
}

/// A mask and whether the person can trust it (ARQ-130, ARQ-131).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segmentation {
    /// Coverage of the selected zone, with the input's width and height.
    pub mask: AlphaMask,
    /// True when the result is plausible (ARQ-131).
    pub reliable: bool,
}

/// Isolates a subject from an image (02 §4.7).
pub trait Segmenter: Send + Sync {
    /// CPU or GPU, and the longest side of the working copy.
    fn requirements(&self) -> SegReq;

    /// Segments `img`.
    ///
    /// # Errors
    /// [`SegError`] for an empty image, a format without colour, a non-finite
    /// number, or a cancelled token. Nothing is computed before the checks.
    fn segment(
        &self,
        img: ImageView<'_>,
        hints: &SegHints,
        progress: &dyn Progress,
        cancel: &CancelToken,
    ) -> Result<Segmentation, SegError>;
}

/// The colour-only segmenter of the MVP (D-45).
#[derive(Clone, Copy, Debug, Default)]
pub struct ClassicSegmenter;

impl Segmenter for ClassicSegmenter {
    fn requirements(&self) -> SegReq {
        SegReq {
            gpu: false,
            max_work_side: 2048,
        }
    }

    fn segment(
        &self,
        img: ImageView<'_>,
        hints: &SegHints,
        _progress: &dyn Progress,
        _cancel: &CancelToken,
    ) -> Result<Segmentation, SegError> {
        let (width, height) = (img.width(), img.height());
        if width == 0 || height == 0 {
            return Err(SegError::EmptyImage { width, height });
        }
        if matches!(img.data(), PixelData::Alpha8(_)) {
            return Err(SegError::UnsupportedFormat {
                format: img.data().format(),
            });
        }
        hints.check_finite()?;
        check_pixels(&img)?;
        let len = usize::try_from(u64::from(width) * u64::from(height)).unwrap_or(0);
        let mask = AlphaMask::new(width, height, vec![0; len])?;
        Ok(Segmentation {
            mask,
            reliable: false,
        })
    }
}

/// Every `f32` pixel value is finite (ARQ-110); the index is the element's.
fn check_pixels(img: &ImageView<'_>) -> Result<(), BufferError> {
    if let PixelData::RgbaF32(px) = img.data()
        && let Some(index) = px.iter().position(|v| !v.is_finite())
    {
        return Err(BufferError::NonFinite {
            field: "pixels",
            index,
        });
    }
    Ok(())
}
