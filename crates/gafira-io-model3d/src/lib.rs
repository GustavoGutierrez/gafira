//! Adapter crate `gafira-io-model3d` (spec 100 §3.1, §6.1, §9): reads `.glb` (glTF 2.0) and
//! `.obj` files as untrusted data into a validated, normalized [`gafira_model3d::Scene`].
//! It reads only the bytes it is given: no paths, URLs or external buffers are followed.

#![forbid(unsafe_code)]

mod decoder;
mod glb;
mod obj;

pub use decoder::{
    ModelDecoder, camera_of, document_format, format_of, lighting_of, params_of, pose_of,
    render_parallel,
};

use std::fmt;

use gafira_model3d::{Limits, Scene, SceneError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFormat {
    Glb,
    Obj,
}

impl ModelFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Glb => "glb",
            Self::Obj => "obj",
        }
    }
}

/// Limits of one import (spec 100 §9); the defaults suit a desktop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportLimits {
    pub max_file_bytes: usize,
    /// Depth of the node tree of a GLB.
    pub max_node_depth: usize,
    pub scene: Limits,
}

impl Default for ImportLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: 256 * 1024 * 1024,
            max_node_depth: 64,
            scene: Limits::default(),
        }
    }
}

/// Something the import left out, shown to the user without blocking it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportWarning {
    /// The OBJ names a material library that was not provided: neutral material.
    MissingMaterials,
    /// An optional glTF extension the renderer does not use.
    IgnoredExtension(String),
    /// A texture that could not be decoded: its material keeps its colour.
    TextureSkipped,
    /// Points, lines or other non-triangle parts that are not drawn.
    NonTriangleParts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    TooLarge {
        bytes: usize,
        limit: usize,
    },
    /// Neither a GLB nor an OBJ.
    UnknownFormat,
    /// The file is damaged: the reason in English, for logs.
    Damaged(String),
    /// A required glTF extension (Draco, Meshopt, KTX2...) without a decoder.
    UnsupportedExtension(String),
    /// A buffer or image that points outside the file.
    ExternalReference,
    Scene(SceneError),
    Cancelled,
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { bytes, limit } => {
                write!(f, "the file is too large ({bytes} bytes, limit {limit})")
            }
            Self::UnknownFormat => write!(f, "not a GLB or OBJ model"),
            Self::Damaged(why) => write!(f, "the model is damaged: {why}"),
            Self::UnsupportedExtension(e) => {
                write!(f, "the model needs an unsupported extension: {e}")
            }
            Self::ExternalReference => write!(f, "the model refers to files outside it"),
            Self::Scene(e) => write!(f, "{e}"),
            Self::Cancelled => write!(f, "the import was cancelled"),
        }
    }
}

impl std::error::Error for ImportError {}

impl From<SceneError> for ImportError {
    fn from(e: SceneError) -> Self {
        Self::Scene(e)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Imported {
    pub scene: Scene,
    pub warnings: Vec<ImportWarning>,
}

/// The format of `bytes`, from its content; `name` only breaks ties for text files.
pub fn detect_format(bytes: &[u8], name: &str) -> Option<ModelFormat> {
    if bytes.len() >= 12
        && &bytes[0..4] == b"glTF"
        && u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) == 2
    {
        return Some(ModelFormat::Glb);
    }
    obj::looks_like_obj(bytes, name).then_some(ModelFormat::Obj)
}

/// Reads a model. `cancelled` is polled between parts; it returning true stops the import.
pub fn import(
    bytes: &[u8],
    format: ModelFormat,
    limits: &ImportLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<Imported, ImportError> {
    if bytes.len() > limits.max_file_bytes {
        return Err(ImportError::TooLarge {
            bytes: bytes.len(),
            limit: limits.max_file_bytes,
        });
    }
    if cancelled() {
        return Err(ImportError::Cancelled);
    }
    // The readers check ranges first; a parser panic on a hostile file still must not take
    // the app down (spec 100 AC-15).
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match format {
        ModelFormat::Glb => glb::read(bytes, limits, cancelled),
        ModelFormat::Obj => obj::read(bytes, limits, cancelled),
    }))
    .unwrap_or_else(|_| {
        Err(ImportError::Damaged(
            "the reader stopped on invalid data".into(),
        ))
    })?;
    let (scene, warnings) = result;
    if cancelled() {
        return Err(ImportError::Cancelled);
    }
    let scene = scene.normalized(&limits.scene)?;
    Ok(Imported { scene, warnings })
}

/// Fails early when adding `add` to `count` passes `max`.
pub(crate) fn budget(
    count: &mut usize,
    add: usize,
    max: usize,
    err: fn(usize) -> SceneError,
) -> Result<(), ImportError> {
    *count = count.saturating_add(add);
    if *count > max {
        Err(ImportError::Scene(err(*count)))
    } else {
        Ok(())
    }
}
