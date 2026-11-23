//! Marca in the rail of the editor (spec 093): the brand templates, the
//! kit in use with its selector, and its logos, colours, text styles,
//! photos, graphics and icons. What a click does is told apart: a template
//! goes on the page, a file or a style is inserted, a colour is applied.

use std::collections::BTreeMap;
use std::path::PathBuf;

use egui::{Align, Color32, Layout, Rect, RichText, Sense, Stroke, Vec2};
use gafira_doc::msg::{Message, MessageKey};

use crate::brand::{BrandFiles, BrandKit, BrandStore};
use crate::brand_screen::{file_tile, rgb, span_style};
use crate::recent;
use crate::style::Theme;
use crate::text::Texts;

fn t(texts: &Texts, key: &'static str) -> String {
    texts.t(&Message {
        key: MessageKey(key),
        args: Vec::new(),
    })
}

/// What a click in the panel asks the editor for.
pub(crate) enum PanelAsk {
    /// Put this brand template on the page.
    Template(PathBuf),
    /// Insert this file in the middle of the page.
    File(PathBuf),
    /// Insert a text of this style, or give the selected texts this style.
    Text(gafira_doc::SpanStyle, String),
    /// A text of this style dropped at a point of the screen.
    DropText(gafira_doc::SpanStyle, String, egui::Pos2),
    /// A file dropped at a point of the screen.
    DropFile(PathBuf, egui::Pos2),
    /// Apply this colour (`0xRRGGBB`) to the selection, or to the page.
    Color(u32),
    /// Open Marca, on this kit if given.
    Manage(Option<String>),
}

/// What the panel keeps between frames: the kits, read again every few
/// seconds, and the thumbnails of the templates.
#[derive(Default)]
pub(crate) struct BrandPanel {
    kits: Vec<BrandKit>,
    active: Option<String>,
    read_at: Option<f64>,
    thumbs: BTreeMap<PathBuf, crate::brand_screen::Thumb>,
    /// «Guardar como plantilla de marca», while it is open: the kit and
    /// the name.
    pub(crate) saving: Option<(String, String)>,
}

impl BrandPanel {
    /// Read the kits again when they are older than two seconds.
    pub(crate) fn refresh(&mut self, store: &dyn BrandStore, now: f64) {
        if self.read_at.is_some_and(|at| now - at < 2.0) {
            return;
        }
        self.kits = store.kits();
        self.active = store.active();
        self.read_at = Some(now);
    }

    /// The kit in use: the one chosen, or the first.
    pub(crate) fn kit(&self) -> Option<&BrandKit> {
        self.active
            .as_ref()
            .and_then(|id| self.kits.iter().find(|k| &k.id == id))
            .or_else(|| self.kits.first())
    }

    pub(crate) fn kits(&self) -> &[BrandKit] {
        &self.kits
    }

    fn choose(&mut self, store: &dyn BrandStore, id: &str) {
        store.set_active(id);
        self.active = Some(id.to_owned());
    }
}

/// The colours of the kit in use, kept for the colour pickers of the frame.
pub(crate) fn set_colors(ctx: &egui::Context, colors: Vec<u32>) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("brand.colors"), colors));
}

/// The colours [`set_colors`] kept.
pub(crate) fn colors(ctx: &egui::Context) -> Vec<u32> {
    ctx.data(|d| d.get_temp::<Vec<u32>>(egui::Id::new("brand.colors")))
        .unwrap_or_default()
}

/// «Colores de la marca» under a colour picker: the colours of the kit in
/// use as swatches; the one clicked.
pub(crate) fn swatches(ui: &mut egui::Ui, texts: &Texts) -> Option<gafira_color::Color> {
    let colors = colors(ui.ctx());
    if colors.is_empty() {
        return None;
    }
    let mut picked = None;
    ui.add_space(4.0);
    ui.label(
        RichText::new(t(texts, "brand.colors_of_brand"))
            .small()
            .weak(),
    );
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(6.0);
        for c in colors {
            let (rect, response) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
            ui.painter().rect_filled(rect, 6.0, rgb(c));
            let border = ui.visuals().widgets.noninteractive.bg_stroke.color;
            ui.painter().rect_stroke(
                rect,
                6.0,
                Stroke::new(1.0, border),
                egui::StrokeKind::Inside,
            );
            if response
                .on_hover_text(format!("#{c:06X}"))
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                picked = Some(crate::colors::hex(c));
            }
        }
    });
    picked
}

