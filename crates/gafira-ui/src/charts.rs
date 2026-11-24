//! Charts on the canvas and their editor (spec 083): their pixels, the
//! panel «Gráfico» with the data and the looks, the wide table of data, and
//! the controls of the bar.

use std::hash::{Hash, Hasher};

use egui::{Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};
use gafira_chart::{Colored, Column};
use gafira_color::Color;
use gafira_doc::{ChartCombine, ChartKind, ChartLabels, ChartNode};
use gafira_doc::{NodeId, PageId};

use crate::canvas;
use crate::chart_templates::{Group, PALETTES, TEMPLATES, dress, kind_key, palette};
use crate::style::Theme;
use crate::text::Texts;
use crate::widgets;

pub(crate) fn plain(texts: &Texts, key: &'static str) -> String {
    texts.t(&gafira_doc::msg::Message {
        key: gafira_doc::msg::MessageKey(key),
        args: Vec::new(),
    })
}

// ---- Pixels --------------------------------------------------------------

/// The longest side of the texture of a chart, in px.
const TEXTURE_MAX: f64 = 4096.0;

/// The key of the pixels of a chart in a box.
pub(crate) fn key(c: &ChartNode, rest: impl Hash) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    format!("{c:?}").hash(&mut hasher);
    rest.hash(&mut hasher);
    hasher.finish()
}

/// How far the words of a chart may go past its box, in px.
pub(crate) fn overhang(c: &ChartNode) -> f64 {
    c.text.size
}

/// The pixels of `c` in a box of `w` × `h` px at `zoom`, with its
/// [`overhang`] around: the same outlines as the export.
pub(crate) fn image(c: &ChartNode, (w, h): (f64, f64), zoom: f32) -> Option<egui::ColorImage> {
    if !(w > 0.0 && h > 0.0) {
        return None;
    }
    let margin = overhang(c);
    let (full_w, full_h) = (w + 2.0 * margin, h + 2.0 * margin);
    let scale = f64::from(zoom).min(TEXTURE_MAX / full_w.max(full_h));
    if !(scale.is_finite() && scale > 0.0) {
        return None;
    }
    // Within 1..=4096: the casts are exact.
    let (pw, ph) = (
        (full_w * scale).round().max(1.0) as u32,
        (full_h * scale).round().max(1.0) as u32,
    );
    let mut pixmap = tiny_skia::Pixmap::new(pw, ph)?;
    let at = kurbo::Affine::scale(scale) * kurbo::Affine::translate((margin, margin));
    for (path, color) in gafira_render::chart_outlines(c, (w, h), canvas::text_fonts()) {
        if let Some(path) = canvas::skia_path(&(at * path)) {
            crate::tables::fill(&mut pixmap, &path, color);
        }
    }
    Some(egui::ColorImage::from_rgba_premultiplied(
        [pw as usize, ph as usize],
        pixmap.data(),
    ))
}

// ---- The editor ----------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Tab {
    #[default]
    Data,
    Looks,
}

/// The panel «Gráfico» open on a chart.
pub(crate) struct Editing {
    pub page: PageId,
    pub node: NodeId,
    tab: Tab,
    wide: bool,
    /// The cell written last.
    cell: Option<(usize, usize)>,
    preview: Option<(u64, egui::TextureHandle)>,
}

/// What the editor asks for.
#[derive(Default)]
pub(crate) struct Asks {
    pub chart: Option<Box<ChartNode>>,
    /// «Importar datos».
    pub import: bool,
    pub close: bool,
    /// «Fuente».
    pub font: bool,
}

impl Asks {
    /// The chart to change: the one changed already, or `of`.
    fn edit(&mut self, of: &ChartNode, change: impl FnOnce(&mut ChartNode)) {
        let mut next = self.chart.take().unwrap_or_else(|| Box::new(of.clone()));
        change(&mut next);
        if *next != *of {
            self.chart = Some(next);
        }
    }
}

/// The height of a row of the data.
const ROW: f32 = 30.0;

/// The empty rows and columns after the data, to write in.
const SPARE: (usize, usize) = (12, 3);

impl Editing {
    pub fn new(page: PageId, node: NodeId) -> Self {
        Self {
            page,
            node,
            tab: Tab::default(),
            wide: false,
            cell: None,
            preview: None,
        }
    }

