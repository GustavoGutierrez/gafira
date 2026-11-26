//! A bulk run on its own thread (spec 104 §10.3): the rows filled in, the
//! files written one after another, and what the window shows meanwhile:
//! the phase, how many are done, failed and left, the file being written,
//! the time it takes and a way to cancel.
//!
//! Exports of files one by one resolve a row at a time, so a large run never
//! holds every page (AC 12); a combined PDF and the editable `.gaf` need
//! every page in one document.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use gafira_bulk::{FieldRef, Names, Resolved};
use gafira_doc::contract::{CancelToken, Progress};
use gafira_doc::{DataSet, Document, Node, NodeKind, PageId, TextSizing};
use gafira_engine::{DocumentSaver, PreparedExport};
use gafira_ports::export::ExportScope;

use crate::canvas;

/// What a run is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    /// Filling the rows into the template.
    Preparing,
    /// Writing the files.
    Exporting,
    /// Writing the editable `.gaf`.
    Saving,
    /// Every row went through.
    Done,
    /// The person stopped it; the files written stay.
    Cancelled,
}

/// Something of one row that did not go as it should.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Note {
    /// A file of the row could not be written.
    Failed {
        row: usize,
        file: String,
        why: String,
    },
    /// A text of the row does not fit its box (§9).
    Overflow { row: usize, element: String },
}

/// What the window shows of a run.
#[derive(Clone, Debug)]
pub(crate) struct Status {
    pub phase: Phase,
    /// Units of the phase done and in all (rows, files or pages).
    pub done: usize,
    pub total: usize,
    pub failed: usize,
    /// Files written in all.
    pub written: usize,
    /// The file being written, or the row being filled.
    pub current: String,
    pub started: Instant,
    /// When the phase started, for the time left.
    pub phase_started: Instant,
    pub notes: Vec<Note>,
    /// A failure that stopped the whole run.
    pub stopped: Option<String>,
    /// Where the files go.
    pub place: Option<PathBuf>,
    /// How long the whole run took, once it ended.
    pub took: Option<Duration>,
    /// The first file written, for «Mostrar en carpeta».
    pub first: Option<PathBuf>,
}

impl Status {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            phase: Phase::Preparing,
            done: 0,
            total: 0,
            failed: 0,
            written: 0,
            current: String::new(),
            started: now,
            phase_started: now,
            notes: Vec::new(),
            stopped: None,
            place: None,
            took: None,
            first: None,
        }
    }

    /// The fraction of the phase done, in `[0, 1]`.
    pub(crate) fn fraction(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        (self.done as f32 / self.total as f32).clamp(0.0, 1.0)
    }

    /// The time the phase still needs, once there are enough units to
    /// tell (§10.3: no estimate before).
    pub(crate) fn left(&self) -> Option<Duration> {
        let enough =
            self.done >= 3 || (self.done > 0 && self.phase_started.elapsed().as_secs() >= 2);
        if !enough || self.done >= self.total {
            return None;
        }
        let per = self.phase_started.elapsed().as_secs_f64() / self.done as f64;
        Some(Duration::from_secs_f64(
            per * (self.total - self.done) as f64,
        ))
    }
}

/// Where the files go.
#[derive(Clone, Debug)]
pub(crate) enum Target {
    /// One file: a combined PDF or the `.gaf`.
    File(PathBuf),
    /// A folder for a file per row or per page.
    Folder(PathBuf),
}

/// The format of the files of a run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Written {
    pub extension: String,
    /// The format holds several pages (PDF, PowerPoint): a file per row,
    /// or one file with every row when `one_file`.
    pub multipage: bool,
    pub one_file: bool,
}

impl Written {
    /// One file with every page of every row.
    fn combined(&self) -> bool {
        self.multipage && self.one_file
    }
}

/// Everything a run needs, taken on the window thread.
pub(crate) struct Job {
    pub template: Arc<Document>,
    pub set: Arc<DataSet>,
    pub rows: Vec<usize>,
    pub pages: Vec<PageId>,
    /// The format the files are written in; none writes only the `.gaf`.
    pub format: Option<Written>,
    pub target: Target,
    /// The editable `.gaf` to write too, if asked.
    pub gaf: Option<PathBuf>,
    pub file_name: String,
    /// The checked export of the format, to repeat over each document.
    pub export: Option<PreparedExport>,
    pub saver: Option<DocumentSaver>,
}

