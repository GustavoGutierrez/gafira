//! How an image is made to look (spec 085, FD-32 to FD-35): its adjustments,
//! those of its zones, a filter and a shadow. Its pixels do not change;
//! `gafira-raster` applies the look when they are drawn.

use gafira_color::{Color, Rgba};
use serde::{Deserialize, Serialize};

use gafira_geom::Point;

use crate::{AssetRef, ContentHash, Placement};

fn none(v: &f64) -> bool {
    *v == 0.0
}

fn off(v: &bool) -> bool {
    !*v
}

/// A colour of the image moved to another (FD-33).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColorShift {
    pub color: Color,
    /// Degrees, −180 to 180.
    #[serde(default)]
    pub hue: f64,
    /// −100 to 100.
    #[serde(default)]
    pub saturation: f64,
}

/// The adjustments of FD-33, 0 changing nothing. Each in its range there.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImageAdjust {
    #[serde(skip_serializing_if = "none")]
    pub temperature: f64,
    #[serde(skip_serializing_if = "none")]
    pub tint: f64,
    #[serde(skip_serializing_if = "none")]
    pub brightness: f64,
    #[serde(skip_serializing_if = "none")]
    pub contrast: f64,
    #[serde(skip_serializing_if = "none")]
    pub highlights: f64,
    #[serde(skip_serializing_if = "none")]
    pub shadows: f64,
    #[serde(skip_serializing_if = "none")]
    pub whites: f64,
    #[serde(skip_serializing_if = "none")]
    pub blacks: f64,
    #[serde(skip_serializing_if = "none")]
    pub hue: f64,
    #[serde(skip_serializing_if = "none")]
    pub saturation: f64,
    #[serde(skip_serializing_if = "none")]
    pub vibrance: f64,
    #[serde(skip_serializing_if = "off")]
    pub invert: bool,
    #[serde(skip_serializing_if = "none")]
    pub sharpness: f64,
    #[serde(skip_serializing_if = "none")]
    pub clarity: f64,
    #[serde(skip_serializing_if = "none")]
    pub vignette: f64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub colors: Vec<ColorShift>,
}

impl ImageAdjust {
    /// The most colours moved.
    pub const MAX_COLORS: usize = 8;

    /// Whether it changes nothing.
    #[must_use]
    pub fn is_plain(&self) -> bool {
        *self == Self::default()
    }

    fn is_valid(&self) -> bool {
        let within = |list: &[f64], range: std::ops::RangeInclusive<f64>| {
            list.iter().all(|v| range.contains(v))
        };
        let both = [
            self.temperature,
            self.tint,
            self.brightness,
            self.contrast,
            self.highlights,
            self.shadows,
            self.whites,
            self.blacks,
            self.saturation,
            self.vibrance,
            self.vignette,
        ];
        within(&both, -100.0..=100.0)
            && within(&[self.hue], -180.0..=180.0)
            && within(&[self.sharpness, self.clarity], 0.0..=100.0)
            && self.colors.len() <= Self::MAX_COLORS
            && self.colors.iter().all(|c| {
                is_color(c.color)
                    && within(&[c.hue], -180.0..=180.0)
                    && within(&[c.saturation], -100.0..=100.0)
            })
    }
}

fn is_color(color: Color) -> bool {
    color.is_finite() && (0.0..=1.0).contains(&color.alpha())
}

/// How a zone was chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ZoneKind {
    Element,
    Area,
    Foreground,
    Background,
}

/// A part of the image with adjustments of its own (FD-32).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageZone {
    pub kind: ZoneKind,
    /// Where it is: the opacity, or the light, of an image of any size.
    pub mask: AssetRef,
    /// Whether it is all but the mask.
    #[serde(default, skip_serializing_if = "off")]
    pub inverted: bool,
    #[serde(default)]
    pub adjust: ImageAdjust,
}

/// A filter of the catalogue (FD-34).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageFilter {
    pub id: String,
    /// 0 to 100.
    pub intensity: f64,
}

/// The shapes a shadow takes (FD-35).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ShadowKind {
    Glow,
    Drop,
    Outline,
    Curved,
    Floating,
    Angled,
    Ground,
}

