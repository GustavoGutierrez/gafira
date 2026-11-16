//! The regions of the Design editor (03 §7.1, §7.3): top bar, tab rail, side
//! panel, canvas with the page, inspector and bottom bar.

use std::sync::Arc;

use egui::{Align, Align2, Color32, Layout, Pos2, Rect, RichText, Stroke, Vec2};
use gafira_doc::{Document, NodeId, PageId, Placement, ShapeGeometry};
use gafira_edit::{GestureId, InsertNodes, InsertPage, Property, RemoveNodes, SetProperty};
use gafira_engine::Gafira;
use gafira_theme::Shadow;

use crate::canvas::{self, Shape};
use crate::files::{FileAction, project_name};
use crate::format::fit_zoom;
use crate::shell::Go;
use crate::style::{Theme, color32, px};
use crate::text::Texts;
use crate::versions::Versions;
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
    /// The Inspector tab on display: 0 Posición, 1 Capas, 2 Imagen.
    inspector_tab: usize,
    /// The selected element (03 §9.1); one at a time for now.
    selected: Option<(PageId, NodeId)>,
    /// The drag in progress: its gesture (one undo entry, ARQ-33), where the
    /// element started and how far the pointer went.
    drag: Option<Drag>,
    /// «Versiones» and «Guardar versión…» (spec 031).
    versions: Versions,
}

struct Drag {
    gesture: Option<GestureId>,
    origin: Placement,
    offset: Vec2,
    /// The resize handle held, or `None` for a move.
    handle: Option<canvas::Handle>,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            tab: 0,
            zoom_factor: 1.0,
            zoom: 1.0,
            inspector_tab: 0,
            selected: None,
            drag: None,
            versions: Versions::default(),
        }
    }
}

/// What the person asked for this frame; applied after drawing, so no
/// panel holds the engine while another reads it.
#[derive(Default)]
struct Asked {
    home: bool,
    file: Option<FileAction>,
    /// «Versiones» in the top bar, «Guardar versión…» in Archivo.
    versions: bool,
    save_version: bool,
    undo: bool,
    redo: bool,
    add_page: bool,
    /// A new piece of this geometry on the selected element's page, or the
    /// first page.
    insert: Option<ShapeGeometry>,
    /// A new selection (`Some(None)` clears it).
    select: Option<Option<(PageId, NodeId)>>,
    drag_begin: Option<(Placement, Option<canvas::Handle>)>,
    drag_to: Option<Vec2>,
    drag_end: bool,
    delete: bool,
    /// A placement typed in the Inspector for the selected element.
    place: Option<Placement>,
    /// A fill picked in the Inspector (`Some(None)` removes it).
    fill: Option<Option<gafira_color::Color>>,
    /// An opacity set in the Inspector, 0 to 1.
    opacity: Option<f32>,
    /// A new text element of this preset (Texto tab, spec 027).
    /// With the words it starts with, already in the interface language.
    insert_text: Option<(canvas::TextPreset, String)>,
    /// New rich text for the selected text element (Inspector).
    text: Option<gafira_doc::RichText>,
    /// A root element to move to this index of its page (Capas).
    reorder: Option<(PageId, NodeId, usize)>,
    /// Ver → Modo oscuro switched to this value (D-82).
    dark: Option<bool>,
}

/// What the top bar shows about the session.
struct Header<'a> {
    can_undo: bool,
    can_redo: bool,
    modified: bool,
    name: Option<String>,
    notice: Option<&'a str>,
}