/// A run on its thread.
pub(crate) struct Run {
    status: Arc<Mutex<Status>>,
    cancel: CancelToken,
    thread: Option<JoinHandle<()>>,
    over: Arc<AtomicBool>,
}

impl Run {
    /// Starts `job`; `wake` asks the window to draw again.
    pub(crate) fn start(job: Job, wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        let mut first = Status::new();
        first.place = Some(match &job.target {
            Target::File(path) => path.clone(),
            Target::Folder(folder) => folder.clone(),
        });
        let status = Arc::new(Mutex::new(first));
        let cancel = CancelToken::new();
        let over = Arc::new(AtomicBool::new(false));
        let thread = {
            let (status, cancel, over) = (Arc::clone(&status), cancel.clone(), Arc::clone(&over));
            std::thread::Builder::new()
                .name("gafira-bulk".to_owned())
                .spawn(move || {
                    let worker = Worker {
                        status: &status,
                        cancel: &cancel,
                        wake: &*wake,
                        last_wake: Mutex::new(Instant::now()),
                    };
                    worker.run(job);
                    over.store(true, Ordering::SeqCst);
                    wake();
                })
        };
        if thread.is_err() {
            let mut s = status.lock().unwrap_or_else(PoisonError::into_inner);
            s.stopped = Some("the thread of the run could not start".to_owned());
        }
        Self {
            status,
            cancel,
            thread: thread.ok(),
            over,
        }
    }

    /// What it is doing now.
    pub(crate) fn status(&self) -> Status {
        self.status
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Asks it to stop after the file being written; what is written stays.
    pub(crate) fn cancel(&self) {
        self.cancel.cancel();
    }

    pub(crate) fn is_cancelling(&self) -> bool {
        self.cancel.is_cancelled()
    }

    /// Whether the thread ended; then it is joined.
    pub(crate) fn poll(&mut self) -> bool {
        if !self.over.load(Ordering::SeqCst) && self.thread.is_some() {
            return false;
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            let mut s = self.status.lock().unwrap_or_else(PoisonError::into_inner);
            s.stopped
                .get_or_insert_with(|| "the run stopped unexpectedly".to_owned());
        }
        true
    }
}

impl Run {
    /// Cancel and wait: the window is closing.
    pub(crate) fn stop(mut self) {
        self.cancel();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The side of the run that works.
struct Worker<'a> {
    status: &'a Mutex<Status>,
    cancel: &'a CancelToken,
    wake: &'a (dyn Fn() + Send + Sync),
    last_wake: Mutex<Instant>,
}

/// The progress of one export, as pages of the phase.
struct Pages<'a> {
    worker: &'a Worker<'a>,
    from: usize,
    count: usize,
}

impl Progress for Pages<'_> {
    fn report(&self, fraction: f32) {
        // A page count: the cast keeps whole pages.
        let done = (fraction.clamp(0.0, 1.0) * self.count as f32) as usize;
        self.worker
            .update(|s| s.done = self.from + done.min(self.count));
    }
}

