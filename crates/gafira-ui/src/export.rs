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
/// `gafira-io-pdf`): the scale as a slider (spec 074), the background over
/// the preview.
const SCALE: &str = "scale";
const TRANSPARENT: &str = "transparent";

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
/// One option value kept by «Recordar esta configuración» (spec 074): the
/// kinds an export option has, a password never among them.
#[derive(Clone, Debug, PartialEq)]
pub enum PresetValue {
    Float(f64),
    Int(i64),
    Bool(bool),
    Choice(String),
}

/// The settings kept for one format: option id and value.
pub type Preset = Vec<(String, PresetValue)>;

/// Where the export settings of each format are kept between sessions (the
/// app's `export-presets.json`, spec 074).
pub trait ExportPresets {
    /// The settings kept for `format`, if any.
    fn load(&self, format: &str) -> Option<Vec<(String, PresetValue)>>;
    /// Keep `values` for `format`, or forget them with `None`.
    fn save(&self, format: &str, values: Option<Vec<(String, PresetValue)>>);
}

/// The values of `values` worth keeping: no secret text (a password).
pub(crate) fn preset_of(values: &ParamValues, schema: &ParamSchema) -> Vec<(String, PresetValue)> {
    schema
        .params()
        .iter()
        .filter_map(|def| {
            let value = match &def.kind {
                ParamKind::Float { .. } => PresetValue::Float(values.float(def.id)?),
                ParamKind::Int { .. } => PresetValue::Int(values.int(def.id)?),
                ParamKind::Bool { .. } => PresetValue::Bool(values.bool(def.id)?),
                ParamKind::Choice { .. } => PresetValue::Choice(values.choice(def.id)?.to_owned()),
                ParamKind::Color { .. } | ParamKind::Text { .. } => return None,
            };
            Some((def.id.to_owned(), value))
        })
        .collect()
}

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
    /// «Proteger con contraseña»: shows the password field (spec 073).
    protect: bool,
    /// The settings kept for each format, read when the dialog opens.
    presets: std::collections::HashMap<String, Vec<(String, PresetValue)>>,
    /// «Recordar esta configuración para este formato» (spec 074).
    pub(crate) remember: bool,
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
            protect: false,
            presets: std::collections::HashMap::new(),
            remember: false,
        };
        dialog.pick(0);
        dialog
    }

    /// The dialog with the settings kept for each format (spec 074): the
    /// first format opens with its own.
    pub(crate) fn with_presets(mut self, presets: &dyn ExportPresets) -> Self {
        for format in &self.formats {
            if let Some(kept) = presets.load(format.id) {
                self.presets.insert(format.id.to_owned(), kept);
            }
        }
        let at = self.format;
        self.pick(at);
        self
    }

    /// What «Exportar» keeps for the chosen format: its settings when
    /// «Recordar» is on, or `None` to forget them.
    pub(crate) fn preset(&self) -> Option<(String, Option<Preset>)> {
        let format = self.format()?;
        let kept = self
            .remember
            .then(|| preset_of(&self.values, &format.schema));
        Some((format.id.to_owned(), kept))
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
        // The settings kept for this format, over its defaults (spec 074);
        // a value the schema no longer takes is skipped.
        self.remember = false;
        if let Some(kept) = self.presets.get(format.id) {
            self.remember = true;
            for (id, value) in kept {
                let Some(def) = format.schema.params().iter().find(|d| d.id == id.as_str()) else {
                    continue;
                };
                let value = match value {
                    PresetValue::Float(v) => ParamValue::Float(*v),
                    PresetValue::Int(v) => ParamValue::Int(*v),
                    PresetValue::Bool(v) => ParamValue::Bool(*v),
                    PresetValue::Choice(v) => ParamValue::Choice(v.clone()),
                };
                let mut trial = self.values.clone();
                trial.set(def.id, value);
                if format.schema.check_values(&trial).is_ok() {
                    self.values = trial;
                }
            }
        }
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
    /// An Excel file of pages with no table (S81-14).
    NoTables,
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
    if format.id == "xlsx" && !gafira_io_xlsx::has_tables(document, &pages) {
        return Err(Refusal::NoTables);
    }
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
pub(crate) fn start(
    engine: &mut Gafira,
    dialog: &Dialog,
    ctx: &egui::Context,
    window: &eframe::Frame,
) -> Start {
    let Some(format) = dialog.format() else {
        return Start::Cancelled;
    };
    let ext = format.extension;
    let start = engine.projects().path();
    // Over Gafira's window, centred on it (owner report: it opened behind).
    let mut picker = rfd::FileDialog::new()
        .set_parent(window)
        .add_filter(ext.to_ascii_uppercase(), &[ext]);
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
    // Room inside the window (owner request): nothing touches its edges.
    let frame = egui::Frame::popup(&ctx.style())
        .inner_margin(egui::Margin::same(24))
        .corner_radius(theme.radius_md());
    egui::Modal::new(egui::Id::new("export"))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_width(660.0);
            ui.label(
                RichText::new(texts.t(&gafira_doc::msg!("export.title"))).font(theme.font("h3")),
            );
            ui.add_space(16.0);
            let refused = document
                .as_deref()
                .map_or(Err(Refusal::Nothing), |document| request(dialog, document));
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(220.0);
                    preview_column(ui, theme, texts, dialog, document.as_deref());
                });
                ui.add_space(24.0);
                ui.vertical(|ui| {
                    options_column(ui, theme, texts, dialog, document.as_deref(), &refused);
                });
            });
            ui.add_space(20.0);
            ui.separator();
            ui.add_space(12.0);
            // The actions at the bottom right: «Exportar» the one that leads.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let ready = refused.is_ok();
                let export = ui.add_enabled_ui(ready, |ui| {
                    let button = egui::Button::new(
                        RichText::new(texts.t(&gafira_doc::msg!("export.action")))
                            .font(theme.font("ui-label"))
                            .color(theme.c("primary.fg")),
                    )
                    .fill(theme.c("primary.bg"))
                    .corner_radius(theme.radius_md())
                    .min_size(egui::vec2(140.0, 40.0));
                    ui.add(button)
                });
                if export.inner.hovered() && ready {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if export.inner.clicked() {
                    answer = Answer::Export;
                }
                ui.add_space(8.0);
                let cancel = ui.add(
                    egui::Button::new(
                        RichText::new(texts.t(&gafira_doc::msg!("leave.cancel")))
                            .font(theme.font("ui-label")),
                    )
                    .corner_radius(theme.radius_md())
                    .min_size(egui::vec2(110.0, 40.0)),
                );
                if cancel.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if cancel.clicked() {
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
    // Two columns: what each choice is, and its control (owner request:
    // drop-down lists and switches, with room between rows).
    let name = |ui: &mut egui::Ui, key: &Message| {
        ui.label(RichText::new(texts.t(key)).font(theme.font("ui-label")));
    };
    const WIDE: f32 = 220.0;
    egui::Grid::new("export.options")
        .num_columns(2)
        .spacing([16.0, 14.0])
        .min_col_width(110.0)
        .show(ui, |ui| {
            name(ui, &gafira_doc::msg!("export.format"));
            let shown = dialog
                .format()
                .map(|f| texts.t(&format_label(f.id)))
                .unwrap_or_default();
            let mut picked = None;
            egui::ComboBox::from_id_salt("export.format")
                .width(WIDE)
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    for (index, format) in dialog.formats.iter().enumerate() {
                        let label = texts.t(&format_label(format.id));
                        if ui.selectable_label(index == dialog.format, label).clicked() {
                            picked = Some(index);
                        }
                    }
                });
            if let Some(index) = picked.filter(|i| *i != dialog.format) {
                dialog.pick(index);
                dialog.protect = false;
            }
            ui.end_row();

            name(ui, &gafira_doc::msg!("export.pages"));
            let pages = [
                (Pages::All, gafira_doc::msg!("export.pages_all")),
                (Pages::Current, gafira_doc::msg!("export.pages_current")),
                (Pages::Range, gafira_doc::msg!("export.pages_range")),
            ];
            let shown = pages
                .iter()
                .find(|(p, _)| *p == dialog.pages)
                .map(|(_, key)| texts.t(key))
                .unwrap_or_default();
            egui::ComboBox::from_id_salt("export.pages")
                .width(WIDE)
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    for (choice, key) in &pages {
                        if ui
                            .selectable_label(dialog.pages == *choice, texts.t(key))
                            .clicked()
                        {
                            dialog.choose_pages(*choice);
                        }
                    }
                });
            ui.end_row();
            if dialog.pages == Pages::Range {
                ui.label("");
                ui.vertical(|ui| {
                    ui.add(
                        widgets::field(
                            &mut dialog.range,
                            texts.t(&gafira_doc::msg!("export.range_hint")),
                        )
                        .desired_width(WIDE),
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
                });
                ui.end_row();
            }
            if matches!(refused, Err(Refusal::NoTables)) {
                ui.label("");
                ui.label(
                    RichText::new(texts.t(&gafira_doc::msg!("export.no_tables")))
                        .color(theme.c("error.fg")),
                );
                ui.end_row();
            }
            format_options(ui, theme, texts, dialog, WIDE);
        });
}