/// The shadow of an image. Lengths are shares of its smaller side.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageShadow {
    pub kind: ShadowKind,
    pub color: Color,
    /// 0 to 1.
    pub opacity: f64,
    /// 0 to [`Self::MAX_LENGTH`].
    pub blur: f64,
    pub distance: f64,
    pub size: f64,
    /// Degrees clockwise from the right, −180 to 180.
    pub angle_deg: f64,
}

impl ImageShadow {
    pub const MAX_LENGTH: f64 = 0.3;

    /// The shadow `kind` starts with.
    #[must_use]
    pub fn new(kind: ShadowKind) -> Self {
        let of = |opacity, blur, distance, size, angle_deg| Self {
            kind,
            color: Color::Srgb(Rgba::new(0.094, 0.141, 0.231, 1.0)),
            opacity,
            blur,
            distance,
            size,
            angle_deg,
        };
        match kind {
            ShadowKind::Glow => of(0.7, 0.04, 0.0, 0.01, 0.0),
            ShadowKind::Drop => of(0.5, 0.02, 0.035, 0.0, 45.0),
            ShadowKind::Outline => of(1.0, 0.0, 0.0, 0.02, 0.0),
            ShadowKind::Curved => of(0.55, 0.02, 0.05, 0.0, 90.0),
            ShadowKind::Floating => of(0.5, 0.03, 0.08, 0.04, 90.0),
            ShadowKind::Angled => of(0.45, 0.02, 0.02, 0.08, 30.0),
            ShadowKind::Ground => of(0.45, 0.03, 0.12, 0.2, 35.0),
        }
    }

    /// How far it may reach past the image, as a share of its smaller side.
    #[must_use]
    pub fn reach(&self) -> f64 {
        // A blur is seen up to three times its radius.
        let soft = 3.0 * self.blur + 0.005;
        soft + match self.kind {
            ShadowKind::Glow | ShadowKind::Outline => self.size,
            ShadowKind::Drop | ShadowKind::Curved => self.distance,
            ShadowKind::Floating | ShadowKind::Angled | ShadowKind::Ground => {
                self.distance + self.size
            }
        }
    }

    /// How far past a box of `w` × `h` an image of `pixels`, its shadow
    /// around it, is drawn: across and down.
    #[must_use]
    pub fn bleed(&self, pixels: (u32, u32), (w, h): (f64, f64)) -> (f64, f64) {
        let (pw, ph) = (f64::from(pixels.0), f64::from(pixels.1));
        let margin = self.margin_within(pw.min(ph));
        let (inner_w, inner_h) = (pw - 2.0 * margin, ph - 2.0 * margin);
        if inner_w <= 0.0 || inner_h <= 0.0 {
            return (0.0, 0.0);
        }
        (margin / inner_w * w, margin / inner_h * h)
    }

    /// The whole pixels of margin around an image whose smaller side is `side`.
    #[must_use]
    pub fn margin(&self, side: f64) -> f64 {
        (self.reach() * side).ceil()
    }

    /// The margin of an image that, with it, has `padded` as its smaller side.
    fn margin_within(&self, padded: f64) -> f64 {
        let reach = self.reach();
        let near = (padded * reach / (1.0 + 2.0 * reach)).floor();
        // It was rounded up: the one that gives `padded` back.
        (0..3)
            .map(|n| near + f64::from(n))
            .find(|m| self.margin(padded - 2.0 * m) == *m)
            .unwrap_or(near)
    }

    fn is_valid(&self) -> bool {
        let length = 0.0..=Self::MAX_LENGTH;
        is_color(self.color)
            && (0.0..=1.0).contains(&self.opacity)
            && length.contains(&self.blur)
            && length.contains(&self.distance)
            && length.contains(&self.size)
            && (-180.0..=180.0).contains(&self.angle_deg)
    }
}

/// How an image is made to look (spec 085).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImageLook {
    #[serde(skip_serializing_if = "ImageAdjust::is_plain")]
    pub adjust: ImageAdjust,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub zones: Vec<ImageZone>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<ImageFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadow: Option<ImageShadow>,
}

impl ImageLook {
    pub const MAX_ZONES: usize = 8;
    /// The most bytes of the name of a filter.
    pub const MAX_FILTER: usize = 64;

