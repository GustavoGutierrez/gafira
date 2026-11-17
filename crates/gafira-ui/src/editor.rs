//! The regions of the Design editor (03 §7.1, §7.3): top bar, tab rail, side
//! panel, canvas with the page, inspector and bottom bar.

use std::sync::Arc;

use egui::{
    Align, Align2, Color32, Key, KeyboardShortcut, Layout, Modifiers, Pos2, Rect, RichText, Stroke,
    Vec2,
};
use gafira_doc::msg::MessageKey;
use gafira_doc::{Document, NodeId, Page, PageId, Placement, ShapeGeometry};
use gafira_edit::{
    Command, GestureId, InsertNodes, InsertPage, Property, RemoveNodes, SetProperty, Transaction,
};
use gafira_engine::Gafira;
use gafira_theme::Shadow;

use crate::canvas::{self, Shape};
use crate::files::{FileAction, project_name};
use crate::format::fit_zoom;
use crate::icons::{self, Icon, Weight};
use crate::selection::Selection;
use crate::shell::Go;
use crate::style::{Theme, color32, px};
use crate::text::Texts;
use crate::versions::Versions;
use crate::widgets::{self, IconButton, Surface};
use crate::zoom::{self, Zoom, ZoomAsk};

/// The tabs of the rail, in the order of 03 §2.3, with their icons.
const TABS: [(&str, Icon); 6] = [
    ("tab.templates", Icon::Templates),
    ("tab.pieces", Icon::Pieces),
    ("tab.text", Icon::Text),
    ("tab.identity", Icon::Identity),
    ("tab.library", Icon::Library),
    ("tab.drawing", Icon::Drawing),
];

/// The margin around the page on the canvas, in logical px.
const CANVAS_MARGIN: f32 = 48.0;

/// The space between two pages on the canvas, in logical px (03 §7.3: the
/// pages stack vertically).
const PAGE_GAP: f32 = 56.0;

/// The shortcuts of 03 §9.2 that tooltips and the Edición menu show.
const UNDO: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Z);
const REDO: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::Z);
const ZOOM_IN: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Equals);
const ZOOM_OUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Minus);
const ZOOM_FIT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Num0);
const SELECT_ALL: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::A);

/// «Deshacer · Ctrl+Z»: an action and its shortcut as the platform writes it
/// (03 §8.1, Tooltip).
fn with_keys(ctx: &egui::Context, texts: &Texts, action: String, keys: KeyboardShortcut) -> String {
    texts.t(&gafira_doc::msg!(
        "tooltip.shortcut",
        action = Text(action),
        keys = Text(ctx.format_shortcut(&keys))
    ))
}

/// The editor's view state; the document lives in the engine and is read
/// from a snapshot every frame (02 §5.5).
pub(crate) struct Editor {
    tab: usize,
    /// How the zoom is chosen: «Ajustar» or a fixed scale (spec 037).
    view: Zoom,
    /// The zoom of the last frame, for the bottom bar.
    zoom: f32,
    /// The Inspector tab on display: 0 Posición, 1 Capas, 2 Imagen.
    inspector_tab: usize,
    /// The selected elements, all of one page (03 §9.1, spec 040).
    selected: Selection,
    /// The textures of the images on display, by content hash; `None`
    /// when the bytes could not be read or decoded (a placeholder is drawn).
    textures: std::collections::HashMap<gafira_doc::ContentHash, Option<egui::TextureHandle>>,
    /// The drag in progress: its gesture (one undo entry, ARQ-33), where the
    /// selected elements started and how far the pointer went.
    drag: Option<Drag>,
    /// The marquee being drawn from empty canvas (spec 040).
    marquee: Option<Marquee>,
    /// «Versiones» and «Guardar versión…» (spec 031).
    versions: Versions,
}

struct Drag {
    gesture: Option<GestureId>,
    /// The page of the selection.
    page: PageId,
    /// Each selected element and where it started.
    origins: Vec<(NodeId, Placement)>,
    offset: Vec2,
    /// The resize handle held, or `None` for a move. Only a single
    /// selection resizes.
    handle: Option<canvas::Handle>,
}

/// A marquee on the canvas, in screen px: where the drag started, where the
/// pointer is, and whether Shift adds to the selection (spec 040).
#[derive(Clone, Copy, Debug)]
struct Marquee {
    from: Pos2,
    to: Pos2,
    add: bool,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            tab: 0,
            view: Zoom::Fit,
            zoom: 1.0,
            inspector_tab: 0,
            selected: Selection::default(),
            textures: std::collections::HashMap::new(),
            drag: None,
            marquee: None,
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
    /// The name of a version to save, from «Guardar versión» (spec 036).
    version: Option<String>,
    undo: bool,
    redo: bool,
    add_page: bool,
    /// A new piece of this geometry on the selected element's page, or the
    /// first page.
    insert: Option<ShapeGeometry>,
    /// A new selection (an empty one clears it, Esc).
    select: Option<Selection>,
    /// Ctrl+A or Edición → Seleccionar todo (spec 040).
    select_all: bool,
    /// A drag of the selection begins: a move, or a resize by this handle.
    drag_begin: Option<Option<canvas::Handle>>,
    drag_to: Option<Vec2>,
    drag_end: bool,
    delete: bool,
    /// A placement typed in the Inspector for the selected element.
    place: Option<Placement>,
    /// A fill picked in the Inspector (`Some(None)` removes it).
    fill: Option<Option<gafira_color::Color>>,
    /// An opacity set in the Inspector, 0 to 1.
    opacity: Option<f32>,
    /// An outline set in the Inspector (`Some(None)` removes it, spec 038).
    stroke: Option<Option<gafira_doc::ShapeStroke>>,
    /// A corner radius set in the Inspector, in px (spec 038).
    corner_radius: Option<f64>,
    /// Biblioteca → Importar imagen… (spec 029).
    import_image: bool,
    /// A new text element of this preset (Texto tab, spec 027).
    /// With the words it starts with, already in the interface language.
    insert_text: Option<(canvas::TextPreset, String)>,
    /// New rich text for the selected text element (Inspector).
    text: Option<gafira_doc::RichText>,
    /// A root element to move to this index of its page (Capas).
    reorder: Option<(PageId, NodeId, usize)>,
    /// Ver → Modo oscuro switched to this value (D-82).
    dark: Option<bool>,
    /// Archivo → Ajustes (spec 034).
    settings: bool,
    /// «Exportar» was pressed (spec 030).
    export: bool,
    /// A zoom button, menu item or shortcut (spec 037).
    zoom: Option<ZoomAsk>,
}

/// What the top bar shows about the session.
struct Header<'a> {
    can_undo: bool,
    can_redo: bool,
    modified: bool,
    name: Option<String>,
    notice: Option<&'a str>,
}

