//! Layer-4 adapter `gafira-io-vector` (spec 066, S66-10): the outline of a
//! custom frame from a vector file, an SVG or the first page of a PDF.
//!
//! Every filled path of the file is gathered, as drawn (with its
//! transforms), into one outline; the outline is moved into the unit box of
//! a `ShapePath`, and its proportions are kept apart. An outline with more
//! orders than a document holds is flattened into straight segments, more
//! coarsely each time, until it fits.

#![forbid(unsafe_code)]

use gafira_doc::{PathCommand, ShapePath};
use kurbo::{Affine, BezPath, ParamCurveNearest, PathEl, Point, Shape};

/// The outline of a frame from a file.
#[derive(Clone, Debug, PartialEq)]
pub struct Outline {
    /// In the unit box, its bounds touching every side.
    pub path: ShapePath,
    /// Width over height of the drawing.
    pub aspect: f64,
    /// The size of the document, in px at 96 per inch: the SVG's own size,
    /// or the PDF page (spec 068 shows it under the preview).
    pub size: (f64, f64),
}

/// Why a file gives no outline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VectorError {
    /// The file could not be read as an SVG or a PDF.
    Unreadable,
    /// The file draws no filled shape with an area.
    NoShape,
}

impl std::fmt::Display for VectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreadable => f.write_str("the file is not a readable SVG or PDF"),
            Self::NoShape => f.write_str("the file draws no filled shape"),
        }
    }
}

impl std::error::Error for VectorError {}

/// The outline of the filled shapes of an SVG.
///
/// # Errors
/// [`VectorError::Unreadable`] or [`VectorError::NoShape`].
pub fn from_svg(bytes: &[u8]) -> Result<Outline, VectorError> {
    use resvg::usvg;
    let tree = usvg::Tree::from_data(bytes, &usvg::Options::default())
        .map_err(|_| VectorError::Unreadable)?;
    let mut out = BezPath::new();
    svg_group(tree.root(), &mut out);
    let size = tree.size();
    outline(out, (f64::from(size.width()), f64::from(size.height())))
}

/// How a shape of an imported SVG is filled (spec 067).
#[derive(Clone, Debug, PartialEq)]
pub enum Fill {
    Solid(gafira_color::Color),
    Gradient(gafira_doc::Gradient),
}

/// One shape of an imported SVG: its box in the drawing's px, its outline in
/// that box (unit coordinates), its fill and its outline stroke (colour and
/// width in the drawing's px).
#[derive(Clone, Debug, PartialEq)]
pub struct VectorShape {
    pub bounds: (f64, f64, f64, f64),
    pub path: ShapePath,
    pub fill: Option<Fill>,
    pub stroke: Option<(gafira_color::Color, f64)>,
}

/// An SVG as Gafira shapes, bottom to top, and the drawing's own size.
#[derive(Clone, Debug, PartialEq)]
pub struct Drawing {
    pub size: (f64, f64),
    pub shapes: Vec<VectorShape>,
}

/// The most shapes an imported drawing brings.
pub const MAX_SHAPES: usize = 2000;

/// An SVG as vector shapes of Gafira, «tal cual» (spec 067): every path
/// with its fill (a colour, or a linear or radial gradient) and its outline.
/// Text and embedded pictures are not shapes and are left out.
///
/// # Errors
/// [`VectorError::Unreadable`], or [`VectorError::NoShape`] when it has
/// no path.
pub fn svg_drawing(bytes: &[u8]) -> Result<Drawing, VectorError> {
    use resvg::usvg;
    let tree = usvg::Tree::from_data(bytes, &usvg::Options::default())
        .map_err(|_| VectorError::Unreadable)?;
    let mut shapes = Vec::new();
    collect(tree.root(), &mut shapes);
    if shapes.is_empty() {
        return Err(VectorError::NoShape);
    }
    let size = tree.size();
    Ok(Drawing {
        size: (f64::from(size.width()), f64::from(size.height())),
        shapes,
    })
}

