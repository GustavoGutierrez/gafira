//! Rotating and flipping an element (spec 041) and the transform that puts
//! its local box on the screen. Pure functions over a `Placement`, so they
//! are tested without a window.
//!
//! The document stores the rotation as it is given (`Placement` never
//! normalizes); everything the interface writes goes through
//! [`normalized`], so a rotation set from the canvas, the Inspector or the
//! Edición menu is always in `[0, 360)` degrees, clockwise on the y-down
//! page.

use egui::{Pos2, Rect};
use gafira_doc::Placement;
use gafira_geom::{Affine, Point};

/// `degrees` in `[0, 360)`. A `-0.0` and a value that rounds up to a full
/// turn give `+0.0`; a non-finite value comes back as it is, for the
/// command to refuse.
pub(crate) fn normalized(degrees: f64) -> f64 {
    let turn = degrees.rem_euclid(360.0);
    // `rem_euclid` of a tiny negative value rounds to 360 itself.
    if turn >= 360.0 { 0.0 } else { turn + 0.0 }
}

/// `degrees` snapped to the nearest multiple of 15 (FD-06: halfway goes to
/// the larger one) and normalized. A value the snap refuses comes back
/// normalized but unsnapped.
pub(crate) fn snapped(degrees: f64) -> f64 {
    normalized(gafira_layout::snap_rotation(degrees).unwrap_or(degrees))
}

/// `origin` turned by `delta` degrees clockwise about its centre: the
/// box keeps its place and size, only the rotation changes.
pub(crate) fn turned_by(origin: &Placement, delta: f64) -> Placement {
    let mut placement = *origin;
    placement.rotation_deg = normalized(origin.rotation_deg + delta);
    placement
}

/// The axis of a flip, in page space (spec 041).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Axis {
    /// Mirror left to right (Mayús+H).
    Horizontal,
    /// Mirror top to bottom (Mayús+V).
    Vertical,
}

/// `origin` mirrored about its centre along the page `axis`, whatever its
/// rotation. The local flip is applied before the rotation and the shear
/// (`Placement::to_transform`), so a page mirror is the local flip of that
/// axis with the rotation and the shear negated: `F·R(θ)·S(k) = R(-θ)·S(-k)·F`.
pub(crate) fn flipped(origin: &Placement, axis: Axis) -> Placement {
    let mut placement = *origin;
    match axis {
        Axis::Horizontal => placement.flip.x = !origin.flip.x,
        Axis::Vertical => placement.flip.y = !origin.flip.y,
    }
    placement.rotation_deg = normalized(-origin.rotation_deg);
    placement.skew = -origin.skew + 0.0;
    placement
}

/// Whether `placement` draws as its plain box: no rotation, shear or flip.
pub(crate) fn upright(placement: &Placement) -> bool {
    normalized(placement.rotation_deg) == 0.0
        && placement.skew == 0.0
        && !placement.flip.x
        && !placement.flip.y
}

/// The transform from the local box of `placement` (`[0,w] x [0,h]`, page
/// px) to the screen, for a page drawn at `page_rect` with `zoom`: the same
/// placement transform the render draws with (`gafira-render`, D-35),
/// scaled by the zoom and moved to the page.
pub(crate) fn to_screen(page_rect: Rect, zoom: f32, placement: &Placement) -> Affine {
    Affine::translate((f64::from(page_rect.min.x), f64::from(page_rect.min.y)))
        * Affine::scale(f64::from(zoom))
        * placement.to_transform()
}

/// Where the local point `(x, y)` of `transform` lands on the screen.
pub(crate) fn screen(transform: Affine, x: f64, y: f64) -> Pos2 {
    let at = transform * Point::new(x, y);
    Pos2::new(at.x as f32, at.y as f32)
}

/// The local box point under the screen point `pointer`, or `None` when
/// the box is flat (no area) or the transform is not finite.
pub(crate) fn local(transform: Affine, pointer: Pos2) -> Option<Point> {
    let det = transform.determinant();
    if !det.is_finite() || det.abs() < 1e-12 {
        return None;
    }
    let at = transform.inverse() * Point::new(f64::from(pointer.x), f64::from(pointer.y));
    (at.x.is_finite() && at.y.is_finite()).then_some(at)
}

/// The four corners of the local box of `placement` on the screen,
/// clockwise from the local top-left.
pub(crate) fn corners(transform: Affine, placement: &Placement) -> [Pos2; 4] {
    let (w, h) = (placement.w, placement.h);
    [
        screen(transform, 0.0, 0.0),
        screen(transform, w, 0.0),
        screen(transform, w, h),
        screen(transform, 0.0, h),
    ]
}