    /// The panel.
    pub fn panel(&mut self, ui: &mut Ui, theme: &Theme, texts: &Texts, c: &ChartNode) -> Asks {
        let mut asks = Asks::default();
        ui.horizontal(|ui| {
            ui.label(RichText::new(plain(texts, "chart.title")).font(theme.font("h3")));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let close = ui
                    .add(egui::Button::new(RichText::new("×").size(22.0)).frame(false))
                    .on_hover_text(plain(texts, "chart.close"));
                asks.close = close.clicked();
            });
        });
        ui.add_space(8.0);
        kinds(ui, texts, c, "panel", &mut asks);
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            let half = (ui.available_width() - 8.0) / 2.0;
            for (tab, key) in [(Tab::Data, "chart.data"), (Tab::Looks, "chart.looks")] {
                let on = self.tab == tab;
                let text = RichText::new(plain(texts, key)).font(theme.font("ui-label"));
                let button = egui::Button::selectable(on, text).min_size(Vec2::new(half, 34.0));
                if ui.add(button).clicked() {
                    self.tab = tab;
                }
            }
        });
        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .id_salt("chart.panel")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width() - 12.0);
                match self.tab {
                    Tab::Data => {
                        self.grid(ui, theme, texts, c, (230.0, 96.0, "panel"), &mut asks);
                        ui.add_space(8.0);
                        self.data_buttons(ui, theme, texts, c, &mut asks);
                        ui.add_space(14.0);
                        roles(ui, theme, texts, c, "panel", &mut asks);
                    }
                    Tab::Looks => looks(ui, theme, texts, c, &mut asks),
                }
            });
        asks
    }

    /// «Editar la tabla de datos», over everything.
    pub fn wide(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        (c, size): (&ChartNode, (f64, f64)),
    ) -> Asks {
        let mut asks = Asks::default();
        if !self.wide {
            return asks;
        }
        let screen = ctx.content_rect().size();
        let room = (screen - Vec2::new(120.0, 120.0)).max(Vec2::new(480.0, 320.0));
        let frame = egui::Frame::popup(&ctx.style()).inner_margin(egui::Margin::same(24));
        let modal = egui::Modal::new(egui::Id::new("chart_data")).frame(frame);
        let response = modal.show(ctx, |ui| {
            ui.set_width(room.x);
            ui.set_height(room.y);
            ui.horizontal(|ui| {
                let title = RichText::new(plain(texts, "chart.edit_data"));
                ui.label(title.font(theme.font("h3")));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let text = plain(texts, "chart.done");
                    if widgets::primary(ui, theme, text).clicked() {
                        self.wide = false;
                    }
                });
            });
            ui.add_space(12.0);
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(260.0);
                    self.preview(ui, theme, c, size);
                    ui.add_space(12.0);
                    kinds(ui, texts, c, "wide", &mut asks);
                    ui.add_space(12.0);
                    roles(ui, theme, texts, c, "wide", &mut asks);
                    ui.add_space(12.0);
                    self.data_buttons(ui, theme, texts, c, &mut asks);
                });
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    let tall = ui.available_height() - 8.0;
                    self.grid(ui, theme, texts, c, (tall, 170.0, "wide"), &mut asks);
                });
            });
        });
        if response.should_close() {
            self.wide = false;
        }
        asks
    }

    fn preview(&mut self, ui: &mut Ui, theme: &Theme, c: &ChartNode, size: (f64, f64)) {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(260.0, 200.0), Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, theme.radius_md(), egui::Color32::WHITE);
        let border = Stroke::new(1.0, theme.c("border"));
        painter.rect_stroke(rect, theme.radius_md(), border, egui::StrokeKind::Inside);
        let key = key(c, (size.0.to_bits(), size.1.to_bits()));
        if self.preview.as_ref().is_none_or(|(of, _)| *of != key) {
            // Sizes of a page: the cast loses nothing seen.
            let k = (520.0 / size.0.max(size.1)) as f32;
            self.preview = image(c, size, k).map(|pixels| {
                let options = egui::TextureOptions::LINEAR;
                (key, ui.ctx().load_texture("chart-preview", pixels, options))
            });
        }
        if let Some((_, texture)) = &self.preview {
            let size = texture.size_vec2();
            let room = rect.shrink(6.0);
            let k = (room.width() / size.x).min(room.height() / size.y);
            let place = Rect::from_center_size(rect.center(), size * k);
            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            painter.image(texture.id(), place, uv, egui::Color32::WHITE);
        }
    }

    fn data_buttons(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        texts: &Texts,
        c: &ChartNode,
        asks: &mut Asks,
    ) {
        ui.horizontal_wrapped(|ui| {
            let text = plain(texts, "chart.import");
            if widgets::secondary(ui, theme, text).clicked() {
                asks.import = true;
            }
            if !self.wide {
                let text = plain(texts, "chart.expand");
                if widgets::secondary(ui, theme, text).clicked() {
                    self.wide = true;
                }
            }
            let text = plain(texts, "chart.swap");
            let swap = widgets::secondary(ui, theme, text);
            if swap
                .on_hover_text(plain(texts, "chart.swap_hint"))
                .clicked()
            {
                asks.edit(c, |c| {
                    gafira_chart::transpose(c);
                });
            }
            let text = plain(texts, "chart.clear");
            if widgets::secondary(ui, theme, text).clicked() {
                asks.edit(c, gafira_chart::clear);
            }
        });
    }

    /// The data, a cell each, with room to write more; `tall` px of it,
    /// columns of `wide` px.
    fn grid(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        texts: &Texts,
        c: &ChartNode,
        (tall, wide, place): (f32, f32, &'static str),
        asks: &mut Asks,
    ) {
        let node = self.node;
        let id = |row: usize, col: usize| egui::Id::new(("chart_cell", place, node, row, col));
        let focused = ui.memory(|m| m.focused());
        let writing = self.cell.filter(|(r, col)| focused == Some(id(*r, *col)));
        if let Some(at) = writing {
            paste(ui, c, at, asks);
        }
        // As many as fill the room, and some more past the data.
        let fill = |room: f32, side: f32| (room / side).max(0.0) as usize;
        let rows = (c.rows() + SPARE.0)
            .max(fill(tall, ROW))
            .min(ChartNode::MAX_ROWS);
        let columns = (c.columns + SPARE.1)
            .max(fill(ui.available_width(), wide))
            .min(ChartNode::MAX_COLUMNS);
        let kinds: Vec<Column> = (0..columns)
            .map(|col| gafira_chart::column(c, col))
            .collect();
        let by = gafira_chart::colored(c);
        let places = gafira_chart::places(c);
        let (_, values) = gafira_chart::columns(c);
        let names = gafira_chart::color_names(c);
        const HEAD: f32 = 34.0;
        let line = Stroke::new(1.0, theme.c("border"));
        let mut next_focus = None;
        egui::Frame::NONE.stroke(line).show(ui, |ui| {
            egui::ScrollArea::both()
                .id_salt(("chart.grid", place))
                .max_height(tall)
                .min_scrolled_height(tall)
                .auto_shrink([false, false])
                .show_rows(ui, ROW, rows + 1, |ui, shown| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    ui.set_min_width(HEAD + wide * columns as f32);
                    for line_of in shown {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::ZERO;
                            // The first line says what each column holds.
                            let Some(row) = line_of.checked_sub(1) else {
                                square(ui, theme, HEAD);
                                for (col, kind) in kinds.iter().enumerate() {
                                    let rect = square(ui, theme, wide);
                                    let text = match kind {
                                        Column::Numbers => "123",
                                        _ => "abc",
                                    };
                                    let at = rect.center();
                                    let color = theme.c("text.secondary");
                                    ui.painter().text(
                                        at,
                                        egui::Align2::CENTER_CENTER,
                                        text,
                                        theme.font("caption"),
                                        color,
                                    );
                                    let series = values.iter().position(|v| *v == col);
                                    if by == Colored::Series
                                        && let Some(series) = series
                                    {
                                        let at = rect.left_center() + Vec2::new(14.0, 0.0);
                                        dot(ui, c, series, at, (place, "column"), asks);
                                    }
                                }
                                return;
                            };
                            let head = square(ui, theme, HEAD);
                            let color = match by {
                                Colored::Rows => places.get(row).copied().flatten(),
                                _ => None,
                            };
                            if let Some(index) = color.filter(|i| *i < names.len()) {
                                dot(ui, c, index, head.center(), (place, "row"), asks);
                            }
                            for (col, kind) in kinds.iter().enumerate() {
                                let cell = Cell {
                                    id: id(row, col),
                                    row,
                                    col,
                                    wide,
                                    numbers: *kind == Column::Numbers,
                                };
                                if cell.show(ui, theme, texts, c, asks) {
                                    self.cell = Some((row, col));
                                }
                                if self.cell == Some((row, col))
                                    && ui.memory(|m| m.focused()).is_none()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    next_focus = Some((row + 1, col));
                                }
                            }
                        });
                    }
                });
        });
        if let Some((row, col)) = next_focus.filter(|(row, _)| *row < rows) {
            ui.memory_mut(|m| m.request_focus(id(row, col)));
            self.cell = Some((row, col));
        }
    }
}

