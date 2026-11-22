//! Spreadsheets brought to the canvas (spec 082): a file of Excel or of
//! separated values as a table, the sheet to insert when there are several,
//! and what is said when one cannot be read.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use egui::{Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};
use gafira_doc::msg::Message;
use gafira_doc::{NodeId, PageId, TableNode};
use gafira_io_xlsx::{Locale, ReadError, ReadSheet};

use crate::style::Theme;
use crate::tables::Outlines;
use crate::text::Texts;
use crate::widgets;

/// The largest file read, in bytes.
const MAX_FILE: u64 = 32 << 20;

/// The size of the preview, in px of screen.
const PREVIEW: Vec2 = Vec2::new(420.0, 280.0);

/// The least a preview shrinks: past it, its first cells are shown.
const PREVIEW_LEAST: f32 = 0.45;

fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// Whether `path` names a spreadsheet, read or not: one that is not says so.
pub(crate) fn is_sheet(path: &Path) -> bool {
    matches!(
        extension(path).as_str(),
        "xlsx" | "xlsm" | "xltx" | "xltm" | "xls" | "xlsb" | "ods" | "csv" | "tsv"
    )
}

/// Why a file did not come in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Failure {
    Read(ReadError),
    /// The file cannot be opened.
    Open,
}

impl Failure {
    fn message(self) -> Message {
        match self {
            Self::Read(ReadError::Unsupported) => gafira_doc::msg!("sheet_import.unsupported"),
            Self::Read(ReadError::TooLarge) => gafira_doc::msg!("sheet_import.too_large"),
            Self::Read(ReadError::Damaged) => gafira_doc::msg!("sheet_import.damaged"),
            Self::Read(ReadError::Empty) => gafira_doc::msg!("sheet_import.empty"),
            Self::Open => gafira_doc::msg!("sheet_import.open"),
        }
    }
}

/// The sheets of the file at `path`.
pub(crate) fn read(path: &Path, texts: &Texts) -> Result<Vec<ReadSheet>, Failure> {
    if extension(path) == "ods" {
        return Err(Failure::Read(ReadError::Unsupported));
    }
    let size = std::fs::metadata(path).map_err(|_| Failure::Open)?.len();
    if size > MAX_FILE {
        return Err(Failure::Read(ReadError::TooLarge));
    }
    let bytes = std::fs::read(path).map_err(|_| Failure::Open)?;
    let sheets = if matches!(extension(path).as_str(), "csv" | "tsv") {
        let name = path.file_stem().and_then(|n| n.to_str());
        gafira_io_xlsx::read_csv(&bytes, name.unwrap_or_default()).map(|sheet| vec![fitted(sheet)])
    } else {
        gafira_io_xlsx::read(&bytes, marks(texts))
    };
    sheets.map_err(Failure::Read)
}

/// The columns of separated values, in px: a file of them says no widths.
const COLUMN: std::ops::RangeInclusive<f64> = 56.0..=320.0;

/// `sheet` with its columns as wide as their words, measured with the fonts
/// of the canvas (S82-08).
fn fitted(mut sheet: ReadSheet) -> ReadSheet {
    use crate::tables::{Act, change};
    let all = gafira_table::Range::all(&sheet.table);
    let (w, h) = sheet.size;
    let placement = gafira_doc::Placement::new(0.0, 0.0, w, h);
    if let Some((mut table, _, _)) = change(&sheet.table, &placement, all, &Act::FitColumns) {
        for width in &mut table.columns {
            *width = width.clamp(*COLUMN.start(), *COLUMN.end());
        }
        sheet.size = gafira_table::size(&table);
        sheet.table = table;
    }
    sheet
}

/// The marks of the numbers in the language of the interface.
fn marks(texts: &Texts) -> Locale {
    match texts.locale {
        gafira_ports::i18n::Locale::Es => Locale {
            comma: true,
            yes: "VERDADERO",
            no: "FALSO",
        },
        _ => Locale {
            comma: false,
            yes: "TRUE",
            no: "FALSE",
        },
    }
}

