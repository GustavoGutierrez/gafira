//! Spec 100: the model pose, the camera and the fit (critical logic, test-first).

use gafira_model3d::{
    Camera, CameraKind, Pose, ViewPreset, euler_degrees_from_quat, normalize_degrees,
    quat_from_euler_degrees,
};

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

// Spec 100 §4: rotation is stored as a quaternion; X/Y/Z angles only in the UI.
#[test]
fn s100_euler_round_trip_through_the_quaternion() {
    for (x, y, z) in [
        (0.0, 0.0, 0.0),
        (-20.0, 35.0, 0.0),
        (45.0, -90.0 + 1.0, 30.0),
        (170.0, 10.0, -60.0),
    ] {
        let q = quat_from_euler_degrees([x, y, z]);
        let back = euler_degrees_from_quat(q);
        let again = quat_from_euler_degrees(back);
        // The same orientation, even if the angles are another valid triple.
        let dot: f32 = (0..4).map(|i| q[i] * again[i]).sum();
        assert!(dot.abs() > 0.9999, "{x},{y},{z} -> {back:?}");
    }
    let back = euler_degrees_from_quat(quat_from_euler_degrees([-20.0, 35.0, 0.0]));
    assert!(
        close(back[0], -20.0) && close(back[1], 35.0) && close(back[2], 0.0),
        "{back:?}"
    );
}

// Spec 100 §3.4: angles are normalized to (-180, 180].
#[test]
fn s100_angles_are_normalized() {
    assert!(close(normalize_degrees(190.0), -170.0));
    assert!(close(normalize_degrees(-190.0), 170.0));
    assert!(close(normalize_degrees(180.0), 180.0));
    assert!(close(normalize_degrees(-180.0), 180.0));
    assert!(close(normalize_degrees(720.0 + 15.0), 15.0));
    assert!(close(normalize_degrees(f32::NAN), 0.0));
}

// Spec 100 §4: an identity pose leaves a point unchanged; rotation, scale and position compose.
#[test]
fn s100_pose_transforms_points() {
    let p = Pose::default();
    assert_eq!(p.apply([1.0, 2.0, 3.0]), [1.0, 2.0, 3.0]);
    let p = Pose {
        rotation: quat_from_euler_degrees([0.0, 90.0, 0.0]),
        position: [0.5, 0.0, 0.0],
        scale: [2.0, 2.0, 2.0],
    };
    let q = p.apply([1.0, 0.0, 0.0]);
    // Scale 2, then +90° about Y sends +X to -Z, then move by +0.5 X.
    assert!(
        close(q[0], 0.5) && close(q[1], 0.0) && close(q[2], -2.0),
        "{q:?}"
    );
}

// Spec 100 AC-07: the quick views give the same orientation every time.
#[test]
fn s100_quick_views_are_deterministic() {
    for preset in ViewPreset::ALL {
        let a = Camera::default().with_preset(preset);
        let b = Camera::default().with_preset(preset).with_preset(preset);
        assert_eq!(a, b, "{preset:?}");
    }
    let front = Camera::default().with_preset(ViewPreset::Front);
    assert!(close(front.yaw_degrees, 0.0) && close(front.elevation_degrees, 0.0));
    let top = Camera::default().with_preset(ViewPreset::Top);
    assert!(close(top.elevation_degrees, 90.0));
    let iso = Camera::default().with_preset(ViewPreset::Isometric);
    assert!(close(iso.yaw_degrees, -45.0) && close(iso.elevation_degrees, 35.264));
}

// Spec 100 §3.4: the front camera looks down -Z from +Z, so the model's front faces the viewer.
#[test]
fn s100_front_camera_sees_the_front() {
    let cam = Camera::default().with_preset(ViewPreset::Front);
    let eye = cam.eye();
    assert!(
        eye[2] > 0.0 && close(eye[0], 0.0) && close(eye[1], 0.0),
        "{eye:?}"
    );
    // The model centre projects to the centre of the frame.
    let c = cam.project([0.0, 0.0, 0.0], 400.0, 300.0).expect("visible");
    assert!(close(c[0], 200.0) && close(c[1], 150.0), "{c:?}");
    // +Y is up on screen, +X is right.
    let up = cam.project([0.0, 0.5, 0.0], 400.0, 300.0).expect("visible");
    let right = cam.project([0.5, 0.0, 0.0], 400.0, 300.0).expect("visible");
    assert!(up[1] < 150.0 && right[0] > 200.0);
}

