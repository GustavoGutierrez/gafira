//! «Exportar» (03 §7.5, spec 030): the small dialog of this cut, the system
//! save dialog and the engine call behind them.
//!
//! The dialog offers PNG of the current page at 1× or 2×, or PDF of every
//! page. The options of the full dialog of 03 §7.5 (preview, estimated size,
//! options built from `Exporter::options_schema`) are a later cut.

use std::path::{Path, PathBuf};

use egui::RichText;
use gafira_doc::contract::{CancelToken, NoProgress, ParamValue, ParamValues};
use gafira_doc::msg::ToMessage;
use gafira_doc::{Document, PageId};
use gafira_engine::{EngineError, Gafira};
use gafira_ports::export::{ExportRequest, ExportScope};

use crate::files::{Outcome, project_name};
use crate::style::Theme;
use crate::text::Texts;
use crate::widgets;

/// The option ids of the exporters (`gafira-io-image`, `gafira-io-pdf`).
const SCALE: &str = "scale";

/// What the person picked in the dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Choice {
    /// The current page as PNG, at 1× or 2×.
    Png { double: bool },
    /// Every page in one PDF.
    Pdf,
}

impl Default for Choice {
    fn default() -> Self {
        Self::Png { double: false }
    }
}

impl Choice {
    /// The format id of the exporter.
    fn format(self) -> &'static str {
        match self {
            Self::Png { .. } => "png",
            Self::Pdf => "pdf",
        }
    }

    /// The extension of the file, without the dot.
    pub(crate) fn extension(self) -> &'static str {
        self.format()
    }
}

/// The dialog while it is open: the choice so far and the page it exports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Dialog {
    pub(crate) choice: Choice,
    /// The page of the selected element, if any: «página actual».
    pub(crate) page: Option<PageId>,
}

/// What the person did with the dialog this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    Open,
    Cancel,
    Export,
}

/// The export request for `choice` over `document`: PNG of `page` (or the
/// first page), PDF of every page in order.
pub(crate) fn request(
    choice: Choice,
    document: &Document,
    page: Option<PageId>,
) -> Option<ExportRequest> {
    let mut options = ParamValues::new();
    let scope = match choice {
        Choice::Png { double } => {
            options.set(SCALE, ParamValue::Float(if double { 2.0 } else { 1.0 }));
            let page = page
                .filter(|id| document.page(*id).is_some())
                .or_else(|| document.pages().first().map(|page| page.id))?;
            ExportScope::Pages(vec![page])
        }
        Choice::Pdf => ExportScope::Pages(document.pages().iter().map(|page| page.id).collect()),
    };
    Some(ExportRequest {
        format_id: choice.format().to_owned(),
        scope,
        options,
    })
}

/// `path` with the extension of `choice`, added when the person typed none
/// or another one.
pub(crate) fn with_extension(path: PathBuf, choice: Choice) -> PathBuf {
    let ext = choice.extension();
    if path
        .extension()
        .is_some_and(|current| current.eq_ignore_ascii_case(ext))
    {
        path
    } else {
        let mut name = path.into_os_string();
        name.push(".");
        name.push(ext);
        PathBuf::from(name)
    }
}

/// Export the open document as `dialog` says, to `path`.
pub(crate) fn export_to(
    engine: &mut Gafira,
    dialog: Dialog,
    path: &Path,
) -> Result<(), EngineError> {
    let document = engine.edit().document().ok_or(EngineError::NoSession)?;
    let request = request(dialog.choice, &document, dialog.page).ok_or(EngineError::NoSession)?;
    engine
        .export()
        .export(&request, path, &NoProgress, &CancelToken::new())
        .map(drop)
}

