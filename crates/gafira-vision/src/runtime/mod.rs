//! How a model is run (S84-04): another way is another [`Runtime`].

use gafira_ports::vision::VisionError;

#[cfg(feature = "onnx")]
pub mod onnx;
pub mod tract;

/// Runs models.
pub trait Runtime: Send + Sync {
    /// Its name, for the person and for what is measured.
    fn id(&self) -> &'static str;

    /// Makes the model of `bytes` ready for inputs of `shape`.
    ///
    /// # Errors
    /// [`VisionError`] when it does not run it.
    fn load(&self, bytes: &[u8], shape: [usize; 4]) -> Result<Box<dyn Session>, VisionError>;
}

/// A model ready to run.
pub trait Session: Send + Sync {
    /// The first output for `input`, of the shape it was loaded for.
    ///
    /// # Errors
    /// [`VisionError::Model`].
    fn run(&self, input: Vec<f32>) -> Result<Vec<f32>, VisionError>;

    /// How many pieces it is worth running at once.
    fn at_once(&self) -> usize {
        1
    }
}
