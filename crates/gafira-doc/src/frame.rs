//! Frames (spec 066, FD-18, D-107): a photo cut out by a shape. The photo
//! lives in the frame as a reference to an asset, so it is not a layer of
//! its own and replacing it is one change.

use gafira_geom::{Affine, Rect};
use serde::{Deserialize, Serialize};

use crate::timeline::AssetRef;
use crate::{ShapeGeometry, ShapeStroke};

/// How the photo sits in the box of its frame (spec 066, «Reglas de
/// encaje»).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fit {
    /// «Rellenar»: covers the box; what overflows is cut.
    #[default]
    Fill,
    /// «Encajar»: the whole photo shows; the rest stays empty.
    Fit,
    /// «Estirar»: stretched to the box.
    Stretch,
    /// «Original»: its natural size, centred.
    None,
}

/// The photo of a frame: the asset and its size in px, so the fit is known
/// without opening the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameContent {
    pub asset: AssetRef,
    pub natural: (u32, u32),
}

/// A frame (spec 066): `clip_shape` in the box of the node cuts the photo.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrameNode {
    pub clip_shape: ShapeGeometry,
    /// The rounding of a rectangle, in px; 0 for square corners.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub corner_radius: f64,
    /// The outline drawn over the cut edge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<ShapeStroke>,
    /// The photo; `None` is an empty frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<FrameContent>,
    #[serde(default)]
    pub fit: Fit,
    /// How far the photo is moved from the centre, as a share of the box.
    #[serde(default)]
    pub content_offset: (f64, f64),
    /// A factor on the scale of `fit`; above zero.
    #[serde(default = "one")]
    pub content_scale: f64,
    /// «Girar»: the photo turned about its centre, clockwise, in degrees.
    /// Written only when set.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub content_rotation_deg: f64,
}

fn is_zero(value: &f64) -> bool {
    value.to_bits() == 0
}

fn one() -> f64 {
    1.0
}

impl FrameNode {
    /// An empty frame of `clip_shape`, filled as the photo arrives.
    #[must_use]
    pub const fn new(clip_shape: ShapeGeometry) -> Self {
        Self {
            clip_shape,
            corner_radius: 0.0,
            stroke: None,
            content: None,
            fit: Fit::Fill,
            content_offset: (0.0, 0.0),
            content_scale: 1.0,
            content_rotation_deg: 0.0,
        }
    }