/// A square of the edge of the data.
fn square(ui: &mut Ui, theme: &Theme, wide: f32) -> Rect {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(wide, ROW), Sense::hover());
    ui.painter()
        .rect_filled(rect, 0.0, theme.c("surface.sunken"));
    edge(ui, theme, rect);
    rect
}

fn edge(ui: &Ui, theme: &Theme, rect: Rect) {
    let line = Stroke::new(1.0, theme.c("border"));
    let painter = ui.painter();
    // Inside the cell: the next one would cover its edge.
    let rect = Rect::from_min_max(rect.min, rect.max - Vec2::splat(0.5));
    painter.line_segment([rect.right_top(), rect.right_bottom()], line);
    painter.line_segment([rect.left_bottom(), rect.right_bottom()], line);
}

struct Cell {
    id: egui::Id,
    row: usize,
    col: usize,
    wide: f32,
    numbers: bool,
}

impl Cell {
    /// A field with the words of the cell; whether it has the keys.
    fn show(
        &self,
        ui: &mut Ui,
        theme: &Theme,
        texts: &Texts,
        c: &ChartNode,
        asks: &mut Asks,
    ) -> bool {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(self.wide, ROW), Sense::hover());
        // The rows below are there to write in; a column more is farther.
        let inside = self.col < c.columns;
        let fill = if inside { "surface" } else { "surface.sunken" };
        ui.painter().rect_filled(rect, 0.0, theme.c(fill));
        edge(ui, theme, rect);
        let mut words = c.cell(self.row, self.col).to_owned();
        let head = self.row == 0;
        // A cell of a column of numbers that is not one is left out, and
        // says so.
        let wrong =
            self.numbers && !head && !words.trim().is_empty() && !gafira_chart::is_number(&words);
        let font = theme.font(if head { "ui-label" } else { "body-sm" });
        let color = if wrong {
            theme.c("error.fg")
        } else {
            theme.c("text")
        };
        let align = if self.numbers && !head {
            egui::Align::RIGHT
        } else {
            egui::Align::LEFT
        };
        let field = egui::TextEdit::singleline(&mut words)
            .id(self.id)
            .font(font)
            .text_color(color)
            .frame(false)
            .char_limit(ChartNode::MAX_WORDS / 4)
            .margin(Vec2::new(10.0, 5.0))
            .horizontal_align(align);
        let response = ui.put(rect.shrink(1.5), field);
        if response.has_focus() {
            let stroke = Stroke::new(2.0, theme.c("primary.bg"));
            let painter = ui.painter();
            painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
        }
        if response.changed() {
            let at = (self.row, self.col);
            asks.edit(c, |c| {
                gafira_chart::set_cell(c, at.0, at.1, &words);
            });
        }
        let has_keys = response.has_focus();
        if wrong {
            response.on_hover_text(plain(texts, "chart.not_number"));
        }
        has_keys
    }
}

