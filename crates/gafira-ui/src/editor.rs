//! The regions of the Design editor (03 §7.1, §7.3): top bar, tab rail, side
//! panel, canvas with the page, inspector and bottom bar.

use std::sync::Arc;

use egui::{Align, Align2, Color32, Layout, Pos2, Rect, RichText, Stroke, Vec2};
use gafira_doc::Document;
use gafira_edit::InsertPage;
use gafira_engine::Gafira;
use gafira_theme::Shadow;

use crate::format::fit_zoom;
use crate::shell::Go;
use crate::style::{Theme, color32, px};
use crate::text::Texts;
use crate::widgets;

/// The tabs of the rail, in the order of 03 §2.3.
const TABS: [&str; 6] = [
    "tab.templates",
    "tab.pieces",
    "tab.text",
    "tab.identity",
    "tab.library",
    "tab.drawing",
];

/// The margin around the page on the canvas, in logical px.
const CANVAS_MARGIN: f32 = 48.0;

/// The space between two pages on the canvas, in logical px (03 §7.3: the
/// pages stack vertically).
const PAGE_GAP: f32 = 56.0;

/// The editor's view state; the document lives in the engine and is read
/// from a snapshot every frame (02 §5.5).
pub(crate) struct Editor {
    tab: usize,
    /// A factor over the fitting zoom, changed with Ctrl + wheel.
    zoom_factor: f32,
    /// The zoom of the last frame, for the bottom bar.
    zoom: f32,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            tab: 0,
            zoom_factor: 1.0,
            zoom: 1.0,
        }
    }
}

/// What the person asked for this frame; applied after drawing, so no
/// panel holds the engine while another reads it.
#[derive(Default)]
struct Asked {
    home: bool,
    undo: bool,
    redo: bool,
    add_page: bool,
}

/// The pages of `document` in logical px at its dpi.
fn page_sizes(document: &Document) -> Vec<Vec2> {
    document
        .pages()
        .iter()
        .filter_map(|page| page.size.to_px(document.dpi()).ok())
        .map(|size| Vec2::new(size.width() as f32, size.height() as f32))
        .collect()
}

