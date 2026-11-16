//! The PNG exporter (FD-55, spec 030).

use std::io::{self, Write};

use gafira_doc::contract::{CancelToken, ParamDef, ParamKind, ParamSchema, ParamValues, Progress};
use gafira_doc::msg;
use gafira_doc::msg::MessageKey;
use gafira_doc::{Dpi, Page, PageId};
use gafira_geom::IRect;
use gafira_ports::common::LegalClass;
use gafira_ports::export::{
    ExportCaps, ExportError, ExportIssue, ExportJob, ExportReport, ExportRequest, ExportScope,
    ExportSink, ExportedFile, Exporter, IssueSeverity, PageRenderRequest,
};
use gafira_ports::format::{ByteSignature, FormatDescriptor, FormatKind};
use gafira_ports::render::RenderError;
use gafira_raster::BufferError;
use gafira_time::Flicks;

use crate::pixels::straight_srgb8;

/// The largest side of an exported image, in pixels (D-71, FD-55).
pub const MAX_SIDE: u32 = 16_384;

/// The option id of the scale (FD-55, «Escala»).
pub const SCALE: &str = "scale";

/// The option id of the transparent background (FD-55, «Fondo transparente»).
pub const TRANSPARENT: &str = "transparent";

/// The PNG format (FD-55).
pub const PNG_FORMAT: FormatDescriptor = FormatDescriptor {
    id: "png",
    extensions: &["png"],
    mime: &["image/png"],
    signatures: &[ByteSignature {
        offset: 0,
        bytes: b"\x89PNG\r\n\x1a\n",
    }],
    kind: FormatKind::Raster,
    legal: LegalClass::RoyaltyFree,
};

/// The size in whole pixels of `page` drawn `scale` times its size.
///
/// Each side is rounded and at least one pixel.
///
/// # Errors
/// A size the unit math cannot give, or a side above [`MAX_SIDE`] (D-71).
pub fn page_pixels(page: &Page, dpi: Dpi, scale: f64) -> Result<(u32, u32), ExportError> {
    let size = page
        .size
        .to_px(dpi)
        .map_err(|_| invalid("the page size is not finite"))?;
    let side = |px: f64| -> Option<u32> {
        let scaled = (px * scale).round();
        (scaled.is_finite() && scaled <= f64::from(MAX_SIDE)).then(|| (scaled as u32).max(1))
    };
    match (side(size.width()), side(size.height())) {
        (Some(width), Some(height)) => Ok((width, height)),
        _ => Err(ExportError::Render(RenderError::Buffer(
            BufferError::SizeOverflow {
                width: (size.width() * scale).min(f64::from(u32::MAX)) as u32,
                height: (size.height() * scale).min(f64::from(u32::MAX)) as u32,
            },
        ))),
    }
}

/// An export the request or the document makes impossible.
fn invalid(detail: &str) -> ExportError {
    ExportError::Io(io::Error::new(io::ErrorKind::InvalidInput, detail))
}

/// The pages of a request, or why it has none to write.
fn pages_of(request: &ExportRequest) -> Result<&[PageId], ExportIssue> {
    let blocking = |message| ExportIssue {
        severity: IssueSeverity::Error,
        message,
    };
    match &request.scope {
        ExportScope::Pages(pages) if pages.is_empty() => Err(blocking(msg!("export.no_pages"))),
        ExportScope::Pages(pages) => Ok(pages),
        // FD-57 «Selección» is not in this cut (spec 030).
        ExportScope::Nodes { .. } => Err(blocking(msg!("export.selection_unsupported"))),
    }
}

/// The PNG exporter: one 8-bit sRGB RGBA file per page of the scope, at a
/// scale of 0.5× to 4×, on the page background or transparent (FD-55).
pub struct PngExporter {
    schema: ParamSchema,
}

impl Default for PngExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl PngExporter {
    /// The exporter with the options of FD-55 this cut has.
    #[must_use]
    pub fn new() -> Self {
        Self {
            schema: ParamSchema::new(vec![
                ParamDef {
                    id: SCALE,
                    label: MessageKey("export.scale"),
                    kind: ParamKind::Float {
                        min: 0.5,
                        max: 4.0,
                        default: 1.0,
                    },
                    animatable: false,
                },
                ParamDef {
                    id: TRANSPARENT,
                    label: MessageKey("export.transparent"),
                    kind: ParamKind::Bool { default: false },
                    animatable: false,
                },
            ]),
        }
    }

    /// The options of `request` with every default filled in.
    fn options(&self, request: &ExportRequest) -> Result<ParamValues, ExportError> {
        self.schema
            .resolve(&request.options)
            .map_err(|_| invalid("invalid PNG options"))
    }
}

impl Exporter for PngExporter {
    fn descriptor(&self) -> &FormatDescriptor {
        &PNG_FORMAT
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
                message: gafira_doc::msg::ToMessage::to_message(&error),
            });
        }
        if let Err(issue) = pages_of(request) {
            issues.push(issue);
        }
        issues
    }

    fn export(
        &self,
        job: &ExportJob<'_>,
        sink: &mut dyn ExportSink,
        progress: &dyn Progress,
        cancel: &CancelToken,
    ) -> Result<ExportReport, ExportError> {
        let options = self.options(job.request)?;
        let scale = options.float(SCALE).unwrap_or(1.0);
        let transparent = options.bool(TRANSPARENT).unwrap_or(false);
        let pages = pages_of(job.request).map_err(|_| invalid("no page to export"))?;
        let mut report = ExportReport::default();
        for (at, id) in pages.iter().enumerate() {
            cancel.check()?;
            let page = job
                .document
                .page(*id)
                .ok_or_else(|| invalid("the page is not in the document"))?;
            let (width, height) = page_pixels(page, job.document.dpi(), scale)?;
            let image = job.pages.render(
                &PageRenderRequest {
                    page: *id,
                    time: Flicks::default(),
                    scale: scale as f32,
                    region: IRect::new(0, 0, width as i32, height as i32),
                    transparent,
                },
                cancel,
            )?;
            let pixels = straight_srgb8(&image).ok_or_else(|| invalid("the render is a mask"))?;
            let encoded = encode(image.width(), image.height(), &pixels)?;
            let name = if pages.len() == 1 {
                "page.png".to_owned()
            } else {
                format!("page-{}.png", at + 1)
            };
            let mut out = sink.create(&name)?;
            out.write_all(&encoded)?;
            out.flush()?;
            report.files.push(ExportedFile {
                name,
                bytes: encoded.len() as u64,
            });
            progress.report((at + 1) as f32 / pages.len() as f32);
        }
        Ok(report)
    }
}

/// The PNG bytes of straight 8-bit sRGB RGBA pixels, with the `sRGB` chunk
/// (FD-55, «Perfil sRGB incrustado»).
fn encode(width: u32, height: u32, pixels: &[u8]) -> Result<Vec<u8>, ExportError> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
    let mut writer = encoder.write_header().map_err(png_error)?;
    writer.write_image_data(pixels).map_err(png_error)?;
    writer.finish().map_err(png_error)?;
    Ok(out)
}

/// An encoder error as the I/O error of the export.
fn png_error(error: png::EncodingError) -> ExportError {
    match error {
        png::EncodingError::IoError(error) => ExportError::Io(error),
        other => ExportError::Io(io::Error::other(other.to_string())),
    }
}
