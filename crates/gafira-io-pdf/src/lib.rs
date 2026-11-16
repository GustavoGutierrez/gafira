//! Layer-4 adapter `gafira-io-pdf` (02 §2.1): the PDF exporter (FD-56).
//!
//! This first cut writes one PDF page per page of the scope, of the page's
//! size in points, with the page drawn as one image («Aplanar», spec 030).
//! The pixels come from the `PageRenderer` of the `ExportJob`; the exporter
//! never sees `gafira-render` (ARQ-06). Vector content, selectable text,
//! bleed, crop marks, presets and CMYK are later cuts.

#![forbid(unsafe_code)]

use std::io::{self, Write};

use gafira_doc::contract::{CancelToken, ParamDef, ParamKind, ParamSchema, ParamValues, Progress};
use gafira_doc::msg;
use gafira_doc::msg::{MessageKey, ToMessage};
use gafira_doc::{Dpi, Page, PageId};
use gafira_geom::IRect;
use gafira_io_image::{page_pixels, straight_srgb8};
use gafira_ports::common::LegalClass;
use gafira_ports::export::{
    ExportCaps, ExportError, ExportIssue, ExportJob, ExportReport, ExportRequest, ExportScope,
    ExportSink, ExportedFile, Exporter, IssueSeverity, PageRenderRequest,
};
use gafira_ports::format::{ByteSignature, FormatDescriptor, FormatKind};
use gafira_time::Flicks;

/// The option id of the resolution the pages are drawn at, as a multiple of
/// their size in px.
pub const SCALE: &str = "scale";

/// The name of the one file the exporter writes.
pub const FILE_NAME: &str = "document.pdf";

/// The PDF format (FD-56).
pub const PDF_FORMAT: FormatDescriptor = FormatDescriptor {
    id: "pdf",
    extensions: &["pdf"],
    mime: &["application/pdf"],
    signatures: &[ByteSignature {
        offset: 0,
        bytes: b"%PDF-",
    }],
    kind: FormatKind::Document,
    legal: LegalClass::RoyaltyFree,
};

/// The size of `page` in PDF points (1/72 in): its size in px at the
/// document's `dpi`.
///
/// # Errors
/// A size the unit math cannot give.
pub fn page_points(page: &Page, dpi: Dpi) -> Result<(f32, f32), ExportError> {
    let px = page
        .size
        .to_px(dpi)
        .map_err(|_| invalid("the page size is not finite"))?;
    let per_px = 72.0 / f64::from(dpi.get());
    Ok(((px.width() * per_px) as f32, (px.height() * per_px) as f32))
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
        ExportScope::Nodes { .. } => Err(blocking(msg!("export.selection_unsupported"))),
    }
}

/// The PDF exporter: every page of the scope, in order, in one file.
pub struct PdfExporter {
    schema: ParamSchema,
}

impl Default for PdfExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfExporter {
    /// The exporter, drawing the pages at 2× by default.
    #[must_use]
    pub fn new() -> Self {
        Self {
            schema: ParamSchema::new(vec![ParamDef {
                id: SCALE,
                label: MessageKey("export.resolution"),
                kind: ParamKind::Float {
                    min: 1.0,
                    max: 4.0,
                    default: 2.0,
                },
                animatable: false,
            }]),
        }
    }

    /// The options of `request` with every default filled in.
    fn options(&self, request: &ExportRequest) -> Result<ParamValues, ExportError> {
        self.schema
            .resolve(&request.options)
            .map_err(|_| invalid("invalid PDF options"))
    }
}

impl Exporter for PdfExporter {
    fn descriptor(&self) -> &FormatDescriptor {
        &PDF_FORMAT
    }

    fn capabilities(&self) -> ExportCaps {
        ExportCaps {
            multipage: true,
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
        let scale = options.float(SCALE).unwrap_or(2.0);
        let pages = pages_of(job.request).map_err(|_| invalid("no page to export"))?;
        let dpi = job.document.dpi();
        let mut pdf = krilla::Document::new();
        for (at, id) in pages.iter().enumerate() {
            cancel.check()?;
            let page = job
                .document
                .page(*id)
                .ok_or_else(|| invalid("the page is not in the document"))?;
            let (width, height) = page_pixels(page, dpi, scale)?;
            let image = job.pages.render(
                &PageRenderRequest {
                    page: *id,
                    time: Flicks::default(),
                    scale: scale as f32,
                    region: IRect::new(0, 0, width as i32, height as i32),
                    transparent: false,
                },
                cancel,
            )?;
            let pixels = straight_srgb8(&image).ok_or_else(|| invalid("the render is a mask"))?;
            let (points_w, points_h) = page_points(page, dpi)?;
            let settings = krilla::page::PageSettings::from_wh(points_w, points_h)
                .ok_or_else(|| invalid("the page has no area"))?;
            let size = krilla::geom::Size::from_wh(points_w, points_h)
                .ok_or_else(|| invalid("the page has no area"))?;
            let bitmap = krilla::image::Image::from_rgba8(pixels, image.width(), image.height());
            let mut pdf_page = pdf.start_page_with(settings);
            let mut surface = pdf_page.surface();
            surface.draw_image(bitmap, size);
            surface.finish();
            pdf_page.finish();
            progress.report((at + 1) as f32 / (pages.len() + 1) as f32);
        }
        let bytes = pdf
            .finish()
            .map_err(|error| ExportError::Io(io::Error::other(format!("{error:?}"))))?;
        let mut out = sink.create(FILE_NAME)?;
        out.write_all(&bytes)?;
        out.flush()?;
        progress.report(1.0);
        Ok(ExportReport {
            files: vec![ExportedFile {
                name: FILE_NAME.to_owned(),
                bytes: bytes.len() as u64,
            }],
            warnings: Vec::new(),
        })
    }
}
