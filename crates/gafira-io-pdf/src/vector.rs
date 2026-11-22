//! A page written as vectors (spec 073): the drawing operations of the page
//! become PDF paths, gradients, images and real text with its fonts
//! embedded (subsets), so text stays selectable and shapes sharp at any
//! zoom.

use std::collections::HashMap;
use std::sync::Arc;

use gafira_color::Color;
use gafira_doc::{BlendMode, GradientKind};
use gafira_ports::render::{GlyphRun, RenderOp};
use gafira_raster::ImageBuf;
use krilla::color::{cmyk, rgb};
use krilla::geom::{Path, PathBuilder, Point, Size, Transform};
use krilla::num::NormalizedF32;
use krilla::paint::{Fill, FillRule, LinearGradient, Paint, RadialGradient, SpreadMethod, Stop};
use krilla::surface::Surface;
use krilla::text::{Font, GlyphId, KrillaGlyph};
use kurbo::{Affine, BezPath, PathEl};

/// How colours are written: RGB for screens, CMYK for print (spec 073).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Profile {
    Rgb,
    Cmyk,
}

/// What the page is drawn with.
pub(crate) struct Settings {
    pub(crate) profile: Profile,
    /// Images as JPEG, no larger than they are shown (spec 073).
    pub(crate) compress: bool,
    /// Page px to PDF points.
    pub(crate) points: f64,
}

/// A drawing step, nested: layers become groups.
enum Node {
    Fill {
        path: Arc<BezPath>,
        paint: Painted,
        transform: Affine,
    },
    Image {
        image: Arc<ImageBuf>,
        transform: Affine,
    },
    Text {
        path: Arc<BezPath>,
        color: Color,
        transform: Affine,
        runs: Arc<Vec<GlyphRun>>,
    },
    Group {
        children: Vec<Node>,
        opacity: f32,
        blend: BlendMode,
        clip: Option<(Arc<BezPath>, Affine)>,
    },
}

enum Painted {
    Color(Color),
    Gradient(Arc<gafira_doc::Gradient>, (f64, f64)),
}

/// The operations as nested nodes, or `None` when one of them has no
/// vector form (a mask, an effect, a transition): that page is drawn as
/// pixels instead.
fn nest(ops: &[RenderOp]) -> Option<Vec<Node>> {
    let mut stack: Vec<Vec<Node>> = vec![Vec::new()];
    for op in ops {
        match op {
            RenderOp::PushLayer => stack.push(Vec::new()),
            RenderOp::PopLayer { blend, opacity } => {
                let children = stack.pop()?;
                stack.last_mut()?.push(Node::Group {
                    children,
                    opacity: *opacity,
                    blend: *blend,
                    clip: None,
                });
            }
            RenderOp::FillPath {
                path,
                color,
                transform,
            } => stack.last_mut()?.push(Node::Fill {
                path: Arc::clone(path),
                paint: Painted::Color(*color),
                transform: *transform,
            }),
            RenderOp::FillGradient {
                path,
                gradient,
                size,
                transform,
            } => stack.last_mut()?.push(Node::Fill {
                path: Arc::clone(path),
                paint: Painted::Gradient(Arc::clone(gradient), *size),
                transform: *transform,
            }),
            RenderOp::DrawImage { image, transform } => stack.last_mut()?.push(Node::Image {
                image: Arc::clone(image),
                transform: *transform,
            }),
            RenderOp::Text {
                path,
                color,
                transform,
                runs,
            } => stack.last_mut()?.push(Node::Text {
                path: Arc::clone(path),
                color: *color,
                transform: *transform,
                runs: Arc::clone(runs),
            }),
            // Cuts what the open layer drew so far: those nodes go into a
            // clipped group; what comes after is not cut.
            RenderOp::ClipPath { path, transform } => {
                let open = stack.last_mut()?;
                let children = std::mem::take(open);
                open.push(Node::Group {
                    children,
                    opacity: 1.0,
                    blend: BlendMode::Normal,
                    clip: Some((Arc::clone(path), *transform)),
                });
            }
            RenderOp::ApplyMask { .. }
            | RenderOp::ApplyEffect { .. }
            | RenderOp::ApplyTransition { .. } => return None,
        }
    }
    if stack.len() == 1 { stack.pop() } else { None }
}

/// The fonts of an export, each read once.
#[derive(Default)]
pub(crate) struct Fonts(HashMap<(usize, u32), Option<Font>>);