impl Worker<'_> {
    fn update(&self, change: impl FnOnce(&mut Status)) {
        change(&mut self.status.lock().unwrap_or_else(PoisonError::into_inner));
        // The window draws at most some 30 times a second for a run.
        let mut last = self
            .last_wake
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if last.elapsed() >= Duration::from_millis(33) {
            *last = Instant::now();
            (self.wake)();
        }
    }

    fn phase(&self, phase: Phase, total: usize) {
        self.update(|s| {
            s.phase = phase;
            s.done = 0;
            s.total = total;
            s.phase_started = Instant::now();
        });
    }

    fn cancelled(&self) -> bool {
        if self.cancel.is_cancelled() {
            self.update(|s| s.phase = Phase::Cancelled);
            return true;
        }
        false
    }

    fn run(&self, mut job: Job) {
        let saver = job.saver.take();
        let combined = job.format.as_ref().is_none_or(Written::combined);
        let whole = if combined || job.gaf.is_some() {
            match self.resolve_all(&job) {
                Some(resolved) => Some(resolved),
                None => return,
            }
        } else {
            None
        };
        match (&job.format, &whole) {
            (Some(format), Some(resolved)) if format.combined() => {
                if !self.export_combined(&job, resolved) {
                    return;
                }
            }
            (Some(format), _) if !format.combined() => {
                if !self.export_each(&job, format) {
                    return;
                }
            }
            _ => {}
        }
        if let (Some(resolved), Some(path)) = (whole, gaf_path(&job)) {
            if self.cancelled() {
                return;
            }
            if let Some(saver) = saver {
                self.save(saver, resolved.document, &path);
            }
        }
        self.update(|s| {
            if s.phase != Phase::Cancelled {
                s.phase = Phase::Done;
                s.done = s.total;
            }
        });
    }

    /// Every row in one document, with what does not fit noted.
    fn resolve_all(&self, job: &Job) -> Option<Resolved> {
        self.phase(Phase::Preparing, job.rows.len());
        let mut go_on = |done: usize| {
            self.update(|s| {
                s.done = done;
                s.current = format!("{}", job.rows.get(done).map_or(0, |r| r + 1));
            });
            !self.cancel.is_cancelled()
        };
        let resolved = gafira_bulk::resolve_with(
            &job.template,
            &job.set,
            &job.rows,
            &job.pages,
            &mut go_on,
            &fit_node,
        );
        match resolved {
            Ok(Some(resolved)) => {
                self.note_overflow(&resolved);
                Some(resolved)
            }
            Ok(None) => {
                self.update(|s| s.phase = Phase::Cancelled);
                None
            }
            Err(error) => {
                self.update(|s| s.stopped = Some(error.to_string()));
                None
            }
        }
    }

    fn note_overflow(&self, resolved: &Resolved) {
        if resolved.overflow.is_empty() {
            return;
        }
        let notes: Vec<Note> = resolved
            .overflow
            .iter()
            .filter_map(|(page, node)| {
                let at = resolved
                    .document
                    .pages()
                    .iter()
                    .position(|p| p.id == *page)?;
                let element = resolved.document.page(*page)?.node(*node)?;
                Some(Note::Overflow {
                    row: resolved.rows.get(at).copied()? + 1,
                    element: element_name(element),
                })
            })
            .collect();
        self.update(|s| s.notes.extend(notes));
    }

    /// The combined PDF: every page, in order.
    fn export_combined(&self, job: &Job, resolved: &Resolved) -> bool {
        let (Some(export), Target::File(path)) = (&job.export, &job.target) else {
            return true;
        };
        let pages: Vec<PageId> = resolved.document.pages().iter().map(|p| p.id).collect();
        self.phase(Phase::Exporting, pages.len());
        let name = file_name(path);
        self.update(|s| s.current.clone_from(&name));
        let progress = Pages {
            worker: self,
            from: 0,
            count: pages.len(),
        };
        let run = export.again(
            Arc::new(resolved.document.clone()),
            ExportScope::Pages(pages),
            path.clone(),
        );
        match run.run(&progress, self.cancel) {
            Ok(_) => self.update(|s| {
                s.written += 1;
                s.first.get_or_insert_with(|| path.clone());
            }),
            Err(_) if self.cancel.is_cancelled() => {
                self.update(|s| s.phase = Phase::Cancelled);
                return false;
            }
            Err(error) => {
                self.update(|s| s.stopped = Some(error.to_string()));
                return false;
            }
        }
        true
    }

    /// A file per row, or per row and page, a row resolved at a time.
    fn export_each(&self, job: &Job, format: &Written) -> bool {
        let (Some(export), Target::Folder(folder)) = (&job.export, &job.target) else {
            return true;
        };
        let per_row = if format.multipage { 1 } else { job.pages.len() };
        let extension = format.extension.as_str();
        self.phase(Phase::Exporting, job.rows.len() * per_row);
        let mut names = taken(folder);
        for &row in &job.rows {
            if self.cancelled() {
                return false;
            }
            let resolved = gafira_bulk::resolve_with(
                &job.template,
                &job.set,
                &[row],
                &job.pages,
                &mut |_| true,
                &fit_node,
            );
            let resolved = match resolved {
                Ok(Some(resolved)) => resolved,
                Ok(None) => return false,
                Err(error) => {
                    let why = error.to_string();
                    self.fail_row(row, String::new(), why, per_row);
                    continue;
                }
            };
            self.note_overflow(&resolved);
            let stem = stem(&job.file_name, &job.set, row);
            let document = Arc::new(resolved.document);
            let pages: Vec<PageId> = document.pages().iter().map(|p| p.id).collect();
            let files: Vec<(Vec<PageId>, String)> = if per_row == 1 {
                vec![(pages, names.unique(&stem, extension))]
            } else {
                pages
                    .iter()
                    .enumerate()
                    .map(|(k, page)| {
                        let stem = if pages.len() > 1 {
                            format!("{stem}-p{}", k + 1)
                        } else {
                            stem.clone()
                        };
                        (vec![*page], names.unique(&stem, extension))
                    })
                    .collect()
            };
            for (scope, file) in files {
                if self.cancelled() {
                    return false;
                }
                self.update(|s| s.current.clone_from(&file));
                let path = folder.join(&file);
                let run = export.again(
                    Arc::clone(&document),
                    ExportScope::Pages(scope),
                    path.clone(),
                );
                match run.run(&gafira_doc::contract::NoProgress, self.cancel) {
                    Ok(_) => self.update(|s| {
                        s.written += 1;
                        s.done += 1;
                        s.first.get_or_insert(path);
                    }),
                    Err(_) if self.cancel.is_cancelled() => {
                        self.update(|s| s.phase = Phase::Cancelled);
                        return false;
                    }
                    Err(error) => self.fail_row(row, file, error.to_string(), 1),
                }
            }
        }
        true
    }

    fn fail_row(&self, row: usize, file: String, why: String, units: usize) {
        self.update(|s| {
            s.failed += units;
            s.done += units;
            s.notes.push(Note::Failed {
                row: row + 1,
                file,
                why,
            });
        });
    }

    /// The editable `.gaf` with every page.
    fn save(&self, saver: DocumentSaver, document: Document, path: &Path) {
        self.phase(Phase::Saving, 1);
        self.update(|s| s.current = file_name(path));
        match saver.run(document, path) {
            Ok(()) => self.update(|s| {
                s.written += 1;
                s.done = 1;
                s.first.get_or_insert_with(|| path.to_path_buf());
            }),
            Err(error) => self.update(|s| s.stopped = Some(error.to_string())),
        }
    }
}