/// The panel; what a click asked for, if anything.
pub(crate) fn show(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    panel: &mut BrandPanel,
    store: &dyn BrandStore,
) -> Option<PanelAsk> {
    panel.refresh(store, ui.input(|i| i.time));
    let mut ask = None;
    egui::ScrollArea::vertical()
        .id_salt("editor.brand")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let Some(kit) = panel.kit().cloned() else {
                empty(ui, theme, texts, &mut ask);
                return;
            };
            // The brand templates.
            let templates = store.templates(&kit.id);
            if header(
                ui,
                theme,
                &t(texts, "brand.templates"),
                Some(&t(texts, "home.see_all")),
            ) {
                ask = Some(PanelAsk::Manage(Some(kit.id.clone())));
            }
            if templates.is_empty() {
                note(ui, theme, &t(texts, "brand.none_templates"));
            } else {
                row(ui, |ui| {
                    for listing in &templates {
                        if template_tile(ui, theme, texts, panel, store, &listing.path) {
                            ask = Some(PanelAsk::Template(listing.path.clone()));
                        }
                    }
                });
            }
            ui.add_space(14.0);
            // The kit in use and its selector.
            ui.horizontal(|ui| {
                let kits: Vec<(String, String)> = panel
                    .kits()
                    .iter()
                    .map(|k| (k.id.clone(), k.name.clone()))
                    .collect();
                let mut chosen = None;
                egui::ComboBox::from_id_salt("editor.brand.kit")
                    .selected_text(RichText::new(&kit.name).font(theme.font("ui-label")))
                    .width(ui.available_width() - 90.0)
                    .show_ui(ui, |ui| {
                        for (id, name) in &kits {
                            if ui.selectable_label(*id == kit.id, name).clicked() {
                                chosen = Some(id.clone());
                            }
                        }
                    });
                if let Some(id) = chosen {
                    panel.choose(store, &id);
                }
                if ui.link(t(texts, "brand.manage")).clicked() {
                    ask = Some(PanelAsk::Manage(Some(kit.id.clone())));
                }
            });
            ui.add_space(14.0);
            // Logos.
            files(
                ui,
                theme,
                texts,
                &kit,
                BrandFiles::Logos,
                "brand.logos",
                &mut ask,
            );
            // Colours.
            header(ui, theme, &t(texts, "brand.colors"), None);
            if kit.palettes.iter().all(|p| p.colors.is_empty()) {
                note(ui, theme, &t(texts, "brand.none_colors"));
            }
            for palette in &kit.palettes {
                if palette.colors.is_empty() {
                    continue;
                }
                ui.label(
                    RichText::new(&palette.name)
                        .font(theme.font("caption"))
                        .color(theme.c("text.secondary")),
                );
                let width = ui.available_width();
                let (strip, _) = ui.allocate_exact_size(Vec2::new(width, 40.0), Sense::hover());
                let w = strip.width() / palette.colors.len() as f32;
                for (i, color) in palette.colors.iter().enumerate() {
                    let cell = Rect::from_min_size(
                        egui::pos2(strip.left() + w * i as f32, strip.top()),
                        Vec2::new(w, strip.height()),
                    );
                    let response = ui.interact(
                        cell,
                        ui.id().with(("brand.color", &palette.name, i)),
                        Sense::click(),
                    );
                    let mut corners = egui::CornerRadius::ZERO;
                    if i == 0 {
                        corners.nw = 8;
                        corners.sw = 8;
                    }
                    if i + 1 == palette.colors.len() {
                        corners.ne = 8;
                        corners.se = 8;
                    }
                    ui.painter().rect_filled(cell, corners, rgb(color.rgb));
                    if response.hovered() {
                        ui.painter().rect_stroke(
                            cell.shrink(1.0),
                            corners,
                            Stroke::new(2.0, theme.c("accent")),
                            egui::StrokeKind::Inside,
                        );
                    }
                    if response
                        .on_hover_text(t(texts, "brand.apply_color"))
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        ask = Some(PanelAsk::Color(color.rgb));
                    }
                }
                ui.add_space(8.0);
            }
            ui.add_space(14.0);
            // Text styles: a click adds a text of that style.
            header(ui, theme, &t(texts, "brand.fonts"), None);
            for style in &kit.styles {
                let span = span_style(style);
                let family = crate::editor::text_family(ui.ctx(), theme, &span);
                let size = (style.size as f32).clamp(13.0, 30.0);
                let mut text = RichText::new(&style.name)
                    .font(egui::FontId::new(size, family))
                    .color(theme.c("text"));
                if style.italic {
                    text = text.italics();
                }
                let button = egui::Button::new(text)
                    .fill(theme.c("surface.sunken"))
                    .corner_radius(8.0)
                    .min_size(Vec2::new(ui.available_width(), 44.0));
                let response = ui.add(button).interact(Sense::click_and_drag());
                if let Some(dropped) = dragged(ui, &response, &style.name, theme) {
                    ask = Some(PanelAsk::DropText(
                        span.clone(),
                        style.name.clone(),
                        dropped,
                    ));
                }
                if response.on_hover_text(t(texts, "brand.add_text")).clicked() {
                    ask = Some(PanelAsk::Text(span, style.name.clone()));
                }
                ui.add_space(6.0);
            }
            ui.add_space(14.0);
            files(
                ui,
                theme,
                texts,
                &kit,
                BrandFiles::Photos,
                "brand.photos",
                &mut ask,
            );
            files(
                ui,
                theme,
                texts,
                &kit,
                BrandFiles::Graphics,
                "brand.graphics",
                &mut ask,
            );
            files(
                ui,
                theme,
                texts,
                &kit,
                BrandFiles::Icons,
                "brand.icons",
                &mut ask,
            );
        });
    ask
}

