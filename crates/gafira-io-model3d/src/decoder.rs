//! The 3D side of the `ImageDecoder` port (spec 100 §8, D-130): reads a model once, keeps a
//! few read models, and draws them with the CPU rasterizer, bands in parallel.

use std::sync::{Arc, Mutex};

use gafira_doc::{ContentHash, Model3dFormat, Model3dLighting, Model3dNode, Model3dProjection};
use gafira_model3d::{Camera, CameraKind, Lighting, Pose, Prepared, RenderParams, Scene};
use gafira_ports::export::ImageDecoder;
use gafira_ports::media::MediaError;
use gafira_raster::{ImageBuf, PixelBuf, premultiply_u8};
use rayon::prelude::*;

use crate::{ImportError, ImportLimits, ModelFormat, import};

/// How many read models are kept for the next picture.
const KEPT: usize = 4;

/// The reader format of a document format.
#[must_use]
pub fn format_of(format: Model3dFormat) -> ModelFormat {
    match format {
        Model3dFormat::Glb => ModelFormat::Glb,
        Model3dFormat::Obj => ModelFormat::Obj,
    }
}

/// The document format of a reader format.
#[must_use]
pub fn document_format(format: ModelFormat) -> Model3dFormat {
    match format {
        ModelFormat::Glb => Model3dFormat::Glb,
        ModelFormat::Obj => Model3dFormat::Obj,
    }
}

/// The renderer's lighting preset of a document one.
#[must_use]
pub fn lighting_of(lighting: Model3dLighting) -> Lighting {
    match lighting {
        Model3dLighting::Soft => Lighting::Soft,
        Model3dLighting::Studio => Lighting::Studio,
        Model3dLighting::Dramatic => Lighting::Dramatic,
    }
}

/// The renderer's camera of a document one.
#[must_use]
pub fn camera_of(model: &Model3dNode) -> Camera {
    // The document keeps f64; the renderer works in f32.
    let c = model.camera;
    Camera {
        kind: match c.projection {
            Model3dProjection::Perspective => CameraKind::Perspective,
            Model3dProjection::Orthographic => CameraKind::Orthographic,
        },
        yaw_degrees: c.yaw as f32,
        elevation_degrees: c.elevation as f32,
        zoom: c.zoom as f32,
        fov_degrees: c.fov as f32,
    }
}

/// The renderer's pose of a document model.
#[must_use]
pub fn pose_of(model: &Model3dNode) -> Pose {
    Pose {
        rotation: model.rotation.map(|v| v as f32),
        position: model.position.map(|v| v as f32),
        scale: model.scale.map(|v| v as f32),
    }
}

/// What to draw for `model` at `size` pixels with `samples` per pixel side.
#[must_use]
pub fn params_of(model: &Model3dNode, (width, height): (u32, u32), samples: u32) -> RenderParams {
    RenderParams {
        pose: pose_of(model),
        camera: camera_of(model),
        lighting: lighting_of(model.lighting),
        width,
        height,
        samples,
    }
}

/// Draws `scene` with its bands in parallel: straight-alpha sRGB RGBA8.
#[must_use]
pub fn render_parallel(scene: &Scene, params: &RenderParams) -> Vec<u8> {
    let prepared = Prepared::new(scene, params);
    let bands: Vec<Vec<u8>> = (0..prepared.band_count())
        .into_par_iter()
        .map(|i| prepared.render_band(i))
        .collect();
    bands.concat()
}

/// An [`ImageDecoder`] that decodes images with `inner` and draws 3D models itself.
pub struct ModelDecoder<D> {
    inner: D,
    limits: ImportLimits,
    scenes: Mutex<Vec<(ContentHash, Arc<Scene>)>>,
}

impl<D> ModelDecoder<D> {
    pub fn new(inner: D) -> Self {
        Self {
            inner,
            limits: ImportLimits::default(),
            scenes: Mutex::new(Vec::new()),
        }
    }

    /// The read model of the file `bytes`, named by its content `hash`; read once while it
    /// is among the last few used.
    pub fn scene(
        &self,
        hash: ContentHash,
        bytes: &[u8],
        format: ModelFormat,
    ) -> Result<Arc<Scene>, ImportError> {
        if let Ok(mut kept) = self.scenes.lock()
            && let Some(i) = kept.iter().position(|(h, _)| *h == hash)
        {
            let entry = kept.remove(i);
            let scene = Arc::clone(&entry.1);
            kept.push(entry);
            return Ok(scene);
        }
        let scene = Arc::new(import(bytes, format, &self.limits, &|| false)?.scene);
        if let Ok(mut kept) = self.scenes.lock() {
            kept.push((hash, Arc::clone(&scene)));
            if kept.len() > KEPT {
                kept.remove(0);
            }
        }
        Ok(scene)
    }
}

impl<D: ImageDecoder> ImageDecoder for ModelDecoder<D> {
    fn decode(&self, bytes: &[u8]) -> Result<ImageBuf, MediaError> {
        self.inner.decode(bytes)
    }

    fn render_model(
        &self,
        model: &Model3dNode,
        bytes: &[u8],
        size: (u32, u32),
    ) -> Result<ImageBuf, MediaError> {
        let corrupt = |detail: String| MediaError::Corrupt { detail };
        let scene = self
            .scene(model.asset.as_hash(), bytes, format_of(model.format))
            .map_err(|e| corrupt(e.to_string()))?;
        let params = params_of(model, size, 2).clamped();
        let mut pixels = render_parallel(&scene, &params);
        for pixel in pixels.chunks_exact_mut(4) {
            let alpha = pixel[3];
            for channel in &mut pixel[..3] {
                *channel = premultiply_u8(*channel, alpha);
            }
        }
        ImageBuf::new(params.width, params.height, PixelBuf::Rgba8(pixels))
            .map_err(|e| corrupt(e.to_string()))
    }
}
