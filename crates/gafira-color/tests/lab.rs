//! Lab conversion and color differences seen from outside the crate (ARQ-122).

use gafira_color::{Lab, LinearRgba, delta_e76, delta_e2000};

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

const fn lab(l: f64, a: f64, b: f64) -> Lab {
    Lab { l, a, b }
}

// ARQ-122 AC4
#[test]
fn arq_122_delta_e76_of_the_published_pair_is_five() {
    let d = delta_e76(lab(50.0, 0.0, 0.0), lab(53.0, 4.0, 0.0));
    assert!((d - 5.0).abs() <= 1e-12, "{d}");
}

// ARQ-122: every axis counts, and the result is a distance, not a squared one.
#[test]
fn arq_122_delta_e76_uses_all_three_axes_and_the_square_root() {
    let cases = [
        (lab(10.0, 20.0, 30.0), lab(11.0, 22.0, 32.0), 3.0),
        (lab(0.0, 0.0, 0.0), lab(0.0, 0.0, 7.0), 7.0),
        (lab(0.0, 0.0, 0.0), lab(0.0, -7.0, 0.0), 7.0),
        (lab(9.0, 0.0, 0.0), lab(0.0, 0.0, 0.0), 9.0),
        (lab(5.0, 5.0, 5.0), lab(5.0, 5.0, 5.0), 0.0),
    ];
    for (a, b, expected) in cases {
        for d in [delta_e76(a, b), delta_e76(b, a)] {
            assert!((d - expected).abs() <= 1e-12, "{a:?} {b:?}: {d}");
        }
    }
}

/// The 34 pairs of Sharma, Wu and Dalal, Color Res. Appl. 30(1), 2005, table I, as
/// published by the authors (`ciede2000testdata.txt`): `L1 a1 b1 L2 a2 b2 dE`.
const SHARMA: [[f64; 7]; 34] = [
    [50.0000, 2.6772, -79.7751, 50.0000, 0.0000, -82.7485, 2.0425],
    [50.0000, 3.1571, -77.2803, 50.0000, 0.0000, -82.7485, 2.8615],
    [50.0000, 2.8361, -74.0200, 50.0000, 0.0000, -82.7485, 3.4412],
    [
        50.0000, -1.3802, -84.2814, 50.0000, 0.0000, -82.7485, 1.0000,
    ],
    [
        50.0000, -1.1848, -84.8006, 50.0000, 0.0000, -82.7485, 1.0000,
    ],
    [
        50.0000, -0.9009, -85.5211, 50.0000, 0.0000, -82.7485, 1.0000,
    ],
    [50.0000, 0.0000, 0.0000, 50.0000, -1.0000, 2.0000, 2.3669],
    [50.0000, -1.0000, 2.0000, 50.0000, 0.0000, 0.0000, 2.3669],
    [50.0000, 2.4900, -0.0010, 50.0000, -2.4900, 0.0009, 7.1792],
    [50.0000, 2.4900, -0.0010, 50.0000, -2.4900, 0.0010, 7.1792],
    [50.0000, 2.4900, -0.0010, 50.0000, -2.4900, 0.0011, 7.2195],
    [50.0000, 2.4900, -0.0010, 50.0000, -2.4900, 0.0012, 7.2195],
    [50.0000, -0.0010, 2.4900, 50.0000, 0.0009, -2.4900, 4.8045],
    [50.0000, -0.0010, 2.4900, 50.0000, 0.0010, -2.4900, 4.8045],
    [50.0000, -0.0010, 2.4900, 50.0000, 0.0011, -2.4900, 4.7461],
    [50.0000, 2.5000, 0.0000, 50.0000, 0.0000, -2.5000, 4.3065],
    [50.0000, 2.5000, 0.0000, 73.0000, 25.0000, -18.0000, 27.1492],
    [50.0000, 2.5000, 0.0000, 61.0000, -5.0000, 29.0000, 22.8977],
    [50.0000, 2.5000, 0.0000, 56.0000, -27.0000, -3.0000, 31.9030],
    [50.0000, 2.5000, 0.0000, 58.0000, 24.0000, 15.0000, 19.4535],
    [50.0000, 2.5000, 0.0000, 50.0000, 3.1736, 0.5854, 1.0000],
    [50.0000, 2.5000, 0.0000, 50.0000, 3.2972, 0.0000, 1.0000],
    [50.0000, 2.5000, 0.0000, 50.0000, 1.8634, 0.5757, 1.0000],
    [50.0000, 2.5000, 0.0000, 50.0000, 3.2592, 0.3350, 1.0000],
    [
        60.2574, -34.0099, 36.2677, 60.4626, -34.1751, 39.4387, 1.2644,
    ],
    [
        63.0109, -31.0961, -5.8663, 62.8187, -29.7946, -4.0864, 1.2630,
    ],
    [61.2901, 3.7196, -5.3901, 61.4292, 2.2480, -4.9620, 1.8731],
    [35.0831, -44.1164, 3.7933, 35.0232, -40.0716, 1.5901, 1.8645],
    [
        22.7233, 20.0904, -46.6940, 23.0331, 14.9730, -42.5619, 2.0373,
    ],
    [36.4612, 47.8580, 18.3852, 36.2715, 50.5065, 21.2231, 1.4146],
    [90.8027, -2.0831, 1.4410, 91.1528, -1.6435, 0.0447, 1.4441],
    [90.9257, -0.5406, -0.9208, 88.6381, -0.8985, -0.7239, 1.5381],
    [6.7747, -0.2908, -2.4247, 5.8714, -0.0985, -2.2286, 0.6377],
    [2.0776, 0.0795, -1.1350, 0.9033, -0.0636, -0.5514, 0.9082],
];

// ARQ-122 AC1
#[test]
fn arq_122_delta_e2000_matches_the_34_sharma_pairs() {
    for (i, row) in SHARMA.iter().enumerate() {
        let d = delta_e2000(lab(row[0], row[1], row[2]), lab(row[3], row[4], row[5]));
        assert!(
            (d - row[6]).abs() <= 1e-4,
            "pair {}: got {d}, published {}",
            i + 1,
            row[6]
        );
    }
}

// ARQ-122: CIEDE2000 does not depend on argument order.
#[test]
fn arq_122_delta_e2000_is_symmetric() {
    for (i, row) in SHARMA.iter().enumerate() {
        let (p, q) = (lab(row[0], row[1], row[2]), lab(row[3], row[4], row[5]));
        let (forward, backward) = (delta_e2000(p, q), delta_e2000(q, p));
        assert!(
            (forward - backward).abs() <= 1e-10,
            "pair {}: {forward} vs {backward}",
            i + 1
        );
    }
}
