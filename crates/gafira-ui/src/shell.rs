//! The window, the screens and the way between them (03 §6.1).

use std::time::Duration;

use std::sync::Arc;

use gafira_edit::InsertPage;
use gafira_engine::{EngineError, Gafira, Waker};
use gafira_ports::i18n::Locale;
use gafira_theme::ThemeTokens;

use crate::editor::Editor;
use crate::files::{self, FileAction, Outcome};
use crate::format::StartFormat;
use crate::home::Home;
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
    Editor(Editor),
}

/// What a screen asks the shell to do after drawing.
pub(crate) enum Go {
    Stay,
    Home,
    Editor(StartFormat),
    File(FileAction),
}

struct Shell {
    engine: Gafira,
    theme: Theme,
    texts: Texts,
    version: &'static str,
    screen: Screen,
    /// How long the Startup screen has been up, in seconds.
    elapsed: f32,
    /// The last failure, shown in the editor's status line until the next
    /// action.
    notice: Option<String>,
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
            let theme = Theme::new(tokens);
            theme.apply(&cc.egui_ctx);
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
            Ok(Box::new(Shell {
                engine,
                theme,
                texts: Texts::new(config.locale.unwrap_or(resolved.locale)),
                version: config.version,
                screen: Screen::Splash,
                elapsed: 0.0,
                notice: None,
            }))
        }),
    )
}

impl eframe::App for Shell {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // The window reads the document from snapshots each frame; the events
        // only wake it (02 §5.2), so the queue is drained and dropped.
        while self.engine.events().try_recv().is_ok() {}
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
            Screen::Home(home) => home.show(ctx, &self.theme, &self.texts),
            Screen::Editor(editor) => editor.show(
                ctx,
                &mut self.engine,
                &self.theme,
                &self.texts,
                self.notice.as_deref(),
            ),
        };
        if !matches!(go, Go::Stay) {
            // The new screen draws on the next frame; without this request
            // it would wait for the next input event.
            ctx.request_repaint();
        }
        match go {
            Go::Stay => {}
            Go::Home => self.screen = Screen::Home(Home::default()),
            Go::File(action) => {
                self.notice = None;
                match files::run(&self.engine, action) {
                    Outcome::Cancelled => {}
                    Outcome::Done => {
                        if action == FileAction::Open {
                            self.screen = Screen::Editor(Editor::default());
                        }
                    }
                    Outcome::Failed(message, detail) => {
                        eprintln!("{action:?}: {detail}");
                        self.notice = Some(self.texts.t(&message));
                    }
                }
            }
            Go::Editor(format) => match start(&mut self.engine, format) {
                Ok(()) => self.screen = Screen::Editor(Editor::default()),
                Err(error) => eprintln!("new document: {error}"),
            },
        }
    }
}

/// A new document with one page of `format` (03 §6.2, «Elegir»).
///
/// The page is the first edit of the document, so undo can take it away; a
/// `new_document` that takes the first page's size is a gap of the engine
/// (spec 018, notes).
pub(crate) fn start(engine: &mut Gafira, format: StartFormat) -> Result<(), EngineError> {
    engine.projects().new_document()?;
    engine.edit().execute(Box::new(InsertPage::new(
        0,
        String::new(),
        format.page_size(),
    )))?;
    Ok(())
}
