//! Layer-4 adapter `gafira-io-pptx` (02 §2.1): the PowerPoint exporter
//! (spec 075), written from the public ECMA-376 standard
//! (PresentationML, DrawingML).
//!
//! One slide per page of the scope, editable in PowerPoint, Keynote or
//! LibreOffice: shapes as shapes (their geometry, fill, gradient, outline,
//! turn and flip), text as text boxes (font, size, colour, bold, italic,
//! alignment, letter spacing, line height; a curved text with the arch
//! warp), images and frame photos as pictures with the original file and
//! the frame's crop, lines as connectors with their ends, groups as groups,
//! and the page background. Fonts are not embedded: the slide names them.

#![forbid(unsafe_code)]

mod xml;

use std::collections::HashMap;
use std::io::{self, Read, Write};

use gafira_doc::contract::{CancelToken, ParamSchema, Progress};
use gafira_doc::msg;
use gafira_doc::msg::ToMessage;
use gafira_doc::{AssetRef, Document, ImageNode, Page, PageId};
use gafira_ports::common::LegalClass;
use gafira_ports::export::{
    ExportCaps, ExportError, ExportIssue, ExportJob, ExportReport, ExportRequest, ExportScope,
    ExportSink, ExportedFile, Exporter, IssueSeverity,
};
use gafira_ports::export::{ImageDecoder, decode_image};
use gafira_ports::format::{ByteSignature, FormatDescriptor, FormatKind};
use gafira_ports::store::ContentStore;

/// The name of the one file the exporter writes.
pub const FILE_NAME: &str = "presentation.pptx";

/// The PowerPoint format.
pub const PPTX_FORMAT: FormatDescriptor = FormatDescriptor {
    id: "pptx",
    extensions: &["pptx"],
    mime: &["application/vnd.openxmlformats-officedocument.presentationml.presentation"],
    signatures: &[ByteSignature {
        offset: 0,
        bytes: b"PK\x03\x04",
    }],
    kind: FormatKind::Document,
    legal: LegalClass::RoyaltyFree,
};

/// The PowerPoint exporter: every page of the scope as one slide.
pub struct PptxExporter {
    schema: ParamSchema,
}

impl Default for PptxExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl PptxExporter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            schema: ParamSchema::new(Vec::new()),
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

impl Exporter for PptxExporter {
    fn descriptor(&self) -> &FormatDescriptor {
        &PPTX_FORMAT
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
        let ids = pages_of(job.request).map_err(|_| invalid("no page to export"))?;
        let pages: Vec<&Page> = ids
            .iter()
            .map(|id| {
                job.document
                    .page(*id)
                    .ok_or_else(|| invalid("the page is not in the document"))
            })
            .collect::<Result<_, _>>()?;
        let mut media = Media::new(job.store);
        let mut slides = Vec::new();
        let mut skipped = false;
        for (at, page) in pages.iter().enumerate() {
            cancel.check()?;
            let slide = xml::slide(job.document, page, &mut media);
            skipped |= slide.skipped;
            slides.push(slide);
            progress.report((at + 1) as f32 / (pages.len() + 1) as f32);
        }
        let first = pages.first().ok_or_else(|| invalid("no page to export"))?;
        let size = xml::slide_size(job.document, first)?;
        let bytes = package(&slides, &media, size)?;
        let mut out = sink.create(FILE_NAME)?;
        out.write_all(&bytes)?;
        out.flush()?;
        progress.report(1.0);
        Ok(ExportReport {
            files: vec![ExportedFile {
                name: FILE_NAME.to_owned(),
                bytes: bytes.len() as u64,
            }],
            warnings: if skipped {
                vec![msg!("export.pptx_some_skipped")]
            } else {
                Vec::new()
            },
        })
    }
}

/// `image` as a PNG.
fn png_of(image: &gafira_raster::ImageBuf) -> Option<Vec<u8>> {
    let pixels = gafira_raster::straight_srgb8(image)?;
    let rgba = image::RgbaImage::from_raw(image.width(), image.height(), pixels)?;
    let mut png = Vec::new();
    rgba.write_to(&mut io::Cursor::new(&mut png), image::ImageFormat::Png)
        .ok()?;
    Some(png)
}

/// The pictures of the presentation: each asset once, as PNG or JPEG.
pub(crate) struct Media<'a> {
    store: &'a dyn ContentStore,
    /// Each asset: its file name in `ppt/media/` and its bytes.
    files: Vec<(String, Vec<u8>)>,
    by_asset: HashMap<AssetRef, Option<usize>>,
    /// The size of the picture of each image with a look (spec 085).
    pub(crate) looked: HashMap<AssetRef, (u32, u32)>,
}

impl<'a> Media<'a> {
    fn new(store: &'a dyn ContentStore) -> Self {
        Self {
            store,
            files: Vec::new(),
            by_asset: HashMap::new(),
            looked: HashMap::new(),
        }
    }