fn affine(t: resvg::tiny_skia::Transform) -> Affine {
    Affine::new([
        f64::from(t.sx),
        f64::from(t.ky),
        f64::from(t.kx),
        f64::from(t.sy),
        f64::from(t.tx),
        f64::from(t.ty),
    ])
}

fn bez(path: &resvg::tiny_skia::Path) -> BezPath {
    use resvg::tiny_skia::PathSegment as S;
    let p = |q: resvg::tiny_skia::Point| Point::new(f64::from(q.x), f64::from(q.y));
    let mut out = BezPath::new();
    for segment in path.segments() {
        match segment {
            S::MoveTo(a) => out.move_to(p(a)),
            S::LineTo(a) => out.line_to(p(a)),
            S::QuadTo(a, b) => out.quad_to(p(a), p(b)),
            S::CubicTo(a, b, c) => out.curve_to(p(a), p(b), p(c)),
            S::Close => out.close_path(),
        }
    }
    out
}

fn colour(c: resvg::usvg::Color, alpha: f32) -> gafira_color::Color {
    gafira_color::Color::Srgb(gafira_color::Rgba::new(
        f32::from(c.red) / 255.0,
        f32::from(c.green) / 255.0,
        f32::from(c.blue) / 255.0,
        alpha.clamp(0.0, 1.0),
    ))
}

/// The fill of `paint` at `opacity`, its gradients laid by `at` (the path's
/// transform); a pattern gives a mid grey.
fn fill_of(paint: &resvg::usvg::Paint, opacity: f32, at: Affine) -> Fill {
    use gafira_doc::{Gradient, GradientKind, GradientStop};
    use resvg::usvg::Paint;
    let stops = |stops: &[resvg::usvg::Stop]| {
        stops
            .iter()
            .map(|s| GradientStop {
                offset: s.offset().get(),
                color: colour(s.color(), s.opacity().get() * opacity),
            })
            .collect::<Vec<_>>()
    };
    let gradient = |kind, stops: Vec<GradientStop>| {
        let gradient = Gradient { kind, stops };
        if gradient.problem().is_none() {
            Fill::Gradient(gradient)
        } else {
            gradient.stops.first().map_or(
                Fill::Solid(colour(resvg::usvg::Color::black(), opacity)),
                |s| Fill::Solid(s.color),
            )
        }
    };
    match paint {
        Paint::Color(c) => Fill::Solid(colour(*c, opacity)),
        Paint::LinearGradient(g) => {
            let t = at * affine(g.transform());
            let a = t * Point::new(f64::from(g.x1()), f64::from(g.y1()));
            let b = t * Point::new(f64::from(g.x2()), f64::from(g.y2()));
            let angle = (b.y - a.y).atan2(b.x - a.x).to_degrees();
            gradient(GradientKind::Linear { angle_deg: angle }, stops(g.stops()))
        }
        Paint::RadialGradient(g) => gradient(GradientKind::Radial, stops(g.stops())),
        Paint::Pattern(_) => Fill::Solid(gafira_color::Color::Srgb(gafira_color::Rgba::new(
            0.5, 0.5, 0.5, opacity,
        ))),
    }
}

