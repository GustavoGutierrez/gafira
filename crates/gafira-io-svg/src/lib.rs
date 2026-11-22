//! Layer-4 adapter `gafira-io-svg` (02 §2.1): the SVG exporter (spec 074).
//!
//! One SVG file per page of the scope, written as vectors from the drawing
//! operations the `PageRenderer` gives (spec 073): paths, gradients, images
//! (as PNG), clips and layers, and text as real text with its fonts
//! embedded, or as outlines. «Fondo transparente» leaves out the white of
//! the page. A page with a step that has no vector form (a mask, an effect)
//! is embedded as one image. The exporter never sees `gafira-render`.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::{self, Write};
use std::sync::Arc;

use gafira_color::Color;
use gafira_doc::contract::{CancelToken, ParamDef, ParamKind, ParamSchema, ParamValues, Progress};
use gafira_doc::msg;
use gafira_doc::msg::{MessageKey, ToMessage};
use gafira_doc::{BlendMode, GradientKind, PageId};
use gafira_geom::IRect;
use gafira_io_image::page_pixels;
use gafira_ports::common::LegalClass;
use gafira_ports::export::{
    ExportCaps, ExportError, ExportIssue, ExportJob, ExportReport, ExportRequest, ExportScope,
    ExportSink, ExportedFile, Exporter, IssueSeverity, PageRenderRequest,
};
use gafira_ports::format::{ByteSignature, FormatDescriptor, FormatKind};
use gafira_ports::render::{GlyphRun, RenderOp};
use gafira_raster::ImageBuf;
use gafira_time::Flicks;
use kurbo::{Affine, BezPath, PathEl};

/// The option ids.
pub const TRANSPARENT: &str = "transparent";
pub const TEXT_AS_PATHS: &str = "text_as_paths";

/// The SVG format.
pub const SVG_FORMAT: FormatDescriptor = FormatDescriptor {
    id: "svg",
    extensions: &["svg"],
    mime: &["image/svg+xml"],
    signatures: &[ByteSignature {
        offset: 0,
        bytes: b"<svg",
    }],
    kind: FormatKind::Vector,
    legal: LegalClass::RoyaltyFree,
};

/// The SVG exporter: one file per page.
pub struct SvgExporter {
    schema: ParamSchema,
}

impl Default for SvgExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl SvgExporter {
    /// The exporter: real text, on the page's white, by default.
    #[must_use]
    pub fn new() -> Self {
        let def = |id, label, default| ParamDef {
            id,
            label: MessageKey(label),
            kind: ParamKind::Bool { default },
            animatable: false,
        };
        Self {
            schema: ParamSchema::new(vec![
                def(TRANSPARENT, "export.transparent", false),
                def(TEXT_AS_PATHS, "export.svg_text_as_paths", false),
            ]),
        }
    }
}

fn invalid(detail: &str) -> ExportError {
    ExportError::Io(io::Error::new(io::ErrorKind::InvalidInput, detail))
}

fn pages_of(request: &ExportRequest) -> Result<&[PageId], ExportIssue> {
    let blocking = |message| ExportIssue {
        severity: IssueSeverity::Error,
        message,
    };
    match &request.scope {
        ExportScope::Pages(pages) if pages.is_empty() => Err(blocking(msg!("export.no_pages"))),
        ExportScope::Pages(pages) => Ok(pages),
        ExportScope::Nodes { .. } => Err(blocking(msg!("export.selection_unsupported"))),
    }
}

impl Exporter for SvgExporter {
    fn descriptor(&self) -> &FormatDescriptor {
        &SVG_FORMAT
    }

    fn capabilities(&self) -> ExportCaps {
        ExportCaps {
            transparency: true,
            ..ExportCaps::default()
        }
    }

    fn options_schema(&self) -> &ParamSchema {
        &self.schema
    }

    fn validate(&self, request: &ExportRequest) -> Vec<ExportIssue> {
        let mut issues = Vec::new();
        if let Err(error) = self.schema.check_values(&request.options) {
            issues.push(ExportIssue {
                severity: IssueSeverity::Error,
                message: error.to_message(),
            });
        }
        issues.extend(pages_of(request).err());
        issues
    }

