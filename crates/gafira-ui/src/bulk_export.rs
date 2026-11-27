//! «Exportar en lote» (spec 104 §10, owner redesign after the review of
//! the panel): a dialog like «Exportar», with the preview of a row on the
//! left and, on the right, the format and its options (the same as the
//! export dialog, and the settings it remembers), the rows, the template
//! pages, the names of the files and the editable `.gaf`. The run then goes
//! on in a card over the bottom right of the window, where an export shows
//! its progress, and ends there with «Mostrar en carpeta».

use std::path::PathBuf;

use egui::{RichText, Ui, Vec2};
use gafira_bulk::FieldRef;
use gafira_doc::{BulkProfile, DataSet, Document, PageId};
use gafira_engine::Gafira;

use crate::bulk_run::{Note, Phase, Status, Target, Written};
use crate::export::{self, Format};
use crate::style::Theme;
use crate::text::Texts;
use crate::widgets;

fn plain(texts: &Texts, key: &'static str) -> String {
    crate::charts::plain(texts, key)
}

/// The width of the controls of the right column.
const WIDE: f32 = 240.0;

/// The id the dialog gives «Gafira editable», after the export formats.
const GAF: &str = "gaf";

/// The dialog while it is open.
pub(crate) struct Dialog {
    /// The export formats and their options, as «Exportar» has them.
    export: export::Dialog,
    /// «Gafira editable (.gaf)» chosen instead of an export format.
    gaf_only: bool,
    /// A format of several pages writes one file with every row.
    pub one_file: bool,
    /// Also write the editable `.gaf`.
    pub editable: bool,
    pub file_name: String,
    /// The rows, from 1, both included; none is every row.
    pub range: Option<(usize, usize)>,
    /// The template pages; empty is every visible page.
    pub pages: Vec<PageId>,
    /// The row and the template page of the preview.
    row: usize,
    page: usize,
    preview: Option<(u64, egui::TextureHandle)>,
}

/// What the person did with the dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    Open,
    Cancel,
    Export,
}

impl Dialog {
    /// The dialog over the formats of `formats` that write pages, with the
    /// settings `presets` keeps and the last export of `set`.
    pub(crate) fn new(
        formats: Vec<Format>,
        presets: &dyn export::ExportPresets,
        set: &DataSet,
    ) -> Self {
        // A spreadsheet of the tables of a design is not a design per row.
        let formats: Vec<Format> = formats.into_iter().filter(|f| f.id != "xlsx").collect();
        let at = formats
            .iter()
            .position(|f| f.id == set.profile.format)
            .unwrap_or(0);
        let mut dialog = export::Dialog::new(formats, None).with_presets(presets);
        dialog.pick(at);
        let profile = &set.profile;
        Self {
            export: dialog,
            gaf_only: profile.format == GAF,
            one_file: profile.one_file,
            editable: profile.editable,
            file_name: profile.file_name.clone(),
            range: None,
            pages: set.template_pages.clone(),
            row: 0,
            page: 0,
            preview: None,
        }
    }

    /// The chosen export format, none for only the `.gaf`.
    pub(crate) fn format(&self) -> Option<&Format> {
        (!self.gaf_only).then(|| self.export.format()).flatten()
    }

    /// The format, how it writes and its options, for the run.
    pub(crate) fn written(&self) -> Option<(&str, Written, gafira_doc::contract::ParamValues)> {
        let format = self.format()?;
        let written = Written {
            extension: match format.extension {
                "jpeg" => "jpg".to_owned(),
                other => other.to_owned(),
            },
            multipage: format.multipage,
            one_file: self.one_file,
        };
        Some((format.id, written, self.export.values.clone()))
    }

    /// The settings to keep for the format (spec 074).
    pub(crate) fn preset(&self) -> Option<(String, Option<export::Preset>)> {
        self.format().and(self.export.preset())
    }

    /// The export profile of the document after this export.
    pub(crate) fn profile(&self) -> BulkProfile {
        BulkProfile {
            format: self
                .format()
                .map_or_else(|| GAF.to_owned(), |f| f.id.to_owned()),
            one_file: self.one_file,
            file_name: self.file_name.clone(),
            editable: self.editable,
        }
    }