/// Several cells in the clipboard go to as many, from the one at `at`.
fn paste(ui: &Ui, c: &ChartNode, at: (usize, usize), asks: &mut Asks) {
    let pasted = ui.input_mut(|i| {
        let found = i.events.iter().position(
            |e| matches!(e, egui::Event::Paste(text) if text.trim_end().contains(['\t', '\n'])),
        )?;
        match i.events.remove(found) {
            egui::Event::Paste(text) => Some(text),
            _ => None,
        }
    });
    let Some(text) = pasted else { return };
    let grid = gafira_table::parse_grid(&text);
    asks.edit(c, |c| {
        for (r, row) in grid.iter().enumerate() {
            for (col, words) in row.iter().enumerate() {
                gafira_chart::set_cell(c, at.0 + r, at.1 + col, words);
            }
        }
    });
    ui.ctx().request_repaint();
}

/// A dot with the colour `index`, which a click changes.
fn dot(
    ui: &mut Ui,
    c: &ChartNode,
    index: usize,
    at: Pos2,
    salt: (&'static str, &'static str),
    asks: &mut Asks,
) {
    let rect = Rect::from_center_size(at, Vec2::splat(18.0));
    ui.push_id((salt, index), |ui| {
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        child.spacing_mut().interact_size = Vec2::splat(16.0);
        swatch(&mut child, c, index, asks);
    });
}

/// The colour at `index`, which a click changes.
fn swatch(ui: &mut Ui, c: &ChartNode, index: usize, asks: &mut Asks) -> egui::Response {
    let place = index % c.colors.len().max(1);
    let color = c.colors.get(place).copied();
    let mut rgba = canvas::picker_rgba(color);
    let response = crate::eyedropper::color_edit(ui, &mut rgba);
    if response.changed() {
        let picked = canvas::picked_color(color, rgba);
        asks.edit(c, |c| set_color(c, index, picked));
    }
    response
}

/// Makes `color` the one at `index`: the colours before it that go over
/// again are written down, so no other changes.
pub(crate) fn set_color(c: &mut ChartNode, index: usize, color: Color) {
    if index >= gafira_doc::MAX_CHART_COLORS || c.colors.is_empty() {
        return;
    }
    let first = c.colors.len();
    while c.colors.len() <= index {
        let next = c.colors[c.colors.len() % first];
        c.colors.push(next);
    }
    c.colors[index] = color;
}

