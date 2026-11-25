//! The orbit camera around the model's frame (spec 100 §3.4): turn (yaw), elevation and zoom,
//! in perspective or orthographic projection.

use glam::{Quat, Vec3};

use crate::pose::normalize_degrees;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CameraKind {
    #[default]
    Perspective,
    Orthographic,
}

/// The quick views of spec 100 §3.4 (AC-07).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewPreset {
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
    Isometric,
}

impl ViewPreset {
    pub const ALL: [ViewPreset; 7] = [
        Self::Front,
        Self::Left,
        Self::Right,
        Self::Top,
        Self::Bottom,
        Self::Back,
        Self::Isometric,
    ];

    /// The (yaw, elevation) of the view, in degrees.
    pub fn angles(self) -> (f32, f32) {
        match self {
            Self::Front => (0.0, 0.0),
            Self::Back => (180.0, 0.0),
            Self::Left => (90.0, 0.0),
            Self::Right => (-90.0, 0.0),
            Self::Top => (0.0, 90.0),
            Self::Bottom => (0.0, -90.0),
            // The classic isometric view: 45° around, arctan(1/√2) up.
            Self::Isometric => (-45.0, 35.264),
        }
    }

    /// The preset whose angles match these, if any.
    pub fn matching(yaw: f32, elevation: f32) -> Option<Self> {
        Self::ALL.into_iter().find(|p| {
            let (y, e) = p.angles();
            let dy = normalize_degrees(yaw - y).abs();
            (elevation - e).abs() < 0.05 && dy < 0.05
        })
    }
}

/// The camera's orthonormal frame in model-frame coordinates.
#[derive(Debug, Clone, Copy)]
pub struct CameraBasis {
    pub right: Vec3,
    pub up: Vec3,
    /// Points from the target towards the eye.
    pub back: Vec3,
}

/// An orbit camera aimed at the centre of the model's frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub kind: CameraKind,
    /// Turn around the vertical axis, degrees in (-180, 180]; positive moves the eye to the left.
    pub yaw_degrees: f32,
    /// Height of the eye, degrees in [-90, 90].
    pub elevation_degrees: f32,
    /// 1 frames the whole model; 2 shows it twice as large.
    pub zoom: f32,
    /// Vertical field of view of the perspective camera, degrees.
    pub fov_degrees: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            kind: CameraKind::Perspective,
            yaw_degrees: 0.0,
            elevation_degrees: 0.0,
            zoom: 1.0,
            fov_degrees: 35.0,
        }
    }
}

/// How much of the half frame the unit sphere fills at zoom 1, so it never touches the border.
const FIT_MARGIN: f32 = 0.98;
const NEAR: f32 = 0.01;

impl Camera {
    pub const MIN_ZOOM: f32 = 0.2;
    pub const MAX_ZOOM: f32 = 8.0;
    pub const MIN_FOV: f32 = 10.0;
    pub const MAX_FOV: f32 = 90.0;

    pub fn with_preset(self, preset: ViewPreset) -> Self {
        let (yaw_degrees, elevation_degrees) = preset.angles();
        Self {
            yaw_degrees,
            elevation_degrees,
            ..self
        }
    }

    /// The same camera with every value finite and inside its limits.
    pub fn clamped(self) -> Self {
        let fin = |v: f32, d: f32| if v.is_finite() { v } else { d };
        Self {
            kind: self.kind,
            yaw_degrees: normalize_degrees(self.yaw_degrees),
            elevation_degrees: fin(self.elevation_degrees, 0.0).clamp(-90.0, 90.0),
            zoom: fin(self.zoom, 1.0).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM),
            fov_degrees: fin(self.fov_degrees, 35.0).clamp(Self::MIN_FOV, Self::MAX_FOV),
        }
    }

    pub(crate) fn orientation(&self) -> Quat {
        let c = self.clamped();
        Quat::from_rotation_y(-c.yaw_degrees.to_radians())
            * Quat::from_rotation_x(-c.elevation_degrees.to_radians())
    }

    pub fn basis(&self) -> CameraBasis {
        let q = self.orientation();
        CameraBasis {
            right: q * Vec3::X,
            up: q * Vec3::Y,
            back: q * Vec3::Z,
        }
    }

    fn half_tan(&self) -> f32 {
        (self.clamped().fov_degrees.to_radians() / 2.0).tan()
    }

    /// Distance from the eye to the centre: the unit sphere fills the frame at zoom 1.
    pub fn distance(&self) -> f32 {
        let t = self.half_tan() * FIT_MARGIN;
        (1.0 + 1.0 / (t * t)).sqrt()
    }

    /// Position of the eye in the model's frame.
    pub fn eye(&self) -> [f32; 3] {
        (self.basis().back * self.distance()).to_array()
    }

    /// Point in view space: x right, y up, z towards the viewer (visible points have z < 0).
    pub fn to_view(&self, point: [f32; 3]) -> [f32; 3] {
        let b = self.basis();
        let p = Vec3::from_array(point);
        match self.kind {
            CameraKind::Perspective => {
                let rel = p - b.back * self.distance();
                [rel.dot(b.right), rel.dot(b.up), rel.dot(b.back)]
            }
            // The orthographic eye sits far enough back that the whole unit sphere is in front.
            CameraKind::Orthographic => {
                let rel = p - b.back * ORTHO_EYE_DISTANCE;
                [rel.dot(b.right), rel.dot(b.up), rel.dot(b.back)]
            }
        }
    }

    /// Normalized device coordinates of a view-space point: (±1 at the half of the frame's
    /// smaller side) plus the view depth; `None` behind the eye.
    pub fn ndc(&self, view: [f32; 3]) -> Option<[f32; 3]> {
        let c = self.clamped();
        let [x, y, z] = view;
        match c.kind {
            CameraKind::Perspective => {
                if -z < NEAR {
                    return None;
                }
                let f = c.zoom / self.half_tan();
                Some([f * x / -z, f * y / -z, -z])
            }
            CameraKind::Orthographic => {
                let f = c.zoom * FIT_MARGIN;
                Some([f * x, f * y, -z])
            }
        }
    }

    /// Screen position (pixels, y down) of a model-frame point in a `width` × `height` frame.
    pub fn project(&self, point: [f32; 3], width: f32, height: f32) -> Option<[f32; 2]> {
        let [nx, ny, _] = self.ndc(self.to_view(point))?;
        let half = width.min(height) / 2.0;
        Some([width / 2.0 + nx * half, height / 2.0 - ny * half])
    }

    /// The eye used by [`Self::to_view`]: the orthographic eye sits farther back.
    pub(crate) fn eye_for_render(&self) -> [f32; 3] {
        let d = match self.kind {
            CameraKind::Perspective => self.distance(),
            CameraKind::Orthographic => ORTHO_EYE_DISTANCE,
        };
        (self.basis().back * d).to_array()
    }

    pub(crate) fn near() -> f32 {
        NEAR
    }
}

/// The farthest a moved and scaled model can reach from the centre, for the orthographic eye.
const ORTHO_EYE_DISTANCE: f32 = 41.0;
