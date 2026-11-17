//! SVG path data to flat contours (spec 037, 03 §13).
//!
//! Shared by `build.rs`, which turns the vendored Phosphor files into the
//! geometry table the binary embeds, and by the unit tests of `icons`. It
//! reads only what the icons use: one `<svg viewBox="0 0 256 256">` with
//! `<path d="…">` elements, every command of the SVG path grammar (absolute
//! and relative `M L H V C S Q T A Z`), curves and arcs flattened to line
//! segments within a tolerance in icon units.

use std::fmt;

/// The side of the Phosphor grid, in icon units.
pub(crate) const GRID: f32 = 256.0;

/// The largest distance between a curve and its flattened segments, in
/// icon units (0.5 of 256 is 0.04 px at 20 px).
pub(crate) const TOLERANCE: f32 = 0.5;

/// A closed polygon of the outline, in icon units.
pub(crate) type Contour = Vec<[f32; 2]>;

/// Why a file or a path could not be read.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PathError {
    /// The file is not a 256-unit Phosphor icon.
    ViewBox,
    /// The file draws with an element other than `<path>`.
    Element(String),
    /// The file has no `<path d="…">`.
    NoPath,
    /// A letter that is not a path command.
    Command(char),
    /// A command lacks numbers, or a number is malformed.
    Number(usize),
    /// Drawing before the first `M`.
    NoMoveTo,
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ViewBox => write!(f, "the viewBox is not 0 0 256 256"),
            Self::Element(name) => write!(f, "unsupported element <{name}>"),
            Self::NoPath => write!(f, "no <path d=\"…\">"),
            Self::Command(c) => write!(f, "unknown path command {c:?}"),
            Self::Number(at) => write!(f, "malformed or missing number at byte {at}"),
            Self::NoMoveTo => write!(f, "drawing before the first moveto"),
        }
    }
}

/// The contours of every `<path>` of an icon file.
pub(crate) fn svg_contours(svg: &str) -> Result<Vec<Contour>, PathError> {
    if !svg.contains("viewBox=\"0 0 256 256\"") {
        return Err(PathError::ViewBox);
    }
    for element in [
        "rect", "circle", "ellipse", "line", "polyline", "polygon", "g", "use",
    ] {
        if svg.contains(&format!("<{element} ")) || svg.contains(&format!("<{element}>")) {
            return Err(PathError::Element(element.to_owned()));
        }
    }
    let mut contours = Vec::new();
    let mut rest = svg;
    let mut found = false;
    while let Some(start) = rest.find("<path") {
        rest = &rest[start + 5..];
        let end = rest.find('>').unwrap_or(rest.len());
        let tag = &rest[..end];
        let Some(d) = tag.find(" d=\"") else {
            return Err(PathError::NoPath);
        };
        let data = &tag[d + 4..];
        let data = &data[..data.find('"').unwrap_or(data.len())];
        contours.extend(contours_of(data, TOLERANCE)?);
        found = true;
        rest = &rest[end..];
    }
    if found {
        Ok(contours)
    } else {
        Err(PathError::NoPath)
    }
}

