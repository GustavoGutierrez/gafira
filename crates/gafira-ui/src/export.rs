//! «Exportar» (03 §7.5, specs 030, 035 and 039): the dialog, the system
//! save dialog and the engine call behind them, the export on its thread
//! and the card that offers «Mostrar en carpeta» when it ends.
//!
//! The dialog lists the formats of the registry and builds their options
//! from `Exporter::options_schema`: the scale in steps of 1× to 4× within
//! the range of the schema, a check box per boolean option and a slider per
//! whole number (the JPEG quality). The pages are all of them, the current
//! one or a range such as `1-3, 5`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;

use egui::{Color32, RichText};
use gafira_doc::contract::{
    CancelToken, ParamDef, ParamKind, ParamSchema, ParamValue, ParamValues, Progress,
};
use gafira_doc::msg::{Message, MsgArg, ToMessage};
use gafira_doc::{Document, Page, PageId};
use gafira_engine::{EngineError, Gafira, PreparedExport};
use gafira_ports::export::{ExportError, ExportRequest, ExportScope, Exporter};

use crate::files::{Outcome, project_name};
use crate::style::Theme;
use crate::text::Texts;
use crate::widgets;

/// The option ids the dialog draws in its own way (`gafira-io-image`,
/// `gafira-io-pdf`): the scale as steps, the background over the preview.
const SCALE: &str = "scale";
const TRANSPARENT: &str = "transparent";

/// The scale steps the dialog offers, kept to the range of the schema.
const STEPS: [(f64, &str); 4] = [
    (1.0, "export.scale_1x"),
    (2.0, "export.scale_2x"),
    (3.0, "export.scale_3x"),
    (4.0, "export.scale_4x"),
];

/// One format of the dialog, as its exporter describes it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Format {
    /// The format id of the exporter.
    pub(crate) id: &'static str,
    /// The extension of the file, without the dot.
    pub(crate) extension: &'static str,
    /// One file for every page (PDF), not one file per page.
    pub(crate) multipage: bool,
    pub(crate) schema: ParamSchema,
}

impl Format {
    fn of(exporter: &dyn Exporter) -> Self {
        let descriptor = exporter.descriptor();
        Self {
            id: descriptor.id,
            extension: descriptor
                .extensions
                .first()
                .copied()
                .unwrap_or(descriptor.id),
            multipage: exporter.capabilities().multipage,
            schema: exporter.options_schema().clone(),
        }
    }
}

/// The formats the engine can write, in the order of the registry.
pub(crate) fn formats(engine: &Gafira) -> Vec<Format> {
    engine
        .export()
        .exporters()
        .map(|exporter| Format::of(exporter.as_ref()))
        .collect()
}

/// Which pages go out (FD-57).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pages {
    All,
    Current,
    /// The text of [`Dialog::range`].
    Range,
}

/// What the preview shows and the texture that shows it.
struct Preview {
    page: PageId,
    transparent: bool,
    /// `None` when the page could not be drawn.
    texture: Option<egui::TextureHandle>,
}

/// The dialog while it is open.
pub(crate) struct Dialog {
    pub(crate) formats: Vec<Format>,
    /// The index of the chosen format in `formats`.
    format: usize,
    /// The options of the chosen format, from the defaults of its schema.
    pub(crate) values: ParamValues,
    pub(crate) pages: Pages,
    /// The range as the person types it: `1-3, 5`.
    pub(crate) range: String,
    /// Whether the person chose the pages; until then they follow the
    /// format (all for a PDF, the current one for an image).
    chose_pages: bool,
    /// The page of the selected element, if any: «Página actual».
    pub(crate) page: Option<PageId>,
    preview: Option<Preview>,
}

impl Dialog {
    /// The dialog on the first format, at its defaults.
    pub(crate) fn new(formats: Vec<Format>, page: Option<PageId>) -> Self {
        let mut dialog = Self {
            formats,
            format: 0,
            values: ParamValues::new(),
            pages: Pages::Current,
            range: String::new(),
            chose_pages: false,
            page,
            preview: None,
        };
        dialog.pick(0);
        dialog
    }

    /// The chosen format, if the registry has any.
    pub(crate) fn format(&self) -> Option<&Format> {
        self.formats.get(self.format)
    }

    /// Choose format `index`: its options go back to their defaults, and the
    /// pages follow it until the person chose them.
    pub(crate) fn pick(&mut self, index: usize) {
        let Some(format) = self.formats.get(index) else {
            return;
        };
        self.format = index;
        self.values = format.schema.defaults();
        if !self.chose_pages {
            self.pages = if format.multipage {
                Pages::All
            } else {
                Pages::Current
            };
        }
    }

    /// Choose the pages.
    pub(crate) fn choose_pages(&mut self, pages: Pages) {
        self.pages = pages;
        self.chose_pages = true;
    }

    /// Whether the chosen format draws the page without its background.
    fn transparent(&self) -> bool {
        self.format()
            .is_some_and(|format| format.schema.get(TRANSPARENT).is_some())
            && self.values.bool(TRANSPARENT).unwrap_or(false)
    }
}

/// Why a range of pages is refused (FD-57).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RangeError {
    /// Nothing but spaces and commas.
    Empty,
    /// A part that is not a number or `a-b`.
    Syntax,
    /// A page 0, past the last page, or a range that ends before it starts.
    OutOfRange,
}

