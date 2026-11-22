//! The window, the screens and the way between them (03 §6.1).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gafira_engine::{AutosaveOutcome, EngineError, Gafira, RecoveryEntry, SaveRequest, Waker};
use gafira_ports::i18n::LanguagePreference;
use gafira_ports::i18n::Locale;
use gafira_ports::settings::Appearance;

use crate::app_icon;
use crate::settings::{self, Change, SettingsView};
use gafira_theme::ThemeTokens;

use crate::dialogs::{self, LeaveChoice, RecoveryChoice};
use crate::editor::Editor;
use crate::export;
use crate::files::{self, FileAction, Outcome};
#[cfg(test)]
use crate::format::StartFormat;
use crate::home::{Home, Thumbs};
use crate::recent::{self, RecentStore};
use crate::saving::{self, Ended, Saving};
use crate::splash;
use crate::style::Theme;
use crate::text::Texts;

/// Builds the engine once the window exists, with the waker that repaints
/// it when an event arrives (02 §5.2).
pub type EngineFactory = Box<dyn FnOnce(Arc<dyn Waker>) -> Result<Gafira, EngineError>>;

/// «Mostrar en carpeta» (spec 039): opens the system file manager on the
/// folder of a file, without waiting for it. The composition root gives it
/// from `gafira-platform`.
pub type ShowInFolder = Box<dyn Fn(&Path) -> std::io::Result<()>>;

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
    /// The imported files of Biblioteca (spec 068).
    pub library: Box<dyn crate::LibraryStore>,
    /// «Mostrar en carpeta» after an export (spec 039).
    pub show_in_folder: ShowInFolder,
    /// The fonts installed on the system, filled in the background (spec
    /// 062).
    pub system_fonts: crate::SystemFonts,
    /// The export settings kept per format (spec 074).
    pub export_presets: Box<dyn crate::ExportPresets>,
    /// The remote photo catalogues of Elementos → Fotos (spec 071, D-111);
    /// `None` when the build has no network.
    pub photos: Option<std::sync::Arc<dyn gafira_ports::photos::PhotoCatalog>>,
    /// The vector illustrations of Elementos → Ilustraciones (spec 072).
    pub illustrations: Option<std::sync::Arc<dyn gafira_ports::photos::IllustrationCatalog>>,
    /// The models that work on images (spec 084); none without them.
    pub vision: Option<std::sync::Arc<dyn gafira_ports::vision::VisionModels>>,
    /// A project to open once the splash ends: the `.gaf` the system
    /// passed (a double click on a file, spec 078).
    pub open: Option<PathBuf>,
    /// The monitor the app opens on, `(x, y, width, height)` in desktop
    /// px: the one under the pointer (spec 078). The splash is centred on
    /// it; `None` leaves the centring to the windowing library.
    pub start_monitor: Option<(i32, i32, u32, u32)>,
}

/// Wakes egui when the engine publishes, so the window repaints.
struct RepaintWaker(egui::Context);

impl Waker for RepaintWaker {
    fn wake(&self) {
        self.0.request_repaint();
    }
}

/// The default and minimum main window, in logical px (03 §7.1).
const MAIN_WINDOW: egui::Vec2 = egui::Vec2::new(1440.0, 900.0);
const MAIN_WINDOW_MIN: egui::Vec2 = egui::Vec2::new(1280.0, 720.0);

/// Turn the borderless splash window into the main window, centred on where
/// the splash was.
fn grow_into_main_window(ctx: &egui::Context) {
    let (outer, monitor) = ctx.input(|i| (i.viewport().outer_rect, i.viewport().monitor_size));
    let size = splash::main_size(MAIN_WINDOW, MAIN_WINDOW_MIN, monitor);
    ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(MAIN_WINDOW_MIN));
    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
    if let Some(outer) = outer {
        let at = splash::grown_from(outer, size).max(egui::Pos2::ZERO);
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(at));
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
    /// A new document on this page: its size and guides (spec 076).
    Create(Box<gafira_doc::Page>),
    File(FileAction),
    /// Open a project of «Recientes».
    OpenRecent(PathBuf),
    /// Switch the interface to dark (`true`) or light, and remember it.
    Dark(bool),
    /// Open Ajustes over the current screen (03 §7.8, spec 034).
    Settings,
    /// Open «Exportar» for the page of the selection, if any (spec 030).
    Export(Option<gafira_doc::PageId>),
    /// Save a named version (spec 031), on its thread (spec 036).
    SaveVersion(String),
}