/// The selected shape, if it is still in `document` (an undo may have taken
/// it away).
fn selected_shape(
    document: Option<&Document>,
    selected: Option<(PageId, NodeId)>,
) -> Option<Shape> {
    let (page, node) = selected?;
    let page = document?.page(page)?;
    canvas::shapes(page)
        .into_iter()
        .find(|shape| shape.node == node)
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
        notice: Option<&str>,
    ) -> Go {
        let document = engine.edit().document();
        let (can_undo, can_redo) = engine.edit().can_undo_redo();
        let header = Header {
            can_undo,
            can_redo,
            modified: engine.projects().is_modified(),
            name: project_name(engine.projects().path().as_deref()),
            notice,
        };
        let mut asked = Asked::default();
        ctx.input(|i| {
            if i.modifiers.command && i.key_pressed(egui::Key::S) {
                asked.file = Some(if i.modifiers.shift {
                    FileAction::SaveAs
                } else {
                    FileAction::Save
                });
            }
            if i.modifiers.command && i.key_pressed(egui::Key::O) {
                asked.file = Some(FileAction::Open);
            }
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
        let typing = ctx.wants_keyboard_input();
        if !typing
            && ctx
                .input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace))
        {
            asked.delete = true;
        }
        let selection = selected_shape(document.as_deref(), self.selected);
        if selection.is_none() {
            self.selected = None;
        }
        self.bars(ctx, theme, texts, &document, &header, &mut asked);
        self.rail(ctx, theme, texts);
        self.side_panel(ctx, theme, texts, &mut asked);
        self.inspector(
            ctx,
            theme,
            texts,
            document.as_deref(),
            selection.as_ref(),
            &mut asked,
        );
        self.canvas(ctx, theme, texts, document.as_deref(), &mut asked);
        if asked.versions {
            self.versions.open_list();
        }
        if asked.save_version {
            self.versions.ask_name();
        }
        self.versions.show(ctx, engine, theme, texts);
        self.apply(engine, &document, &asked, theme)
    }

    fn bars(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        document: &Option<Arc<Document>>,
        header: &Header<'_>,
        asked: &mut Asked,
    ) {
        widgets::topbar(ctx, theme, "editor.topbar", |ui| {
            if widgets::on_topbar(ui, theme, texts.t(&gafira_doc::msg!("nav.home")), true).clicked()
            {
                asked.home = true;
            }
            ui.add_space(12.0);
            let file = RichText::new(texts.t(&gafira_doc::msg!("menu.file")))
                .font(theme.font("ui-label"))
                .color(theme.c("topbar.fg.muted"));
            ui.menu_button(file, |ui| {
                ui.set_min_width(200.0);
                if ui.button(texts.t(&gafira_doc::msg!("file.new"))).clicked() {
                    asked.home = true;
                    ui.close();
                }
                for (key, action) in [
                    (gafira_doc::msg!("file.open"), FileAction::Open),
                    (gafira_doc::msg!("file.save"), FileAction::Save),
                    (gafira_doc::msg!("file.save_as"), FileAction::SaveAs),
                    (gafira_doc::msg!("file.save_copy"), FileAction::SaveCopy),
                ] {
                    if ui.button(texts.t(&key)).clicked() {
                        asked.file = Some(action);
                        ui.close();
                    }
                }
                if ui
                    .button(texts.t(&gafira_doc::msg!("file.save_version")))
                    .clicked()
                {
                    asked.save_version = true;
                    ui.close();
                }
            });
            widgets::on_topbar(ui, theme, texts.t(&gafira_doc::msg!("menu.edit")), false);
            let view = RichText::new(texts.t(&gafira_doc::msg!("menu.view")))
                .font(theme.font("ui-label"))
                .color(theme.c("topbar.fg.muted"));
            ui.menu_button(view, |ui| {
                ui.set_min_width(200.0);
                let mut dark = theme.dark;
                if ui
                    .checkbox(&mut dark, texts.t(&gafira_doc::msg!("view.dark_mode")))
                    .changed()
                {
                    asked.dark = Some(dark);
                    ui.close();
                }
            });
            widgets::on_topbar(ui, theme, texts.t(&gafira_doc::msg!("menu.size")), false);
            ui.add_space(24.0);
            ui.label(
                RichText::new(
                    header
                        .name
                        .clone()
                        .unwrap_or_else(|| texts.t(&gafira_doc::msg!("editor.untitled"))),
                )
                .font(theme.font("ui-label"))
                .color(theme.c("topbar.fg")),
            );
            ui.add_space(16.0);
            let undo = ui.add_enabled_ui(header.can_undo, |ui| {
                widgets::on_topbar(ui, theme, texts.t(&gafira_doc::msg!("editor.undo")), false)
            });
            asked.undo |= undo.inner.clicked();
            let redo = ui.add_enabled_ui(header.can_redo, |ui| {
                widgets::on_topbar(ui, theme, texts.t(&gafira_doc::msg!("editor.redo")), false)
            });
            asked.redo |= redo.inner.clicked();
            ui.add_space(16.0);
            let status = if header.modified {
                gafira_doc::msg!("editor.unsaved")
            } else {
                gafira_doc::msg!("editor.saved")
            };
            let (text, color) = match header.notice {
                Some(notice) => (notice.to_owned(), theme.c("primary.bg")),
                None => (texts.t(&status), theme.c("topbar.fg.muted")),
            };
            ui.label(RichText::new(text).font(theme.font("caption")).color(color));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                widgets::primary(ui, theme, texts.t(&gafira_doc::msg!("editor.export")));
                ui.add_space(8.0);
                asked.versions |= widgets::on_topbar(
                    ui,
                    theme,
                    texts.t(&gafira_doc::msg!("editor.versions")),
                    false,
                )
                .clicked();
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

    fn side_panel(&self, ctx: &egui::Context, theme: &Theme, texts: &Texts, asked: &mut Asked) {
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
                if TABS[self.tab] == "tab.text" {
                    for (key, words, preset) in [
                        (
                            gafira_doc::msg!("text.add_title"),
                            gafira_doc::msg!("text.title"),
                            canvas::TextPreset::Title,
                        ),
                        (
                            gafira_doc::msg!("text.add_subtitle"),
                            gafira_doc::msg!("text.subtitle"),
                            canvas::TextPreset::Subtitle,
                        ),
                        (
                            gafira_doc::msg!("text.add_body"),
                            gafira_doc::msg!("text.body"),
                            canvas::TextPreset::Body,
                        ),
                    ] {
                        let (size, display) = preset.size_and_display();
                        let font = if display {
                            egui::FontId::new(
                                size as f32 * 0.4,
                                egui::FontFamily::Name(crate::style::DISPLAY.into()),
                            )
                        } else {
                            egui::FontId::proportional(size as f32 * 0.5)
                        };
                        let button = egui::Button::new(RichText::new(texts.t(&key)).font(font))
                            .min_size(Vec2::new(ui.available_width(), 48.0));
                        if ui.add(button).clicked() {
                            asked.insert_text = Some((preset, texts.t(&words)));
                        }
                        ui.add_space(8.0);
                    }
                    return;
                }
                if TABS[self.tab] == "tab.pieces" {
                    ui.label(
                        RichText::new(texts.t(&gafira_doc::msg!("pieces.shapes")))
                            .font(theme.font("ui-label"))
                            .color(theme.c("text")),
                    );
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        for (key, geometry) in [
                            (gafira_doc::msg!("pieces.rect"), ShapeGeometry::Rect),
                            (gafira_doc::msg!("pieces.ellipse"), ShapeGeometry::Ellipse),
                        ] {
                            if widgets::secondary(ui, theme, texts.t(&key)).clicked() {
                                asked.insert = Some(geometry);
                            }
                        }
                    });
                    return;
                }
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

    fn inspector(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        document: Option<&Document>,
        selection: Option<&Shape>,
        asked: &mut Asked,
    ) {
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
                        let color = if index == self.inspector_tab {
                            theme.c("text")
                        } else {
                            theme.c("text.muted")
                        };
                        let tab = egui::Button::new(
                            RichText::new(texts.t(key))
                                .font(theme.font("ui-label"))
                                .color(color),
                        )
                        .frame(false);
                        if ui.add(tab).clicked() {
                            self.inspector_tab = index;
                        }
                        ui.add_space(4.0);
                    }
                });
                ui.add_space(24.0);
                if self.inspector_tab == 1 {
                    self.layers(ui, theme, texts, document, asked);
                    return;
                }
                let Some(shape) = selection else {
                    ui.label(
                        RichText::new(texts.t(&gafira_doc::msg!("inspector.empty")))
                            .font(theme.font("body-sm"))
                            .color(theme.c("text.muted")),
                    );
                    return;
                };
                if let Some(text) = &shape.text {
                    text_fields(ui, theme, texts, text, asked);
                    ui.add_space(16.0);
                }
                let mut placement = shape.placement;
                let mut changed = false;
                egui::Grid::new("inspector.position")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        for (key, value, min) in [
                            (gafira_doc::msg!("inspector.x"), &mut placement.x, f64::MIN),
                            (gafira_doc::msg!("inspector.y"), &mut placement.y, f64::MIN),
                            (gafira_doc::msg!("inspector.width"), &mut placement.w, 1.0),
                            (gafira_doc::msg!("inspector.height"), &mut placement.h, 1.0),
                        ] {
                            ui.label(texts.t(&key));
                            changed |= ui
                                .add(
                                    egui::DragValue::new(value)
                                        .speed(1.0)
                                        .range(min..=f64::MAX)
                                        .max_decimals(1),
                                )
                                .changed();
                            ui.end_row();
                        }
                    });
                if changed {
                    asked.place = Some(placement);
                }
                ui.add_space(16.0);
                if shape.text.is_none() {
                    ui.horizontal(|ui| {
                        ui.label(texts.t(&gafira_doc::msg!("inspector.fill")));
                        let mut color = shape
                            .fill
                            .map_or(Color32::TRANSPARENT, canvas::screen_color);
                        if ui.color_edit_button_srgba(&mut color).changed() {
                            asked.fill = Some(Some(canvas::document_color(color)));
                        }
                        if shape.fill.is_some()
                            && ui
                                .small_button(texts.t(&gafira_doc::msg!("inspector.no_fill")))
                                .clicked()
                        {
                            asked.fill = Some(None);
                        }
                    });
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(texts.t(&gafira_doc::msg!("inspector.opacity")));
                    let mut percent = shape.opacity * 100.0;
                    // The unit comes from the catalog, like every visible text (ARQ-97).
                    let slider = egui::Slider::new(&mut percent, 0.0..=100.0)
                        .max_decimals(0)
                        .custom_formatter(|value, _| {
                            texts.t(&gafira_doc::msg!(
                                "format.percent",
                                value = Text(format!("{value:.0}"))
                            ))
                        });
                    if ui.add(slider).changed() {
                        asked.opacity = Some(percent / 100.0);
                    }
                });
            });
    }

    /// The Capas tab: the root elements of the selected element's page (or
    /// the first page), front first; a click selects, Subir and Bajar move
    /// the selected one by one step (03 §2.4).
    fn layers(
        &self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        document: Option<&Document>,
        asked: &mut Asked,
    ) {
        let Some(document) = document else { return };
        let page = self
            .selected
            .and_then(|(page, _)| document.page(page))
            .or_else(|| document.pages().first().map(AsRef::as_ref));
        let Some(page) = page else { return };
        let rows = canvas::layers(page);
        if rows.is_empty() {
            ui.label(
                RichText::new(texts.t(&gafira_doc::msg!("layers.empty")))
                    .font(theme.font("body-sm"))
                    .color(theme.c("text.muted")),
            );
            return;
        }
        let last = page.roots.len().saturating_sub(1);
        for row in rows {
            let selected = self.selected == Some((page.id, row.node));
            let label = row.name.clone().unwrap_or_else(|| {
                texts.t(&gafira_doc::msg::Message {
                    key: gafira_doc::msg::MessageKey(row.kind),
                    args: Vec::new(),
                })
            });
            ui.horizontal(|ui| {
                if ui.selectable_label(selected, label).clicked() {
                    asked.select = Some(Some((page.id, row.node)));
                }
                if selected {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let down =
                            egui::Button::new(texts.t(&gafira_doc::msg!("layers.down"))).small();
                        if ui.add_enabled(row.index > 0, down).clicked() {
                            asked.reorder = Some((page.id, row.node, row.index - 1));
                        }
                        let up = egui::Button::new(texts.t(&gafira_doc::msg!("layers.up"))).small();
                        if ui.add_enabled(row.index < last, up).clicked() {
                            asked.reorder = Some((page.id, row.node, row.index + 1));
                        }
                    });
                }
            });
        }
    }

    fn canvas(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        document: Option<&Document>,
        asked: &mut Asked,
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
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(width.max(area.width()), height.max(area.height())),
                            egui::Sense::click_and_drag(),
                        );
                        let painter = ui.painter();
                        let pages = document.map(Document::pages).unwrap_or_default();
                        let pointer = response.interact_pointer_pos();
                        let pressed = response.clicked() || response.drag_started();
                        let mut picked: Option<(PageId, NodeId, Placement)> = None;
                        let mut grabbed: Option<(Placement, canvas::Handle)> = None;
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
                            if let Some(page_doc) = pages.get(index) {
                                let shapes = canvas::shapes(page_doc);
                                for shape in &shapes {
                                    let at = canvas::on_screen(page, zoom, &shape.placement);
                                    if let Some(text) = &shape.text {
                                        draw_text(painter, theme, at, zoom, shape.opacity, text);
                                    }
                                    let fill = canvas::paint(shape);
                                    match shape.geometry {
                                        ShapeGeometry::Rect => {
                                            painter.rect_filled(at, 0.0, fill);
                                        }
                                        ShapeGeometry::Ellipse => {
                                            painter.add(egui::Shape::ellipse_filled(
                                                at.center(),
                                                at.size() / 2.0,
                                                fill,
                                            ));
                                        }
                                    }
                                    if self.selected == Some((shape.page, shape.node)) {
                                        let selection = theme.c("canvas.selection");
                                        painter.rect_stroke(
                                            at.expand(1.0),
                                            0.0,
                                            Stroke::new(2.0, selection),
                                            egui::StrokeKind::Outside,
                                        );
                                        for handle in canvas::HANDLES {
                                            let square = Rect::from_center_size(
                                                canvas::handle_at(at, handle),
                                                Vec2::splat(canvas::HANDLE_HALF * 2.0),
                                            );
                                            painter.rect(
                                                square,
                                                2.0,
                                                theme.c("surface"),
                                                Stroke::new(1.5, selection),
                                                egui::StrokeKind::Inside,
                                            );
                                        }
                                        if response.drag_started()
                                            && let Some(pos) = pointer
                                            && let Some(handle) = canvas::handle_hit(at, pos)
                                        {
                                            grabbed = Some((shape.placement, handle));
                                        }
                                    }
                                }
                                if pressed
                                    && let Some(at) = pointer
                                    && page.contains(at)
                                    && let Some(node) = canvas::hit(&shapes, page, zoom, at)
                                {
                                    let placement = shapes
                                        .iter()
                                        .find(|shape| shape.node == node)
                                        .map(|shape| shape.placement);
                                    if let Some(placement) = placement {
                                        picked = Some((page_doc.id, node, placement));
                                    }
                                }
                            }
                            top = page.max.y + PAGE_GAP;
                        }
                        if let Some((placement, handle)) = grabbed {
                            // A handle of the selection wins over whatever
                            // lies under it.
                            asked.drag_begin = Some((placement, Some(handle)));
                        } else {
                            if pressed {
                                asked.select = Some(picked.map(|(page, node, _)| (page, node)));
                            }
                            if response.drag_started()
                                && let Some((_, _, placement)) = picked
                            {
                                asked.drag_begin = Some((placement, None));
                            }
                        }
                        if response.dragged() && self.drag.is_some() {
                            asked.drag_to = Some(response.drag_delta());
                        }
                        if response.drag_stopped() {
                            asked.drag_end = true;
                        }
                    });
            });
    }
}

