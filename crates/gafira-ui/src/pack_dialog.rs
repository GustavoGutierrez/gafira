//! The window that shows a `.gafpack` before it is installed (spec 097,
//! §8.2, §13.2): its thumbnail, name and description in the language of
//! the interface, its type, declared author, version, licence and what it
//! holds by section. «Instalar» appears only for a package that passed
//! every check; an invalid one lists its problems and installs nothing.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};

use egui::{Color32, RichText, Sense, Stroke, Vec2};
use gafira_doc::msg::{Message, MessageKey};
use gafira_ports::i18n::Locale;

use crate::packs::{
    InstalledPack, PackInspection, PackProblem, PackProgress, PackStore, PackType, counts,
};
use crate::style::Theme;
use crate::text::Texts;
use crate::widgets;

fn t(texts: &Texts, key: &'static str) -> String {
    texts.t(&Message {
        key: MessageKey(key),
        args: Vec::new(),
    })
}

/// One problem as a line: what is wrong, then what it is about.
pub(crate) fn problem_line(texts: &Texts, problem: &PackProblem) -> String {
    let what = t(texts, problem.key);
    if problem.about.is_empty() {
        what
    } else {
        format!("{what}: {}", problem.about)
    }
}

/// A size in bytes as KB or MB.
pub(crate) fn size_text(bytes: u64) -> String {
    let kb = bytes as f64 / 1024.0;
    if kb < 1024.0 {
        format!("{kb:.0} KB")
    } else {
        format!("{:.1} MB", kb / 1024.0)
    }
}

/// A check or an install running on its own thread, and how far it is.
struct Task<T> {
    done: Receiver<T>,
    progress: Arc<Mutex<PackProgress>>,
}

impl<T: Send + 'static> Task<T> {
    /// Runs `work` on a thread; its progress repaints `ctx`.
    fn spawn(
        ctx: &egui::Context,
        work: impl FnOnce(&dyn Fn(PackProgress)) -> T + Send + 'static,
    ) -> Self {
        let (send, done) = mpsc::channel();
        let progress = Arc::new(Mutex::new(PackProgress::default()));
        let shared = Arc::clone(&progress);
        let ctx = ctx.clone();
        let started = std::thread::Builder::new()
            .name("gafpack".to_owned())
            .spawn(move || {
                let report = |p: PackProgress| {
                    if let Ok(mut now) = shared.lock() {
                        *now = p;
                    }
                    ctx.request_repaint();
                };
                let _ = send.send(work(&report));
                ctx.request_repaint();
            });
        if let Err(error) = started {
            eprintln!("gafpack thread: {error}");
        }
        Self { done, progress }
    }

    fn progress(&self) -> PackProgress {
        self.progress.lock().map(|p| *p).unwrap_or_default()
    }
}

/// The package window.
pub(crate) struct PackDialog {
    path: PathBuf,
    /// What the check found, once it ended.
    seen: Option<Result<PackInspection, Vec<PackProblem>>>,
    checking: Option<Task<Result<PackInspection, Vec<PackProblem>>>>,
    installing: Option<Task<Result<InstalledPack, Vec<PackProblem>>>>,
    thumb: Option<egui::TextureHandle>,
    /// What installing it gave, once it ended.
    outcome: Option<Result<InstalledPack, Vec<PackProblem>>>,
}

/// What the window asks of the screen under it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PackAsk {
    Stay,
    Close,
    /// Installed: the screen may refresh what lists packages.
    Installed,
}

impl PackDialog {
    /// The window over the package at `path`, checked on a thread of its
    /// own while the window shows how far it is.
    pub(crate) fn open(store: &Arc<dyn PackStore>, path: PathBuf, ctx: &egui::Context) -> Self {
        let store = Arc::clone(store);
        let file = path.clone();
        let checking = Task::spawn(ctx, move |progress| store.inspect(&file, progress));
        Self {
            path,
            seen: None,
            checking: Some(checking),
            installing: None,
            thumb: None,
            outcome: None,
        }
    }

    /// Whether a check or an install is running.
    fn busy(&self) -> bool {
        self.checking.is_some() || self.installing.is_some()
    }

    /// Takes what the running task gave, if it ended.
    fn pump(&mut self) {
        if let Some(task) = &self.checking
            && let Ok(found) = task.done.try_recv()
        {
            self.seen = Some(found);
            self.checking = None;
        }
        if let Some(task) = &self.installing
            && let Ok(found) = task.done.try_recv()
        {
            self.outcome = Some(found);
            self.installing = None;
        }
    }

