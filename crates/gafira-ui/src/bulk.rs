//! «Crear en lote» (spec 104): the panel of the rail that connects a table
//! to the design in three steps, Datos → Conectar → Generar, and the wide
//! table to read and write the data. The run itself is in `bulk_run`.
//!
//! A column is a field and a row a variation: a text says `{{key}}` where
//! the value of its row goes. The table travels inside the `.gaf`
//! (`DataSet`); the panel never changes the document by itself, it asks the
//! editor through [`Asks`], so every change is one undo step.

use std::sync::Arc;

use egui::{Pos2, RichText, Sense, Ui, Vec2};
use gafira_bulk::{FieldRef, Grid, Imported};
use gafira_doc::{DataColumn, DataRow, DataSet, DataSource, Document, NodeId, PageId};

use crate::bulk_run::{Phase, Status};
use crate::data_grid::{self, Cells, Focus};
use crate::style::Theme;
use crate::text::Texts;
use crate::widgets;

pub(crate) fn plain(texts: &Texts, key: &'static str) -> String {
    crate::charts::plain(texts, key)
}

/// The step of the panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Step {
    #[default]
    Data,
    Connect,
    Review,
}

/// A table read or pasted that waits for its row of names (§5.2).
pub(crate) struct Pending {
    pub grid: Grid,
    pub header: Option<usize>,
    pub source: DataSource,
    pub name: String,
    /// The table with the names chosen, made again when they change.
    imported: Imported,
}

impl Pending {
    pub(crate) fn new(grid: Grid, source: DataSource, name: String, texts: &Texts) -> Self {
        let header = gafira_bulk::guess_header(&grid);
        let imported = gafira_bulk::from_grid(&grid, header, &unnamed(texts));
        Self {
            grid,
            header,
            source,
            name,
            imported,
        }
    }

    fn choose(&mut self, header: Option<usize>, texts: &Texts) {
        if header != self.header {
            self.header = header;
            self.imported = gafira_bulk::from_grid(&self.grid, header, &unnamed(texts));
        }
    }

    /// The table to put in the document.
    fn table(&self) -> DataSet {
        let mut set = self.imported.set.clone();
        set.name.clone_from(&self.name);
        let mut source = self.source.clone();
        source.header_row = self.header.map(|h| h + 1);
        set.source = Some(source);
        set
    }
}

/// «Columna A», in the language of the interface.
fn unnamed(texts: &Texts) -> impl Fn(usize) -> String + '_ {
    move |i| {
        texts.t(&gafira_doc::msg!(
            "bulk.unnamed",
            letter = Text(gafira_bulk::column_letter(i))
        ))
    }
}

/// What the panel asks the editor for.
#[derive(Default)]
pub(crate) struct Asks {
    /// «Subir CSV o Excel».
    pub import: bool,
    /// The new connected data (none removes it), one undo step.
    pub data: Option<Option<DataSet>>,
    /// A new text with this field on the page in view.
    pub insert: Option<String>,
    /// A new text with this field where it was dropped.
    pub drop: Option<(String, PageId, (f64, f64))>,
    /// The words of the selected text become this field.
    pub replace: Option<String>,
    /// This field goes after the words of the selected text.
    pub append: Option<String>,
    /// «Exportar en lote…».
    pub export: bool,
    /// «Ir»: select this element.
    pub go_to: Option<(PageId, NodeId)>,
    /// A picture of a row for the preview: its key, the row and the page.
    pub preview: Option<(u64, usize, PageId)>,
}

/// What the panel sees of the editor.
pub(crate) struct View<'a> {
    pub doc: &'a Document,
    /// A text, or a shape that holds one, is the only selection.
    pub can_fill: bool,
    /// What that text says, to name it.
    pub selected: Option<String>,
    /// The page and point under a screen point, if it is over a page.
    pub drop_point: &'a dyn Fn(Pos2) -> Option<(PageId, (f64, f64))>,
}

/// The panel «Crear en lote».
#[derive(Default)]
pub(crate) struct Panel {
    pub step: Step,
    /// The row shown, from 0.
    pub row: usize,
    /// The template page shown in the preview.
    page: usize,
    pub pending: Option<Pending>,
    /// The words of «Pegar desde una hoja», while it is open.
    pasting: Option<String>,
    /// The wide table is open.
    wide: bool,
    focus: Focus,
    wide_focus: Focus,
    search: String,
    preview: Option<(u64, egui::TextureHandle)>,
    /// The picture the editor drew for the key it was asked.
    pub picture: Option<(u64, egui::ColorImage)>,
    /// The file being read, for the person.
    pub reading: Option<String>,
    /// The kind and the name of the file being read.
    pub source_file: Option<(String, String)>,
    /// A new table being written in the wide grid, not yet in the design.
    draft: Option<DataSet>,
    /// The key of the field copied last, said on its button.
    copied: Option<String>,
    /// The row shown as large as the window allows, and its picture.
    pub zoom: bool,
    big: Option<(u64, egui::TextureHandle)>,
    pub big_picture: Option<(u64, egui::ColorImage)>,
}

