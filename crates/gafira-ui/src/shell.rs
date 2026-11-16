//! The window, the screens and the way between them (03 §6.1).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gafira_engine::{AutosaveOutcome, EngineError, Gafira, RecoveryEntry, Waker};
use gafira_ports::i18n::Locale;
use gafira_ports::settings::Appearance;
use gafira_theme::ThemeTokens;

use crate::dialogs::{self, LeaveChoice, RecoveryChoice};
use crate::editor::Editor;
use crate::export;
use crate::files::{self, FileAction, Outcome};
use crate::format::StartFormat;
use crate::home::Home;
use crate::recent::{self, RecentStore};
use crate::splash;
use crate::style::Theme;
use crate::text::Texts;

/// Builds the engine once the window exists, with the waker that repaints
/// it when an event arrives (02 §5.2).
pub type EngineFactory = Box<dyn FnOnce(Arc<dyn Waker>) -> Result<Gafira, EngineError>>;

/// What the composition root hands the shell.
pub struct UiConfig {
    /// The interface language, resolved by the D-08 rule. `None` takes the
    /// one the engine's settings resolve to (spec 009).
    pub locale: Option<Locale>,
    /// The app version the Startup screen shows.
    pub version: &'static str,
    /// The engine, built inside the window's creation.
    pub engine: EngineFactory,
    /// The recent projects of Inicio (spec 025).
    pub recent: Box<dyn RecentStore>,
}

/// Wakes egui when the engine publishes, so the window repaints.
struct RepaintWaker(egui::Context);

impl Waker for RepaintWaker {
    fn wake(&self) {
        self.0.request_repaint();
    }
}

/// The screen on display.
enum Screen {
    Splash,
    Home(Home),
    /// Boxed: the editor holds the canvas state and its textures.
    Editor(Box<Editor>),
}

/// What a screen asks the shell to do after drawing.
pub(crate) enum Go {
    Stay,
    Home,
    Editor(StartFormat),
    File(FileAction),
    /// Open a project of «Recientes».
    OpenRecent(PathBuf),
    /// Switch the interface to dark (`true`) or light, and remember it.
    Dark(bool),
    /// Open «Exportar» for the page of the selection, if any (spec 030).
    Export(Option<gafira_doc::PageId>),
}

struct Shell {
    engine: Gafira,
    recent: Box<dyn RecentStore>,
    theme: Theme,
    texts: Texts,
    version: &'static str,
    screen: Screen,
    /// How long the Startup screen has been up, in seconds.
    elapsed: f32,
    /// The last failure, shown in the editor's status line until the next
    /// action.
    notice: Option<String>,
    /// The autosave job running on its thread, if any.
    autosave: Option<mpsc::Receiver<AutosaveOutcome>>,
    /// Whether the recovery scan already ran.
    scanned: bool,
    /// The projects a previous run left slots for (03 §7.9).
    recovery: Vec<RecoveryEntry>,
    /// The entry whose «Descartar» waits for confirmation.
    confirm_discard: Option<usize>,
    /// Why the last recovery action failed.
    recovery_failure: Option<String>,
    /// The destination «¿Guardar los cambios?» holds back.
    leaving: Option<Leave>,
    /// «Exportar» while its dialog is open (spec 030).
    export: Option<export::Dialog>,
}

/// Open the main window and run until it closes (03 §7.1: minimum
/// 1280 × 720 logical px).
///
/// # Errors
/// When the window or its graphics context cannot be created.
pub fn run(config: UiConfig) -> eframe::Result {
    let tokens = ThemeTokens::embedded().map_err(|e| eframe::Error::AppCreation(Box::new(e)))?;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Gafira")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1280.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Gafira",
        options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            let mut theme = Theme::new(tokens);
            let engine = (config.engine)(Arc::new(RepaintWaker(cc.egui_ctx.clone())))
                .map_err(|e| e.to_string())?;
            // A store that cannot be read leaves the defaults in force and
            // announced; the error is only for the log (S9-04).
            let resolved = match engine.settings().start() {
                Ok(resolved) => resolved,
                Err(error) => {
                    eprintln!("settings: {error}");
                    engine.settings().locale()
                }
            };
            theme.dark = engine.settings().get().appearance == Appearance::Dark;
            theme.apply(&cc.egui_ctx);
            Ok(Box::new(Shell {
                engine,
                recent: config.recent,
                theme,
                texts: Texts::new(config.locale.unwrap_or(resolved.locale)),
                version: config.version,
                screen: Screen::Splash,
                elapsed: 0.0,
                notice: None,
                autosave: None,
                scanned: false,
                recovery: Vec::new(),
                confirm_discard: None,
                recovery_failure: None,
                leaving: None,
                export: None,
            }))
        }),
    )
}