/// The screen box aligned to the axes that holds the turned box of
/// `placement`: what the marquee and the box of a multi-selection use
/// (FD-06, D-28: a turned node counts by its axis-aligned box).
pub(crate) fn screen_box(page_rect: Rect, zoom: f32, placement: &Placement) -> Rect {
    let [a, b, c, d] = corners(to_screen(page_rect, zoom, placement), placement);
    Rect::from_points(&[a, b, c, d])
}

/// The gap between the top edge and the rotate handle, in screen px: one
/// 24 px target (`size.target.min` of 03) above the top-middle handle, so
/// the two hit areas touch and never overlap.
pub(crate) const ROTATE_GAP: f32 = 24.0;

/// Where the rotate handle sits: `ROTATE_GAP` screen px out from the middle
/// of the local top edge, along the local up direction, so it turns with
/// the element.
pub(crate) fn rotate_handle(transform: Affine, placement: &Placement) -> Pos2 {
    let top = screen(transform, placement.w / 2.0, 0.0);
    let [a, b, c, d, ..] = transform.as_coeffs();
    let up = (Affine::new([a, b, c, d, 0.0, 0.0]) * Point::new(0.0, -1.0)).to_vec2();
    let length = up.hypot();
    if !length.is_finite() || length == 0.0 {
        return top;
    }
    let step = up * (f64::from(ROTATE_GAP) / length);
    Pos2::new(top.x + step.x as f32, top.y + step.y as f32)
}

/// `origin` turned by a drag of the rotate handle about `center` (screen
/// px) from `from` to `to`: the change of the pointer's angle around the
/// centre is added to the rotation. With `snap` (Shift) the result goes to
/// the nearest multiple of 15° (FD-06). Always normalized.
pub(crate) fn rotated(
    origin: &Placement,
    center: Pos2,
    from: Pos2,
    to: Pos2,
    snap: bool,
) -> Placement {
    let angle = |p: Pos2| f64::from(p.y - center.y).atan2(f64::from(p.x - center.x));
    let delta = (angle(to) - angle(from)).to_degrees();
    let mut placement = turned_by(origin, delta);
    if snap {
        placement.rotation_deg = snapped(origin.rotation_deg + delta);
    }
    placement
}

#[cfg(test)]
mod tests {
    use egui::Vec2 as ScreenVec;
    use gafira_doc::Flip;
    use proptest::prelude::*;

    use super::*;

    fn place(x: f64, y: f64, w: f64, h: f64, rotation: f64) -> Placement {
        let mut p = Placement::new(x, y, w, h);
        p.rotation_deg = rotation;
        p
    }

    fn near(a: Pos2, b: Pos2) -> bool {
        (a - b).length() < 1e-3
    }

    // S41-01: every rotation the interface writes is in [0, 360).
    #[test]
    fn s41_01_rotations_are_normalized_to_one_turn() {
        for (input, want) in [
            (0.0, 0.0),
            (90.0, 90.0),
            (360.0, 0.0),
            (450.0, 90.0),
            (-90.0, 270.0),
            (-360.0, 0.0),
            (-720.5, 359.5),
            (359.999, 359.999),
        ] {
            assert_eq!(normalized(input), want, "{input}");
        }
        assert_eq!(normalized(-0.0).to_bits(), 0.0f64.to_bits(), "no -0.0");
        // rem_euclid of a tiny negative value rounds up to 360 itself.
        assert_eq!(normalized(-1e-20), 0.0);
        assert!(normalized(-1e-20) < 360.0);
        assert!(normalized(f64::NAN).is_nan());
        assert!(!normalized(f64::INFINITY).is_finite());
    }

    // S41-02: Shift snaps to the nearest 15° (FD-06 (i)) and stays in one
    // turn.
    #[test]
    fn s41_02_shift_snaps_to_15_degrees() {
        assert_eq!(snapped(7.4), 0.0);
        assert_eq!(snapped(7.5), 15.0);
        assert_eq!(snapped(-7.5), 0.0, "halfway goes to the larger one");
        assert_eq!(snapped(-8.0), 345.0);
        assert_eq!(snapped(352.5), 0.0, "a full turn is 0");
        assert_eq!(snapped(400.0), 45.0);
        assert!(snapped(f64::NAN).is_nan());
    }