impl Panel {
    /// The panel, in the side panel of the editor.
    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        texts: &Texts,
        view: &View<'_>,
    ) -> Asks {
        let mut asks = Asks::default();
        ui.label(RichText::new(plain(texts, "bulk.title")).font(theme.font("h3")));
        ui.add_space(4.0);
        ui.label(
            RichText::new(plain(texts, "bulk.intro"))
                .font(theme.font("body-sm"))
                .color(theme.c("text.secondary")),
        );
        ui.add_space(10.0);
        self.steps(ui, theme, texts, view.doc.data().is_some());
        ui.add_space(10.0);
        egui::ScrollArea::vertical()
            .id_salt("bulk.panel")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width() - 8.0);
                match (self.step, view.doc.data()) {
                    (Step::Data, data) | (_, data @ None) => {
                        self.step = Step::Data;
                        self.data(ui, theme, texts, data, &mut asks);
                    }
                    (Step::Connect, Some(set)) => {
                        self.connect(ui, theme, texts, view, set, &mut asks)
                    }
                    (Step::Review, Some(set)) => {
                        self.review(ui, theme, texts, view, set, &mut asks);
                    }
                }
            });
        asks
    }

    /// «1 Datos · 2 Conectar · 3 Generar».
    fn steps(&mut self, ui: &mut Ui, theme: &Theme, texts: &Texts, has_data: bool) {
        ui.horizontal(|ui| {
            // Three buttons and the two gaps between them fill the row
            // exactly: a wider row would widen the side panel every frame.
            let gap = ui.spacing().item_spacing.x;
            let third = ((ui.available_width() - 2.0 * gap) / 3.0).floor();
            for (step, key) in [
                (Step::Data, "bulk.step_data"),
                (Step::Connect, "bulk.step_connect"),
                (Step::Review, "bulk.step_review"),
            ] {
                let on = self.step == step;
                let text = RichText::new(plain(texts, key)).font(theme.font("ui-label"));
                let button = egui::Button::selectable(on, text).min_size(Vec2::new(third, 34.0));
                let enabled = has_data || step == Step::Data;
                let response = ui.add_enabled(enabled, button);
                if response.clicked() {
                    self.step = step;
                }
                if !enabled {
                    response.on_disabled_hover_text(plain(texts, "bulk.need_data"));
                }
            }
        });
    }

    // ---- Step 1: Datos -------------------------------------------------

    fn data(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        texts: &Texts,
        data: Option<&DataSet>,
        asks: &mut Asks,
    ) {
        if let Some(file) = &self.reading {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(18.0).color(theme.c("accent")));
                ui.label(texts.t(&gafira_doc::msg!("bulk.reading", file = Text(file.clone()))));
            });
            return;
        }
        if self.pending.is_some() {
            self.names(ui, theme, texts, asks);
            return;
        }
        if let Some(words) = &mut self.pasting {
            hint(ui, theme, &plain(texts, "bulk.paste_hint"));
            ui.add(
                egui::TextEdit::multiline(words)
                    .desired_rows(8)
                    .desired_width(f32::INFINITY)
                    .font(theme.font("body-sm")),
            );
            ui.add_space(8.0);
            let mut done = None;
            ui.horizontal(|ui| {
                if widgets::primary(ui, theme, plain(texts, "bulk.paste_use")).clicked() {
                    done = Some(true);
                }
                if widgets::secondary(ui, theme, plain(texts, "bulk.cancel")).clicked() {
                    done = Some(false);
                }
            });
            match done {
                Some(true) => {
                    let grid = gafira_table::parse_grid(&self.pasting.take().unwrap_or_default());
                    if grid.iter().any(|r| r.iter().any(|c| !c.trim().is_empty())) {
                        let source = DataSource {
                            kind: "paste".to_owned(),
                            ..DataSource::default()
                        };
                        let name = plain(texts, "bulk.pasted");
                        self.pending = Some(Pending::new(grid, source, name, texts));
                    }
                }
                Some(false) => self.pasting = None,
                None => {}
            }
            return;
        }
        let Some(set) = data else {
            self.sources(ui, theme, texts, asks);
            return;
        };
        // The table in the document: its size, the cells and what to do.
        card(ui, theme, |ui| {
            ui.label(RichText::new(&set.name).font(theme.font("ui-label")));
            ui.label(
                RichText::new(texts.t(&gafira_doc::msg!(
                    "bulk.summary",
                    rows = Count(set.rows.len() as u64),
                    fields = Count(set.columns.len() as u64)
                )))
                .color(theme.c("text.secondary")),
            );
        });
        ui.add_space(8.0);
        let mut cells = DataCells::new(set, texts);
        let salt = ("bulk", "panel");
        data_grid::grid(ui, theme, &mut cells, &mut self.focus, (240.0, 120.0, salt));
        if let Some(next) = cells.out {
            asks.data = Some(Some(next));
        }
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if widgets::secondary(ui, theme, plain(texts, "bulk.edit_table")).clicked() {
                self.wide = true;
            }
            if widgets::secondary(ui, theme, plain(texts, "bulk.replace_data")).clicked() {
                asks.import = true;
            }
            if widgets::secondary(ui, theme, plain(texts, "bulk.paste")).clicked() {
                self.pasting = Some(String::new());
            }
            if widgets::secondary(ui, theme, plain(texts, "bulk.remove_data"))
                .on_hover_text(plain(texts, "bulk.remove_hint"))
                .clicked()
            {
                asks.data = Some(None);
            }
        });
        ui.add_space(12.0);
        hint(ui, theme, &plain(texts, "bulk.kept_inside"));
        ui.add_space(12.0);
        if widgets::primary(ui, theme, plain(texts, "bulk.next_connect")).clicked() {
            self.step = Step::Connect;
        }
    }

    /// The three ways to bring a table (§5.1).
    fn sources(&mut self, ui: &mut Ui, theme: &Theme, texts: &Texts, asks: &mut Asks) {
        instructions(
            ui,
            theme,
            texts,
            &["bulk.how_1", "bulk.how_2", "bulk.how_3", "bulk.how_4"],
        );
        ui.add_space(12.0);
        // Actions look like buttons, not like the steps above them (owner
        // request): the main one coral, the others outlined.
        let width = ui.available_width();
        let upload =
            action(theme, plain(texts, "bulk.upload"), true).min_size(Vec2::new(width, 40.0));
        if ui.add(upload).clicked() {
            asks.import = true;
        }
        ui.add_space(8.0);
        let paste =
            action(theme, plain(texts, "bulk.paste"), false).min_size(Vec2::new(width, 40.0));
        if ui.add(paste).clicked() {
            self.pasting = Some(String::new());
        }
        ui.add_space(8.0);
        let create = action(theme, plain(texts, "bulk.create_table"), false)
            .min_size(Vec2::new(width, 40.0));
        if ui.add(create).clicked() {
            // A draft until «Usar esta tabla»: «Cancelar» leaves no trace.
            self.draft = Some(new_table(texts));
            self.wide = true;
        }
        ui.add_space(10.0);
        hint(ui, theme, &plain(texts, "bulk.formats"));
    }

    /// The row of names of a table that waits, and «Cargar» (§5.2).
    fn names(&mut self, ui: &mut Ui, theme: &Theme, texts: &Texts, asks: &mut Asks) {
        let Some(pending) = &mut self.pending else {
            return;
        };
        ui.label(RichText::new(&pending.name).font(theme.font("ui-label")));
        if let Some(sheet) = &pending.source.sheet {
            hint(ui, theme, sheet);
        }
        ui.add_space(8.0);
        ui.label(plain(texts, "bulk.header_row"));
        let row_label = |r: usize, grid: &Grid| {
            let words: Vec<&str> = grid[r]
                .iter()
                .map(|c| c.trim())
                .filter(|c| !c.is_empty())
                .take(3)
                .collect();
            texts.t(&gafira_doc::msg!(
                "bulk.header_option",
                row = Count(r as u64 + 1),
                words = Text(words.join(", "))
            ))
        };
        let shown = match pending.header {
            Some(r) => row_label(r, &pending.grid),
            None => plain(texts, "bulk.no_header"),
        };
        let mut header = pending.header;
        egui::ComboBox::from_id_salt("bulk.header")
            .width(ui.available_width() - 16.0)
            .selected_text(shown)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut header, None, plain(texts, "bulk.no_header"));
                for r in 0..pending.grid.len().min(20) {
                    ui.selectable_value(&mut header, Some(r), row_label(r, &pending.grid));
                }
            });
        pending.choose(header, texts);
        let imported = &pending.imported;
        ui.add_space(8.0);
        // The fields and the first rows, as they will come in.
        egui::ScrollArea::horizontal()
            .id_salt("bulk.pending")
            .show(ui, |ui| {
                egui::Grid::new("bulk.pending.grid")
                    .striped(true)
                    .spacing(Vec2::new(12.0, 4.0))
                    .show(ui, |ui| {
                        for column in imported.set.columns.iter().take(8) {
                            ui.label(RichText::new(&column.name).strong());
                        }
                        ui.end_row();
                        for row in imported.set.rows.iter().take(5) {
                            for value in row.values.iter().take(8) {
                                let short: String = value.chars().take(24).collect();
                                ui.label(short);
                            }
                            ui.end_row();
                        }
                    });
            });
        ui.add_space(8.0);
        let set = &imported.set;
        hint(
            ui,
            theme,
            &texts.t(&gafira_doc::msg!(
                "bulk.summary",
                rows = Count(set.rows.len() as u64),
                fields = Count(set.columns.len() as u64)
            )),
        );
        let warn = |ui: &mut Ui, count: usize, key: gafira_doc::msg::Message| {
            if count > 0 {
                ui.label(RichText::new(texts.t(&key)).color(theme.c("warning.fg")));
            }
        };
        warn(
            ui,
            imported.empty_rows,
            gafira_doc::msg!("bulk.empty_rows", count = Count(imported.empty_rows as u64)),
        );
        warn(
            ui,
            imported.uneven_rows,
            gafira_doc::msg!(
                "bulk.uneven_rows",
                count = Count(imported.uneven_rows as u64)
            ),
        );
        warn(
            ui,
            imported.cut_rows + imported.cut_columns,
            gafira_doc::msg!(
                "bulk.cut",
                rows = Count(imported.cut_rows as u64),
                columns = Count(imported.cut_columns as u64)
            ),
        );
        ui.add_space(10.0);
        let mut done = None;
        ui.horizontal(|ui| {
            let load = texts.t(&gafira_doc::msg!(
                "bulk.load",
                count = Count(set.rows.len() as u64)
            ));
            let enabled = !set.columns.is_empty();
            let button = ui.add_enabled_ui(enabled, |ui| widgets::primary(ui, theme, load));
            if button.inner.clicked() {
                done = Some(true);
            }
            if widgets::secondary(ui, theme, plain(texts, "bulk.cancel")).clicked() {
                done = Some(false);
            }
        });
        match done {
            Some(true) => {
                asks.data = self.pending.take().map(|p| Some(p.table()));
                self.row = 0;
                self.step = Step::Connect;
            }
            Some(false) => self.pending = None,
            None => {}
        }
    }

    // ---- Step 2: Conectar ----------------------------------------------

    fn connect(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        texts: &Texts,
        view: &View<'_>,
        set: &DataSet,
        asks: &mut Asks,
    ) {
        instructions(ui, theme, texts, &["bulk.connect_1", "bulk.connect_2"]);
        let example = set.columns.first().map_or("fila", |c| c.key.as_str());
        let example = gafira_bulk::token(&FieldRef::Key(example.to_owned()));
        hint(
            ui,
            theme,
            &texts.t(&gafira_doc::msg!("bulk.connect_3", example = Text(example))),
        );
        ui.add_space(10.0);
        let pages = gafira_bulk::template_pages(view.doc, set);
        let usage = gafira_bulk::usage(view.doc, &pages);
        // What «Reemplazar» and «Añadir al final» act on (owner report).
        let target = match &view.selected {
            Some(name) => texts.t(&gafira_doc::msg!(
                "bulk.selected_text",
                name = Text(name.clone())
            )),
            None => plain(texts, "bulk.no_selected_text"),
        };
        card(ui, theme, |ui| {
            let color = if view.can_fill {
                "text"
            } else {
                "text.secondary"
            };
            ui.label(
                RichText::new(target)
                    .font(theme.font("body-sm"))
                    .color(theme.c(color)),
            );
        });
        ui.add_space(8.0);
        ui.add(widgets::field(
            &mut self.search,
            plain(texts, "bulk.search"),
        ));
        ui.add_space(8.0);
        let row = self.row.min(set.rows.len().saturating_sub(1));
        let wanted = fold(&self.search);
        for column in &set.columns {
            if !wanted.is_empty()
                && !fold(&column.name).contains(&wanted)
                && !fold(&column.key).contains(&wanted)
            {
                continue;
            }
            let field = FieldRef::Key(column.key.clone());
            let uses = usage.get(&field).copied().unwrap_or(0)
                + usage
                    .get(&FieldRef::Name(column.name.clone()))
                    .copied()
                    .unwrap_or(0);
            let example = set.value(row, column.id).unwrap_or_default();
            let copied = self.copied.as_deref() == Some(column.key.as_str());
            if chip(
                ui,
                theme,
                texts,
                view,
                (column, example, uses),
                copied,
                asks,
            ) {
                self.copied = Some(column.key.clone());
            }
            ui.add_space(6.0);
        }
        // The row number is a field too.
        let numbered = DataColumn::new(0, &plain(texts, "bulk.row_number"), "fila");
        let uses = usage
            .get(&FieldRef::Key("fila".to_owned()))
            .copied()
            .unwrap_or(0);
        let copied = self.copied.as_deref() == Some("fila");
        if chip(
            ui,
            theme,
            texts,
            view,
            (&numbered, &(row + 1).to_string(), uses),
            copied,
            asks,
        ) {
            self.copied = Some("fila".to_owned());
        }
        ui.add_space(10.0);
        problems(ui, theme, texts, view.doc, set, &pages, asks);
        ui.add_space(12.0);
        if widgets::primary(ui, theme, plain(texts, "bulk.next_review")).clicked() {
            self.step = Step::Review;
        }
    }

    // ---- Step 3: Revisar -----------------------------------------------

    /// A row as it will look, what stops an export or deserves a look, and
    /// «Exportar en lote…», which opens the dialog with the formats.
    fn review(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        texts: &Texts,
        view: &View<'_>,
        set: &DataSet,
        asks: &mut Asks,
    ) {
        let pages = gafira_bulk::template_pages(view.doc, set);
        self.preview(ui, theme, texts, view, set, &pages, asks);
        ui.add_space(12.0);
        let all: Vec<usize> = (0..set.rows.len()).collect();
        let blocked = problems(ui, theme, texts, view.doc, set, &pages, asks);
        empty_values(ui, theme, texts, view.doc, set, &pages, &all);
        ui.add_space(12.0);
        let width = ui.available_width();
        let export =
            action(theme, plain(texts, "bulk.export"), true).min_size(Vec2::new(width, 40.0));
        let response = ui.add_enabled(!blocked, export);
        if response
            .on_hover_text(plain(texts, "bulk.export_hint"))
            .clicked()
        {
            asks.export = true;
        }
    }

    /// The picture of a row and «‹ Fila 3 de 120 ›» (§9).
    #[allow(clippy::too_many_arguments)]
    fn preview(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        texts: &Texts,
        view: &View<'_>,
        set: &DataSet,
        pages: &[PageId],
        asks: &mut Asks,
    ) {
        let total = set.rows.len();
        if total == 0 || pages.is_empty() {
            return;
        }
        self.row = self.row.min(total - 1);
        self.page = self.page.min(pages.len() - 1);
        let page = pages[self.page];
        let key = picture_key(view.doc, set, self.row, page);
        if let Some((of, pixels)) = self.picture.take_if(|(of, _)| *of == key) {
            let options = egui::TextureOptions::LINEAR;
            self.preview = Some((of, ui.ctx().load_texture("bulk-preview", pixels, options)));
        }
        if self.preview.as_ref().is_none_or(|(of, _)| *of != key) {
            asks.preview = Some((key, self.row, page));
        }
        let side = ui.available_width();
        // A click opens the row as large as the window allows.
        let (rect, response) = ui.allocate_exact_size(Vec2::new(side, side * 0.7), Sense::click());
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ZoomIn);
        }
        if response
            .on_hover_text(plain(texts, "bulk.zoom_hint"))
            .clicked()
        {
            self.zoom = true;
        }
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, theme.radius_md(), theme.c("surface.sunken"));
        match &self.preview {
            Some((of, texture)) if *of == key => {
                let size = texture.size_vec2();
                let room = rect.shrink(8.0);
                let k = (room.width() / size.x).min(room.height() / size.y);
                let place = egui::Rect::from_center_size(rect.center(), size * k);
                let uv = egui::Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                painter.image(texture.id(), place, uv, egui::Color32::WHITE);
            }
            _ => {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    plain(texts, "bulk.drawing"),
                    theme.font("caption"),
                    theme.c("text.secondary"),
                );
            }
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui
                .button("«")
                .on_hover_text(plain(texts, "bulk.first"))
                .clicked()
            {
                self.row = 0;
            }
            if ui
                .button("‹")
                .on_hover_text(plain(texts, "bulk.previous"))
                .clicked()
            {
                self.row = self.row.saturating_sub(1);
            }
            ui.label(texts.t(&gafira_doc::msg!(
                "bulk.row_of",
                row = Count(self.row as u64 + 1),
                total = Count(total as u64)
            )));
            if ui
                .button("›")
                .on_hover_text(plain(texts, "bulk.next"))
                .clicked()
            {
                self.row = (self.row + 1).min(total - 1);
            }
            if ui
                .button("»")
                .on_hover_text(plain(texts, "bulk.last"))
                .clicked()
            {
                self.row = total - 1;
            }
        });
        if pages.len() > 1 {
            ui.horizontal(|ui| {
                if ui.button("‹").clicked() {
                    self.page = self.page.saturating_sub(1);
                }
                ui.label(texts.t(&gafira_doc::msg!(
                    "bulk.page_of",
                    page = Count(self.page as u64 + 1),
                    total = Count(pages.len() as u64)
                )));
                if ui.button("›").clicked() {
                    self.page = (self.page + 1).min(pages.len() - 1);
                }
            });
        }
        ui.add_space(4.0);
        if ui
            .add(small(theme, plain(texts, "bulk.zoom")))
            .on_hover_text(plain(texts, "bulk.zoom_hint"))
            .clicked()
        {
            self.zoom = true;
        }
        hint(ui, theme, &plain(texts, "bulk.preview_hint"));
    }

    /// The row as large as the window allows, to look at it closely: ← and
    /// → go through the rows, Esc closes it. The picture it needs, if any:
    /// its key, the row, the page and the size to draw it in.
    pub(crate) fn zoomed(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        document: Option<&Document>,
    ) -> Option<(u64, usize, PageId, (u32, u32))> {
        if !self.zoom {
            return None;
        }
        let Some((document, set)) = document.and_then(|d| Some((d, d.data()?))) else {
            self.zoom = false;
            return None;
        };
        let pages = gafira_bulk::template_pages(document, set);
        let total = set.rows.len();
        if total == 0 || pages.is_empty() {
            self.zoom = false;
            return None;
        }
        ctx.input(|i| {
            if i.key_pressed(egui::Key::ArrowRight) {
                self.row = (self.row + 1).min(total - 1);
            }
            if i.key_pressed(egui::Key::ArrowLeft) {
                self.row = self.row.saturating_sub(1);
            }
        });
        self.row = self.row.min(total - 1);
        self.page = self.page.min(pages.len() - 1);
        let page = pages[self.page];
        let screen = ctx.content_rect().size();
        let room = (screen - Vec2::new(96.0, 96.0)).max(Vec2::new(480.0, 360.0));
        // The picture, a little larger than its room, for sharp edges.
        let scale = ctx.pixels_per_point();
        let fit = (
            (room.x * scale).round().max(1.0) as u32,
            ((room.y - 72.0) * scale).round().max(1.0) as u32,
        );
        let key = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            picture_key(document, set, self.row, page).hash(&mut hasher);
            fit.hash(&mut hasher);
            hasher.finish()
        };
        if let Some((of, pixels)) = self.big_picture.take_if(|(of, _)| *of == key) {
            let options = egui::TextureOptions::LINEAR;
            self.big = Some((of, ctx.load_texture("bulk-zoom", pixels, options)));
        }
        let wanted = self
            .big
            .as_ref()
            .is_none_or(|(of, _)| *of != key)
            .then_some((key, self.row, page, fit));
        let frame = egui::Frame::popup(&ctx.style()).inner_margin(egui::Margin::same(16));
        let modal = egui::Modal::new(egui::Id::new("bulk_zoom")).frame(frame);
        let response = modal.show(ctx, |ui| {
            ui.set_width(room.x);
            ui.set_height(room.y);
            ui.horizontal(|ui| {
                if ui
                    .button("‹")
                    .on_hover_text(plain(texts, "bulk.previous"))
                    .clicked()
                {
                    self.row = self.row.saturating_sub(1);
                }
                ui.label(
                    RichText::new(texts.t(&gafira_doc::msg!(
                        "bulk.row_of",
                        row = Count(self.row as u64 + 1),
                        total = Count(total as u64)
                    )))
                    .font(theme.font("ui-label")),
                );
                if ui
                    .button("›")
                    .on_hover_text(plain(texts, "bulk.next"))
                    .clicked()
                {
                    self.row = (self.row + 1).min(total - 1);
                }
                if pages.len() > 1 {
                    ui.add_space(16.0);
                    if ui.button("‹").clicked() {
                        self.page = self.page.saturating_sub(1);
                    }
                    ui.label(texts.t(&gafira_doc::msg!(
                        "bulk.page_of",
                        page = Count(self.page as u64 + 1),
                        total = Count(pages.len() as u64)
                    )));
                    if ui.button("›").clicked() {
                        self.page = (self.page + 1).min(pages.len() - 1);
                    }
                }
                ui.add_space(16.0);
                hint(ui, theme, &plain(texts, "bulk.zoom_keys"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close = egui::Button::new(RichText::new("×").size(22.0)).frame(false);
                    if ui
                        .add(close)
                        .on_hover_text(plain(texts, "bulk.close"))
                        .clicked()
                    {
                        self.zoom = false;
                    }
                });
            });
            ui.add_space(8.0);
            let (rect, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, theme.radius_md(), theme.c("surface.sunken"));
            match &self.big {
                Some((of, texture)) if *of == key => {
                    let size = texture.size_vec2();
                    let room = rect.shrink(8.0);
                    let k = (room.width() / size.x).min(room.height() / size.y);
                    let place = egui::Rect::from_center_size(rect.center(), size * k);
                    let uv = egui::Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                    painter.image(texture.id(), place, uv, egui::Color32::WHITE);
                }
                _ => {
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        plain(texts, "bulk.drawing"),
                        theme.font("caption"),
                        theme.c("text.secondary"),
                    );
                }
            }
        });
        if response.should_close() {
            self.zoom = false;
        }
        wanted.filter(|_| self.zoom)
    }

    // ---- The wide table ------------------------------------------------

    /// «Editar tabla», over everything: the table of the design, whose
    /// changes are kept as they are written («Listo» or «Cerrar» end it), or
    /// a new draft, which comes in only with «Usar esta tabla» («Cancelar»
    /// and closing it leave nothing).
    pub(crate) fn wide(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        set: Option<&DataSet>,
    ) -> Option<Option<DataSet>> {
        if !self.wide {
            return None;
        }
        let drafting = self.draft.is_some();
        let shown = match (&self.draft, set) {
            (Some(draft), _) => draft.clone(),
            (None, Some(set)) => set.clone(),
            (None, None) => {
                self.wide = false;
                return None;
            }
        };
        let mut out = None;
        let mut close = false;
        let mut accept = false;
        let screen = ctx.content_rect().size();
        let room = (screen - Vec2::new(120.0, 120.0)).max(Vec2::new(480.0, 320.0));
        let frame = egui::Frame::popup(&ctx.style()).inner_margin(egui::Margin::same(24));
        let modal = egui::Modal::new(egui::Id::new("bulk_data")).frame(frame);
        let response = modal.show(ctx, |ui| {
            ui.set_width(room.x);
            ui.set_height(room.y);
            ui.horizontal(|ui| {
                let title = if drafting {
                    "bulk.new_table"
                } else {
                    "bulk.edit_table"
                };
                ui.label(RichText::new(plain(texts, title)).font(theme.font("h3")));
                ui.label(
                    RichText::new(texts.t(&gafira_doc::msg!(
                        "bulk.summary",
                        rows = Count(shown.rows.len() as u64),
                        fields = Count(shown.columns.len() as u64)
                    )))
                    .color(theme.c("text.secondary")),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let x = egui::Button::new(RichText::new("×").size(22.0)).frame(false);
                    let hint = if drafting {
                        "bulk.cancel"
                    } else {
                        "bulk.close"
                    };
                    if ui.add(x).on_hover_text(plain(texts, hint)).clicked() {
                        close = true;
                    }
                    if drafting {
                        if widgets::primary(ui, theme, plain(texts, "bulk.use_table")).clicked() {
                            accept = true;
                        }
                        if ui
                            .add(action(theme, plain(texts, "bulk.cancel"), false))
                            .clicked()
                        {
                            close = true;
                        }
                    } else if widgets::primary(ui, theme, plain(texts, "bulk.done")).clicked() {
                        close = true;
                    }
                });
            });
            hint(ui, theme, &plain(texts, "bulk.table_hint"));
            ui.add_space(8.0);
            let mut cells = DataCells::new(&shown, texts);
            let tall = ui.available_height() - 8.0;
            let salt = ("bulk", "wide");
            data_grid::grid(
                ui,
                theme,
                &mut cells,
                &mut self.wide_focus,
                (tall, 170.0, salt),
            );
            out = cells.out;
        });
        if response.should_close() {
            close = true;
        }
        if drafting {
            return self.end_draft(out, accept, close);
        }
        if close {
            self.wide = false;
        }
        out.map(Some)
    }
}