/// The options of the chosen format, rows of a two-column grid already open:
/// the controls of its schema and «Recordar esta configuración» (spec 074).
/// The bulk export dialog shows the same (spec 104).
pub(crate) fn format_options(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    dialog: &mut Dialog,
    wide: f32,
) {
    let name = |ui: &mut egui::Ui, key: &Message| {
        ui.label(RichText::new(texts.t(key)).font(theme.font("ui-label")));
    };
    let Some(format) = dialog.format().cloned() else {
        return;
    };
    let flatten = dialog.values.bool("flatten").unwrap_or(false);
    for def in format.schema.params() {
        // The resolution of a PDF only counts when it is flattened;
        // the password only once «Proteger con contraseña» is on.
        if format.id == "pdf" && def.id == SCALE && !flatten {
            continue;
        }
        if def.id == "password" {
            name(ui, &gafira_doc::msg!("export.pdf_protect"));
            ui.horizontal(|ui| {
                if widgets::toggle(ui, theme, &mut dialog.protect).changed() && !dialog.protect {
                    dialog.values.set(def.id, ParamValue::Text(String::new()));
                }
                info(
                    ui,
                    theme,
                    texts.t(&label(gafira_doc::msg::MessageKey(
                        "export.pdf_password_hint",
                    ))),
                );
            });
            ui.end_row();
            if !dialog.protect {
                continue;
            }
        }
        name(ui, &label(def.label));
        control(ui, theme, texts, def, &mut dialog.values, wide);
        ui.end_row();
    }
    // «Recordar esta configuración»: these settings next time this
    // format is chosen (spec 074).
    name(ui, &gafira_doc::msg!("export.remember"));
    ui.horizontal(|ui| {
        widgets::toggle(ui, theme, &mut dialog.remember);
        info(
            ui,
            theme,
            texts.t(&label(gafira_doc::msg::MessageKey("export.remember_hint"))),
        );
    });
    // The row ends here: what the caller adds goes on a row of its own.
    ui.end_row();
}

