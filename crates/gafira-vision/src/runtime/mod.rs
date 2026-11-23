//! How a model is run (S84-04): another way is another [`Runtime`].

use gafira_ports::vision::VisionError;

#[cfg(feature = "onnx")]
pub mod onnx;
pub mod tract;

/// What goes into a model: numbers, or bytes given as numbers from 0 to 255.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Input {
    pub shape: [usize; 4],
    pub bytes: bool,
}

impl Input {
    #[must_use]
    pub const fn numbers(shape: [usize; 4]) -> Self {
        Self {
            shape,
            bytes: false,
        }
    }

    #[must_use]
    pub const fn bytes(shape: [usize; 4]) -> Self {
        Self { shape, bytes: true }
    }

    /// How many values there are.
    #[must_use]
    pub fn count(self) -> usize {
        self.shape.iter().product()
    }
}

/// `values` as bytes.
pub(crate) fn as_bytes(values: Vec<f32>) -> Vec<u8> {
    // Clamped first: the cast drops nothing.
    let byte = |v: f32| v.round().clamp(0.0, 255.0) as u8;
    values.into_iter().map(byte).collect()
}

/// Runs models.
pub trait Runtime: Send + Sync {
    /// Its name, for the person and for what is measured.
    fn id(&self) -> &'static str;

    /// Makes the model of `bytes` ready for `inputs`.
    ///
    /// # Errors
    /// [`VisionError`] when it does not run it.
    fn load(&self, bytes: &[u8], inputs: &[Input]) -> Result<Box<dyn Session>, VisionError>;
}

/// A model ready to run.
pub trait Session: Send + Sync {
    /// The first output for `inputs`, each of the shape it was loaded for.
    ///
    /// # Errors
    /// [`VisionError::Model`].
    fn run(&self, inputs: Vec<Vec<f32>>) -> Result<Vec<f32>, VisionError>;

    /// How many pieces it is worth running at once.
    fn at_once(&self) -> usize {
        1
    }
}
