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
mod reliable;
mod slic;
mod table;
mod work;

use std::sync::Arc;

use gafira_doc::contract::{CancelToken, Describe, Descriptor, Progress};
use gafira_doc::msg::MessageKey;
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
    /// Estimated peak memory of one run at the longest working copy, in bytes.
    /// A first estimate; SG3 calibrates it against a measurement (Q-48).
    pub ram_bytes: u64,
}

/// Bytes per pixel of the working copy: colour, labels, SLIC distances, masks
/// and the node data, rounded up.
const BYTES_PER_WORK_PIXEL: u64 = 16;
/// Rows of one guided-filter band with its margins, and the `f32` buffers it keeps.
const FILTER_ROWS: u64 = 282;
const FILTER_BUFFERS: u64 = 8;

/// Estimated peak memory for a working copy whose longest side is `side`
/// (the square is the worst case); SG3 calibrates it (Q-48).
fn ram_estimate(side: u64) -> u64 {
    side * side * BYTES_PER_WORK_PIXEL + FILTER_BUFFERS * 4 * FILTER_ROWS * side
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
pub trait Segmenter: Describe + Send + Sync {
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

static DESCRIPTOR: Descriptor =
    Descriptor::new("segment.classic", MessageKey("segment.classic"), 1);

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

impl Describe for ClassicSegmenter {
    fn descriptor(&self) -> &Descriptor {
        &DESCRIPTOR
    }
}

impl Segmenter for ClassicSegmenter {
    fn requirements(&self) -> SegReq {
        SegReq {
            gpu: false,
            max_work_side: 2048,
            ram_bytes: ram_estimate(work::WORK_SIDE as u64),
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
        let (data, reliable) = if hints.zone == SegZone::All {
            // Nothing to separate: the whole image, after one look at the token.
            let meter = Meter::new(progress, cancel, 1 + (w * h) as u64);
            meter.step(1)?;
            let data = vec![255; w * h];
            let reliable = reliable::assess(&img, &data, &meter)?;
            meter.finish()?;
            (data, reliable)
        } else {
            let (ww, wh) = work::working_size(w, h);
            let planned = (w * h) as u64
                + slic::planned(ww, wh)
                + cut::planned(ww, wh)
                + refine::planned(w, h)
                + hints.stroke_cost(work_scale(w, h, (ww, wh)), (ww, wh))
                + hints.stroke_cost((1.0, 1.0), (w, h))
                + (w * h) as u64;
            let meter = Meter::new(progress, cancel, planned);
            let mut data = self.subject(&src, hints, (ww, wh), &meter)?;
            if hints.zone == SegZone::Background {
                for a in &mut data {
                    *a = 255 - *a;
                }
            }
            let reliable = reliable::assess(&img, &data, &meter)?;
            meter.finish()?;
            (data, reliable)
        };
        let mask = AlphaMask::new(width, height, data)?;
        Ok(Segmentation { mask, reliable })
    }
}

impl ClassicSegmenter {
    /// The foreground coverage of the whole image, seeds and «Elemento» applied.
    fn subject(
        &self,
        src: &Source<'_>,
        hints: &SegHints,
        (ww, wh): (usize, usize),
        meter: &Meter<'_>,
    ) -> Result<Vec<u8>, gafira_doc::contract::Cancelled> {
        let (w, h) = (src.width, src.height);
        let work = WorkImage::build(src, meter)?;
        let sp = slic::slic(&work, &self.pool, meter)?;
        let nodes = cut::nodes(&work, &sp, meter)?;
        let edges = slic::adjacency(&sp, (ww, wh), meter)?;
        // Seeds reach the graph through the superpixels most of their pixels fall in.
        let scale = work_scale(w, h, (ww, wh));
        let mut hits = vec![[0u32; 2]; nodes.len()];
        for (side, strokes) in [&hints.foreground, &hints.background]
            .into_iter()
            .enumerate()
        {
            for stroke in strokes {
                stroke.for_covered(
                    scale,
                    (ww, wh),
                    |units| meter.step(units),
                    |x, y| {
                        hits[sp.labels[y * ww + x] as usize][side] += 1;
                    },
                )?;
            }
        }
        let pins: Vec<cut::Pin> = hits
            .iter()
            .map(|&[fg, bg]| match fg.cmp(&bg) {
                std::cmp::Ordering::Greater => cut::Pin::Foreground,
                std::cmp::Ordering::Less => cut::Pin::Background,
                std::cmp::Ordering::Equal => cut::Pin::Free,
            })
            .collect();
        let mut foreground = cut::grabcut(&nodes, &edges, &pins, meter, cut::round_units(ww, wh))?;
        if hints.zone == SegZone::Element {
            foreground = cut::central(&nodes, &edges, &foreground, &pins, (ww, wh));
        }
        let coarse: Vec<u8> = sp
            .labels
            .iter()
            .map(|&l| if foreground[l as usize] { 255 } else { 0 })
            .collect();
        drop((sp, work));
        let mut data = refine::upscale(src, &coarse, (ww, wh), &self.pool, meter)?;
        // Under a stroke the person's word is final; foreground wins an overlap.
        for (value, strokes) in [(0, &hints.background), (255, &hints.foreground)] {
            for stroke in strokes {
                stroke.for_covered(
                    (1.0, 1.0),
                    (w, h),
                    |units| meter.step(units),
                    |x, y| {
                        data[y * w + x] = value;
                    },
                )?;
            }
        }
        Ok(data)
    }
}

/// The per-axis scale from the source to the working copy (the block mapping of
/// `work`).
fn work_scale(w: usize, h: usize, (ww, wh): (usize, usize)) -> (f64, f64) {
    (ww as f64 / w as f64, wh as f64 / h as f64)
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