/// The pages of `text`, such as `1-3, 5`, for a document of `count` pages:
/// their indices from 0, in document order, each once (FD-57 (a)).
pub(crate) fn parse_range(text: &str, count: usize) -> Result<Vec<usize>, RangeError> {
    let number = |part: &str| -> Result<usize, RangeError> {
        let part = part.trim();
        if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(RangeError::Syntax);
        }
        // Only digits: a number too large for `usize` is past the last page.
        let page = part.parse::<usize>().unwrap_or(usize::MAX);
        if page == 0 || page > count {
            return Err(RangeError::OutOfRange);
        }
        Ok(page - 1)
    };
    let mut pages = BTreeSet::new();
    for part in text.split(',').filter(|part| !part.trim().is_empty()) {
        match part.split_once('-') {
            Some((first, last)) => {
                let (first, last) = (number(first)?, number(last)?);
                if first > last {
                    return Err(RangeError::OutOfRange);
                }
                pages.extend(first..=last);
            }
            None => {
                pages.insert(number(part)?);
            }
        }
    }
    if pages.is_empty() {
        return Err(RangeError::Empty);
    }
    Ok(pages.into_iter().collect())
}

/// Why the dialog cannot build a request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// No format, or a document with no page.
    Nothing,
    Range(RangeError),
}

/// The page «Página actual» means: the page of the selection when it is
/// still there, else the first one.
fn current(document: &Document, page: Option<PageId>) -> Option<PageId> {
    page.filter(|id| document.page(*id).is_some())
        .or_else(|| document.pages().first().map(|page| page.id))
}

/// The pages of the request `dialog` builds over `document`, in document
/// order.
pub(crate) fn scope(dialog: &Dialog, document: &Document) -> Result<Vec<PageId>, Refusal> {
    let all = || document.pages().iter().map(|page| page.id);
    let pages: Vec<PageId> = match dialog.pages {
        Pages::All => all().collect(),
        Pages::Current => current(document, dialog.page).into_iter().collect(),
        Pages::Range => parse_range(&dialog.range, document.pages().len())
            .map_err(Refusal::Range)?
            .into_iter()
            .filter_map(|index| document.pages().get(index).map(|page| page.id))
            .collect(),
    };
    // A hidden page is not exported (FD-02 b); a range still counts it.
    let pages: Vec<PageId> = pages
        .into_iter()
        .filter(|id| document.page(*id).is_some_and(|page| !page.hidden))
        .collect();
    if pages.is_empty() {
        return Err(Refusal::Nothing);
    }
    Ok(pages)
}

/// The export request of `dialog` over `document`: its format, its pages
/// and the options of the schema of the format, and nothing else (S39-04).
pub(crate) fn request(dialog: &Dialog, document: &Document) -> Result<ExportRequest, Refusal> {
    let format = dialog.format().ok_or(Refusal::Nothing)?;
    let pages = scope(dialog, document)?;
    let mut options = ParamValues::new();
    for def in format.schema.params() {
        if let Some(value) = dialog.values.get(def.id) {
            options.set(def.id, value.clone());
        }
    }
    Ok(ExportRequest {
        format_id: format.id.to_owned(),
        scope: ExportScope::Pages(pages),
        options,
    })
}

/// The size in whole pixels of `page` at `scale`, each side rounded and at
/// least one pixel, as the image exporters compute it (S39-06).
pub(crate) fn pixels(page: &Page, document: &Document, scale: f64) -> Option<(u64, u64)> {
    let size = page.size.to_px(document.dpi()).ok()?;
    let side = |px: f64| {
        let scaled = (px * scale).round();
        // A side past `u64` is far past every limit; it reads as unknown.
        (scaled.is_finite() && scaled < u64::MAX as f64).then(|| (scaled as u64).max(1))
    };
    Some((side(size.width())?, side(size.height())?))
}

/// `path` with the extension of `format`, added when the person typed none
/// or another one. `jpeg` stands for `jpg`.
pub(crate) fn with_extension(path: PathBuf, format: &Format) -> PathBuf {
    let ext = format.extension;
    let same = |current: &std::ffi::OsStr| {
        current.eq_ignore_ascii_case(ext) || (ext == "jpg" && current.eq_ignore_ascii_case("jpeg"))
    };
    if path.extension().is_some_and(same) {
        path
    } else {
        let mut name = path.into_os_string();
        name.push(".");
        name.push(ext);
        PathBuf::from(name)
    }
}

/// Export the open document as `dialog` says, to `path`, on this thread.
#[cfg(test)]
pub(crate) fn export_to(
    engine: &mut Gafira,
    dialog: &Dialog,
    path: &Path,
) -> Result<(), EngineError> {
    let document = engine.edit().document().ok_or(EngineError::NoSession)?;
    let request = request(dialog, &document).map_err(|_| EngineError::NoSession)?;
    engine
        .export()
        .export(
            &request,
            path,
            &gafira_doc::contract::NoProgress,
            &CancelToken::new(),
        )
        .map(drop)
}

/// The progress of the export thread, read by the window each frame; every
/// report wakes the window (spec 035).
pub(crate) struct Fraction {
    bits: AtomicU32,
    wake: Box<dyn Fn() + Send + Sync>,
}

impl Fraction {
    fn new(wake: Box<dyn Fn() + Send + Sync>) -> Self {
        Self {
            bits: AtomicU32::new(0.0_f32.to_bits()),
            wake,
        }
    }

    /// The last fraction reported, in `[0, 1]`.
    pub(crate) fn get(&self) -> f32 {
        f32::from_bits(self.bits.load(Ordering::Relaxed))
    }
}