/// What to do once a save on its thread succeeded (spec 036).
struct AfterSave {
    /// Put the project in «Recientes» (not for a copy).
    remember: bool,
    /// Where «¿Guardar los cambios?» was going.
    then: Option<Leave>,
    /// The notice of a failure.
    failed: gafira_doc::msg::Message,
}

struct Shell {
    engine: Gafira,
    /// Ajustes, when open, drawn instead of the screen (spec 034).
    settings: Option<SettingsView>,
    recent: Box<dyn RecentStore>,
    /// The imported files of Biblioteca, shared with every editor (spec 068).
    library: std::rc::Rc<dyn crate::LibraryStore>,
    /// The photo catalogues, shared with every editor (spec 071).
    vision: Option<std::sync::Arc<dyn gafira_ports::vision::VisionModels>>,
    photos: Option<std::sync::Arc<dyn gafira_ports::photos::PhotoCatalog>>,
    illustrations: Option<std::sync::Arc<dyn gafira_ports::photos::IllustrationCatalog>>,
    export_presets: Box<dyn crate::ExportPresets>,
    theme: Theme,
    texts: Texts,
    version: &'static str,
    screen: Screen,
    /// How long the Startup screen has been up, in seconds.
    elapsed: f32,
    /// The project to open when the splash ends (spec 078).
    open_at_start: Option<PathBuf>,
    /// The last failure, shown in the editor's status line until the next
    /// action.
    notice: Option<String>,
    /// The autosave job running on its thread, if any.
    autosave: Option<mpsc::Receiver<AutosaveOutcome>>,
    /// Whether the recovery scan already ran.
    scanned: bool,
    /// What Copiar or Cortar took last, for the whole session: a paste
    /// reaches the next project too (spec 043).
    clipboard: Option<std::sync::Arc<crate::clipboard::Clip>>,
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
    /// The export running on its thread (spec 035).
    exporting: Option<export::Running>,
    /// The first file of the last export that ended well, while its card
    /// offers «Mostrar en carpeta» (spec 039).
    exported: Option<PathBuf>,
    show_in_folder: ShowInFolder,
    /// The save running on its thread and what follows it (spec 036).
    saving: Option<(Saving, AfterSave)>,
    /// Repaints the window from the threads of a save.
    waker: egui::Context,
    /// The textures of the thumbnails of «Recientes» (spec 033).
    thumbs: Thumbs,
}