// ---- Writing `{{` in a text (§7.3 rules 1 and 2) -------------------------

/// The field being written before the cursor (a char index): the byte where
/// its `{{` starts and the letters after it; `None` when the cursor is not
/// inside an open `{{`.
pub(crate) fn partial_field(text: &str, cursor: usize) -> Option<(usize, String)> {
    let end = text
        .char_indices()
        .nth(cursor)
        .map_or(text.len(), |(at, _)| at);
    let before = &text[..end];
    let start = before.rfind("{{")?;
    let partial = &before[start + 2..];
    let simple = partial
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_');
    let escaped = before[..start].ends_with('\\');
    (simple && !escaped).then(|| (start, partial.to_owned()))
}

/// The columns whose key or name match `partial`, without case or accents,
/// those that start with it first; at most `most`.
pub(crate) fn matching(
    columns: &[(String, String)],
    partial: &str,
    most: usize,
) -> Vec<(String, String)> {
    let wanted = fold(partial);
    let mut starts = Vec::new();
    let mut inside = Vec::new();
    for (key, name) in columns {
        let (k, n) = (fold(key), fold(name));
        if wanted.is_empty() || k.starts_with(&wanted) || n.starts_with(&wanted) {
            starts.push((key.clone(), name.clone()));
        } else if k.contains(&wanted) || n.contains(&wanted) {
            inside.push((key.clone(), name.clone()));
        }
    }
    starts.extend(inside);
    starts.truncate(most);
    starts
}