/// The catalog key of the name of format `id`.
pub(crate) fn format_label(id: &str) -> Message {
    match id {
        "png" => gafira_doc::msg!("export.png"),
        "jpeg" => gafira_doc::msg!("export.jpeg"),
        "webp" => gafira_doc::msg!("export.webp"),
        "avif" => gafira_doc::msg!("export.avif"),
        "pdf" => gafira_doc::msg!("export.pdf"),
        "svg" => gafira_doc::msg!("export.svg"),
        "pptx" => gafira_doc::msg!("export.pptx"),
        "xlsx" => gafira_doc::msg!("export.xlsx"),
        other => Message {
            key: gafira_doc::msg::MessageKey("export.format"),
            args: vec![("format", MsgArg::Text(other.to_owned()))],
        },
    }
}

/// The control of one option of the schema, by its kind, `wide` across:
/// a switch, a drop-down list, a slider or a field, with its «i» when it
/// has an explanation.
fn control(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    def: &ParamDef,
    values: &mut ParamValues,
    wide: f32,
) {
    let name = texts.t(&label(def.label));
    let hint = |ui: &mut egui::Ui| {
        if let Some(key) = hint_of(def.id) {
            info(ui, theme, texts.t(&label(gafira_doc::msg::MessageKey(key))));
        }
    };
    ui.horizontal(|ui| match &def.kind {
        // The scale is dragged (owner request), in steps of a quarter.
        ParamKind::Float { min, max, default } => {
            let mut value = values.float(def.id).unwrap_or(*default);
            // The scale is dragged in steps of a quarter.
            let (step, decimals, suffix) = if def.id == SCALE {
                (0.25, 2, "×")
            } else {
                (0.0, 2, "")
            };
            let slider = crate::slider::Slider::new(name.clone(), &mut value, (*min, *max))
                .rest(*default)
                .step(step, decimals)
                .suffix(suffix)
                .unnamed();
            if ui
                .vertical(|ui| {
                    ui.set_width(wide - 40.0);
                    slider.show(ui, theme)
                })
                .inner
            {
                values.set(def.id, ParamValue::Float(value));
            }
            hint(ui);
        }
        ParamKind::Int { min, max, default } => {
            // A whole number in an i64 well inside f64's exact range.
            let (mut value, range, rest) = (
                values.int(def.id).unwrap_or(*default) as f64,
                (*min as f64, *max as f64),
                *default as f64,
            );
            let slider = crate::slider::Slider::new(name.clone(), &mut value, range)
                .rest(rest)
                .unnamed();
            if ui
                .vertical(|ui| {
                    ui.set_width(wide - 40.0);
                    slider.show(ui, theme)
                })
                .inner
            {
                // Rounded within the range: the cast is exact.
                values.set(def.id, ParamValue::Int(value.round() as i64));
            }
            hint(ui);
        }
        ParamKind::Bool { default } => {
            let mut value = values.bool(def.id).unwrap_or(*default);
            if widgets::toggle(ui, theme, &mut value).changed() {
                values.set(def.id, ParamValue::Bool(value));
            }
            hint(ui);
        }
        ParamKind::Choice { options, default } => {
            let current = values.choice(def.id).unwrap_or(default).to_owned();
            let shown = options
                .iter()
                .find(|o| o.id == current)
                .map(|o| texts.t(&label(o.label)))
                .unwrap_or_default();
            egui::ComboBox::from_id_salt(("export.option", def.id))
                .width(wide)
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    for choice in *options {
                        let name = texts.t(&label(choice.label));
                        if ui.selectable_label(current == choice.id, name).clicked() {
                            values.set(def.id, ParamValue::Choice(choice.id.to_owned()));
                        }
                    }
                });
            hint(ui);
        }
        // No export format has a colour option yet.
        ParamKind::Color { .. } => {}
        // A line of text; a secret one (a password) is hidden as it is typed.
        ParamKind::Text { max_len, secret } => {
            let mut value = values.text(def.id).unwrap_or_default().to_owned();
            let field = widgets::field(&mut value, String::new())
                .password(*secret)
                .char_limit(*max_len)
                .desired_width(wide);
            if ui.add(field).changed() {
                values.set(def.id, ParamValue::Text(value));
            }
        }
    });
}