/// The selected shapes still in `document`, in selection order (an undo may
/// have taken some away).
fn selected_shapes(document: Option<&Document>, selected: &Selection) -> Vec<Shape> {
    let Some(page) = selected.page().and_then(|page| document?.page(page)) else {
        return Vec::new();
    };
    let shapes = canvas::shapes(page);
    selected
        .nodes()
        .iter()
        .filter_map(|node| shapes.iter().find(|shape| shape.node == *node).cloned())
        .collect()
}

/// The selection a finished marquee makes (spec 040): the shapes it touches
/// on the page where it started, or else on the first page it touches.
/// With Shift they join `current`; without, they replace it.
fn marquee_selection(
    current: &Selection,
    marquee: Marquee,
    laid: &[(PageId, Rect, Vec<Shape>)],
    zoom: f32,
) -> Selection {
    let rect = Rect::from_two_pos(marquee.from, marquee.to);
    let touched = |(id, page, shapes): &(PageId, Rect, Vec<Shape>)| {
        (*id, canvas::in_marquee(shapes, *page, zoom, rect))
    };
    let on = laid
        .iter()
        .find(|(_, page, _)| page.contains(marquee.from))
        .map(touched)
        .or_else(|| {
            laid.iter()
                .map(touched)
                .find(|(_, nodes)| !nodes.is_empty())
        });
    let mut next = if marquee.add {
        current.clone()
    } else {
        Selection::default()
    };
    if let Some((page, nodes)) = on {
        next.add(page, nodes);
    }
    next
}