    /// Whether there is nothing to it.
    #[must_use]
    pub fn is_plain(&self) -> bool {
        *self == Self::default()
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.adjust.is_valid()
            && self.zones.len() <= Self::MAX_ZONES
            && self.zones.iter().all(|zone| zone.adjust.is_valid())
            && self.filter.as_ref().is_none_or(|f| {
                (1..=Self::MAX_FILTER).contains(&f.id.len()) && (0.0..=100.0).contains(&f.intensity)
            })
            && self.shadow.as_ref().is_none_or(ImageShadow::is_valid)
    }

    /// What the pixels of `asset`, its part `crop` with this look, are kept
    /// under.
    #[must_use]
    pub fn key(&self, asset: AssetRef, crop: Option<&ImageCrop>) -> AssetRef {
        // Four runs of FNV-1a from as many starts: a name for what is kept in
        // memory, not a proof of content.
        const PRIME: u64 = 0x0000_0100_0000_01b3;
        let written = serde_json::to_vec(&(self, crop)).unwrap_or_default();
        let mut out = [0; 32];
        for (lane, bytes) in out.chunks_exact_mut(8).enumerate() {
            // Four lanes: the cast is exact.
            let mut hash = 0xcbf2_9ce4_8422_2325 ^ (lane as u64 + 1).wrapping_mul(PRIME);
            for byte in asset.as_hash().0.iter().chain(&written) {
                hash = (hash ^ u64::from(*byte)).wrapping_mul(PRIME).rotate_left(5);
            }
            bytes.copy_from_slice(&hash.to_le_bytes());
        }
        AssetRef::new(ContentHash(out))
    }
}

/// The shapes an image is cut to.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CropShape {
    #[default]
    Rect,
    /// Its corners are round: a share of its smaller side.
    Rounded(f64),
    Ellipse,
    /// Drawn by hand: points, as shares of the sides of the part.
    Path(Vec<[f64; 2]>),
}

/// The part of an image that shows (spec 088): shares of its sides, so a
/// better resolution does not change it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageCrop {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    #[serde(default, skip_serializing_if = "is_rect")]
    pub shape: CropShape,
}

fn is_rect(shape: &CropShape) -> bool {
    *shape == CropShape::Rect
}

impl Default for ImageCrop {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
            shape: CropShape::Rect,
        }
    }
}

impl ImageCrop {
    /// The least of a side that shows.
    pub const LEAST: f64 = 0.001;
    /// The most points of a shape drawn by hand.
    pub const MAX_POINTS: usize = 512;

    /// Whether all of the image shows.
    #[must_use]
    pub fn is_whole(&self) -> bool {
        *self == Self::default()
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        // Shares are sums of others: not exact.
        let most = 1.0 + 1e-9;
        let side = |from: f64, size: f64| {
            from >= 0.0 && size >= Self::LEAST && from + size <= most && from.is_finite()
        };
        side(self.x, self.w)
            && side(self.y, self.h)
            && match &self.shape {
                CropShape::Rect | CropShape::Ellipse => true,
                CropShape::Rounded(radius) => (0.0..=0.5).contains(radius),
                CropShape::Path(points) => {
                    (3..=Self::MAX_POINTS).contains(&points.len())
                        && points.iter().flatten().all(|v| (0.0..=1.0).contains(v))
                }
            }
    }

    /// The box all of the image has, when its part has the box `part`: what
    /// is seen stays where it is, however it is turned.
    #[must_use]
    pub fn whole_box(&self, part: &Placement) -> Placement {
        let (w, h) = (part.w / self.w, part.h / self.h);
        boxed(part, (-self.x * w, -self.y * h), (w, h))
    }

    /// The box its part has, when all of the image has the box `whole`.
    #[must_use]
    pub fn part_box(&self, whole: &Placement) -> Placement {
        let at = (self.x * whole.w, self.y * whole.h);
        boxed(whole, at, (self.w * whole.w, self.h * whole.h))
    }
}

/// The box of `size` that begins at `from` inside the one of `placement`,
/// turned as it is.
fn boxed(placement: &Placement, from: (f64, f64), (w, h): (f64, f64)) -> Placement {
    let middle = placement.to_transform() * Point::new(from.0 + w / 2.0, from.1 + h / 2.0);
    Placement {
        x: middle.x - w / 2.0,
        y: middle.y - h / 2.0,
        w,
        h,
        ..*placement
    }
}