/// What a file is opened for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Purpose {
    Table,
    /// A new chart of its data (S83-06).
    Chart,
    /// The data of this chart (S83-07).
    Data(PageId, NodeId),
}

impl Purpose {
    /// The rows and columns of `sheet` that come in.
    fn room(self, sheet: &ReadSheet) -> (usize, usize) {
        match self {
            Self::Table => (sheet.table.rows.len(), sheet.table.columns.len()),
            _ => (sheet.data.len(), sheet.data.first().map_or(0, Vec::len)),
        }
    }

    fn cuts(self, sheet: &ReadSheet) -> bool {
        let (rows, columns) = self.room(sheet);
        rows < sheet.used.0 || columns < sheet.used.1
    }
}

/// A sheet to bring in, from its file, for what and where.
pub(crate) type Answered = (ReadSheet, Purpose, Option<Pos2>);

/// A file that waits for the person.
pub(crate) struct Ask {
    file: PathBuf,
    /// Where it was dropped, in screen px.
    at: Option<Pos2>,
    purpose: Purpose,
    found: Result<Vec<ReadSheet>, Failure>,
    chosen: usize,
    previews: HashMap<usize, Option<egui::TextureHandle>>,
}

/// The files that wait, the first one shown.
#[derive(Default)]
pub(crate) struct Asks(VecDeque<Ask>);

/// What the person answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    Insert(usize),
    Cancel,
}