/// One step for many commands: the command itself when there is one, a
/// `Transaction` that merges by gesture when there are more (spec 040).
fn batch(mut commands: Vec<Box<dyn Command>>) -> Option<Box<dyn Command>> {
    match commands.len() {
        0 | 1 => commands.pop(),
        _ => Some(Box::new(Transaction::new(
            MessageKey("edit.set_property"),
            commands,
        ))),
    }
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
    /// The page of the selection, or the first page.
    fn current_page<'a>(&self, document: &'a Document) -> Option<&'a Page> {
        self.selected
            .page()
            .and_then(|page| document.page(page))
            .or_else(|| document.pages().first().map(AsRef::as_ref))
    }

    /// Drops from the selection what `document` no longer draws (an undo
    /// may have taken it away) and returns the shapes that stay, in
    /// selection order (spec 040).
    fn prune(&mut self, document: Option<&Document>) -> Vec<Shape> {
        let shapes = selected_shapes(document, &self.selected);
        self.selected
            .retain(|node| shapes.iter().any(|shape| shape.node == node));
        shapes
    }

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
            for key in [
                egui::Key::Equals,
                egui::Key::Plus,
                egui::Key::Minus,
                egui::Key::Num0,
                egui::Key::Num1,
            ] {
                if i.key_pressed(key) {
                    asked.zoom = ZoomAsk::from_shortcut(i.modifiers.command, key).or(asked.zoom);
                }
            }
        });
        let typing = ctx.wants_keyboard_input();
        if !typing {
            ctx.input(|i| {
                if i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace) {
                    asked.delete = true;
                }
                if i.key_pressed(egui::Key::Escape) {
                    asked.select = Some(Selection::default());
                }
                if i.modifiers.command && i.key_pressed(egui::Key::A) {
                    asked.select_all = true;
                }
            });
        }
        self.load_textures(ctx, engine, document.as_deref());
        let selection = self.prune(document.as_deref());
        self.bars(ctx, theme, texts, &document, &header, &mut asked);
        if let Some(ask) = asked.zoom {
            self.view = ask.apply(self.zoom);
        }
        self.rail(ctx, theme, texts);
        self.side_panel(ctx, theme, texts, &mut asked);
        self.inspector(
            ctx,
            theme,
            texts,
            document.as_deref(),
            &selection,
            &mut asked,
        );
        self.canvas(ctx, theme, texts, document.as_deref(), &mut asked);
        if asked.versions {
            self.versions.open_list();
        }
        if asked.save_version {
            self.versions.ask_name();
        }
        asked.version = self.versions.show(ctx, engine, theme, texts);
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
            if IconButton::new(Icon::Home, texts.t(&gafira_doc::msg!("nav.home")))
                .on(Surface::Topbar)
                .show(ui, theme)
                .clicked()
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
                ui.separator();
                if ui
                    .button(texts.t(&gafira_doc::msg!("settings.title")))
                    .clicked()
                {
                    asked.settings = true;
                    ui.close();
                }
            });
            let edit = RichText::new(texts.t(&gafira_doc::msg!("menu.edit")))
                .font(theme.font("ui-label"))
                .color(theme.c("topbar.fg.muted"));
            ui.menu_button(edit, |ui| {
                ui.set_min_width(200.0);
                let undo = egui::Button::new(texts.t(&gafira_doc::msg!("editor.undo")))
                    .shortcut_text(ui.ctx().format_shortcut(&UNDO));
                if ui.add_enabled(header.can_undo, undo).clicked() {
                    asked.undo = true;
                    ui.close();
                }
                let redo = egui::Button::new(texts.t(&gafira_doc::msg!("editor.redo")))
                    .shortcut_text(ui.ctx().format_shortcut(&REDO));
                if ui.add_enabled(header.can_redo, redo).clicked() {
                    asked.redo = true;
                    ui.close();
                }
                ui.separator();
                let delete = egui::Button::new(texts.t(&gafira_doc::msg!("editor.delete")));
                if ui.add_enabled(!self.selected.is_empty(), delete).clicked() {
                    asked.delete = true;
                    ui.close();
                }
                let all = egui::Button::new(texts.t(&gafira_doc::msg!("editor.select_all")))
                    .shortcut_text(ui.ctx().format_shortcut(&SELECT_ALL));
                if ui.add_enabled(document.is_some(), all).clicked() {
                    asked.select_all = true;
                    ui.close();
                }
            });
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
            for (icon, key, shortcut, enabled, flag) in [
                (
                    Icon::Undo,
                    gafira_doc::msg!("editor.undo"),
                    UNDO,
                    header.can_undo,
                    &mut asked.undo,
                ),
                (
                    Icon::Redo,
                    gafira_doc::msg!("editor.redo"),
                    REDO,
                    header.can_redo,
                    &mut asked.redo,
                ),
            ] {
                let name = texts.t(&key);
                let tooltip = with_keys(ui.ctx(), texts, name.clone(), shortcut);
                let button = ui.add_enabled_ui(enabled, |ui| {
                    IconButton::new(icon, name)
                        .on(Surface::Topbar)
                        .tooltip(tooltip)
                        .show(ui, theme)
                });
                *flag |= button.inner.clicked();
            }
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
                asked.export |= widgets::primary_icon(
                    ui,
                    theme,
                    Icon::Export,
                    texts.t(&gafira_doc::msg!("editor.export")),
                )
                .clicked();
                ui.add_space(8.0);
                asked.versions |= widgets::on_topbar_icon(
                    ui,
                    theme,
                    Icon::Versions,
                    texts.t(&gafira_doc::msg!("editor.versions")),
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
                        asked.zoom = self.zoom_controls(ui, theme, texts).or(asked.zoom);
                    });
                });
            });
    }

    /// Alejar, Acercar and the zoom menu (Ajustar, 50, 100, 200 %) of the
    /// bottom bar (03 §7.3), laid out right to left.
    fn zoom_controls(&self, ui: &mut egui::Ui, theme: &Theme, texts: &Texts) -> Option<ZoomAsk> {
        let mut ask = None;
        let percent = |scale: f32| {
            texts.t(&gafira_doc::msg!(
                "format.percent",
                value = Text(format!("{:.0}", scale * 100.0))
            ))
        };
        let caret = icons::image(
            ui.ctx(),
            Icon::Expand,
            Weight::Regular,
            px(theme.tokens.size.icon.sm),
            theme.c("text"),
        );
        let label = RichText::new(percent(self.zoom)).font(theme.font("ui-label"));
        let menu = ui.menu_button((label, caret), |ui| {
            ui.set_min_width(120.0);
            let fit = ui.selectable_label(
                self.view == Zoom::Fit,
                texts.t(&gafira_doc::msg!("zoom.fit")),
            );
            if fit
                .on_hover_text(with_keys(
                    ui.ctx(),
                    texts,
                    texts.t(&gafira_doc::msg!("zoom.fit")),
                    ZOOM_FIT,
                ))
                .clicked()
            {
                ask = Some(ZoomAsk::Fit);
                ui.close();
            }
            for scale in zoom::PRESETS {
                if ui
                    .selectable_label(self.view == Zoom::Fixed(scale), percent(scale))
                    .clicked()
                {
                    ask = Some(ZoomAsk::To(scale));
                    ui.close();
                }
            }
        });
        // Named «Zoom», with the scale on display as its value.
        let name = texts.t(&gafira_doc::msg!("zoom.menu"));
        let value = percent(self.zoom);
        menu.response.widget_info(|| egui::WidgetInfo {
            current_text_value: Some(value.clone()),
            ..egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, &name)
        });
        for (icon, key, shortcut, step) in [
            (
                Icon::ZoomIn,
                gafira_doc::msg!("zoom.in"),
                ZOOM_IN,
                ZoomAsk::In,
            ),
            (
                Icon::ZoomOut,
                gafira_doc::msg!("zoom.out"),
                ZOOM_OUT,
                ZoomAsk::Out,
            ),
        ] {
            let name = texts.t(&key);
            let tooltip = with_keys(ui.ctx(), texts, name.clone(), shortcut);
            let limit = match step {
                ZoomAsk::In => self.zoom < zoom::MAX,
                _ => self.zoom > zoom::MIN,
            };
            let button = ui.add_enabled_ui(limit, |ui| {
                IconButton::new(icon, name)
                    .tooltip(tooltip)
                    .small()
                    .show(ui, theme)
            });
            if button.inner.clicked() {
                ask = Some(step);
            }
        }
        ask
    }

    fn rail(&mut self, ctx: &egui::Context, theme: &Theme, texts: &Texts) {
        egui::SidePanel::left("editor.rail")
            .exact_width(px(theme.tokens.size.rail))
            .resizable(false)
            .frame(egui::Frame::NONE.fill(theme.c("topbar.bg")))
            .show(ctx, |ui| {
                ui.add_space(8.0);
                for (index, (key, icon)) in TABS.iter().enumerate() {
                    let label = texts.t(&gafira_doc::msg::Message {
                        key: gafira_doc::msg::MessageKey(key),
                        args: Vec::new(),
                    });
                    let response = widgets::rail_tab(ui, theme, *icon, &label, index == self.tab);
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
                if TABS[self.tab].0 == "tab.library" {
                    let import = widgets::secondary(
                        ui,
                        theme,
                        texts.t(&gafira_doc::msg!("library.import_image")),
                    );
                    if import.clicked() {
                        asked.import_image = true;
                    }
                    return;
                }
                if TABS[self.tab].0 == "tab.text" {
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
                if TABS[self.tab].0 == "tab.pieces" {
                    ui.label(
                        RichText::new(texts.t(&gafira_doc::msg!("pieces.shapes")))
                            .font(theme.font("ui-label"))
                            .color(theme.c("text")),
                    );
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        for (key, icon, geometry) in [
                            (
                                gafira_doc::msg!("pieces.rect"),
                                Icon::Rect,
                                ShapeGeometry::Rect,
                            ),
                            (
                                gafira_doc::msg!("pieces.ellipse"),
                                Icon::Ellipse,
                                ShapeGeometry::Ellipse,
                            ),
                        ] {
                            if widgets::secondary_icon(ui, theme, icon, texts.t(&key)).clicked() {
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
        selection: &[Shape],
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
                let shape = match selection {
                    [] => {
                        ui.label(
                            RichText::new(texts.t(&gafira_doc::msg!("inspector.empty")))
                                .font(theme.font("body-sm"))
                                .color(theme.c("text.muted")),
                        );
                        return;
                    }
                    [shape] => shape,
                    many => {
                        many_fields(ui, theme, texts, many, asked);
                        return;
                    }
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
                    fill_field(ui, texts, shape, asked);
                }
                if shape.text.is_none() && shape.image.is_none() {
                    ui.add_space(8.0);
                    outline_fields(ui, texts, shape, asked);
                }
                ui.add_space(8.0);
                opacity_field(ui, texts, shape, asked);
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
        let Some(page) = self.current_page(document) else {
            return;
        };
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
            let selected = self.selected.contains(page.id, row.node);
            let label = row.name.clone().unwrap_or_else(|| {
                texts.t(&gafira_doc::msg::Message {
                    key: gafira_doc::msg::MessageKey(row.kind),
                    args: Vec::new(),
                })
            });
            ui.horizontal(|ui| {
                if ui.selectable_label(selected, label).clicked() {
                    // Ctrl/Cmd or Shift + click toggles the row (spec 040).
                    let toggle = ui.input(|i| i.modifiers.command || i.modifiers.shift);
                    asked.select = Some(if toggle {
                        let mut next = self.selected.clone();
                        next.toggle(page.id, row.node);
                        next
                    } else {
                        Selection::one(page.id, row.node)
                    });
                }
                // Subir, Bajar and the row's Eliminar act on one element.
                if selected && self.selected.len() == 1 {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let delete = IconButton::new(
                            Icon::Delete,
                            texts.t(&gafira_doc::msg!("editor.delete")),
                        )
                        .small()
                        .show(ui, theme);
                        asked.delete |= delete.clicked();
                        let down = ui.add_enabled_ui(row.index > 0, |ui| {
                            IconButton::new(
                                Icon::SendBackward,
                                texts.t(&gafira_doc::msg!("layers.down")),
                            )
                            .small()
                            .show(ui, theme)
                        });
                        if down.inner.clicked() {
                            asked.reorder = Some((page.id, row.node, row.index - 1));
                        }
                        let up = ui.add_enabled_ui(row.index < last, |ui| {
                            IconButton::new(
                                Icon::BringForward,
                                texts.t(&gafira_doc::msg!("layers.up")),
                            )
                            .small()
                            .show(ui, theme)
                        });
                        if up.inner.clicked() {
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
                    self.view = Zoom::Fixed(zoom::wheel(self.zoom, scroll));
                }
                let sizes = document.map(page_sizes).unwrap_or_default();
                let Some(first) = sizes.first().copied() else {
                    return;
                };
                self.zoom = self.view.resolve(fit_zoom(
                    (first.x, first.y),
                    (area.width(), area.height()),
                    CANVAS_MARGIN,
                ));
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
                        // A drag starts only once the pointer has moved a few
                        // px; what it grabbed is decided where it was pressed.
                        let pointer = if response.drag_started() {
                            ui.input(|input| input.pointer.press_origin())
                                .or_else(|| response.interact_pointer_pos())
                        } else {
                            response.interact_pointer_pos()
                        };
                        let pressed = response.clicked() || response.drag_started();
                        let (shift, press) =
                            ui.input(|i| (i.modifiers.shift, i.pointer.press_origin()));
                        let single = self.selected.single();
                        let mut picked: Option<(PageId, NodeId)> = None;
                        let mut grabbed: Option<canvas::Handle> = None;
                        // Each page's id, screen box and shapes, for the marquee.
                        let mut laid: Vec<(PageId, Rect, Vec<Shape>)> = Vec::new();
                        let selection = theme.c("canvas.selection");
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
                                    if let Some(asset) = shape.image {
                                        let tint = Color32::WHITE
                                            .gamma_multiply(shape.opacity.clamp(0.0, 1.0));
                                        match self
                                            .textures
                                            .get(&asset.as_hash())
                                            .and_then(Option::as_ref)
                                        {
                                            Some(texture) => {
                                                let uv = Rect::from_min_max(
                                                    Pos2::ZERO,
                                                    Pos2::new(1.0, 1.0),
                                                );
                                                painter.image(texture.id(), at, uv, tint);
                                            }
                                            None => {
                                                painter.rect_filled(
                                                    at,
                                                    0.0,
                                                    theme.c("surface.sunken"),
                                                );
                                            }
                                        }
                                    }
                                    if let Some(text) = &shape.text {
                                        draw_text(painter, theme, at, zoom, shape.opacity, text);
                                    }
                                    painter.add(canvas::painted(shape, at, zoom));
                                    if !self.selected.contains(shape.page, shape.node) {
                                        continue;
                                    }
                                    painter.rect_stroke(
                                        at.expand(1.0),
                                        0.0,
                                        Stroke::new(2.0, selection),
                                        egui::StrokeKind::Outside,
                                    );
                                    // Only a single selection resizes.
                                    if single == Some((shape.page, shape.node)) {
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
                                            grabbed = Some(handle);
                                        }
                                    }
                                }
                                if self.selected.len() > 1
                                    && self.selected.page() == Some(page_doc.id)
                                {
                                    // The box around a multi-selection (spec 040).
                                    let boxes = shapes
                                        .iter()
                                        .filter(|s| self.selected.contains(s.page, s.node))
                                        .map(|s| canvas::on_screen(page, zoom, &s.placement));
                                    if let Some(all) = canvas::bounds(boxes) {
                                        painter.rect_stroke(
                                            all.expand(4.0),
                                            0.0,
                                            Stroke::new(1.0, selection),
                                            egui::StrokeKind::Outside,
                                        );
                                    }
                                }
                                if pressed
                                    && let Some(at) = pointer
                                    && page.contains(at)
                                    && let Some(node) = canvas::hit(&shapes, page, zoom, at)
                                {
                                    picked = Some((page_doc.id, node));
                                }
                                laid.push((page_doc.id, page, shapes));
                            }
                            top = page.max.y + PAGE_GAP;
                        }
                        if let Some(handle) = grabbed {
                            // A handle of the selection wins over whatever
                            // lies under it.
                            asked.drag_begin = Some(Some(handle));
                        } else if response.clicked() {
                            // Click selects one; Shift+click toggles it.
                            let mut next = self.selected.clone();
                            match (picked, shift) {
                                (Some((page, node)), true) => next.toggle(page, node),
                                (Some((page, node)), false) => next = Selection::one(page, node),
                                (None, true) => {}
                                (None, false) => next.clear(),
                            }
                            asked.select = Some(next);
                        } else if response.drag_started() {
                            match picked {
                                // A drag on a selected element moves the
                                // whole selection; on another one it selects
                                // it first (Shift adds it).
                                Some((page, node)) => {
                                    if !self.selected.contains(page, node) {
                                        let mut next = self.selected.clone();
                                        if shift {
                                            next.add(page, [node]);
                                        } else {
                                            next = Selection::one(page, node);
                                        }
                                        asked.select = Some(next);
                                    }
                                    asked.drag_begin = Some(None);
                                }
                                // On empty canvas it draws a marquee.
                                None => {
                                    if let Some(from) = press.or(pointer) {
                                        self.marquee = Some(Marquee {
                                            from,
                                            to: pointer.unwrap_or(from),
                                            add: shift,
                                        });
                                    }
                                }
                            }
                        }
                        if response.dragged() {
                            if let Some(marquee) = self.marquee.as_mut() {
                                if let Some(to) = pointer {
                                    marquee.to = to;
                                }
                            } else if self.drag.is_some() {
                                asked.drag_to = Some(response.drag_delta());
                            }
                        }
                        if let Some(marquee) = self.marquee {
                            painter.rect(
                                Rect::from_two_pos(marquee.from, marquee.to),
                                0.0,
                                theme.c("selection.bg").gamma_multiply(0.5),
                                Stroke::new(1.0, selection),
                                egui::StrokeKind::Inside,
                            );
                        }
                        if response.drag_stopped() {
                            match self.marquee.take() {
                                Some(marquee) => {
                                    asked.select = Some(marquee_selection(
                                        &self.selected,
                                        marquee,
                                        &laid,
                                        zoom,
                                    ));
                                }
                                None => asked.drag_end = true,
                            }
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
        if let Some(select) = &asked.select {
            self.selected = select.clone();
        }
        if asked.select_all
            && let Some(document) = engine.edit().document()
            && let Some(page) = self.current_page(&document)
        {
            self.selected = Selection::of(page.id, canvas::roots(page));
        }
        if let Some(geometry) = asked.insert
            && let Some(document) = document
            && let Some(page) = self.current_page(document)
            && let Ok(size) = page.size.to_px(document.dpi())
        {
            let px = Vec2::new(size.width() as f32, size.height() as f32);
            let spec = canvas::new_piece(px, geometry, canvas::document_color(theme.c("accent")));
            let command = InsertNodes::new(page.id, None, page.roots.len(), vec![spec]);
            let next = NodeId::new(document.next_id());
            match engine.edit().execute(Box::new(command)) {
                Ok(_) => self.selected = Selection::one(page.id, next),
                Err(error) => eprintln!("insert: {error}"),
            }
        }
        if asked.import_image
            && let Some(document) = document
        {
            self.import_image(engine, document);
        }
        if let Some((preset, words)) = &asked.insert_text
            && let Some(document) = document
            && let Some(page) = self.current_page(document)
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
                Ok(_) => self.selected = Selection::one(page.id, next),
                Err(error) => eprintln!("insert text: {error}"),
            }
        }
        if let (Some(text), Some((page, node))) = (&asked.text, self.selected.single()) {
            let command =
                SetProperty::new(page, vec![node], Property::TextContent(text.clone()), None);
            log("text", engine.edit().execute(Box::new(command)).map(drop));
        }
        if let Some(handle) = asked.drag_begin
            && let Some(page) = self.selected.page()
        {
            // Where every selected element starts, from the newest document.
            let now = engine.edit().document();
            let origins = selected_shapes(now.as_deref(), &self.selected)
                .into_iter()
                .map(|shape| (shape.node, shape.placement))
                .collect::<Vec<_>>();
            let gesture = engine.edit().begin_gesture().ok();
            self.drag = Some(Drag {
                gesture,
                page,
                handle: handle.filter(|_| origins.len() == 1),
                origins,
                offset: Vec2::ZERO,
            });
        }
        if let (Some(delta), Some(drag)) = (asked.drag_to, self.drag.as_mut()) {
            drag.offset += delta;
            let zoom = self.zoom;
            // One placement per element; several are one Transaction, so the
            // drag stays one undo step (spec 040).
            let commands = drag
                .origins
                .iter()
                .map(|(node, origin)| {
                    let to = match drag.handle {
                        Some(handle) => canvas::resized(origin, handle, drag.offset, zoom),
                        None => canvas::moved(origin, drag.offset, zoom),
                    };
                    let command = SetProperty::new(
                        drag.page,
                        vec![*node],
                        Property::Placement(to),
                        drag.gesture,
                    );
                    Box::new(command) as Box<dyn Command>
                })
                .collect();
            if let Some(command) = batch(commands) {
                log("move", engine.edit().execute(command).map(drop));
            }
        }
        if asked.drag_end && self.drag.take().is_some() {
            log("end gesture", engine.edit().end_gesture());
        }
        if let (Some(placement), Some((page, node))) = (asked.place, self.selected.single()) {
            let command = SetProperty::new(page, vec![node], Property::Placement(placement), None);
            log("place", engine.edit().execute(Box::new(command)).map(drop));
        }
        // Fill, outline, radius and opacity go to every selected element in
        // one SetProperty, so a change is one undo step (spec 040).
        let all = self.selected.nodes().to_vec();
        if let Some(page) = self.selected.page() {
            let values = [
                ("fill", asked.fill.map(Property::Fill)),
                ("outline", asked.stroke.map(Property::Stroke)),
                (
                    "corner radius",
                    asked.corner_radius.map(Property::CornerRadius),
                ),
                ("opacity", asked.opacity.map(Property::Opacity)),
            ];
            for (what, value) in values {
                if let Some(value) = value {
                    let command = SetProperty::new(page, all.clone(), value, None);
                    log(what, engine.edit().execute(Box::new(command)).map(drop));
                }
            }
        }
        if let Some((page, node, index)) = asked.reorder {
            let command = gafira_edit::Reorder::new(page, node, index);
            log(
                "reorder",
                engine.edit().execute(Box::new(command)).map(drop),
            );
        }
        // Every selected element goes in one RemoveNodes: one undo step.
        if asked.delete
            && let Some(page) = self.selected.page()
        {
            self.selected.clear();
            log(
                "delete",
                engine
                    .edit()
                    .execute(Box::new(RemoveNodes::new(page, all)))
                    .map(drop),
            );
        }
        if let Some(dark) = asked.dark {
            Go::Dark(dark)
        } else if asked.settings {
            Go::Settings
        } else if let Some(name) = &asked.version {
            Go::SaveVersion(name.clone())
        } else if asked.export {
            Go::Export(self.selected.page())
        } else if asked.home {
            Go::Home
        } else if let Some(action) = asked.file {
            Go::File(action)
        } else {
            Go::Stay
        }
    }
}

impl Editor {
    /// Decode, once, the images the document shows into textures (spec
    /// 029), reading the bytes from the content store; large images are
    /// scaled down to 2048 px for the canvas.
    fn load_textures(&mut self, ctx: &egui::Context, engine: &Gafira, document: Option<&Document>) {
        let Some(document) = document else { return };
        for page in document.pages() {
            for shape in canvas::shapes(page) {
                let Some(asset) = shape.image else { continue };
                let hash = asset.as_hash();
                if self.textures.contains_key(&hash) {
                    continue;
                }
                let texture = decode(engine, hash).map(|image| {
                    ctx.load_texture(
                        format!("asset-{}", asset.hex()),
                        image,
                        egui::TextureOptions::LINEAR,
                    )
                });
                self.textures.insert(hash, texture);
            }
        }
    }

    /// Biblioteca → Importar imagen…: pick a file, import it through
    /// `AssetService` (embedded, deduplicated by hash) and place it on the
    /// selected element's page, or the first one.
    fn import_image(&mut self, engine: &mut Gafira, document: &Document) {
        let picked = rfd::FileDialog::new()
            .add_filter("Imagen", &["png", "jpg", "jpeg"])
            .pick_file();
        let Some(path) = picked else { return };
        let Ok(pixels) = image::image_dimensions(&path) else {
            eprintln!("import image: {} is not a readable image", path.display());
            return;
        };
        let Some(page) = self.current_page(document) else {
            return;
        };
        let Ok(size) = page.size.to_px(document.dpi()) else {
            return;
        };
        let (page_id, index) = (page.id, page.roots.len());
        match engine.assets().import(&path) {
            Ok(asset) => {
                let px = Vec2::new(size.width() as f32, size.height() as f32);
                let spec = canvas::new_image(px, pixels, asset);
                let next = engine.edit().document().map(|d| NodeId::new(d.next_id()));
                match engine.edit().execute(Box::new(InsertNodes::new(
                    page_id,
                    None,
                    index,
                    vec![spec],
                ))) {
                    Ok(_) => {
                        if let Some(node) = next {
                            self.selected = Selection::one(page_id, node);
                        }
                    }
                    Err(error) => eprintln!("insert image: {error}"),
                }
            }
            Err(error) => eprintln!("import image: {error}"),
        }
    }
}

/// The bytes of `hash` from the store, decoded and scaled for the canvas.
fn decode(engine: &Gafira, hash: gafira_doc::ContentHash) -> Option<egui::ColorImage> {
    let mut bytes = Vec::new();
    let mut source = engine.ports().store.open(hash).ok()?;
    std::io::Read::read_to_end(&mut source, &mut bytes).ok()?;
    let mut decoded = image::load_from_memory(&bytes).ok()?;
    if decoded.width() > 2048 || decoded.height() > 2048 {
        decoded = decoded.thumbnail(2048, 2048);
    }
    let rgba = decoded.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(
        size,
        rgba.as_raw(),
    ))
}

/// The Inspector fields of a shape's outline (colour, «Sin contorno»,
/// width) and, for a rectangle, its corner radius (spec 038). Each change
/// is one command, so one undo step.
/// «Relleno»: a colour picker showing `shape`'s fill and «Sin relleno».
fn fill_field(ui: &mut egui::Ui, texts: &Texts, shape: &Shape, asked: &mut Asked) {
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

/// «Opacidad»: a slider showing `shape`'s opacity in percent.
fn opacity_field(ui: &mut egui::Ui, texts: &Texts, shape: &Shape, asked: &mut Asked) {
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
}

/// The Inspector of a multi-selection (spec 040): how many elements, then
/// the fields they all have, showing the primary's value and set on all of
/// them in one step: the fill when every one is a shape, and the opacity.
/// Position, size, outline, radius and text stay with a single selection.
fn many_fields(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    shapes: &[Shape],
    asked: &mut Asked,
) {
    let Some(primary) = shapes.last() else { return };
    ui.label(
        RichText::new(texts.t(&gafira_doc::msg!(
            "inspector.count",
            count = Count(shapes.len() as u64)
        )))
        .font(theme.font("ui-label"))
        .color(theme.c("text")),
    );
    ui.add_space(16.0);
    if shapes
        .iter()
        .all(|shape| shape.text.is_none() && shape.image.is_none())
    {
        fill_field(ui, texts, primary, asked);
        ui.add_space(8.0);
    }
    opacity_field(ui, texts, primary, asked);
}

fn outline_fields(ui: &mut egui::Ui, texts: &Texts, shape: &Shape, asked: &mut Asked) {
    let px = |value: f64| {
        texts.t(&gafira_doc::msg!(
            "unit.px",
            value = Text(format!("{value:.1}"))
        ))
    };
    ui.horizontal(|ui| {
        ui.label(texts.t(&gafira_doc::msg!("inspector.outline")));
        let mut color = shape.stroke.map_or(Color32::TRANSPARENT, |stroke| {
            canvas::screen_color(stroke.color)
        });
        if ui.color_edit_button_srgba(&mut color).changed() {
            let width = shape
                .stroke
                .map_or(canvas::NEW_OUTLINE_WIDTH, |stroke| stroke.width);
            asked.stroke = Some(Some(gafira_doc::ShapeStroke::new(
                canvas::document_color(color),
                width,
            )));
        }
        if shape.stroke.is_some()
            && ui
                .small_button(texts.t(&gafira_doc::msg!("inspector.no_outline")))
                .clicked()
        {
            asked.stroke = Some(None);
        }
    });
    if let Some(stroke) = shape.stroke {
        ui.horizontal(|ui| {
            ui.label(texts.t(&gafira_doc::msg!("inspector.outline_width")));
            let mut width = stroke.width;
            let field = egui::DragValue::new(&mut width)
                .speed(0.5)
                .range(0.0..=gafira_doc::ShapeStroke::MAX_WIDTH)
                .custom_formatter(|value, _| px(value));
            if ui.add(field).changed() {
                asked.stroke = Some(Some(gafira_doc::ShapeStroke::new(stroke.color, width)));
            }
        });
    }
    if shape.geometry == ShapeGeometry::Rect {
        ui.horizontal(|ui| {
            ui.label(texts.t(&gafira_doc::msg!("inspector.corner_radius")));
            let half = shape.placement.w.min(shape.placement.h).max(0.0) / 2.0;
            let mut radius = shape.corner_radius.min(half);
            let field = egui::DragValue::new(&mut radius)
                .speed(0.5)
                .range(0.0..=half)
                .custom_formatter(|value, _| px(value));
            if ui.add(field).changed() {
                asked.corner_radius = Some(radius);
            }
        });
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
        EngineConfig, EventChannel, ExportPorts, Gafira, NoopWaker, Ports, SettingsPorts,
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
        engine_with(repository, Arc::new(Nothing))
    }

    /// [`engine_over`] with `store` as the content store.
    pub(crate) fn engine_with(
        repository: Arc<dyn ProjectRepository>,
        store: Arc<dyn ContentStore>,
    ) -> Gafira {
        let dir = std::env::temp_dir().join(format!("gafira-ui-{}", std::process::id()));
        let clock = Arc::new(SystemClock::new());
        let ports = Ports::new(
            repository,
            store,
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
            ExportPorts::new(
                crate::export::tests::pages(),
                Arc::new(gafira_io_image::RasterDecoder::new()),
            ),
        );
        Gafira::new(
            ports,
            crate::export::tests::registries(),
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
            Selection::one(shapes[0].page, shapes[0].node),
            "the new piece is selected"
        );
        let origin = shapes[0].placement;

        // A drag of three moves is one gesture: one undo takes it back.
        editor.zoom = 2.0;
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                drag_begin: Some(None),
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
        assert_eq!(editor.selected, Selection::default());
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
                drag_begin: Some(Some(corner)),
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

    // S38-08: the outline and the corner radius change through the engine,
    // one undo step each.
    #[test]
    fn s38_08_outline_and_corner_radius_change_through_the_engine_and_undo() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let theme = theme();
        let mut editor = Editor::default();
        let snapshot = engine.edit().document();
        let ask = |editor: &mut Editor, engine: &mut Gafira, asked: Asked| {
            editor.apply(engine, &snapshot, &asked, &theme);
        };
        ask(
            &mut editor,
            &mut engine,
            Asked {
                insert: Some(ShapeGeometry::Rect),
                ..Asked::default()
            },
        );
        let blue = canvas::document_color(Color32::from_rgb(0, 0, 255));
        let outline = Some(gafira_doc::ShapeStroke::new(blue, 3.0));
        ask(
            &mut editor,
            &mut engine,
            Asked {
                stroke: Some(outline),
                ..Asked::default()
            },
        );
        assert_eq!(the_shapes(&mut engine)[0].stroke, outline);
        ask(
            &mut editor,
            &mut engine,
            Asked {
                corner_radius: Some(12.0),
                ..Asked::default()
            },
        );
        assert_eq!(the_shapes(&mut engine)[0].corner_radius, 12.0);
        ask(
            &mut editor,
            &mut engine,
            Asked {
                stroke: Some(None),
                ..Asked::default()
            },
        );
        assert_eq!(the_shapes(&mut engine)[0].stroke, None);
        engine.edit().undo().expect("an undo");
        assert_eq!(the_shapes(&mut engine)[0].stroke, outline);
        engine.edit().undo().expect("an undo");
        assert_eq!(the_shapes(&mut engine)[0].corner_radius, 0.0);
        assert_eq!(the_shapes(&mut engine)[0].stroke, outline);
        engine.edit().undo().expect("an undo");
        assert_eq!(the_shapes(&mut engine)[0].stroke, None);
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
        assert!(!editor.selected.is_empty(), "the new text is selected");

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

    // S29-03
    #[test]
    fn s29_03_an_imported_image_is_placed_drawn_from_the_store_and_undone() {
        let dir = std::env::temp_dir().join(format!("gafira-ui-image-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let png = dir.join("dot.png");
        image::RgbaImage::from_pixel(40, 20, image::Rgba([255, 0, 0, 255]))
            .save(&png)
            .expect("a png");
        let store = Arc::new(gafira_assets::FsStore::new(dir.join("store")));
        let mut engine = engine_with(Arc::new(Nothing), store);
        start(&mut engine, StartFormat::Social).expect("a new document");

        let asset = engine.assets().import(&png).expect("imports");
        let page = engine.edit().document().expect("a document").pages()[0].id;
        let spec = canvas::new_image(Vec2::new(1080.0, 1080.0), (40, 20), asset);
        engine
            .edit()
            .execute(Box::new(InsertNodes::new(page, None, 0, vec![spec])))
            .expect("inserts");
        let shapes = the_shapes(&mut engine);
        assert_eq!(shapes[0].image, Some(asset));
        assert_eq!((shapes[0].placement.w, shapes[0].placement.h), (40.0, 20.0));

        let ctx = egui::Context::default();
        let mut editor = Editor::default();
        let document = engine.edit().document();
        editor.load_textures(&ctx, &engine, document.as_deref());
        let texture = editor
            .textures
            .get(&asset.as_hash())
            .and_then(Option::as_ref)
            .expect("decoded");
        assert_eq!(texture.size(), [40, 20]);

        engine.edit().undo().expect("an undo");
        assert!(the_shapes(&mut engine).is_empty());
        std::fs::remove_dir_all(dir).expect("clean up");
    }

    /// One frame of the whole editor in a 1440 × 900 window, with Ctrl/Cmd
    /// and `key` pressed when given.
    fn frame(ctx: &egui::Context, editor: &mut Editor, engine: &mut Gafira, key: Option<Key>) {
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0))),
            ..Default::default()
        };
        if let Some(key) = key {
            input.modifiers = Modifiers::COMMAND;
            input.events.push(egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::COMMAND,
            });
        }
        let (theme, texts) = (theme(), Texts::new(gafira_ports::i18n::Locale::Es));
        let _ = ctx.run(input, |ctx| {
            editor.show(ctx, engine, &theme, &texts, None);
        });
    }

    // S37-07: the shortcuts of 03 §9.2 move the canvas zoom.
    #[test]
    fn s37_07_ctrl_1_ctrl_equals_ctrl_minus_and_ctrl_0_drive_the_canvas_zoom() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let ctx = egui::Context::default();
        let mut editor = Editor::default();
        frame(&ctx, &mut editor, &mut engine, None);
        assert_eq!(editor.view, Zoom::Fit);
        let fit = editor.zoom;
        assert!(
            fit > 0.05 && fit < 1.0,
            "a 1080 px page fits below 100 %: {fit}"
        );

        frame(&ctx, &mut editor, &mut engine, Some(Key::Num1));
        assert_eq!(editor.view, Zoom::Fixed(1.0));
        assert_eq!(editor.zoom, 1.0);
        frame(&ctx, &mut editor, &mut engine, Some(Key::Equals));
        assert_eq!(editor.view, Zoom::Fixed(1.25));
        frame(&ctx, &mut editor, &mut engine, Some(Key::Minus));
        assert_eq!(editor.view, Zoom::Fixed(1.0));
        frame(&ctx, &mut editor, &mut engine, Some(Key::Num0));
        assert_eq!(editor.view, Zoom::Fit);
        assert_eq!(editor.zoom, fit);
    }

    /// A Social document with a rectangle and an ellipse, the ellipse moved
    /// 400 px right of the rectangle; `editor` has the ellipse selected.
    fn two_pieces(engine: &mut Gafira, editor: &mut Editor) -> (PageId, NodeId, NodeId) {
        start(engine, StartFormat::Social).expect("a new document");
        let theme = theme();
        for geometry in [ShapeGeometry::Rect, ShapeGeometry::Ellipse] {
            let snapshot = engine.edit().document();
            editor.apply(
                engine,
                &snapshot,
                &Asked {
                    insert: Some(geometry),
                    ..Asked::default()
                },
                &theme,
            );
        }
        let shapes = the_shapes(engine);
        let mut aside = shapes[1].placement;
        aside.x += 400.0;
        let snapshot = engine.edit().document();
        editor.apply(
            engine,
            &snapshot,
            &Asked {
                place: Some(aside),
                ..Asked::default()
            },
            &theme,
        );
        (shapes[0].page, shapes[0].node, shapes[1].node)
    }

    /// `asked` applied by `editor` over the newest document.
    fn ask(engine: &mut Gafira, editor: &mut Editor, asked: Asked) -> Go {
        let snapshot = engine.edit().document();
        editor.apply(engine, &snapshot, &asked, &theme())
    }

    /// How many entries the history can undo.
    fn undo_depth(engine: &mut Gafira) -> usize {
        let mut depth = 0;
        while engine.edit().can_undo_redo().0 {
            engine.edit().undo().expect("an undo");
            depth += 1;
        }
        for _ in 0..depth {
            engine.edit().redo().expect("a redo");
        }
        depth
    }

    // S40-05: a drag of two selected elements moves both in one undo step.
    #[test]
    fn s40_05_a_drag_of_a_multi_selection_moves_all_in_one_undo() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, ellipse) = two_pieces(&mut engine, &mut editor);
        let before = the_shapes(&mut engine);
        let depth = undo_depth(&mut engine);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                select: Some(Selection::of(page, [rect, ellipse])),
                ..Asked::default()
            },
        );
        editor.zoom = 2.0;
        // A handle held with two selected moves instead of resizing.
        let corner = canvas::Handle { x: 1, y: 1 };
        for asked in [
            Asked {
                drag_begin: Some(Some(corner)),
                ..Asked::default()
            },
            Asked {
                drag_to: Some(Vec2::new(10.0, 4.0)),
                ..Asked::default()
            },
            Asked {
                drag_to: Some(Vec2::new(10.0, 4.0)),
                ..Asked::default()
            },
            Asked {
                drag_to: Some(Vec2::new(10.0, 4.0)),
                ..Asked::default()
            },
            Asked {
                drag_end: true,
                ..Asked::default()
            },
        ] {
            ask(&mut engine, &mut editor, asked);
        }
        let after = the_shapes(&mut engine);
        for (moved, origin) in after.iter().zip(&before) {
            assert_eq!(
                (moved.placement.x - origin.placement.x),
                15.0,
                "{:?}",
                moved.node
            );
            assert_eq!(moved.placement.y - origin.placement.y, 6.0);
            assert_eq!(
                (moved.placement.w, moved.placement.h),
                (origin.placement.w, origin.placement.h),
                "moved, not resized"
            );
        }
        assert_eq!(undo_depth(&mut engine), depth + 1, "one entry for the drag");
        engine.edit().undo().expect("an undo");
        assert_eq!(the_shapes(&mut engine), before, "one undo takes both back");
    }

    // S40-06: Delete removes every selected element in one undo step.
    #[test]
    fn s40_06_delete_removes_the_whole_selection_in_one_undo() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, ellipse) = two_pieces(&mut engine, &mut editor);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                insert: Some(ShapeGeometry::Rect),
                ..Asked::default()
            },
        );
        let kept = the_shapes(&mut engine)[2].node;
        let depth = undo_depth(&mut engine);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                select: Some(Selection::of(page, [rect, ellipse])),
                ..Asked::default()
            },
        );
        ask(
            &mut engine,
            &mut editor,
            Asked {
                delete: true,
                ..Asked::default()
            },
        );
        let left: Vec<NodeId> = the_shapes(&mut engine).iter().map(|s| s.node).collect();
        assert_eq!(left, [kept]);
        assert!(editor.selected.is_empty());
        assert_eq!(undo_depth(&mut engine), depth + 1);
        engine.edit().undo().expect("an undo");
        assert_eq!(
            the_shapes(&mut engine).len(),
            3,
            "one undo brings both back"
        );
    }

    // S40-07: an undo that takes selected elements away prunes them.
    #[test]
    fn s40_07_an_undo_prunes_what_it_took_away() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, ellipse) = two_pieces(&mut engine, &mut editor);
        editor.selected = Selection::of(page, [rect, ellipse]);
        // Undo the move aside, then the ellipse's insert.
        engine.edit().undo().expect("an undo");
        engine.edit().undo().expect("an undo");
        let kept = editor.prune(engine.edit().document().as_deref());
        assert_eq!(editor.selected, Selection::one(page, rect));
        assert_eq!(kept.iter().map(|s| s.node).collect::<Vec<_>>(), [rect]);
        engine.edit().undo().expect("an undo");
        assert!(editor.prune(engine.edit().document().as_deref()).is_empty());
        assert_eq!(editor.selected, Selection::default());
    }

    // S40-08: fill and opacity from the Inspector go to every selected
    // element, one undo step each.
    #[test]
    fn s40_08_fill_and_opacity_apply_to_the_whole_selection_in_one_undo() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, ellipse) = two_pieces(&mut engine, &mut editor);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                select: Some(Selection::of(page, [rect, ellipse])),
                ..Asked::default()
            },
        );
        let depth = undo_depth(&mut engine);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                fill: Some(None),
                opacity: Some(0.5),
                ..Asked::default()
            },
        );
        for shape in the_shapes(&mut engine) {
            assert_eq!(shape.fill, None);
            assert_eq!(shape.opacity, 0.5);
        }
        assert_eq!(undo_depth(&mut engine), depth + 2, "fill, then opacity");
        engine.edit().undo().expect("an undo");
        for shape in the_shapes(&mut engine) {
            assert_eq!(shape.opacity, 1.0, "one undo restores both");
            assert!(shape.fill.is_none());
        }
        engine.edit().undo().expect("an undo");
        assert!(the_shapes(&mut engine).iter().all(|s| s.fill.is_some()));
    }

    // S40-02: a marquee takes the shapes on the page where it started (or
    // the first page it touches); Shift adds them; touching nothing clears
    // without Shift and keeps the selection with it.
    #[test]
    fn s40_02_a_marquee_selects_on_one_page_and_shift_adds() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, ellipse) = two_pieces(&mut engine, &mut editor);
        let shapes = the_shapes(&mut engine);
        let first = Rect::from_min_size(Pos2::ZERO, Vec2::splat(1080.0));
        let second = Rect::from_min_size(Pos2::new(0.0, 1200.0), Vec2::splat(1080.0));
        let laid = vec![
            (page, first, shapes.clone()),
            (PageId::new(999), second, Vec::new()),
        ];
        let rect_at = canvas::on_screen(first, 1.0, &shapes[0].placement);
        let marquee = |from: Pos2, to: Pos2, add: bool| Marquee { from, to, add };
        let current = Selection::one(page, ellipse);

        let inside = marquee(rect_at.min - Vec2::splat(5.0), rect_at.center(), false);
        assert_eq!(
            marquee_selection(&current, inside, &laid, 1.0),
            Selection::one(page, rect)
        );
        let added = Marquee {
            add: true,
            ..inside
        };
        assert_eq!(
            marquee_selection(&current, added, &laid, 1.0),
            Selection::of(page, [ellipse, rect])
        );
        // Started above the pages, it still selects on the page it touches.
        let from_outside = marquee(Pos2::new(rect_at.min.x, -40.0), rect_at.center(), false);
        assert_eq!(
            marquee_selection(&current, from_outside, &laid, 1.0),
            Selection::one(page, rect)
        );
        let empty = marquee(Pos2::new(5.0, 1210.0), Pos2::new(50.0, 1250.0), false);
        assert_eq!(
            marquee_selection(&current, empty, &laid, 1.0),
            Selection::default()
        );
        let empty_added = Marquee { add: true, ..empty };
        assert_eq!(
            marquee_selection(&current, empty_added, &laid, 1.0),
            current
        );
    }

    // S40-09: Ctrl+A selects the drawn roots of the page and Esc clears.
    #[test]
    fn s40_09_ctrl_a_selects_everything_and_escape_clears() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, ellipse) = two_pieces(&mut engine, &mut editor);
        let ctx = egui::Context::default();
        frame(&ctx, &mut editor, &mut engine, Some(Key::A));
        assert_eq!(editor.selected, Selection::of(page, [rect, ellipse]));
        frame(&ctx, &mut editor, &mut engine, None);
        assert_eq!(editor.selected.len(), 2, "kept across frames");
        frame(&ctx, &mut editor, &mut engine, Some(Key::Escape));
        assert_eq!(editor.selected, Selection::default());
    }
}