/// The kinds, by their groups.
fn kinds(ui: &mut Ui, texts: &Texts, c: &ChartNode, place: &str, asks: &mut Asks) {
    let wide = ui.available_width();
    egui::ComboBox::from_id_salt(("chart.kind", place))
        .selected_text(plain(texts, kind_key(c.kind)))
        .width(wide)
        .height(420.0)
        .show_ui(ui, |ui| {
            for group in Group::ALL {
                ui.add_space(4.0);
                ui.label(RichText::new(plain(texts, group.key())).small().strong());
                let mut seen: Vec<ChartKind> = Vec::new();
                for template in TEMPLATES.iter().filter(|t| t.group == group) {
                    if seen.contains(&template.kind) {
                        continue;
                    }
                    seen.push(template.kind);
                    let on = c.kind == template.kind;
                    let text = plain(texts, kind_key(template.kind));
                    if ui.selectable_label(on, text).clicked() && !on {
                        asks.edit(c, |c| set_kind(c, template.kind));
                    }
                }
            }
        });
}

/// Makes `c` of `kind`: its data, colours and words stay, and the rest is as
/// the kind is at first.
pub(crate) fn set_kind(c: &mut ChartNode, kind: ChartKind) {
    let (size, weight) = (c.text.size, c.weight);
    c.kind = kind;
    dress(c);
    c.text.size = size;
    // As thick for these words as it is at first.
    let k = size / crate::chart_templates::dressed(kind).text.size;
    c.weight = (c.weight * k).clamp(0.1, ChartNode::MAX_WEIGHT);
    if !weight.is_finite() {
        c.weight = 1.0;
    }
}

/// «Configuración del gráfico»: the column of the labels, the ones of the
/// values, and what is done with the rows of the same label.
fn roles(ui: &mut Ui, theme: &Theme, texts: &Texts, c: &ChartNode, place: &str, asks: &mut Asks) {
    let title = |ui: &mut Ui, key| {
        let text = RichText::new(plain(texts, key)).font(theme.font("body-sm"));
        ui.label(text.color(theme.c("text.secondary")));
    };
    let name = |col: usize| {
        let words = c.cell(0, col).trim();
        if words.is_empty() {
            format!("{} {}", plain(texts, "chart.column"), col + 1)
        } else {
            words.to_owned()
        }
    };
    let (labels, values) = gafira_chart::columns(c);
    let used: Vec<usize> = (0..c.columns)
        .filter(|col| gafira_chart::column(c, *col) != Column::Empty)
        .collect();
    let wide = ui.available_width();
    let text = RichText::new(plain(texts, "chart.setup")).font(theme.font("ui-label"));
    ui.label(text);
    ui.add_space(6.0);
    title(ui, "chart.labels_of");
    egui::ComboBox::from_id_salt(("chart.labels", place))
        .selected_text(labels.map_or_else(|| plain(texts, "chart.no_labels"), name))
        .width(wide)
        .show_ui(ui, |ui| {
            for col in &used {
                if ui
                    .selectable_label(labels == Some(*col), name(*col))
                    .clicked()
                {
                    asks.edit(c, |c| {
                        c.labels = Some(*col);
                        c.values.retain(|v| v != col);
                    });
                }
            }
        });
    ui.add_space(6.0);
    title(ui, "chart.values_of");
    let chosen: Vec<String> = values.iter().map(|col| name(*col)).collect();
    let chosen = if chosen.is_empty() {
        plain(texts, "chart.no_values")
    } else {
        chosen.join(", ")
    };
    egui::ComboBox::from_id_salt(("chart.values", place))
        .selected_text(chosen)
        .width(wide)
        .truncate()
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show_ui(ui, |ui| {
            for col in used.iter().filter(|col| labels != Some(**col)) {
                let mut on = values.contains(col);
                if ui.checkbox(&mut on, name(*col)).changed() {
                    asks.edit(c, |c| {
                        let mut next = values.clone();
                        next.retain(|v| v != col);
                        // With one series, the one chosen takes its place.
                        if on && single(c.kind) {
                            next.clear();
                        }
                        if on {
                            next.push(*col);
                        }
                        c.values = next;
                    });
                }
            }
        });
    ui.add_space(6.0);
    title(ui, "chart.combine");
    egui::ComboBox::from_id_salt(("chart.combine", place))
        .selected_text(plain(texts, combine_key(c.combine)))
        .width(wide)
        .show_ui(ui, |ui| {
            for how in [
                ChartCombine::None,
                ChartCombine::Sum,
                ChartCombine::Mean,
                ChartCombine::Min,
                ChartCombine::Max,
                ChartCombine::Count,
            ] {
                let on = c.combine == how;
                if ui
                    .selectable_label(on, plain(texts, combine_key(how)))
                    .clicked()
                {
                    asks.edit(c, |c| c.combine = how);
                }
            }
        });
}

