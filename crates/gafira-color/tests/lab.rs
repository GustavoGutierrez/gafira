//! Lab conversion and color differences seen from outside the crate (ARQ-122).

use gafira_color::{Lab, LinearRgba};

fn lab_of(r: f32, g: f32, b: f32) -> Lab {
    Lab::from_linear_srgb(LinearRgba::new(r, g, b, 1.0))
}

fn assert_lab(actual: Lab, expected: [f64; 3], tol: f64) {
    let got = [actual.l, actual.a, actual.b];
    for i in 0..3 {
        assert!(
            (got[i] - expected[i]).abs() <= tol,
            "axis {i}: {got:?} vs {expected:?} (tol {tol})"
        );
    }
}

// ARQ-122 AC2
#[test]
fn arq_122_white_is_l100_and_black_is_zero() {
    assert_lab(lab_of(1.0, 1.0, 1.0), [100.0, 0.0, 0.0], 1e-3);
}

// ARQ-122 AC2
#[test]
fn arq_122_black_is_zero() {
    assert_lab(lab_of(0.0, 0.0, 0.0), [0.0, 0.0, 0.0], 1e-3);
}

// ARQ-122 AC3: CSS Color 4 section 9.1 values, tolerance 0.02.
#[test]
fn arq_122_blue_and_yellow_match_the_published_values() {
    assert_lab(lab_of(0.0, 0.0, 1.0), [29.567, 68.298, -112.0294], 0.02);
    assert_lab(lab_of(1.0, 1.0, 0.0), [97.607, -15.753, 93.388], 0.02);
}

// ARQ-122: the linear branch of the CIE piecewise function (kappa), values from an
// independent f64 evaluation of the CSS Color 4 section 19 pipeline.
#[test]
fn arq_122_dark_colors_use_the_linear_branch() {
    assert_lab(lab_of(0.001, 0.001, 0.001), [0.903_296_3, 0.0, 0.0], 1e-6);
    assert_lab(
        lab_of(0.002, 0.0, 0.0),
        [0.401_954_5, 1.788_826_3, 0.640_461_5],
        1e-5,
    );
    assert_lab(
        lab_of(0.001, 0.004, 0.002),
        [2.900_738_1, -3.366_600_9, 1.546_252_3],
        1e-5,
    );
}

// ARQ-122: a mid-range color, cube-root branch on all three axes.
#[test]
fn arq_122_mid_range_color_matches_the_reference_pipeline() {
    assert_lab(
        lab_of(0.5, 0.25, 0.75),
        [64.638_473, 31.919_065, -37.438_607],
        1e-5,
    );
}