/// Where the editable `.gaf` goes, if anywhere.
fn gaf_path(job: &Job) -> Option<PathBuf> {
    match (&job.format, &job.target) {
        (None, Target::File(path)) => Some(path.clone()),
        _ => job.gaf.clone(),
    }
}

/// The name of a file, for the person.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The names already in `folder`: none is written over (AC 10).
fn taken(folder: &Path) -> Names {
    let mut names = Names::default();
    if let Ok(entries) = std::fs::read_dir(folder) {
        for entry in entries.flatten() {
            names.take(&entry.file_name().to_string_lossy());
        }
    }
    names
}

/// The stem of the files of `row`: the name with its fields, made safe.
pub(crate) fn stem(template: &str, set: &DataSet, row: usize) -> String {
    let value = |field: &FieldRef| gafira_bulk::lookup(set, row, field);
    let (name, _) = gafira_bulk::resolve_plain(template, &value);
    gafira_bulk::sanitize(&name)
}

/// What an element is called for the person: its name, else its words.
pub(crate) fn element_name(node: &Node) -> String {
    if let Some(name) = node.base.name.as_ref().filter(|n| !n.trim().is_empty()) {
        return name.clone();
    }
    let words = match &node.kind {
        NodeKind::Text(text) => text.content.plain_text(),
        NodeKind::Shape(shape) => shape
            .text
            .as_ref()
            .map(|t| t.content.plain_text())
            .unwrap_or_default(),
        _ => String::new(),
    };
    let short: String = words.chars().take(32).collect();
    if words.chars().count() > 32 {
        format!("{short}…")
    } else {
        short
    }
}

/// The box of a filled text made to follow its new words, as the canvas
/// does while writing (specs 061, 095): wider for one that grows sideways,
/// taller for one that grows down. Whether its words still do not fit: a
/// box of fixed size that is too small for them (§9 «Avisar»).
pub(crate) fn fit_node(node: &mut Node) -> bool {
    let NodeKind::Text(text) = &node.kind else {
        return false;
    };
    let mut placed = node.base.placement;
    if text.sizing == TextSizing::AutoWidth
        && let Some(natural) = canvas::natural_width(text)
        && let Some(wide) = canvas::text_width_fit(text.align, &placed, natural)
    {
        placed = wide;
    }
    let laid = canvas::curved_height(text, placed.w.abs().max(1.0), placed.h.abs().max(1.0));
    let mut over = false;
    if let Some(laid) = laid {
        match text.sizing {
            TextSizing::Fixed => over = laid > placed.h.abs() + 0.5,
            sizing => {
                if let Some(tall) = canvas::text_fit(sizing, &placed, laid) {
                    placed = tall;
                }
            }
        }
    }
    node.base.placement = placed;
    over
}

