//! Layer-2 engine crate `gafira-segment`: the `Segmenter` contract and the
//! `ClassicSegmenter` (02 §4.7, ARQ-130). Pure domain code: no I/O, no clocks.

#![forbid(unsafe_code)]

mod cut;
mod error;
mod flow;
mod gmm;
mod hints;
mod meter;
mod refine;
mod slic;
mod table;
mod work;

use std::sync::Arc;

use gafira_doc::contract::{CancelToken, Progress};
use gafira_raster::{AlphaMask, BufferError, ImageView, PixelData};

use meter::Meter;
use work::{Source, WorkImage};

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

/// The colour-only segmenter of the MVP (D-45). It runs its parallel passes on
/// the pool it is given, so the trait does not depend on `rayon`.
#[derive(Clone)]
pub struct ClassicSegmenter {
    pool: Arc<rayon::ThreadPool>,
}

impl ClassicSegmenter {
    /// A segmenter that runs on `pool`.
    #[must_use]
    pub fn new(pool: Arc<rayon::ThreadPool>) -> Self {
        Self { pool }
    }
}

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
        progress: &dyn Progress,
        cancel: &CancelToken,
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
        let src = Source::new(&img);
        let (w, h) = (src.width, src.height);
        let (ww, wh) = work::working_size(w, h);
        let planned =
            (w * h) as u64 + slic::planned(ww, wh) + cut::planned(ww, wh) + refine::planned(w, h);
        let meter = Meter::new(progress, cancel, planned);
        let work = WorkImage::build(&src, &meter)?;
        let sp = slic::slic(&work, &self.pool, &meter)?;
        let nodes = cut::nodes(&work, &sp, &meter)?;
        let edges = slic::adjacency(&sp, (ww, wh), &meter)?;
        let foreground = cut::grabcut(&nodes, &edges, &meter, cut::round_units(ww, wh))?;
        let coarse: Vec<u8> = sp
            .labels
            .iter()
            .map(|&l| if foreground[l as usize] { 255 } else { 0 })
            .collect();
        drop((sp, work));
        let data = refine::upscale(&src, &coarse, (ww, wh), &self.pool, &meter)?;
        meter.finish()?;
        let mask = AlphaMask::new(width, height, data)?;
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
