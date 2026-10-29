use gafira_anim::Interpolate;
use gafira_geom::Transform2D;
use proptest::prelude::*;

fn tf(c: [f64; 6]) -> Transform2D {
    Transform2D::from(c)
}

fn bits(t: Transform2D) -> [u64; 6] {
    t.coeffs().map(f64::to_bits)
}

/// §3.4 tolerance: 1e-9 * max(1, |c|) per coefficient.
fn assert_close(got: Transform2D, want: [f64; 6]) {
    let got = got.coeffs();
    for i in 0..6 {
        let tol = 1e-9 * want[i].abs().max(1.0);
        assert!(
            (got[i] - want[i]).abs() <= tol,
            "coeff {i}: {got:?} vs {want:?}"
        );
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn lerp6(a: [f64; 6], b: [f64; 6], t: f32) -> [f64; 6] {
    let mut out = [0.0; 6];
    for i in 0..6 {
        out[i] = lerp(a[i], b[i], f64::from(t));
    }
    out
}

fn linear(c: [f64; 4], tx: f64, ty: f64) -> [f64; 6] {
    [c[0], c[1], c[2], c[3], tx, ty]
}

fn det(c: [f64; 6]) -> f64 {
    c[0] * c[3] - c[1] * c[2]
}

#[test]
fn arq_19_transform_endpoints_are_exact_for_fixed_cases() {
    let cases = [
        (
            tf([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
            tf([2.0, 1.0, -1.0, 3.0, 4.0, 5.0]),
        ),
        (tf([0.0; 6]), tf([1.0, 2.0, 3.0, 4.0, 5.0, 6.0])),
        (
            tf([1.0, 2.0, 2.0, 4.0, 0.3, 0.7]),
            tf([0.0, 0.0, 0.0, 1.0, 0.1, 0.2]),
        ),
        (tf([0.0; 6]), tf([0.0; 6])),
    ];
    for (a, b) in cases {
        assert_eq!(bits(a.interpolate(&b, 0.0)), bits(a));
        assert_eq!(bits(a.interpolate(&b, 1.0)), bits(b));
    }
}

#[test]
fn arq_19_transform_two_translations_interpolate_only_the_translation() {
    let a = tf([1.0, 0.0, 0.0, 1.0, 10.0, -4.0]);
    let b = tf([1.0, 0.0, 0.0, 1.0, 30.0, 8.0]);
    assert_close(a.interpolate(&b, 0.25), [1.0, 0.0, 0.0, 1.0, 15.0, -1.0]);
}

#[test]
fn arq_19_transform_rotation_takes_the_short_way() {
    let r = |deg: f64| Transform2D::rotation(deg.to_radians());
    // 350 to 10 passes through 0, not 180.
    assert_close(
        r(350.0).interpolate(&r(10.0), 0.5),
        [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    );
    assert_close(
        r(10.0).interpolate(&r(350.0), 0.5),
        [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    );
    // 170 to -170 passes through 180.
    assert_close(r(170.0).interpolate(&r(-170.0), 0.5), r(180.0).coeffs());
    // 0 to 90 is a plain 45 at the middle.
    assert_close(r(0.0).interpolate(&r(90.0), 0.5), r(45.0).coeffs());
}

#[test]
fn arq_19_transform_rotation_extrapolates_without_wrapping() {
    let r = |deg: f64| Transform2D::rotation(deg.to_radians());
    assert_close(r(0.0).interpolate(&r(90.0), 2.0), r(180.0).coeffs());
    assert_close(r(0.0).interpolate(&r(90.0), -1.0), r(-90.0).coeffs());
}

#[test]
fn arq_19_transform_two_scales_interpolate_per_axis_and_extrapolate() {
    let a = Transform2D::scale_xy(1.0, 2.0);
    let b = Transform2D::scale_xy(3.0, 6.0);
    assert_close(a.interpolate(&b, 0.5), [2.0, 0.0, 0.0, 4.0, 0.0, 0.0]);
    assert_close(a.interpolate(&b, 1.5), [4.0, 0.0, 0.0, 8.0, 0.0, 0.0]);
    assert_close(a.interpolate(&b, -0.5), [0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn arq_19_transform_flip_is_carried_by_the_y_scale() {
    let a = Transform2D::scale_xy(1.0, -1.0);
    let b = Transform2D::scale_xy(1.0, -3.0);
    assert_close(a.interpolate(&b, 0.5), [1.0, 0.0, 0.0, -2.0, 0.0, 0.0]);
    let c = Transform2D::scale_xy(-1.0, 1.0);
    let d = Transform2D::scale_xy(-3.0, 1.0);
    assert_close(c.interpolate(&d, 0.5), [-2.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
}

#[test]
fn arq_19_transform_flip_blends_the_rotation_of_the_positive_x_scale_frame() {
    // Flip-y to the axis swap [0, 1, 1, 0]: the frames have sx > 0, rotations
    // 0 and 90 degrees, sy = -1. The middle is a rotation of 45 degrees.
    let h = std::f64::consts::FRAC_1_SQRT_2;
    let a = Transform2D::scale_xy(1.0, -1.0);
    let b = tf([0.0, 1.0, 1.0, 0.0, 0.0, 0.0]);
    assert_close(a.interpolate(&b, 0.5), [h, h, h, -h, 0.0, 0.0]);
}

#[test]
fn arq_19_transform_shear_from_identity_is_linear_and_continuous() {
    for k in [0.5, 1.0, -2.0, 10.0, -50.0] {
        let shear = tf([1.0, 0.0, k, 1.0, 0.0, 0.0]);
        let id = Transform2D::IDENTITY;
        assert_close(
            id.interpolate(&shear, 0.5),
            [1.0, 0.0, 0.5 * k, 1.0, 0.0, 0.0],
        );
        let mut prev = id.interpolate(&shear, 0.0).coeffs();
        for step in 1..=1000_u32 {
            let t = step as f32 * 1e-3;
            let cur = id.interpolate(&shear, t).coeffs();
            for i in 0..6 {
                assert!(
                    (cur[i] - prev[i]).abs() <= 1e-2 * k.abs().max(1.0),
                    "k {k} t {t} coeff {i}"
                );
            }
            prev = cur;
        }
    }
}

#[test]
fn arq_19_transform_scale_zero_to_b_scales_the_linear_part_by_t() {
    let b = tf([1.5, 0.5, -0.7, 2.0, 4.0, -6.0]);
    let rotated = Transform2D::rotation(0.7).then(Transform2D::scale_xy(2.0, 3.0));
    for target in [b, rotated, Transform2D::scale_xy(2.0, -3.0)] {
        let bc = target.coeffs();
        for t in [0.1_f32, 0.25, 0.5, 0.9, 1.5] {
            let tt = f64::from(t);
            let want = linear(
                [bc[0] * tt, bc[1] * tt, bc[2] * tt, bc[3] * tt],
                bc[4] * tt,
                bc[5] * tt,
            );
            assert_close(Transform2D::scale(0.0).interpolate(&target, t), want);
        }
    }
}

#[test]
fn arq_19_transform_scale_x_zero_to_identity_gives_half_scale_at_the_middle() {
    let a = Transform2D::scale_xy(0.0, 1.0);
    assert_close(
        a.interpolate(&Transform2D::IDENTITY, 0.5),
        [0.5, 0.0, 0.0, 1.0, 0.0, 0.0],
    );
    assert_close(
        Transform2D::IDENTITY.interpolate(&a, 0.5),
        [0.5, 0.0, 0.0, 1.0, 0.0, 0.0],
    );
}

#[test]
fn arq_19_transform_rank_one_endpoint_jumps_as_documented() {
    let a = tf([1.0, 2.0, 2.0, 4.0, 0.0, 0.0]);
    let id = Transform2D::IDENTITY;
    assert_eq!(bits(a.interpolate(&id, 0.0)), bits(a));
    assert_close(a.interpolate(&id, 0.5), [1.0, 0.0, 0.0, 2.5, 0.0, 0.0]);
    let near = a.interpolate(&id, 1e-6).coeffs();
    let want = [1.0, 0.0, 0.0, 4.0, 0.0, 0.0];
    for i in 0..6 {
        assert!((near[i] - want[i]).abs() <= 1e-5, "coeff {i}: {near:?}");
    }
    // Mirrored: the jump is just before t = 1.
    let near = id.interpolate(&a, 1.0 - 1e-6).coeffs();
    for i in 0..6 {
        assert!((near[i] - want[i]).abs() <= 1e-5, "coeff {i}: {near:?}");
    }
}

#[test]
fn arq_19_transform_both_degenerate_interpolate_the_six_coefficients() {
    let a = tf([1.0, 2.0, 2.0, 4.0, 1.0, -1.0]);
    let b = tf([0.0, 0.0, 0.0, 3.0, 5.0, 7.0]);
    for t in [0.25_f32, 0.5, 1.5, -1.0] {
        assert_close(a.interpolate(&b, t), lerp6(a.coeffs(), b.coeffs(), t));
    }
}

// ARQ-18
#[test]
fn arq_18_transform_is_finite_checks_every_coefficient() {
    let ok = [1.0, 0.0, 0.0, 1.0, 5.0, 6.0];
    assert!(tf(ok).is_finite());
    for i in 0..6 {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut c = ok;
            c[i] = bad;
            assert!(!tf(c).is_finite(), "coeff {i} = {bad}");
        }
    }
}

fn coeff() -> impl Strategy<Value = f64> {
    -3.0_f64..=3.0
}

fn matrix() -> impl Strategy<Value = [f64; 6]> {
    prop::array::uniform6(coeff())
}

fn t01() -> impl Strategy<Value = f32> {
    (1_u32..=99).prop_map(|n| n as f32 / 100.0)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1024))]

    #[test]
    fn arq_19_transform_same_sign_determinants_keep_their_sign(
        a in matrix(), b in matrix(), t in t01(),
    ) {
        prop_assume!(det(a).abs() > 1e-3 && det(b).abs() > 1e-3);
        // Swapping the rows negates the determinant: same sign without rejects.
        let b = if det(a).signum() == det(b).signum() { b } else { [b[2], b[3], b[0], b[1], b[4], b[5]] };
        let r = tf(a).interpolate(&tf(b), t).coeffs();
        prop_assert_eq!(det(r).signum(), det(a).signum(), "{:?}", r);
    }

    #[test]
    fn arq_19_transform_endpoints_are_exact(
        a in matrix(), b in matrix(), degenerate in any::<bool>(),
    ) {
        // A degenerate endpoint: second row is a multiple of the first.
        let a = if degenerate { [a[0], a[1], 2.0 * a[0], 2.0 * a[1], a[4], a[5]] } else { a };
        let (ta, tb) = (tf(a), tf(b));
        prop_assert_eq!(bits(ta.interpolate(&tb, 0.0)), bits(ta));
        prop_assert_eq!(bits(ta.interpolate(&tb, 1.0)), bits(tb));
        prop_assert_eq!(bits(tb.interpolate(&ta, 0.0)), bits(tb));
        prop_assert_eq!(bits(tb.interpolate(&ta, 1.0)), bits(ta));
    }

    #[test]
    fn arq_19_transform_equal_endpoints_stay_put_within_tolerance(
        a in matrix(), t in -2.0_f32..=3.0,
    ) {
        prop_assume!(det(a).abs() > 1e-3);
        let r = tf(a).interpolate(&tf(a), t);
        let tol_ok = r.coeffs().iter().zip(a).all(|(g, w)| (g - w).abs() <= 1e-9 * w.abs().max(1.0));
        prop_assert!(tol_ok, "{:?} vs {:?}", r.coeffs(), a);
    }

    #[test]
    fn arq_19_transform_rotations_follow_the_short_arc(
        alpha in -3.0_f64..=3.0, beta in -3.0_f64..=3.0, t in -1.0_f32..=2.0,
    ) {
        // Reference angle by the shortest signed difference, wrapped by hand.
        let mut d = beta - alpha;
        if d > std::f64::consts::PI { d -= std::f64::consts::TAU; }
        if d < -std::f64::consts::PI { d += std::f64::consts::TAU; }
        let want = Transform2D::rotation(alpha + d * f64::from(t)).coeffs();
        let got = Transform2D::rotation(alpha).interpolate(&Transform2D::rotation(beta), t);
        let ok = got.coeffs().iter().zip(want).all(|(g, w)| (g - w).abs() <= 1e-9 * w.abs().max(1.0));
        prop_assert!(ok, "{:?} vs {:?}", got.coeffs(), want);
    }

    #[test]
    fn arq_19_transform_degenerate_pairs_are_the_linear_blend(
        r in prop::array::uniform4(-3_i32..=3), m in -3_i32..=3, n in -3_i32..=3,
        tr in prop::array::uniform4(-3.0_f64..=3.0), t in -1.0_f32..=2.0,
    ) {
        // Integer rows: second row is an exact multiple of the first, det == 0.
        let a = [f64::from(r[0]), f64::from(r[1]), f64::from(r[0] * m), f64::from(r[1] * m), tr[0], tr[1]];
        let b = [f64::from(r[2]), f64::from(r[3]), f64::from(r[2] * n), f64::from(r[3] * n), tr[2], tr[3]];
        let got = tf(a).interpolate(&tf(b), t).coeffs();
        let want = lerp6(a, b, t);
        let ok = got.iter().zip(want).all(|(g, w)| (g - w).abs() <= 1e-9 * w.abs().max(1.0));
        prop_assert!(ok, "{:?} vs {:?}", got, want);
    }
}

fn linear_only(c: [f64; 4]) -> Transform2D {
    tf(linear(c, 0.0, 0.0))
}

/// ARQ-19 Q-33 tolerance: |got - want| <= 1e-9 * max(1, |want|) per coefficient.
fn assert_abs_rel(got: Transform2D, want: [f64; 6]) {
    let got = got.coeffs();
    for i in 0..6 {
        let tol = 1e-9 * want[i].abs().max(1.0);
        assert!(
            got[i].is_finite() && (got[i] - want[i]).abs() <= tol,
            "coeff {i}: {got:?} vs {want:?}"
        );
    }
}

// ARQ-19, Q-33
#[test]
fn arq_19_q33_rank_one_end_with_rounding_determinant_is_degenerate() {
    let id = Transform2D::IDENTITY;
    let rounded = linear_only([0.1, 0.3, 0.7, 2.1]);
    assert!(
        det(rounded.coeffs()) != 0.0,
        "premise: det is rounding noise"
    );
    let got = rounded.interpolate(&id, 0.5);
    assert_abs_rel(got, [0.55, 0.0, 0.0, 1.55, 0.0, 0.0]);
    let exact = linear_only([1.1, 2.2, 3.3, 6.6]);
    assert_abs_rel(exact.interpolate(&id, 0.5), [1.05, 0.0, 0.0, 3.8, 0.0, 0.0]);
}

// ARQ-19, Q-33
#[test]
fn arq_19_q33_finite_coefficients_with_non_finite_determinant_are_degenerate() {
    let big = linear_only([1e200, 1e200, 1e200, 2e200]);
    let got = big.interpolate(&Transform2D::IDENTITY, 0.5);
    assert_abs_rel(got, [5e199, 0.0, 0.0, 1e200, 0.0, 0.0]);
    assert!(got.is_finite());
}

// ARQ-19, Q-33: a huge end with sine 1e-13 between columns (det 0.2) is
// degenerate; an absolute threshold would decompose it with a shear near 1e13.
#[test]
fn arq_19_q33_large_scale_with_tiny_column_sine_is_degenerate() {
    let (s, c) = 1e-13_f64.sin_cos();
    let big = linear_only([1e6, 0.0, 2e6 * c, 2e6 * s]);
    let got = big.interpolate(&Transform2D::IDENTITY, 0.5);
    assert_abs_rel(got, [500000.5, 0.0, 0.0, (1.0 + 2e-7) / 2.0, 0.0, 0.0]);
}

// ARQ-19, Q-33: a tiny end with columns at a right angle (det 1e-14) is not
// degenerate; an absolute threshold would borrow rotation(2.0) instead.
#[test]
fn arq_19_q33_tiny_rotated_scale_keeps_its_own_rotation() {
    let rot = |th: f64| {
        let (s, c) = th.sin_cos();
        [c, s, -s, c]
    };
    let a = rot(0.3).map(|v| 1e-7 * v);
    let got = linear_only(a).interpolate(&linear_only(rot(2.0)), 0.5);
    let w = rot(1.15).map(|v| 0.50000005 * v);
    assert_abs_rel(got, [w[0], w[1], w[2], w[3], 0.0, 0.0]);
}

// ARQ-19, Q-33: the threshold is the sine of the angle between the columns.
#[test]
fn arq_19_q33_threshold_is_relative_to_the_column_lengths() {
    // sin(angle) = 1e-13 (below 1e-12): degenerate, borrows the identity frame.
    let below = linear_only([1.0, 0.0, 1.0, 1e-13]);
    assert_abs_rel(
        below.interpolate(&Transform2D::IDENTITY, 0.5),
        [1.0, 0.0, 0.0, 0.5 * (1e-13 + 1.0), 0.0, 0.0],
    );
    // sin(angle) = 1e-11 (above 1e-12): decomposed, sy = 1e-11, k = 1e11.
    let above = linear_only([1.0, 0.0, 1.0, 1e-11]);
    let c = above.interpolate(&Transform2D::IDENTITY, 0.5).coeffs();
    assert!(c[2].abs() > 1e3, "decomposed with a large shear: {c:?}");
}

// ARQ-19, S1
#[test]
fn arq_19_transform_non_finite_endpoint_never_panics() {
    let ok = Transform2D::IDENTITY;
    for i in 0..6 {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut c = [1.0, 0.0, 0.0, 1.0, 5.0, 6.0];
            c[i] = bad;
            for t in [0.0_f32, 0.25, 0.5, 1.0, 1.5] {
                let _ = tf(c).interpolate(&ok, t);
                let _ = ok.interpolate(&tf(c), t);
                let _ = tf(c).interpolate(&tf(c), t);
            }
        }
    }
}

// ARQ-19, Q-33: an end with a non-finite coefficient (outside the contracted
// domain, ARQ-18 makes such a track invalid) is treated as degenerate and,
// whatever the other end is, the six coefficients are blended linearly; it is
// never decomposed nor does it lend or borrow a frame. Only the non-finite
// coefficient itself may come out non-finite.
#[test]
fn arq_19_transform_end_with_a_non_finite_coefficient_blends_the_six_coefficients_linearly() {
    let (theta, k) = (2.0_f64, 0.8_f64);
    let (s, c) = theta.sin_cos();
    let rotated = tf([
        2.0 * c,
        2.0 * s,
        3.0 * (k * c - s),
        3.0 * (k * s + c),
        1.0,
        -2.0,
    ]);
    let t = 0.5_f32;
    for bad in [f64::NAN, f64::INFINITY] {
        // Non-finite translation with a perfectly decomposable linear part.
        let broken = tf([1.0, 0.0, 0.0, 1.0, bad, 4.0]);
        let first = broken.interpolate(&rotated, t).coeffs();
        let last = rotated.interpolate(&broken, t).coeffs();
        let want_first = lerp6(broken.coeffs(), rotated.coeffs(), t);
        let want_last = lerp6(rotated.coeffs(), broken.coeffs(), t);
        for i in [0, 1, 2, 3, 5] {
            assert!(
                (first[i] - want_first[i]).abs() <= 1e-9 * want_first[i].abs().max(1.0),
                "first, coeff {i} (bad = {bad}): {first:?} vs {want_first:?}"
            );
            assert!(
                (last[i] - want_last[i]).abs() <= 1e-9 * want_last[i].abs().max(1.0),
                "last, coeff {i} (bad = {bad}): {last:?} vs {want_last:?}"
            );
        }
    }
}

// ARQ-19, C1: "un volteo es siempre sy < 0": sx stays positive and sy crosses 0.
#[test]
fn arq_19_transform_flip_from_identity_collapses_the_y_axis_at_the_middle() {
    let id = Transform2D::IDENTITY;
    let flip = Transform2D::scale_xy(1.0, -1.0);
    assert_close(id.interpolate(&flip, 0.5), [1.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
    assert_close(flip.interpolate(&id, 0.5), [1.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
    assert_close(id.interpolate(&flip, 0.25), [1.0, 0.0, 0.0, 0.5, 0.0, 0.0]);
    assert_close(id.interpolate(&flip, 1.5), [1.0, 0.0, 0.0, -2.0, 0.0, 0.0]);
}

// ARQ-19, C1: scale(-1, 1) has det < 0 too: sx = 1, theta = pi, sy = -1.
#[test]
fn arq_19_transform_horizontal_flip_from_identity_goes_through_a_half_turn_frame() {
    let id = Transform2D::IDENTITY;
    let got = id
        .interpolate(&Transform2D::scale_xy(-1.0, 1.0), 0.5)
        .coeffs();
    // theta = pi/2, sx = 1, sy = 0: [0, 1, 0, 0] (or its mirror at exactly pi).
    assert!(got[0].abs() < 1e-9 && got[2].abs() < 1e-9 && got[3].abs() < 1e-9);
    assert!((got[1].abs() - 1.0).abs() < 1e-9, "{got:?}");
}

// ARQ-19, C1: rotated mixed-sign pair.
#[test]
fn arq_19_transform_rotated_flip_keeps_the_rotation_and_collapses_the_y_axis() {
    let r = Transform2D::rotation(0.5);
    let (s, c) = 0.5_f64.sin_cos();
    // r * scale(1, -1): second column negated.
    let flipped = tf([c, s, s, -c, 0.0, 0.0]);
    assert!(det(flipped.coeffs()) < 0.0);
    assert_close(r.interpolate(&flipped, 0.5), [c, s, 0.0, 0.0, 0.0, 0.0]);
    assert_close(flipped.interpolate(&r, 0.5), [c, s, 0.0, 0.0, 0.0, 0.0]);
}

/// Independent reference decomposition `(sx, sy, k, theta)` for det != 0.
fn reference_parts(c: [f64; 6]) -> (f64, f64, f64, f64) {
    let (c1, c2) = ((c[0], c[1]), (c[2], c[3]));
    let sx = c1.0.hypot(c1.1);
    let sy = det(c) / sx;
    let k = (c1.0 * c2.0 + c1.1 * c2.1) / (sx * sy);
    (sx, sy, k, c1.1.atan2(c1.0))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    // ARQ-19, C1: opposite-sign determinants blend sx > 0 and sy through 0.
    #[test]
    fn arq_19_transform_opposite_sign_pairs_match_the_decomposition_reference(
        a in matrix(), b in matrix(), t in t01(),
    ) {
        prop_assume!(det(a).abs() > 1e-3 && det(b).abs() > 1e-3);
        let b = if det(a).signum() != det(b).signum() { b } else { [b[2], b[3], b[0], b[1], b[4], b[5]] };
        prop_assert!(det(a) * det(b) < 0.0);
        let (pa, pb) = (reference_parts(a), reference_parts(b));
        let mut dth = pb.3 - pa.3;
        if dth > std::f64::consts::PI { dth -= std::f64::consts::TAU; }
        if dth < -std::f64::consts::PI { dth += std::f64::consts::TAU; }
        // Skip the tie at exactly half a turn, where the short way is ambiguous.
        prop_assume!(dth.abs() < std::f64::consts::PI - 1e-6);
        let t64 = f64::from(t);
        let (sx, sy, k) = (lerp(pa.0, pb.0, t64), lerp(pa.1, pb.1, t64), lerp(pa.2, pb.2, t64));
        let (sn, cs) = (pa.3 + dth * t64).sin_cos();
        let want = [sx * cs, sx * sn, sy * (k * cs - sn), sy * (k * sn + cs), lerp(a[4], b[4], t64), lerp(a[5], b[5], t64)];
        let got = tf(a).interpolate(&tf(b), t).coeffs();
        let ok = got.iter().zip(want).all(|(g, w)| (g - w).abs() <= 1e-9 * w.abs().max(1.0));
        prop_assert!(ok, "{:?} vs {:?}", got, want);
    }
}

// ARQ-19, Q-28 B: a degenerate end with every linear coefficient non-zero
// (columns (2, 1) and (4, 2), det 0) borrows both scales from a frame with a
// non-trivial rotation AND a non-zero shear; every term of the borrowed
// sx = a*(k*s+co) - b*(k*co-s) and sy = d*co - c*s is then non-zero.
#[test]
fn arq_19_transform_degenerate_end_with_full_linear_part_borrows_both_scales() {
    let (theta, k, bsx, bsy) = (0.7_f64, 0.8_f64, 2.0_f64, 3.0_f64);
    let (s, c) = theta.sin_cos();
    let b = tf([
        bsx * c,
        bsx * s,
        bsy * (k * c - s),
        bsy * (k * s + c),
        1.0,
        -2.0,
    ]);
    let a = tf([2.0, 1.0, 4.0, 2.0, 3.0, 4.0]);
    // Independent expectation (python3 f64): S = U^-1 R^T L_a, with
    // S11 = 2.5927766115593833 and S22 = -1.047186374381787; at t = 0.5 the
    // scales are (S11 + bsx) / 2 and (S22 + bsy) / 2, the shear and angle are B's.
    let (s11, s22) = (2.592_776_611_559_383_3_f64, -1.047_186_374_381_787_f64);
    let want = [
        1.756_374_654_647_047,
        1.479_373_963_349_072_6,
        -0.031_580_840_840_290_18,
        1.250_011_953_389_199_2,
        2.0,
        1.0,
    ];
    assert_close(a.interpolate(&b, 0.5), want);
    // Degenerate end last: the same scales, blended from B's.
    let (sx, sy) = (0.5 * (bsx + s11), 0.5 * (bsy + s22));
    assert_close(
        b.interpolate(&a, 0.5),
        [sx * c, sx * s, sy * (k * c - s), sy * (k * s + c), 2.0, 1.0],
    );
}

// ARQ-19, Q-28 B, S3: a degenerate end whose off-diagonal part does not fit
// the borrowed (theta, k) frame of a rotated and sheared B.
#[test]
fn arq_19_transform_degenerate_end_uses_the_diagonal_in_a_rotated_sheared_frame() {
    let (theta, k, bsx, bsy) = (0.7_f64, 0.8_f64, 2.0_f64, 3.0_f64);
    let (s, c) = theta.sin_cos();
    let b = tf([
        bsx * c,
        bsx * s,
        bsy * (k * c - s),
        bsy * (k * s + c),
        1.0,
        -2.0,
    ]);
    let a = tf([1.0, 0.0, 0.0, 0.0, 3.0, 4.0]); // scaleY(0), det 0
    // Independent expectation: S = U^-1 R^T L_a, diagonal (S11, S22).
    // R^T L_a with L_a = [[1, 0], [0, 0]] (columns (1,0) and (0,0)).
    let rt_l = [[c, 0.0], [-s, 0.0]];
    let s11 = rt_l[0][0] - k * rt_l[1][0];
    let s22 = rt_l[1][1];
    let t = 0.5;
    let sx = lerp(s11, bsx, t);
    let sy = lerp(s22, bsy, t);
    let want = [
        sx * c,
        sx * s,
        sy * (k * c - s),
        sy * (k * s + c),
        lerp(3.0, 1.0, t),
        lerp(4.0, -2.0, t),
    ];
    let got = a.interpolate(&b, 0.5);
    assert_close(got, want);
    // A plain 6-coefficient blend would give a different linear part.
    let plain = lerp6(a.coeffs(), b.coeffs(), 0.5);
    let far = got
        .coeffs()
        .iter()
        .zip(plain)
        .any(|(g, p)| (g - p).abs() > 1e-2);
    assert!(far, "borrowing must differ from the plain blend: {plain:?}");
    // Mirrored: the degenerate end last, blend towards it.
    assert_close(
        b.interpolate(&a, 0.5),
        [
            lerp(bsx, s11, t) * c,
            lerp(bsx, s11, t) * s,
            lerp(bsy, s22, t) * (k * c - s),
            lerp(bsy, s22, t) * (k * s + c),
            lerp(1.0, 3.0, t),
            lerp(-2.0, 4.0, t),
        ],
    );
}

// ARQ-19, Q-33: |c1| = 1e6, |c2| = 1e-6, column sine 1e-3 (|det| = 1e-3, far
// above 1e-12·|c1|·|c2|): decomposed, so the shear survives; the degenerate
// path would zero c (index 2).
#[test]
fn arq_19_q33_unequal_columns_with_sine_1e_3_are_decomposed_in_both_orders() {
    let id = Transform2D::IDENTITY;
    let x = 1e-6 * (1.0_f64 - 1e-6).sqrt();
    // columns (1e6, 0) and (x, 1e-9): theta 0, sx 1e6, sy 1e-9, k = x / 1e-9.
    let a = linear_only([1e6, 0.0, x, 1e-9]);
    let sx = 0.5 * (1e6 + 1.0);
    let sy = 0.5 * (1e-9 + 1.0);
    let k = 0.5 * (x / 1e-9);
    let want = [sx, 0.0, sy * k, sy, 0.0, 0.0];
    assert!(want[2] > 100.0, "premise: decomposed result has a shear");
    assert_abs_rel(a.interpolate(&id, 0.5), want);
    assert_abs_rel(id.interpolate(&a, 0.5), want);

    // swapped columns (x, 1e-9) and (1e6, 0): flip carried by sy = det / sx,
    // theta = atan2(1e-9, x), k = (c1 . c2) / det.
    let b = linear_only([x, 1e-9, 1e6, 0.0]);
    let sx0 = x.hypot(1e-9);
    let th0 = 1e-9_f64.atan2(x);
    let sy0 = (x * 0.0 - 1e-9 * 1e6) / sx0;
    let k0 = (x * 1e6) / (sx0 * sy0);
    let (th, sx, sy, k) = (0.5 * th0, 0.5 * (sx0 + 1.0), 0.5 * (sy0 + 1.0), 0.5 * k0);
    let (s, c) = th.sin_cos();
    let want = [sx * c, sx * s, sy * (k * c - s), sy * (k * s + c), 0.0, 0.0];
    assert!(
        want[2].abs() > 1e5,
        "premise: decomposed result has a shear"
    );
    assert_abs_rel(b.interpolate(&id, 0.5), want);
    assert_abs_rel(id.interpolate(&b, 0.5), want);
}

// ARQ-19, Q-33: |det| == 1e-12·|c1|·|c2| exactly in f64 is degenerate (<=).
#[test]
fn arq_19_q33_determinant_exactly_at_the_threshold_is_degenerate() {
    let e = linear_only([1.0, 0.0, 1.0, 1e-12]);
    assert_eq!(
        det(e.coeffs()).abs(),
        1e-12 * (1.0_f64.hypot(0.0) * 1.0_f64.hypot(1e-12)),
        "premise: exact boundary"
    );
    let want = [1.0, 0.0, 0.0, (1.0 + 1e-12) / 2.0, 0.0, 0.0];
    assert_abs_rel(e.interpolate(&Transform2D::IDENTITY, 0.5), want);
    assert_abs_rel(Transform2D::IDENTITY.interpolate(&e, 0.5), want);
    // exact zero shear, which the decomposed path (k = 1e12 / 2) never gives
    assert_eq!(e.interpolate(&Transform2D::IDENTITY, 0.5).coeffs()[2], 0.0);
}

// ARQ-19, Q-33: the tiny-rotation case, reverse direction.
#[test]
fn arq_19_q33_tiny_rotated_scale_keeps_its_own_rotation_reversed() {
    let rot = |th: f64| {
        let (s, c) = th.sin_cos();
        [c, s, -s, c]
    };
    let tiny = rot(0.3).map(|v| 1e-7 * v);
    let got = linear_only(rot(2.0)).interpolate(&linear_only(tiny), 0.5);
    let w = rot(1.15).map(|v| 0.50000005 * v);
    assert_abs_rel(got, [w[0], w[1], w[2], w[3], 0.0, 0.0]);
}

// ARQ-19, S8: extrapolation with exactly one degenerate end (sine 1e-13, huge):
// the degenerate end borrows the identity frame, parts extrapolate linearly.
#[test]
fn arq_19_s8_extrapolation_with_one_degenerate_end() {
    let (s, c) = 1e-13_f64.sin_cos();
    let big = linear_only([1e6, 0.0, 2e6 * c, 2e6 * s]);
    let sy0 = 1e6 * (2e6 * s) / 1e6;
    let id = Transform2D::IDENTITY;
    for t in [-0.5_f32, 1.5, 2.0] {
        let tt = f64::from(t);
        assert_abs_rel(
            big.interpolate(&id, t),
            [lerp(1e6, 1.0, tt), 0.0, 0.0, lerp(sy0, 1.0, tt), 0.0, 0.0],
        );
        assert_abs_rel(
            id.interpolate(&big, t),
            [lerp(1.0, 1e6, tt), 0.0, 0.0, lerp(1.0, sy0, tt), 0.0, 0.0],
        );
    }
}