/// A spreadsheet read on its own thread, every row of it (spec 104 §1: the
/// window stays available while a file comes in).
pub(crate) struct Reading {
    pub path: PathBuf,
    receiver: mpsc::Receiver<Result<Vec<gafira_io_xlsx::ReadSheet>, crate::sheets::Failure>>,
}

impl Reading {
    pub(crate) fn start(
        path: PathBuf,
        locale: gafira_io_xlsx::Locale,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel();
        let file = path.clone();
        let reach = gafira_io_xlsx::Reach {
            rows: DataSet::MAX_ROWS + 1,
            columns: DataSet::MAX_COLUMNS,
        };
        // Without a thread the receiver reads as disconnected: a failure.
        let _ = std::thread::Builder::new()
            .name("gafira-bulk-read".to_owned())
            .spawn(move || {
                let _ = sender.send(crate::sheets::read_reach(&file, locale, reach));
                wake();
            });
        Self { path, receiver }
    }

    /// What was read, once it was.
    pub(crate) fn poll(
        &self,
    ) -> Option<Result<Vec<gafira_io_xlsx::ReadSheet>, crate::sheets::Failure>> {
        match self.receiver.try_recv() {
            Ok(found) => Some(found),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(crate::sheets::Failure::Open)),
        }
    }
}

/// The run of the connected data of `document` over `rows`, writing to
/// `target` in `format` with `options` (none: only the `.gaf`) and, if
/// asked, the editable `.gaf` at `gaf`; the export checked by the engine.
///
/// # Errors
/// What the engine says when the export is refused; a document without
/// connected data is [`gafira_engine::EngineError::NoSession`].
pub(crate) fn job(
    engine: &gafira_engine::Gafira,
    document: &Arc<Document>,
    (rows, target, gaf): (Vec<usize>, Target, Option<PathBuf>),
    format: Option<(&str, Written, gafira_doc::contract::ParamValues)>,
) -> Result<Job, gafira_engine::EngineError> {
    let set = document
        .data_arc()
        .cloned()
        .ok_or(gafira_engine::EngineError::NoSession)?;
    let pages = gafira_bulk::template_pages(document, &set);
    let (export, written) = match format {
        Some((id, written, options)) => {
            let request = gafira_ports::export::ExportRequest {
                format_id: id.to_owned(),
                scope: ExportScope::Pages(pages.clone()),
                options,
            };
            let prepared =
                engine
                    .export()
                    .prepare_document(Arc::clone(document), request, PathBuf::new())?;
            (Some(prepared), Some(written))
        }
        None => (None, None),
    };
    let saver = (gaf.is_some() || written.is_none()).then(|| engine.projects().document_saver());
    Ok(Job {
        template: Arc::clone(document),
        file_name: set.profile.file_name.clone(),
        set,
        rows,
        pages,
        format: written,
        target,
        gaf,
        export,
        saver,
    })
}

/// The picture of `row` on the template page `page`, as large as fits in
/// `fit`: the row resolved apart, its boxes following their words, drawn by
/// the export renderers. A picture of nothing when it cannot be drawn.
pub(crate) fn row_picture(
    engine: &gafira_engine::Gafira,
    (document, set): (&Document, &DataSet),
    (row, page): (usize, PageId),
    fit: (u32, u32),
) -> egui::ColorImage {
    let resolved =
        gafira_bulk::resolve_with(document, set, &[row], &[page], &mut |_| true, &fit_node);
    let picture = match resolved {
        Ok(Some(resolved)) => resolved.document.pages().first().and_then(|first| {
            engine
                .export()
                .preview_document(&resolved.document, first.id, fit)
                .ok()
        }),
        _ => None,
    };
    match picture {
        Some(p) => {
            egui::ColorImage::from_rgba_unmultiplied([p.width as usize, p.height as usize], &p.rgba)
        }
        None => egui::ColorImage::filled([1, 1], egui::Color32::TRANSPARENT),
    }
}
