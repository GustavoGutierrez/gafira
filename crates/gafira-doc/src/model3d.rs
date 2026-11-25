//! The 3D model element of spec 100: an embedded `.glb` or `.obj` asset and how it is seen —
//! the model's pose in its own frame, the orbit camera and the lighting preset. The pixels
//! on the page are a projection drawn from these (D-130); the element's box, opacity and
//! rotation on the page stay in `ElementBase`, separate from the 3D pose (spec 100 §4).

use serde::{Deserialize, Serialize};

use crate::ContentHash;
use crate::timeline::AssetRef;

/// The file format of the model asset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Model3dFormat {
    Glb,
    Obj,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Model3dProjection {
    #[default]
    Perspective,
    Orthographic,
}

/// The lighting presets of the 3D panel (spec 100).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Model3dLighting {
    /// Bright and neutral, the default.
    #[default]
    Studio,
    Warm,
    Cool,
    /// Low key with a strong side light; files of 0.3.0 call it `dramatic`.
    #[serde(alias = "dramatic")]
    Dark,
    Soft,
    /// Daylight: a warm sun and a blue sky.
    Natural,
}

impl Model3dLighting {
    pub const ALL: [Self; 6] = [
        Self::Studio,
        Self::Warm,
        Self::Cool,
        Self::Dark,
        Self::Soft,
        Self::Natural,
    ];
}

/// The orbit camera aimed at the centre of the model's frame.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Model3dCamera {
    pub projection: Model3dProjection,
    /// Turn around the vertical axis, degrees in (-180, 180].
    pub yaw: f64,
    /// Height of the eye, degrees in [-90, 90].
    pub elevation: f64,
    /// 1 frames the whole model.
    pub zoom: f64,
    /// Vertical field of view of the perspective camera, degrees.
    pub fov: f64,
}

impl Model3dCamera {
    pub const MIN_ZOOM: f64 = 0.2;
    pub const MAX_ZOOM: f64 = 8.0;
    pub const MIN_FOV: f64 = 10.0;
    pub const MAX_FOV: f64 = 90.0;

    /// The first view of an imported model: a three-quarter view from slightly above.
    pub const INITIAL: Self = Self {
        projection: Model3dProjection::Perspective,
        yaw: -30.0,
        elevation: 15.0,
        zoom: 1.0,
        fov: 35.0,
    };

    #[must_use]
    pub fn is_valid(&self) -> bool {
        [self.yaw, self.elevation, self.zoom, self.fov]
            .iter()
            .all(|v| v.is_finite())
            && (-180.0..=180.0).contains(&self.yaw)
            && (-90.0..=90.0).contains(&self.elevation)
            && (Self::MIN_ZOOM..=Self::MAX_ZOOM).contains(&self.zoom)
            && (Self::MIN_FOV..=Self::MAX_FOV).contains(&self.fov)
    }
}

/// A 3D model element (spec 100).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Model3dNode {
    /// The original model file, unchanged, in the asset table (P7).
    pub asset: AssetRef,
    pub format: Model3dFormat,
    /// Unit quaternion `[x, y, z, w]` of the model in its frame.
    pub rotation: [f64; 4],
    /// Offset of the model in its frame, in model radii.
    pub position: [f64; 3],
    pub scale: [f64; 3],
    pub camera: Model3dCamera,
    pub lighting: Model3dLighting,
}

impl Model3dNode {
    pub const MIN_SCALE: f64 = 0.05;
    pub const MAX_SCALE: f64 = 20.0;
    pub const MAX_OFFSET: f64 = 10.0;

    /// A model as it was imported: centred, upright, at the initial view.
    #[must_use]
    pub const fn new(asset: AssetRef, format: Model3dFormat) -> Self {
        Self {
            asset,
            format,
            rotation: [0.0, 0.0, 0.0, 1.0],
            position: [0.0; 3],
            scale: [1.0; 3],
            camera: Model3dCamera::INITIAL,
            lighting: Model3dLighting::Studio,
        }
    }

    /// The same model and asset with the pose, camera and lighting of import (spec 100
    /// «Restablecer»).
    #[must_use]
    pub const fn reset(&self) -> Self {
        Self::new(self.asset, self.format)
    }