    fn export(
        &self,
        job: &ExportJob<'_>,
        sink: &mut dyn ExportSink,
        progress: &dyn Progress,
        cancel: &CancelToken,
    ) -> Result<ExportReport, ExportError> {
        let options: ParamValues = self
            .schema
            .resolve(&job.request.options)
            .map_err(|_| invalid("invalid SVG options"))?;
        let transparent = options.bool(TRANSPARENT).unwrap_or(false);
        let outlines = options.bool(TEXT_AS_PATHS).unwrap_or(false);
        let pages = pages_of(job.request).map_err(|_| invalid("no page to export"))?;
        let dpi = job.document.dpi();
        let mut report = ExportReport::default();
        for (at, id) in pages.iter().enumerate() {
            cancel.check()?;
            let page = job
                .document
                .page(*id)
                .ok_or_else(|| invalid("the page is not in the document"))?;
            let (w, h) = page_pixels(page, dpi, 1.0)?;
            let mut ops = job.pages.vectors(*id, cancel)?;
            // `vectors` puts the white of the page first: left out when
            // transparent.
            if transparent
                && let Some(list) = ops.as_mut()
                && !list.is_empty()
            {
                list.remove(0);
            }
            let svg = match ops.and_then(|ops| page_svg(&ops, (w, h), outlines)) {
                Some(svg) => svg,
                None => {
                    // No vector form: the page as one image.
                    let image = job.pages.render(
                        &PageRenderRequest {
                            page: *id,
                            time: Flicks::default(),
                            scale: 2.0,
                            region: IRect::new(0, 0, (w * 2) as i32, (h * 2) as i32),
                            transparent,
                        },
                        cancel,
                    )?;
                    image_page(&image, (w, h)).ok_or_else(|| invalid("the render is a mask"))?
                }
            };
            let name = if pages.len() == 1 {
                "page.svg".to_owned()
            } else {
                format!("page-{}.svg", at + 1)
            };
            let mut out = sink.create(&name)?;
            out.write_all(svg.as_bytes())?;
            out.flush()?;
            report.files.push(ExportedFile {
                name,
                bytes: svg.len() as u64,
            });
            progress.report((at + 1) as f32 / pages.len() as f32);
        }
        Ok(report)
    }
}

/// What an SVG page holds besides its drawing: gradients, clips and fonts.
#[derive(Default)]
struct Defs {
    body: String,
    count: usize,
    fonts: HashMap<usize, String>,
}

impl Defs {
    fn id(&mut self, prefix: &str) -> String {
        self.count += 1;
        format!("{prefix}{}", self.count)
    }
}

/// The page as SVG, or `None` when a step has no vector form.
fn page_svg(ops: &[RenderOp], (w, h): (u32, u32), outlines: bool) -> Option<String> {
    let mut defs = Defs::default();
    let mut body = String::new();
    // Open groups: whether each has a clip waiting to wrap it.
    let mut open: Vec<String> = Vec::new();
    for op in ops {
        match op {
            RenderOp::PushLayer => open.push(std::mem::take(&mut body)),
            RenderOp::PopLayer { blend, opacity } => {
                let inner = std::mem::replace(&mut body, open.pop()?);
                let mut style = String::new();
                if *blend != BlendMode::Normal {
                    let _ = write!(style, " style=\"mix-blend-mode:{}\"", blend_name(*blend));
                }
                if *opacity < 1.0 {
                    let _ = write!(style, " opacity=\"{}\"", num(f64::from(*opacity)));
                }
                let _ = write!(body, "<g{style}>{inner}</g>");
            }
            RenderOp::FillPath {
                path,
                color,
                transform,
            } => body.push_str(&path_element(path, *transform, &fill(*color))),
            RenderOp::FillGradient {
                path,
                gradient,
                size,
                transform,
            } => {
                let id = defs.id("g");
                gradient_def(&mut defs.body, &id, gradient, *size);
                body.push_str(&path_element(
                    path,
                    *transform,
                    &format!("fill=\"url(#{id})\""),
                ));
            }
            RenderOp::DrawImage { image, transform } => {
                let data = png_data(image)?;
                let _ = write!(
                    body,
                    "<image width=\"{}\" height=\"{}\" transform=\"{}\" href=\"data:image/png;base64,{}\"/>",
                    image.width(),
                    image.height(),
                    matrix(*transform),
                    data
                );
            }
            RenderOp::Text {
                path,
                color,
                transform,
                runs,
            } => {
                if outlines || runs.is_empty() {
                    body.push_str(&path_element(path, *transform, &fill(*color)));
                } else {
                    let _ = write!(
                        body,
                        "<g transform=\"{}\" {}>",
                        matrix(*transform),
                        fill(*color)
                    );
                    for run in runs.iter() {
                        text_run(&mut body, &mut defs, run);
                    }
                    body.push_str("</g>");
                }
            }
            RenderOp::ClipPath { path, transform } => {
                let id = defs.id("c");
                let _ = write!(
                    defs.body,
                    "<clipPath id=\"{id}\">{}</clipPath>",
                    path_element(path, *transform, "")
                );
                let inner = std::mem::take(&mut body);
                let _ = write!(body, "<g clip-path=\"url(#{id})\">{inner}</g>");
            }
            RenderOp::ApplyMask { .. }
            | RenderOp::ApplyEffect { .. }
            | RenderOp::ApplyTransition { .. } => return None,
        }
    }
    if !open.is_empty() {
        return None;
    }
    let mut svg = String::new();
    let _ = write!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">"
    );
    if !defs.body.is_empty() || !defs.fonts.is_empty() {
        svg.push_str("<defs>");
        if !defs.fonts.is_empty() {
            svg.push_str("<style>");
            let mut faces: Vec<&String> = defs.fonts.values().collect();
            faces.sort();
            for face in faces {
                svg.push_str(face);
            }
            svg.push_str("</style>");
        }
        svg.push_str(&defs.body);
        svg.push_str("</defs>");
    }
    svg.push_str(&body);
    svg.push_str("</svg>\n");
    Some(svg)
}