/// The bytes of a font, shared with krilla.
struct Bytes(Arc<[u8]>);

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Fonts {
    fn get(&mut self, run: &GlyphRun) -> Option<Font> {
        let key = (Arc::as_ptr(&run.font).cast::<u8>() as usize, run.index);
        self.0
            .entry(key)
            .or_insert_with(|| {
                let data: Arc<dyn AsRef<[u8]> + Send + Sync> =
                    Arc::new(Bytes(Arc::clone(&run.font)));
                Font::new(data.into(), run.index)
            })
            .clone()
    }
}

/// Writes `ops` on `surface`; `false` when the page has a step with no
/// vector form, and nothing was written.
pub(crate) fn draw(
    surface: &mut Surface<'_>,
    ops: &[RenderOp],
    settings: &Settings,
    fonts: &mut Fonts,
) -> bool {
    let Some(nodes) = nest(ops) else {
        return false;
    };
    let k = settings.points;
    surface.push_transform(&Transform::from_scale(k as f32, k as f32));
    for node in &nodes {
        node_out(surface, node, settings, fonts);
    }
    surface.pop();
    true
}

fn node_out(surface: &mut Surface<'_>, node: &Node, settings: &Settings, fonts: &mut Fonts) {
    match node {
        Node::Fill {
            path,
            paint,
            transform,
        } => {
            let Some(path) = pdf_path(path, *transform) else {
                return;
            };
            let (paint, opacity) = match paint {
                Painted::Color(color) => (colour(*color, settings.profile), alpha(*color)),
                Painted::Gradient(gradient, size) => (
                    gradient_paint(gradient, *size, *transform, settings.profile),
                    1.0,
                ),
            };
            surface.set_fill(Some(Fill {
                paint,
                opacity: NormalizedF32::new(opacity).unwrap_or(NormalizedF32::ONE),
                rule: FillRule::NonZero,
            }));
            surface.set_stroke(None);
            surface.draw_path(&path);
        }
        Node::Image { image, transform } => {
            let Some(image_out) = pdf_image(image, *transform, settings) else {
                return;
            };
            let Some(size) = Size::from_wh(image.width() as f32, image.height() as f32) else {
                return;
            };
            surface.push_transform(&affine(*transform));
            surface.draw_image(image_out, size);
            surface.pop();
        }
        Node::Text {
            path,
            color,
            transform,
            runs,
        } => {
            surface.set_fill(Some(Fill {
                paint: colour(*color, settings.profile),
                opacity: NormalizedF32::new(alpha(*color)).unwrap_or(NormalizedF32::ONE),
                rule: FillRule::NonZero,
            }));
            surface.set_stroke(None);
            // Real text, its font embedded; without runs, its outline.
            let mut written = !runs.is_empty();
            if written {
                surface.push_transform(&affine(*transform));
                for run in runs.iter() {
                    if !glyphs_out(surface, run, fonts) {
                        written = false;
                    }
                }
                surface.pop();
            }
            if !written && let Some(path) = pdf_path(path, *transform) {
                surface.draw_path(&path);
            }
        }
        Node::Group {
            children,
            opacity,
            blend,
            clip,
        } => {
            let mut pushed = 0;
            if let Some((path, transform)) = clip
                && let Some(path) = pdf_path(path, *transform)
            {
                surface.push_clip_path(&path, &FillRule::NonZero);
                pushed += 1;
            }
            if *blend != BlendMode::Normal {
                surface.push_blend_mode(blend_mode(*blend));
                pushed += 1;
            }
            if *opacity < 1.0 {
                surface.push_opacity(
                    NormalizedF32::new(opacity.clamp(0.0, 1.0)).unwrap_or(NormalizedF32::ONE),
                );
                pushed += 1;
            }
            for child in children {
                node_out(surface, child, settings, fonts);
            }
            for _ in 0..pushed {
                surface.pop();
            }
        }
    }
}

/// One run of glyphs as text: each glyph advanced to where the next one
/// starts, on the run's first baseline.
fn glyphs_out(surface: &mut Surface<'_>, run: &GlyphRun, fonts: &mut Fonts) -> bool {
    let Some(first) = run.glyphs.first() else {
        return true;
    };
    let Some(font) = fonts.get(run) else {
        return false;
    };
    let size = run.size.max(0.01);
    let glyphs: Vec<KrillaGlyph> = run
        .glyphs
        .iter()
        .enumerate()
        .map(|(i, glyph)| {
            let next = run.glyphs.get(i + 1).map_or(glyph.x, |n| n.x);
            KrillaGlyph::new(
                GlyphId::new(glyph.id),
                (next - glyph.x) / size,
                0.0,
                (glyph.y - first.y) / size,
                0.0,
                glyph.text.clone(),
                None,
            )
        })
        .collect();
    surface.draw_glyphs(
        Point::from_xy(first.x, first.y),
        &glyphs,
        font,
        &run.text,
        size,
        false,
    );
    true
}