/// The contours of one path's `d`, curves flattened within `tolerance`.
pub(crate) fn contours_of(d: &str, tolerance: f32) -> Result<Vec<Contour>, PathError> {
    let mut parser = Parser {
        bytes: d.as_bytes(),
        at: 0,
    };
    let mut out: Vec<Contour> = Vec::new();
    let mut current: Contour = Vec::new();
    let mut pen = [0.0_f32; 2];
    let mut start = [0.0_f32; 2];
    // The reflected control point of the previous C/S or Q/T.
    let mut last_cubic: Option<[f32; 2]> = None;
    let mut last_quad: Option<[f32; 2]> = None;
    let mut command: Option<char> = None;
    let mut started = false;
    loop {
        parser.skip_separators();
        let Some(&byte) = parser.bytes.get(parser.at) else {
            break;
        };
        let letter = if byte.is_ascii_alphabetic() {
            parser.at += 1;
            char::from(byte)
        } else {
            // A repeated command: after a moveto the pairs are linetos.
            match command {
                Some('M') => 'L',
                Some('m') => 'l',
                Some(c) if c != 'Z' && c != 'z' => c,
                _ => return Err(PathError::Number(parser.at)),
            }
        };
        let relative = letter.is_ascii_lowercase();
        let base = if relative { pen } else { [0.0, 0.0] };
        let upper = letter.to_ascii_uppercase();
        if upper != 'M' && !started {
            return Err(PathError::NoMoveTo);
        }
        let (cubic, quad) = match upper {
            'M' => {
                let p = parser.point(base)?;
                if current.len() > 1 {
                    out.push(std::mem::take(&mut current));
                }
                current.clear();
                current.push(p);
                pen = p;
                start = p;
                started = true;
                (None, None)
            }
            'L' => {
                let p = parser.point(base)?;
                current.push(p);
                pen = p;
                (None, None)
            }
            'H' => {
                let x = parser.number()? + base[0];
                pen = [x, pen[1]];
                current.push(pen);
                (None, None)
            }
            'V' => {
                let y = parser.number()? + base[1];
                pen = [pen[0], y];
                current.push(pen);
                (None, None)
            }
            'C' | 'S' => {
                let c1 = if upper == 'C' {
                    parser.point(base)?
                } else {
                    last_cubic.map_or(pen, |c| reflect(c, pen))
                };
                let c2 = parser.point(base)?;
                let p = parser.point(base)?;
                cubic_to(&mut current, pen, c1, c2, p, tolerance);
                pen = p;
                (Some(c2), None)
            }
            'Q' | 'T' => {
                let c = if upper == 'Q' {
                    parser.point(base)?
                } else {
                    last_quad.map_or(pen, |c| reflect(c, pen))
                };
                let p = parser.point(base)?;
                quad_to(&mut current, pen, c, p, tolerance);
                pen = p;
                (None, Some(c))
            }
            'A' => {
                let rx = parser.number()?;
                let ry = parser.number()?;
                let rotation = parser.number()?;
                let large = parser.flag()?;
                let sweep = parser.flag()?;
                let p = parser.point(base)?;
                let arc = Arc {
                    from: pen,
                    radii: [rx, ry],
                    rotation,
                    large,
                    sweep,
                    to: p,
                };
                arc.flatten(&mut current, tolerance);
                pen = p;
                (None, None)
            }
            'Z' => {
                if current.len() > 1 {
                    out.push(std::mem::take(&mut current));
                }
                current.clear();
                current.push(start);
                pen = start;
                (None, None)
            }
            _ => return Err(PathError::Command(letter)),
        };
        last_cubic = cubic;
        last_quad = quad;
        command = Some(letter);
    }
    if current.len() > 1 {
        out.push(current);
    }
    // A closing point equal to the first adds nothing to a polygon.
    for contour in &mut out {
        if contour.len() > 1 && contour.first() == contour.last() {
            contour.pop();
        }
    }
    out.retain(|c| c.len() >= 3);
    Ok(out)
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    fn skip_separators(&mut self) {
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| b.is_ascii_whitespace() || *b == b',')
        {
            self.at += 1;
        }
    }

    fn number(&mut self) -> Result<f32, PathError> {
        self.skip_separators();
        let begin = self.at;
        let mut end = begin;
        if matches!(self.bytes.get(end), Some(b'-' | b'+')) {
            end += 1;
        }
        let mut dot = false;
        let mut digits = false;
        while let Some(&b) = self.bytes.get(end) {
            if b.is_ascii_digit() {
                digits = true;
            } else if b == b'.' && !dot {
                dot = true;
            } else {
                break;
            }
            end += 1;
        }
        if digits && matches!(self.bytes.get(end), Some(b'e' | b'E')) {
            let mut exp = end + 1;
            if matches!(self.bytes.get(exp), Some(b'-' | b'+')) {
                exp += 1;
            }
            if self.bytes.get(exp).is_some_and(u8::is_ascii_digit) {
                end = exp;
                while self.bytes.get(end).is_some_and(u8::is_ascii_digit) {
                    end += 1;
                }
            }
        }
        if !digits {
            return Err(PathError::Number(begin));
        }
        let text =
            std::str::from_utf8(&self.bytes[begin..end]).map_err(|_| PathError::Number(begin))?;
        let value: f32 = text.parse().map_err(|_| PathError::Number(begin))?;
        self.at = end;
        Ok(value)
    }

    /// An arc flag: one `0` or `1`, which may touch the next number.
    fn flag(&mut self) -> Result<bool, PathError> {
        self.skip_separators();
        let value = match self.bytes.get(self.at) {
            Some(b'0') => false,
            Some(b'1') => true,
            _ => return Err(PathError::Number(self.at)),
        };
        self.at += 1;
        Ok(value)
    }

    fn point(&mut self, base: [f32; 2]) -> Result<[f32; 2], PathError> {
        let x = self.number()?;
        let y = self.number()?;
        Ok([base[0] + x, base[1] + y])
    }
}

fn reflect(control: [f32; 2], about: [f32; 2]) -> [f32; 2] {
    [2.0 * about[0] - control[0], 2.0 * about[1] - control[1]]
}

fn length(v: [f32; 2]) -> f32 {
    v[0].hypot(v[1])
}

