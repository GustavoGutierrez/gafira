//! Layer-4 adapter `gafira-io-xlsx` (02 §2.1): the Excel exporter of the
//! tables (spec 081), written from the public ECMA-376 standard
//! (SpreadsheetML).
//!
//! A sheet a table: words as words, numbers as numbers, the merges, the
//! sizes, the fills, the borders and the fonts. Nothing a cell says becomes
//! a formula.
//!
//! The column letters, the stored column widths, the `_xHHHH_` escapes and
//! the interning of the styles are adapted from a project under MIT OR
//! Apache-2.0; its notice is in `THIRD_PARTY_NOTICES.md`.

#![forbid(unsafe_code)]

mod csv;
mod look;
mod number;
mod package;
mod read;
mod sheet;
mod styles;
mod xml;

use std::collections::BTreeSet;
use std::io::{self, Cursor, Write};

use gafira_doc::contract::{CancelToken, ParamSchema, Progress};
use gafira_doc::msg;
use gafira_doc::msg::ToMessage;
use gafira_doc::{Document, Node, NodeKind, Page, PageId, TableNode};
use gafira_ports::common::LegalClass;
use gafira_ports::export::{
    ExportCaps, ExportError, ExportIssue, ExportJob, ExportReport, ExportRequest, ExportScope,
    ExportSink, ExportedFile, Exporter, IssueSeverity,
};
use gafira_ports::format::{ByteSignature, FormatDescriptor, FormatKind};

pub use csv::read_csv;
pub use number::{Locale, show};
pub use read::{MAX_DATA_ROWS, ReadError, ReadSheet, read};
use styles::Styles;
use xml::escape;

/// The name of the one file the exporter writes.
pub const FILE_NAME: &str = "tables.xlsx";

/// The Excel format.
pub const XLSX_FORMAT: FormatDescriptor = FormatDescriptor {
    id: "xlsx",
    extensions: &["xlsx"],
    mime: &["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
    signatures: &[ByteSignature {
        offset: 0,
        bytes: b"PK\x03\x04",
    }],
    kind: FormatKind::Document,
    legal: LegalClass::RoyaltyFree,
};

/// A table and the sheet it goes to.
pub struct Sheet<'a> {
    /// The name asked for; Excel's limits are kept.
    pub name: &'a str,
    pub table: &'a TableNode,
    /// The box of the table, in px of the document.
    pub size: (f64, f64),
    /// The px of the document in an inch.
    pub dpi: f64,
}

/// The tables of `page`, back to front, in the order of its layers, those
/// inside groups too and the hidden ones not.
#[must_use]
pub fn tables_of(page: &Page) -> Vec<&Node> {
    fn visit<'a>(page: &'a Page, id: gafira_doc::NodeId, depth: usize, out: &mut Vec<&'a Node>) {
        let Some(node) = page.node(id) else { return };
        // Deeper than any valid document: a broken tree ends.
        if !node.base.visible || depth > 4096 {
            return;
        }
        if matches!(node.kind, NodeKind::Table(_)) {
            out.push(node);
        }
        for child in &node.children {
            visit(page, *child, depth + 1, out);
        }
    }
    let mut out = Vec::new();
    for root in &page.roots {
        visit(page, *root, 0, &mut out);
    }
    out
}

/// Whether `pages` of `document` hold a table to export.
#[must_use]
pub fn has_tables(document: &Document, pages: &[PageId]) -> bool {
    pages
        .iter()
        .filter_map(|id| document.page(*id))
        .any(|page| !tables_of(page).is_empty())
}