impl Asks {
    /// Takes what was found in a file: gives back the only sheet when it
    /// comes in whole, and else asks or says why not.
    pub fn take(
        &mut self,
        file: &Path,
        (purpose, at): (Purpose, Option<Pos2>),
        found: Result<Vec<ReadSheet>, Failure>,
    ) -> Option<ReadSheet> {
        match found {
            Ok(mut sheets) if sheets.len() == 1 && !purpose.cuts(&sheets[0]) => sheets.pop(),
            found => {
                self.0.push_back(Ask {
                    file: file.to_owned(),
                    at,
                    purpose,
                    found,
                    chosen: 0,
                    previews: HashMap::new(),
                });
                None
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The first file is answered: the sheet to insert and where, if any.
    pub fn answer(&mut self, answer: Answer) -> Option<Answered> {
        let ask = self.0.pop_front()?;
        let Answer::Insert(index) = answer else {
            return None;
        };
        let sheet = ask.found.ok()?.into_iter().nth(index)?;
        Some((sheet, ask.purpose, ask.at))
    }

    /// The dialog of the first file that waits.
    pub fn show(&mut self, ctx: &egui::Context, theme: &Theme, texts: &Texts) -> Option<Answer> {
        let ask = self.0.front_mut()?;
        let mut answer = None;
        let frame = egui::Frame::popup(&ctx.style()).inner_margin(egui::Margin::same(24));
        let modal = egui::Modal::new(egui::Id::new("sheet_import")).frame(frame);
        let response = modal.show(ctx, |ui| {
            let name = ask.file.file_name().and_then(|n| n.to_str());
            let name = name.unwrap_or_default().to_owned();
            match &ask.found {
                Ok(_) => answer = choose(ui, theme, texts, ask, &name),
                Err(failure) => {
                    ui.set_width(440.0);
                    let title = gafira_doc::msg!("sheet_import.failed");
                    ui.label(RichText::new(texts.t(&title)).font(theme.font("h3")));
                    file_name(ui, theme, &name);
                    ui.label(texts.t(&failure.message()));
                    ui.add_space(12.0);
                    let close = texts.t(&gafira_doc::msg!("sheet_import.close"));
                    if widgets::primary(ui, theme, close).clicked() {
                        answer = Some(Answer::Cancel);
                    }
                }
            }
        });
        if response.should_close() && answer.is_none() {
            answer = Some(Answer::Cancel);
        }
        answer
    }
}

fn file_name(ui: &mut Ui, theme: &Theme, name: &str) {
    let text = RichText::new(name)
        .font(theme.font("body-sm"))
        .color(theme.c("text.secondary"));
    ui.add(egui::Label::new(text).truncate());
    ui.add_space(8.0);
}

/// «¿Qué hoja quieres insertar?»: the sheets, the one chosen and its
/// preview.
fn choose(ui: &mut Ui, theme: &Theme, texts: &Texts, ask: &mut Ask, name: &str) -> Option<Answer> {
    let Ok(sheets) = &ask.found else { return None };
    let several = sheets.len() > 1;
    let list = if several { 200.0 } else { 0.0 };
    ui.set_width(PREVIEW.x + list + if several { 16.0 } else { 0.0 });
    let title = if several {
        gafira_doc::msg!("sheet_import.title")
    } else {
        gafira_doc::msg!("sheet_import.title_one")
    };
    ui.label(RichText::new(texts.t(&title)).font(theme.font("h3")));
    file_name(ui, theme, name);
    let mut chosen = ask.chosen.min(sheets.len().saturating_sub(1));
    let mut insert = false;
    ui.horizontal_top(|ui| {
        if several {
            ui.vertical(|ui| {
                ui.set_width(list);
                egui::ScrollArea::vertical()
                    .id_salt("sheet_import_list")
                    .max_height(PREVIEW.y)
                    .min_scrolled_height(PREVIEW.y)
                    .show(ui, |ui| {
                        for (i, sheet) in sheets.iter().enumerate() {
                            let response = row(ui, theme, sheet, chosen == i, list - 12.0);
                            if response.clicked() {
                                chosen = i;
                            }
                            insert |= response.double_clicked();
                        }
                    });
            });
            ui.add_space(8.0);
        }
        if let Some(sheet) = sheets.get(chosen) {
            let texture = ask
                .previews
                .entry(chosen)
                .or_insert_with(|| preview(ui.ctx(), sheet));
            paint(ui, theme, texture.as_ref());
        }
    });
    if let Some(sheet) = sheets.get(chosen).filter(|s| ask.purpose.cuts(s)) {
        ui.add_space(8.0);
        let (rows, columns) = ask.purpose.room(sheet);
        let note = gafira_doc::msg!(
            "sheet_import.cut",
            rows = Count(rows as u64),
            columns = Count(columns as u64),
            all_rows = Count(sheet.used.0 as u64),
            all_columns = Count(sheet.used.1 as u64),
        );
        ui.label(RichText::new(texts.t(&note)).font(theme.font("body-sm")));
    }
    ui.add_space(12.0);
    let mut cancel = false;
    ui.horizontal(|ui| {
        let text = match ask.purpose {
            Purpose::Table => gafira_doc::msg!("sheet_import.insert"),
            Purpose::Chart => gafira_doc::msg!("sheet_import.chart"),
            Purpose::Data(..) => gafira_doc::msg!("sheet_import.data"),
        };
        let text = texts.t(&text);
        insert |= widgets::primary(ui, theme, text).clicked();
        let text = texts.t(&gafira_doc::msg!("sheet_import.cancel"));
        cancel = widgets::secondary(ui, theme, text).clicked();
    });
    insert |= ui.input(|i| i.key_pressed(egui::Key::Enter));
    ask.chosen = chosen;
    if cancel {
        Some(Answer::Cancel)
    } else if insert {
        Some(Answer::Insert(chosen))
    } else {
        None
    }
}

/// A sheet of the list: its name and its size.
fn row(ui: &mut Ui, theme: &Theme, sheet: &ReadSheet, on: bool, width: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 46.0), Sense::click());
    let painter = ui.painter().with_clip_rect(rect);
    let shape = rect.shrink2(Vec2::new(1.0, 2.0));
    if on {
        painter.rect_filled(shape, theme.radius_md(), theme.c("surface.sunken"));
    }
    if on || response.hovered() || response.has_focus() {
        let color = theme.c("primary.bg");
        let stroke = Stroke::new(if on { 2.0 } else { 1.0 }, color);
        painter.rect_stroke(shape, theme.radius_md(), stroke, egui::StrokeKind::Inside);
    }
    let line = |text: String, font, color| {
        let mut job = egui::text::LayoutJob::simple_singleline(text, font, color);
        job.wrap = egui::text::TextWrapping::truncate_at_width(width - 24.0);
        ui.fonts_mut(|f| f.layout_job(job))
    };
    let color = theme.c("text");
    let name = line(sheet.name.clone(), theme.font("body"), color);
    let size = format!("{} × {}", sheet.used.0, sheet.used.1);
    let color = theme.c("text.secondary");
    let size = line(size, theme.font("body-sm"), color);
    painter.galley(rect.min + Vec2::new(12.0, 5.0), name, color);
    painter.galley(rect.min + Vec2::new(12.0, 24.0), size, color);
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, on, &sheet.name)
    });
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// How much a table of `size` shrinks in the preview.
fn preview_scale(size: (f64, f64)) -> f32 {
    // Sizes of a screen: the casts lose nothing seen.
    let room = PREVIEW - Vec2::splat(16.0);
    let fit = (room.x / size.0 as f32).min(room.y / size.1 as f32);
    fit.clamp(PREVIEW_LEAST, 1.0)
}