/// `text` with the field written from byte `start` up to the cursor (a
/// char index) completed as `{{key}}`; the cursor after it, as a char index.
pub(crate) fn complete_field(
    text: &str,
    (start, cursor): (usize, usize),
    key: &str,
) -> (String, usize) {
    let end = text
        .char_indices()
        .nth(cursor)
        .map_or(text.len(), |(at, _)| at);
    let rest = &text[end..];
    // A `}}` already there is not written twice.
    let rest = rest.strip_prefix("}}").unwrap_or(rest);
    let token = gafira_bulk::token(&FieldRef::Key(key.to_owned()));
    let out = format!("{}{token}{rest}", &text[..start]);
    let at = text[..start].chars().count() + token.chars().count();
    (out, at)
}

impl Panel {
    /// The draft after a frame of the wide grid: its changes kept, put in the
    /// design with «Usar esta tabla», dropped with «Cancelar».
    fn end_draft(
        &mut self,
        changed: Option<DataSet>,
        accept: bool,
        close: bool,
    ) -> Option<Option<DataSet>> {
        if let Some(next) = changed {
            self.draft = Some(next);
        }
        if accept {
            self.wide = false;
            self.row = 0;
            self.step = Step::Data;
            return self.draft.take().map(Some);
        }
        if close {
            self.wide = false;
            self.draft = None;
        }
        None
    }
}