/// Whether a kind draws one column of values.
fn single(kind: ChartKind) -> bool {
    matches!(
        kind,
        ChartKind::Columns
            | ChartKind::Rows
            | ChartKind::Pie
            | ChartKind::Donut
            | ChartKind::Pack
            | ChartKind::Treemap
            | ChartKind::NestedPack
            | ChartKind::Sunburst
            | ChartKind::Histogram
            | ChartKind::Funnel
    )
}

fn combine_key(how: ChartCombine) -> &'static str {
    match how {
        ChartCombine::None => "chart.combine_none",
        ChartCombine::Sum => "chart.combine_sum",
        ChartCombine::Mean => "chart.combine_mean",
        ChartCombine::Min => "chart.combine_min",
        ChartCombine::Max => "chart.combine_max",
        ChartCombine::Count => "chart.combine_count",
    }
}

fn numbers_key(how: ChartLabels) -> &'static str {
    match how {
        ChartLabels::None => "chart.numbers_none",
        ChartLabels::Value => "chart.numbers_value",
        ChartLabels::Percent => "chart.numbers_percent",
    }
}

/// The name of an option, its switch and what it sets.
type Switch = (&'static str, bool, fn(&mut ChartNode, bool));

/// «Personalizar».
fn looks(ui: &mut Ui, theme: &Theme, texts: &Texts, c: &ChartNode, asks: &mut Asks) {
    use ChartKind as K;
    let title = |ui: &mut Ui, key| {
        ui.add_space(12.0);
        let text = RichText::new(plain(texts, key)).font(theme.font("ui-label"));
        ui.label(text.color(theme.c("text")));
        ui.add_space(4.0);
    };
    title(ui, "chart.colors");
    for (index, (key, _)) in PALETTES.iter().enumerate() {
        let colors = palette(index);
        let on = c.colors == colors;
        if palette_row(ui, theme, &plain(texts, key), &colors, on) {
            asks.edit(c, |c| c.colors = palette(index));
        }
    }
    ui.add_space(6.0);
    let names = gafira_chart::color_names(c);
    let count = names.len().min(gafira_doc::MAX_CHART_COLORS);
    for (index, name) in names.iter().enumerate().take(count) {
        ui.push_id(("color", index), |ui| {
            ui.horizontal(|ui| {
                swatch(ui, c, index, asks);
                let text = if name.is_empty() { "—" } else { name };
                ui.add(egui::Label::new(text).truncate());
            });
        });
    }
    if matches!(c.kind, K::Columns | K::Rows | K::ProgressBar) {
        let mut on = c.by_label;
        if switch(ui, theme, plain(texts, "chart.by_label"), &mut on) {
            asks.edit(c, |c| c.by_label = on);
        }
    }
    title(ui, "chart.text");
    let mut words = c.title.clone();
    let field = widgets::field(&mut words, plain(texts, "chart.title_hint"));
    if ui.add(field.char_limit(ChartNode::MAX_WORDS / 4)).changed() {
        asks.edit(c, |c| c.title = words);
    }
    ui.add_space(6.0);
    text_controls(ui, theme, texts, c, asks);
    title(ui, "chart.show");
    // One row, centred down, so the label and its field line up.
    let row = Vec2::new(ui.available_width(), 28.0);
    ui.allocate_ui_with_layout(
        row,
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.label(plain(texts, "chart.numbers"));
            egui::ComboBox::from_id_salt("chart.numbers")
                .selected_text(plain(texts, numbers_key(c.numbers)))
                .show_ui(ui, |ui| {
                    for how in [ChartLabels::None, ChartLabels::Value, ChartLabels::Percent] {
                        let on = c.numbers == how;
                        if ui
                            .selectable_label(on, plain(texts, numbers_key(how)))
                            .clicked()
                        {
                            asks.edit(c, |c| c.numbers = how);
                        }
                    }
                });
        },
    );
    let axes = matches!(
        c.kind,
        K::Columns
            | K::Rows
            | K::GroupedColumns
            | K::GroupedRows
            | K::StackedColumns
            | K::StackedRows
            | K::Line
            | K::StackedArea
            | K::Area
            | K::PercentArea
            | K::Scatter
            | K::Bubbles
            | K::DotColumns
            | K::DotRows
            | K::Histogram
            | K::Radar
    );
    let lines = matches!(c.kind, K::Line | K::Area | K::Radar);
    let smooth = matches!(c.kind, K::Line | K::Area | K::StackedArea | K::PercentArea);
    let mut switches: Vec<Switch> = vec![
        ("chart.names", c.names, |c, on| c.names = on),
        ("chart.legend", c.legend, |c, on| c.legend = on),
    ];
    if axes {
        switches.push(("chart.axis", c.axis, |c, on| c.axis = on));
        switches.push(("chart.grid", c.grid, |c, on| c.grid = on));
    }
    if smooth {
        switches.push(("chart.smooth", c.smooth, |c, on| c.smooth = on));
    }
    if lines {
        switches.push(("chart.markers", c.markers, |c, on| c.markers = on));
    }
    for (key, was, set) in switches {
        let mut on = was;
        if switch(ui, theme, plain(texts, key), &mut on) {
            asks.edit(c, |c| set(c, on));
        }
    }
    let bars = matches!(
        c.kind,
        K::Columns
            | K::Rows
            | K::GroupedColumns
            | K::GroupedRows
            | K::StackedColumns
            | K::StackedRows
            | K::Histogram
            | K::ProgressBar
    );
    let rounded = bars || matches!(c.kind, K::Treemap | K::ProgressRing | K::ProgressArc);
    let hollow = matches!(
        c.kind,
        K::Donut | K::Sunburst | K::ProgressRing | K::ProgressArc | K::Gauge
    );
    let weighed = lines || matches!(c.kind, K::Scatter | K::DotColumns | K::DotRows);
    if rounded || hollow || weighed {
        title(ui, "chart.shape");
    }
    if rounded {
        share(ui, (theme, texts), "chart.round", c.round, 1.0, |v| {
            asks.edit(c, |c| c.round = v);
        });
    }
    if bars {
        share(
            ui,
            (theme, texts),
            "chart.gap",
            c.gap,
            ChartNode::MAX_GAP,
            |v| {
                asks.edit(c, |c| c.gap = v);
            },
        );
    }
    if hollow {
        share(
            ui,
            (theme, texts),
            "chart.hole",
            c.hole,
            ChartNode::MAX_HOLE,
            |v| {
                asks.edit(c, |c| c.hole = v);
            },
        );
    }
    if weighed {
        ui.allocate_ui_with_layout(
            row,
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(plain(texts, "chart.weight"));
                let mut weight = c.weight;
                let field = egui::DragValue::new(&mut weight)
                    .range(0.5..=ChartNode::MAX_WEIGHT)
                    .speed(0.1)
                    .max_decimals(1);
                if ui.add(field).changed() {
                    asks.edit(c, |c| c.weight = weight);
                }
            },
        );
    }
}