impl Progress for Fraction {
    fn report(&self, fraction: f32) {
        let fraction = if fraction.is_finite() {
            fraction.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.bits.store(fraction.to_bits(), Ordering::Relaxed);
        (self.wake)();
    }
}

/// An export running on its thread (spec 035): the window keeps drawing and
/// answering while it runs.
pub(crate) struct Running {
    receiver: mpsc::Receiver<Result<(), EngineError>>,
    progress: Arc<Fraction>,
    cancel: CancelToken,
    thread: Option<JoinHandle<()>>,
    /// The first file the export writes.
    path: PathBuf,
}

impl Running {
    /// The fraction done, in `[0, 1]`.
    pub(crate) fn fraction(&self) -> f32 {
        self.progress.get()
    }

    /// The first file the export writes, for «Mostrar en carpeta».
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Ask the export to stop; it ends as [`Outcome::Cancelled`] and leaves
    /// no file.
    pub(crate) fn cancel(&self) {
        self.cancel.cancel();
    }

    /// How the export ended, once it did. A thread that stopped without an
    /// answer (a panic in a renderer or an exporter) is a failure, never a
    /// crash of the window.
    pub(crate) fn poll(&mut self) -> Option<Outcome> {
        let result = match self.receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.join();
                return Some(Outcome::Failed(
                    gafira_doc::msg!("export.failed"),
                    "the export thread stopped without an answer".to_owned(),
                ));
            }
        };
        self.join();
        Some(match result {
            Ok(()) => Outcome::Done,
            Err(EngineError::Export(ExportError::Cancelled(_))) => Outcome::Cancelled,
            Err(error) => Outcome::Failed(failure(&error), error.to_string()),
        })
    }

    /// Cancel and wait for the thread: the window is closing.
    pub(crate) fn stop(mut self) {
        self.cancel();
        self.join();
    }

    fn join(&mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Run `job` on a new thread; `wake` is called on every progress report and
/// when it ends.
pub(crate) fn spawn(job: PreparedExport, wake: Arc<dyn Fn() + Send + Sync>) -> Running {
    let (sender, receiver) = mpsc::channel();
    let tick = Arc::clone(&wake);
    let progress = Arc::new(Fraction::new(Box::new(move || tick())));
    let cancel = CancelToken::new();
    let path = job.path().to_path_buf();
    let thread = {
        let progress = Arc::clone(&progress);
        let cancel = cancel.clone();
        std::thread::Builder::new()
            .name("gafira-export".to_owned())
            .spawn(move || {
                let result = job.run(progress.as_ref(), &cancel).map(drop);
                let _ = sender.send(result);
                wake();
            })
    };
    Running {
        receiver,
        progress,
        cancel,
        // No thread: the receiver reads as disconnected, a failure.
        thread: thread.ok(),
        path,
    }
}

/// How starting an export went.
pub(crate) enum Start {
    /// The person closed the save dialog.
    Cancelled,
    /// The export runs on its thread.
    Running(Running),
    /// It was refused before it ran: the message and the detail for the log.
    Failed(Message, String),
}

/// The system save dialog, the checks of the engine and the export on its
/// thread (spec 035).
pub(crate) fn start(engine: &mut Gafira, dialog: &Dialog, ctx: &egui::Context) -> Start {
    let Some(format) = dialog.format() else {
        return Start::Cancelled;
    };
    let ext = format.extension;
    let start = engine.projects().path();
    let mut picker = rfd::FileDialog::new().add_filter(ext.to_ascii_uppercase(), &[ext]);
    if let Some(folder) = start.as_deref().and_then(Path::parent) {
        picker = picker.set_directory(folder);
    }
    let name = project_name(start.as_deref()).unwrap_or_else(|| "Gafira".to_owned());
    picker = picker.set_file_name(format!("{name}.{ext}"));
    let Some(path) = picker.save_file().map(|path| with_extension(path, format)) else {
        return Start::Cancelled;
    };
    match prepare(engine, dialog, path) {
        Ok(job) => {
            let waker = ctx.clone();
            Start::Running(spawn(job, Arc::new(move || waker.request_repaint())))
        }
        Err(error) => Start::Failed(failure(&error), error.to_string()),
    }
}

/// The checked export of the open document as `dialog` says, to `path`.
pub(crate) fn prepare(
    engine: &mut Gafira,
    dialog: &Dialog,
    path: PathBuf,
) -> Result<PreparedExport, EngineError> {
    let document = engine.edit().document().ok_or(EngineError::NoSession)?;
    let request = request(dialog, &document).map_err(|_| EngineError::NoSession)?;
    engine.export().prepare(request, path)
}

/// The card of a running export, over the bottom right of the window and
/// never modal: the progress and «Cancelar» (03 §8.1). `true` when the
/// person pressed «Cancelar».
pub(crate) fn progress(ctx: &egui::Context, theme: &Theme, texts: &Texts, fraction: f32) -> bool {
    let mut cancel = false;
    card(ctx, |ui| {
        ui.label(
            RichText::new(texts.t(&gafira_doc::msg!("export.running")))
                .font(theme.font("ui-label")),
        );
        ui.add(egui::ProgressBar::new(fraction).show_percentage());
        if widgets::secondary(ui, theme, texts.t(&gafira_doc::msg!("export.cancel"))).clicked() {
            cancel = true;
        }
    });
    cancel
}

/// What the person did with the card of a finished export.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Finished {
    Open,
    ShowInFolder,
    Close,
}