    /// The bar of the running task: what it does and how far it is.
    fn progress_bar(&self, ui: &mut egui::Ui, theme: &Theme, texts: &Texts) {
        let progress = self
            .installing
            .as_ref()
            .or(None)
            .map(Task::progress)
            .or_else(|| self.checking.as_ref().map(Task::progress))
            .unwrap_or_default();
        let file = self
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let doing = if self.installing.is_some() {
            t(texts, "packs.installing")
        } else {
            t(texts, "packs.checking")
        };
        ui.label(
            RichText::new(doing.replace("{file}", &file))
                .font(theme.font("body"))
                .color(theme.c("text")),
        );
        ui.add_space(12.0);
        let share = progress.share();
        ui.add(
            egui::ProgressBar::new(share)
                .desired_height(10.0)
                .fill(theme.c("accent"))
                .corner_radius(5.0),
        );
        ui.add_space(6.0);
        let detail = if progress.total > 0 {
            format!(
                "{:.0} % · {} / {}",
                share * 100.0,
                size_text(progress.done),
                size_text(progress.total)
            )
        } else {
            t(texts, "packs.reading")
        };
        ui.label(
            RichText::new(detail)
                .font(theme.font("caption"))
                .color(theme.c("text.secondary")),
        );
    }

    /// Draws the window; what the screen should do after it.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        store: &Arc<dyn PackStore>,
    ) -> PackAsk {
        self.pump();
        let mut ask = PackAsk::Stay;
        let mut install = false;
        if self.thumb.is_none()
            && let Some(Ok(seen)) = &self.seen
            && let Ok(image) =
                image::load_from_memory_with_format(&seen.thumbnail, image::ImageFormat::WebP)
        {
            let image = image.thumbnail(512, 512).to_rgba8();
            let size = [image.width() as usize, image.height() as usize];
            let pixels = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
            self.thumb = Some(ctx.load_texture("pack.thumb", pixels, egui::TextureOptions::LINEAR));
        }
        let spanish = texts.locale == Locale::Es;
        let title = match &self.outcome {
            Some(Ok(_)) => t(texts, "packs.installed_title"),
            _ => t(texts, "packs.install_title"),
        };
        let busy = self.busy();
        crate::brand_screen::modal_of(ctx, theme, &title, 560.0, |ui| {
            if busy {
                self.progress_bar(ui, theme, texts);
                // A check can be left; an install goes on to its end.
                if self.checking.is_some() {
                    ui.add_space(14.0);
                    if widgets::secondary(ui, theme, t(texts, "packs.cancel")).clicked() {
                        ask = PackAsk::Close;
                    }
                }
                return;
            }
            if let Some(Ok(pack)) = &self.outcome {
                let name = pack.manifest.name.get(spanish).to_owned();
                ui.label(
                    RichText::new(t(texts, "packs.installed_body").replace("{name}", &name))
                        .font(theme.font("body"))
                        .color(theme.c("text")),
                );
                ui.add_space(8.0);
                section_counts(ui, theme, texts, &counts(&pack.manifest));
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    if widgets::primary(ui, theme, t(texts, "packs.done")).clicked() {
                        ask = PackAsk::Installed;
                    }
                });
                return;
            }
            let Some(seen) = &self.seen else {
                return;
            };
            match seen {
                Err(problems) => {
                    let file = self
                        .path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    ui.label(
                        RichText::new(t(texts, "packs.invalid").replace("{file}", &file))
                            .font(theme.font("body"))
                            .color(theme.c("text")),
                    );
                    ui.add_space(8.0);
                    problem_list(ui, theme, texts, problems);
                    ui.add_space(18.0);
                    if widgets::secondary(ui, theme, t(texts, "packs.close")).clicked() {
                        ask = PackAsk::Close;
                    }
                }
                Ok(seen) => {
                    let m = &seen.manifest;
                    ui.horizontal_top(|ui| {
                        let side = 150.0;
                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
                        checkerboard(ui, theme, rect);
                        if let Some(thumb) = &self.thumb {
                            ui.painter().image(
                                thumb.id(),
                                rect,
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                Color32::WHITE,
                            );
                        }
                        ui.add_space(14.0);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(m.name.get(spanish))
                                    .font(theme.font("h3"))
                                    .color(theme.c("text")),
                            );
                            let kind = PackType::parse(&m.package_type)
                                .map(|k| t(texts, k.key()))
                                .unwrap_or_default();
                            muted(
                                ui,
                                theme,
                                &format!("{} {} · {}", t(texts, "packs.version"), m.version, kind),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new(m.description.get(spanish))
                                    .font(theme.font("body-sm"))
                                    .color(theme.c("text")),
                            );
                            ui.add_space(6.0);
                            muted(ui, theme, &author_line(texts, &m.author.name, &seen.signer));
                        });
                    });
                    ui.add_space(14.0);
                    ui.label(
                        RichText::new(t(texts, "packs.contents"))
                            .font(theme.font("ui-label"))
                            .color(theme.c("text")),
                    );
                    ui.add_space(4.0);
                    section_counts(ui, theme, texts, &counts(m));
                    ui.add_space(10.0);
                    muted(
                        ui,
                        theme,
                        &format!("{}: {}", t(texts, "packs.license"), m.license),
                    );
                    muted(
                        ui,
                        theme,
                        &t(texts, "packs.compatible").replace("{version}", &m.min_gafira_version),
                    );
                    muted(
                        ui,
                        theme,
                        &format!("{}: {}", t(texts, "packs.size"), size_text(seen.size)),
                    );
                    ui.add_space(8.0);
                    // The state: valid, and whether it replaces an installed
                    // version.
                    let state = match seen.installed.as_deref() {
                        Some(v) if v == m.version => t(texts, "packs.state_same"),
                        Some(v) => t(texts, "packs.state_update").replace("{version}", v),
                        None => t(texts, "packs.state_valid"),
                    };
                    ui.label(
                        RichText::new(state)
                            .font(theme.font("body-sm"))
                            .color(theme.c("success.fg")),
                    );
                    if let Some(publisher) = seen.signer.verified() {
                        ui.label(
                            RichText::new(t(texts, "packs.verified").replace("{name}", publisher))
                                .font(theme.font("body-sm"))
                                .color(theme.c("success.fg")),
                        );
                    } else {
                        muted(ui, theme, &t(texts, "packs.unverified"));
                    }
                    if let Some(Err(problems)) = &self.outcome {
                        ui.add_space(8.0);
                        problem_list(ui, theme, texts, problems);
                    }
                    ui.add_space(18.0);
                    ui.horizontal(|ui| {
                        let label = match seen.installed.as_deref() {
                            Some(v) if v == m.version => t(texts, "packs.reinstall"),
                            Some(_) => t(texts, "packs.update"),
                            None => t(texts, "packs.install"),
                        };
                        if widgets::primary(ui, theme, label).clicked() {
                            install = true;
                        }
                        if widgets::secondary(ui, theme, t(texts, "packs.cancel")).clicked() {
                            ask = PackAsk::Close;
                        }
                    });
                }
            }
        });
        if install && let Some(Ok(seen)) = &self.seen {
            let store = Arc::clone(store);
            let file = self.path.clone();
            let checked = seen.clone();
            self.outcome = None;
            self.installing = Some(Task::spawn(ctx, move |progress| {
                store.install(&file, Some(&checked), progress)
            }));
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape))
            && ask == PackAsk::Stay
            && self.installing.is_none()
        {
            ask = if matches!(self.outcome, Some(Ok(_))) {
                PackAsk::Installed
            } else {
                PackAsk::Close
            };
        }
        ask
    }
}

