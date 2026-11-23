//! What the raster exporters share (FD-55, specs 030 and 039): the page size
//! in pixels, the scope, the options common to every image format and the
//! loop that draws and writes one file per page.

use std::io::{self, Write};

use gafira_doc::contract::{CancelToken, ParamDef, ParamKind, ParamSchema, ParamValues, Progress};
use gafira_doc::msg;
use gafira_doc::msg::{MessageKey, ToMessage};
use gafira_doc::{Dpi, Page, PageId};
use gafira_geom::IRect;
use gafira_ports::export::{
    ExportCaps, ExportError, ExportIssue, ExportJob, ExportReport, ExportRequest, ExportScope,
    ExportSink, ExportedFile, Exporter, IssueSeverity, PageRenderRequest,
};
use gafira_ports::format::FormatDescriptor;
use gafira_ports::render::RenderError;
use gafira_raster::{BufferError, straight_srgb8};
use gafira_time::Flicks;

/// The largest side of an exported image, in pixels (D-71, FD-55).
pub const MAX_SIDE: u32 = 16_384;

/// The option id of the scale (FD-55, «Escala»).
pub const SCALE: &str = "scale";

/// The option id of the transparent background (FD-55, «Fondo transparente»).
pub const TRANSPARENT: &str = "transparent";

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
pub(crate) fn invalid(detail: &str) -> ExportError {
    ExportError::Io(io::Error::new(io::ErrorKind::InvalidInput, detail))
}

/// The scale option of every image format: 0.5× to 4×, 1× by default.
pub(crate) fn scale_param() -> ParamDef {
    ParamDef {
        id: SCALE,
        label: MessageKey("export.scale"),
        kind: ParamKind::Float {
            min: 0.5,
            max: 4.0,
            default: 1.0,
        },
        animatable: false,
    }
}

/// The transparent background option, off by default.
pub(crate) fn transparent_param() -> ParamDef {
    ParamDef {
        id: TRANSPARENT,
        label: MessageKey("export.transparent"),
        kind: ParamKind::Bool { default: false },
        animatable: false,
    }
}

/// Turns the straight 8-bit sRGB RGBA pixels of one page into the bytes of
/// one file, with the resolved options of the request.
pub(crate) type Encode = fn(u32, u32, Vec<u8>, &ParamValues) -> Result<Vec<u8>, ExportError>;

/// One raster format: one file per page of the scope, drawn at the scale of
/// the request and encoded by `encode`.
pub(crate) struct Raster {
    pub(crate) format: &'static FormatDescriptor,
    pub(crate) schema: ParamSchema,
    /// Whether the format keeps an alpha channel; without it the page is
    /// drawn on its background whatever the request says.
    pub(crate) transparency: bool,
    pub(crate) encode: Encode,
}

impl Raster {
    /// The options of `request` with every default filled in.
    fn options(&self, request: &ExportRequest) -> Result<ParamValues, ExportError> {
        self.schema
            .resolve(&request.options)
            .map_err(|_| invalid("invalid export options"))
    }

    /// The name of file `at` of `count`: `page.<ext>`, or `page-<n>.<ext>`
    /// when there are several.
    fn name(&self, at: usize, count: usize) -> String {
        let ext = self.format.extensions.first().copied().unwrap_or("img");
        if count == 1 {
            format!("page.{ext}")
        } else {
            format!("page-{}.{ext}", at + 1)
        }
    }
}

impl Exporter for Raster {
    fn descriptor(&self) -> &FormatDescriptor {
        self.format
    }

    fn capabilities(&self) -> ExportCaps {
        ExportCaps {
            transparency: self.transparency,
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
        let options = self.options(job.request)?;
        let scale = options.float(SCALE).unwrap_or(1.0);
        let transparent = self.transparency && options.bool(TRANSPARENT).unwrap_or(false);
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
            let encoded = (self.encode)(image.width(), image.height(), pixels, &options)?;
            let name = self.name(at, pages.len());
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

/// Implements [`Exporter`] for a newtype over [`Raster`] by delegation.
macro_rules! delegate_exporter {
    ($name:ident) => {
        impl gafira_ports::export::Exporter for $name {
            fn descriptor(&self) -> &gafira_ports::format::FormatDescriptor {
                gafira_ports::export::Exporter::descriptor(&self.0)
            }

            fn capabilities(&self) -> gafira_ports::export::ExportCaps {
                gafira_ports::export::Exporter::capabilities(&self.0)
            }

            fn options_schema(&self) -> &gafira_doc::contract::ParamSchema {
                gafira_ports::export::Exporter::options_schema(&self.0)
            }

            fn validate(
                &self,
                request: &gafira_ports::export::ExportRequest,
            ) -> Vec<gafira_ports::export::ExportIssue> {
                gafira_ports::export::Exporter::validate(&self.0, request)
            }

            fn export(
                &self,
                job: &gafira_ports::export::ExportJob<'_>,
                sink: &mut dyn gafira_ports::export::ExportSink,
                progress: &dyn gafira_doc::contract::Progress,
                cancel: &gafira_doc::contract::CancelToken,
            ) -> Result<gafira_ports::export::ExportReport, gafira_ports::export::ExportError> {
                gafira_ports::export::Exporter::export(&self.0, job, sink, progress, cancel)
            }
        }
    };
}

pub(crate) use delegate_exporter;