    // S41-03: a turn by 90° keeps the box and its centre.
    #[test]
    fn s41_03_turning_keeps_the_box_and_wraps() {
        let origin = place(10.0, 20.0, 40.0, 10.0, 300.0);
        let right = turned_by(&origin, 90.0);
        assert_eq!(right.rotation_deg, 30.0);
        assert_eq!(
            (right.x, right.y, right.w, right.h),
            (10.0, 20.0, 40.0, 10.0)
        );
        let left = turned_by(&place(0.0, 0.0, 1.0, 1.0, 0.0), -90.0);
        assert_eq!(left.rotation_deg, 270.0);
        // The centre is a fixed point of the turn.
        let c = right.to_transform() * Point::new(20.0, 5.0);
        assert!((c.x - 30.0).abs() < 1e-9 && (c.y - 25.0).abs() < 1e-9);
    }

    // S41-04: a flip mirrors the element about its centre along the page
    // axis, whatever its rotation and shear.
    proptest! {
        #[test]
        fn s41_04_a_flip_is_a_page_mirror_about_the_centre(
            x in -1e3f64..1e3, y in -1e3f64..1e3,
            w in 1.0f64..1e3, h in 1.0f64..1e3,
            rot in 0.0f64..360.0, skew in -60.0f64..60.0,
            fx: bool, fy: bool, horizontal: bool,
            u in 0.0f64..1.0, v in 0.0f64..1.0,
        ) {
            let mut origin = place(x, y, w, h, rot);
            origin.skew = skew;
            origin.flip = Flip { x: fx, y: fy };
            let axis = if horizontal { Axis::Horizontal } else { Axis::Vertical };
            let flipped = flipped(&origin, axis);
            let (cx, cy) = (x + w / 2.0, y + h / 2.0);
            let mirror = match axis {
                Axis::Horizontal => Affine::new([-1.0, 0.0, 0.0, 1.0, 2.0 * cx, 0.0]),
                Axis::Vertical => Affine::new([1.0, 0.0, 0.0, -1.0, 0.0, 2.0 * cy]),
            };
            let local = Point::new(u * w, v * h);
            let want = mirror * origin.to_transform() * local;
            let got = flipped.to_transform() * local;
            let tol = 1e-6 * (1.0 + want.x.abs().max(want.y.abs()));
            prop_assert!((got.x - want.x).abs() < tol && (got.y - want.y).abs() < tol,
                "{got:?} vs {want:?}");
            prop_assert!((0.0..360.0).contains(&flipped.rotation_deg));
            prop_assert_eq!((flipped.x, flipped.y, flipped.w, flipped.h), (x, y, w, h));
        }
    }

    #[test]
    fn s41_04_flipping_twice_gives_the_element_back() {
        let mut origin = place(5.0, 6.0, 30.0, 20.0, 30.0);
        origin.skew = 10.0;
        let once = flipped(&origin, Axis::Horizontal);
        assert_eq!(once.flip, Flip { x: true, y: false });
        assert_eq!(once.rotation_deg, 330.0);
        assert_eq!(once.skew, -10.0);
        let twice = flipped(&once, Axis::Horizontal);
        assert_eq!(twice, origin);
        let vertical = flipped(&place(0.0, 0.0, 1.0, 1.0, 0.0), Axis::Vertical);
        assert_eq!(vertical.flip, Flip { x: false, y: true });
        assert_eq!(vertical.rotation_deg.to_bits(), 0.0f64.to_bits());
        assert_eq!(vertical.skew.to_bits(), 0.0f64.to_bits(), "no -0.0");
    }

    // S41-05: the canvas puts a box where the render does: the render's
    // transform (`scale * placement`, D-35) moved to the page.
    #[test]
    fn s41_05_the_screen_transform_is_the_render_transform_on_the_page() {
        let mut p = place(10.0, 20.0, 40.0, 10.0, 90.0);
        p.flip.x = true;
        let page = Rect::from_min_size(Pos2::new(100.0, 50.0), ScreenVec::splat(500.0));
        let t = to_screen(page, 2.0, &p);
        let render = Affine::scale(2.0) * p.to_transform();
        for (u, v) in [(0.0, 0.0), (40.0, 0.0), (40.0, 10.0), (13.0, 7.0)] {
            let r = render * Point::new(u, v);
            let want = Pos2::new(100.0 + r.x as f32, 50.0 + r.y as f32);
            assert!(near(screen(t, u, v), want), "{u},{v}");
        }
        // Turned 90° about (30, 25): the box stands upright, 10 wide, 40 high.
        let bbox = screen_box(page, 1.0, &place(10.0, 20.0, 40.0, 10.0, 90.0));
        assert!(near(bbox.min, Pos2::new(125.0, 55.0)), "{bbox:?}");
        assert!(near(bbox.max, Pos2::new(135.0, 95.0)), "{bbox:?}");
    }