/// Segments for a curve whose second differences reach `m` (Wang's formula
/// with `factor` = n(n-1)/8 for degree n).
fn segments(m: f32, factor: f32, tolerance: f32) -> u16 {
    let n = (factor * m / tolerance).sqrt().ceil();
    if n.is_finite() {
        // At most 64: the cast is exact for the clamped range.
        n.clamp(1.0, 64.0) as u16
    } else {
        1
    }
}

fn cubic_to(
    out: &mut Contour,
    p0: [f32; 2],
    c1: [f32; 2],
    c2: [f32; 2],
    p3: [f32; 2],
    tolerance: f32,
) {
    let d1 = [p0[0] - 2.0 * c1[0] + c2[0], p0[1] - 2.0 * c1[1] + c2[1]];
    let d2 = [c1[0] - 2.0 * c2[0] + p3[0], c1[1] - 2.0 * c2[1] + p3[1]];
    let n = segments(length(d1).max(length(d2)), 0.75, tolerance);
    for i in 1..=n {
        let t = f32::from(i) / f32::from(n);
        let u = 1.0 - t;
        let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        out.push([
            a * p0[0] + b * c1[0] + c * c2[0] + d * p3[0],
            a * p0[1] + b * c1[1] + c * c2[1] + d * p3[1],
        ]);
    }
}

fn quad_to(out: &mut Contour, p0: [f32; 2], c: [f32; 2], p2: [f32; 2], tolerance: f32) {
    let d = [p0[0] - 2.0 * c[0] + p2[0], p0[1] - 2.0 * c[1] + p2[1]];
    let n = segments(length(d), 0.25, tolerance);
    for i in 1..=n {
        let t = f32::from(i) / f32::from(n);
        let u = 1.0 - t;
        let (a, b, e) = (u * u, 2.0 * u * t, t * t);
        out.push([
            a * p0[0] + b * c[0] + e * p2[0],
            a * p0[1] + b * c[1] + e * p2[1],
        ]);
    }
}

/// An elliptical arc in endpoint form (SVG 1.1 F.6.5).
struct Arc {
    from: [f32; 2],
    radii: [f32; 2],
    rotation: f32,
    large: bool,
    sweep: bool,
    to: [f32; 2],
}