fn collect(group: &resvg::usvg::Group, out: &mut Vec<VectorShape>) {
    use resvg::usvg::Node;
    for node in group.children() {
        if out.len() >= MAX_SHAPES {
            return;
        }
        match node {
            Node::Group(group) => collect(group, out),
            Node::Path(path) if path.is_visible() => {
                let at = affine(path.abs_transform());
                let mut drawn = bez(path.data());
                drawn.apply_affine(at);
                let scale = at.determinant().abs().sqrt();
                let stroke = path.stroke().and_then(|s| match s.paint() {
                    resvg::usvg::Paint::Color(c) => Some((
                        colour(*c, s.opacity().get()),
                        f64::from(s.width().get()) * scale,
                    )),
                    _ => None,
                });
                let fill = path
                    .fill()
                    .map(|f| fill_of(f.paint(), f.opacity().get(), at));
                if fill.is_none() && stroke.is_none() {
                    continue;
                }
                // A straight line has no area: its box takes the stroke.
                let mut b = drawn.bounding_box();
                let half = stroke.map_or(0.5, |(_, w)| (w / 2.0).max(0.5));
                if b.width() <= 0.0 {
                    b = b.inflate(half, 0.0);
                }
                if b.height() <= 0.0 {
                    b = b.inflate(0.0, half);
                }
                if let Ok(outline) = outline(drawn, (b.width(), b.height())) {
                    out.push(VectorShape {
                        bounds: (b.x0, b.y0, b.width(), b.height()),
                        path: outline.path,
                        fill,
                        stroke,
                    });
                }
            }
            _ => {}
        }
    }
}

/// An SVG drawn so its longer side is `side` px, straight RGBA: a thumbnail
/// (spec 072).
///
/// # Errors
/// As [`svg_pixels`].
pub fn svg_thumbnail(bytes: &[u8], side: f32) -> Result<(Vec<u8>, u32, u32), VectorError> {
    let tree = resvg::usvg::Tree::from_data(bytes, &resvg::usvg::Options::default())
        .map_err(|_| VectorError::Unreadable)?;
    let longest = tree.size().width().max(tree.size().height()).max(1.0);
    svg_pixels(bytes, side / longest)
}

/// An SVG drawn as it is, in colour, `scale` times its own size (at most
/// 4096 px a side): straight (not premultiplied) RGBA and its size. For
/// «Como imagen» when an SVG is imported (spec 067).
///
/// # Errors
/// [`VectorError::Unreadable`] when the file is not an SVG, or
/// [`VectorError::NoShape`] when it has no size.
pub fn svg_pixels(bytes: &[u8], scale: f32) -> Result<(Vec<u8>, u32, u32), VectorError> {
    use resvg::{tiny_skia, usvg};
    let tree = usvg::Tree::from_data(bytes, &usvg::Options::default())
        .map_err(|_| VectorError::Unreadable)?;
    let size = tree.size();
    let scale = scale.min(4096.0 / size.width().max(size.height()));
    // Within 1..=4096: the casts are exact.
    let (w, h) = (
        (size.width() * scale).round().max(1.0) as u32,
        (size.height() * scale).round().max(1.0) as u32,
    );
    let mut pixmap = tiny_skia::Pixmap::new(w, h).ok_or(VectorError::NoShape)?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    let straight = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    Ok((straight, w, h))
}

fn svg_group(group: &resvg::usvg::Group, out: &mut BezPath) {
    use resvg::usvg::Node;
    for node in group.children() {
        match node {
            Node::Group(group) => svg_group(group, out),
            Node::Path(path) if path.fill().is_some() => {
                let t = path.abs_transform();
                let at = Affine::new([
                    f64::from(t.sx),
                    f64::from(t.ky),
                    f64::from(t.kx),
                    f64::from(t.sy),
                    f64::from(t.tx),
                    f64::from(t.ty),
                ]);
                let mut drawn = BezPath::new();
                for segment in path.data().segments() {
                    use resvg::tiny_skia::PathSegment as S;
                    let p = |q: resvg::tiny_skia::Point| Point::new(f64::from(q.x), f64::from(q.y));
                    match segment {
                        S::MoveTo(a) => drawn.move_to(p(a)),
                        S::LineTo(a) => drawn.line_to(p(a)),
                        S::QuadTo(a, b) => drawn.quad_to(p(a), p(b)),
                        S::CubicTo(a, b, c) => drawn.curve_to(p(a), p(b), p(c)),
                        S::Close => drawn.close_path(),
                    }
                }
                drawn.apply_affine(at);
                out.extend(drawn);
            }
            _ => {}
        }
    }
}