/// The card of a finished export (03 §7.5): «Tu diseño está listo para
/// compartir», «Mostrar en carpeta» and «Cerrar», never modal.
pub(crate) fn finished(ctx: &egui::Context, theme: &Theme, texts: &Texts) -> Finished {
    let mut answer = Finished::Open;
    card(ctx, |ui| {
        ui.label(
            RichText::new(texts.t(&gafira_doc::msg!("export.done"))).font(theme.font("ui-label")),
        );
        ui.horizontal(|ui| {
            if widgets::primary(
                ui,
                theme,
                texts.t(&gafira_doc::msg!("export.show_in_folder")),
            )
            .clicked()
            {
                answer = Finished::ShowInFolder;
            }
            if widgets::secondary(ui, theme, texts.t(&gafira_doc::msg!("export.close"))).clicked() {
                answer = Finished::Close;
            }
        });
    });
    answer
}

/// A card over the bottom right of the window.
fn card(ctx: &egui::Context, body: impl FnOnce(&mut egui::Ui)) {
    egui::Area::new(egui::Id::new("export-progress"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -48.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_width(280.0);
                body(ui);
            });
        });
}

/// What the status line says when an export fails.
fn failure(error: &EngineError) -> Message {
    match error {
        EngineError::ExportNotAllowed => gafira_doc::msg!("export.not_allowed"),
        EngineError::ExportBlocked(_) => error.to_message(),
        _ => gafira_doc::msg!("export.failed"),
    }
}

/// What the person did with the dialog this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    Open,
    Cancel,
    Export,
}

/// The message of a catalog key with no arguments.
fn label(key: gafira_doc::msg::MessageKey) -> Message {
    Message {
        key,
        args: Vec::new(),
    }
}

/// The dialog over the screen (03 §7.5): the preview and its size on the
/// left, the format, the pages and the options on the right.
pub(crate) fn show(
    ctx: &egui::Context,
    theme: &Theme,
    texts: &Texts,
    engine: &mut Gafira,
    dialog: &mut Dialog,
) -> Answer {
    let mut answer = Answer::Open;
    let document = engine.edit().document();
    refresh_preview(ctx, engine, dialog, document.as_deref());
    egui::Modal::new(egui::Id::new("export")).show(ctx, |ui| {
        ui.set_width(640.0);
        ui.label(RichText::new(texts.t(&gafira_doc::msg!("export.title"))).font(theme.font("h3")));
        ui.add_space(12.0);
        let refused = document
            .as_deref()
            .map_or(Err(Refusal::Nothing), |document| request(dialog, document));
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(240.0);
                preview_column(ui, theme, texts, dialog, document.as_deref());
            });
            ui.add_space(16.0);
            ui.vertical(|ui| {
                options_column(ui, theme, texts, dialog, document.as_deref(), &refused);
            });
        });
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            let ready = refused.is_ok();
            if ui
                .add_enabled_ui(ready, |ui| {
                    widgets::primary(ui, theme, texts.t(&gafira_doc::msg!("export.action")))
                })
                .inner
                .clicked()
            {
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

/// Draw the preview again when its page or its background changed.
fn refresh_preview(
    ctx: &egui::Context,
    engine: &Gafira,
    dialog: &mut Dialog,
    document: Option<&Document>,
) {
    let Some(page) = document.and_then(|document| current(document, dialog.page)) else {
        dialog.preview = None;
        return;
    };
    let transparent = dialog.transparent();
    if dialog
        .preview
        .as_ref()
        .is_some_and(|shown| shown.page == page && shown.transparent == transparent)
    {
        return;
    }
    let texture = match engine.export().preview(page, transparent) {
        Ok(preview) => Some(ctx.load_texture(
            "export-preview",
            egui::ColorImage::from_rgba_unmultiplied(
                [preview.width as usize, preview.height as usize],
                &preview.rgba,
            ),
            egui::TextureOptions::LINEAR,
        )),
        Err(error) => {
            eprintln!("export preview: {error}");
            None
        }
    };
    dialog.preview = Some(Preview {
        page,
        transparent,
        texture,
    });
}

/// The preview of the current page, on a checkerboard when the background
/// is transparent, and the size that goes out.
fn preview_column(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    dialog: &Dialog,
    document: Option<&Document>,
) {
    ui.label(
        RichText::new(texts.t(&gafira_doc::msg!("export.preview"))).font(theme.font("ui-label")),
    );
    let side = 240.0;
    let (frame, _) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::hover());
    let painter = ui.painter_at(frame);
    painter.rect_filled(frame, 0.0, theme.c("surface.sunken"));
    if let Some(texture) = dialog.preview.as_ref().and_then(|p| p.texture.as_ref()) {
        let size = texture.size_vec2();
        let fit = (side / size.x.max(size.y)).min(1.0);
        let at = egui::Rect::from_center_size(frame.center(), size * fit);
        if dialog.transparent() {
            checkerboard(&painter, at, theme.c("surface"), theme.c("border"));
        }
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        painter.image(texture.id(), at, uv, Color32::WHITE);
        painter.rect_stroke(
            at,
            0.0,
            egui::Stroke::new(1.0, theme.c("border")),
            egui::StrokeKind::Outside,
        );
    }
    ui.add_space(8.0);
    let scale = dialog.values.float(SCALE).unwrap_or(1.0);
    let size = document.and_then(|document| {
        let page = document.page(current(document, dialog.page)?)?;
        pixels(page, document, scale)
    });
    if let Some((width, height)) = size {
        ui.label(
            RichText::new(texts.t(&gafira_doc::msg!(
                "export.pixels",
                width = Count(width),
                height = Count(height)
            )))
            .color(theme.c("text.muted")),
        );
    }
    let multipage = dialog.format().is_some_and(|format| format.multipage);
    let files = document.and_then(|document| scope(dialog, document).ok());
    if let Some(files) = files.filter(|files| !multipage && files.len() > 1) {
        ui.label(
            RichText::new(texts.t(&gafira_doc::msg!(
                "export.files",
                count = Count(files.len() as u64)
            )))
            .color(theme.c("text.muted")),
        );
    }
}