    /// Every page of every row goes in one file: a combined PDF or the
    /// `.gaf` alone.
    pub(crate) fn one_target_file(&self) -> bool {
        self.format().is_none_or(|f| f.multipage && self.one_file)
    }

    /// The extension of that one file.
    pub(crate) fn extension(&self) -> String {
        self.written()
            .map_or_else(|| GAF.to_owned(), |(_, w, _)| w.extension)
    }

    /// The rows of the run, from 0.
    pub(crate) fn rows(&self, set: &DataSet) -> Vec<usize> {
        let total = set.rows.len();
        match self.range {
            None => (0..total).collect(),
            Some((from, to)) => {
                let from = from.max(1).min(total.max(1)) - 1;
                (from..to.min(total)).collect()
            }
        }
    }
}

/// The dialog over the screen: the preview of a row on the left, the
/// choices on the right, the summary and the actions at the bottom.
pub(crate) fn show(
    ctx: &egui::Context,
    theme: &Theme,
    texts: &Texts,
    engine: &mut Gafira,
    dialog: &mut Dialog,
) -> Answer {
    let mut answer = Answer::Open;
    let Some(document) = engine.edit().document() else {
        return Answer::Cancel;
    };
    let Some(set) = document.data() else {
        return Answer::Cancel;
    };
    let mut chosen = set.clone();
    chosen.template_pages.clone_from(&dialog.pages);
    let pages = gafira_bulk::template_pages(&document, &chosen);
    refresh_preview(ctx, engine, dialog, (&document, set), &pages);
    let frame = egui::Frame::popup(&ctx.style())
        .inner_margin(egui::Margin::same(24))
        .corner_radius(theme.radius_md());
    egui::Modal::new(egui::Id::new("bulk_export"))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_width(720.0);
            ui.label(RichText::new(plain(texts, "bulk_export.title")).font(theme.font("h3")));
            ui.label(
                RichText::new(texts.t(&gafira_doc::msg!(
                    "bulk_export.subtitle",
                    name = Text(set.name.clone())
                )))
                .color(theme.c("text.secondary")),
            );
            ui.add_space(16.0);
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(260.0);
                    preview_column(ui, theme, texts, dialog, set, &pages);
                });
                ui.add_space(24.0);
                ui.vertical(|ui| {
                    choices(ui, theme, texts, dialog, (&document, set));
                });
            });
            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);
            let rows = dialog.rows(set);
            let unknown = gafira_bulk::unknown(&document, &chosen, &pages).len();
            let count = rows.len() * pages.len();
            let files = if dialog.one_target_file() {
                1
            } else if dialog.format().is_some_and(|f| f.multipage) {
                rows.len()
            } else {
                count
            };
            let files = files + usize::from(dialog.editable && dialog.format().is_some());
            ui.label(texts.t(&gafira_doc::msg!(
                "bulk_export.summary",
                rows = Count(rows.len() as u64),
                per = Count(pages.len() as u64),
                pages = Count(count as u64),
                files = Count(files as u64)
            )));
            if unknown > 0 {
                ui.label(
                    RichText::new(plain(texts, "bulk_export.unknown")).color(theme.c("error.fg")),
                );
            }
            crate::bulk::empty_values(ui, theme, texts, &document, set, &pages, &rows);
            ui.add_space(12.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let ready = unknown == 0 && count > 0;
                let label = texts.t(&gafira_doc::msg!(
                    "bulk.generate",
                    count = Count(count as u64)
                ));
                let export = ui.add_enabled_ui(ready, |ui| {
                    let button = egui::Button::new(
                        RichText::new(label)
                            .font(theme.font("ui-label"))
                            .color(theme.c("primary.fg")),
                    )
                    .fill(theme.c("primary.bg"))
                    .corner_radius(theme.radius_md())
                    .min_size(Vec2::new(160.0, 40.0));
                    ui.add(button)
                });
                if export.inner.clicked() {
                    answer = Answer::Export;
                }
                ui.add_space(8.0);
                let cancel = egui::Button::new(
                    RichText::new(plain(texts, "bulk.cancel")).font(theme.font("ui-label")),
                )
                .corner_radius(theme.radius_md())
                .min_size(Vec2::new(110.0, 40.0));
                if ui.add(cancel).clicked() {
                    answer = Answer::Cancel;
                }
            });
        });
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        answer = Answer::Cancel;
    }
    answer
}