    /// Whether its numbers can be drawn: a finite scale above zero, a
    /// finite offset and corner radius, a photo of some size.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.content_scale.is_finite()
            && self.content_scale > 0.0
            && self.content_offset.0.is_finite()
            && self.content_offset.1.is_finite()
            && self.corner_radius.is_finite()
            && self.content_rotation_deg.is_finite()
            && self.corner_radius >= 0.0
            && self
                .content
                .is_none_or(|c| c.natural.0 > 0 && c.natural.1 > 0)
    }

    /// The offset after the photo is dragged by (`dx`, `dy`) px of the box
    /// `w` × `h` (S66-07): with «Rellenar» it stops where the box would show
    /// a gap, so the stored offset is the one drawn.
    #[must_use]
    pub fn moved(&self, w: f64, h: f64, dx: f64, dy: f64) -> (f64, f64) {
        if !(w > 0.0 && h > 0.0) {
            return self.content_offset;
        }
        let wanted = (
            self.content_offset.0 + dx / w,
            self.content_offset.1 + dy / h,
        );
        self.size(w, h)
            .map_or(wanted, |size| self.held(w, h, wanted, size))
    }

    /// Where the whole photo lies in the box `w` × `h`, before it is turned
    /// about its centre by `content_rotation_deg` (S66-07).
    #[must_use]
    pub fn photo_bounds(&self, w: f64, h: f64) -> Option<Rect> {
        let (dw, dh) = self.size(w, h)?;
        let offset = self.held(w, h, self.content_offset, (dw, dh));
        let centre = (w / 2.0 + offset.0 * w, h / 2.0 + offset.1 * h);
        Some(Rect::new(
            centre.0 - dw / 2.0,
            centre.1 - dh / 2.0,
            centre.0 + dw / 2.0,
            centre.1 + dh / 2.0,
        ))
    }

    /// The transform from the px of the photo to the box of the frame,
    /// `w` × `h`; `None` for an empty frame (S66-03).
    #[must_use]
    pub fn content_transform(&self, w: f64, h: f64) -> Option<Affine> {
        let content = self.content?;
        let (nw, nh) = (f64::from(content.natural.0), f64::from(content.natural.1));
        let bounds = self.photo_bounds(w, h)?;
        let centre = bounds.center();
        Some(
            Affine::translate((centre.x, centre.y))
                * Affine::rotate(self.content_rotation_deg.to_radians())
                * Affine::scale_non_uniform(bounds.width() / nw, bounds.height() / nh)
                * Affine::translate((-nw / 2.0, -nh / 2.0)),
        )
    }

    /// The size the photo is drawn at in the box `w` × `h`. «Rellenar» takes
    /// the turn into account: the turned photo still covers the box.
    fn size(&self, w: f64, h: f64) -> Option<(f64, f64)> {
        let content = self.content?;
        let (nw, nh) = (f64::from(content.natural.0), f64::from(content.natural.1));
        if !(nw > 0.0 && nh > 0.0 && w > 0.0 && h > 0.0) {
            return None;
        }
        let (sin, cos) = self.content_rotation_deg.to_radians().sin_cos();
        let (s, c) = (sin.abs(), cos.abs());
        let (sx, sy) = match self.fit {
            Fit::Fill => {
                let k = ((w * c + h * s) / nw).max((w * s + h * c) / nh);
                (k, k)
            }
            Fit::Fit => {
                let k = (w / nw).min(h / nh);
                (k, k)
            }
            Fit::Stretch => (w / nw, h / nh),
            Fit::None => (1.0, 1.0),
        };
        Some((nw * sx * self.content_scale, nh * sy * self.content_scale))
    }

    /// `offset` as far as it can go with «Rellenar» keeping the box covered
    /// by the photo of `size`, turned: first across, then down. Other fits
    /// move freely.
    fn held(&self, w: f64, h: f64, offset: (f64, f64), size: (f64, f64)) -> (f64, f64) {
        if self.fit != Fit::Fill {
            return offset;
        }
        let (sin, cos) = self.content_rotation_deg.to_radians().sin_cos();
        let axes = [((cos, sin), size.0 / 2.0), ((-sin, cos), size.1 / 2.0)];
        let corners = [
            (-w / 2.0, -h / 2.0),
            (w / 2.0, -h / 2.0),
            (-w / 2.0, h / 2.0),
            (w / 2.0, h / 2.0),
        ];
        // The largest share `t` of `step` from `from` (box px) that keeps
        // every corner on the photo; 0 when even `from` does not.
        let reach = |from: (f64, f64), step: (f64, f64)| {
            let mut t: f64 = 1.0;
            for &(x, y) in &corners {
                for &((ux, uy), half) in &axes {
                    let a = ux * (x - from.0) + uy * (y - from.1);
                    let b = ux * step.0 + uy * step.1;
                    if a.abs() > half + 1e-9 {
                        return 0.0;
                    }
                    if b > 1e-12 {
                        t = t.min((a + half) / b);
                    } else if b < -1e-12 {
                        t = t.min((half - a) / -b);
                    }
                }
            }
            t.clamp(0.0, 1.0)
        };
        let across = offset.0 * w;
        let x = across * reach((0.0, 0.0), (across, 0.0));
        let down = offset.1 * h;
        let y = down * reach((x, 0.0), (0.0, down));
        (x / w, y / h)
    }
}