impl Arc {
    fn flatten(&self, out: &mut Contour, tolerance: f32) {
        let (mut rx, mut ry) = (self.radii[0].abs(), self.radii[1].abs());
        if self.from == self.to {
            return;
        }
        if rx == 0.0 || ry == 0.0 {
            out.push(self.to);
            return;
        }
        let (sin, cos) = self.rotation.to_radians().sin_cos();
        let dx = (self.from[0] - self.to[0]) / 2.0;
        let dy = (self.from[1] - self.to[1]) / 2.0;
        let x1 = cos * dx + sin * dy;
        let y1 = -sin * dx + cos * dy;
        // Radii too small for the endpoints grow until they fit.
        let lambda = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
        if lambda > 1.0 {
            let s = lambda.sqrt();
            rx *= s;
            ry *= s;
        }
        let num = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1;
        let den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
        let mut coef = (num / den).max(0.0).sqrt();
        if self.large == self.sweep {
            coef = -coef;
        }
        let cx1 = coef * rx * y1 / ry;
        let cy1 = -coef * ry * x1 / rx;
        let cx = cos * cx1 - sin * cy1 + (self.from[0] + self.to[0]) / 2.0;
        let cy = sin * cx1 + cos * cy1 + (self.from[1] + self.to[1]) / 2.0;
        let angle = |ux: f32, uy: f32| uy.atan2(ux);
        let theta = angle((x1 - cx1) / rx, (y1 - cy1) / ry);
        let mut delta = angle((-x1 - cx1) / rx, (-y1 - cy1) / ry) - theta;
        let tau = std::f32::consts::TAU;
        if self.sweep && delta < 0.0 {
            delta += tau;
        } else if !self.sweep && delta > 0.0 {
            delta -= tau;
        }
        // The angle a chord of the larger radius may span within tolerance.
        let r = rx.max(ry);
        let step = 2.0 * (1.0 - (tolerance / r).min(1.0)).acos();
        let n = if step > 0.0 {
            let n = (delta.abs() / step).ceil();
            if n.is_finite() {
                n.clamp(1.0, 128.0) as u16
            } else {
                1
            }
        } else {
            1
        };
        for i in 1..=n {
            let a = theta + delta * f32::from(i) / f32::from(n);
            let (s, c) = a.sin_cos();
            let (ex, ey) = (rx * c, ry * s);
            out.push([cx + cos * ex - sin * ey, cy + sin * ex + cos * ey]);
        }
        // The last point is the endpoint exactly, not its rounding.
        if let Some(last) = out.last_mut() {
            *last = self.to;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f32; 2], b: [f32; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3
    }

    #[test]
    fn s37_01_absolute_and_relative_lines_with_implicit_lineto() {
        let contours = contours_of("M10,10 20,10 l0,10 H10 v-5 Z", 0.5).expect("parses");
        assert_eq!(contours.len(), 1);
        let c = &contours[0];
        assert_eq!(
            c,
            &vec![
                [10.0, 10.0],
                [20.0, 10.0],
                [20.0, 20.0],
                [10.0, 20.0],
                [10.0, 15.0]
            ]
        );
    }

    #[test]
    fn s37_01_a_relative_moveto_after_close_starts_from_the_subpath_start() {
        let contours = contours_of("M10 10h10v10h-10z m5 5h2v2h-2z", 0.5).expect("parses");
        assert_eq!(contours.len(), 2);
        assert_eq!(contours[1][0], [15.0, 15.0]);
    }

    #[test]
    fn s37_01_compact_numbers_and_arc_flags_parse() {
        // `-.5.5` is two numbers; `011` is two flags and a number.
        let contours = contours_of("M0 0l10-.5.5 10a5 5 0 0110 0z", 0.5).expect("parses");
        let c = &contours[0];
        assert!(close(c[1], [10.0, -0.5]));
        assert!(close(c[2], [10.5, 9.5]));
        assert!(close(*c.last().expect("point"), [20.5, 9.5]));
    }

    #[test]
    fn s37_01_an_arc_stays_on_its_circle_and_ends_at_its_endpoint() {
        // A half circle of radius 50 around (100, 100).
        let contours = contours_of("M50,100A50,50,0,0,0,150,100Z", 0.5).expect("parses");
        let c = &contours[0];
        assert!(c.len() > 8, "flattened into several segments");
        for p in c {
            let r = (p[0] - 100.0).hypot(p[1] - 100.0);
            assert!((r - 50.0).abs() < 0.01, "{p:?} is off the circle");
            // sweep 0 from the left to the right point goes through the bottom (y > 100).
            assert!(p[1] >= 100.0 - 1e-3, "{p:?}");
        }
        assert!(close(*c.last().expect("point"), [150.0, 100.0]));
    }

    #[test]
    fn s37_01_curves_flatten_within_tolerance_and_smooth_ones_reflect() {
        let contours = contours_of(
            "M0,0C0,100,100,100,100,0S200,-100,200,0Q250,50,300,0T400,0Z",
            0.5,
        )
        .expect("parses");
        let c = &contours[0];
        // The cubic peaks at y = 75 at t = 0.5; S reflects to a trough of -75.
        let max = c.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
        let min = c.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
        assert!((max - 75.0).abs() < 0.5, "{max}");
        assert!((min + 75.0).abs() < 0.5, "{min}");
        assert!(c.iter().any(|p| close(*p, [400.0, 0.0])));
        // T reflects Q's control (250, 50) to (350, -50): a trough at x = 350.
        let trough = c
            .iter()
            .filter(|p| p[0] > 300.0)
            .map(|p| p[1])
            .fold(f32::MAX, f32::min);
        assert!((trough + 25.0).abs() < 0.5, "{trough}");
    }

    #[test]
    fn s37_01_malformed_data_is_an_error_not_a_guess() {
        assert_eq!(contours_of("L0,0", 0.5), Err(PathError::NoMoveTo));
        assert_eq!(contours_of("M0,0 X1,1", 0.5), Err(PathError::Command('X')));
        assert!(matches!(
            contours_of("M0,0 L1", 0.5),
            Err(PathError::Number(_))
        ));
        assert!(matches!(
            contours_of("M0,0 A1,1,0,2,0,5,5", 0.5),
            Err(PathError::Number(_))
        ));
    }

    #[test]
    fn s37_01_svg_files_other_than_256_unit_paths_are_rejected() {
        let ok = r#"<svg viewBox="0 0 256 256"><path d="M0,0H10V10Z"/></svg>"#;
        assert_eq!(svg_contours(ok).expect("parses").len(), 1);
        assert_eq!(
            svg_contours(r#"<svg viewBox="0 0 24 24"><path d="M0,0H1V1Z"/></svg>"#),
            Err(PathError::ViewBox)
        );
        assert_eq!(
            svg_contours(r#"<svg viewBox="0 0 256 256"><rect x="1"/></svg>"#),
            Err(PathError::Element("rect".to_owned()))
        );
        assert_eq!(
            svg_contours(r#"<svg viewBox="0 0 256 256"></svg>"#),
            Err(PathError::NoPath)
        );
    }
}