impl Editor {
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        engine: &mut Gafira,
        theme: &Theme,
        texts: &Texts,
    ) -> Go {
        let document = engine.edit().document();
        let (can_undo, can_redo) = engine.edit().can_undo_redo();
        let modified = engine.projects().is_modified();
        let mut asked = Asked::default();
        ctx.input(|i| {
            if i.modifiers.command && i.key_pressed(egui::Key::Z) {
                if i.modifiers.shift {
                    asked.redo = true;
                } else {
                    asked.undo = true;
                }
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Y) {
                asked.redo = true;
            }
        });
        self.bars(
            ctx,
            theme,
            texts,
            &document,
            (can_undo, can_redo, modified),
            &mut asked,
        );
        self.rail(ctx, theme, texts);
        self.side_panel(ctx, theme, texts);
        self.inspector(ctx, theme, texts);
        self.canvas(ctx, theme, texts, document.as_deref());
        apply(engine, &document, &asked)
    }

    fn bars(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        document: &Option<Arc<Document>>,
        (can_undo, can_redo, modified): (bool, bool, bool),
        asked: &mut Asked,
    ) {
        widgets::topbar(ctx, theme, "editor.topbar", |ui| {
            if widgets::on_topbar(ui, theme, texts.t(&gafira_doc::msg!("nav.home")), true).clicked()
            {
                asked.home = true;
            }
            ui.add_space(12.0);
            for key in [
                gafira_doc::msg!("menu.file"),
                gafira_doc::msg!("menu.edit"),
                gafira_doc::msg!("menu.view"),
                gafira_doc::msg!("menu.size"),
            ] {
                widgets::on_topbar(ui, theme, texts.t(&key), false);
            }
            ui.add_space(24.0);
            ui.label(
                RichText::new(texts.t(&gafira_doc::msg!("editor.untitled")))
                    .font(theme.font("ui-label"))
                    .color(theme.c("topbar.fg")),
            );
            ui.add_space(16.0);
            let undo = ui.add_enabled_ui(can_undo, |ui| {
                widgets::on_topbar(ui, theme, texts.t(&gafira_doc::msg!("editor.undo")), false)
            });
            asked.undo |= undo.inner.clicked();
            let redo = ui.add_enabled_ui(can_redo, |ui| {
                widgets::on_topbar(ui, theme, texts.t(&gafira_doc::msg!("editor.redo")), false)
            });
            asked.redo |= redo.inner.clicked();
            ui.add_space(16.0);
            let status = if modified {
                gafira_doc::msg!("editor.unsaved")
            } else {
                gafira_doc::msg!("editor.saved")
            };
            ui.label(
                RichText::new(texts.t(&status))
                    .font(theme.font("caption"))
                    .color(theme.c("topbar.fg.muted")),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                widgets::primary(ui, theme, texts.t(&gafira_doc::msg!("editor.export")));
                ui.add_space(8.0);
                widgets::on_topbar(
                    ui,
                    theme,
                    texts.t(&gafira_doc::msg!("editor.versions")),
                    false,
                );
            });
        });
        egui::TopBottomPanel::bottom("editor.bottombar")
            .exact_height(px(theme.tokens.size.bottombar))
            .frame(
                egui::Frame::NONE
                    .fill(theme.c("bg"))
                    .stroke(Stroke::new(1.0, theme.c("border")))
                    .inner_margin(egui::Margin::symmetric(12, 0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(texts.t(&gafira_doc::msg!("bottom.notes")));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let pages = document.as_ref().map_or(0, |d| d.pages().len());
                        asked.add_page |= ui
                            .button(texts.t(&gafira_doc::msg!("editor.add_page")))
                            .clicked();
                        ui.add_space(16.0);
                        ui.label(texts.t(&gafira_doc::msg!("bottom.pages")));
                        ui.add_space(16.0);
                        ui.label(texts.t(&gafira_doc::msg!(
                            "editor.page_of",
                            current = Count(u64::from(pages > 0)),
                            total = Count(pages as u64)
                        )));
                        ui.add_space(16.0);
                        ui.label(texts.t(&gafira_doc::msg!(
                            "format.percent",
                            value = Text(format!("{:.0}", self.zoom * 100.0))
                        )));
                    });
                });
            });
    }

    fn rail(&mut self, ctx: &egui::Context, theme: &Theme, texts: &Texts) {
        egui::SidePanel::left("editor.rail")
            .exact_width(px(theme.tokens.size.rail))
            .resizable(false)
            .frame(egui::Frame::NONE.fill(theme.c("topbar.bg")))
            .show(ctx, |ui| {
                ui.add_space(8.0);
                for (index, key) in TABS.iter().enumerate() {
                    let active = index == self.tab;
                    let (rect, response) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 56.0),
                        egui::Sense::click(),
                    );
                    if response.hovered() {
                        ui.painter().rect_filled(rect, 0.0, theme.c("topbar.hover"));
                    }
                    if active {
                        let bar = Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height()));
                        ui.painter().rect_filled(bar, 0.0, theme.c("primary.bg"));
                    }
                    let color = if active {
                        theme.c("topbar.fg")
                    } else {
                        theme.c("topbar.fg.muted")
                    };
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        texts.t(&gafira_doc::msg::Message {
                            key: gafira_doc::msg::MessageKey(key),
                            args: Vec::new(),
                        }),
                        theme.font("caption"),
                        color,
                    );
                    if response.clicked() {
                        self.tab = index;
                    }
                }
            });
    }

    fn side_panel(&self, ctx: &egui::Context, theme: &Theme, texts: &Texts) {
        egui::SidePanel::left("editor.side")
            .default_width(px(theme.tokens.size.panel_left))
            .width_range(280.0..=480.0)
            .frame(
                egui::Frame::NONE
                    .fill(theme.c("bg"))
                    .stroke(Stroke::new(1.0, theme.c("border")))
                    .inner_margin(egui::Margin::same(16)),
            )
            .show(ctx, |ui| {
                let mut query = String::new();
                ui.add(
                    egui::TextEdit::singleline(&mut query)
                        .hint_text(texts.t(&gafira_doc::msg!("editor.search_templates")))
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(16.0);
                let card = Vec2::new((ui.available_width() - 12.0) / 2.0, 120.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(12.0);
                    for _ in 0..6 {
                        let (rect, _) = ui.allocate_exact_size(card, egui::Sense::hover());
                        ui.painter().rect(
                            rect,
                            theme.radius_md(),
                            theme.c("surface"),
                            Stroke::new(1.0, theme.c("border")),
                            egui::StrokeKind::Inside,
                        );
                    }
                });
            });
    }

    fn inspector(&self, ctx: &egui::Context, theme: &Theme, texts: &Texts) {
        egui::SidePanel::right("editor.inspector")
            .exact_width(px(theme.tokens.size.panel_right))
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(theme.c("surface"))
                    .stroke(Stroke::new(1.0, theme.c("border")))
                    .inner_margin(egui::Margin::same(16)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (index, key) in [
                        gafira_doc::msg!("inspector.position"),
                        gafira_doc::msg!("inspector.layers"),
                        gafira_doc::msg!("inspector.image"),
                    ]
                    .iter()
                    .enumerate()
                    {
                        let color = if index == 0 {
                            theme.c("text")
                        } else {
                            theme.c("text.muted")
                        };
                        ui.label(
                            RichText::new(texts.t(key))
                                .font(theme.font("ui-label"))
                                .color(color),
                        );
                        ui.add_space(8.0);
                    }
                });
                ui.add_space(24.0);
                ui.label(
                    RichText::new(texts.t(&gafira_doc::msg!("inspector.empty")))
                        .font(theme.font("body-sm"))
                        .color(theme.c("text.muted")),
                );
            });
    }

    fn canvas(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        document: Option<&Document>,
    ) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme.c("workspace")))
            .show(ctx, |ui| {
                let area = ui.max_rect();
                let (scroll, command) =
                    ui.input(|i| (i.smooth_scroll_delta.y, i.modifiers.command));
                if command && scroll != 0.0 && ui.rect_contains_pointer(area) {
                    self.zoom_factor = (self.zoom_factor * (1.0 + scroll / 400.0)).clamp(0.1, 32.0);
                }
                let sizes = document.map(page_sizes).unwrap_or_default();
                let Some(first) = sizes.first().copied() else {
                    return;
                };
                self.zoom = (fit_zoom(
                    (first.x, first.y),
                    (area.width(), area.height()),
                    CANVAS_MARGIN,
                ) * self.zoom_factor)
                    .clamp(0.05, 32.0);
                let zoom = self.zoom;
                let width =
                    sizes.iter().map(|s| s.x).fold(0.0, f32::max) * zoom + 2.0 * CANVAS_MARGIN;
                let height: f32 =
                    sizes.iter().map(|s| s.y * zoom + PAGE_GAP).sum::<f32>() + CANVAS_MARGIN;
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let (rect, _) = ui.allocate_exact_size(
                            Vec2::new(width.max(area.width()), height.max(area.height())),
                            egui::Sense::hover(),
                        );
                        let painter = ui.painter();
                        let mut top =
                            rect.min.y + CANVAS_MARGIN.max((area.height() - height).max(0.0) / 2.0);
                        for (index, size) in sizes.iter().enumerate() {
                            let page = Rect::from_min_size(
                                Pos2::new(rect.center().x - size.x * zoom / 2.0, top),
                                *size * zoom,
                            );
                            if let Some(Shadow::Drop {
                                x,
                                y,
                                blur,
                                color,
                                alpha,
                            }) = theme.tokens.shadow.get(&1).copied()
                            {
                                let shadow = egui::epaint::Shadow {
                                    offset: [to_i8(x), to_i8(y)],
                                    blur: to_u8(blur),
                                    spread: 0,
                                    color: color32(color).gamma_multiply(alpha),
                                };
                                painter.add(shadow.as_shape(page, 0.0));
                            }
                            // The page colour belongs to the document (white
                            // by default, 03 §7.1), not to the theme.
                            painter.rect_filled(page, 0.0, Color32::WHITE);
                            painter.text(
                                Pos2::new(page.min.x, page.min.y - 8.0),
                                Align2::LEFT_BOTTOM,
                                texts.t(&gafira_doc::msg!(
                                    "editor.page",
                                    number = Count(index as u64 + 1)
                                )),
                                theme.font("caption"),
                                theme.c("text.secondary"),
                            );
                            top = page.max.y + PAGE_GAP;
                        }
                    });
            });
    }
}

