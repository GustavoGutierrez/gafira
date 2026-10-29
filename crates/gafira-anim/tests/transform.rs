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