/// Draw the row of the preview again when it, its page or the design
/// changed.
fn refresh_preview(
    ctx: &egui::Context,
    engine: &Gafira,
    dialog: &mut Dialog,
    (document, set): (&Document, &DataSet),
    pages: &[PageId],
) {
    let total = set.rows.len();
    if total == 0 || pages.is_empty() {
        dialog.preview = None;
        return;
    }
    dialog.row = dialog.row.min(total - 1);
    dialog.page = dialog.page.min(pages.len() - 1);
    let page = pages[dialog.page];
    let key = crate::bulk::picture_key(document, set, dialog.row, page);
    if dialog.preview.as_ref().is_some_and(|(of, _)| *of == key) {
        return;
    }
    let image =
        crate::bulk_run::row_picture(engine, (document, set), (dialog.row, page), (520, 520));
    let options = egui::TextureOptions::LINEAR;
    dialog.preview = Some((key, ctx.load_texture("bulk-export-preview", image, options)));
}

/// The row as it will look, and «‹ Fila 3 de 120 ›».
fn preview_column(
    ui: &mut Ui,
    theme: &Theme,
    texts: &Texts,
    dialog: &mut Dialog,
    set: &DataSet,
    pages: &[PageId],
) {
    ui.label(RichText::new(plain(texts, "export.preview")).font(theme.font("ui-label")));
    let side = 260.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(side, side), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, theme.radius_md(), theme.c("surface.sunken"));
    if let Some((_, texture)) = &dialog.preview {
        let size = texture.size_vec2();
        let room = rect.shrink(8.0);
        let k = (room.width() / size.x).min(room.height() / size.y);
        let place = egui::Rect::from_center_size(rect.center(), size * k);
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        painter.image(texture.id(), place, uv, egui::Color32::WHITE);
    }
    let total = set.rows.len();
    if total == 0 {
        return;
    }
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui
            .button("«")
            .on_hover_text(plain(texts, "bulk.first"))
            .clicked()
        {
            dialog.row = 0;
        }
        if ui
            .button("‹")
            .on_hover_text(plain(texts, "bulk.previous"))
            .clicked()
        {
            dialog.row = dialog.row.saturating_sub(1);
        }
        ui.label(texts.t(&gafira_doc::msg!(
            "bulk.row_of",
            row = Count(dialog.row as u64 + 1),
            total = Count(total as u64)
        )));
        if ui
            .button("›")
            .on_hover_text(plain(texts, "bulk.next"))
            .clicked()
        {
            dialog.row = (dialog.row + 1).min(total - 1);
        }
        if ui
            .button("»")
            .on_hover_text(plain(texts, "bulk.last"))
            .clicked()
        {
            dialog.row = total - 1;
        }
    });
    if pages.len() > 1 {
        ui.horizontal(|ui| {
            if ui.button("‹").clicked() {
                dialog.page = dialog.page.saturating_sub(1);
            }
            ui.label(texts.t(&gafira_doc::msg!(
                "bulk.page_of",
                page = Count(dialog.page as u64 + 1),
                total = Count(pages.len() as u64)
            )));
            if ui.button("›").clicked() {
                dialog.page = (dialog.page + 1).min(pages.len() - 1);
            }
        });
    }
}

