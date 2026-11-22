//! Models that work on images (spec 084, D-115): taking the background away
//! and giving an image a better resolution. Each model and each way to run
//! it is an adapter's; the engine and the interface know these contracts
//! alone, so a model can be changed for another.

use std::fmt;
use std::sync::Arc;

use gafira_doc::contract::{CancelToken, Progress};
use gafira_raster::AlphaMask;

/// An image of straight sRGB bytes with its opacity, row by row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pixels {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl Pixels {
    /// The image of `rgba`; none when they are not `width` × `height` pixels
    /// of four bytes, one at least.
    #[must_use]
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Option<Self> {
        let count = (width as usize).checked_mul(height as usize)?;
        (count > 0 && count.checked_mul(4)? == rgba.len()).then_some(Self {
            width,
            height,
            rgba,
        })
    }

    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    #[must_use]
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    #[must_use]
    pub fn into_rgba(self) -> Vec<u8> {
        self.rgba
    }
}

/// What a model does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VisionTask {
    /// Taking the background away.
    Cutout,
    /// A better resolution.
    Upscale,
}

/// What went wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VisionError {
    Cancelled,
    /// No connection to download from.
    Offline,
    /// What was downloaded is not what it should be.
    Corrupt,
    /// The model or the runtime is not installed, or does not run here.
    Unavailable(String),
    /// The image is larger than what can be worked on.
    TooLarge,
    /// The file system, or the disk is full.
    Io(String),
    /// The model failed.
    Model(String),
}

impl fmt::Display for VisionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => f.write_str("cancelled"),
            Self::Offline => f.write_str("no connection"),
            Self::Corrupt => f.write_str("the download is not what it should be"),
            Self::Unavailable(what) => write!(f, "not available: {what}"),
            Self::TooLarge => f.write_str("the image is too large"),
            Self::Io(what) => write!(f, "file: {what}"),
            Self::Model(what) => write!(f, "model: {what}"),
        }
    }
}

impl std::error::Error for VisionError {}

/// Takes the background of an image away.
pub trait BackgroundRemover: Send + Sync {
    /// The opacity of each pixel of `image`: 255 on its subject.
    ///
    /// # Errors
    /// [`VisionError`]; a cancelled one leaves nothing behind.
    fn mask(
        &self,
        image: &Pixels,
        progress: &dyn Progress,
        cancel: &CancelToken,
    ) -> Result<AlphaMask, VisionError>;
}

/// Gives an image a better resolution.
pub trait Upscaler: Send + Sync {
    /// The most it makes an image larger.
    fn most(&self) -> u32;

    /// `image`, `factor` times as wide and as tall.
    ///
    /// # Errors
    /// [`VisionError::TooLarge`] when the result would be; the others too.
    fn upscale(
        &self,
        image: &Pixels,
        factor: u32,
        progress: &dyn Progress,
        cancel: &CancelToken,
    ) -> Result<Pixels, VisionError>;
}

/// A graphics card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gpu {
    pub name: String,
    /// Its own memory: not the one of the processor.
    pub dedicated: bool,
}

/// What the models run on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Machine {
    pub threads: usize,
    /// 0 when it is not known.
    pub memory_bytes: u64,
    /// The best one found.
    pub gpu: Option<Gpu>,
}

/// What a model or a runtime is, and how it stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelInfo {
    pub id: &'static str,
    /// None for a runtime.
    pub task: Option<VisionTask>,
    pub name: &'static str,
    /// What it is for, a key of the catalogues.
    pub about: &'static str,
    pub license: &'static str,
    /// Where it comes from, for the person.
    pub source: &'static str,
    /// What is downloaded; 0 for one that comes with Gafira.
    pub download_bytes: u64,
    pub bundled: bool,
    pub installed: bool,
    /// The one used for its task.
    pub chosen: bool,
    /// The right one for this machine.
    pub recommended: bool,
    /// How long a piece of work took here, and on what, once measured.
    pub measured: Option<(u32, &'static str)>,
}

/// The models of this machine: which there are, and the ones in use.
pub trait VisionModels: Send + Sync {
    fn machine(&self) -> Machine;

    /// The models, by task, the lightest first.
    fn models(&self) -> Vec<ModelInfo>;

    /// The ways to run them that can be installed.
    fn runtimes(&self) -> Vec<ModelInfo>;

    /// Downloads a model or a runtime.
    ///
    /// # Errors
    /// [`VisionError`]; what was there before stays.
    fn install(
        &self,
        id: &str,
        progress: &dyn Progress,
        cancel: &CancelToken,
    ) -> Result<(), VisionError>;

    /// Deletes what was downloaded.
    ///
    /// # Errors
    /// [`VisionError::Io`].
    fn uninstall(&self, id: &str) -> Result<(), VisionError>;

    /// Uses the model `id` for its task from now on.
    ///
    /// # Errors
    /// [`VisionError::Unavailable`] when it is not installed.
    fn choose(&self, id: &str) -> Result<(), VisionError>;

    /// The one in use.
    ///
    /// # Errors
    /// [`VisionError`] when it cannot be run.
    fn remover(&self) -> Result<Arc<dyn BackgroundRemover>, VisionError>;

    /// The one in use.
    ///
    /// # Errors
    /// [`VisionError`] when it cannot be run.
    fn upscaler(&self) -> Result<Arc<dyn Upscaler>, VisionError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // S84-03: an image is its bytes, all of them.
    #[test]
    fn s84_03_pixels() {
        assert!(Pixels::new(2, 1, vec![0; 8]).is_some());
        assert!(Pixels::new(2, 1, vec![0; 7]).is_none());
        assert!(Pixels::new(0, 1, vec![]).is_none());
        assert!(Pixels::new(u32::MAX, u32::MAX, vec![0; 4]).is_none());
    }
}
