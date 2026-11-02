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

/// `25^7`, the constant of the `G` and `R_C` terms.
const POW25_7: f64 = 6_103_515_625.0;

/// Hue angle in degrees, `[0, 360)`; 0 for the achromatic point.
fn hue_degrees(b: f64, a_prime: f64) -> f64 {
    if a_prime == 0.0 && b == 0.0 {
        return 0.0;
    }
    let h = b.atan2(a_prime).to_degrees();
    if h < 0.0 { h + 360.0 } else { h }
}

/// CIEDE2000 color difference with `kL = kC = kH = 1`
/// (Sharma, Wu and Dalal, 2005).
#[must_use]
pub fn delta_e2000(a: Lab, b: Lab) -> f64 {
    let c_bar = (a.a.hypot(a.b) + b.a.hypot(b.b)) / 2.0;
    let c_bar7 = c_bar.powi(7);
    let g = 0.5 * (1.0 - (c_bar7 / (c_bar7 + POW25_7)).sqrt());
    let (a1, a2) = ((1.0 + g) * a.a, (1.0 + g) * b.a);
    let (c1, c2) = (a1.hypot(a.b), a2.hypot(b.b));
    let (h1, h2) = (hue_degrees(a.b, a1), hue_degrees(b.b, a2));

    let dl = b.l - a.l;
    let dc = c2 - c1;
    let chromatic = c1 * c2 != 0.0;
    let mut dh = 0.0;
    if chromatic {
        dh = h2 - h1;
        if dh > 180.0 {
            dh -= 360.0;
        } else if dh < -180.0 {
            dh += 360.0;
        }
    }
    let d_big_h = 2.0 * (c1 * c2).sqrt() * (dh.to_radians() / 2.0).sin();

    let l_bar = (a.l + b.l) / 2.0;
    let c_bar_p = (c1 + c2) / 2.0;
    let h_bar = if !chromatic {
        h1 + h2
    } else if (h1 - h2).abs() <= 180.0 {
        (h1 + h2) / 2.0
    } else if h1 + h2 < 360.0 {
        (h1 + h2 + 360.0) / 2.0
    } else {
        (h1 + h2 - 360.0) / 2.0
    };

    let t = 1.0 - 0.17 * (h_bar - 30.0).to_radians().cos()
        + 0.24 * (2.0 * h_bar).to_radians().cos()
        + 0.32 * (3.0 * h_bar + 6.0).to_radians().cos()
        - 0.20 * (4.0 * h_bar - 63.0).to_radians().cos();
    let d_theta = 30.0 * (-((h_bar - 275.0) / 25.0).powi(2)).exp();
    let c_bar_p7 = c_bar_p.powi(7);
    let r_c = 2.0 * (c_bar_p7 / (c_bar_p7 + POW25_7)).sqrt();
    let l50 = (l_bar - 50.0).powi(2);
    let s_l = 1.0 + 0.015 * l50 / (20.0 + l50).sqrt();
    let s_c = 1.0 + 0.045 * c_bar_p;
    let s_h = 1.0 + 0.015 * c_bar_p * t;
    let r_t = -(2.0 * d_theta).to_radians().sin() * r_c;

    let (tl, tc, th) = (dl / s_l, dc / s_c, d_big_h / s_h);
    (tl * tl + tc * tc + th * th + r_t * tc * th).sqrt()
}