/// A palette: its colours and its name; whether it was chosen.
fn palette_row(ui: &mut Ui, theme: &Theme, name: &str, colors: &[Color], on: bool) -> bool {
    let size = Vec2::new(ui.available_width(), 30.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let painter = ui.painter();
    if on || response.hovered() || response.has_focus() {
        painter.rect_filled(rect, theme.radius_md(), theme.c("surface.sunken"));
    }
    if on {
        let stroke = Stroke::new(1.5, theme.c("primary.bg"));
        painter.rect_stroke(rect, theme.radius_md(), stroke, egui::StrokeKind::Inside);
    }
    for (i, color) in colors.iter().enumerate() {
        let at = rect.left_center() + Vec2::new(14.0 + 14.0 * i as f32, 0.0);
        let [r, g, b, a] = canvas::picker_rgba(Some(*color));
        let color = egui::Color32::from_rgba_unmultiplied(r, g, b, a);
        painter.circle_filled(at, 5.5, color);
    }
    painter.text(
        rect.right_center() - Vec2::new(10.0, 0.0),
        egui::Align2::RIGHT_CENTER,
        name,
        theme.font("body-sm"),
        theme.c("text"),
    );
    response
        .widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, on, name));
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

fn switch(ui: &mut Ui, theme: &Theme, name: String, on: &mut bool) -> bool {
    ui.horizontal(|ui| {
        ui.label(name);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            widgets::toggle(ui, theme, on).changed()
        })
        .inner
    })
    .inner
}

fn share(
    ui: &mut Ui,
    (theme, texts): (&Theme, &Texts),
    key: &'static str,
    value: f64,
    most: f64,
    set: impl FnOnce(f64),
) {
    let mut percent = value * 100.0;
    if crate::slider::Slider::new(plain(texts, key), &mut percent, (0.0, most * 100.0))
        .suffix(crate::slider::percent(texts))
        .show(ui, theme)
    {
        set((percent / 100.0).clamp(0.0, most));
    }
}

