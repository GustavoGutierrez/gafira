//! Layer-2 domain crate `gafira-model3d` (spec 100): the scene of an imported 3D model,
//! its pose, the camera, the lighting presets and a deterministic CPU rasterizer (D-130).
//! Pure domain code: no I/O, no threads, no clocks.

#![forbid(unsafe_code)]

mod camera;
mod light;
mod pose;
mod raster;
mod scene;

pub use camera::{Camera, CameraBasis, CameraKind, ViewPreset};
pub use light::Lighting;
pub use pose::{Pose, euler_degrees_from_quat, normalize_degrees, quat_from_euler_degrees};
pub use raster::{Prepared, RenderParams, render};
pub use scene::{AlphaMode, Limits, Material, Primitive, Scene, SceneError, Texture};