/// The system save dialog, then the export.
pub(crate) fn run(engine: &mut Gafira, dialog: Dialog) -> Outcome {
    let ext = dialog.choice.extension();
    let start = engine.projects().path();
    let mut picker = rfd::FileDialog::new().add_filter(ext.to_ascii_uppercase(), &[ext]);
    if let Some(folder) = start.as_deref().and_then(Path::parent) {
        picker = picker.set_directory(folder);
    }
    let name = project_name(start.as_deref()).unwrap_or_else(|| "Gafira".to_owned());
    picker = picker.set_file_name(format!("{name}.{ext}"));
    let Some(path) = picker
        .save_file()
        .map(|path| with_extension(path, dialog.choice))
    else {
        return Outcome::Cancelled;
    };
    match export_to(engine, dialog, &path) {
        Ok(()) => Outcome::Done,
        Err(error) => Outcome::Failed(failure(&error), error.to_string()),
    }
}

/// What the status line says when an export fails.
fn failure(error: &EngineError) -> gafira_doc::msg::Message {
    match error {
        EngineError::ExportNotAllowed => gafira_doc::msg!("export.not_allowed"),
        EngineError::ExportBlocked(_) => error.to_message(),
        _ => gafira_doc::msg!("export.failed"),
    }
}

/// The dialog of this cut over the screen (03 §7.5).
pub(crate) fn show(
    ctx: &egui::Context,
    theme: &Theme,
    texts: &Texts,
    dialog: &mut Dialog,
) -> Answer {
    let mut answer = Answer::Open;
    egui::Modal::new(egui::Id::new("export")).show(ctx, |ui| {
        ui.set_width(360.0);
        ui.label(RichText::new(texts.t(&gafira_doc::msg!("export.title"))).font(theme.font("h3")));
        ui.add_space(12.0);
        ui.label(
            RichText::new(texts.t(&gafira_doc::msg!("export.format"))).font(theme.font("ui-label")),
        );
        let png = matches!(dialog.choice, Choice::Png { .. });
        if ui
            .radio(png, texts.t(&gafira_doc::msg!("export.png")))
            .clicked()
            && !png
        {
            dialog.choice = Choice::default();
        }
        if let Choice::Png { double } = &mut dialog.choice {
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(texts.t(&gafira_doc::msg!("export.scale")));
                ui.radio_value(double, false, texts.t(&gafira_doc::msg!("export.scale_1x")));
                ui.radio_value(double, true, texts.t(&gafira_doc::msg!("export.scale_2x")));
            });
        }
        if ui
            .radio(
                dialog.choice == Choice::Pdf,
                texts.t(&gafira_doc::msg!("export.pdf")),
            )
            .clicked()
        {
            dialog.choice = Choice::Pdf;
        }
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            if widgets::primary(ui, theme, texts.t(&gafira_doc::msg!("export.action"))).clicked() {
                answer = Answer::Export;
            }
            if widgets::secondary(ui, theme, texts.t(&gafira_doc::msg!("leave.cancel"))).clicked() {
                answer = Answer::Cancel;
            }
        });
    });
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        answer = Answer::Cancel;
    }
    answer
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use gafira_engine::Registries;
    use gafira_io_image::PngExporter;
    use gafira_io_pdf::PdfExporter;
    use gafira_ports::common::UnixTime;
    use gafira_ports::export::{Exporter, PageRenderers};
    use gafira_ports::system::Clock;
    use gafira_render::{FontBook, GraphPageRenderers};
    use gafira_render_cpu::CpuBackend;

    use super::*;
    use crate::editor::tests::engine;
    use crate::format::StartFormat;
    use crate::shell::start;

    struct Zero;

    impl Clock for Zero {
        fn now(&self) -> UnixTime {
            UnixTime(0)
        }

        fn monotonic(&self) -> Duration {
            Duration::ZERO
        }
    }

    /// The renderers the app composes: the graph on the CPU backend, with the
    /// fonts of the theme.
    pub(crate) fn pages() -> Arc<dyn PageRenderers> {
        let mut fonts = FontBook::new();
        for font in gafira_theme::fonts() {
            fonts = fonts.with(font.family, font.bytes);
        }
        Arc::new(GraphPageRenderers::new(
            Box::new(|| Box::new(CpuBackend::new())),
            Arc::new(Zero),
            Arc::new(fonts),
        ))
    }

    /// The registries the app composes: PNG and PDF.
    pub(crate) fn registries() -> Registries {
        let mut registries = Registries::new();
        for exporter in [
            Arc::new(PngExporter::new()) as Arc<dyn Exporter>,
            Arc::new(PdfExporter::new()),
        ] {
            registries
                .formats
                .register_exporter(exporter)
                .expect("the exporters register");
        }
        registries
    }

    // S30-07: the choices of the dialog build the requests of the engine.
    #[test]
    fn s30_07_the_dialog_choices_build_the_export_requests() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a document");
        let document = engine.edit().document().expect("a snapshot");
        let first = document.pages()[0].id;

        let png = request(Choice::Png { double: true }, &document, None).expect("a request");
        assert_eq!(png.format_id, "png");
        assert_eq!(png.scope, ExportScope::Pages(vec![first]));
        assert_eq!(png.options.float(SCALE), Some(2.0));
        let one = request(Choice::Png { double: false }, &document, Some(first)).expect("one");
        assert_eq!(one.options.float(SCALE), Some(1.0));
        // A page that went away falls back to the first one.
        let gone = request(Choice::default(), &document, Some(PageId::new(999))).expect("gone");
        assert_eq!(gone.scope, ExportScope::Pages(vec![first]));

        let pdf = request(Choice::Pdf, &document, Some(first)).expect("a request");
        assert_eq!(pdf.format_id, "pdf");
        let all: Vec<PageId> = document.pages().iter().map(|p| p.id).collect();
        assert_eq!(pdf.scope, ExportScope::Pages(all));

        assert_eq!(
            with_extension("/w/cartel".into(), Choice::Pdf),
            PathBuf::from("/w/cartel.pdf")
        );
        assert_eq!(
            with_extension("/w/cartel.PNG".into(), Choice::default()),
            PathBuf::from("/w/cartel.PNG")
        );
    }

    // S30-07: the app's engine writes a PNG and a PDF of the open document to
    // the chosen paths; the temporary files go away.
    #[test]
    fn s30_07_the_engine_of_the_app_writes_png_and_pdf() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a document");
        let dir = std::env::temp_dir().join(format!("gafira-ui-export-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let png = dir.join("cartel.png");
        let pdf = dir.join("cartel.pdf");
        let result = (|| {
            export_to(
                &mut engine,
                Dialog {
                    choice: Choice::Png { double: false },
                    page: None,
                },
                &png,
            )?;
            export_to(
                &mut engine,
                Dialog {
                    choice: Choice::Pdf,
                    page: None,
                },
                &pdf,
            )
        })();
        let png_bytes = std::fs::read(&png);
        let pdf_bytes = std::fs::read(&pdf);
        let left: Vec<_> = std::fs::read_dir(&dir)
            .map(|entries| entries.filter_map(Result::ok).map(|e| e.path()).collect())
            .unwrap_or_default();
        let _ = std::fs::remove_dir_all(&dir);
        result.expect("both exports run");
        assert!(png_bytes.expect("the PNG").starts_with(b"\x89PNG"));
        assert!(pdf_bytes.expect("the PDF").starts_with(b"%PDF-"));
        assert_eq!(left.len(), 2, "no temporary is left: {left:?}");
    }

    // S30-07: the notice of a finished export is the brand's.
    #[test]
    fn s30_07_the_notice_and_the_dialog_texts_exist() {
        let texts = Texts::new(gafira_ports::i18n::Locale::Es);
        assert_eq!(
            texts.t(&gafira_doc::msg!("export.done")),
            "Tu diseño está listo para compartir"
        );
        assert_eq!(
            failure(&EngineError::ExportNotAllowed),
            gafira_doc::msg!("export.not_allowed")
        );
        assert_eq!(
            failure(&EngineError::UnknownFormat),
            gafira_doc::msg!("export.failed")
        );
    }
}