impl Editor {
    /// Run what the person asked, through the engine (02 §5.5: every change
    /// is a command); a refused action is logged and the view stays.
    fn apply(
        &mut self,
        engine: &mut Gafira,
        document: &Option<Arc<Document>>,
        asked: &Asked,
        theme: &Theme,
    ) -> Go {
        let log = |what: &str, result: Result<(), gafira_engine::EngineError>| {
            if let Err(error) = result {
                eprintln!("{what}: {error}");
            }
        };
        if asked.undo {
            log("undo", engine.edit().undo().map(drop));
        }
        if asked.redo {
            log("redo", engine.edit().redo().map(drop));
        }
        let last = document
            .as_ref()
            .and_then(|d| d.pages().last().map(|page| (d.pages().len(), page.size)));
        if asked.add_page
            && let Some((at, size)) = last
        {
            log(
                "add page",
                engine
                    .edit()
                    .execute(Box::new(InsertPage::new(at, String::new(), size)))
                    .map(drop),
            );
        }
        if let Some(select) = asked.select {
            self.selected = select;
        }
        if let Some(geometry) = asked.insert
            && let Some(document) = document
        {
            let page = self
                .selected
                .and_then(|(page, _)| document.page(page))
                .or_else(|| document.pages().first().map(AsRef::as_ref));
            if let Some(page) = page
                && let Ok(size) = page.size.to_px(document.dpi())
            {
                let px = Vec2::new(size.width() as f32, size.height() as f32);
                let spec =
                    canvas::new_piece(px, geometry, canvas::document_color(theme.c("accent")));
                let command = InsertNodes::new(page.id, None, page.roots.len(), vec![spec]);
                let next = NodeId::new(document.next_id());
                match engine.edit().execute(Box::new(command)) {
                    Ok(_) => self.selected = Some((page.id, next)),
                    Err(error) => eprintln!("insert: {error}"),
                }
            }
        }
        if let Some((preset, words)) = &asked.insert_text
            && let Some(document) = document
        {
            let page = self
                .selected
                .and_then(|(page, _)| document.page(page))
                .or_else(|| document.pages().first().map(AsRef::as_ref));
            if let Some(page) = page
                && let Ok(size) = page.size.to_px(document.dpi())
            {
                let (font_size, display) = preset.size_and_display();
                let tokens = &theme.tokens;
                let style = gafira_doc::SpanStyle {
                    font: if display {
                        tokens.typography.display.clone()
                    } else {
                        tokens.typography.ui.clone()
                    },
                    size: font_size,
                    color: canvas::document_color(crate::style::color32(tokens.brand.ink)),
                };
                let px = Vec2::new(size.width() as f32, size.height() as f32);
                let spec = canvas::new_text(px, words, &style);
                let command = InsertNodes::new(page.id, None, page.roots.len(), vec![spec]);
                let next = NodeId::new(document.next_id());
                match engine.edit().execute(Box::new(command)) {
                    Ok(_) => self.selected = Some((page.id, next)),
                    Err(error) => eprintln!("insert text: {error}"),
                }
            }
        }
        if let (Some(text), Some((page, node))) = (&asked.text, self.selected) {
            let command =
                SetProperty::new(page, vec![node], Property::TextContent(text.clone()), None);
            log("text", engine.edit().execute(Box::new(command)).map(drop));
        }
        if let Some((origin, handle)) = asked.drag_begin {
            let gesture = engine.edit().begin_gesture().ok();
            self.drag = Some(Drag {
                gesture,
                origin,
                offset: Vec2::ZERO,
                handle,
            });
        }
        if let (Some(delta), Some(drag), Some((page, node))) =
            (asked.drag_to, self.drag.as_mut(), self.selected)
        {
            drag.offset += delta;
            let to = match drag.handle {
                Some(handle) => canvas::resized(&drag.origin, handle, drag.offset, self.zoom),
                None => canvas::moved(&drag.origin, drag.offset, self.zoom),
            };
            let command = SetProperty::new(page, vec![node], Property::Placement(to), drag.gesture);
            log("move", engine.edit().execute(Box::new(command)).map(drop));
        }
        if asked.drag_end && self.drag.take().is_some() {
            log("end gesture", engine.edit().end_gesture());
        }
        if let (Some(placement), Some((page, node))) = (asked.place, self.selected) {
            let command = SetProperty::new(page, vec![node], Property::Placement(placement), None);
            log("place", engine.edit().execute(Box::new(command)).map(drop));
        }
        if let (Some(fill), Some((page, node))) = (asked.fill, self.selected) {
            let command = SetProperty::new(page, vec![node], Property::Fill(fill), None);
            log("fill", engine.edit().execute(Box::new(command)).map(drop));
        }
        if let (Some(opacity), Some((page, node))) = (asked.opacity, self.selected) {
            let command = SetProperty::new(page, vec![node], Property::Opacity(opacity), None);
            log(
                "opacity",
                engine.edit().execute(Box::new(command)).map(drop),
            );
        }
        if let Some((page, node, index)) = asked.reorder {
            let command = gafira_edit::Reorder::new(page, node, index);
            log(
                "reorder",
                engine.edit().execute(Box::new(command)).map(drop),
            );
        }
        if asked.delete
            && let Some((page, node)) = self.selected.take()
        {
            log(
                "delete",
                engine
                    .edit()
                    .execute(Box::new(RemoveNodes::new(page, vec![node])))
                    .map(drop),
            );
        }
        if let Some(dark) = asked.dark {
            Go::Dark(dark)
        } else if asked.home {
            Go::Home
        } else if let Some(action) = asked.file {
            Go::File(action)
        } else {
            Go::Stay
        }
    }
}