/// The workbook of `sheets`; an error when there is none.
pub fn workbook(sheets: &[Sheet<'_>]) -> io::Result<Vec<u8>> {
    if sheets.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no table to export",
        ));
    }
    let names = sheet_names(sheets.iter().map(|s| s.name));
    let mut styles = Styles::default();
    let parts: Vec<String> = sheets
        .iter()
        .map(|sheet| sheet::write(sheet, &mut styles))
        .collect();
    let mut types = String::new();
    let mut listed = String::new();
    let mut rels = String::new();
    for (i, name) in names.iter().enumerate() {
        let n = i + 1;
        types.push_str(&format!(
            "<Override PartName=\"/xl/worksheets/sheet{n}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>"
        ));
        listed.push_str(&format!(
            "<sheet name=\"{}\" sheetId=\"{n}\" r:id=\"rId{n}\"/>",
            escape(name)
        ));
        rels.push_str(&format!(
            "<Relationship Id=\"rId{n}\" Type=\"{REL}/worksheet\" Target=\"worksheets/sheet{n}.xml\"/>"
        ));
    }
    rels.push_str(&format!(
        "<Relationship Id=\"rId{}\" Type=\"{REL}/styles\" Target=\"styles.xml\"/>",
        names.len() + 1
    ));
    let mut buffer = Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut buffer);
    // A fixed date: the same tables give the same bytes.
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default());
    let mut put = |name: &str, text: &str| -> io::Result<()> {
        zip.start_file(name, options).map_err(io::Error::other)?;
        zip.write_all(text.as_bytes())
    };
    put(
        "[Content_Types].xml",
        &format!(
            "{DECL}<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/>{types}<Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/></Types>"
        ),
    )?;
    put(
        "_rels/.rels",
        &format!(
            "{DECL}<Relationships xmlns=\"{PACKAGE_REL}\"><Relationship Id=\"rId1\" Type=\"{REL}/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>"
        ),
    )?;
    put(
        "xl/workbook.xml",
        &format!(
            "{DECL}<workbook xmlns=\"{MAIN}\" xmlns:r=\"{REL}\"><sheets>{listed}</sheets></workbook>"
        ),
    )?;
    put(
        "xl/_rels/workbook.xml.rels",
        &format!("{DECL}<Relationships xmlns=\"{PACKAGE_REL}\">{rels}</Relationships>"),
    )?;
    put("xl/styles.xml", &styles.xml())?;
    for (i, part) in parts.iter().enumerate() {
        put(&format!("xl/worksheets/sheet{}.xml", i + 1), part)?;
    }
    zip.finish().map_err(io::Error::other)?;
    Ok(buffer.into_inner())
}

pub(crate) const DECL: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n";
pub(crate) const MAIN: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const PACKAGE_REL: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

/// The longest name of a sheet, in UTF-16 units.
const MAX_NAME: usize = 31;

/// The names Excel takes for the sheets asked for as `names`: none of its
/// reserved characters, at most 31 long, no two alike but for their case.
fn sheet_names<'a>(names: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut taken: BTreeSet<String> = BTreeSet::new();
    let mut out = Vec::new();
    for asked in names {
        let clean: String = asked
            .chars()
            .map(|c| {
                if c.is_control() || "/\\?*[]:".contains(c) {
                    ' '
                } else {
                    c
                }
            })
            .collect();
        let clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
        let clean = clean.trim_matches('\'').trim();
        let base = if clean.is_empty() { "Tabla" } else { clean };
        let mut name = cut(base, MAX_NAME);
        let mut n = 1;
        while !taken.insert(name.to_lowercase()) {
            n += 1;
            let tail = format!(" {n}");
            name = format!("{}{tail}", cut(base, MAX_NAME - tail.len()));
        }
        out.push(name);
    }
    out
}

/// `text` cut to `most` UTF-16 units, between characters.
fn cut(text: &str, most: usize) -> String {
    let mut length = 0;
    text.chars()
        .take_while(|c| {
            length += c.len_utf16();
            length <= most
        })
        .collect::<String>()
        .trim_end()
        .to_owned()
}

/// The Excel exporter: the tables of the pages of the scope, a sheet each.
pub struct XlsxExporter {
    schema: ParamSchema,
}

impl Default for XlsxExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl XlsxExporter {
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

impl Exporter for XlsxExporter {
    fn descriptor(&self) -> &FormatDescriptor {
        &XLSX_FORMAT
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
        let dpi = f64::from(job.document.dpi().get());
        let mut sheets = Vec::new();
        for id in ids {
            cancel.check()?;
            let page = job
                .document
                .page(*id)
                .ok_or_else(|| invalid("the page is not in the document"))?;
            for node in tables_of(page) {
                if let NodeKind::Table(table) = &node.kind {
                    let p = &node.base.placement;
                    sheets.push(Sheet {
                        name: node.base.name.as_deref().unwrap_or(""),
                        table,
                        size: (p.w.abs(), p.h.abs()),
                        dpi,
                    });
                }
            }
        }
        let bytes = workbook(&sheets)?;
        cancel.check()?;
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