/// A page of pixels as an SVG of one image.
fn image_page(image: &ImageBuf, (w, h): (u32, u32)) -> Option<String> {
    let data = png_data(image)?;
    Some(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\"><image width=\"{w}\" height=\"{h}\" href=\"data:image/png;base64,{data}\"/></svg>\n"
    ))
}

/// One run of text: each cluster at its place, in its font, embedded once.
fn text_run(body: &mut String, defs: &mut Defs, run: &GlyphRun) {
    let key = Arc::as_ptr(&run.font).cast::<u8>() as usize;
    let next = defs.fonts.len();
    let family = format!("gafira-font-{next}");
    let family = match defs.fonts.get(&key) {
        Some(face) => face.split('\'').nth(1).unwrap_or(&family).to_owned(),
        None => {
            defs.fonts.insert(
                key,
                format!(
                    "@font-face{{font-family:'{family}';src:url(data:font/ttf;base64,{});}}",
                    base64(&run.font)
                ),
            );
            family
        }
    };
    let _ = write!(
        body,
        "<text font-family=\"'{family}'\" font-size=\"{}\" xml:space=\"preserve\">",
        num(f64::from(run.size))
    );
    for glyph in &run.glyphs {
        let Some(words) = run.text.get(glyph.text.clone()) else {
            continue;
        };
        if words.is_empty() {
            continue;
        }
        let _ = write!(
            body,
            "<tspan x=\"{}\" y=\"{}\">{}</tspan>",
            num(f64::from(glyph.x)),
            num(f64::from(glyph.y)),
            escape(words)
        );
    }
    body.push_str("</text>");
}

fn path_element(path: &BezPath, transform: Affine, attributes: &str) -> String {
    format!(
        "<path d=\"{}\" transform=\"{}\" {attributes}/>",
        path_data(path),
        matrix(transform)
    )
}

fn path_data(path: &BezPath) -> String {
    let mut d = String::new();
    for el in path.elements() {
        let _ = match *el {
            PathEl::MoveTo(p) => write!(d, "M{} {}", num(p.x), num(p.y)),
            PathEl::LineTo(p) => write!(d, "L{} {}", num(p.x), num(p.y)),
            PathEl::QuadTo(c, p) => {
                write!(d, "Q{} {} {} {}", num(c.x), num(c.y), num(p.x), num(p.y))
            }
            PathEl::CurveTo(a, b, p) => write!(
                d,
                "C{} {} {} {} {} {}",
                num(a.x),
                num(a.y),
                num(b.x),
                num(b.y),
                num(p.x),
                num(p.y)
            ),
            PathEl::ClosePath => write!(d, "Z"),
        };
    }
    d
}