fn preview(ctx: &egui::Context, sheet: &ReadSheet) -> Option<egui::TextureHandle> {
    let k = preview_scale(sheet.size) * ctx.pixels_per_point();
    let shown = head(&sheet.table, f64::from(PREVIEW_LEAST));
    let size = gafira_table::size(&shown);
    let pixels = crate::tables::image(&shown, size, k, None, &mut Outlines::default())?;
    Some(ctx.load_texture("sheet-preview", pixels, egui::TextureOptions::LINEAR))
}

/// The first rows and columns of `t`, those a preview at `scale` can show.
fn head(t: &TableNode, scale: f64) -> TableNode {
    let count = |sides: &[f64], room: f32| {
        let mut left = f64::from(room) / scale;
        let shown = sides.iter().take_while(|side| {
            let fits = left > 0.0;
            left -= **side;
            fits
        });
        shown.count().max(1)
    };
    let rows = count(&t.rows, PREVIEW.y);
    let cols = count(&t.columns, PREVIEW.x);
    let mut out = t.clone();
    out.rows.truncate(rows);
    out.columns.truncate(cols);
    out.cells = t
        .cells
        .chunks(t.columns.len())
        .take(rows)
        .flat_map(|row| row[..cols].to_vec())
        .collect();
    out.merges = t
        .merges
        .iter()
        .filter(|m| m.row < rows && m.col < cols)
        .map(|m| gafira_doc::CellMerge {
            rows: m.rows.min(rows - m.row),
            cols: m.cols.min(cols - m.col),
            ..*m
        })
        .filter(|m| m.rows * m.cols > 1)
        .collect();
    out
}

