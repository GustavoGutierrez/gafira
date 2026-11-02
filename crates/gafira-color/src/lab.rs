//! CIE L*a*b* (D50) and the two color-difference formulas built on it.

use crate::LinearRgba;
use crate::math::{Mat3, SRGB_TO_XYZ, mul, mul_vec};

/// D50 white as XYZ with `Y = 1`, from the chromaticity `(0.3457, 0.3585)`.
const D50_WHITE: [f64; 3] = [0.3457 / 0.3585, 1.0, (1.0 - 0.3457 - 0.3585) / 0.3585];

/// Bradford chromatic adaptation, XYZ D65 to XYZ D50.
///
/// Derived as `M^-1 * diag(M*w50 / M*w65) * M` with the Bradford cone matrix `M`
/// and the D65 and D50 whites of CSS Color 4; it equals the matrix of its section 19.
const D65_TO_D50: Mat3 = [
    [
        1.047_929_792_544_996_6,
        0.022_946_870_601_609_614,
        -0.050_192_266_289_205_194,
    ],
    [
        0.029_627_808_770_055_8,
        0.990_434_426_753_879_8,
        -0.017_073_799_063_418_806,
    ],
    [
        -0.009_243_040_646_204_514,
        0.015_055_191_490_298_15,
        0.751_874_281_428_137_1,
    ],
];

/// Linear sRGB to XYZ D50.
const SRGB_TO_XYZ_D50: Mat3 = mul(&D65_TO_D50, &SRGB_TO_XYZ);

/// CIE epsilon, `216/24389`.
const EPSILON: f64 = 216.0 / 24389.0;
/// CIE kappa, `24389/27`.
const KAPPA: f64 = 24389.0 / 27.0;

/// The CIE piecewise function applied to a white-relative XYZ component.
fn f(t: f64) -> f64 {
    if t > EPSILON {
        t.cbrt()
    } else {
        (KAPPA * t + 16.0) / 116.0
    }
}

/// A CIE L*a*b* color relative to the D50 white.
///
/// A plain value: nothing is validated, and non-finite input gives
/// unspecified output (ARQ-122).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lab {
    /// Lightness, 0 (black) to 100 (white).
    pub l: f64,
    /// Green-red axis.
    pub a: f64,
    /// Blue-yellow axis.
    pub b: f64,
}

impl Lab {
    /// Converts linear sRGB to Lab through XYZ D50 (Bradford), as CSS Color 4 §19.
    /// Alpha is ignored.
    #[must_use]
    pub fn from_linear_srgb(c: LinearRgba) -> Lab {
        let xyz = mul_vec(
            &SRGB_TO_XYZ_D50,
            [f64::from(c.r), f64::from(c.g), f64::from(c.b)],
        );
        let [fx, fy, fz] = [0, 1, 2].map(|i| f(xyz[i] / D50_WHITE[i]));
        Lab {
            l: 116.0 * fy - 16.0,
            a: 500.0 * (fx - fy),
            b: 200.0 * (fy - fz),
        }
    }
}

/// CIE76 color difference: the Euclidean distance in Lab (CSS Color 4 §20.1).
#[must_use]
pub fn delta_e76(a: Lab, b: Lab) -> f64 {
    ((a.l - b.l).powi(2) + (a.a - b.a).powi(2) + (a.b - b.b).powi(2)).sqrt()
}