/// A checkerboard of 8 px squares over `rect`: what a transparent
/// background looks like.
fn checkerboard(painter: &egui::Painter, rect: egui::Rect, light: Color32, dark: Color32) {
    const SQUARE: f32 = 8.0;
    painter.rect_filled(rect, 0.0, light);
    let mut y = rect.top();
    let mut row = 0;
    while y < rect.bottom() {
        let mut x = rect.left() + if row % 2 == 0 { 0.0 } else { SQUARE };
        while x < rect.right() {
            let square = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(SQUARE, SQUARE))
                .intersect(rect);
            painter.rect_filled(square, 0.0, dark.gamma_multiply(0.35));
            x += 2.0 * SQUARE;
        }
        y += SQUARE;
        row += 1;
    }
}

/// The format, the pages and the options of the schema.
fn options_column(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    dialog: &mut Dialog,
    document: Option<&Document>,
    refused: &Result<ExportRequest, Refusal>,
) {
    let heading = |ui: &mut egui::Ui, key: Message| {
        ui.label(RichText::new(texts.t(&key)).font(theme.font("ui-label")));
    };
    heading(ui, gafira_doc::msg!("export.format"));
    ui.horizontal_wrapped(|ui| {
        let mut picked = None;
        for (index, format) in dialog.formats.iter().enumerate() {
            let name = texts.t(&format_label(format.id));
            if ui.radio(index == dialog.format, name).clicked() && index != dialog.format {
                picked = Some(index);
            }
        }
        if let Some(index) = picked {
            dialog.pick(index);
        }
    });
    ui.add_space(8.0);
    heading(ui, gafira_doc::msg!("export.pages"));
    ui.horizontal(|ui| {
        for (pages, key) in [
            (Pages::All, gafira_doc::msg!("export.pages_all")),
            (Pages::Current, gafira_doc::msg!("export.pages_current")),
            (Pages::Range, gafira_doc::msg!("export.pages_range")),
        ] {
            if ui.radio(dialog.pages == pages, texts.t(&key)).clicked() {
                dialog.choose_pages(pages);
            }
        }
    });
    if dialog.pages == Pages::Range {
        ui.add(
            egui::TextEdit::singleline(&mut dialog.range)
                .hint_text(texts.t(&gafira_doc::msg!("export.range_hint")))
                .desired_width(200.0),
        );
        if let Err(Refusal::Range(_)) = refused {
            let count = document.map_or(0, |document| document.pages().len());
            ui.label(
                RichText::new(texts.t(&gafira_doc::msg!(
                    "export.range_invalid",
                    count = Count(count as u64)
                )))
                .color(theme.c("error.fg")),
            );
        }
    }
    let Some(format) = dialog.format().cloned() else {
        return;
    };
    for def in format.schema.params() {
        ui.add_space(8.0);
        option(ui, theme, texts, def, &mut dialog.values);
    }
}

/// The catalog key of the name of format `id`.
fn format_label(id: &str) -> Message {
    match id {
        "png" => gafira_doc::msg!("export.png"),
        "jpeg" => gafira_doc::msg!("export.jpeg"),
        "webp" => gafira_doc::msg!("export.webp"),
        "pdf" => gafira_doc::msg!("export.pdf"),
        other => Message {
            key: gafira_doc::msg::MessageKey("export.format"),
            args: vec![("format", MsgArg::Text(other.to_owned()))],
        },
    }
}

/// The scale steps of [`STEPS`] inside `[min, max]`.
pub(crate) fn steps(min: f64, max: f64) -> Vec<(f64, &'static str)> {
    STEPS
        .iter()
        .copied()
        .filter(|(step, _)| (min..=max).contains(step))
        .collect()
}