fn paint(ui: &mut Ui, theme: &Theme, texture: Option<&egui::TextureHandle>) {
    let (rect, _) = ui.allocate_exact_size(PREVIEW, Sense::hover());
    let painter = ui.painter().with_clip_rect(rect);
    // White paper, in the dark theme too.
    painter.rect_filled(rect, theme.radius_md(), egui::Color32::WHITE);
    if let Some(texture) = texture {
        let size = texture.size_vec2() / ui.ctx().pixels_per_point();
        let room = rect.shrink(8.0);
        let slack = (room.size() - size).max(Vec2::ZERO) / 2.0;
        let place = Rect::from_min_size(room.min + slack, size);
        let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        painter.image(texture.id(), place, uv, egui::Color32::WHITE);
    }
    let stroke = Stroke::new(1.0, theme.c("border"));
    painter.rect_stroke(rect, theme.radius_md(), stroke, egui::StrokeKind::Inside);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet(rows: usize, cols: usize) -> ReadSheet {
        let style = gafira_doc::CellStyle::new(
            "Inter",
            16.0,
            gafira_color::Color::Srgb(gafira_color::Rgba::new(0.0, 0.0, 0.0, 1.0)),
        );
        let table = TableNode::new(rows.min(50), cols.min(20), (80.0, 24.0), &style);
        ReadSheet {
            name: "Hoja".to_owned(),
            size: gafira_table::size(&table),
            table,
            used: (rows, cols),
            data: vec![vec![String::new(); cols.min(20)]; rows.min(201)],
        }
    }

    // S82-06: one sheet that fits comes in at once; several, or a cut one,
    // ask first; S82-07: what cannot be read says so.
    #[test]
    fn s82_06_when_it_asks() {
        let file = Path::new("/x/libro.xlsx");
        let mut asks = Asks::default();
        assert!(
            asks.take(file, (Purpose::Table, None), Ok(vec![sheet(3, 2)]))
                .is_some()
        );
        assert!(asks.is_empty());
        assert!(
            asks.take(file, (Purpose::Table, None), Ok(vec![sheet(80, 2)]))
                .is_none()
        );
        // S83-06: a chart takes more rows.
        let found = Ok(vec![sheet(80, 2)]);
        assert!(asks.take(file, (Purpose::Chart, None), found).is_some());
        let at = Some(Pos2::new(5.0, 6.0));
        let two = Ok(vec![sheet(3, 2), sheet(4, 4)]);
        assert!(asks.take(file, (Purpose::Table, at), two).is_none());
        let failure = Err(Failure::Read(ReadError::Empty));
        assert!(asks.take(file, (Purpose::Table, None), failure).is_none());
        assert!(asks.answer(Answer::Cancel).is_none(), "the cut one");
        let (chosen, _, place) = asks.answer(Answer::Insert(1)).expect("a sheet");
        assert_eq!((chosen.used, place), ((4, 4), at));
        assert!(
            asks.answer(Answer::Insert(0)).is_none(),
            "nothing to insert"
        );
        assert!(asks.is_empty());
    }

    // S82-05, S82-07: the files taken, read or not.
    #[test]
    fn s82_07_spreadsheets_are_known_by_their_extension() {
        for name in ["a.xlsx", "A.XLSM", "a.xls", "a.ods", "a.csv", "a.TSV"] {
            assert!(is_sheet(Path::new(name)), "{name}");
        }
        for name in ["a.png", "a.docx", "xlsx", "a"] {
            assert!(!is_sheet(Path::new(name)), "{name}");
        }
        let texts = Texts::new(gafira_ports::i18n::Locale::Es);
        let ods = read(Path::new("/no/such.ods"), &texts);
        assert_eq!(ods, Err(Failure::Read(ReadError::Unsupported)));
        let missing = read(Path::new("/no/such.csv"), &texts);
        assert_eq!(missing, Err(Failure::Open));
    }

    // S82-08: the columns of a CSV hold their words in one line.
    #[test]
    fn s82_08_words_do_not_wrap() {
        let csv = b"Ciudad;Nota\nWWWWWWWWWW;7\nCali;una nota mas larga que las otras";
        let sheet = fitted(gafira_io_xlsx::read_csv(csv, "x").expect("read"));
        assert!(sheet.table.is_valid());
        let t = &sheet.table;
        for (i, cell) in t.cells.iter().enumerate() {
            let width = t.columns[i % 2];
            let tall = |w| {
                gafira_render::cell_outline(cell, (w, 1.0), t.padding, crate::canvas::text_fonts())
                    .map(|(_, h)| h)
            };
            assert_eq!(tall(width), tall(10_000.0), "{}", cell.text);
        }
    }

    // S82-06: the preview of a large table shows its first cells.
    #[test]
    fn s82_06_the_preview_is_the_head() {
        let mut table = sheet(50, 20).table;
        table.merges.push(gafira_doc::CellMerge {
            row: 0,
            col: 0,
            rows: 1,
            cols: 20,
        });
        let head = head(&table, 0.45);
        assert!(head.is_valid());
        assert!(head.rows.len() < 50 && head.columns.len() < 20);
        assert_eq!(head.merges[0].cols, head.columns.len());
    }
}