/// Where the person was going when «¿Guardar los cambios?» stopped them.
#[derive(Clone)]
enum Leave {
    Home,
    Open,
    Recent(PathBuf),
}

impl Shell {
    /// Pump the autosave (02 §7.2): hand the due job to a thread and account
    /// for it when it comes back. The engine schedules; the host runs (P13).
    fn pump_autosave(&mut self, ctx: &egui::Context) {
        if let Some(pending) = &self.autosave {
            match pending.try_recv() {
                Ok(outcome) => {
                    if let Some(error) = outcome.error() {
                        eprintln!("autosave: {error}");
                    }
                    self.engine.projects().autosave_finished(outcome);
                    self.autosave = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => self.autosave = None,
            }
        }
        if self.autosave.is_none()
            && let Some(job) = self.engine.projects().autosave_due()
        {
            let (sender, receiver) = mpsc::channel();
            let waker = ctx.clone();
            std::thread::spawn(move || {
                let _ = sender.send(job.run());
                waker.request_repaint();
            });
            self.autosave = Some(receiver);
        }
        // The engine has no timer: look again in a second.
        ctx.request_repaint_after(Duration::from_secs(1));
    }

    /// Leave the editor for `to`, closing the session (its slots go with it).
    fn proceed(&mut self, to: Leave) {
        self.engine.projects().close();
        self.screen = self.home();
        match to {
            Leave::Home => {}
            Leave::Open => self.file(FileAction::Open),
            Leave::Recent(path) => self.open_recent(&path),
        }
    }

    /// Inicio with the recent projects as they are now.
    fn home(&self) -> Screen {
        Screen::Home(Home::new(self.recent.list()))
    }

    /// Put the open project in «Recientes» (spec 025).
    fn remember(&self) {
        if let Some(path) = self.engine.projects().path() {
            recent::remember(self.recent.as_ref(), &path);
        }
    }

    /// Open a project of «Recientes». When it fails, Inicio reads the list
    /// again, so a file that went away leaves it.
    fn open_recent(&mut self, path: &Path) {
        self.notice = None;
        match self.engine.projects().open(path) {
            Ok(()) => {
                self.remember();
                self.screen = Screen::Editor(Box::default());
            }
            Err(error) => {
                eprintln!("open recent: {error}");
                self.screen = self.home();
            }
        }
    }

    fn file(&mut self, action: FileAction) {
        self.notice = None;
        match files::run(&self.engine, action) {
            Outcome::Cancelled => {}
            Outcome::Done => {
                if recent::records(action) {
                    self.remember();
                }
                if action == FileAction::Open {
                    self.screen = Screen::Editor(Box::default());
                }
            }
            Outcome::Failed(message, detail) => {
                eprintln!("{action:?}: {detail}");
                self.notice = Some(self.texts.t(&message));
            }
        }
    }

    /// The export dialog, then the save dialog and the export (spec 030).
    fn export(&mut self, ctx: &egui::Context) {
        let Some(dialog) = self.export.as_mut() else {
            return;
        };
        match export::show(ctx, &self.theme, &self.texts, dialog) {
            export::Answer::Open => {}
            export::Answer::Cancel => self.export = None,
            export::Answer::Export => {
                let dialog = *dialog;
                self.export = None;
                self.notice = match export::run(&mut self.engine, dialog) {
                    Outcome::Cancelled => None,
                    Outcome::Done => Some(self.texts.t(&gafira_doc::msg!("export.done"))),
                    Outcome::Failed(message, detail) => {
                        eprintln!("export: {detail}");
                        Some(self.texts.t(&message))
                    }
                };
            }
        }
    }

    /// The dialogs over the screen; `true` when one is open.
    fn dialogs(&mut self, ctx: &egui::Context) -> bool {
        if self.export.is_some() {
            self.export(ctx);
            return true;
        }
        if let Some(to) = self.leaving.clone() {
            match dialogs::leave(ctx, &self.theme, &self.texts) {
                LeaveChoice::None => {}
                LeaveChoice::Cancel => self.leaving = None,
                LeaveChoice::Discard => {
                    self.leaving = None;
                    self.proceed(to);
                }
                LeaveChoice::Save => {
                    self.leaving = None;
                    self.file(FileAction::Save);
                    if !self.engine.projects().is_modified() {
                        self.proceed(to);
                    }
                }
            }
            return true;
        }
        if self.recovery.is_empty() {
            return false;
        }
        let failure = self.recovery_failure.clone();
        match dialogs::recovery(
            ctx,
            &self.theme,
            &self.texts,
            &self.recovery,
            self.confirm_discard,
            failure.as_deref(),
        ) {
            RecoveryChoice::None => {}
            RecoveryChoice::CancelDiscard => self.confirm_discard = None,
            RecoveryChoice::AskDiscard(index) => self.confirm_discard = Some(index),
            RecoveryChoice::Discard(index) => {
                let entry = self.recovery.remove(index);
                self.engine.projects().discard_recovery(entry.project_id);
                self.confirm_discard = None;
            }
            RecoveryChoice::Recover(index) => {
                let id = self.recovery[index].project_id;
                match self.engine.projects().recover(id) {
                    Ok(()) => {
                        self.remember();
                        self.recovery.clear();
                        self.screen = Screen::Editor(Box::default());
                    }
                    Err(error) => {
                        eprintln!("recover: {error}");
                        self.recovery_failure =
                            Some(self.texts.t(&gafira_doc::msg!("recovery.failed")));
                    }
                }
            }
            RecoveryChoice::OpenSaved(index) => {
                let entry = self.recovery[index].clone();
                if let Some(origin) = &entry.origin {
                    match self.engine.projects().open(origin) {
                        Ok(()) => {
                            self.remember();
                            self.engine.projects().discard_recovery(entry.project_id);
                            self.recovery.clear();
                            self.screen = Screen::Editor(Box::default());
                        }
                        Err(error) => {
                            eprintln!("open: {error}");
                            self.recovery_failure =
                                Some(self.texts.t(&gafira_doc::msg!("file.open_failed")));
                        }
                    }
                }
            }
        }
        true
    }
}

impl eframe::App for Shell {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // The window reads the document from snapshots each frame; the events
        // only wake it (02 §5.2), so the queue is drained and dropped.
        while self.engine.events().try_recv().is_ok() {}
        self.pump_autosave(ctx);
        let go = match &mut self.screen {
            Screen::Splash => {
                self.elapsed += ctx.input(|i| i.unstable_dt);
                let progress = splash::progress(self.elapsed);
                splash::show(ctx, &self.theme, &self.texts, self.version, progress);
                if progress >= 1.0 {
                    Go::Home
                } else {
                    ctx.request_repaint_after(Duration::from_millis(16));
                    Go::Stay
                }
            }
            Screen::Home(home) => home.show(ctx, &self.theme, &self.texts, unix_now()),
            Screen::Editor(editor) => editor.show(
                ctx,
                &mut self.engine,
                &self.theme,
                &self.texts,
                self.notice.as_deref(),
            ),
        };
        let blocked = self.dialogs(ctx);
        let go = if blocked { Go::Stay } else { go };
        if !matches!(go, Go::Stay) {
            // The new screen draws on the next frame; without this request
            // it would wait for the next input event.
            ctx.request_repaint();
        }
        let in_editor = matches!(self.screen, Screen::Editor(_));
        let modified = self.engine.projects().is_modified();
        match go {
            Go::Stay => {}
            Go::Home if in_editor && modified => self.leaving = Some(Leave::Home),
            Go::Home if in_editor => self.proceed(Leave::Home),
            Go::Home => {
                self.screen = self.home();
                // Once, as Inicio first appears (03 §6.1).
                if !self.scanned {
                    self.scanned = true;
                    match self.engine.projects().recoverable() {
                        Ok(entries) => self.recovery = entries,
                        Err(error) => eprintln!("recovery scan: {error}"),
                    }
                }
            }
            Go::File(FileAction::Open) if in_editor && modified => self.leaving = Some(Leave::Open),
            Go::File(action) => self.file(action),
            Go::OpenRecent(path) if in_editor && modified => {
                self.leaving = Some(Leave::Recent(path));
            }
            Go::OpenRecent(path) => self.open_recent(&path),
            Go::Dark(dark) => {
                self.theme.dark = dark;
                self.theme.apply(ctx);
                let appearance = if dark {
                    Appearance::Dark
                } else {
                    Appearance::Light
                };
                let settings = self.engine.settings().get().with_appearance(appearance);
                if let Err(error) = self.engine.settings().set(settings) {
                    eprintln!("settings: {error}");
                }
            }
            Go::Export(page) => {
                self.notice = None;
                self.export = Some(export::Dialog {
                    choice: export::Choice::default(),
                    page,
                });
            }
            Go::Editor(format) => match start(&mut self.engine, format) {
                Ok(()) => self.screen = Screen::Editor(Box::default()),
                Err(error) => eprintln!("new document: {error}"),
            },
        }
    }
}

/// A new document whose first page has the size of `format` (03 §6.2,
/// «Elegir»); the page is part of the document, so undo keeps it (spec 016).
pub(crate) fn start(engine: &mut Gafira, format: StartFormat) -> Result<(), EngineError> {
    engine.projects().new_document_with(format.page_size())
}

/// Seconds since the Unix epoch, for the relative times of Inicio.
fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok())
        .unwrap_or(0)
}
