//! Curved text (FD-44).

use gafira_geom::Size;

use crate::layout::{check_input_with, finite, in_range};
use crate::{TextError, TextLayout, TextStyle, layout_box};

/// The arc a line of text follows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arc {
    /// Radius, in px.
    pub radius: f64,
    /// Centre of the arc.
    pub center: (f64, f64),
    /// Point where the line starts.
    pub start: (f64, f64),
    /// Point where the line ends.
    pub end: (f64, f64),
}

/// One cluster placed on the curve.
#[derive(Clone, Debug, PartialEq)]
pub struct CurvedCluster {
    /// X of the centre of its advance.
    pub x: f64,
    /// Y of the centre of its advance.
    pub y: f64,
    /// Rotation, in radians.
    pub rotation: f64,
}

/// One line of curved text.
#[derive(Clone, Debug, PartialEq)]
pub struct CurvedLine {
    /// The arc, none when the line is straight.
    pub arc: Option<Arc>,
    /// The clusters of the line.
    pub clusters: Vec<CurvedCluster>,
}

/// Degrees of arc for each unit of the bend (D-26): ±100 is ±180 degrees.
const DEGREES_PER_UNIT: f64 = 1.8;

/// Places the lines of a layout on arcs (FD-44).
///
/// `bend` is in `[-100, 100]`, ends included, and the arc covers `theta = bend x 1.8 degrees`:
/// positive opens downwards (an arch), negative upwards (a cup). `L` is the advance of a line
/// without the white space that hangs at its end ([`crate::Cluster::trailing`]), also when that space
/// is inside the line; `s` counts from the start of that content and does not include the
/// advance of the space, which has no place on the arc: it sits where the content before it
/// ends. A line of advance `L` follows an arc of length `L`, so the radius is `L / |theta|`. The
/// arc is centred in `width`. A cluster sits where the centre of its advance, at `s` from the
/// start of the line, is on the arc, and turns `(s / L - 0.5) x theta` radians (clockwise is
/// positive, y points down).
///
/// Lines are concentric and share `theta`. With `sigma` the sign of `theta`, `b_i` the baseline
/// of a line and `L_i > 0` its advance, the reference radius is
/// `R_ref = max_i(L_i / |theta| + sigma (b_i - b_ref))` over the lines with `L_i > 0` (any of
/// them gives `b_ref`), and each line has `R_i = R_ref - sigma (b_i - b_ref)`, so
/// `R_i |theta| >= L_i`, every radius is positive, and the centre is at `b_ref + sigma R_ref`.
/// The line that gives the maximum has `R_i = L_i / |theta|`; a single line is `L / |theta|`.
/// Each cluster turns by its own line's `s / L_i`. A bend of 0, or one so small that `theta` is
/// 0, computes no radius and leaves the text straight, as does a line with `L = 0` (empty, or
/// only white space that hangs). A negative `width` acts as 0. The result is checked: a
/// position that is not finite is an error.
pub fn curve_layout(
    layout: &TextLayout,
    width: f64,
    bend: f64,
) -> Result<Vec<CurvedLine>, TextError> {
    finite("curve.width", width)?;
    finite("curve.bend", bend)?;
    in_range("curve.bend", bend, (-100.0, 100.0))?;
    let center_x = width.max(0.0) / 2.0;
    let theta = bend * DEGREES_PER_UNIT.to_radians();
    let sigma = theta.signum();
    let content = |line: &crate::Line| {
        line.clusters
            .iter()
            .filter(|c| !c.trailing)
            .map(|c| f64::from(c.advance))
            .sum::<f64>()
    };
    // The first line with content gives the baseline the others are measured from; the maximum
    // does not depend on that choice.
    let ref_baseline = layout
        .lines
        .iter()
        .find(|l| content(l) > 0.0)
        .map_or(0.0, |l| f64::from(l.baseline));
    let ref_radius = layout
        .lines
        .iter()
        .filter(|l| content(l) > 0.0)
        .map(|l| content(l) / theta.abs() + sigma * (f64::from(l.baseline) - ref_baseline))
        .fold(f64::MIN, f64::max);
    let lines: Vec<CurvedLine> = layout
        .lines
        .iter()
        .map(|line| {
            let straight = || CurvedLine {
                arc: None,
                clusters: line
                    .clusters
                    .iter()
                    .map(|c| CurvedCluster {
                        x: f64::from(c.x) + f64::from(c.advance) / 2.0,
                        y: f64::from(line.baseline),
                        rotation: 0.0,
                    })
                    .collect(),
            };
            let length = content(line);
            if theta == 0.0 || length <= 0.0 {
                return straight();
            }
            let baseline = f64::from(line.baseline);
            let radius = ref_radius - sigma * (baseline - ref_baseline);
            let place = |alpha: f64| {
                (
                    center_x + sigma * radius * alpha.sin(),
                    baseline + sigma * 2.0 * radius * (alpha / 2.0).sin().powi(2),
                )
            };
            let mut before = 0.0;
            let clusters = line
                .clusters
                .iter()
                .map(|c| {
                    // The white space that hangs takes no room: it is where the content ends.
                    let advance = if c.trailing {
                        0.0
                    } else {
                        f64::from(c.advance)
                    };
                    let s = before + advance / 2.0;
                    before += advance;
                    let alpha = (s / length - 0.5) * theta;
                    let (x, y) = place(alpha);
                    CurvedCluster {
                        x,
                        y,
                        rotation: alpha,
                    }
                })
                .collect();
            CurvedLine {
                arc: Some(Arc {
                    radius,
                    center: (center_x, ref_baseline + sigma * ref_radius),
                    start: place(-theta / 2.0),
                    end: place(theta / 2.0),
                }),
                clusters,
            }
        })
        .collect();
    let finite_point =
        |p: &CurvedCluster| p.x.is_finite() && p.y.is_finite() && p.rotation.is_finite();
    let finite_arc = |a: &Option<Arc>| {
        a.is_none_or(|a| {
            [
                a.radius, a.center.0, a.center.1, a.start.0, a.start.1, a.end.0, a.end.1,
            ]
            .iter()
            .all(|v| v.is_finite())
        })
    };
    if lines
        .iter()
        .all(|l| finite_arc(&l.arc) && l.clusters.iter().all(finite_point))
    {
        Ok(lines)
    } else {
        Err(TextError::NonFinite { field: "output" })
    }
}

/// A layout and the curve of its lines.
#[derive(Clone, Debug, PartialEq)]
pub struct CurvedText {
    /// The straight layout.
    pub layout: TextLayout,
    /// The curved lines, one for each line of `layout`.
    pub lines: Vec<CurvedLine>,
}

/// Lays `text` out and curves it.
pub fn layout_curved(
    text: &str,
    fonts: &[&[u8]],
    style: &TextStyle,
    dpi: f64,
    bounds: Size,
    bend: f64,
) -> Result<CurvedText, TextError> {
    check_input_with(style, dpi, bounds, &[("curve.bend", bend)])?;
    in_range("curve.bend", bend, (-100.0, 100.0))?;
    let layout = layout_box(text, fonts, style, dpi, bounds)?;
    let lines = curve_layout(&layout, bounds.width(), bend)?;
    Ok(CurvedText { layout, lines })
}