/// The font, the size, the colour, bold and italic of the words.
fn text_controls(ui: &mut Ui, theme: &Theme, texts: &Texts, c: &ChartNode, asks: &mut Asks) {
    ui.horizontal_wrapped(|ui| {
        if ui
            .button(&c.text.font)
            .on_hover_text(plain(texts, "chart.font"))
            .clicked()
        {
            asks.font = true;
        }
        ink_controls(ui, theme, texts, c, asks);
    });
}

fn ink_controls(ui: &mut Ui, theme: &Theme, texts: &Texts, c: &ChartNode, asks: &mut Asks) {
    let mut ink = canvas::picker_rgba(Some(c.text.color));
    ui.push_id("chart.ink", |ui| {
        let button = crate::eyedropper::color_edit(ui, &mut ink);
        if button.on_hover_text(plain(texts, "table.ink")).changed() {
            let picked = canvas::picked_color(Some(c.text.color), ink);
            asks.edit(c, |c| c.text.color = picked);
        }
    });
    let mut size = c.text.size;
    let field = egui::DragValue::new(&mut size)
        .range(1.0..=gafira_doc::TextNode::MAX_SIZE)
        .speed(0.5)
        .max_decimals(1);
    if ui
        .add(field)
        .on_hover_text(plain(texts, "table.size"))
        .changed()
    {
        asks.edit(c, |c| c.text.size = size);
    }
    let letter = |ui: &mut Ui, text: &str, key: &'static str, on: bool| {
        let label = RichText::new(text).font(theme.font("ui-label"));
        ui.selectable_label(on, label)
            .on_hover_text(plain(texts, key))
            .clicked()
    };
    if letter(ui, "B", "inspector.bold", c.text.bold) {
        asks.edit(c, |c| c.text.bold = !c.text.bold);
    }
    if letter(ui, "I", "inspector.italic", c.text.italic) {
        asks.edit(c, |c| c.text.italic = !c.text.italic);
    }
}

/// The most colours in the bar.
const IN_BAR: usize = 5;

/// The controls of a chart in the bar over it (S83-08); whether «Editar» was
/// pressed, and the changes.
pub(crate) fn bar(ui: &mut Ui, theme: &Theme, texts: &Texts, c: &ChartNode) -> (bool, Asks) {
    let mut asks = Asks::default();
    let edit = ui.button(plain(texts, "chart.edit")).clicked();
    let names = gafira_chart::color_names(c);
    for (index, name) in names.iter().enumerate().take(IN_BAR) {
        ui.push_id(("chart.bar", index), |ui| {
            swatch(ui, c, index, &mut asks).on_hover_text(name);
        });
    }
    ink_controls(ui, theme, texts, c, &mut asks);
    (edit, asks)
}

/// The chart in the inspector.
pub(crate) fn inspector(ui: &mut Ui, theme: &Theme, texts: &Texts, c: &ChartNode) -> (bool, Asks) {
    let mut asks = Asks::default();
    let text = plain(texts, "chart.edit_chart");
    let edit = widgets::primary(ui, theme, text).clicked();
    looks(ui, theme, texts, c, &mut asks);
    (edit, asks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_templates::dressed;

    // S83-05: a colour past the last one leaves the others as they were.
    #[test]
    fn s83_05_a_colour_of_its_own() {
        let mut c = dressed(ChartKind::Pie);
        let before = c.colors.clone();
        let white = crate::colors::hex(0xFF_FFFF);
        set_color(&mut c, 9, white);
        assert_eq!(c.colors.len(), 10);
        assert_eq!(
            (c.colors[0], c.colors[8], c.colors[9]),
            (before[0], before[0], white)
        );
        set_color(&mut c, 500, white);
        assert!(c.is_valid() && c.colors.len() == 10);
    }

    // S83-07: another kind keeps the data, the colours and the words.
    #[test]
    fn s83_07_another_kind() {
        let texts = Texts::new(gafira_ports::i18n::Locale::Es);
        for template in TEMPLATES {
            let mut c = TEMPLATES[0].chart(&texts);
            c.text.size = 40.0;
            let before = c.clone();
            set_kind(&mut c, template.kind);
            assert!(c.is_valid());
            assert_eq!(
                (&c.cells, &c.colors, &c.text),
                (&before.cells, &before.colors, &before.text)
            );
        }
    }

    // S83-03: the canvas draws a chart, and none without a box.
    #[test]
    fn s83_03_pixels() {
        let texts = Texts::new(gafira_ports::i18n::Locale::Es);
        let c = TEMPLATES[0].chart(&texts);
        let pixels = image(&c, (200.0, 150.0), 1.0).expect("pixels");
        assert!(pixels.pixels.iter().any(|p| p.a() > 0));
        assert!(image(&c, (0.0, 10.0), 1.0).is_none());
    }
}
