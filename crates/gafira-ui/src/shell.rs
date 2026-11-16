//! The window, the screens and the way between them (03 §6.1).

use std::time::Duration;

use gafira_ports::i18n::Locale;
use gafira_theme::ThemeTokens;

use crate::editor::Editor;
use crate::format::StartFormat;
use crate::home::Home;
use crate::splash;
use crate::style::Theme;
use crate::text::Texts;

/// What the composition root hands the shell.
pub struct UiConfig {
    /// The interface language, resolved by the D-08 rule.
    pub locale: Locale,
    /// The app version the Startup screen shows.
    pub version: &'static str,
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
}

struct Shell {
    theme: Theme,
    texts: Texts,
    version: &'static str,
    screen: Screen,
    /// How long the Startup screen has been up, in seconds.
    elapsed: f32,
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
            Ok(Box::new(Shell {
                theme,
                texts: Texts::new(config.locale),
                version: config.version,
                screen: Screen::Splash,
                elapsed: 0.0,
            }))
        }),
    )
}

impl eframe::App for Shell {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
            Screen::Editor(editor) => editor.show(ctx, &self.theme, &self.texts),
        };
        match go {
            Go::Stay => {}
            Go::Home => self.screen = Screen::Home(Home::default()),
            Go::Editor(format) => self.screen = Screen::Editor(Editor::new(format)),
        }
    }
}