fn muted(ui: &mut egui::Ui, theme: &Theme, text: &str) {
    ui.label(
        RichText::new(text)
            .font(theme.font("caption"))
            .color(theme.c("text.secondary")),
    );
}

/// «N Ilustraciones», one line per section.
fn section_counts(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    counts: &std::collections::BTreeMap<crate::packs::PackSection, usize>,
) {
    for (section, n) in counts {
        ui.label(
            RichText::new(format!(
                "{n} {}",
                t(
                    texts,
                    if *n == 1 {
                        section.key_one()
                    } else {
                        section.key()
                    }
                )
            ))
            .font(theme.font("body-sm"))
            .color(theme.c("text")),
        );
    }
}

/// The problems, each on its line, the first ones only.
/// Who made the package: the verified publisher when its signature holds
/// (spec 112), else the declared author, said to be unverified.
pub(crate) fn author_line(texts: &Texts, declared: &str, signer: &crate::PackSigner) -> String {
    match signer.verified() {
        Some(publisher) => t(texts, "packs.author_verified").replace("{name}", publisher),
        None => t(texts, "packs.author").replace("{name}", declared),
    }
}

fn problem_list(ui: &mut egui::Ui, theme: &Theme, texts: &Texts, problems: &[PackProblem]) {
    egui::Frame::NONE
        .fill(theme.c("error.bg"))
        .corner_radius(8.0)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            for problem in problems.iter().take(8) {
                ui.label(
                    RichText::new(format!("• {}", problem_line(texts, problem)))
                        .font(theme.font("body-sm"))
                        .color(theme.c("error.fg")),
                );
            }
            if problems.len() > 8 {
                ui.label(
                    RichText::new(
                        t(texts, "packs.more_problems")
                            .replace("{n}", &(problems.len() - 8).to_string()),
                    )
                    .font(theme.font("caption"))
                    .color(theme.c("error.fg")),
                );
            }
        });
}

/// A light checkerboard under a picture that may be see-through (§4.1.3).
pub(crate) fn checkerboard(ui: &egui::Ui, theme: &Theme, rect: egui::Rect) {
    let painter = ui.painter();
    painter.rect_filled(rect, 8.0, theme.c("surface"));
    let cell = 12.0;
    let dark = theme.c("surface.sunken");
    let mut y = rect.top();
    let mut row = 0;
    while y < rect.bottom() {
        let mut x = rect.left() + if row % 2 == 0 { 0.0 } else { cell };
        while x < rect.right() {
            let square =
                egui::Rect::from_min_size(egui::pos2(x, y), Vec2::splat(cell)).intersect(rect);
            painter.rect_filled(square, 0.0, dark);
            x += 2.0 * cell;
        }
        y += cell;
        row += 1;
    }
    painter.rect_stroke(
        rect,
        8.0,
        Stroke::new(1.0, theme.c("border")),
        egui::StrokeKind::Inside,
    );
}