// Spec 100 §3.1 and "Encuadrar": at zoom 1 the unit sphere (the normalized model) fits the
// smaller side of the frame, in perspective and orthographic.
#[test]
fn s100_fit_keeps_the_unit_sphere_inside_the_frame() {
    for kind in [CameraKind::Perspective, CameraKind::Orthographic] {
        for (w, h) in [(400.0, 300.0), (300.0, 400.0), (500.0, 500.0)] {
            let cam = Camera {
                kind,
                ..Camera::default()
            }
            .with_preset(ViewPreset::Isometric);
            let min_side = f32::min(w, h);
            for i in 0..64 {
                let a = i as f32 / 64.0 * std::f32::consts::TAU;
                for b in [-1.2_f32, -0.6, 0.0, 0.6, 1.2] {
                    let pt = [a.cos() * b.cos(), b.sin(), a.sin() * b.cos()];
                    let s = cam.project(pt, w, h).expect("visible");
                    let dx = s[0] - w / 2.0;
                    let dy = s[1] - h / 2.0;
                    assert!(
                        (dx * dx + dy * dy).sqrt() <= min_side / 2.0 + 0.5,
                        "{kind:?} {w}x{h} {s:?}"
                    );
                }
            }
        }
    }
}

// Spec 100 §3.4: zoom changes the camera distance, never the element; zoom 2 doubles the size.
#[test]
fn s100_zoom_scales_the_projection() {
    for kind in [CameraKind::Perspective, CameraKind::Orthographic] {
        let base = Camera {
            kind,
            ..Camera::default()
        }
        .with_preset(ViewPreset::Front);
        let near = Camera { zoom: 2.0, ..base };
        let a = base.project([0.0, 0.0, 0.0], 400.0, 400.0).expect("v");
        let b = base.project([0.0, 0.1, 0.0], 400.0, 400.0).expect("v");
        let c = near.project([0.0, 0.1, 0.0], 400.0, 400.0).expect("v");
        let d1 = a[1] - b[1];
        let d2 = a[1] - c[1];
        assert!((d2 / d1 - 2.0).abs() < 0.05, "{kind:?} {d1} {d2}");
    }
    assert_eq!(
        Camera {
            zoom: 100.0,
            ..Camera::default()
        }
        .clamped()
        .zoom,
        Camera::MAX_ZOOM
    );
    assert_eq!(
        Camera {
            zoom: 0.0,
            ..Camera::default()
        }
        .clamped()
        .zoom,
        Camera::MIN_ZOOM
    );
}

// Spec 100 §3.4 "Girar": turning about the screen's vertical axis keeps the rotation a unit
// quaternion and turning back restores it.
#[test]
fn s100_view_turns_are_reversible() {
    let cam = Camera::default().with_preset(ViewPreset::Isometric);
    let start = Pose::default();
    let turned = start.turned_in_view(&cam, 30.0, -15.0);
    let back = turned
        .turned_in_view(&cam, 0.0, 15.0)
        .turned_in_view(&cam, -30.0, 0.0);
    let n: f32 = turned.rotation.iter().map(|v| v * v).sum();
    assert!(close(n, 1.0));
    let dot: f32 = (0..4).map(|i| back.rotation[i] * start.rotation[i]).sum();
    assert!(dot.abs() > 0.999, "{:?}", back.rotation);
}

// Spec 100 §9: non-finite input never reaches the pose.
#[test]
fn s100_pose_sanitizes_non_finite_values() {
    let p = Pose {
        rotation: [f32::NAN, 0.0, 0.0, 1.0],
        position: [f32::INFINITY, 0.0, 0.0],
        scale: [0.0, -1.0, f32::NAN],
    }
    .sanitized();
    assert_eq!(p.rotation, [0.0, 0.0, 0.0, 1.0]);
    assert_eq!(p.position, [0.0, 0.0, 0.0]);
    assert!(
        p.scale
            .iter()
            .all(|s| *s >= Pose::MIN_SCALE && s.is_finite())
    );
}
