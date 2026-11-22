//! A code taken apart into shapes and an image (S80-13), for «Convertir en
//! formas» and the exports that know only shapes.

use gafira_doc::{
    ImageNode, NodeKind, PathCommand, Placement, QrFill, QrNode, ShapeGeometry, ShapeNode,
    ShapePath,
};
use kurbo::{BezPath, PathEl, Point, Shape};

use crate::{QrError, draw};

/// A shape or the image of a code, and where it lies.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub kind: NodeKind,
    pub placement: Placement,
}

/// The parts of `qr` in the box `placement`, back to front; none for a box
/// without area. Every part shares the centre, the turn and the flips of the
/// box, so together they look as the code does.
pub fn parts(qr: &QrNode, placement: &Placement) -> Result<Vec<Part>, QrError> {
    let drawing = draw(qr)?;
    let Some((k, _)) = drawing.fit(placement.w, placement.h) else {
        return Ok(Vec::new());
    };
    let centred = |w: f64, h: f64| Placement {
        x: placement.x + (placement.w - w) / 2.0,
        y: placement.y + (placement.h - h) / 2.0,
        w,
        h,
        ..*placement
    };
    let side = drawing.size * k;
    let mut out = Vec::new();
    if let Some(color) = drawing.background {
        let whole = side + 2.0 * drawing.margin * k;
        let mut shape = ShapeNode::new(ShapeGeometry::Rect);
        shape.fill = Some(color);
        out.push(Part {
            kind: NodeKind::Shape(shape),
            placement: centred(whole, whole),
        });
    }
    for layer in &drawing.layers {
        for commands in pieces(&layer.path, drawing.size)? {
            let mut shape = ShapeNode::new(ShapeGeometry::Path(ShapePath::new(commands)));
            match &layer.fill {
                QrFill::Solid(color) => shape.fill = Some(*color),
                QrFill::Gradient(gradient) => shape.gradient = Some(gradient.clone()),
            }
            out.push(Part {
                kind: NodeKind::Shape(shape),
                placement: centred(side, side),
            });
        }
    }
    if let Some(logo) = drawing.logo {
        out.push(Part {
            kind: NodeKind::Image(ImageNode::new(logo.asset)),
            placement: centred(logo.rect.width() * k, logo.rect.height() * k),
        });
    }
    Ok(out)
}

/// `path` in the unit box of a code of `size` modules, as few paths of at
/// most [`ShapePath::MAX_COMMANDS`] orders as it takes, a hole always with
/// the outline around it.
fn pieces(path: &BezPath, size: f64) -> Result<Vec<Vec<PathCommand>>, QrError> {
    // Outlines first, each with its holes: clockwise with y down is a
    // positive area.
    let mut groups: Vec<(BezPath, Vec<PathCommand>)> = Vec::new();
    let mut holes: Vec<BezPath> = Vec::new();
    for sub in subpaths(path) {
        if sub.area() > 0.0 {
            let orders = orders(&sub, size);
            groups.push((sub, orders));
        } else {
            holes.push(sub);
        }
    }
    for hole in holes {
        let inside = hole.bounding_box().center();
        // The smallest outline around it.
        let around = groups
            .iter_mut()
            .filter(|(outline, _)| {
                let (b, h) = (outline.bounding_box(), hole.bounding_box());
                b.x0 <= h.x0
                    && b.y0 <= h.y0
                    && b.x1 >= h.x1
                    && b.y1 >= h.y1
                    && (outline.winding(inside) != 0 || touches(outline, &hole))
            })
            .min_by(|a, b| {
                let area = |g: &(BezPath, Vec<PathCommand>)| g.0.bounding_box().area();
                area(a).total_cmp(&area(b))
            });
        if let Some((_, orders_of)) = around {
            orders_of.extend(orders(&hole, size));
        }
    }
    let mut out: Vec<Vec<PathCommand>> = Vec::new();
    for (_, group) in groups {
        if group.len() > ShapePath::MAX_COMMANDS {
            return Err(QrError::TooLong);
        }
        match out.last_mut() {
            Some(last) if last.len() + group.len() <= ShapePath::MAX_COMMANDS => last.extend(group),
            _ => out.push(group),
        }
    }
    Ok(out)
}

/// Whether the first point of `hole` lies inside `outline`.
fn touches(outline: &BezPath, hole: &BezPath) -> bool {
    matches!(hole.elements().first(), Some(PathEl::MoveTo(p)) if outline.winding(*p) != 0)
}

fn subpaths(path: &BezPath) -> Vec<BezPath> {
    let mut out: Vec<BezPath> = Vec::new();
    for el in path.elements() {
        if matches!(el, PathEl::MoveTo(_)) {
            out.push(BezPath::new());
        }
        if let Some(last) = out.last_mut() {
            last.push(*el);
        }
    }
    out
}

fn orders(path: &BezPath, size: f64) -> Vec<PathCommand> {
    let at = |p: Point| [p.x / size, p.y / size];
    path.elements()
        .iter()
        .map(|el| match *el {
            PathEl::MoveTo(p) => PathCommand::MoveTo(at(p)),
            PathEl::LineTo(p) => PathCommand::LineTo(at(p)),
            PathEl::QuadTo(c, p) => PathCommand::QuadTo(at(c), at(p)),
            PathEl::CurveTo(c, d, p) => PathCommand::CubicTo(at(c), at(d), at(p)),
            PathEl::ClosePath => PathCommand::Close,
        })
        .collect()
}