/// The format and its options, the rows, the pages, the names and the
/// editable file: a two-column grid like «Exportar».
fn choices(
    ui: &mut Ui,
    theme: &Theme,
    texts: &Texts,
    dialog: &mut Dialog,
    (document, set): (&Document, &DataSet),
) {
    let name = |ui: &mut Ui, key: &'static str| {
        ui.label(RichText::new(plain(texts, key)).font(theme.font("ui-label")));
    };
    egui::ScrollArea::vertical()
        .id_salt("bulk_export.choices")
        .max_height(440.0)
        .show(ui, |ui| {
            // Choices stay on one line each, as in «Exportar».
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            egui::Grid::new("bulk_export.grid")
                .num_columns(2)
                .spacing([16.0, 14.0])
                .min_col_width(110.0)
                .show(ui, |ui| {
                    name(ui, "export.format");
                    let shown = match dialog.format() {
                        Some(format) => texts.t(&export::format_label(format.id)),
                        None => plain(texts, "bulk_export.gaf"),
                    };
                    let mut picked: Option<Option<usize>> = None;
                    egui::ComboBox::from_id_salt("bulk_export.format")
                        .width(WIDE)
                        .selected_text(shown)
                        .show_ui(ui, |ui| {
                            let current = dialog.format().map(|f| f.id);
                            for (index, format) in dialog.export.formats.iter().enumerate() {
                                let label = texts.t(&export::format_label(format.id));
                                if ui
                                    .selectable_label(current == Some(format.id), label)
                                    .clicked()
                                {
                                    picked = Some(Some(index));
                                }
                            }
                            let label = plain(texts, "bulk_export.gaf");
                            if ui.selectable_label(current.is_none(), label).clicked() {
                                picked = Some(None);
                            }
                        });
                    match picked {
                        Some(Some(index)) => {
                            dialog.gaf_only = false;
                            dialog.export.pick(index);
                        }
                        Some(None) => dialog.gaf_only = true,
                        None => {}
                    }
                    ui.end_row();
                    if dialog.format().is_some_and(|f| f.multipage) {
                        name(ui, "bulk_export.files");
                        ui.vertical(|ui| {
                            ui.radio_value(
                                &mut dialog.one_file,
                                true,
                                plain(texts, "bulk_export.one_file"),
                            );
                            ui.radio_value(
                                &mut dialog.one_file,
                                false,
                                plain(texts, "bulk_export.file_per_row"),
                            );
                        });
                        ui.end_row();
                    }
                    if dialog.format().is_some() {
                        export::format_options(ui, theme, texts, &mut dialog.export, WIDE);
                    }
                    name(ui, "bulk.rows");
                    rows_choice(ui, texts, dialog, set.rows.len());
                    ui.end_row();
                    if document.pages().len() > 1 {
                        name(ui, "bulk.template_pages");
                        pages_choice(ui, texts, dialog, document);
                        ui.end_row();
                    }
                    if !dialog.one_target_file() {
                        name(ui, "bulk.file_names");
                        names_choice(ui, theme, texts, dialog, set);
                        ui.end_row();
                    }
                    if dialog.format().is_some() {
                        name(ui, "bulk_export.editable");
                        ui.horizontal(|ui| {
                            widgets::toggle(ui, theme, &mut dialog.editable);
                            export::info(ui, theme, plain(texts, "bulk.also_gaf_hint"));
                        });
                        ui.end_row();
                    }
                });
        });
}

fn rows_choice(ui: &mut Ui, texts: &Texts, dialog: &mut Dialog, total: usize) {
    ui.vertical(|ui| {
        let all = texts.t(&gafira_doc::msg!(
            "bulk_export.all_rows",
            count = Count(total as u64)
        ));
        if ui.radio(dialog.range.is_none(), all).clicked() {
            dialog.range = None;
        }
        if ui
            .radio(dialog.range.is_some(), plain(texts, "bulk.some_rows"))
            .clicked()
            && dialog.range.is_none()
        {
            dialog.range = Some((1, total.max(1)));
        }
        if let Some((from, to)) = &mut dialog.range {
            ui.horizontal(|ui| {
                ui.label(plain(texts, "bulk.from"));
                ui.add(egui::DragValue::new(from).range(1..=total.max(1)));
                ui.label(plain(texts, "bulk.to"));
                ui.add(egui::DragValue::new(to).range(1..=total.max(1)));
            });
            if *to < *from {
                *to = *from;
            }
        }
    });
}

fn pages_choice(ui: &mut Ui, texts: &Texts, dialog: &mut Dialog, document: &Document) {
    ui.vertical(|ui| {
        for (k, page) in document.pages().iter().enumerate() {
            let mut on = dialog.pages.is_empty() && !page.hidden || dialog.pages.contains(&page.id);
            let name = if page.name.is_empty() {
                texts.t(&gafira_doc::msg!("bulk.page", number = Count(k as u64 + 1)))
            } else {
                format!("{} · {}", k + 1, page.name)
            };
            if ui.checkbox(&mut on, name).changed() {
                // From «every page» to an explicit list, then this change.
                if dialog.pages.is_empty() {
                    dialog.pages = document
                        .pages()
                        .iter()
                        .filter(|p| !p.hidden)
                        .map(|p| p.id)
                        .collect();
                }
                if on {
                    dialog.pages.push(page.id);
                } else {
                    dialog.pages.retain(|p| *p != page.id);
                }
                let order: Vec<PageId> = document
                    .pages()
                    .iter()
                    .map(|p| p.id)
                    .filter(|p| dialog.pages.contains(p))
                    .collect();
                dialog.pages = order;
            }
        }
    });
}