/// No kit yet: a line and «Crear kit de marca».
fn empty(ui: &mut egui::Ui, theme: &Theme, texts: &Texts, ask: &mut Option<PanelAsk>) {
    ui.add_space(12.0);
    ui.label(
        RichText::new(t(texts, "brand.no_kits"))
            .font(theme.font("ui-label"))
            .color(theme.c("text")),
    );
    ui.add_space(4.0);
    ui.label(
        RichText::new(t(texts, "brand.no_kits_panel"))
            .font(theme.font("body-sm"))
            .color(theme.c("text.secondary")),
    );
    ui.add_space(12.0);
    if crate::widgets::primary(ui, theme, t(texts, "brand.create_title")).clicked() {
        *ask = Some(PanelAsk::Manage(None));
    }
}

/// A title with «Ver todo» on its right; `true` when that was clicked.
fn header(ui: &mut egui::Ui, theme: &Theme, title: &str, more: Option<&str>) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(title)
                .font(theme.font("ui-label"))
                .color(theme.c("text")),
        );
        if let Some(more) = more {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                clicked = ui
                    .link(RichText::new(more).font(theme.font("body-sm")))
                    .clicked();
            });
        }
    });
    ui.add_space(6.0);
    clicked
}

fn note(ui: &mut egui::Ui, theme: &Theme, text: &str) {
    egui::Frame::NONE
        .stroke(Stroke::new(1.0, theme.c("border")))
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(text)
                    .font(theme.font("body-sm"))
                    .color(theme.c("text.muted")),
            );
        });
    ui.add_space(8.0);
}

/// A row that scrolls sideways.
fn row(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::ScrollArea::horizontal()
        .id_salt(ui.next_auto_id())
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                add(ui);
            });
        });
}

/// A section of files: each one a tile; a click inserts it.
fn files(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    kit: &BrandKit,
    kind: BrandFiles,
    key: &'static str,
    ask: &mut Option<PanelAsk>,
) {
    header(ui, theme, &t(texts, key), None);
    if kit.files(kind).is_empty() {
        note(ui, theme, &t(texts, "brand.none_files"));
        ui.add_space(6.0);
        return;
    }
    row(ui, |ui| {
        for path in kit.files(kind) {
            let (response, _) = file_tile(ui, theme, path, 96.0);
            let response = response.interact(Sense::click_and_drag());
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if let Some(dropped) = dragged(ui, &response, &name, theme) {
                *ask = Some(PanelAsk::DropFile(path.clone(), dropped));
            }
            if response.on_hover_text(t(texts, "brand.insert")).clicked() {
                *ask = Some(PanelAsk::File(path.clone()));
            }
        }
    });
    ui.add_space(14.0);
}

/// One brand template: its thumbnail; `true` when clicked.
fn template_tile(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    panel: &mut BrandPanel,
    store: &dyn BrandStore,
    path: &std::path::Path,
) -> bool {
    let thumb = panel
        .thumbs
        .entry(path.to_path_buf())
        .or_insert_with(|| {
            store.thumbnail(path).map(|thumb| {
                let texture = ui.ctx().load_texture(
                    format!("brand-template:{}", path.display()),
                    egui::ColorImage::from_rgba_unmultiplied(
                        [thumb.width as usize, thumb.height as usize],
                        &thumb.rgba,
                    ),
                    egui::TextureOptions::LINEAR,
                );
                (texture, (thumb.width, thumb.height))
            })
        })
        .clone();
    let (tile, response) = ui.allocate_exact_size(Vec2::new(110.0, 110.0), Sense::click());
    ui.painter()
        .rect_filled(tile, 8.0, theme.c("surface.sunken"));
    if let Some((texture, (w, h))) = thumb {
        let (fw, fh) = recent::fit(w, h, (tile.width() - 8.0, tile.height() - 8.0));
        ui.painter().image(
            texture.id(),
            Rect::from_center_size(tile.center(), Vec2::new(fw, fh)),
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    if response.hovered() {
        ui.painter().rect_stroke(
            tile,
            8.0,
            Stroke::new(2.0, theme.c("accent")),
            egui::StrokeKind::Inside,
        );
    }
    response
        .on_hover_text(t(texts, "brand.use_on_page"))
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

/// While `response` is dragged, a closed hand and a ghost of `label` under
/// the pointer; where it was let go, once.
fn dragged(
    ui: &egui::Ui,
    response: &egui::Response,
    label: &str,
    theme: &Theme,
) -> Option<egui::Pos2> {
    let at = ui.input(|i| i.pointer.latest_pos());
    if response.hovered() && !response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if response.dragged()
        && let Some(at) = at
    {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        let painter = ui.ctx().layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("brand.ghost"),
        ));
        painter.text(
            at,
            egui::Align2::CENTER_CENTER,
            label,
            theme.font("ui-label"),
            theme.c("text").gamma_multiply(0.6),
        );
    }
    if response.drag_stopped() { at } else { None }
}