/// The outline of the filled shapes of the first page of a PDF.
///
/// # Errors
/// [`VectorError::Unreadable`] or [`VectorError::NoShape`].
pub fn from_pdf(bytes: &[u8]) -> Result<Outline, VectorError> {
    use hayro_syntax::content::ops::TypedInstruction as Op;
    let pdf = hayro_syntax::Pdf::new(bytes.to_vec()).map_err(|_| VectorError::Unreadable)?;
    let page = pdf.pages().first().ok_or(VectorError::Unreadable)?;
    let n = |v: &hayro_syntax::object::Number| v.as_f64();
    let mut states = vec![Affine::IDENTITY];
    let mut current = BezPath::new();
    let mut out = BezPath::new();
    // A clipping path (`W n`) is the outline of a vector trace that cuts an
    // image: used when the page fills nothing.
    let mut clips = BezPath::new();
    let mut clip_pending = false;
    let mut last = Point::ZERO;
    let mut ops = page.typed_operations();
    while let Some(op) = ops.next() {
        let ctm = *states.last().unwrap_or(&Affine::IDENTITY);
        match op {
            Op::SaveState(_) => states.push(ctm),
            Op::RestoreState(_) => {
                if states.len() > 1 {
                    states.pop();
                }
            }
            Op::Transform(t) => {
                let m = Affine::new([n(&t.0), n(&t.1), n(&t.2), n(&t.3), n(&t.4), n(&t.5)]);
                if let Some(top) = states.last_mut() {
                    *top = ctm * m;
                }
            }
            Op::MoveTo(m) => {
                last = Point::new(n(&m.0), n(&m.1));
                current.move_to(ctm * last);
            }
            Op::LineTo(l) => {
                last = Point::new(n(&l.0), n(&l.1));
                current.line_to(ctm * last);
            }
            Op::CubicTo(c) => {
                let (a, b) = (Point::new(n(&c.0), n(&c.1)), Point::new(n(&c.2), n(&c.3)));
                let end = Point::new(n(&c.4), n(&c.5));
                current.curve_to(ctm * a, ctm * b, ctm * end);
                last = end;
            }
            Op::CubicStartTo(c) => {
                let b = Point::new(n(&c.0), n(&c.1));
                let end = Point::new(n(&c.2), n(&c.3));
                current.curve_to(ctm * last, ctm * b, ctm * end);
                last = end;
            }
            Op::CubicEndTo(c) => {
                let a = Point::new(n(&c.0), n(&c.1));
                let end = Point::new(n(&c.2), n(&c.3));
                current.curve_to(ctm * a, ctm * end, ctm * end);
                last = end;
            }
            Op::ClosePath(_) => current.close_path(),
            Op::RectPath(r) => {
                let rect = kurbo::Rect::from_origin_size((n(&r.0), n(&r.1)), (n(&r.2), n(&r.3)));
                let mut path = rect.to_path(0.1);
                path.apply_affine(ctm);
                current.extend(path);
            }
            // Filled: kept. Stroked only, or a clip: dropped.
            Op::FillPathNonZero(_)
            | Op::FillPathNonZeroCompatibility(_)
            | Op::FillPathEvenOdd(_)
            | Op::FillAndStrokeNonZero(_)
            | Op::FillAndStrokeEvenOdd(_)
            | Op::CloseFillAndStrokeNonZero(_)
            | Op::CloseFillAndStrokeEvenOdd(_) => {
                out.extend(std::mem::take(&mut current));
            }
            Op::ClipNonZero(_) | Op::ClipEvenOdd(_) => clip_pending = true,
            Op::EndPath(_) if clip_pending => {
                clips.extend(std::mem::take(&mut current));
                clip_pending = false;
            }
            Op::StrokePath(_) | Op::CloseAndStrokePath(_) | Op::EndPath(_) => {
                current = BezPath::new();
                clip_pending = false;
            }
            _ => {}
        }
    }
    if out.elements().is_empty() {
        out = clips;
    }
    // PDF runs y up; a page and a frame run it down.
    out.apply_affine(Affine::FLIP_Y);
    let media = page.media_box();
    let px = 96.0 / 72.0;
    outline(
        out,
        (
            (media.x1 - media.x0).abs() * px,
            (media.y1 - media.y0).abs() * px,
        ),
    )
}

