//! The pose of the model inside its own frame (spec 100 §4): rotation, position and scale,
//! separate from the element's 2D transform on the page.

use glam::{EulerRot, Quat, Vec3};

use crate::Camera;

/// Wraps an angle into (-180, 180]; a non-finite angle becomes 0.
pub fn normalize_degrees(degrees: f32) -> f32 {
    if !degrees.is_finite() {
        return 0.0;
    }
    let wrapped = degrees.rem_euclid(360.0);
    let d = if wrapped > 180.0 {
        wrapped - 360.0
    } else {
        wrapped
    };
    // -180 and 180 are the same angle; the range keeps 180.
    if d <= -180.0 { 180.0 } else { d }
}

/// The quaternion `[x, y, z, w]` of the X, then Y, then Z angles in degrees (intrinsic XYZ).
pub fn quat_from_euler_degrees(euler: [f32; 3]) -> [f32; 4] {
    let [x, y, z] = euler.map(|d| if d.is_finite() { d.to_radians() } else { 0.0 });
    Quat::from_euler(EulerRot::XYZ, x, y, z)
        .normalize()
        .to_array()
}

/// The X, Y and Z angles in degrees, each in (-180, 180], of a rotation quaternion.
pub fn euler_degrees_from_quat(q: [f32; 4]) -> [f32; 3] {
    let q = to_quat(q);
    let (x, y, z) = q.to_euler(EulerRot::XYZ);
    [x, y, z].map(|r| {
        let d = normalize_degrees(r.to_degrees());
        // Snap the float noise of exact angles (35.000004 -> 35).
        let rounded = (d * 1000.0).round() / 1000.0;
        if rounded == -0.0 { 0.0 } else { rounded }
    })
}

pub(crate) fn to_quat(q: [f32; 4]) -> Quat {
    let quat = Quat::from_array(q);
    if !quat.is_finite() || quat.length_squared() < 1e-12 {
        Quat::IDENTITY
    } else {
        quat.normalize()
    }
}

/// The model's rotation, position and scale in the normalized model space, where the imported
/// model is centred on the origin and fits a sphere of radius 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    /// Unit quaternion `[x, y, z, w]`.
    pub rotation: [f32; 4],
    pub position: [f32; 3],
    pub scale: [f32; 3],
}

impl Default for Pose {
    fn default() -> Self {
        Self {
            rotation: [0.0, 0.0, 0.0, 1.0],
            position: [0.0; 3],
            scale: [1.0; 3],
        }
    }
}

impl Pose {
    pub const MIN_SCALE: f32 = 0.05;
    pub const MAX_SCALE: f32 = 20.0;
    /// How far the model may move from the centre of its frame, in model radii.
    pub const MAX_OFFSET: f32 = 10.0;

    /// Scale, then rotate, then move a point of the model.
    pub fn apply(&self, point: [f32; 3]) -> [f32; 3] {
        let p = Vec3::from_array(point) * Vec3::from_array(self.scale);
        (to_quat(self.rotation) * p + Vec3::from_array(self.position)).to_array()
    }

    /// Turns the model about the screen's vertical axis by `yaw_degrees`, then about the
    /// screen's horizontal axis by `pitch_degrees`, as seen from `camera`.
    pub fn turned_in_view(&self, camera: &Camera, yaw_degrees: f32, pitch_degrees: f32) -> Self {
        let basis = camera.basis();
        let up = Quat::from_axis_angle(basis.up, finite(yaw_degrees).to_radians());
        let right = Quat::from_axis_angle(basis.right, -finite(pitch_degrees).to_radians());
        let rotation = (right * up * to_quat(self.rotation)).normalize();
        Self {
            rotation: rotation.to_array(),
            ..*self
        }
    }

    /// Turns the model about one of its frame's axes (0 = X, 1 = Y, 2 = Z) by `degrees`.
    pub fn turned_about_axis(&self, axis: usize, degrees: f32) -> Self {
        let a = match axis {
            0 => Vec3::X,
            1 => Vec3::Y,
            _ => Vec3::Z,
        };
        let r = Quat::from_axis_angle(a, finite(degrees).to_radians());
        Self {
            rotation: (r * to_quat(self.rotation)).normalize().to_array(),
            ..*self
        }
    }

    /// The same pose with every value finite and inside its limits.
    pub fn sanitized(&self) -> Self {
        let rotation = to_quat(self.rotation).to_array();
        let position = self.position.map(|v| {
            if v.is_finite() {
                v.clamp(-Self::MAX_OFFSET, Self::MAX_OFFSET)
            } else {
                0.0
            }
        });
        let scale = self.scale.map(|s| {
            if s.is_finite() {
                s.clamp(Self::MIN_SCALE, Self::MAX_SCALE)
            } else {
                1.0
            }
        });
        Self {
            rotation,
            position,
            scale,
        }
    }
}

fn finite(v: f32) -> f32 {
    if v.is_finite() { v } else { 0.0 }
}