/// `path` laid on the page by `transform`, as a PDF path.
fn pdf_path(path: &BezPath, transform: Affine) -> Option<Path> {
    let mut builder = PathBuilder::new();
    for el in (transform * path.clone()).elements() {
        match *el {
            PathEl::MoveTo(p) => builder.move_to(p.x as f32, p.y as f32),
            PathEl::LineTo(p) => builder.line_to(p.x as f32, p.y as f32),
            PathEl::QuadTo(c, p) => builder.quad_to(c.x as f32, c.y as f32, p.x as f32, p.y as f32),
            PathEl::CurveTo(a, b, p) => builder.cubic_to(
                a.x as f32, a.y as f32, b.x as f32, b.y as f32, p.x as f32, p.y as f32,
            ),
            PathEl::ClosePath => builder.close(),
        }
    }
    builder.finish()
}

fn affine(t: Affine) -> Transform {
    let [a, b, c, d, e, f] = t.as_coeffs();
    Transform::from_row(a as f32, b as f32, c as f32, d as f32, e as f32, f as f32)
}

/// The straight sRGB channels of `color`, 0 to 255.
fn srgb8(color: Color) -> [u8; 4] {
    let Color::Srgb(c) = color.convert(gafira_color::ColorSpaceId::Srgb) else {
        return [0, 0, 0, 255];
    };
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    [q(c.r), q(c.g), q(c.b), q(c.a)]
}

fn alpha(color: Color) -> f32 {
    f32::from(srgb8(color)[3]) / 255.0
}

/// `color` in the profile of the export; CMYK from sRGB by the simple
/// rule (no ICC profile, spec 073).
fn colour(color: Color, profile: Profile) -> Paint {
    let [r, g, b, _] = srgb8(color);
    match profile {
        Profile::Rgb => rgb::Color::new(r, g, b).into(),
        Profile::Cmyk => {
            let [c, m, y, k] = to_cmyk(r, g, b);
            cmyk::Color::new(c, m, y, k).into()
        }
    }
}

fn stop_colour(color: Color, profile: Profile) -> krilla::color::Color {
    let [r, g, b, _] = srgb8(color);
    match profile {
        Profile::Rgb => rgb::Color::new(r, g, b).into(),
        Profile::Cmyk => {
            let [c, m, y, k] = to_cmyk(r, g, b);
            cmyk::Color::new(c, m, y, k).into()
        }
    }
}

/// sRGB to CMYK by the simple rule: K is what no channel lights, and C, M,
/// Y share the rest.
pub(crate) fn to_cmyk(r: u8, g: u8, b: u8) -> [u8; 4] {
    let (r, g, b) = (
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
    );
    let k = 1.0 - r.max(g).max(b);
    if k >= 1.0 {
        return [0, 0, 0, 255];
    }
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let part = |v: f32| (1.0 - v - k) / (1.0 - k);
    [q(part(r)), q(part(g)), q(part(b)), q(k)]
}

/// A gradient laid in a `size` box that `transform` maps to the page, as
/// the canvas lays it.
fn gradient_paint(
    gradient: &gafira_doc::Gradient,
    (w, h): (f64, f64),
    transform: Affine,
    profile: Profile,
) -> Paint {
    let stops: Vec<Stop> = gradient
        .stops
        .iter()
        .map(|stop| Stop {
            offset: NormalizedF32::new(stop.offset.clamp(0.0, 1.0)).unwrap_or(NormalizedF32::ZERO),
            color: stop_colour(stop.color, profile),
            opacity: NormalizedF32::new(alpha(stop.color)).unwrap_or(NormalizedF32::ONE),
        })
        .collect();
    let (cx, cy) = (w / 2.0, h / 2.0);
    match gradient.kind {
        GradientKind::Linear { angle_deg } => {
            let (sin, cos) = angle_deg.to_radians().sin_cos();
            let half = (cx * cos).abs() + (cy * sin).abs();
            LinearGradient {
                x1: (cx - cos * half) as f32,
                y1: (cy - sin * half) as f32,
                x2: (cx + cos * half) as f32,
                y2: (cy + sin * half) as f32,
                transform: affine(transform),
                spread_method: SpreadMethod::Pad,
                stops,
                anti_alias: true,
            }
            .into()
        }
        GradientKind::Radial => RadialGradient {
            fx: cx as f32,
            fy: cy as f32,
            fr: 0.0,
            cx: cx as f32,
            cy: cy as f32,
            cr: cx.hypot(cy) as f32,
            transform: affine(transform),
            spread_method: SpreadMethod::Pad,
            stops,
            anti_alias: true,
        }
        .into(),
    }
}