/// The catalog key of the explanation an option shows on its «i» icon
/// (spec 073), by option id.
fn hint_of(id: &str) -> Option<&'static str> {
    match id {
        "compress" => Some("export.pdf_compress_hint"),
        "flatten" => Some("export.pdf_flatten_hint"),
        "password" => Some("export.pdf_password_hint"),
        "color_profile" => Some("export.pdf_color_hint"),
        "text_as_paths" => Some("export.svg_text_as_paths_hint"),
        "png_quality" => Some("export.png_quality_hint"),
        "quality" => Some("export.jpeg_quality_hint"),
        "webp_quality" => Some("export.webp_quality_hint"),
        "avif_quality" => Some("export.avif_quality_hint"),
        _ => None,
    }
}

/// A small «i» in a circle that shows `hint` on hover.
pub(crate) fn info(ui: &mut egui::Ui, theme: &Theme, hint: String) {
    let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(16.0), egui::Sense::hover());
    let colour = theme.c("text.secondary");
    let painter = ui.painter();
    painter.circle_stroke(rect.center(), 7.0, egui::Stroke::new(1.2, colour));
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "i",
        egui::FontId::proportional(10.0),
        colour,
    );
    response.on_hover_text(hint);
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use gafira_engine::Registries;
    use gafira_io_image::{AvifExporter, JpegExporter, PngExporter, WebpExporter};
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

    /// The registries the app composes: PNG, JPEG, WebP and PDF (spec 039),
    /// SVG (spec 074) and PowerPoint (spec 075).
    pub(crate) fn registries() -> Registries {
        let mut registries = Registries::new();
        for exporter in [
            Arc::new(PngExporter::new()) as Arc<dyn Exporter>,
            Arc::new(JpegExporter::new()),
            Arc::new(WebpExporter::new()),
            Arc::new(AvifExporter::new()),
            Arc::new(PdfExporter::new()),
            Arc::new(gafira_io_svg::SvgExporter::new()),
            Arc::new(gafira_io_pptx::PptxExporter::new()),
            Arc::new(gafira_io_xlsx::XlsxExporter::new()),
        ] {
            registries
                .formats
                .register_exporter(exporter)
                .expect("the exporters register");
        }
        registries
    }

    /// An engine with a new Redes document of `pages` pages.
    fn document_engine(pages: usize) -> crate::editor::tests::TestEngine {
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

    /// Export settings kept in memory.
    #[derive(Default)]
    struct Kept(std::cell::RefCell<std::collections::HashMap<String, Vec<(String, PresetValue)>>>);

    impl ExportPresets for Kept {
        fn load(&self, format: &str) -> Option<Vec<(String, PresetValue)>> {
            self.0.borrow().get(format).cloned()
        }

        fn save(&self, format: &str, values: Option<Vec<(String, PresetValue)>>) {
            match values {
                Some(values) => self.0.borrow_mut().insert(format.to_owned(), values),
                None => self.0.borrow_mut().remove(format),
            };
        }
    }

    // S74-03: «Recordar esta configuración» keeps a format's settings, not
    // its password, and the next dialog opens with them; off, it forgets.
    #[test]
    fn s74_03_the_settings_of_a_format_are_kept() {
        let engine = document_engine(1);
        let kept = Kept::default();
        let mut dialog = dialog_on(&engine, "pdf", None).with_presets(&kept);
        assert!(!dialog.remember);
        dialog.values.set("compress", ParamValue::Bool(true));
        dialog
            .values
            .set("color_profile", ParamValue::Choice("cmyk".to_owned()));
        dialog
            .values
            .set("password", ParamValue::Text("secreto".to_owned()));
        dialog.remember = true;
        let (format, values) = dialog.preset().expect("a format");
        kept.save(&format, values);
        let saved = kept.load("pdf").expect("kept");
        assert!(
            saved.iter().all(|(id, _)| id != "password"),
            "never the password"
        );
        let again = Dialog::new(formats(&engine), None).with_presets(&kept);
        let index = again
            .formats
            .iter()
            .position(|f| f.id == "pdf")
            .expect("pdf");
        let mut again = again;
        again.pick(index);
        assert!(again.remember);
        assert_eq!(again.values.bool("compress"), Some(true));
        assert_eq!(again.values.choice("color_profile"), Some("cmyk"));
        assert_eq!(again.values.text("password"), Some(""));
        again.remember = false;
        let (format, values) = again.preset().expect("a format");
        kept.save(&format, values);
        assert!(kept.load("pdf").is_none(), "forgotten");
    }

    /// The text of `code`, drawn in `placement`, in the exported `image`.
    fn scanned(
        image: &image::DynamicImage,
        code: &gafira_doc::QrNode,
        placement: &gafira_doc::Placement,
    ) -> Option<String> {
        let drawing = gafira_qr::draw(code).expect("drawn");
        let (k, (x0, y0)) = drawing.fit(placement.w, placement.h).expect("a box");
        let grey = image.to_luma8();
        let px = |v: f64, most: u32| (v.max(0.0) as u32).min(most - 1);
        gafira_testkit::qr::read(drawing.size as usize, |x, y| {
            let (x, y) = (placement.x + x0 + x * k, placement.y + y0 + y * k);
            grey.get_pixel(px(x, grey.width()), px(y, grey.height()))[0]
        })
    }

    // S80-12: a QR code with shapes, a gradient and a logo goes out in every
    // format of the app, from the engine, and the files are read back.
    #[test]
    fn s80_12_a_qr_code_exports_in_every_format() {
        use gafira_doc::{QrBody, QrEyeBall, QrEyeFrame, QrFill, QrLogo};
        let dir = std::env::temp_dir().join(format!("gafira-s80-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let logo = dir.join("logo.png");
        image::RgbaImage::from_pixel(32, 24, image::Rgba([220, 40, 40, 255]))
            .save(&logo)
            .expect("a png");
        let store = Arc::new(gafira_assets::FsStore::new(dir.join("store")));
        let mut engine =
            crate::editor::tests::engine_with(Arc::new(crate::editor::tests::Nothing), store);
        start(&mut engine, StartFormat::Social).expect("a document");
        let asset = engine.assets().import(&logo).expect("imports");
        let text = "https://example.com/gafira?qr=1";
        let mut code = crate::qr::fresh();
        code.content = gafira_doc::QrContent::Url {
            url: text.to_owned(),
        };
        code.style.body = QrBody::FluidCorners;
        code.style.eye_frame = QrEyeFrame::Leaf;
        code.style.eye_ball = QrEyeBall::Flower;
        code.style.fill = QrFill::Gradient(crate::colors::gradient(0x0018_243b, 0x004a_2c82, 90.0));
        code.logo = Some(QrLogo {
            asset,
            natural: (32, 24),
            share: 0.22,
            clear_behind: true,
        });
        let document = engine.edit().document().expect("a document");
        let page = &document.pages()[0];
        let size = page.size.to_px(document.dpi()).expect("px");
        let spec = crate::qr::spec(code.clone(), (size.width(), size.height()), None);
        let placement = spec.placement;
        engine
            .edit()
            .execute(Box::new(gafira_edit::InsertNodes::new(
                page.id,
                None,
                0,
                vec![spec],
            )))
            .expect("the code");
        for id in ["png", "pdf", "svg", "pptx"] {
            let dialog = dialog_on(&engine, id, None);
            let path = dir.join(format!("code.{id}"));
            export_to(&mut engine, &dialog, &path).expect(id);
            let bytes = std::fs::read(&path).expect("the file");
            match id {
                "png" | "jpeg" => {
                    let image = image::load_from_memory(&bytes).expect("an image");
                    assert_eq!(
                        scanned(&image, &code, &placement).as_deref(),
                        Some(text),
                        "{id}"
                    );
                }
                "webp" => assert_eq!(&bytes[8..12], b"WEBP"),
                "avif" => assert_eq!(&bytes[4..12], b"ftypavif"),
                "pdf" => assert!(bytes.starts_with(b"%PDF")),
                "svg" => {
                    use resvg::{tiny_skia, usvg};
                    let options = usvg::Options::default();
                    let tree = usvg::Tree::from_data(&bytes, &options).expect("an SVG");
                    let side = tree.size().to_int_size();
                    let mut pixmap =
                        tiny_skia::Pixmap::new(side.width(), side.height()).expect("a size");
                    let at = tiny_skia::Transform::identity();
                    resvg::render(&tree, at, &mut pixmap.as_mut());
                    let image = image::RgbaImage::from_raw(
                        side.width(),
                        side.height(),
                        pixmap.data().to_vec(),
                    );
                    let image = image::DynamicImage::from(image.expect("pixels"));
                    assert_eq!(
                        scanned(&image, &code, &placement).as_deref(),
                        Some(text),
                        "svg"
                    );
                }
                _ => assert!(bytes.starts_with(b"PK"), "a package"),
            }
        }
        if std::env::var_os("GAFIRA_KEEP_EXPORTS").is_none() {
            let _ = std::fs::remove_dir_all(&dir);
        }
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

    // S81-12 to S81-14: a table goes out in every format from the engine:
    // Excel only with tables, the words of the PDF as text, not pixels.
    #[test]
    fn s81_14_a_table_exports_in_every_format() {
        let mut engine = document_engine(1);
        let dir = std::env::temp_dir().join(format!("gafira-s81-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let document = engine.edit().document().expect("a document");
        let dialog = dialog_on(&engine, "xlsx", None);
        assert_eq!(request(&dialog, &document).err(), Some(Refusal::NoTables));
        let page = &document.pages()[0];
        let size = page.size.to_px(document.dpi()).expect("px");
        let texts = Texts::new(gafira_ports::i18n::Locale::Es);
        let spec = crate::table_templates::spec(
            crate::table_templates::Template::Budget.table(&texts),
            (size.width(), size.height()),
            None,
            "Presupuesto".to_owned(),
        );
        engine
            .edit()
            .execute(Box::new(gafira_edit::InsertNodes::new(
                page.id,
                None,
                0,
                vec![spec],
            )))
            .expect("the table");
        for id in ["png", "jpeg", "webp", "avif", "pdf", "svg", "pptx", "xlsx"] {
            let dialog = dialog_on(&engine, id, None);
            let path = dir.join(format!("table.{id}"));
            export_to(&mut engine, &dialog, &path).expect(id);
            let bytes = std::fs::read(&path).expect("the file");
            match id {
                "svg" => {
                    let svg = String::from_utf8(bytes).expect("utf-8");
                    assert!(svg.contains("<text"), "the words are text");
                    assert!(!svg.contains("<image"), "nothing is pixels");
                }
                "pdf" => {
                    let has = |what: &[u8]| bytes.windows(what.len()).any(|w| w == what);
                    assert!(has(b"/ToUnicode"), "the words can be copied");
                    assert!(!has(b"/Subtype /Image") && !has(b"/Subtype/Image"));
                }
                _ => assert!(bytes.len() > 500, "{id}"),
            }
        }
        if std::env::var_os("GAFIRA_KEEP_EXPORTS").is_none() {
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    // S83-09: every kind of chart goes out in every format from the engine:
    // in the PDF and the SVG, paths and words, no pixels.
    #[test]
    fn s83_09_charts_export_in_every_format() {
        let mut engine = document_engine(1);
        let dir = std::env::temp_dir().join(format!("gafira-s83-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let document = engine.edit().document().expect("a document");
        let page = &document.pages()[0];
        let size = page.size.to_px(document.dpi()).expect("px");
        let texts = Texts::new(gafira_ports::i18n::Locale::Es);
        let (w, h) = (size.width() / 6.0, size.height() / 6.0);
        let specs = crate::chart_templates::TEMPLATES
            .iter()
            .enumerate()
            .map(|(i, template)| {
                let chart = template.chart(&texts);
                let name = template.key.to_owned();
                let room = (size.width(), size.height());
                let mut spec = crate::chart_templates::spec(chart, room, None, name);
                // A grid of six by six, the words in proportion.
                let (col, row) = (f64::from(i as u32 % 6), f64::from(i as u32 / 6));
                let k = w * 0.9 / spec.placement.w;
                spec.placement = gafira_doc::Placement::new(
                    (col + 0.05) * w,
                    (row + 0.05) * h,
                    w * 0.9,
                    h * 0.9,
                );
                if let gafira_doc::NodeKind::Chart(chart) = &mut spec.kind {
                    chart.text.size = (chart.text.size * k).max(1.0);
                    chart.weight *= k;
                }
                spec
            })
            .collect();
        let insert = gafira_edit::InsertNodes::new(page.id, None, 0, specs);
        engine.edit().execute(Box::new(insert)).expect("the charts");
        for id in ["png", "jpeg", "webp", "avif", "pdf", "svg", "pptx"] {
            let dialog = dialog_on(&engine, id, None);
            let path = dir.join(format!("charts.{id}"));
            export_to(&mut engine, &dialog, &path).expect(id);
            let bytes = std::fs::read(&path).expect("the file");
            match id {
                "svg" => {
                    let svg = String::from_utf8(bytes).expect("utf-8");
                    assert!(svg.contains("<text"), "the words are text");
                    assert!(!svg.contains("<image"), "nothing is pixels");
                }
                "pdf" => {
                    let has = |what: &[u8]| bytes.windows(what.len()).any(|w| w == what);
                    assert!(has(b"/ToUnicode"), "the words can be copied");
                    assert!(!has(b"/Subtype /Image") && !has(b"/Subtype/Image"));
                }
                _ => assert!(bytes.len() > 500, "{id}"),
            }
        }
        if std::env::var_os("GAFIRA_KEEP_EXPORTS").is_none() {
            let _ = std::fs::remove_dir_all(&dir);
        }
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
            ["png", "jpeg", "webp", "avif", "pdf", "svg", "pptx", "xlsx"]
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
        assert_eq!(jpeg.options.choice("quality"), Some("high"));
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

    // S89-03: a text with effects is drawn in every format, and its words
    // stay text in SVG and PDF, also with no inside.
    #[test]
    fn s89_03_a_text_with_effects_exports_in_every_format() {
        use gafira_doc::{NodeKind, Placement, SpanStyle, TextEffect, TextEffectKind, TextNode};
        let dir = std::env::temp_dir().join(format!("gafira-ui-effects-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let style = SpanStyle {
            font: "Inter".to_owned(),
            size: 60.0,
            color: crate::colors::hex(0x18_24_3B),
            bold: true,
            italic: false,
        };
        for kind in TextEffectKind::ALL {
            let mut engine = document_engine(1);
            let document = engine.edit().document().expect("a snapshot");
            let page = document.pages()[0].id;
            let mut text = TextNode::plain("Hola", &style);
            text.effects = vec![TextEffect::new(kind)];
            let place = Placement::new(40.0, 40.0, 300.0, 120.0);
            let spec = gafira_edit::NodeSpec::new(NodeKind::Text(text), place);
            let insert = gafira_edit::InsertNodes::new(page, None, 0, vec![spec]);
            engine.edit().execute(Box::new(insert)).expect("the text");
            for id in ["png", "pdf", "svg", "pptx"] {
                let dialog = dialog_on(&engine, id, None);
                let path = dir.join(format!("text.{id}"));
                export_to(&mut engine, &dialog, &path).expect(id);
                let bytes = std::fs::read(&path).expect("the file");
                match id {
                    "svg" => {
                        let svg = String::from_utf8(bytes).expect("utf-8");
                        assert!(svg.contains("<text"), "{kind:?}");
                    }
                    "pdf" => {
                        let has = |what: &[u8]| bytes.windows(what.len()).any(|w| w == what);
                        assert!(has(b"/Font"), "{kind:?}");
                    }
                    _ => assert!(!bytes.is_empty()),
                }
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    // S39-04: the pages follow the format until the person chose them.
    #[test]
    fn s39_04_the_pages_follow_the_format_until_they_are_chosen() {
        let engine = document_engine(1);
        let mut dialog = dialog_on(&engine, "png", None);
        assert_eq!(dialog.pages, Pages::Current);
        let pdf = dialog.formats.iter().position(|f| f.id == "pdf").unwrap();
        dialog.pick(pdf);
        assert_eq!(dialog.pages, Pages::All, "PDF");
        dialog.choose_pages(Pages::Current);
        dialog.pick(0);
        dialog.pick(pdf);
        assert_eq!(dialog.pages, Pages::Current, "chosen");
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
            ("avif", dir.join("cartel.avif"), b"\0\0\0"),
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
        assert_eq!(left.len(), outputs.len(), "no temporary is left: {left:?}");
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