/// The Inspector fields of a text element: its words, size and colour,
/// applied to every span (one style per element for now, spec 027).
fn text_fields(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    text: &gafira_doc::TextNode,
    asked: &mut Asked,
) {
    let Some(style) = text.content.first_style().cloned() else {
        return;
    };
    ui.label(
        RichText::new(texts.t(&gafira_doc::msg!("inspector.text"))).font(theme.font("ui-label")),
    );
    let mut words = text.content.plain_text();
    let edit = egui::TextEdit::multiline(&mut words)
        .desired_rows(3)
        .desired_width(f32::INFINITY);
    if ui.add(edit).changed() {
        asked.text = Some(gafira_doc::RichText::plain(&words, &style));
    }
    ui.add_space(8.0);
    let mut size = style.size;
    let mut color = canvas::screen_color(style.color);
    ui.horizontal(|ui| {
        ui.label(texts.t(&gafira_doc::msg!("inspector.font_size")));
        let size_changed = ui
            .add(
                egui::DragValue::new(&mut size)
                    .speed(1.0)
                    .range(1.0..=gafira_doc::TextNode::MAX_SIZE),
            )
            .changed();
        ui.label(texts.t(&gafira_doc::msg!("inspector.text_color")));
        let color_changed = ui.color_edit_button_srgba(&mut color).changed();
        if size_changed || color_changed {
            let restyled = gafira_doc::SpanStyle {
                size,
                color: canvas::document_color(color),
                ..style.clone()
            };
            asked.text = Some(gafira_doc::RichText::plain(&words, &restyled));
        }
    });
}