/// A key of the picture of `row` on `page`: it changes with the page, its
/// elements and the table, which are shared until they change.
pub(crate) fn picture_key(doc: &Document, set: &DataSet, row: usize, page: PageId) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let shown = doc.pages().iter().find(|p| p.id == page);
    shown.map(|p| Arc::as_ptr(p) as usize).hash(&mut hasher);
    doc.data_arc()
        .map(|d| Arc::as_ptr(d) as usize)
        .hash(&mut hasher);
    std::ptr::from_ref(set).hash(&mut hasher);
    (row, page.get()).hash(&mut hasher);
    hasher.finish()
}

/// `text` to compare without case or accents (§7.3 rule 2).
fn fold(text: &str) -> String {
    let taken = std::collections::BTreeSet::new();
    let key = gafira_bulk::key_for(text.trim(), &taken);
    if key == "col" && text.trim().is_empty() {
        String::new()
    } else {
        key
    }
}

/// A new table to write in: two columns and three empty rows.
fn new_table(texts: &Texts) -> DataSet {
    let names = [
        plain(texts, "bulk.new_name"),
        plain(texts, "bulk.new_second"),
    ];
    let mut taken = std::collections::BTreeSet::new();
    let columns: Vec<DataColumn> = names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let key = gafira_bulk::key_for(name, &taken);
            taken.insert(key.clone());
            DataColumn {
                id: i as u64 + 1,
                name: name.clone(),
                key,
            }
        })
        .collect();
    let rows = (0..3)
        .map(|k| Arc::new(DataRow::new(3 + k, vec![String::new(); 2])))
        .collect();
    DataSet {
        name: plain(texts, "bulk.new_table"),
        source: Some(DataSource {
            kind: "manual".to_owned(),
            ..DataSource::default()
        }),
        columns,
        rows,
        next_id: 6,
        ..DataSet::default()
    }
}

/// A field: its head (name, field and the value of the row) is dragged to
/// the design; under it, its buttons, which never start a drag (owner
/// report: a drag over the whole card took their clicks). Whether «Copiar»
/// was pressed.
fn chip(
    ui: &mut Ui,
    theme: &Theme,
    texts: &Texts,
    view: &View<'_>,
    (column, example, uses): (&DataColumn, &str, usize),
    copied: bool,
    asks: &mut Asks,
) -> bool {
    let mut copy = false;
    let token = gafira_bulk::token(&FieldRef::Key(column.key.clone()));
    let width = ui.available_width();
    let frame = egui::Frame::NONE
        .fill(theme.c("surface"))
        .stroke(egui::Stroke::new(1.0, theme.c("border")))
        .corner_radius(theme.radius_md())
        .inner_margin(egui::Margin::same(8));
    frame.show(ui, |ui| {
        // The inner margin and the line on each side: no wider than the room.
        ui.set_width(width - 18.0);
        let head = ui.scope(|ui| {
            ui.horizontal(|ui| {
                let grip = crate::icons::image(
                    ui.ctx(),
                    crate::icons::Icon::Move,
                    crate::icons::Weight::Regular,
                    14.0,
                    theme.c("text.secondary"),
                );
                ui.add(grip);
                ui.label(RichText::new(&column.name).font(theme.font("ui-label")));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let used = texts.t(&gafira_doc::msg!("bulk.uses", count = Count(uses as u64)));
                    ui.label(
                        RichText::new(used)
                            .font(theme.font("caption"))
                            .color(theme.c("text.secondary")),
                    );
                });
            });
            ui.label(
                RichText::new(&token)
                    .font(theme.font("caption"))
                    .color(theme.c("accent")),
            );
            let short: String = example.chars().take(40).collect();
            let example = if short.is_empty() {
                plain(texts, "bulk.empty_value")
            } else {
                short
            };
            ui.label(
                RichText::new(example)
                    .font(theme.font("body-sm"))
                    .color(theme.c("text.secondary")),
            );
        });
        // Only the head carries the field (spec 061).
        let response = ui.interact(
            head.response.rect,
            ui.id().with(("bulk.chip", &column.key)),
            Sense::drag(),
        );
        if response.hovered() && !response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        let response = response.on_hover_text(plain(texts, "bulk.drag_hint"));
        let at = ui.input(|i| i.pointer.latest_pos());
        if response.dragged()
            && let Some(at) = at
        {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            let painter = ui.ctx().layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::new("bulk.ghost"),
            ));
            painter.text(
                at,
                egui::Align2::CENTER_CENTER,
                &token,
                theme.font("ui-label"),
                theme.c("accent"),
            );
        }
        if response.drag_stopped()
            && let Some(at) = at
            && let Some((page, point)) = (view.drop_point)(at)
        {
            asks.drop = Some((token.clone(), page, point));
        }
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if ui
                .add(small(theme, plain(texts, "bulk.add_text")))
                .on_hover_text(plain(texts, "bulk.add_text_hint"))
                .clicked()
            {
                asks.insert = Some(token.clone());
            }
            let fill = ui.add_enabled(
                view.can_fill,
                small(theme, plain(texts, "bulk.replace_selected")),
            );
            if fill
                .on_hover_text(plain(texts, "bulk.replace_hint"))
                .on_disabled_hover_text(plain(texts, "bulk.select_text"))
                .clicked()
            {
                asks.replace = Some(token.clone());
            }
            let add = ui.add_enabled(
                view.can_fill,
                small(theme, plain(texts, "bulk.append_selected")),
            );
            if add
                .on_hover_text(plain(texts, "bulk.append_hint"))
                .on_disabled_hover_text(plain(texts, "bulk.select_text"))
                .clicked()
            {
                asks.append = Some(token.clone());
            }
            let key = if copied { "bulk.copied" } else { "bulk.copy" };
            if ui
                .add(small(theme, plain(texts, key)))
                .on_hover_text(plain(texts, "bulk.copy_hint"))
                .clicked()
            {
                ui.ctx().copy_text(token.clone());
                copy = true;
            }
        });
    });
    copy
}

/// The fields with no column: they stop a run (§9). Whether there is any.
fn problems(
    ui: &mut Ui,
    theme: &Theme,
    texts: &Texts,
    doc: &Document,
    set: &DataSet,
    pages: &[PageId],
    asks: &mut Asks,
) -> bool {
    let unknown = gafira_bulk::unknown(doc, set, pages);
    let mut blocked = !unknown.is_empty();
    if set.rows.is_empty() {
        blocked = true;
        ui.label(RichText::new(plain(texts, "bulk.no_rows")).color(theme.c("error.fg")));
    }
    if pages.is_empty() {
        blocked = true;
        ui.label(RichText::new(plain(texts, "bulk.no_pages")).color(theme.c("error.fg")));
    }
    if unknown.is_empty() {
        return blocked;
    }
    ui.label(RichText::new(plain(texts, "bulk.unknown")).color(theme.c("error.fg")));
    let mut seen = std::collections::BTreeSet::new();
    for (page, node, field) in unknown {
        if !seen.insert((page, node, field.clone())) {
            continue;
        }
        let number = doc.pages().iter().position(|p| p.id == page).unwrap_or(0) + 1;
        let element = doc
            .page(page)
            .and_then(|p| p.node(node))
            .map(crate::bulk_run::element_name)
            .unwrap_or_default();
        ui.horizontal_wrapped(|ui| {
            ui.label(texts.t(&gafira_doc::msg!(
                "bulk.unknown_at",
                field = Text(gafira_bulk::token(&field)),
                page = Count(number as u64),
                element = Text(element)
            )));
            if ui.add(small(theme, plain(texts, "bulk.go_to"))).clicked() {
                asks.go_to = Some((page, node));
            }
        });
    }
    blocked
}