fn matrix(t: Affine) -> String {
    let [a, b, c, d, e, f] = t.as_coeffs();
    format!(
        "matrix({} {} {} {} {} {})",
        num(a),
        num(b),
        num(c),
        num(d),
        num(e),
        num(f)
    )
}

/// A number with at most three decimals, no trailing zeros.
fn num(v: f64) -> String {
    if !v.is_finite() {
        return "0".to_owned();
    }
    let s = format!("{:.3}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".to_owned()
    } else {
        s.to_owned()
    }
}

/// The straight sRGB channels of `color`, 0 to 255.
fn srgb8(color: Color) -> [u8; 4] {
    let Color::Srgb(c) = color.convert(gafira_color::ColorSpaceId::Srgb) else {
        return [0, 0, 0, 255];
    };
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    [q(c.r), q(c.g), q(c.b), q(c.a)]
}

fn hex(color: Color) -> (String, f32) {
    let [r, g, b, a] = srgb8(color);
    (format!("#{r:02x}{g:02x}{b:02x}"), f32::from(a) / 255.0)
}

fn fill(color: Color) -> String {
    let (hex, alpha) = hex(color);
    if alpha < 1.0 {
        format!("fill=\"{hex}\" fill-opacity=\"{}\"", num(f64::from(alpha)))
    } else {
        format!("fill=\"{hex}\"")
    }
}

/// A gradient in the `size` box of the path it fills, as the canvas lays
/// it (spec 063).
fn gradient_def(out: &mut String, id: &str, gradient: &gafira_doc::Gradient, (w, h): (f64, f64)) {
    let mut stops = String::new();
    for stop in &gradient.stops {
        let (hex, alpha) = hex(stop.color);
        let _ = write!(
            stops,
            "<stop offset=\"{}\" stop-color=\"{hex}\" stop-opacity=\"{}\"/>",
            num(f64::from(stop.offset.clamp(0.0, 1.0))),
            num(f64::from(alpha))
        );
    }
    let (cx, cy) = (w / 2.0, h / 2.0);
    let _ = match gradient.kind {
        GradientKind::Linear { angle_deg } => {
            let (sin, cos) = angle_deg.to_radians().sin_cos();
            let half = (cx * cos).abs() + (cy * sin).abs();
            write!(
                out,
                "<linearGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\">{stops}</linearGradient>",
                num(cx - cos * half),
                num(cy - sin * half),
                num(cx + cos * half),
                num(cy + sin * half)
            )
        }
        GradientKind::Radial => write!(
            out,
            "<radialGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" cx=\"{}\" cy=\"{}\" r=\"{}\">{stops}</radialGradient>",
            num(cx),
            num(cy),
            num(cx.hypot(cy))
        ),
    };
}

fn blend_name(blend: BlendMode) -> &'static str {
    match blend {
        BlendMode::Normal => "normal",
        BlendMode::Multiply => "multiply",
        BlendMode::Screen => "screen",
        BlendMode::Overlay => "overlay",
        BlendMode::Darken => "darken",
        BlendMode::Lighten => "lighten",
        BlendMode::ColorDodge => "color-dodge",
        BlendMode::ColorBurn => "color-burn",
        BlendMode::HardLight => "hard-light",
        BlendMode::SoftLight => "soft-light",
        BlendMode::Difference => "difference",
        BlendMode::Exclusion => "exclusion",
        BlendMode::Hue => "hue",
        BlendMode::Saturation => "saturation",
        BlendMode::Color => "color",
        BlendMode::Luminosity => "luminosity",
    }
}

/// `image` as base64 PNG.
fn png_data(image: &ImageBuf) -> Option<String> {
    let pixels = gafira_raster::straight_srgb8(image)?;
    let rgba = image::RgbaImage::from_raw(image.width(), image.height(), pixels)?;
    let mut png = Vec::new();
    rgba.write_to(&mut io::Cursor::new(&mut png), image::ImageFormat::Png)
        .ok()?;
    Some(base64(&png))
}

/// Text escaped for XML.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Standard base64 with padding.
fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (k, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if k <= chunk.len() {
                out.push(char::from(TABLE[((n >> shift) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_and_numbers() {
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
        assert_eq!(num(1.5000), "1.5");
        assert_eq!(num(-0.0001), "0");
        assert_eq!(num(2.0), "2");
        assert_eq!(escape("a<b & \"c\""), "a&lt;b &amp; &quot;c&quot;");
    }
}