    /// Draws `image` with its look, the first time, as the PNG [`Self::name`]
    /// gives for what it shows; `None` when it cannot be drawn.
    pub(crate) fn look(&mut self, image: &ImageNode) -> Option<()> {
        let shown = image.shown();
        if self.by_asset.contains_key(&shown) {
            return Some(());
        }
        let drawn = decode_image(
            image,
            self.store,
            &gafira_io_image::RasterDecoder::new(),
            None,
        )
        .ok()?;
        let png = png_of(&drawn)?;
        self.looked.insert(shown, (drawn.width(), drawn.height()));
        self.files
            .push((format!("image{}.png", self.files.len() + 1), png));
        self.by_asset.insert(shown, Some(self.files.len() - 1));
        Some(())
    }

    /// The file name of `asset` in `ppt/media/`, reading it the first time;
    /// `None` when it cannot be read.
    pub(crate) fn name(&mut self, asset: AssetRef) -> Option<String> {
        if let Some(found) = self.by_asset.get(&asset) {
            return found.map(|i| self.files[i].0.clone());
        }
        let added = self.read(asset).map(|(ext, bytes)| {
            let name = format!("image{}.{ext}", self.files.len() + 1);
            self.files.push((name, bytes));
            self.files.len() - 1
        });
        self.by_asset.insert(asset, added);
        added.map(|i| self.files[i].0.clone())
    }

    /// The bytes of `asset` as they are when PNG or JPEG, or as PNG.
    fn read(&self, asset: AssetRef) -> Option<(&'static str, Vec<u8>)> {
        let mut bytes = Vec::new();
        self.store
            .open(asset.as_hash())
            .ok()?
            .read_to_end(&mut bytes)
            .ok()?;
        if bytes.starts_with(b"\x89PNG") {
            return Some(("png", bytes));
        }
        if bytes.starts_with(&[0xFF, 0xD8]) {
            return Some(("jpeg", bytes));
        }
        // Another format (WebP…): decoded and written as PNG.
        let decoded = gafira_io_image::RasterDecoder::new().decode(&bytes).ok()?;
        Some(("png", png_of(&decoded)?))
    }
}

/// The OPC package: the parts of a presentation in a ZIP.
fn package(
    slides: &[xml::Slide],
    media: &Media<'_>,
    size: (i64, i64),
) -> Result<Vec<u8>, ExportError> {
    use zip::write::SimpleFileOptions;
    let failed = |e: zip::result::ZipError| ExportError::Io(io::Error::other(e.to_string()));
    let mut out = io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut out);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut put = |name: &str, bytes: &[u8]| -> Result<(), ExportError> {
        zip.start_file(name, options).map_err(failed)?;
        zip.write_all(bytes)?;
        Ok(())
    };
    put(
        "[Content_Types].xml",
        xml::content_types(slides.len(), media).as_bytes(),
    )?;
    put("_rels/.rels", xml::ROOT_RELS.as_bytes())?;
    put("docProps/app.xml", xml::app(slides.len()).as_bytes())?;
    put("docProps/core.xml", xml::CORE.as_bytes())?;
    put(
        "ppt/presentation.xml",
        xml::presentation(slides.len(), size).as_bytes(),
    )?;
    put(
        "ppt/_rels/presentation.xml.rels",
        xml::presentation_rels(slides.len()).as_bytes(),
    )?;
    put("ppt/presProps.xml", xml::PRES_PROPS.as_bytes())?;
    put("ppt/viewProps.xml", xml::VIEW_PROPS.as_bytes())?;
    put("ppt/tableStyles.xml", xml::TABLE_STYLES.as_bytes())?;
    put("ppt/theme/theme1.xml", xml::THEME.as_bytes())?;
    put("ppt/slideMasters/slideMaster1.xml", xml::MASTER.as_bytes())?;
    put(
        "ppt/slideMasters/_rels/slideMaster1.xml.rels",
        xml::MASTER_RELS.as_bytes(),
    )?;
    put("ppt/slideLayouts/slideLayout1.xml", xml::LAYOUT.as_bytes())?;
    put(
        "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
        xml::LAYOUT_RELS.as_bytes(),
    )?;
    for (at, slide) in slides.iter().enumerate() {
        put(
            &format!("ppt/slides/slide{}.xml", at + 1),
            slide.xml.as_bytes(),
        )?;
        put(
            &format!("ppt/slides/_rels/slide{}.xml.rels", at + 1),
            xml::slide_rels(&slide.pictures).as_bytes(),
        )?;
    }
    for (name, bytes) in &media.files {
        put(&format!("ppt/media/{name}"), bytes)?;
    }
    zip.finish().map_err(failed)?;
    Ok(out.into_inner())
}

/// Whether `document` keeps every page the same size (a presentation has
/// one slide size).
#[must_use]
pub fn same_size(document: &Document, pages: &[PageId]) -> bool {
    let sizes: Vec<_> = pages
        .iter()
        .filter_map(|id| document.page(*id))
        .filter_map(|page| page.size.to_px(document.dpi()).ok())
        .map(|size| {
            (
                (size.width() * 10.0).round(),
                (size.height() * 10.0).round(),
            )
        })
        .collect();
    sizes.windows(2).all(|w| w[0] == w[1])
}
