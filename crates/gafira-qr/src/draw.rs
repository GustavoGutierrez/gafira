//! The outlines of a code.

use gafira_color::Color;
use gafira_doc::timeline::AssetRef;
use gafira_doc::{QrFill, QrNode};
use kurbo::{BezPath, Point, Rect};

use crate::shapes::{Eye, ball_path, body_path, frame_path};
use crate::{Matrix, QrError, payload};

/// A part of a code: one compound path with one colour.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub path: BezPath,
    pub fill: QrFill,
}

/// Where the logo goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogoBox {
    pub asset: AssetRef,
    pub rect: Rect,
}

/// The outlines of a code: 1 unit a module, (0, 0) the corner of its first
/// module. Holes run against their outline, so the non-zero and the
/// even-odd rule fill the same.
#[derive(Clone, Debug, PartialEq)]
pub struct QrDrawing {
    /// The modules of a side, without the margin.
    pub size: f64,
    /// The margin, in modules.
    pub margin: f64,
    pub background: Option<Color>,
    /// Body, rings and centres, back to front.
    pub layers: Vec<Layer>,
    pub logo: Option<LogoBox>,
}

impl QrDrawing {
    /// How a box of `w` × `h` px holds the code and its margin: the px of a
    /// module and where the corner of the first one lies, in px of the box.
    /// The code is square: it sits in the middle of the longer side.
    #[must_use]
    pub fn fit(&self, w: f64, h: f64) -> Option<(f64, (f64, f64))> {
        let side = w.min(h);
        let k = side / (self.size + 2.0 * self.margin);
        if !(k.is_finite() && k > 0.0) {
            return None;
        }
        let corner = |length: f64| (length - side) / 2.0 + self.margin * k;
        Some((k, (corner(w), corner(h))))
    }
}

/// What the modules of `body` look like, in a box of 5 × 5: its tile.
#[must_use]
pub fn swatch(body: gafira_doc::QrBody) -> BezPath {
    body_path(&Matrix::swatch(), body, &|_, _| true)
}

/// How many modules a logo keeps from the edges: the eyes, the timing and
/// the format modules lie outside (S80-10).
const LOGO_MARGIN: f64 = 10.0;

/// The outlines of `qr`.
pub fn draw(qr: &QrNode) -> Result<QrDrawing, QrError> {
    let text = payload(&qr.content).map_err(QrError::Content)?;
    // A logo has no room in version 1.
    let least = if qr.logo.is_some() { 2 } else { 1 };
    let matrix = Matrix::encode(&text, qr.effective_ecc(), least)?;
    // At most 177: the cast is exact.
    let size = matrix.size() as f64;
    let logo = qr.logo.map(|logo| {
        let side = (size * logo.share).min(size - 2.0 * LOGO_MARGIN);
        let (w, h) = (f64::from(logo.natural.0), f64::from(logo.natural.1));
        let k = side / w.max(h);
        LogoBox {
            asset: logo.asset,
            rect: Rect::from_center_size((size / 2.0, size / 2.0), (w * k, h * k)),
        }
    });
    let hidden = match (qr.logo, logo) {
        (Some(asked), Some(drawn)) if asked.clear_behind => Some(drawn.rect.inset(0.5)),
        _ => None,
    };
    let keep = |x: i32, y: i32| {
        let behind = hidden.is_some_and(|hole| {
            let (x, y) = (f64::from(x), f64::from(y));
            x + 1.0 > hole.x0 && x < hole.x1 && y + 1.0 > hole.y0 && y < hole.y1
        });
        !matrix.in_eye(x, y) && !behind
    };
    let style = &qr.style;
    let body = body_path(&matrix, style.body, &keep);
    let (mut rings, mut balls) = (BezPath::new(), BezPath::new());
    for (eye, at) in [
        (Eye::TopLeft, Point::ZERO),
        (Eye::TopRight, Point::new(size - 7.0, 0.0)),
        (Eye::BottomLeft, Point::new(0.0, size - 7.0)),
    ] {
        let facing = if style.mirror_eyes { eye } else { Eye::TopLeft };
        frame_path(&mut rings, at, style.eye_frame, facing);
        ball_path(&mut balls, at, style.eye_ball, facing);
    }
    let or_body = |own: &Option<QrFill>| own.clone().unwrap_or_else(|| style.fill.clone());
    Ok(QrDrawing {
        size,
        margin: f64::from(qr.quiet_zone),
        background: style.background,
        layers: vec![
            Layer {
                path: body,
                fill: style.fill.clone(),
            },
            Layer {
                path: rings,
                fill: or_body(&style.eye_frame_fill),
            },
            Layer {
                path: balls,
                fill: or_body(&style.eye_ball_fill),
            },
        ],
        logo,
    })
}