fn names_choice(ui: &mut Ui, theme: &Theme, texts: &Texts, dialog: &mut Dialog, set: &DataSet) {
    ui.vertical(|ui| {
        let key = set.columns.first().map_or("fila", |c| c.key.as_str());
        let example = format!(
            "diseno_{}",
            gafira_bulk::token(&FieldRef::Key(key.to_owned()))
        );
        let hint = texts.t(&gafira_doc::msg!(
            "bulk.file_names_hint",
            example = Text(example)
        ));
        ui.add(widgets::field(&mut dialog.file_name, hint).desired_width(WIDE));
        ui.horizontal_wrapped(|ui| {
            ui.set_max_width(WIDE + 40.0);
            for column in set.columns.iter().take(6) {
                let token = gafira_bulk::token(&FieldRef::Key(column.key.clone()));
                if ui
                    .small_button(&token)
                    .on_hover_text(&column.name)
                    .clicked()
                {
                    let base = dialog.file_name.trim_end_matches('_');
                    dialog.file_name = if base.is_empty() {
                        token
                    } else {
                        format!("{base}_{token}")
                    };
                }
            }
        });
        let extension = dialog.extension();
        let mut names = gafira_bulk::Names::default();
        for row in 0..set.rows.len().min(3) {
            let stem = crate::bulk_run::stem(&dialog.file_name, set, row);
            ui.label(
                RichText::new(names.unique(&stem, &extension))
                    .font(theme.font("caption"))
                    .color(theme.c("text.secondary")),
            );
        }
    });
}

// ---- The cards of a run, over the bottom right ---------------------------

/// What the person did with the card of a finished run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Finished {
    Open,
    ShowInFolder,
    Close,
}

/// A card where an export shows its progress: over the bottom right, above
/// the card of an export when both run.
fn toast(ctx: &egui::Context, above: bool, body: impl FnOnce(&mut Ui)) {
    let lift = if above { 220.0 } else { 0.0 };
    egui::Area::new(egui::Id::new("bulk-export-card"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -48.0 - lift))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_width(320.0);
                body(ui);
            });
        });
}

/// The run as it goes, with «Cancelar»; whether it was asked to cancel.
pub(crate) fn running(
    ctx: &egui::Context,
    (theme, texts): (&Theme, &Texts),
    status: &Status,
    (cancelling, above): (bool, bool),
) -> bool {
    let mut cancel = false;
    toast(ctx, above, |ui| {
        crate::bulk::progress_card(ui, theme, texts, status, cancelling, &mut cancel);
    });
    cancel
}

/// The run that ended: what it wrote, what failed and what does not fit,
/// with «Mostrar en carpeta» and «Cerrar».
pub(crate) fn finished(
    ctx: &egui::Context,
    (theme, texts): (&Theme, &Texts),
    status: &Status,
    (details, above): (&mut bool, bool),
) -> Finished {
    let mut answer = Finished::Open;
    toast(ctx, above, |ui| {
        let (key, color) = match (&status.stopped, status.phase) {
            (Some(_), _) => ("bulk.report_failed", "error.fg"),
            (None, Phase::Cancelled) => ("bulk.report_cancelled", "warning.fg"),
            _ => ("bulk.report_done", "success.fg"),
        };
        ui.label(
            RichText::new(plain(texts, key))
                .font(theme.font("ui-label"))
                .color(theme.c(color)),
        );
        ui.label(texts.t(&gafira_doc::msg!(
            "bulk.report_counts",
            written = Count(status.written as u64),
            failed = Count(status.failed as u64),
            elapsed = Text(crate::bulk::clock(
                status.took.unwrap_or_default().as_secs()
            ))
        )));
        if let Some(why) = &status.stopped {
            ui.label(RichText::new(why).font(theme.font("caption")));
        }
        if !status.notes.is_empty() {
            let key = if *details {
                "bulk_export.hide_details"
            } else {
                "bulk_export.details"
            };
            if ui.link(plain(texts, key)).clicked() {
                *details = !*details;
            }
            if *details {
                egui::ScrollArea::vertical()
                    .max_height(160.0)
                    .show(ui, |ui| notes(ui, theme, texts, &status.notes));
            }
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if status.first.is_some()
                && widgets::primary(ui, theme, plain(texts, "export.show_in_folder")).clicked()
            {
                answer = Finished::ShowInFolder;
            }
            if widgets::secondary(ui, theme, plain(texts, "export.close")).clicked() {
                answer = Finished::Close;
            }
        });
    });
    answer
}