/// Draw a text element in its box: one galley per paragraph, wrapped to the
/// box width, aligned, stacked downwards. The canvas draws with egui; the
/// document renderer (`gafira-text`) takes over when the canvas renders
/// through `gafira-render`.
fn draw_text(
    painter: &egui::Painter,
    theme: &Theme,
    at: Rect,
    zoom: f32,
    opacity: f32,
    text: &gafira_doc::TextNode,
) {
    let mut y = at.min.y;
    for paragraph in &text.content.paragraphs {
        let Some(style) = paragraph.spans.first().map(|span| &span.style) else {
            continue;
        };
        let words: String = paragraph
            .spans
            .iter()
            .map(|span| span.text.as_str())
            .collect();
        let family = if style.font == theme.tokens.typography.display {
            egui::FontFamily::Name(crate::style::DISPLAY.into())
        } else {
            egui::FontFamily::Proportional
        };
        let size = (style.size as f32 * zoom).max(1.0);
        let color = canvas::screen_color(style.color).gamma_multiply(opacity.clamp(0.0, 1.0));
        let galley = painter.layout(
            words,
            egui::FontId::new(size, family),
            color,
            at.width().max(1.0),
        );
        let x = match text.align {
            gafira_doc::TextAlign::Center => at.center().x - galley.size().x / 2.0,
            gafira_doc::TextAlign::Right => at.max.x - galley.size().x,
            gafira_doc::TextAlign::Left | gafira_doc::TextAlign::Justify => at.min.x,
        };
        let height = galley.size().y.max(size);
        painter.galley(Pos2::new(x, y), galley, color);
        y += height;
    }
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
        SystemProcesses,
    };
    use gafira_ports::license::NoLicensing;
    use gafira_ports::repository::{LoadedProject, ProjectRepository, RepositoryError, Version};
    use gafira_ports::settings::Settings;
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
        fn save(&self, _: &Path, _: &Document, _: &[Version]) -> Result<(), RepositoryError> {
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
        engine_over(Arc::new(Nothing))
    }

    /// [`engine`] with `repository` in place of the absent format.
    pub(crate) fn engine_over(repository: Arc<dyn ProjectRepository>) -> Gafira {
        let dir = std::env::temp_dir().join(format!("gafira-ui-{}", std::process::id()));
        let clock = Arc::new(SystemClock::new());
        let ports = Ports::new(
            repository,
            Arc::new(Nothing),
            SystemPorts::new(
                Arc::new(StdFileSystem),
                clock.clone(),
                Arc::new(RandomIds),
                Arc::new(SystemProcesses::new(dir.join("instances"))),
            ),
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

    /// One frame's [`Asked`] applied by a fresh editor.
    fn run(engine: &mut Gafira, snapshot: &Option<Arc<Document>>, asked: &Asked) -> Go {
        Editor::default().apply(engine, snapshot, asked, &theme())
    }

    fn theme() -> Theme {
        Theme::new(gafira_theme::ThemeTokens::embedded().expect("tokens"))
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
        assert!(matches!(run(&mut engine, &snapshot, &add), Go::Stay));
        assert_eq!(pages(&mut engine).len(), 2);
        assert_eq!(
            pages(&mut engine)[1],
            Vec2::new(1080.0, 1080.0),
            "same size as the last"
        );
        assert_eq!(engine.edit().can_undo_redo(), (true, false));

        let snapshot = engine.edit().document();
        run(
            &mut engine,
            &snapshot,
            &Asked {
                undo: true,
                ..Asked::default()
            },
        );
        assert_eq!(pages(&mut engine).len(), 1);
        assert_eq!(
            engine.edit().can_undo_redo(),
            (false, true),
            "the first page belongs to the new document (spec 016)"
        );

        let snapshot = engine.edit().document();
        run(
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
            run(
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

    #[test]
    fn s19_11_a_project_saved_as_gaf_reopens_with_its_pages_and_identity() {
        let dir = std::env::temp_dir().join(format!("gafira-ui-gaf-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let path = dir.join("poster.gaf");
        let mut engine = engine_over(Arc::new(gafira_format::GafRepository::default()));
        start(&mut engine, StartFormat::Print).expect("a new document");
        let snapshot = engine.edit().document();
        run(
            &mut engine,
            &snapshot,
            &Asked {
                add_page: true,
                ..Asked::default()
            },
        );
        let before = engine.edit().document().expect("a document");

        engine.projects().save_as(&path).expect("saves");
        assert!(!engine.projects().is_modified());
        engine.projects().new_document().expect("another document");
        engine.projects().open(&path).expect("reopens");

        let after = engine.edit().document().expect("a document");
        assert_eq!(*after, *before);
        assert_eq!(engine.projects().path(), Some(path.clone()));
        assert!(!engine.projects().is_modified());
        std::fs::remove_dir_all(dir).expect("clean up");
    }

    fn the_shapes(engine: &mut Gafira) -> Vec<Shape> {
        let document = engine.edit().document().expect("a document");
        canvas::shapes(&document.pages()[0])
    }

    // S20-05
    #[test]
    fn s20_05_a_piece_is_inserted_selected_moved_in_one_undo_and_deleted() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let theme = theme();
        let mut editor = Editor::default();
        let snapshot = engine.edit().document();
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                insert: Some(ShapeGeometry::Ellipse),
                ..Asked::default()
            },
            &theme,
        );
        let shapes = the_shapes(&mut engine);
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].geometry, ShapeGeometry::Ellipse);
        assert_eq!(
            editor.selected,
            Some((shapes[0].page, shapes[0].node)),
            "the new piece is selected"
        );
        let origin = shapes[0].placement;

        // A drag of three moves is one gesture: one undo takes it back.
        editor.zoom = 2.0;
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                drag_begin: Some((origin, None)),
                ..Asked::default()
            },
            &theme,
        );
        for _ in 0..3 {
            editor.apply(
                &mut engine,
                &snapshot,
                &Asked {
                    drag_to: Some(Vec2::new(10.0, 4.0)),
                    ..Asked::default()
                },
                &theme,
            );
        }
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                drag_end: true,
                ..Asked::default()
            },
            &theme,
        );
        let moved = the_shapes(&mut engine)[0].placement;
        assert_eq!((moved.x - origin.x, moved.y - origin.y), (15.0, 6.0));
        engine.edit().undo().expect("an undo");
        assert_eq!(
            the_shapes(&mut engine)[0].placement,
            origin,
            "one entry for the whole drag"
        );
        engine.edit().redo().expect("a redo");

        let mut typed = moved;
        typed.w = 500.0;
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                place: Some(typed),
                ..Asked::default()
            },
            &theme,
        );
        assert_eq!(the_shapes(&mut engine)[0].placement.w, 500.0);

        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                delete: true,
                ..Asked::default()
            },
            &theme,
        );
        assert!(the_shapes(&mut engine).is_empty());
        assert_eq!(editor.selected, None);
        engine.edit().undo().expect("an undo");
        assert_eq!(the_shapes(&mut engine).len(), 1, "delete is undoable");
    }

    // S21-02
    #[test]
    fn s21_02_dark_mode_is_asked_for_and_the_settings_keep_it() {
        use gafira_ports::settings::Appearance;
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let snapshot = engine.edit().document();
        assert!(matches!(
            run(
                &mut engine,
                &snapshot,
                &Asked {
                    dark: Some(true),
                    ..Asked::default()
                }
            ),
            Go::Dark(true)
        ));
        let settings = engine.settings().get().with_appearance(Appearance::Dark);
        engine.settings().set(settings).expect("saves");
        let again = super::tests::engine();
        again.settings().start().expect("loads");
        assert_eq!(again.settings().get().appearance, Appearance::Dark);
        again.settings().set(Settings::default()).expect("resets");
    }

    // S22-03
    #[test]
    fn s22_03_dragging_a_handle_resizes_in_one_undo() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let theme = theme();
        let mut editor = Editor::default();
        let snapshot = engine.edit().document();
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                insert: Some(ShapeGeometry::Rect),
                ..Asked::default()
            },
            &theme,
        );
        let origin = the_shapes(&mut engine)[0].placement;
        editor.zoom = 1.0;
        let corner = canvas::Handle { x: 1, y: 1 };
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                drag_begin: Some((origin, Some(corner))),
                ..Asked::default()
            },
            &theme,
        );
        for _ in 0..2 {
            editor.apply(
                &mut engine,
                &snapshot,
                &Asked {
                    drag_to: Some(Vec2::new(20.0, 10.0)),
                    ..Asked::default()
                },
                &theme,
            );
        }
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                drag_end: true,
                ..Asked::default()
            },
            &theme,
        );
        let resized = the_shapes(&mut engine)[0].placement;
        assert_eq!(
            (resized.x, resized.y),
            (origin.x, origin.y),
            "the top-left corner stays"
        );
        assert_eq!((resized.w - origin.w, resized.h - origin.h), (40.0, 20.0));
        engine.edit().undo().expect("an undo");
        assert_eq!(the_shapes(&mut engine)[0].placement, origin);
    }

    // S23-02
    #[test]
    fn s23_02_fill_and_opacity_change_through_the_engine_and_undo() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let theme = theme();
        let mut editor = Editor::default();
        let snapshot = engine.edit().document();
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                insert: Some(ShapeGeometry::Rect),
                ..Asked::default()
            },
            &theme,
        );
        let blue = canvas::document_color(Color32::from_rgb(0, 0, 255));
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                fill: Some(Some(blue)),
                ..Asked::default()
            },
            &theme,
        );
        assert_eq!(the_shapes(&mut engine)[0].fill, Some(blue));
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                opacity: Some(0.25),
                ..Asked::default()
            },
            &theme,
        );
        assert_eq!(the_shapes(&mut engine)[0].opacity, 0.25);
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                fill: Some(None),
                ..Asked::default()
            },
            &theme,
        );
        assert_eq!(the_shapes(&mut engine)[0].fill, None);
        engine.edit().undo().expect("an undo");
        assert_eq!(the_shapes(&mut engine)[0].fill, Some(blue));
    }

    // S24-02
    #[test]
    fn s24_02_bring_forward_reorders_through_the_engine() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let theme = theme();
        let mut editor = Editor::default();
        for geometry in [ShapeGeometry::Rect, ShapeGeometry::Ellipse] {
            // A fresh snapshot each time, as every frame of the window has.
            let snapshot = engine.edit().document();
            editor.apply(
                &mut engine,
                &snapshot,
                &Asked {
                    insert: Some(geometry),
                    ..Asked::default()
                },
                &theme,
            );
        }
        let page = engine.edit().document().expect("a document").pages()[0].clone();
        let rect = page.roots[0];
        assert_eq!(
            canvas::layers(&page)[0].kind,
            "pieces.ellipse",
            "the newest is in front"
        );
        let snapshot = engine.edit().document();
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                reorder: Some((page.id, rect, 1)),
                ..Asked::default()
            },
            &theme,
        );
        let page = engine.edit().document().expect("a document").pages()[0].clone();
        assert_eq!(
            canvas::layers(&page)[0].node,
            rect,
            "the rectangle came to the front"
        );
        engine.edit().undo().expect("an undo");
        let page = engine.edit().document().expect("a document").pages()[0].clone();
        assert_eq!(page.roots[0], rect);
    }

    // S27-04
    #[test]
    fn s27_04_a_text_is_added_edited_restyled_and_undone_through_the_engine() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let theme = theme();
        let mut editor = Editor::default();
        let snapshot = engine.edit().document();
        let asked = Asked {
            insert_text: Some((canvas::TextPreset::Title, "Título".to_owned())),
            ..Asked::default()
        };
        editor.apply(&mut engine, &snapshot, &asked, &theme);
        let text = |engine: &mut Gafira| the_shapes(engine)[0].text.clone().expect("a text");
        let first = text(&mut engine);
        assert_eq!(first.content.plain_text(), "Título");
        let style = first.content.first_style().expect("a style").clone();
        assert_eq!(
            style.font, theme.tokens.typography.display,
            "a title uses the display family"
        );
        assert_eq!(style.size, 72.0);
        assert!(editor.selected.is_some(), "the new text is selected");

        let edited = gafira_doc::RichText::plain("Hola\nmundo", &style);
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                text: Some(edited.clone()),
                ..Asked::default()
            },
            &theme,
        );
        assert_eq!(text(&mut engine).content, edited);

        let bigger = gafira_doc::SpanStyle {
            size: 90.0,
            ..style.clone()
        };
        let restyled = gafira_doc::RichText::plain("Hola\nmundo", &bigger);
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                text: Some(restyled),
                ..Asked::default()
            },
            &theme,
        );
        assert!(
            text(&mut engine)
                .content
                .spans()
                .all(|span| span.style.size == 90.0)
        );

        let broken = gafira_doc::SpanStyle { size: 0.0, ..style };
        let refused = gafira_doc::RichText::plain("x", &broken);
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                text: Some(refused),
                ..Asked::default()
            },
            &theme,
        );
        assert_eq!(
            text(&mut engine).content.plain_text(),
            "Hola\nmundo",
            "a zero size is refused"
        );

        // Consecutive edits of one text merge into one entry (ARQ-33), so one
        // undo takes back the editing and leaves the inserted title.
        engine.edit().undo().expect("an undo");
        assert_eq!(text(&mut engine).content.plain_text(), "Título");
    }
}