    /// Every asset the element needs.
    pub fn assets(&self) -> impl Iterator<Item = AssetRef> {
        std::iter::once(self.asset)
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        let norm: f64 = self.rotation.iter().map(|v| v * v).sum();
        self.rotation.iter().all(|v| v.is_finite())
            && (norm - 1.0).abs() < 1e-3
            && self
                .position
                .iter()
                .all(|v| v.is_finite() && v.abs() <= Self::MAX_OFFSET)
            && self
                .scale
                .iter()
                .all(|v| v.is_finite() && (Self::MIN_SCALE..=Self::MAX_SCALE).contains(v))
            && self.camera.is_valid()
    }

    /// The key its drawn pixels are kept under in memory: the asset and everything that
    /// changes the picture, never the size of the box.
    #[must_use]
    pub fn shown(&self) -> AssetRef {
        // Four runs of FNV-1a from as many starts: a name for what is kept in memory, not a
        // proof of content (as `ImageLook::key`).
        const PRIME: u64 = 0x0000_0100_0000_01b3;
        let written = serde_json::to_vec(self).unwrap_or_default();
        let mut out = [0; 32];
        for (lane, bytes) in out.chunks_exact_mut(8).enumerate() {
            let mut hash = 0x6d6f_6465_6c33_6421 ^ (lane as u64 + 1).wrapping_mul(PRIME);
            for byte in &written {
                hash = (hash ^ u64::from(*byte)).wrapping_mul(PRIME).rotate_left(5);
            }
            bytes.copy_from_slice(&hash.to_le_bytes());
        }
        AssetRef::new(ContentHash(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node() -> Model3dNode {
        Model3dNode::new(AssetRef::new(ContentHash([0xab; 32])), Model3dFormat::Glb)
    }

    // Spec 100 §4: the wire form is plain data, the asset by hash, the rotation a quaternion.
    #[test]
    fn s100_wire_form_round_trips() {
        let n = node();
        let json = serde_json::to_value(&n).expect("writes");
        assert_eq!(json["asset"], "ab".repeat(32));
        assert_eq!(json["format"], "glb");
        assert_eq!(json["rotation"], serde_json::json!([0.0, 0.0, 0.0, 1.0]));
        assert_eq!(json["camera"]["projection"], "perspective");
        assert_eq!(json["lighting"], "studio");
        let back: Model3dNode = serde_json::from_value(json).expect("reads");
        assert_eq!(back, n);
    }

    // Spec 100 §9: non-finite or out-of-range values are refused.
    #[test]
    fn s100_invalid_values_are_refused() {
        assert!(node().is_valid());
        let cases: [fn(&mut Model3dNode); 7] = [
            |n| n.rotation = [0.0, 0.0, 0.0, 0.0],
            |n| n.rotation[0] = f64::NAN,
            |n| n.scale[1] = 0.0,
            |n| n.position[2] = 99.0,
            |n| n.camera.zoom = 50.0,
            |n| n.camera.elevation = 120.0,
            |n| n.camera.fov = f64::INFINITY,
        ];
        for (i, change) in cases.iter().enumerate() {
            let mut n = node();
            change(&mut n);
            assert!(!n.is_valid(), "case {i}");
        }
    }

    // Spec 100 §6.3: the cache key follows every change of the picture, and only those.
    #[test]
    fn s100_the_cache_key_follows_the_picture() {
        let a = node();
        assert_eq!(a.shown(), node().shown());
        let mut b = node();
        b.camera.yaw = 10.0;
        assert_ne!(a.shown(), b.shown());
        let mut c = node();
        c.lighting = Model3dLighting::Dark;
        assert_ne!(a.shown(), c.shown());
        assert_ne!(a.shown(), a.asset);
    }

    // Spec 100: the six lighting presets keep their names; a 0.3.0 file's `dramatic` opens
    // as the dark preset.
    #[test]
    fn s100_lighting_presets_read_and_write() {
        for (light, id) in Model3dLighting::ALL
            .into_iter()
            .zip(["studio", "warm", "cool", "dark", "soft", "natural"])
        {
            assert_eq!(serde_json::to_value(light).expect("writes"), id);
            assert_eq!(
                serde_json::from_value::<Model3dLighting>(id.into()).expect("reads"),
                light
            );
        }
        let old: Model3dLighting = serde_json::from_value("dramatic".into()).expect("reads");
        assert_eq!(old, Model3dLighting::Dark);
    }

    // Spec 100 «Restablecer»: reset keeps the asset and drops the pose.
    #[test]
    fn s100_reset_restores_the_import_state() {
        let mut n = node();
        n.scale = [2.0; 3];
        n.camera.zoom = 3.0;
        assert_eq!(n.reset(), node());
    }
}
