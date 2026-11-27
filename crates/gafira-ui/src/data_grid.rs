//! A grid of cells to read and write a table of words (spec 083, reused by
//! spec 104): the first line names the columns, only the lines in view are
//! drawn, there is room past the data to write more, Enter goes down and a
//! block pasted from a spreadsheet fills as many cells.
//!
//! What the cells hold and what a change does belong to a [`Cells`]: the
//! data of a chart, the connected data of bulk create.

use egui::{Rect, Sense, Stroke, Ui, Vec2};

use crate::style::Theme;

/// The height of a line of the grid.
pub(crate) const ROW: f32 = 30.0;

/// The width of the squares before each line.
const HEAD: f32 = 34.0;

/// A table the grid shows and changes. Line 0 holds the names of the
/// columns.
pub(crate) trait Cells {
    /// The columns and the lines with words, line 0 included.
    fn size(&self) -> (usize, usize);
    /// The most columns and lines the grid offers.
    fn most(&self) -> (usize, usize);
    /// The empty lines and columns offered past the data.
    fn spare(&self) -> (usize, usize) {
        (12, 3)
    }
    /// The longest words of a cell, in characters.
    fn longest(&self) -> usize;
    /// The words of a cell; empty past the data.
    fn cell(&self, row: usize, col: usize) -> &str;
    /// Write `words` in a cell, past the data too.
    fn set(&mut self, row: usize, col: usize, words: &str);
    /// Whether the column holds numbers: they lean right.
    fn numbers(&self, _col: usize) -> bool {
        false
    }
    /// What is wrong with the words of a cell, said over it in red.
    fn wrong(&self, _row: usize, _col: usize, _words: &str) -> Option<String> {
        None
    }
    /// Draws what goes in the square over a column.
    fn column_head(&mut self, _ui: &mut Ui, _theme: &Theme, _rect: Rect, _col: usize) {}
    /// Draws what goes in the square before a line of data.
    fn row_head(&mut self, _ui: &mut Ui, _theme: &Theme, _rect: Rect, _row: usize) {}
}

/// The cell being written, kept between frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Focus(pub Option<(usize, usize)>);

/// The grid of `cells`, `tall` px high with columns `wide` px wide; `salt`
/// tells its cells from those of another grid.
pub(crate) fn grid(
    ui: &mut Ui,
    theme: &Theme,
    cells: &mut dyn Cells,
    focus: &mut Focus,
    (tall, wide, salt): (f32, f32, impl std::hash::Hash + Copy),
) {
    let id = |row: usize, col: usize| egui::Id::new(("data_cell", salt, row, col));
    let focused = ui.memory(|m| m.focused());
    let writing = focus.0.filter(|(r, col)| focused == Some(id(*r, *col)));
    if let Some(at) = writing {
        paste(ui, cells, at);
    }
    // As many as fill the room, and some more past the data.
    let fill = |room: f32, side: f32| (room / side).max(0.0) as usize;
    let (columns, rows) = cells.size();
    let (most_columns, most_rows) = cells.most();
    let (spare_rows, spare_columns) = cells.spare();
    let rows = (rows + spare_rows).max(fill(tall, ROW)).min(most_rows);
    let columns = (columns + spare_columns)
        .max(fill(ui.available_width(), wide))
        .min(most_columns);
    let line = Stroke::new(1.0, theme.c("border"));
    let mut next_focus = None;
    egui::Frame::NONE.stroke(line).show(ui, |ui| {
        egui::ScrollArea::both()
            .id_salt(("data_grid", salt))
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
                            for col in 0..columns {
                                let rect = square(ui, theme, wide);
                                cells.column_head(ui, theme, rect, col);
                            }
                            return;
                        };
                        let head = square(ui, theme, HEAD);
                        cells.row_head(ui, theme, head, row);
                        for col in 0..columns {
                            let cell = Cell {
                                id: id(row, col),
                                row,
                                col,
                                wide,
                            };
                            if cell.show(ui, theme, cells) {
                                focus.0 = Some((row, col));
                            }
                            if focus.0 == Some((row, col))
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
        focus.0 = Some((row, col));
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

/// Words in the middle of a square of the edge, in the secondary colour.
pub(crate) fn head_words(ui: &Ui, theme: &Theme, rect: Rect, words: &str) {
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        words,
        theme.font("caption"),
        theme.c("text.secondary"),
    );
}

struct Cell {
    id: egui::Id,
    row: usize,
    col: usize,
    wide: f32,
}

impl Cell {
    /// A field with the words of the cell; whether it has the keys.
    fn show(&self, ui: &mut Ui, theme: &Theme, cells: &mut dyn Cells) -> bool {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(self.wide, ROW), Sense::hover());
        // The rows below are there to write in; a column more is farther.
        let inside = self.col < cells.size().0;
        let fill = if inside { "surface" } else { "surface.sunken" };
        ui.painter().rect_filled(rect, 0.0, theme.c(fill));
        edge(ui, theme, rect);
        let mut words = cells.cell(self.row, self.col).to_owned();
        let head = self.row == 0;
        let numbers = cells.numbers(self.col);
        let wrong = cells.wrong(self.row, self.col, &words);
        let font = theme.font(if head { "ui-label" } else { "body-sm" });
        let color = if wrong.is_some() {
            theme.c("error.fg")
        } else {
            theme.c("text")
        };
        let align = if numbers && !head {
            egui::Align::RIGHT
        } else {
            egui::Align::LEFT
        };
        let field = egui::TextEdit::singleline(&mut words)
            .id(self.id)
            .font(font)
            .text_color(color)
            .frame(false)
            .char_limit(cells.longest())
            .margin(Vec2::new(10.0, 5.0))
            .horizontal_align(align);
        let response = ui.put(rect.shrink(1.5), field);
        if response.has_focus() {
            let stroke = Stroke::new(2.0, theme.c("primary.bg"));
            let painter = ui.painter();
            painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
        }
        if response.changed() {
            cells.set(self.row, self.col, &words);
        }
        let has_keys = response.has_focus();
        if let Some(why) = wrong {
            response.on_hover_text(why);
        }
        has_keys
    }
}

/// Several cells in the clipboard go to as many, from the one at `at`.
fn paste(ui: &Ui, cells: &mut dyn Cells, at: (usize, usize)) {
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
    for (r, row) in grid.iter().enumerate() {
        for (col, words) in row.iter().enumerate() {
            cells.set(at.0 + r, at.1 + col, words);
        }
    }
    ui.ctx().request_repaint();
}