/// `drawn` moved into the unit box, with its proportions, simplified until
/// it fits in a document.
fn outline(drawn: BezPath, size: (f64, f64)) -> Result<Outline, VectorError> {
    let bounds = drawn.bounding_box();
    if !(bounds.width() > 0.0 && bounds.height() > 0.0 && bounds.is_finite()) {
        return Err(VectorError::NoShape);
    }
    let to_unit = Affine::scale_non_uniform(1.0 / bounds.width(), 1.0 / bounds.height())
        * Affine::translate((-bounds.x0, -bounds.y0));
    let unit = |p: Point| {
        let q = to_unit * p;
        [q.x.clamp(0.0, 1.0), q.y.clamp(0.0, 1.0)]
    };
    let mut commands: Vec<PathCommand> = drawn
        .iter()
        .map(|el| match el {
            PathEl::MoveTo(p) => PathCommand::MoveTo(unit(p)),
            PathEl::LineTo(p) => PathCommand::LineTo(unit(p)),
            PathEl::QuadTo(a, p) => PathCommand::QuadTo(unit(a), unit(p)),
            PathEl::CurveTo(a, b, p) => PathCommand::CubicTo(unit(a), unit(b), unit(p)),
            PathEl::ClosePath => PathCommand::Close,
        })
        .collect();
    if commands.len() > ShapePath::MAX_COMMANDS {
        commands = simplified(&drawn, bounds)
            .into_iter()
            .flat_map(|(points, closed)| {
                let mut out: Vec<PathCommand> = Vec::with_capacity(points.len() + 1);
                for (index, p) in points.iter().enumerate() {
                    out.push(if index == 0 {
                        PathCommand::MoveTo(unit(*p))
                    } else {
                        PathCommand::LineTo(unit(*p))
                    });
                }
                if closed {
                    out.push(PathCommand::Close);
                }
                out
            })
            .collect();
    }
    Ok(Outline {
        path: ShapePath::new(commands),
        aspect: bounds.width() / bounds.height(),
        size,
    })
}

/// A polyline of `drawn`'s subpaths, and whether each one is closed.
type Rings = Vec<(Vec<Point>, bool)>;

/// `drawn` as straight segments, simplified more coarsely each time until
/// its orders fit in a document; past the size of the drawing, the largest
/// subpaths are kept.
fn simplified(drawn: &BezPath, bounds: kurbo::Rect) -> Rings {
    let size = bounds.width().max(bounds.height());
    let orders = |rings: &Rings| {
        rings
            .iter()
            .map(|(p, c)| p.len() + usize::from(*c))
            .sum::<usize>()
    };
    let mut epsilon = size / 2000.0;
    loop {
        let rings: Rings = polylines(drawn, epsilon / 4.0)
            .into_iter()
            .map(|(points, closed)| (rdp(&points, epsilon), closed))
            .filter(|(points, _)| points.len() >= 2)
            .collect();
        if orders(&rings) <= ShapePath::MAX_COMMANDS {
            return rings;
        }
        if epsilon > size {
            let mut rings = rings;
            rings.sort_by(|a, b| area(&b.0).total_cmp(&area(&a.0)));
            let mut kept = Rings::new();
            for ring in rings {
                if orders(&kept) + ring.0.len() + 1 > ShapePath::MAX_COMMANDS {
                    break;
                }
                kept.push(ring);
            }
            return kept;
        }
        epsilon *= 2.0;
    }
}