/// An image for the PDF: its pixels, or, compressed, a JPEG no larger than
/// twice the size it is shown at (when it has no transparency).
fn pdf_image(
    image: &ImageBuf,
    transform: Affine,
    settings: &Settings,
) -> Option<krilla::image::Image> {
    let pixels = gafira_raster::straight_srgb8(image)?;
    let (w, h) = (image.width(), image.height());
    if settings.compress && pixels.chunks_exact(4).all(|p| p[3] == 255) {
        let rgba = image::RgbaImage::from_raw(w, h, pixels.clone())?;
        // The size it is shown at, in PDF points, twice over.
        let [a, b, c, d, _, _] = transform.as_coeffs();
        let shown_w = (a.hypot(b) * f64::from(w) * settings.points * 2.0)
            .ceil()
            .max(1.0) as u32;
        let shown_h = (c.hypot(d) * f64::from(h) * settings.points * 2.0)
            .ceil()
            .max(1.0) as u32;
        let rgb = image::DynamicImage::ImageRgba8(rgba);
        let rgb = if shown_w < w || shown_h < h {
            rgb.resize(
                shown_w.min(w),
                shown_h.min(h),
                image::imageops::FilterType::Triangle,
            )
        } else {
            rgb
        };
        let mut jpeg = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 78);
        rgb.to_rgb8().write_with_encoder(encoder).ok()?;
        return krilla::image::Image::from_jpeg(jpeg.into(), true).ok();
    }
    Some(krilla::image::Image::from_rgba8(pixels, w, h))
}

fn blend_mode(blend: BlendMode) -> krilla::blend::BlendMode {
    use krilla::blend::BlendMode as B;
    match blend {
        BlendMode::Normal => B::Normal,
        BlendMode::Multiply => B::Multiply,
        BlendMode::Screen => B::Screen,
        BlendMode::Overlay => B::Overlay,
        BlendMode::Darken => B::Darken,
        BlendMode::Lighten => B::Lighten,
        BlendMode::ColorDodge => B::ColorDodge,
        BlendMode::ColorBurn => B::ColorBurn,
        BlendMode::HardLight => B::HardLight,
        BlendMode::SoftLight => B::SoftLight,
        BlendMode::Difference => B::Difference,
        BlendMode::Exclusion => B::Exclusion,
        BlendMode::Hue => B::Hue,
        BlendMode::Saturation => B::Saturation,
        BlendMode::Color => B::Color,
        BlendMode::Luminosity => B::Luminosity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Spec 073: CMYK by the simple rule; black is K only, white nothing.
    #[test]
    fn cmyk_by_the_simple_rule() {
        assert_eq!(to_cmyk(0, 0, 0), [0, 0, 0, 255]);
        assert_eq!(to_cmyk(255, 255, 255), [0, 0, 0, 0]);
        assert_eq!(to_cmyk(255, 0, 0), [0, 255, 255, 0]);
        assert_eq!(to_cmyk(0, 128, 255), [255, 127, 0, 0]);
    }

    // Spec 073: layers become groups; a clip cuts what came before it in
    // its layer; a mask has no vector form.
    #[test]
    fn operations_nest_into_groups() {
        let path = Arc::new(BezPath::new());
        let fill = || RenderOp::FillPath {
            path: Arc::clone(&path),
            color: Color::Srgb(gafira_color::Rgba::new(0.0, 0.0, 0.0, 1.0)),
            transform: Affine::IDENTITY,
        };
        let ops = vec![
            fill(),
            RenderOp::PushLayer,
            fill(),
            RenderOp::ClipPath {
                path: Arc::clone(&path),
                transform: Affine::IDENTITY,
            },
            fill(),
            RenderOp::PopLayer {
                blend: BlendMode::Multiply,
                opacity: 0.5,
            },
        ];
        let nodes = nest(&ops).expect("nests");
        assert_eq!(nodes.len(), 2);
        let Node::Group {
            children, opacity, ..
        } = &nodes[1]
        else {
            panic!("a group")
        };
        assert_eq!(*opacity, 0.5);
        assert_eq!(children.len(), 2, "the clipped part and what came after");
        assert!(matches!(&children[0], Node::Group { clip: Some(_), .. }));
        let unbalanced = [RenderOp::PushLayer];
        assert!(nest(&unbalanced).is_none());
    }
}