/// One option of the schema, drawn by its kind.
fn option(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    def: &ParamDef,
    values: &mut ParamValues,
) {
    let name = texts.t(&label(def.label));
    match &def.kind {
        ParamKind::Float { min, max, default } if def.id == SCALE => {
            let current = values.float(def.id).unwrap_or(*default);
            ui.horizontal(|ui| {
                ui.label(RichText::new(name).font(theme.font("ui-label")));
                for (step, key) in steps(*min, *max) {
                    let chosen = (current - step).abs() < f64::EPSILON;
                    if ui
                        .radio(chosen, texts.t(&label(gafira_doc::msg::MessageKey(key))))
                        .clicked()
                    {
                        values.set(def.id, ParamValue::Float(step));
                    }
                }
            });
        }
        ParamKind::Float { min, max, default } => {
            let mut value = values.float(def.id).unwrap_or(*default);
            ui.horizontal(|ui| {
                ui.label(name);
                if ui.add(egui::Slider::new(&mut value, *min..=*max)).changed() {
                    values.set(def.id, ParamValue::Float(value));
                }
            });
        }
        ParamKind::Int { min, max, default } => {
            let mut value = values.int(def.id).unwrap_or(*default);
            ui.horizontal(|ui| {
                ui.label(name);
                if ui.add(egui::Slider::new(&mut value, *min..=*max)).changed() {
                    values.set(def.id, ParamValue::Int(value));
                }
            });
        }
        ParamKind::Bool { default } => {
            let mut value = values.bool(def.id).unwrap_or(*default);
            if ui.checkbox(&mut value, name).changed() {
                values.set(def.id, ParamValue::Bool(value));
            }
        }
        ParamKind::Choice { options, default } => {
            let current = values.choice(def.id).unwrap_or(default).to_owned();
            ui.horizontal(|ui| {
                ui.label(name);
                for choice in *options {
                    if ui
                        .radio(current == choice.id, texts.t(&label(choice.label)))
                        .clicked()
                    {
                        values.set(def.id, ParamValue::Choice(choice.id.to_owned()));
                    }
                }
            });
        }
        // No export format has a colour option yet.
        ParamKind::Color { .. } => {}
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use gafira_engine::Registries;
    use gafira_io_image::{JpegExporter, PngExporter, WebpExporter};
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

    /// The registries the app composes: PNG, JPEG, WebP and PDF (spec 039).
    pub(crate) fn registries() -> Registries {
        let mut registries = Registries::new();
        for exporter in [
            Arc::new(PngExporter::new()) as Arc<dyn Exporter>,
            Arc::new(JpegExporter::new()),
            Arc::new(WebpExporter::new()),
            Arc::new(PdfExporter::new()),
        ] {
            registries
                .formats
                .register_exporter(exporter)
                .expect("the exporters register");
        }
        registries
    }

    /// An engine with a new Redes document of `pages` pages.
    fn document_engine(pages: usize) -> Gafira {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a document");
        for at in 1..pages {
            let size = engine.edit().document().expect("a document").pages()[0].size;
            engine
                .edit()
                .execute(Box::new(gafira_edit::InsertPage::new(
                    at,
                    String::new(),
                    size,
                )))
                .expect("a page");
        }
        engine
    }

    /// The dialog of `engine` on format `id`.
    fn dialog_on(engine: &Gafira, id: &str, page: Option<PageId>) -> Dialog {
        let mut dialog = Dialog::new(formats(engine), page);
        let index = dialog
            .formats
            .iter()
            .position(|format| format.id == id)
            .expect("the format is registered");
        dialog.pick(index);
        dialog
    }

    // S39-03: `1-3, 5` is pages 1, 2, 3 and 5; spaces do not count and a
    // page named twice goes out once, in document order.
    #[test]
    fn s39_03_a_range_names_its_pages_in_document_order() {
        assert_eq!(parse_range("1-3, 5", 5), Ok(vec![0, 1, 2, 4]));
        assert_eq!(parse_range(" 5 ,1- 2,2", 5), Ok(vec![0, 1, 4]));
        assert_eq!(parse_range("3-3", 3), Ok(vec![2]));
        assert_eq!(parse_range("1,", 1), Ok(vec![0]));
        assert_eq!(parse_range("1-5", 5), Ok(vec![0, 1, 2, 3, 4]));
    }

    // S39-03: an empty range, page 0, a page past the last one, a reversed
    // range or a word is refused, with why.
    #[test]
    fn s39_03_a_wrong_range_is_refused_with_its_reason() {
        assert_eq!(parse_range("", 3), Err(RangeError::Empty));
        assert_eq!(parse_range(" , ", 3), Err(RangeError::Empty));
        assert_eq!(parse_range("0", 3), Err(RangeError::OutOfRange));
        assert_eq!(parse_range("4", 3), Err(RangeError::OutOfRange));
        assert_eq!(parse_range("2-4", 3), Err(RangeError::OutOfRange));
        assert_eq!(parse_range("3-1", 3), Err(RangeError::OutOfRange));
        assert_eq!(
            parse_range("99999999999999999999999", 3),
            Err(RangeError::OutOfRange)
        );
        for syntax in ["a", "1-", "-2", "1-2-3", "+1", "1.5", "1 2"] {
            assert_eq!(parse_range(syntax, 3), Err(RangeError::Syntax), "{syntax}");
        }
    }

    // S39-04: the pages of the dialog reach the request: all of them, the
    // current one (or the first when it went away) or the range.
    #[test]
    fn s39_04_the_pages_of_the_dialog_reach_the_request() {
        let mut engine = document_engine(4);
        let document = engine.edit().document().expect("a snapshot");
        let ids: Vec<PageId> = document.pages().iter().map(|p| p.id).collect();
        let mut dialog = dialog_on(&engine, "png", Some(ids[2]));
        let pages = |dialog: &Dialog| match request(dialog, &document).map(|r| r.scope) {
            Ok(ExportScope::Pages(pages)) => Ok(pages),
            Ok(other) => panic!("{other:?}"),
            Err(refusal) => Err(refusal),
        };
        assert_eq!(pages(&dialog), Ok(vec![ids[2]]), "the current page");
        dialog.page = Some(PageId::new(999));
        assert_eq!(pages(&dialog), Ok(vec![ids[0]]), "the first when gone");
        dialog.choose_pages(Pages::All);
        assert_eq!(pages(&dialog), Ok(ids.clone()));
        dialog.choose_pages(Pages::Range);
        dialog.range = "4, 1-2".to_owned();
        assert_eq!(pages(&dialog), Ok(vec![ids[0], ids[1], ids[3]]));
        dialog.range = "5".to_owned();
        assert_eq!(
            pages(&dialog),
            Err(Refusal::Range(RangeError::OutOfRange)),
            "a page past the last one cannot export"
        );
    }

    // S56-04 (FD-02 b): a hidden page is left out of «Todas» and of a
    // range, which still counts it; hidden alone, nothing is exported.
    #[test]
    fn s56_04_hidden_pages_are_not_exported() {
        let mut engine = document_engine(4);
        let ids: Vec<PageId> = engine
            .edit()
            .document()
            .expect("a snapshot")
            .pages()
            .iter()
            .map(|p| p.id)
            .collect();
        engine
            .edit()
            .execute(Box::new(gafira_edit::SetProperty::page(
                ids[1],
                gafira_edit::Property::PageHidden(true),
                None,
            )))
            .expect("hidden");
        let document = engine.edit().document().expect("a snapshot");
        let mut dialog = dialog_on(&engine, "png", Some(ids[1]));
        dialog.choose_pages(Pages::All);
        assert_eq!(scope(&dialog, &document), Ok(vec![ids[0], ids[2], ids[3]]));
        dialog.choose_pages(Pages::Range);
        dialog.range = "1-3".to_owned();
        assert_eq!(scope(&dialog, &document), Ok(vec![ids[0], ids[2]]));
        dialog.choose_pages(Pages::Current);
        assert_eq!(scope(&dialog, &document), Err(Refusal::Nothing));
    }

    // S39-04: the options of the schema reach the request, and those the
    // format lacks do not; the PDF keeps its resolution of 2×.
    #[test]
    fn s39_04_the_options_come_from_the_schema_of_the_format() {
        let mut engine = document_engine(2);
        let document = engine.edit().document().expect("a snapshot");
        let mut dialog = dialog_on(&engine, "png", None);
        assert_eq!(
            dialog.formats.iter().map(|f| f.id).collect::<Vec<_>>(),
            ["png", "jpeg", "webp", "pdf"]
        );
        dialog.values.set(SCALE, ParamValue::Float(3.0));
        dialog.values.set(TRANSPARENT, ParamValue::Bool(true));
        let png = request(&dialog, &document).expect("a request");
        assert_eq!(png.format_id, "png");
        assert_eq!(png.options.float(SCALE), Some(3.0));
        assert_eq!(png.options.bool(TRANSPARENT), Some(true));

        let jpeg_index = dialog.formats.iter().position(|f| f.id == "jpeg").unwrap();
        dialog.pick(jpeg_index);
        let jpeg = request(&dialog, &document).expect("a request");
        assert_eq!(jpeg.format_id, "jpeg");
        assert_eq!(jpeg.options.int("quality"), Some(90));
        assert_eq!(jpeg.options.float(SCALE), Some(1.0), "back to the default");
        assert_eq!(jpeg.options.bool(TRANSPARENT), None, "JPEG has none");
        dialog.values.set(TRANSPARENT, ParamValue::Bool(true));
        assert_eq!(
            request(&dialog, &document)
                .unwrap()
                .options
                .bool(TRANSPARENT),
            None,
            "an option outside the schema is not sent"
        );

        let pdf = dialog_on(&engine, "pdf", None);
        let request = request(&pdf, &document).expect("a request");
        assert_eq!(request.options.float(SCALE), Some(2.0));
        let all: Vec<PageId> = document.pages().iter().map(|p| p.id).collect();
        assert_eq!(
            request.scope,
            ExportScope::Pages(all),
            "a PDF has every page"
        );
    }

    // S39-04: the pages follow the format until the person chose them.
    #[test]
    fn s39_04_the_pages_follow_the_format_until_they_are_chosen() {
        let engine = document_engine(1);
        let mut dialog = dialog_on(&engine, "png", None);
        assert_eq!(dialog.pages, Pages::Current);
        dialog.pick(3);
        assert_eq!(dialog.pages, Pages::All, "PDF");
        dialog.choose_pages(Pages::Current);
        dialog.pick(0);
        dialog.pick(3);
        assert_eq!(dialog.pages, Pages::Current, "chosen");
    }

    // S39-04: the scale steps keep to the range of the schema.
    #[test]
    fn s39_04_the_scale_steps_keep_to_the_schema() {
        let keys = |min, max| steps(min, max).into_iter().map(|s| s.0).collect::<Vec<_>>();
        assert_eq!(keys(0.5, 4.0), [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(keys(1.0, 2.0), [1.0, 2.0]);
        assert_eq!(keys(2.5, 8.0), [3.0, 4.0]);
    }

    // S39-06: the size in pixels is the page by the scale, rounded.
    #[test]
    fn s39_06_the_pixel_size_is_the_page_by_the_scale() {
        let mut engine = document_engine(1);
        let document = engine.edit().document().expect("a snapshot");
        let page = &document.pages()[0];
        let one = pixels(page, &document, 1.0).expect("a size");
        assert_eq!(pixels(page, &document, 2.0), Some((one.0 * 2, one.1 * 2)));
        assert_eq!(pixels(page, &document, 0.0), Some((1, 1)), "at least 1");
        assert_eq!(pixels(page, &document, f64::NAN), None);
    }

    #[test]
    fn s30_07_with_extension_adds_the_extension_of_the_format() {
        let engine = document_engine(1);
        let dialog = Dialog::new(formats(&engine), None);
        let format = |id: &str| {
            dialog
                .formats
                .iter()
                .find(|format| format.id == id)
                .expect("a format")
                .clone()
        };
        assert_eq!(
            with_extension("/w/cartel".into(), &format("pdf")),
            PathBuf::from("/w/cartel.pdf")
        );
        assert_eq!(
            with_extension("/w/cartel.PNG".into(), &format("png")),
            PathBuf::from("/w/cartel.PNG")
        );
        assert_eq!(
            with_extension("/w/cartel".into(), &format("jpeg")),
            PathBuf::from("/w/cartel.jpg")
        );
        assert_eq!(
            with_extension("/w/cartel.jpeg".into(), &format("jpeg")),
            PathBuf::from("/w/cartel.jpeg")
        );
    }

    // S30-07, S39-08: the app's engine writes PNG, JPEG, WebP and PDF of the
    // open document to the chosen paths; the temporary files go away.
    #[test]
    fn s39_08_the_engine_of_the_app_writes_every_format() {
        let mut engine = document_engine(1);
        let dir = std::env::temp_dir().join(format!("gafira-ui-export-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let outputs: Vec<(&str, PathBuf, &[u8])> = vec![
            ("png", dir.join("cartel.png"), b"\x89PNG"),
            ("jpeg", dir.join("cartel.jpg"), &[0xFF, 0xD8, 0xFF]),
            ("webp", dir.join("cartel.webp"), b"RIFF"),
            ("pdf", dir.join("cartel.pdf"), b"%PDF-"),
        ];
        let result: Result<(), EngineError> = outputs.iter().try_for_each(|(id, path, _)| {
            let dialog = dialog_on(&engine, id, None);
            export_to(&mut engine, &dialog, path)
        });
        let written: Vec<_> = outputs
            .iter()
            .map(|(_, path, _)| std::fs::read(path))
            .collect();
        let left: Vec<_> = std::fs::read_dir(&dir)
            .map(|entries| entries.filter_map(Result::ok).map(|e| e.path()).collect())
            .unwrap_or_default();
        let _ = std::fs::remove_dir_all(&dir);
        result.expect("every export runs");
        for ((id, _, signature), bytes) in outputs.iter().zip(written) {
            assert!(bytes.expect(id).starts_with(signature), "{id}");
        }
        assert_eq!(left.len(), 4, "no temporary is left: {left:?}");
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
            texts.t(&gafira_doc::msg!("export.show_in_folder")),
            "Mostrar en carpeta"
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

    /// Wait, without a window, for `running` to end.
    fn finish(running: &mut Running) -> Outcome {
        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        loop {
            if let Some(outcome) = running.poll() {
                return outcome;
            }
            assert!(std::time::Instant::now() < deadline, "the export ends");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    // S35-04: the export runs on its own thread: `spawn` returns at once,
    // the progress reaches 1 and the PDF appears; the window is woken. The
    // running export knows its file, for «Mostrar en carpeta» (S39-07).
    #[test]
    fn s35_04_the_export_runs_off_the_window_thread() {
        let mut engine = document_engine(1);
        let dir = std::env::temp_dir().join(format!("gafira-ui-bg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let pdf = dir.join("cartel.pdf");
        let dialog = dialog_on(&engine, "pdf", None);
        let job = prepare(&mut engine, &dialog, pdf.clone()).expect("prepares");
        let woken = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = Arc::clone(&woken);
        let window = std::thread::current().id();
        let mut running = spawn(
            job,
            Arc::new(move || {
                assert_ne!(std::thread::current().id(), window, "not the window thread");
                count.fetch_add(1, Ordering::Relaxed);
            }),
        );
        assert_eq!(running.path(), pdf.as_path());
        let outcome = finish(&mut running);
        let bytes = std::fs::read(&pdf);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(matches!(outcome, Outcome::Done));
        assert!(bytes.expect("the PDF").starts_with(b"%PDF-"));
        assert!((running.fraction() - 1.0).abs() < f32::EPSILON);
        assert!(
            woken.load(Ordering::Relaxed) >= 2,
            "progress and the end wake it"
        );
    }

    // S35-05: «Cancelar» ends the export as cancelled and leaves no file.
    #[test]
    fn s35_05_a_cancelled_export_ends_cancelled_without_a_file() {
        let mut engine = document_engine(1);
        let dir = std::env::temp_dir().join(format!("gafira-ui-cancel-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let pdf = dir.join("cartel.pdf");
        let dialog = dialog_on(&engine, "pdf", None);
        let job = prepare(&mut engine, &dialog, pdf.clone()).expect("prepares");
        // Cancelled before the thread looks: deterministic.
        let gate = Arc::new(std::sync::Barrier::new(2));
        let wait = Arc::clone(&gate);
        let first = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let mut running = spawn(
            job,
            Arc::new(move || {
                if first.swap(false, Ordering::Relaxed) {
                    wait.wait();
                }
            }),
        );
        running.cancel();
        gate.wait();
        let outcome = finish(&mut running);
        let left: Vec<_> = std::fs::read_dir(&dir)
            .map(|entries| entries.filter_map(Result::ok).map(|e| e.path()).collect())
            .unwrap_or_default();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(matches!(outcome, Outcome::Cancelled), "cancelled");
        assert!(left.is_empty(), "no file: {left:?}");
    }

    // S35-06: a thread that dies without an answer is a failure the window
    // shows, never a crash of the window.
    #[test]
    fn s35_06_a_thread_that_panics_is_a_failure() {
        let (sender, receiver) = mpsc::channel::<Result<(), EngineError>>();
        let thread = std::thread::spawn(move || {
            let _keep = sender;
            panic!("a renderer bug");
        });
        let mut running = Running {
            receiver,
            progress: Arc::new(Fraction::new(Box::new(|| {}))),
            cancel: CancelToken::new(),
            thread: Some(thread),
            path: PathBuf::from("cartel.pdf"),
        };
        match finish(&mut running) {
            Outcome::Failed(message, _) => assert_eq!(message, gafira_doc::msg!("export.failed")),
            _ => panic!("a failure"),
        }
    }

    // S35-04: the progress is kept in [0, 1].
    #[test]
    fn s35_04_the_fraction_stays_in_range() {
        let fraction = Fraction::new(Box::new(|| {}));
        fraction.report(f32::NAN);
        assert_eq!(fraction.get(), 0.0);
        fraction.report(1.5);
        assert_eq!(fraction.get(), 1.0);
        fraction.report(0.25);
        assert_eq!(fraction.get(), 0.25);
    }
}