/// The subpaths of `drawn` flattened within `tolerance`.
fn polylines(drawn: &BezPath, tolerance: f64) -> Rings {
    let mut rings: Rings = Vec::new();
    kurbo::flatten(drawn.iter(), tolerance.max(1e-6), |el| match el {
        PathEl::MoveTo(p) => rings.push((vec![p], false)),
        PathEl::LineTo(p) => {
            if let Some((ring, _)) = rings.last_mut() {
                ring.push(p);
            }
        }
        PathEl::ClosePath => {
            if let Some((_, closed)) = rings.last_mut() {
                *closed = true;
            }
        }
        _ => {}
    });
    rings
}

/// Ramer–Douglas–Peucker: the points of `points` that keep it within
/// `epsilon`.
fn rdp(points: &[Point], epsilon: f64) -> Vec<Point> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    let mut stack = vec![(0, points.len() - 1)];
    while let Some((a, b)) = stack.pop() {
        let line = kurbo::Line::new(points[a], points[b]);
        let (mut far, mut index) = (0.0, a);
        for (i, p) in points.iter().enumerate().take(b).skip(a + 1) {
            let d = line.nearest(*p, 1e-9).distance_sq.sqrt();
            if d > far {
                far = d;
                index = i;
            }
        }
        if far > epsilon {
            keep[index] = true;
            stack.push((a, index));
            stack.push((index, b));
        }
    }
    points
        .iter()
        .zip(keep)
        .filter_map(|(p, k)| k.then_some(*p))
        .collect()
}