/// The rows with an empty value in a field the design uses: a warning,
/// not a block (§6: an empty value stays empty).
pub(crate) fn empty_values(
    ui: &mut Ui,
    theme: &Theme,
    texts: &Texts,
    doc: &Document,
    set: &DataSet,
    pages: &[PageId],
    rows: &[usize],
) {
    let used: Vec<u64> = gafira_bulk::usage(doc, pages)
        .keys()
        .filter_map(|field| match field {
            FieldRef::Key(key) => set.column_by_key(key),
            FieldRef::Name(name) => set.column_by_name(name),
        })
        .map(|c| c.id)
        .collect();
    if used.is_empty() {
        return;
    }
    let empty: Vec<usize> = rows
        .iter()
        .copied()
        .filter(|row| {
            used.iter()
                .any(|id| set.value(*row, *id).is_some_and(|v| v.trim().is_empty()))
        })
        .collect();
    if empty.is_empty() {
        return;
    }
    let first: Vec<String> = empty.iter().take(5).map(|r| (r + 1).to_string()).collect();
    ui.label(
        RichText::new(texts.t(&gafira_doc::msg!(
            "bulk.empty_values",
            count = Count(empty.len() as u64),
            rows = Text(first.join(", "))
        )))
        .color(theme.c("warning.fg")),
    );
}

/// The phase, the counts, the bar, the file and «Cancelar» (§10.3).
pub(crate) fn progress_card(
    ui: &mut Ui,
    theme: &Theme,
    texts: &Texts,
    status: &Status,
    cancelling: bool,
    cancel: &mut bool,
) {
    card(ui, theme, |ui| {
        let phase = match status.phase {
            Phase::Preparing => "bulk.phase_preparing",
            Phase::Exporting => "bulk.phase_exporting",
            Phase::Saving => "bulk.phase_saving",
            Phase::Done => "bulk.phase_done",
            Phase::Cancelled => "bulk.phase_cancelled",
        };
        ui.horizontal(|ui| {
            ui.add(egui::Spinner::new().size(16.0).color(theme.c("accent")));
            ui.label(RichText::new(plain(texts, phase)).font(theme.font("ui-label")));
        });
        ui.add(egui::ProgressBar::new(status.fraction()).show_percentage());
        let left = status.total.saturating_sub(status.done);
        ui.label(texts.t(&gafira_doc::msg!(
            "bulk.counts",
            done = Count(status.done as u64),
            total = Count(status.total as u64),
            failed = Count(status.failed as u64),
            left = Count(left as u64)
        )));
        if !status.current.is_empty() {
            let key = if status.phase == Phase::Preparing {
                gafira_doc::msg!("bulk.now_row", row = Text(status.current.clone()))
            } else {
                gafira_doc::msg!("bulk.now_file", file = Text(status.current.clone()))
            };
            ui.label(RichText::new(texts.t(&key)).font(theme.font("caption")));
        }
        let elapsed = status.started.elapsed().as_secs();
        let time = match status.left() {
            Some(left) => texts.t(&gafira_doc::msg!(
                "bulk.time_left",
                elapsed = Text(clock(elapsed)),
                left = Text(clock(left.as_secs()))
            )),
            None => texts.t(&gafira_doc::msg!(
                "bulk.time",
                elapsed = Text(clock(elapsed))
            )),
        };
        ui.label(
            RichText::new(time)
                .font(theme.font("caption"))
                .color(theme.c("text.secondary")),
        );
        ui.add_space(4.0);
        if cancelling {
            ui.label(plain(texts, "bulk.cancelling"));
        } else if widgets::secondary(ui, theme, plain(texts, "bulk.cancel_run")).clicked() {
            *cancel = true;
        }
    });
}

/// A button of the panel: coral with ink words for the main action
/// (DIS-15), else outlined in ink on the surface, so neither looks like the
/// boxes of text around it.
fn action(theme: &Theme, text: String, main: bool) -> egui::Button<'static> {
    let (fill, words, line) = if main {
        ("primary.bg", "primary.fg", None)
    } else {
        ("surface", "text", Some("text"))
    };
    let button = egui::Button::new(
        RichText::new(text)
            .font(theme.font("ui-label"))
            .color(theme.c(words)),
    )
    .fill(theme.c(fill))
    .corner_radius(theme.radius_md());
    match line {
        Some(line) => button.stroke(egui::Stroke::new(1.0, theme.c(line))),
        None => button,
    }
}

/// A small outlined button, for the actions of a field.
fn small(theme: &Theme, text: String) -> egui::Button<'static> {
    let button = action(theme, text, false);
    button.min_size(Vec2::new(0.0, 26.0))
}

/// `seconds` as `m:ss`, or `h:mm:ss`.
pub(crate) fn clock(seconds: u64) -> String {
    let (h, m, s) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

fn hint(ui: &mut Ui, theme: &Theme, words: &str) {
    ui.label(
        RichText::new(words)
            .font(theme.font("caption"))
            .color(theme.c("text.secondary")),
    );
}

/// Numbered steps, for a person who never did this before.
fn instructions(ui: &mut Ui, theme: &Theme, texts: &Texts, keys: &[&'static str]) {
    card(ui, theme, |ui| {
        for (k, key) in keys.iter().enumerate() {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!("{}.", k + 1))
                        .font(theme.font("ui-label"))
                        .color(theme.c("accent")),
                );
                ui.label(RichText::new(plain(texts, key)).font(theme.font("body-sm")));
            });
        }
    });
}

fn card(ui: &mut Ui, theme: &Theme, inside: impl FnOnce(&mut Ui)) {
    let width = ui.available_width();
    egui::Frame::NONE
        .fill(theme.c("surface.sunken"))
        .corner_radius(theme.radius_md())
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(width - 20.0);
            inside(ui);
        });
}

/// The connected data as the cells of the grid: line 0 holds the names of
/// the columns; writing past the data adds columns and rows (§6).
struct DataCells<'a> {
    set: &'a DataSet,
    texts: &'a Texts,
    /// The table with the changes of this frame.
    out: Option<DataSet>,
}

impl<'a> DataCells<'a> {
    fn new(set: &'a DataSet, texts: &'a Texts) -> Self {
        Self {
            set,
            texts,
            out: None,
        }
    }

    fn current(&self) -> &DataSet {
        self.out.as_ref().unwrap_or(self.set)
    }
}