    // S41-06: a screen point goes back to the local box, through rotation
    // and flip; a flat box has no inverse.
    #[test]
    fn s41_06_a_screen_point_maps_back_to_the_local_box() {
        let page = Rect::from_min_size(Pos2::ZERO, ScreenVec::splat(500.0));
        let mut p = place(10.0, 20.0, 40.0, 10.0, 45.0);
        p.flip.y = true;
        let t = to_screen(page, 3.0, &p);
        let back = local(t, screen(t, 12.0, 3.0)).expect("invertible");
        assert!((back.x - 12.0).abs() < 1e-3 && (back.y - 3.0).abs() < 1e-3);
        let flat = to_screen(page, 1.0, &place(0.0, 0.0, 0.0, 10.0, 0.0));
        assert!(
            local(flat, Pos2::new(0.0, 5.0)).is_some(),
            "w 0 still inverts"
        );
        let zero = Affine::new([0.0, 0.0, 0.0, 0.0, 1.0, 1.0]);
        assert_eq!(local(zero, Pos2::ZERO), None);
        let nan = to_screen(page, 1.0, &place(f64::NAN, 0.0, 1.0, 1.0, 0.0));
        assert_eq!(local(nan, Pos2::ZERO), None);
    }

    // S41-07: the rotate handle sits 24 px out from the top middle, along
    // the element's own up direction.
    #[test]
    fn s41_07_the_rotate_handle_turns_with_the_element() {
        let page = Rect::from_min_size(Pos2::ZERO, ScreenVec::splat(500.0));
        let upright = place(100.0, 100.0, 40.0, 20.0, 0.0);
        let t = to_screen(page, 2.0, &upright);
        assert!(near(rotate_handle(t, &upright), Pos2::new(240.0, 176.0)));
        // Turned 90° clockwise the top edge faces right.
        let turned = place(100.0, 100.0, 40.0, 20.0, 90.0);
        let t = to_screen(page, 1.0, &turned);
        // Centre (120, 110); the top middle is 10 px to its right.
        assert!(
            near(rotate_handle(t, &turned), Pos2::new(154.0, 110.0)),
            "{:?}",
            rotate_handle(t, &turned)
        );
    }

    // S41-08: dragging the rotate handle adds the change of angle about the
    // centre; Shift snaps the result.
    #[test]
    fn s41_08_a_drag_of_the_rotate_handle_turns_by_the_pointer_angle() {
        let origin = place(0.0, 0.0, 10.0, 10.0, 10.0);
        let c = Pos2::new(100.0, 100.0);
        // From straight up to straight right: a quarter turn clockwise.
        let quarter = rotated(
            &origin,
            c,
            Pos2::new(100.0, 50.0),
            Pos2::new(130.0, 100.0),
            false,
        );
        assert!(
            (quarter.rotation_deg - 100.0).abs() < 1e-9,
            "{}",
            quarter.rotation_deg
        );
        // Back past the start wraps below zero into the turn.
        let back = rotated(
            &origin,
            c,
            Pos2::new(100.0, 50.0),
            Pos2::new(50.0, 100.0),
            false,
        );
        assert!(
            (back.rotation_deg - 280.0).abs() < 1e-9,
            "{}",
            back.rotation_deg
        );
        // 10° + 20° = 30°: a multiple already; 10° + 25° = 35° snaps to 30°.
        let up = Pos2::new(100.0, 0.0);
        let by = |deg: f64| {
            let a = (deg - 90.0).to_radians();
            Pos2::new(
                100.0 + 100.0 * a.cos() as f32,
                100.0 + 100.0 * a.sin() as f32,
            )
        };
        let snap = rotated(&origin, c, up, by(25.0), true);
        assert_eq!(snap.rotation_deg, 30.0);
        let free = rotated(&origin, c, up, by(25.0), false);
        assert!((free.rotation_deg - 35.0).abs() < 1e-3);
        assert_eq!((snap.x, snap.y, snap.w, snap.h), (0.0, 0.0, 10.0, 10.0));
    }

    #[test]
    fn s41_09_upright_means_no_turn_shear_or_flip() {
        assert!(upright(&place(0.0, 0.0, 1.0, 1.0, 0.0)));
        assert!(upright(&place(0.0, 0.0, 1.0, 1.0, 360.0)));
        assert!(!upright(&place(0.0, 0.0, 1.0, 1.0, 90.0)));
        let mut p = place(0.0, 0.0, 1.0, 1.0, 0.0);
        p.flip.x = true;
        assert!(!upright(&p));
        p.flip = Flip { x: false, y: true };
        assert!(!upright(&p));
        p.flip.y = false;
        p.skew = 5.0;
        assert!(!upright(&p));
    }
}