/// The area of the box of `points`.
fn area(points: &[Point]) -> f64 {
    let (mut lo, mut hi) = (
        Point::new(f64::MAX, f64::MAX),
        Point::new(f64::MIN, f64::MIN),
    );
    for p in points {
        lo = Point::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    ((hi.x - lo.x) * (hi.y - lo.y)).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: [f64; 2], b: [f64; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6
    }

    // S66-10: an SVG triangle becomes an outline in the unit box, wider
    // than high as drawn; a stroke-only path is not part of it.
    #[test]
    fn s66_10_an_svg_path_becomes_a_frame_shape() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100">
            <g transform="translate(10 20)"><path d="M0 100 L100 0 L200 100 Z" fill="black"/></g>
            <path d="M0 0 L500 500" stroke="red" fill="none"/>
        </svg>"#;
        let outline = from_svg(svg).expect("an outline");
        assert!((outline.aspect - 2.0).abs() < 1e-6, "{}", outline.aspect);
        assert_eq!(outline.size, (200.0, 100.0), "the SVG's size");
        let points: Vec<[f64; 2]> = outline
            .path
            .commands
            .iter()
            .filter_map(|c| match c {
                PathCommand::MoveTo(p) | PathCommand::LineTo(p) => Some(*p),
                _ => None,
            })
            .collect();
        assert!(
            near(points[0], [0.0, 1.0]) && near(points[1], [0.5, 0.0]),
            "{points:?}"
        );
        assert!(from_svg(b"<svg xmlns='http://www.w3.org/2000/svg'/>").is_err());
        assert_eq!(from_svg(b"not an svg"), Err(VectorError::Unreadable));
    }

    // Spec 067: «tal cual» brings every path as a shape of Gafira, with its
    // box in the drawing, its colour, its gradient and its outline.
    #[test]
    fn s67_an_svg_comes_in_as_vector_shapes() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50">
            <defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="0">
                <stop offset="0" stop-color="#ff0000"/><stop offset="1" stop-color="#0000ff"/>
            </linearGradient></defs>
            <rect x="10" y="10" width="30" height="20" fill="#00ff00"/>
            <rect x="50" y="0" width="40" height="40" fill="url(#g)" stroke="#000000" stroke-width="2"/>
        </svg>"##;
        let drawing = svg_drawing(svg).expect("a drawing");
        assert_eq!(drawing.size, (100.0, 50.0));
        assert_eq!(drawing.shapes.len(), 2);
        let first = &drawing.shapes[0];
        assert_eq!(first.bounds, (10.0, 10.0, 30.0, 20.0));
        assert!(matches!(
            first.fill,
            Some(Fill::Solid(gafira_color::Color::Srgb(c))) if c.g > 0.99 && c.r < 0.01
        ));
        let second = &drawing.shapes[1];
        assert!(
            matches!(second.fill, Some(Fill::Gradient(_))),
            "{:?}",
            second.fill
        );
        assert_eq!(second.stroke.map(|(_, w)| w), Some(2.0));
        assert!(svg_drawing(b"<svg xmlns='http://www.w3.org/2000/svg'/>").is_err());
    }

    // Spec 067: «Como imagen» draws the SVG as it is, in colour, at the
    // scale asked.
    #[test]
    fn s67_an_svg_is_drawn_in_colour() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"><rect width="20" height="20" fill="red"/></svg>"#;
        let (rgba, w, h) = svg_pixels(svg, 2.0).expect("pixels");
        assert_eq!((w, h), (80, 40));
        let at = |x: u32, y: u32| {
            let i = ((y * w + x) * 4) as usize;
            [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
        };
        assert_eq!(at(10, 20), [255, 0, 0, 255], "the red square");
        assert_eq!(at(70, 20)[3], 0, "transparent beside it");
        assert_eq!(svg_pixels(b"nope", 1.0), Err(VectorError::Unreadable));
    }

    /// A one-page PDF whose content stream is `content`.
    fn pdf(content: &str) -> Vec<u8> {
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Contents 4 0 R >>".to_owned(),
            format!(
                "<< /Length {} >>\nstream\n{content}\nendstream",
                content.len()
            ),
        ];
        let mut out = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend(format!("{} 0 obj\n{body}\nendobj\n", index + 1).bytes());
        }
        let xref = out.len();
        out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).bytes());
        for offset in offsets {
            out.extend(format!("{offset:010} 00000 n \n").bytes());
        }
        out.extend(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                objects.len() + 1
            )
            .bytes(),
        );
        out
    }

    // S66-10: a filled PDF path becomes the same kind of outline, its y
    // turned the way a page runs; a stroke alone is not a shape.
    #[test]
    fn s66_10_a_pdf_path_becomes_a_frame_shape() {
        let outline =
            from_pdf(&pdf("q 1 0 0 1 10 10 cm 0 0 m 200 0 l 100 100 l h f Q")).expect("an outline");
        assert!((outline.aspect - 2.0).abs() < 1e-6);
        assert_eq!(outline.size, (400.0, 400.0), "300 pt at 96 px per inch");
        let first = match outline.path.commands[0] {
            PathCommand::MoveTo(p) => p,
            ref other => panic!("{other:?}"),
        };
        assert!(
            near(first, [0.0, 1.0]),
            "the bottom left, y down: {first:?}"
        );
        assert_eq!(
            from_pdf(&pdf("0 0 m 100 100 l S")),
            Err(VectorError::NoShape)
        );
        assert_eq!(from_pdf(b"%PDF nothing"), Err(VectorError::Unreadable));
        // A trace that cuts an image: its clipping path is the outline.
        let clipped =
            from_pdf(&pdf("q 0 0 m 100 0 l 100 50 l h W n 1 0 0 1 0 0 cm Q")).expect("the clip");
        assert!((clipped.aspect - 2.0).abs() < 1e-6);
    }

    // S66-10: an outline longer than a document holds is simplified until
    // it fits, its bounds kept.
    #[test]
    fn s66_10_a_long_outline_is_simplified_to_fit() {
        let mut d = String::from("M0 0");
        for i in 0..6000 {
            let x = f64::from(i) / 10.0;
            d.push_str(&format!(" Q{x} {} {} 0", 1.0 + f64::from(i % 3), x + 0.05));
        }
        d.push_str(" L600 50 L0 50 Z");
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="600" height="50"><path d="{d}"/></svg>"#
        );
        let outline = from_svg(svg.as_bytes()).expect("an outline");
        assert!(outline.path.commands.len() <= ShapePath::MAX_COMMANDS);
        assert!((outline.aspect - 12.0).abs() < 0.1, "{}", outline.aspect);
    }
}