impl Cells for DataCells<'_> {
    fn size(&self) -> (usize, usize) {
        let set = self.current();
        (set.columns.len(), set.rows.len() + 1)
    }

    fn most(&self) -> (usize, usize) {
        (DataSet::MAX_COLUMNS, DataSet::MAX_ROWS + 1)
    }

    fn longest(&self) -> usize {
        DataSet::MAX_CELL_TEXT / 4
    }

    fn cell(&self, row: usize, col: usize) -> &str {
        let set = self.current();
        match row {
            0 => set.columns.get(col).map_or("", |c| c.name.as_str()),
            _ => set
                .rows
                .get(row - 1)
                .and_then(|r| r.values.get(col))
                .map_or("", String::as_str),
        }
    }

    fn set(&mut self, row: usize, col: usize, words: &str) {
        let unnamed = unnamed(self.texts);
        let set = self.out.get_or_insert_with(|| self.set.clone());
        if col >= DataSet::MAX_COLUMNS || row > DataSet::MAX_ROWS {
            return;
        }
        while set.columns.len() <= col {
            let name = unnamed(set.columns.len());
            let taken = set.columns.iter().map(|c| c.key.clone()).collect();
            let key = gafira_bulk::key_for(&name, &taken);
            set.columns.push(DataColumn {
                id: set.next_id,
                name,
                key,
            });
            set.next_id += 1;
            for r in &mut set.rows {
                Arc::make_mut(r).values.push(String::new());
            }
        }
        if row == 0 {
            // The key stays: the fields that name it keep working.
            set.columns[col].name = words.to_owned();
            return;
        }
        while set.rows.len() < row {
            let values = vec![String::new(); set.columns.len()];
            set.rows.push(Arc::new(DataRow::new(set.next_id, values)));
            set.next_id += 1;
        }
        let mut value = words.to_owned();
        if value.len() > DataSet::MAX_CELL_TEXT {
            let mut end = DataSet::MAX_CELL_TEXT;
            while !value.is_char_boundary(end) {
                end -= 1;
            }
            value.truncate(end);
        }
        Arc::make_mut(&mut set.rows[row - 1]).values[col] = value;
    }

    fn column_head(&mut self, ui: &mut Ui, theme: &Theme, rect: egui::Rect, col: usize) {
        let key = self.current().columns.get(col).map(|c| c.key.clone());
        let words = match key {
            Some(key) => format!("{} · {key}", gafira_bulk::column_letter(col)),
            None => gafira_bulk::column_letter(col),
        };
        data_grid::head_words(ui, theme, rect, &words);
    }

    fn row_head(&mut self, ui: &mut Ui, theme: &Theme, rect: egui::Rect, row: usize) {
        let words = if row == 0 {
            String::new()
        } else {
            row.to_string()
        };
        data_grid::head_words(ui, theme, rect, &words);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::bulk_run::Written;
    use gafira_doc::{Document, NodeKind, TextSizing};
    use gafira_ports::i18n::Locale;

    use super::*;
    use crate::bulk_run::{Phase, Run, Target};
    use crate::editor::tests::engine_over;
    use crate::format::StartFormat;
    use crate::shell::start;

    fn texts() -> Texts {
        Texts::new(Locale::Es)
    }

    fn grid(rows: &[&[&str]]) -> Grid {
        rows.iter()
            .map(|r| r.iter().map(|c| (*c).to_owned()).collect())
            .collect()
    }

    /// A folder of its own for the files of a test.
    fn folder(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("gafira-bulk-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a folder");
        dir
    }

    /// An engine with a design that says «Hola, {{nombre}}» and a table.
    fn designed(format: &str) -> crate::editor::tests::TestEngine {
        let repository = Arc::new(gafira_format::GafRepository::default());
        let mut engine = engine_over(repository);
        start(&mut engine, StartFormat::Social).expect("a document");
        let document = engine.edit().document().expect("a document");
        let page = &document.pages()[0];
        let size = page.size.to_px(document.dpi()).expect("px");
        let style = gafira_doc::SpanStyle {
            font: "Inter".to_owned(),
            size: 48.0,
            color: gafira_color::Color::Srgb(gafira_color::Rgba::new(0.0, 0.0, 0.0, 1.0)),
            bold: false,
            italic: false,
        };
        let page_px = Vec2::new(size.width() as f32, size.height() as f32);
        let spec = crate::canvas::new_text(page_px, "Hola, {{nombre}}", &style);
        engine
            .edit()
            .execute(Box::new(gafira_edit::InsertNodes::new(
                page.id,
                None,
                0,
                vec![spec],
            )))
            .expect("the text");
        let pending = Pending::new(
            grid(&[
                &["Nombre", "Curso"],
                &["Ana", "Rust"],
                &["Ana", "Go"],
                &["Luis", "C"],
            ]),
            DataSource::default(),
            "clientes".to_owned(),
            &texts(),
        );
        let mut set = pending.table();
        set.profile.format = format.to_owned();
        set.profile.file_name = "certificado_{{nombre}}".to_owned();
        engine
            .edit()
            .execute(Box::new(gafira_edit::SetData::new(Some(Arc::new(set)))))
            .expect("the data");
        engine
    }

    fn finish(mut run: Run) -> crate::bulk_run::Status {
        let start = std::time::Instant::now();
        while !run.poll() {
            assert!(start.elapsed().as_secs() < 120, "the run ends");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        run.status()
    }

    // S104-50: a PNG per row with safe names that never take one another's
    // place, and the editable `.gaf` with a page per row, its texts filled
    // and without the table (§10.2, AC 10, 16, 17).
    #[test]
    fn s104_50_images_and_the_editable_file() {
        let mut engine = designed("png");
        let document = engine.edit().document().expect("a document");
        let dir = folder("png");
        std::fs::write(dir.join("certificado_Luis.png"), b"there before").expect("a file");
        let gaf = dir.join("lote.gaf");
        let png = Written {
            extension: "png".to_owned(),
            multipage: false,
            one_file: true,
        };
        let job = crate::bulk_run::job(
            &engine,
            &document,
            (
                vec![0, 1, 2],
                Target::Folder(dir.clone()),
                Some(gaf.clone()),
            ),
            Some(("png", png, gafira_doc::contract::ParamValues::new())),
        )
        .expect("a job");
        let status = finish(Run::start(job, Arc::new(|| {})));
        assert_eq!(status.stopped, None);
        assert_eq!(status.phase, Phase::Done);
        assert_eq!((status.written, status.failed), (4, 0));
        for name in [
            "certificado_Ana.png",
            "certificado_Ana-2.png",
            "certificado_Luis-2.png",
        ] {
            let bytes = std::fs::read(dir.join(name)).expect(name);
            assert!(image::load_from_memory(&bytes).is_ok(), "{name}");
        }
        assert_eq!(
            std::fs::read(dir.join("certificado_Luis.png")).expect("kept"),
            b"there before"
        );
        use gafira_ports::repository::ProjectRepository as _;
        let loaded = gafira_format::GafRepository::default()
            .load(&gaf)
            .expect("it reads")
            .document;
        assert_eq!(loaded.pages().len(), 3);
        assert_eq!(loaded.data(), None);
        let words: Vec<String> = loaded
            .pages()
            .iter()
            .map(|p| match &p.nodes[&p.roots[0]].kind {
                NodeKind::Text(t) => t.content.plain_text(),
                _ => String::new(),
            })
            .collect();
        assert_eq!(words, ["Hola, Ana", "Hola, Ana", "Hola, Luis"]);
        // The template is as it was.
        let after = engine.edit().document().expect("a document");
        assert_eq!(after, document);
    }

    // S104-51: one PDF with every page, in order (AC 9).
    #[test]
    fn s104_51_one_combined_pdf() {
        let mut engine = designed("pdf");
        let document = engine.edit().document().expect("a document");
        let dir = folder("pdf");
        let path = dir.join("todos.pdf");
        let pdf = Written {
            extension: "pdf".to_owned(),
            multipage: true,
            one_file: true,
        };
        let job = crate::bulk_run::job(
            &engine,
            &document,
            (vec![0, 1, 2], Target::File(path.clone()), None),
            Some(("pdf", pdf, gafira_doc::contract::ParamValues::new())),
        )
        .expect("a job");
        let status = finish(Run::start(job, Arc::new(|| {})));
        assert_eq!(status.stopped, None);
        assert_eq!(status.written, 1);
        let bytes = std::fs::read(&path).expect("the pdf");
        assert!(bytes.starts_with(b"%PDF"));
    }

    // S104-52: a box that grows sideways follows a longer name; a fixed box
    // too small for it is told (§9 «Avisar»).
    #[test]
    fn s104_52_boxes_follow_their_words() {
        let mut engine = designed("png");
        let document = engine.edit().document().expect("a document");
        let page = &document.pages()[0];
        let mut node = (*page.nodes[&page.roots[0]]).clone();
        let before = node.base.placement;
        if let NodeKind::Text(text) = &mut node.kind {
            text.content = gafira_doc::RichText::plain(
                "Hola, Maximiliano Fernández de la Cruz",
                text.content.first_style().expect("a style"),
            );
        }
        assert!(!crate::bulk_run::fit_node(&mut node));
        assert!(node.base.placement.w > before.w + 10.0);
        if let NodeKind::Text(text) = &mut node.kind {
            text.sizing = TextSizing::Fixed;
        }
        node.base.placement.w = 40.0;
        node.base.placement.h = 10.0;
        assert!(crate::bulk_run::fit_node(&mut node));
    }

    // S104-53: writing past the data adds a field or a row; renaming a field
    // keeps its key, so the texts that name it keep working (§6, AC 3, 6).
    #[test]
    fn s104_53_the_table_grows_as_it_is_written() {
        let texts = texts();
        let set = new_table(&texts);
        let mut cells = DataCells::new(&set, &texts);
        cells.set(0, 0, "Nombre completo");
        cells.set(5, 3, "x");
        let out = cells.out.expect("changed");
        assert_eq!(out.columns.len(), 4);
        assert_eq!(out.columns[0].name, "Nombre completo");
        assert_eq!(out.columns[0].key, set.columns[0].key);
        assert_eq!(out.columns[3].name, "Columna D");
        assert_eq!(out.rows.len(), 5);
        assert_eq!(out.rows[4].values[3], "x");
        assert_eq!(out.problem(), None);
    }

    // S104-54: the row of names is suggested and can be changed; the table
    // keeps where it came from (§5.2).
    #[test]
    fn s104_54_choose_the_row_of_names() {
        let texts = texts();
        let rows = grid(&[&["Informe", ""], &["Nombre", "Curso"], &["Ana", "Rust"]]);
        let mut pending = Pending::new(rows, DataSource::default(), "x".to_owned(), &texts);
        assert_eq!(pending.header, Some(1));
        assert_eq!(pending.table().rows.len(), 1);
        assert_eq!(pending.table().source.and_then(|s| s.header_row), Some(2));
        pending.choose(None, &texts);
        let table = pending.table();
        assert_eq!(table.rows.len(), 3);
        assert_eq!(table.columns[0].name, "Columna A");
    }

    // S104-57: writing `{{` offers the fields; Tab writes the whole field
    // (§7.3 rules 1, 2 and 7).
    #[test]
    fn s104_57_completing_a_field() {
        assert_eq!(partial_field("Hola, {{no", 10), Some((6, "no".to_owned())));
        assert_eq!(partial_field("Hola, {{", 8), Some((6, String::new())));
        assert_eq!(partial_field("Hola, {{nombre}} y", 18), None);
        assert_eq!(partial_field("{{a b", 5), None);
        assert_eq!(partial_field("\\{{no", 5), None);
        assert_eq!(partial_field("Año {{nó", 8), None);
        let columns = vec![
            ("curso".to_owned(), "Curso".to_owned()),
            ("nombre".to_owned(), "Nombre".to_owned()),
            ("apellido_nombre".to_owned(), "Apellido y nombre".to_owned()),
        ];
        let found = matching(&columns, "NOM", 5);
        assert_eq!(found[0].0, "nombre");
        assert_eq!(found[1].0, "apellido_nombre");
        assert_eq!(matching(&columns, "", 2).len(), 2);
        assert_eq!(
            complete_field("Hola, {{no!", (6, 10), "nombre"),
            ("Hola, {{nombre}}!".to_owned(), 16)
        );
        assert_eq!(
            complete_field("Hola, {{no}}!", (6, 10), "nombre"),
            ("Hola, {{nombre}}!".to_owned(), 16)
        );
    }

    // S104-60: «Crear tabla» is a draft: «Cancelar» leaves the design as
    // it was; only «Usar esta tabla» puts it in (owner request).
    #[test]
    fn s104_60_a_new_table_is_a_draft_until_used() {
        let texts = texts();
        let draft = || Panel {
            draft: Some(new_table(&texts)),
            wide: true,
            ..Panel::default()
        };
        // Writing keeps the draft and puts nothing in the design.
        let mut panel = draft();
        let mut written = new_table(&texts);
        written.name = "Invitados".to_owned();
        assert_eq!(panel.end_draft(Some(written), false, false), None);
        assert_eq!(
            panel.draft.as_ref().map(|d| d.name.as_str()),
            Some("Invitados")
        );
        // «Cancelar»: nothing comes in and the window closes.
        assert_eq!(panel.end_draft(None, false, true), None);
        assert!(panel.draft.is_none() && !panel.wide);
        // «Usar esta tabla»: the table comes in, once.
        let mut panel = draft();
        let used = panel.end_draft(None, true, false);
        assert!(matches!(used, Some(Some(ref set)) if set.columns.len() == 2));
        assert!(panel.draft.is_none() && !panel.wide);
    }

    // S104-61: the panel keeps the width of the side panel frame after
    // frame, with or without data (owner report: it widened by itself).
    #[test]
    fn s104_61_the_panel_does_not_widen_itself() {
        let texts = texts();
        let theme = theme();
        // Like the sample of the owner: five fields, long names.
        let rows = grid(&[
            &["Nombre", "Curso", "Fecha", "Código", "Aprobado"],
            &[
                "Valentina Rodríguez de la Fuente",
                "Edición de video para redes",
                "24/11/2026",
                "00129",
                "Sí",
            ],
            &[
                "Ana María Pérez",
                "Diseño gráfico básico",
                "15/11/2026",
                "00125",
                "Sí",
            ],
        ]);
        let set = Pending::new(
            rows.clone(),
            DataSource::default(),
            "participantes".to_owned(),
            &texts,
        )
        .table();
        let with_data = Document::default()
            .with_data(Some(Arc::new(set)))
            .expect("valid");
        for (document, step) in [
            (Document::default(), Step::Data),
            (with_data.clone(), Step::Data),
            (with_data.clone(), Step::Connect),
            (with_data, Step::Review),
        ] {
            let ctx = egui::Context::default();
            theme.apply(&ctx);
            let mut panel = Panel {
                step,
                ..Panel::default()
            };
            // The first case also waits for its row of names.
            if document.data().is_none() {
                panel.pending = Some(Pending::new(
                    rows.clone(),
                    DataSource::default(),
                    "participantes".to_owned(),
                    &texts,
                ));
            }
            let mut widths = Vec::new();
            for _ in 0..12 {
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        Pos2::ZERO,
                        Vec2::new(1440.0, 900.0),
                    )),
                    ..Default::default()
                };
                let _ = ctx.run(input, |ctx| {
                    let shown = egui::SidePanel::left("bulk.test")
                        .default_width(320.0)
                        .width_range(280.0..=480.0)
                        .show(ctx, |ui| {
                            let none = |_: Pos2| None;
                            let view = View {
                                doc: &document,
                                can_fill: false,
                                selected: None,
                                drop_point: &none,
                            };
                            let _ = panel.show(ui, &theme, &texts, &view);
                        });
                    widths.push(shown.response.rect.width());
                });
            }
            // From the first frame on: no jump when the tab opens either.
            assert!(
                widths.iter().all(|w| (w - widths[0]).abs() < 0.5),
                "{step:?}: {widths:?}"
            );
        }
    }

    fn theme() -> Theme {
        Theme::new(gafira_theme::ThemeTokens::embedded().expect("tokens"))
    }

    // S104-55: «Exportar en lote» opens on the last choices of the
    // document, offers the formats of «Exportar» but the spreadsheet, and
    // takes every row or a range from 1.
    #[test]
    fn s104_55_the_bulk_export_dialog() {
        let mut engine = designed("pdf");
        let document = engine.edit().document().expect("a document");
        let mut set = document.data().expect("data").clone();
        set.profile.format = "png".to_owned();
        set.rows = (0..5)
            .map(|k| Arc::new(DataRow::new(100 + k, vec![String::new(); 2])))
            .collect();
        set.next_id = 200;
        let formats = crate::export::formats(&engine);
        let mut dialog = crate::bulk_export::Dialog::new(formats, &NoPresets, &set);
        let (id, written, _) = dialog.written().expect("a format");
        assert_eq!((id, written.extension.as_str()), ("png", "png"));
        assert!(!dialog.one_target_file(), "a file per row and page");
        assert_eq!(dialog.rows(&set), [0, 1, 2, 3, 4]);
        dialog.range = Some((2, 4));
        assert_eq!(dialog.rows(&set), [1, 2, 3]);
        dialog.range = Some((4, 99));
        assert_eq!(dialog.rows(&set), [3, 4]);
        assert_eq!(dialog.profile().format, "png");
        let mut only_gaf = set.clone();
        only_gaf.profile.format = "gaf".to_owned();
        let formats = crate::export::formats(&engine);
        let dialog = crate::bulk_export::Dialog::new(formats, &NoPresets, &only_gaf);
        assert!(dialog.written().is_none());
        assert!(dialog.one_target_file());
        assert_eq!(dialog.extension(), "gaf");
    }

    // S104-63: the dialog draws its preview, its choices and its summary
    // and waits for the person: nothing is written while it is open.
    #[test]
    fn s104_63_the_dialog_draws_and_waits() {
        let mut engine = designed("png");
        let document = engine.edit().document().expect("a document");
        let set = document.data().expect("data").clone();
        let formats = crate::export::formats(&engine);
        let mut dialog = crate::bulk_export::Dialog::new(formats, &NoPresets, &set);
        let ctx = egui::Context::default();
        let theme = theme();
        theme.apply(&ctx);
        let texts = texts();
        for _ in 0..3 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    Pos2::ZERO,
                    Vec2::new(1440.0, 900.0),
                )),
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                let answer =
                    crate::bulk_export::show(ctx, &theme, &texts, &mut engine, &mut dialog);
                assert_eq!(answer, crate::bulk_export::Answer::Open);
            });
        }
        assert_eq!(engine.edit().document().expect("a document"), document);
    }

    struct NoPresets;

    impl crate::export::ExportPresets for NoPresets {
        fn load(&self, _: &str) -> Option<Vec<(String, crate::export::PresetValue)>> {
            None
        }

        fn save(&self, _: &str, _: Option<Vec<(String, crate::export::PresetValue)>>) {}
    }
}