/// Open the main window and run until it closes (03 §7.1: minimum
/// 1280 × 720 logical px).
///
/// # Errors
/// When the window or its graphics context cannot be created.
pub fn run(config: UiConfig) -> eframe::Result {
    crate::system_fonts::install(config.system_fonts.clone());
    let tokens = ThemeTokens::embedded().map_err(|e| eframe::Error::AppCreation(Box::new(e)))?;
    // The window opens as the Startup screen (03 §7.10.2) and grows into
    // the main window when it ends (`Shell::grow_into_main_window`).
    let (w, h) = tokens.splash.window;
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Gafira")
        .with_app_id(app_icon::APP_ID)
        .with_inner_size([w as f32, h as f32])
        // Not `with_resizable(false)`: a borderless window has no handles to
        // resize it, and on X11 a window born fixed keeps the maximize
        // button disabled after it grows.
        .with_decorations(false);
    // Spec 046: an icon that cannot be decoded leaves the system's default.
    if let Some(icon) = app_icon::window_icon() {
        viewport = viewport.with_icon(icon);
    }
    // In the middle of the monitor the app opens on, both ways; the main
    // window then grows from the splash's centre.
    let at = config
        .start_monitor
        .map(|(x, y, mw, mh)| splash::centred_on((x, y, mw, mh), (w, h)));
    if let Some(at) = at {
        viewport = viewport.with_position(at);
    }
    let options = eframe::NativeOptions {
        viewport,
        centered: at.is_none(),
        ..Default::default()
    };
    eframe::run_native(
        "Gafira",
        options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            // The pointing hand over every button, menu and tab (03 §8.1).
            cc.egui_ctx.on_end_pass(
                "pointing-hand",
                Arc::new(|ctx: &egui::Context| crate::widgets::pointing_hand(ctx)),
            );
            // Ctrl+= / Ctrl+- / Ctrl+0 zoom the canvas (03 §9.2, spec 037),
            // not the whole interface.
            cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
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
            let system_dark = cc.egui_ctx.system_theme().map(|t| t == egui::Theme::Dark);
            theme.dark = settings::is_dark(engine.settings().get().appearance, system_dark);
            theme.apply(&cc.egui_ctx);
            Ok(Box::new(Shell {
                settings: None,
                engine,
                recent: config.recent,
                library: std::rc::Rc::from(config.library),
                photos: config.photos,
                export_presets: config.export_presets,
                illustrations: config.illustrations,
                vision: config.vision,
                theme,
                texts: Texts::new(config.locale.unwrap_or(resolved.locale)),
                version: config.version,
                screen: Screen::Splash,
                elapsed: 0.0,
                open_at_start: config.open,
                notice: None,
                autosave: None,
                scanned: false,
                recovery: Vec::new(),
                clipboard: None,
                confirm_discard: None,
                recovery_failure: None,
                leaving: None,
                export: None,
                exporting: None,
                exported: None,
                show_in_folder: config.show_in_folder,
                saving: None,
                waker: cc.egui_ctx.clone(),
                thumbs: Thumbs::default(),
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
    /// A new editor with the shared Biblioteca (spec 068).
    fn new_editor(&self) -> Box<Editor> {
        let mut editor = Box::<Editor>::default();
        editor.library = Some(std::rc::Rc::clone(&self.library));
        editor
            .photo_panel
            .get_mut()
            .set_catalog(self.photos.clone());
        editor
            .illustration_panel
            .get_mut()
            .set_catalog(self.illustrations.clone());
        editor.studio.set_models(self.vision.clone());
        editor
            .illustration_panel
            .get_mut()
            .set_photos(self.photos.clone());
        editor
    }

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
            Leave::Open => self.file(FileAction::Open, None),
            Leave::Recent(path) => self.open_recent(&path),
        }
    }

    /// Inicio with the recent projects as they are now, and the thumbnails
    /// of the ones it shows (spec 033).
    fn home(&self) -> Screen {
        let list = self.recent.list();
        let thumbs = recent::shown(&list)
            .iter()
            .filter_map(|project| {
                let thumb = self.recent.thumbnail(&project.path)?;
                Some((project.path.clone(), thumb))
            })
            .collect();
        Screen::Home(Home::new(list, thumbs))
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
                self.screen = Screen::Editor(self.new_editor());
            }
            Err(error) => {
                eprintln!("open recent: {error}");
                self.screen = self.home();
            }
        }
    }

    /// Run `action` of the Archivo menu; a save goes to its thread and
    /// `then` follows it once it succeeded.
    fn file(&mut self, action: FileAction, then: Option<Leave>) {
        self.notice = None;
        if action != FileAction::Open && self.saving.is_some() {
            self.notice = Some(self.texts.t(&gafira_doc::msg!("file.busy")));
            return;
        }
        match files::plan(&self.engine, action) {
            files::Plan::Cancelled | files::Plan::Opened(Outcome::Cancelled) => {}
            files::Plan::Opened(Outcome::Done) => {
                self.remember();
                self.screen = Screen::Editor(self.new_editor());
            }
            files::Plan::Opened(Outcome::Failed(message, detail)) => {
                eprintln!("{action:?}: {detail}");
                self.notice = Some(self.texts.t(&message));
            }
            files::Plan::Save(request) => self.start_save(
                request,
                AfterSave {
                    remember: recent::records(action),
                    then,
                    failed: gafira_doc::msg!("file.save_failed"),
                },
            ),
        }
    }

    /// «Guardar versión» with `name`; a project never saved asks for its
    /// file first, because a version lives inside the file (spec 031).
    fn save_version(&mut self, name: String) {
        self.notice = None;
        if self.saving.is_some() {
            self.notice = Some(self.texts.t(&gafira_doc::msg!("file.busy")));
            return;
        }
        let at = if self.engine.projects().path().is_some() {
            None
        } else {
            let Some(path) = files::pick_new(&self.engine) else {
                return;
            };
            Some(path)
        };
        self.start_save(
            SaveRequest::Version { name, at },
            AfterSave {
                remember: true,
                then: None,
                failed: gafira_doc::msg!("versions.save_failed"),
            },
        );
    }

    /// Check `request` with the engine and run it on its thread.
    fn start_save(&mut self, request: SaveRequest, after: AfterSave) {
        match self.engine.projects().prepare_save(request) {
            Ok(job) => {
                let waker = self.waker.clone();
                let saving = saving::spawn(job, Arc::new(move || waker.request_repaint()));
                self.saving = Some((saving, after));
            }
            Err(error) => {
                eprintln!("save: {error}");
                self.notice = Some(self.texts.t(&after.failed));
            }
        }
    }

    /// The save on its thread (spec 036): its card while it runs; when it
    /// ends the engine accounts for it, and what waited for it follows.
    fn pump_save(&mut self, ctx: &egui::Context) {
        let Some((running, _)) = self.saving.as_mut() else {
            return;
        };
        let Some(ended) = running.poll() else {
            saving::card(ctx, &self.theme, &self.texts);
            return;
        };
        if let Some((_, after)) = self.saving.take() {
            self.save_ended(ended, after);
        }
        ctx.request_repaint();
    }

    /// Account for a save that ended, then do what waited for it.
    fn save_ended(&mut self, ended: Ended, after: AfterSave) {
        let result = match ended {
            Ended::Ran(outcome) => self
                .engine
                .projects()
                .save_finished(outcome)
                .map_err(|error| error.to_string()),
            Ended::Lost => Err("the save thread stopped without an answer".to_owned()),
        };
        match result {
            Ok(()) => {
                if after.remember {
                    self.remember();
                }
                if let Some(to) = after.then {
                    self.proceed(to);
                }
            }
            Err(detail) => {
                eprintln!("save: {detail}");
                self.notice = Some(self.texts.t(&after.failed));
            }
        }
    }

    /// The export dialog, then the save dialog and the export (spec 030).
    fn export(&mut self, ctx: &egui::Context, window: &eframe::Frame) {
        let Some(dialog) = self.export.as_mut() else {
            return;
        };
        match export::show(ctx, &self.theme, &self.texts, &mut self.engine, dialog) {
            export::Answer::Open => {}
            export::Answer::Cancel => self.export = None,
            export::Answer::Export => {
                let Some(dialog) = self.export.take() else {
                    return;
                };
                // Keep or forget this format's settings (spec 074).
                if let Some((format, kept)) = dialog.preset() {
                    self.export_presets.save(&format, kept);
                }
                if self.exporting.is_some() {
                    self.notice = Some(self.texts.t(&gafira_doc::msg!("export.busy")));
                    return;
                }
                match export::start(&mut self.engine, &dialog, ctx, window) {
                    export::Start::Cancelled => {}
                    export::Start::Running(running) => {
                        self.notice = None;
                        self.exported = None;
                        self.exporting = Some(running);
                    }
                    export::Start::Failed(message, detail) => {
                        eprintln!("export: {detail}");
                        self.notice = Some(self.texts.t(&message));
                    }
                }
            }
        }
    }

    /// The export on its thread (spec 035): its card while it runs, and the
    /// notice when it ends; one that ended well leaves its card with
    /// «Mostrar en carpeta» (spec 039).
    fn pump_export(&mut self, ctx: &egui::Context) {
        let Some(running) = self.exporting.as_mut() else {
            self.pump_exported(ctx);
            return;
        };
        if let Some(outcome) = running.poll() {
            let path = running.path().to_path_buf();
            self.exporting = None;
            self.notice = match outcome {
                Outcome::Cancelled => Some(self.texts.t(&gafira_doc::msg!("export.cancelled"))),
                Outcome::Done => {
                    self.exported = Some(path);
                    None
                }
                Outcome::Failed(message, detail) => {
                    eprintln!("export: {detail}");
                    Some(self.texts.t(&message))
                }
            };
            ctx.request_repaint();
            return;
        }
        if export::progress(ctx, &self.theme, &self.texts, running.fraction()) {
            running.cancel();
        }
    }

    /// The card of the export that ended well, until it is closed.
    fn pump_exported(&mut self, ctx: &egui::Context) {
        let Some(path) = self.exported.as_ref() else {
            return;
        };
        match export::finished(ctx, &self.theme, &self.texts) {
            export::Finished::Open => {}
            export::Finished::Close => self.exported = None,
            export::Finished::ShowInFolder => {
                if let Err(error) = (self.show_in_folder)(path) {
                    eprintln!("show in folder {}: {error}", path.display());
                    self.notice = Some(self.texts.t(&gafira_doc::msg!("export.folder_failed")));
                }
                self.exported = None;
            }
        }
    }

    /// The dialogs over the screen; `true` when one is open.
    fn dialogs(&mut self, ctx: &egui::Context, window: &eframe::Frame) -> bool {
        if self.export.is_some() {
            self.export(ctx, window);
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
                    // The way on follows the save, once it succeeded.
                    self.file(FileAction::Save, Some(to));
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
                        self.screen = Screen::Editor(self.new_editor());
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
                            self.screen = Screen::Editor(self.new_editor());
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

impl Shell {
    /// Ajustes over the whole window; a change applies at once and persists
    /// through `SettingsService` (03 §7.8, §16.1).
    fn show_settings(&mut self, ctx: &egui::Context) {
        let Some(view) = self.settings.as_mut() else {
            return;
        };
        let current = self.engine.settings().get();
        let system = self.engine.settings().preview(LanguagePreference::System);
        let change = settings::show(
            ctx,
            &self.theme,
            &self.texts,
            view,
            current,
            &system,
            self.version,
        );
        match change {
            Change::None => {}
            Change::Back => {
                self.settings = None;
                ctx.request_repaint();
            }
            Change::Language(language) => match self.engine.settings().set_language(language) {
                Ok(resolved) => self.texts.locale = resolved.locale,
                Err(error) => eprintln!("settings: {error}"),
            },
            Change::Appearance(appearance) => {
                let system_dark = ctx.system_theme().map(|t| t == egui::Theme::Dark);
                self.theme.dark = settings::is_dark(appearance, system_dark);
                self.theme.apply(ctx);
                if let Err(error) = self
                    .engine
                    .settings()
                    .set(current.with_appearance(appearance))
                {
                    eprintln!("settings: {error}");
                }
            }
        }
    }
}

impl eframe::App for Shell {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // A closing window stops its export: no temporary is left behind.
        if let Some(running) = self.exporting.take() {
            running.stop();
        }
        // A save is never cut short: wait for it, and let the engine account
        // for it (its lock and its recovery slots).
        if let Some((running, after)) = self.saving.take() {
            let ended = running.wait();
            self.save_ended(
                ended,
                AfterSave {
                    then: None,
                    ..after
                },
            );
        }
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // The window reads the document from snapshots each frame; the events
        // only wake it (02 §5.2), so the queue is drained and dropped.
        while self.engine.events().try_recv().is_ok() {}
        self.pump_autosave(ctx);
        if self.settings.is_some() {
            self.show_settings(ctx);
            return;
        }
        let go = match &mut self.screen {
            Screen::Splash => {
                self.elapsed += ctx.input(|i| i.unstable_dt);
                let progress = splash::progress(self.elapsed);
                splash::show(ctx, &self.theme, &self.texts, self.version, progress);
                if progress >= 1.0 {
                    grow_into_main_window(ctx);
                    match self.open_at_start.take() {
                        Some(path) => Go::OpenRecent(path),
                        None => Go::Home,
                    }
                } else {
                    ctx.request_repaint_after(Duration::from_millis(16));
                    Go::Stay
                }
            }
            Screen::Home(home) => {
                home.show(ctx, &self.theme, &self.texts, unix_now(), &mut self.thumbs)
            }
            Screen::Editor(editor) => editor.show(
                ctx,
                &mut self.engine,
                &self.theme,
                &self.texts,
                self.notice.as_deref(),
                &mut self.clipboard,
            ),
        };
        let blocked = self.dialogs(ctx, frame);
        self.pump_export(ctx);
        self.pump_save(ctx);
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
            Go::File(action) => self.file(action, None),
            Go::SaveVersion(name) => self.save_version(name),
            Go::OpenRecent(path) if in_editor && modified => {
                self.leaving = Some(Leave::Recent(path));
            }
            Go::OpenRecent(path) => self.open_recent(&path),
            Go::Settings => self.settings = Some(SettingsView::default()),
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
                self.export = Some(
                    export::Dialog::new(export::formats(&self.engine), page)
                        .with_presets(self.export_presets.as_ref()),
                );
            }
            Go::Create(page) => match self.engine.projects().new_document_from(*page) {
                Ok(()) => self.screen = Screen::Editor(self.new_editor()),
                Err(error) => eprintln!("new document: {error}"),
            },
        }
    }
}

/// A new document whose first page has the size of `format` (03 §6.2,
/// «Elegir»); the page is part of the document, so undo keeps it (spec 016).
#[cfg(test)]
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