/// The failures and the texts that do not fit, by row.
fn notes(ui: &mut Ui, theme: &Theme, texts: &Texts, notes: &[Note]) {
    let mut overflows: std::collections::BTreeMap<String, Vec<usize>> =
        std::collections::BTreeMap::new();
    for note in notes {
        match note {
            Note::Failed { row, file, why } => {
                ui.label(
                    RichText::new(texts.t(&gafira_doc::msg!(
                        "bulk.failed_row",
                        row = Count(*row as u64),
                        file = Text(file.clone()),
                        why = Text(why.clone())
                    )))
                    .font(theme.font("caption"))
                    .color(theme.c("error.fg")),
                );
            }
            Note::Overflow { row, element } => {
                overflows.entry(element.clone()).or_default().push(*row);
            }
        }
    }
    for (element, rows) in overflows {
        let first: Vec<String> = rows.iter().take(8).map(ToString::to_string).collect();
        ui.label(
            RichText::new(texts.t(&gafira_doc::msg!(
                "bulk.overflow",
                element = Text(element),
                count = Count(rows.len() as u64),
                rows = Text(first.join(", "))
            )))
            .font(theme.font("caption"))
            .color(theme.c("warning.fg")),
        );
    }
}

/// The place of the run: one file asked with the save dialog of the system,
/// or a folder; the editable `.gaf` beside them when asked, never over a
/// file that is there (AC 10). `None` when the person cancelled.
pub(crate) fn target(
    window: Option<&eframe::Frame>,
    dialog: &Dialog,
    set: &DataSet,
) -> Option<(Target, Option<PathBuf>)> {
    let base = Some(gafira_bulk::sanitize(&set.name))
        .filter(|b| !b.is_empty())
        .unwrap_or_else(|| "lote".to_owned());
    let picker = crate::files::picker(window);
    let extension = dialog.extension();
    let target = if dialog.one_target_file() {
        let stem = if dialog.format().is_none() {
            format!("{base}_lote")
        } else {
            base.clone()
        };
        let path = picker
            .add_filter(extension.to_uppercase(), &[extension.as_str()])
            .set_file_name(format!("{stem}.{extension}"))
            .save_file()?;
        Target::File(with_extension(path, &extension))
    } else {
        Target::Folder(picker.pick_folder()?)
    };
    let gaf =
        (dialog.editable && dialog.format().is_some()).then(|| editable_beside(&target, &base));
    Some((target, gaf))
}

/// The editable `.gaf` of a run beside its files.
fn editable_beside(target: &Target, base: &str) -> PathBuf {
    let (folder, stem) = match target {
        Target::File(path) => (
            path.parent()
                .map(std::path::Path::to_path_buf)
                .unwrap_or_default(),
            path.file_stem()
                .map_or_else(|| base.to_owned(), |s| s.to_string_lossy().into_owned()),
        ),
        Target::Folder(folder) => (folder.clone(), format!("{base}_lote")),
    };
    let mut names = gafira_bulk::Names::default();
    if let Ok(entries) = std::fs::read_dir(&folder) {
        for entry in entries.flatten() {
            names.take(&entry.file_name().to_string_lossy());
        }
    }
    folder.join(names.unique(&stem, GAF))
}

/// `path` ending in `.extension`, added when it has another or none.
fn with_extension(path: PathBuf, extension: &str) -> PathBuf {
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(extension))
    {
        path
    } else {
        let mut name = path.into_os_string();
        name.push(".");
        name.push(extension);
        PathBuf::from(name)
    }
}