/// Run what the person asked, through the engine (02 §5.5: every change is
/// a command); a refused action is logged and the view stays.
fn apply(engine: &mut Gafira, document: &Option<Arc<Document>>, asked: &Asked) -> Go {
    if asked.undo
        && let Err(error) = engine.edit().undo()
    {
        eprintln!("undo: {error}");
    }
    if asked.redo
        && let Err(error) = engine.edit().redo()
    {
        eprintln!("redo: {error}");
    }
    let last = document
        .as_ref()
        .and_then(|d| d.pages().last().map(|page| (d.pages().len(), page.size)));
    if asked.add_page
        && let Some((at, size)) = last
        && let Err(error) =
            engine
                .edit()
                .execute(Box::new(InsertPage::new(at, String::new(), size)))
    {
        eprintln!("add page: {error}");
    }
    if asked.home { Go::Home } else { Go::Stay }
}

/// A shadow offset as egui's `i8`; the tokens hold small whole numbers.
fn to_i8(value: f32) -> i8 {
    let mut out: i8 = 0;
    while f32::from(out) < value.min(127.0) {
        out += 1;
    }
    out
}

/// A shadow blur as egui's `u8`.
fn to_u8(value: f32) -> u8 {
    let mut out: u8 = 0;
    while f32::from(out) < value.min(255.0) {
        out += 1;
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use std::io;
    use std::path::Path;
    use std::sync::Arc;

    use gafira_doc::{ContentHash, Document};
    use gafira_engine::{
        EngineConfig, EventChannel, Gafira, NoopWaker, Ports, Registries, SettingsPorts,
        SystemPorts,
    };
    use gafira_i18n::D08LocaleResolver;
    use gafira_platform::{
        EnvLocale, JsonSettings, RandomIds, StdFileSystem, StderrNotifier, SystemClock,
    };
    use gafira_ports::license::NoLicensing;
    use gafira_ports::repository::{LoadedProject, ProjectRepository, RepositoryError};
    use gafira_ports::store::{ContentStore, StoreError};
    use gafira_ports::system::ReadSeek;

    use super::*;
    use crate::format::StartFormat;
    use crate::shell::start;

    struct Nothing;

    fn none() -> io::Error {
        io::ErrorKind::Unsupported.into()
    }

    impl ProjectRepository for Nothing {
        fn load(&self, _: &Path) -> Result<LoadedProject, RepositoryError> {
            Err(RepositoryError::Io(none()))
        }
        fn save(&self, _: &Path, _: &Document) -> Result<(), RepositoryError> {
            Err(RepositoryError::Io(none()))
        }
    }

    impl ContentStore for Nothing {
        fn put(&self, _: &mut dyn io::Read) -> Result<ContentHash, StoreError> {
            Err(StoreError::Io(none()))
        }
        fn open(&self, _: ContentHash) -> Result<Box<dyn ReadSeek + Send>, StoreError> {
            Err(StoreError::Io(none()))
        }
        fn contains(&self, _: ContentHash) -> Result<bool, StoreError> {
            Ok(false)
        }
    }

    /// An engine over the real platform adapters, with no format and no
    /// store, as the app composes it (spec 018).
    pub(crate) fn engine() -> Gafira {
        let dir = std::env::temp_dir().join(format!("gafira-ui-{}", std::process::id()));
        let clock = Arc::new(SystemClock::new());
        let ports = Ports::new(
            Arc::new(Nothing),
            Arc::new(Nothing),
            SystemPorts::new(Arc::new(StdFileSystem), clock.clone(), Arc::new(RandomIds)),
            Arc::new(StderrNotifier),
            Arc::new(NoLicensing),
            SettingsPorts::new(
                Arc::new(JsonSettings::at(dir.join("settings.json"))),
                Arc::new(D08LocaleResolver::new(Arc::new(EnvLocale))),
            ),
        );
        Gafira::new(
            ports,
            Registries::new(),
            EngineConfig::new(dir).expect("a data folder"),
            EventChannel::new(Arc::new(NoopWaker), clock),
        )
        .expect("an engine")
    }

    fn pages(engine: &mut Gafira) -> Vec<Vec2> {
        page_sizes(&engine.edit().document().expect("a document"))
    }

    #[test]
    fn s18_06_start_creates_one_page_of_the_format_with_an_identity() {
        let mut engine = engine();
        start(&mut engine, StartFormat::VideoVertical).expect("a new document");
        assert_eq!(pages(&mut engine), [Vec2::new(1080.0, 1920.0)]);
        let document = engine.edit().document().expect("a document");
        assert!(!document.project_id().is_nil(), "D-63");

        start(&mut engine, StartFormat::Print).expect("another new document");
        let a4 = pages(&mut engine)[0];
        assert!(
            (a4.x - 793.7).abs() < 0.1 && (a4.y - 1122.5).abs() < 0.1,
            "{a4:?}"
        );
    }

    #[test]
    fn s18_07_add_page_undo_and_redo_go_through_the_engine() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let snapshot = engine.edit().document();
        let add = Asked {
            add_page: true,
            ..Asked::default()
        };
        assert!(matches!(apply(&mut engine, &snapshot, &add), Go::Stay));
        assert_eq!(pages(&mut engine).len(), 2);
        assert_eq!(
            pages(&mut engine)[1],
            Vec2::new(1080.0, 1080.0),
            "same size as the last"
        );
        assert_eq!(engine.edit().can_undo_redo(), (true, false));

        let snapshot = engine.edit().document();
        apply(
            &mut engine,
            &snapshot,
            &Asked {
                undo: true,
                ..Asked::default()
            },
        );
        assert_eq!(pages(&mut engine).len(), 1);
        assert_eq!(engine.edit().can_undo_redo(), (true, true));

        let snapshot = engine.edit().document();
        apply(
            &mut engine,
            &snapshot,
            &Asked {
                redo: true,
                ..Asked::default()
            },
        );
        assert_eq!(pages(&mut engine).len(), 2);
        assert!(engine.projects().is_modified());
        assert!(matches!(
            apply(
                &mut engine,
                &snapshot,
                &Asked {
                    home: true,
                    ..Asked::default()
                }
            ),
            Go::Home
        ));
    }
}
