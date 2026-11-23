//! «Tablas» (spec 081, FD-21): the cells chosen on the canvas, the changes
//! asked for, the pixels of a table and the controls of its looks.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use egui::{Color32, ColorImage, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use gafira_color::Color;
use gafira_doc::{
    CellStyle, NodeId, PageId, Placement, ShapeStroke, TableNode, TextAlign, VerticalAlign,
};
use gafira_edit::GestureId;
use gafira_table::{BorderSet, Layout, Range, anchors, edges};

use crate::canvas;
use crate::style::Theme;
use crate::text::Texts;
use crate::widgets;

fn plain(texts: &Texts, key: &'static str) -> String {
    texts.t(&gafira_doc::msg::Message {
        key: gafira_doc::msg::MessageKey(key),
        args: Vec::new(),
    })
}

/// The longest side of the texture of a table, in px.
const TEXTURE_MAX: f64 = 4096.0;
/// How near the border of a column or a row is grabbed, in screen px.
pub(crate) const EDGE_SLACK: f32 = 5.0;

/// The cells chosen in a table (S81-08): from the one pressed first to the
/// last one, and the words being written in the first.
#[derive(Clone, Debug)]
pub(crate) struct Cells {
    pub(crate) page: PageId,
    pub(crate) node: NodeId,
    pub(crate) from: (usize, usize),
    pub(crate) to: (usize, usize),
    pub(crate) writing: Option<Writing>,
}

/// The words being written in a cell, all of them one undo step.
#[derive(Clone, Debug)]
pub(crate) struct Writing {
    pub(crate) words: String,
    pub(crate) gesture: Option<GestureId>,
    pub(crate) focused: bool,
}

impl Cells {
    pub(crate) fn one(page: PageId, node: NodeId, cell: (usize, usize)) -> Self {
        Self {
            page,
            node,
            from: cell,
            to: cell,
            writing: None,
        }
    }

    /// The cells, with the whole of every merge they touch.
    pub(crate) fn range(&self, t: &TableNode) -> Range {
        Range::new(self.from, self.to).over(t)
    }

    /// The cell a step (`rows` down, `cols` right) away from the first one,
    /// past the merge it may be in.
    pub(crate) fn step(&self, t: &TableNode, rows: i32, cols: i32) -> (usize, usize) {
        let at = Range::cell(self.from.0, self.from.1).over(t);
        let next = |first: usize, last: usize, step: i32, count: usize| match step {
            1.. => (last + 1).min(count.saturating_sub(1)),
            0 => first,
            _ => first.saturating_sub(1),
        };
        let cell = (
            next(at.r0, at.r1, rows, t.rows.len()),
            next(at.c0, at.c1, cols, t.columns.len()),
        );
        t.merge_at(cell.0, cell.1).map_or(cell, |m| (m.row, m.col))
    }

    /// The cell after the first one (or before it), row by row; `None` past
    /// the last.
    pub(crate) fn tab(&self, t: &TableNode, back: bool) -> Option<(usize, usize)> {
        let list: Vec<(usize, usize)> = anchors(t).collect();
        let now = Range::cell(self.from.0, self.from.1).over(t);
        let at = list.iter().position(|c| *c == (now.r0, now.c0))?;
        if back {
            list.get(at.checked_sub(1)?).copied()
        } else {
            list.get(at + 1).copied()
        }
    }
}

/// A change asked of a table.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Act {
    RowBefore,
    RowAfter,
    /// A row at the end, whatever the cells chosen.
    RowLast,
    RemoveRows,
    DuplicateRows,
    MoveRow(bool),
    EqualRows,
    FitRows,
    ColumnBefore,
    ColumnAfter,
    ColumnLast,
    RemoveColumns,
    MoveColumn(bool),
    EqualColumns,
    FitColumns,
    Merge,
    Unmerge,
    Clear,
    Words((usize, usize), String),
    Paste(Vec<Vec<String>>),
    PasteStyle(Box<CellStyle>),
    Fill(Option<Color>),
    Ink(Color),
    Size(f64),
    Bold(bool),
    Italic(bool),
    Font(String, bool, bool),
    Align(TextAlign),
    Valign(VerticalAlign),
    Borders(BorderSet, Option<ShapeStroke>),
    Spacing(f64),
    Padding(f64),
    Header(bool),
    ResizeColumn(usize, f64),
    ResizeRow(usize, f64),
}

/// The height the words of a cell need, with its padding.
fn needed(t: &TableNode, layout: &Layout, (r, c): (usize, usize)) -> f64 {
    let Some(cell) = t.cell(r, c) else { return 0.0 };
    if cell.text.is_empty() {
        // Room for a line, so writing does not move the row.
        return cell.style.size * 1.2 + 2.0 * t.padding;
    }
    let width = layout.cell(t, r, c).width();
    gafira_render::cell_outline(cell, (width, 1.0), t.padding, canvas::text_fonts())
        .map_or(0.0, |(_, height)| height + 2.0 * t.padding)
}

/// Makes the rows of `range` tall enough for their words (S81-03); with
/// `exactly`, no taller either.
fn fit_rows(t: &mut TableNode, range: Range, exactly: bool) {
    let layout = Layout::of(t, gafira_table::size(t));
    for r in range.r0..=range.r1.min(t.rows.len().saturating_sub(1)) {
        let need = (0..t.columns.len())
            .filter(|c| t.merge_at(r, *c).is_none_or(|m| m.rows == 1 && m.col == *c))
            .map(|c| needed(t, &layout, (r, c)))
            .fold(gafira_table::MIN_SIZE, f64::max);
        if exactly || t.rows[r] < need {
            t.rows[r] = need;
        }
    }
    // A cell over several rows takes the room it lacks from the last one.
    for m in t.merges.clone().into_iter().filter(|m| m.rows > 1) {
        let last = m.row + m.rows - 1;
        if (range.r0..=range.r1).contains(&m.row) || (range.r0..=range.r1).contains(&last) {
            let layout = Layout::of(t, gafira_table::size(t));
            let lack = needed(t, &layout, (m.row, m.col)) - layout.cell(t, m.row, m.col).height();
            if lack > 0.0 {
                t.rows[last] += lack;
            }
        }
    }
}

/// Makes the columns of `range` as wide as their longest line.
fn fit_columns(t: &mut TableNode, range: Range) {
    use kurbo::Shape as _;
    const WIDE: f64 = 100_000.0;
    for c in range.c0..=range.c1 {
        let mut width = gafira_table::MIN_SIZE;
        for r in 0..t.rows.len() {
            if t.merge_at(r, c).is_some() {
                continue;
            }
            let Some(cell) = t.cell(r, c) else { continue };
            let mut line = cell.clone();
            line.style.align = TextAlign::Left;
            if let Some((path, _)) =
                gafira_render::cell_outline(&line, (WIDE, 1.0), 0.0, canvas::text_fonts())
            {
                // A little more than its ink, or the last word would wrap.
                let need = path.bounding_box().x1 + cell.style.size * 0.15 + 2.0 * t.padding;
                width = width.max(need);
            }
        }
        if let Some(own) = t.columns.get_mut(c) {
            *own = width;
        }
    }
}

/// `placement` with the size `w` × `h`, the corner of its first cell where
/// it was, turned or not (S81-02).
pub(crate) fn resized(placement: &Placement, (w, h): (f64, f64)) -> Placement {
    let origin = kurbo::Point::ZERO;
    let mut next = Placement { w, h, ..*placement };
    let shift = placement.to_transform() * origin - next.to_transform() * origin;
    next.x += shift.x;
    next.y += shift.y;
    next
}

/// `t` in the box `placement` after `act` on the cells of `range`: the
/// table, its box and the cells chosen now. `None` when nothing changes.
pub(crate) fn change(
    t: &TableNode,
    placement: &Placement,
    range: Range,
    act: &Act,
) -> Option<(TableNode, Placement, Range)> {
    let mut next = t.clone();
    let range = range.over(t);
    let mut now = range;
    gafira_table::to_px(&mut next, (placement.w, placement.h));
    let all = Range::all(t);
    // The rows whose words may need more room after the change.
    let mut grow = None;
    let styled = |next: &mut TableNode, f: &dyn Fn(&mut CellStyle)| {
        gafira_table::restyle(next, range, f);
        true
    };
    let done = match act {
        Act::RowBefore => gafira_table::insert_row(&mut next, range.r0, range.r0),
        Act::RowAfter => {
            now = Range::new((range.r1 + 1, range.c0), (range.r1 + 1, range.c0));
            gafira_table::insert_row(&mut next, range.r1 + 1, range.r1)
        }
        Act::RowLast => {
            now = Range::cell(all.r1 + 1, 0);
            gafira_table::insert_row(&mut next, all.r1 + 1, all.r1)
        }
        Act::RemoveRows => gafira_table::remove_rows(&mut next, range.r0, range.r1),
        Act::DuplicateRows => gafira_table::duplicate_rows(&mut next, range.r0, range.r1),
        Act::MoveRow(down) => {
            let moved = range.r0 == range.r1 && gafira_table::move_row(&mut next, range.r0, *down);
            if moved {
                let to = if *down { range.r0 + 1 } else { range.r0 - 1 };
                now = Range::new((to, range.c0), (to, range.c1));
            }
            moved
        }
        Act::EqualRows => {
            let (a, b) = several(range.r0, range.r1, all.r1);
            gafira_table::equalize_rows(&mut next, a, b);
            grow = Some(all);
            true
        }
        Act::FitRows => {
            fit_rows(&mut next, range, true);
            true
        }
        Act::ColumnBefore => gafira_table::insert_column(&mut next, range.c0, range.c0),
        Act::ColumnAfter => {
            now = Range::new((range.r0, range.c1 + 1), (range.r0, range.c1 + 1));
            gafira_table::insert_column(&mut next, range.c1 + 1, range.c1)
        }
        Act::ColumnLast => {
            now = Range::cell(0, all.c1 + 1);
            gafira_table::insert_column(&mut next, all.c1 + 1, all.c1)
        }
        Act::RemoveColumns => gafira_table::remove_columns(&mut next, range.c0, range.c1),
        Act::MoveColumn(right) => {
            let moved =
                range.c0 == range.c1 && gafira_table::move_column(&mut next, range.c0, *right);
            if moved {
                let to = if *right { range.c0 + 1 } else { range.c0 - 1 };
                now = Range::new((range.r0, to), (range.r1, to));
            }
            moved
        }
        Act::EqualColumns => {
            let (a, b) = several(range.c0, range.c1, all.c1);
            gafira_table::equalize_columns(&mut next, a, b);
            grow = Some(all);
            true
        }
        Act::FitColumns => {
            fit_columns(&mut next, range);
            grow = Some(all);
            true
        }
        Act::Merge => gafira_table::merge(&mut next, range),
        Act::Unmerge => gafira_table::unmerge(&mut next, range),
        Act::Clear => {
            gafira_table::clear(&mut next, range);
            true
        }
        Act::Words(cell, words) => {
            grow = Some(Range::cell(cell.0, cell.1).over(t));
            let fits = words.len() <= TableNode::MAX_CELL_TEXT;
            match next.cell_mut(cell.0, cell.1) {
                Some(own) if fits => {
                    own.text.clone_from(words);
                    true
                }
                _ => false,
            }
        }
        Act::Paste(grid) => {
            let done = gafira_table::paste(&mut next, (range.r0, range.c0), grid);
            let wide = grid.iter().map(Vec::len).max().unwrap_or(1).max(1);
            let end = Range::all(&next);
            now = Range::new(
                (range.r0, range.c0),
                (
                    (range.r0 + grid.len().max(1) - 1).min(end.r1),
                    (range.c0 + wide - 1).min(end.c1),
                ),
            );
            grow = Some(end);
            done
        }
        Act::PasteStyle(style) => {
            grow = Some(range);
            gafira_table::paste_style(&mut next, range, style);
            true
        }
        Act::Fill(color) => styled(&mut next, &|s| s.fill = *color),
        Act::Ink(color) => styled(&mut next, &|s| s.color = *color),
        Act::Size(size) => {
            grow = Some(range);
            styled(&mut next, &|s| s.size = *size)
        }
        Act::Bold(on) => {
            grow = Some(range);
            styled(&mut next, &|s| s.bold = *on)
        }
        Act::Italic(on) => styled(&mut next, &|s| s.italic = *on),
        Act::Font(family, bold, italic) => {
            grow = Some(range);
            styled(&mut next, &|s| {
                s.font.clone_from(family);
                s.bold = *bold;
                s.italic = *italic;
            })
        }
        Act::Align(align) => styled(&mut next, &|s| s.align = *align),
        Act::Valign(valign) => styled(&mut next, &|s| s.valign = *valign),
        Act::Borders(which, stroke) => {
            gafira_table::set_borders(&mut next, range, *which, *stroke);
            true
        }
        Act::Spacing(spacing) => {
            next.spacing = spacing.clamp(0.0, TableNode::MAX_SPACING);
            true
        }
        Act::Padding(padding) => {
            next.padding = padding.clamp(0.0, TableNode::MAX_SPACING);
            grow = Some(all);
            true
        }
        Act::Header(on) => {
            next.header = *on;
            true
        }
        Act::ResizeColumn(index, delta) => {
            gafira_table::resize_column(&mut next, *index, *delta);
            grow = Some(all);
            true
        }
        Act::ResizeRow(index, delta) => {
            gafira_table::resize_row(&mut next, *index, *delta);
            grow = Some(Range::new((*index, 0), (*index, all.c1)));
            true
        }
    };
    if !done {
        return None;
    }
    if let Some(range) = grow {
        fit_rows(&mut next, range, false);
    }
    let box_of = resized(placement, gafira_table::size(&next));
    let now = now.over(&next);
    (next.is_valid() && (next != *t || box_of != *placement)).then_some((next, box_of, now))
}

/// The lines to make alike: the chosen ones, or all when only one is.
fn several(from: usize, to: usize, last: usize) -> (usize, usize) {
    if from == to { (0, last) } else { (from, to) }
}

// ---- Pixels --------------------------------------------------------------

/// The outlines of the cells drawn last, so writing in one lays out that one
/// alone.
#[derive(Default)]
pub(crate) struct Outlines(HashMap<u64, Option<Arc<kurbo::BezPath>>>);

impl Outlines {
    fn of(
        &mut self,
        cell: &gafira_doc::TableCell,
        size: (f64, f64),
        padding: f64,
    ) -> Option<Arc<kurbo::BezPath>> {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        cell.text.hash(&mut hasher);
        let s = &cell.style;
        format!(
            "{}{}{}{}{:?}{:?}{size:?}{padding}",
            s.font, s.size, s.bold, s.italic, s.align, s.valign
        )
        .hash(&mut hasher);
        let key = hasher.finish();
        if self.0.len() > 4096 {
            self.0.clear();
        }
        self.0
            .entry(key)
            .or_insert_with(|| {
                gafira_render::cell_outline(cell, size, padding, canvas::text_fonts())
                    .map(|(path, _)| Arc::new(path))
            })
            .clone()
    }
}

/// The key of the pixels of a table in a box.
pub(crate) fn key(t: &TableNode, rest: impl Hash) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    format!("table {t:?}").hash(&mut hasher);
    rest.hash(&mut hasher);
    hasher.finish()
}

/// How far the borders stick out of the box of `t`, in its px.
pub(crate) fn overhang(t: &TableNode) -> f64 {
    let widest = t
        .cells
        .iter()
        .flat_map(|c| {
            let b = c.style.borders;
            [b.top, b.right, b.bottom, b.left]
        })
        .flatten()
        .map(|s| s.width)
        .fold(0.0, f64::max);
    widest / 2.0 + 1.0
}

/// The pixels of `t` in a box of `w` × `h` px at `zoom`, with [`overhang`]
/// around it: the same drawing as the export. The words of `skip` are left
/// out, while they are written over it.
pub(crate) fn image(
    t: &TableNode,
    (w, h): (f64, f64),
    zoom: f32,
    skip: Option<(usize, usize)>,
    outlines: &mut Outlines,
) -> Option<ColorImage> {
    if !(w > 0.0 && h > 0.0) {
        return None;
    }
    let margin = overhang(t);
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
    let layout = Layout::of(t, (w, h));
    let at = kurbo::Affine::scale(scale) * kurbo::Affine::translate((margin, margin));
    let rect = |pixmap: &mut tiny_skia::Pixmap, r: kurbo::Rect, color: Color| {
        use kurbo::Shape as _;
        if let Some(path) = canvas::skia_path(&(at * r.to_path(0.1))) {
            fill(pixmap, &path, color);
        }
    };
    for (r, c) in anchors(t) {
        if let Some(color) = t.cell(r, c).and_then(|cell| cell.style.fill) {
            rect(&mut pixmap, layout.cell(t, r, c), color);
        }
    }
    for edge in edges(t, &layout) {
        let half = edge.stroke.width / 2.0;
        let line = kurbo::Rect::from_points(edge.a, edge.b).inflate(half, half);
        rect(&mut pixmap, line, edge.stroke.color);
    }
    for (r, c) in anchors(t).filter(|cell| Some(*cell) != skip) {
        let Some(cell) = t.cell(r, c).filter(|cell| !cell.text.is_empty()) else {
            continue;
        };
        let place = layout.cell(t, r, c);
        if let Some(path) = outlines.of(cell, (place.width(), place.height()), t.padding) {
            let moved = at * kurbo::Affine::translate(place.origin().to_vec2()) * &*path;
            if let Some(path) = canvas::skia_path(&moved) {
                fill(&mut pixmap, &path, cell.style.color);
            }
        }
    }
    Some(ColorImage::from_rgba_premultiplied(
        [pw as usize, ph as usize],
        pixmap.data(),
    ))
}

pub(crate) fn fill(pixmap: &mut tiny_skia::Pixmap, path: &tiny_skia::Path, color: Color) {
    let [r, g, b, a] = canvas::picker_rgba(Some(color));
    let mut paint = tiny_skia::Paint {
        anti_alias: true,
        ..tiny_skia::Paint::default()
    };
    paint.set_color_rgba8(r, g, b, a);
    pixmap.fill_path(
        path,
        &paint,
        tiny_skia::FillRule::Winding,
        tiny_skia::Transform::identity(),
        None,
    );
}

// ---- Controls ------------------------------------------------------------

/// The pen the border buttons draw with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Pen {
    pub(crate) color: Color,
    pub(crate) width: f64,
}

impl Default for Pen {
    fn default() -> Self {
        Self {
            color: crate::colors::hex(0x0018_243b),
            width: 2.0,
        }
    }
}

fn border_key(which: BorderSet) -> &'static str {
    match which {
        BorderSet::All => "table.border_all",
        BorderSet::Outer => "table.border_outer",
        BorderSet::Inner => "table.border_inner",
        BorderSet::Top => "table.border_top",
        BorderSet::Horizontal => "table.border_horizontal",
        BorderSet::Bottom => "table.border_bottom",
        BorderSet::Left => "table.border_left",
        BorderSet::Vertical => "table.border_vertical",
        BorderSet::Right => "table.border_right",
    }
}

/// The edges of a small grid of two by two that `which` lights, as
/// `(x0, y0, x1, y1)` in halves.
fn lit(which: BorderSet) -> Vec<(f32, f32, f32, f32)> {
    let h = |y: f32| (0.0, y, 2.0, y);
    let v = |x: f32| (x, 0.0, x, 2.0);
    match which {
        BorderSet::All => vec![h(0.0), h(1.0), h(2.0), v(0.0), v(1.0), v(2.0)],
        BorderSet::Outer => vec![h(0.0), h(2.0), v(0.0), v(2.0)],
        BorderSet::Inner => vec![h(1.0), v(1.0)],
        BorderSet::Top => vec![h(0.0)],
        BorderSet::Horizontal => vec![h(1.0)],
        BorderSet::Bottom => vec![h(2.0)],
        BorderSet::Left => vec![v(0.0)],
        BorderSet::Vertical => vec![v(1.0)],
        BorderSet::Right => vec![v(2.0)],
    }
}

/// A button with a small grid, the edges of `which` lit; `None` draws it
/// without any («Sin bordes»).
fn border_button(ui: &mut Ui, theme: &Theme, which: Option<BorderSet>, name: &str) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::click());
    if response.hovered() || response.has_focus() {
        ui.painter()
            .rect_filled(rect, theme.radius_md(), theme.c("secondary.bg"));
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let grid = rect.shrink(8.0);
    let half = grid.width() / 2.0;
    let at = |x: f32, y: f32| grid.min + Vec2::new(x, y) * half;
    let faint = Stroke::new(1.0, theme.c("text.muted").gamma_multiply(0.45));
    for (x0, y0, x1, y1) in lit(BorderSet::All) {
        ui.painter().line_segment([at(x0, y0), at(x1, y1)], faint);
    }
    let strong = Stroke::new(2.0, theme.c("text"));
    for (x0, y0, x1, y1) in which.map(lit).unwrap_or_default() {
        ui.painter().line_segment([at(x0, y0), at(x1, y1)], strong);
    }
    if which.is_none() {
        ui.painter()
            .line_segment([grid.left_bottom(), grid.right_top()], strong);
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
    response.on_hover_text(name).clicked()
}

/// The borders: nine sets and none, the colour and the thickness (S81-05).
pub(crate) fn border_controls(
    ui: &mut Ui,
    theme: &Theme,
    texts: &Texts,
    pen: &mut Pen,
) -> Option<Act> {
    let mut asked = None;
    let stroke = ShapeStroke::new(pen.color, pen.width);
    for line in BorderSet::ALL.chunks(3) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(4.0);
            for which in line {
                let name = plain(texts, border_key(*which));
                if border_button(ui, theme, Some(*which), &name) {
                    asked = Some(Act::Borders(*which, Some(stroke)));
                }
            }
        });
    }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if border_button(ui, theme, None, &plain(texts, "table.border_none")) {
            asked = Some(Act::Borders(BorderSet::All, None));
        }
        let mut rgba = canvas::picker_rgba(Some(pen.color));
        ui.color_edit_button_srgba_unmultiplied(&mut rgba)
            .on_hover_text(plain(texts, "table.border_color"))
            .changed()
            .then(|| pen.color = canvas::picked_color(Some(pen.color), rgba));
        ui.add(
            egui::DragValue::new(&mut pen.width)
                .range(0.5..=40.0)
                .speed(0.25)
                .max_decimals(1)
                .suffix(" px"),
        )
        .on_hover_text(plain(texts, "table.border_width"));
    });
    asked
}

/// «Espaciado de tabla» and «Espaciado de celda».
pub(crate) fn spacing_controls(
    ui: &mut Ui,
    theme: &Theme,
    texts: &Texts,
    t: &TableNode,
) -> Option<Act> {
    let mut asked = None;
    for (key, value, make) in [
        ("table.spacing", t.spacing, Act::Spacing as fn(f64) -> Act),
        ("table.padding", t.padding, Act::Padding),
    ] {
        ui.label(
            egui::RichText::new(plain(texts, key))
                .font(theme.font("body-sm"))
                .color(theme.c("text.secondary")),
        );
        let mut value = value;
        ui.spacing_mut().slider_width = (ui.available_width() - 64.0).max(80.0);
        if ui
            .add(egui::Slider::new(&mut value, 0.0..=TableNode::MAX_SPACING).max_decimals(0))
            .changed()
        {
            asked = Some(make(value));
        }
        ui.add_space(4.0);
    }
    asked
}

/// The looks of the cells `range` of `t`, taken from the first one: fill,
/// words, size and alignment, in one line.
pub(crate) fn look_controls(
    ui: &mut Ui,
    theme: &Theme,
    texts: &Texts,
    t: &TableNode,
    range: Range,
) -> Option<Act> {
    let style = t.cell(range.r0, range.c0)?.style.clone();
    let mut asked = None;
    let mut rgba = canvas::picker_rgba(style.fill);
    if ui
        .color_edit_button_srgba_unmultiplied(&mut rgba)
        .on_hover_text(plain(texts, "table.fill"))
        .changed()
    {
        asked = Some(Act::Fill(Some(canvas::picked_color(style.fill, rgba))));
    }
    let letter = |ui: &mut Ui, text: &str, name: &'static str, on: bool| {
        let label = egui::RichText::new(text).font(theme.font("ui-label"));
        ui.selectable_label(on, label)
            .on_hover_text(plain(texts, name))
            .clicked()
    };
    let mut ink = canvas::picker_rgba(Some(style.color));
    ui.push_id("ink", |ui| {
        if ui
            .color_edit_button_srgba_unmultiplied(&mut ink)
            .on_hover_text(plain(texts, "table.ink"))
            .changed()
        {
            asked = Some(Act::Ink(canvas::picked_color(Some(style.color), ink)));
        }
    });
    let mut size = style.size;
    if ui
        .add(
            egui::DragValue::new(&mut size)
                .range(1.0..=gafira_doc::TextNode::MAX_SIZE)
                .speed(0.5)
                .max_decimals(1),
        )
        .on_hover_text(plain(texts, "table.size"))
        .changed()
    {
        asked = Some(Act::Size(size));
    }
    if letter(ui, "B", "inspector.bold", style.bold) {
        asked = Some(Act::Bold(!style.bold));
    }
    if letter(ui, "I", "inspector.italic", style.italic) {
        asked = Some(Act::Italic(!style.italic));
    }
    asked
}

/// The alignment of the cells `range` of `t`, across and down.
pub(crate) fn align_controls(
    ui: &mut Ui,
    theme: &Theme,
    texts: &Texts,
    t: &TableNode,
    range: Range,
) -> Option<Act> {
    let style = t.cell(range.r0, range.c0)?.style.clone();
    let mut asked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (align, key) in [
            (TextAlign::Left, "table.left"),
            (TextAlign::Center, "table.center"),
            (TextAlign::Right, "table.right"),
        ] {
            if align_button(
                ui,
                theme,
                Lines::Across(align),
                style.align == align,
                &plain(texts, key),
            ) {
                asked = Some(Act::Align(align));
            }
        }
        for (valign, key) in [
            (VerticalAlign::Top, "table.top"),
            (VerticalAlign::Middle, "table.middle"),
            (VerticalAlign::Bottom, "table.bottom"),
        ] {
            if align_button(
                ui,
                theme,
                Lines::Down(valign),
                style.valign == valign,
                &plain(texts, key),
            ) {
                asked = Some(Act::Valign(valign));
            }
        }
    });
    asked
}

/// What an alignment button draws.
#[derive(Clone, Copy)]
enum Lines {
    Across(TextAlign),
    Down(VerticalAlign),
}

fn align_button(ui: &mut Ui, theme: &Theme, lines: Lines, on: bool, name: &str) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::click());
    if on {
        ui.painter()
            .rect_filled(rect, theme.radius_sm(), theme.c("selection.bg"));
    } else if response.hovered() || response.has_focus() {
        ui.painter()
            .rect_filled(rect, theme.radius_sm(), theme.c("secondary.bg"));
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let area = rect.shrink(6.0);
    let pen = Stroke::new(1.5, theme.c("text"));
    for (i, share) in [1.0_f32, 0.6, 0.8].into_iter().enumerate() {
        let (x, y) = match lines {
            Lines::Across(align) => {
                let room = area.width() * (1.0 - share);
                let x = match align {
                    TextAlign::Left | TextAlign::Justify => 0.0,
                    TextAlign::Center => room / 2.0,
                    TextAlign::Right => room,
                };
                (area.left() + x, area.top() + area.height() * i as f32 / 2.0)
            }
            Lines::Down(valign) => {
                if i == 2 {
                    break;
                }
                let start = match valign {
                    VerticalAlign::Top => 0.0,
                    VerticalAlign::Middle => 0.36,
                    VerticalAlign::Bottom => 0.72,
                };
                let room = area.width() * (1.0 - share) / 2.0;
                (
                    area.left() + room,
                    area.top() + area.height() * (start + 0.28 * i as f32),
                )
            }
        };
        ui.painter().line_segment(
            [Pos2::new(x, y), Pos2::new(x + area.width() * share, y)],
            pen,
        );
    }
    response
        .widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, on, name));
    response.on_hover_text(name).clicked()
}

/// What a menu of a row, a column or a cell asks for, besides changes to the
/// table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Menu {
    Copy,
    CopyStyle,
    PasteStyle,
    Duplicate,
    Delete,
    Excel,
    /// «Crear gráfico» (S83-10).
    Chart,
    Lock,
    /// «Alinear a la página», by its place in the list.
    Align(usize),
}

/// What a choice of a menu is.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Picked {
    Act(Act),
    Menu(Menu),
}

/// Which menu is open: of the rows, of the columns, or of the cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    Rows,
    Columns,
    Cells,
}

/// The menu of the cells `range` of `t` (S81-09): what cannot be done is
/// shown without being a choice.
pub(crate) fn menu(
    ui: &mut Ui,
    texts: &Texts,
    (t, placement): (&TableNode, &Placement),
    range: Range,
    side: Side,
    (can_paste_style, aligns): (bool, &[&'static str]),
) -> Option<Picked> {
    let mut picked = None;
    ui.set_width(300.0);
    let mut item = |ui: &mut Ui, key: &'static str, keys: &str, on: bool, what: Picked| {
        let button = egui::Button::new(plain(texts, key))
            .shortcut_text(keys.to_owned())
            .min_size(Vec2::new(ui.available_width(), 0.0));
        if ui.add_enabled(on, button).clicked() {
            picked = Some(what);
            ui.close();
        }
    };
    // What can always be done is not tried out: that measures every cell.
    let can = |act: &Act| {
        matches!(
            act,
            Act::FitRows | Act::FitColumns | Act::EqualRows | Act::EqualColumns | Act::Clear
        ) || change(t, placement, range, act).is_some()
    };
    item(ui, "editor.copy", "Ctrl+C", true, Picked::Menu(Menu::Copy));
    item(
        ui,
        "table.copy_style",
        "Ctrl+Alt+C",
        true,
        Picked::Menu(Menu::CopyStyle),
    );
    item(
        ui,
        "table.paste_style",
        "Ctrl+Alt+V",
        can_paste_style,
        Picked::Menu(Menu::PasteStyle),
    );
    item(
        ui,
        "table.duplicate",
        "",
        true,
        Picked::Menu(Menu::Duplicate),
    );
    item(ui, "table.delete", "", true, Picked::Menu(Menu::Delete));
    let mut group = |ui: &mut Ui, acts: &[(&'static str, Act)]| {
        ui.separator();
        for (key, act) in acts {
            item(ui, key, "", can(act), Picked::Act(act.clone()));
        }
    };
    if side != Side::Columns {
        group(
            ui,
            &[
                ("table.row_before", Act::RowBefore),
                ("table.row_after", Act::RowAfter),
                ("table.row_duplicate", Act::DuplicateRows),
                ("table.row_up", Act::MoveRow(false)),
                ("table.row_down", Act::MoveRow(true)),
                ("table.row_remove", Act::RemoveRows),
            ],
        );
    }
    if side != Side::Rows {
        group(
            ui,
            &[
                ("table.column_before", Act::ColumnBefore),
                ("table.column_after", Act::ColumnAfter),
                ("table.column_left", Act::MoveColumn(false)),
                ("table.column_right", Act::MoveColumn(true)),
                ("table.column_remove", Act::RemoveColumns),
            ],
        );
    }
    group(
        ui,
        &[
            ("table.equal_columns", Act::EqualColumns),
            ("table.equal_rows", Act::EqualRows),
            ("table.fit_rows", Act::FitRows),
            ("table.fit_columns", Act::FitColumns),
        ],
    );
    group(
        ui,
        &[
            ("table.merge", Act::Merge),
            ("table.unmerge", Act::Unmerge),
            ("table.clear", Act::Clear),
        ],
    );
    ui.separator();
    item(ui, "table.chart", "", true, Picked::Menu(Menu::Chart));
    item(ui, "table.excel", "", true, Picked::Menu(Menu::Excel));
    item(ui, "layers.lock", "", true, Picked::Menu(Menu::Lock));
    let button = egui::Button::new(plain(texts, "table.align_page"))
        .right_text("›")
        .min_size(Vec2::new(ui.available_width(), 0.0));
    egui::containers::menu::SubMenuButton::from_button(button).ui(ui, |ui| {
        for (index, key) in aligns.iter().enumerate() {
            if ui.button(plain(texts, key)).clicked() {
                picked = Some(Picked::Menu(Menu::Align(index)));
                ui.close();
            }
        }
    });
    picked
}

/// A small round button on the canvas, `mark` drawn in it.
pub(crate) fn pill(
    ui: &mut Ui,
    theme: &Theme,
    id: egui::Id,
    rect: Rect,
    mark: Mark,
    name: &str,
) -> egui::Response {
    let sense = if mark == Mark::Move {
        Sense::drag()
    } else {
        Sense::click()
    };
    let response = ui.interact(rect, id, sense);
    let fill = if response.hovered() || response.dragged() {
        theme.c("secondary.bg")
    } else {
        theme.c("surface")
    };
    let painter = ui.painter();
    painter.rect(
        rect,
        rect.width().min(rect.height()) / 2.0,
        fill,
        Stroke::new(1.0, theme.c("border")),
        egui::StrokeKind::Inside,
    );
    let ink = theme.c("text");
    let pen = Stroke::new(1.5, ink);
    let c = rect.center();
    match mark {
        Mark::Dots(step) => {
            for i in [-1.0, 0.0, 1.0] {
                painter.circle_filled(c + step * i, 1.5, ink);
            }
        }
        Mark::Plus => {
            painter.line_segment([c - Vec2::new(5.0, 0.0), c + Vec2::new(5.0, 0.0)], pen);
            painter.line_segment([c - Vec2::new(0.0, 5.0), c + Vec2::new(0.0, 5.0)], pen);
        }
        Mark::Arrow(way) => {
            let tip = c + Vec2::new(3.0 * way, 0.0);
            for side in [-5.0, 5.0] {
                painter.line_segment([tip, tip + Vec2::new(-6.0 * way, side)], pen);
            }
        }
        Mark::Move => {
            for way in [
                Vec2::new(1.0, 0.0),
                Vec2::new(-1.0, 0.0),
                Vec2::new(0.0, 1.0),
                Vec2::new(0.0, -1.0),
            ] {
                let tip = c + way * 7.0;
                painter.line_segment([c, tip], pen);
                let side = way.rot90();
                painter.line_segment([tip, tip - way * 3.0 + side * 3.0], pen);
                painter.line_segment([tip, tip - way * 3.0 - side * 3.0], pen);
            }
        }
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(if mark == Mark::Move {
            egui::CursorIcon::Grab
        } else {
            egui::CursorIcon::PointingHand
        });
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
    response.on_hover_text(name)
}

/// What a [`pill`] shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Mark {
    /// Three dots, this far apart.
    Dots(Vec2),
    Plus,
    Move,
    /// To the left (-1) or to the right (1).
    Arrow(f32),
}

/// The fields of a table in Propiedades (S81-10).
pub(crate) fn inspector(
    ui: &mut Ui,
    theme: &Theme,
    texts: &Texts,
    (t, range): (&TableNode, Range),
    pen: &mut Pen,
) -> Option<Act> {
    let mut asked = None;
    let title = |ui: &mut Ui, key: &'static str| {
        ui.add_space(12.0);
        ui.label(
            egui::RichText::new(plain(texts, key))
                .font(theme.font("body-sm"))
                .color(theme.c("text.secondary")),
        );
        ui.add_space(4.0);
    };
    ui.label(
        egui::RichText::new(if range == Range::all(t) {
            plain(texts, "table.scope_all")
        } else {
            plain(texts, "table.scope_cells")
        })
        .font(theme.font("body-sm"))
        .color(theme.c("text.secondary")),
    );
    title(ui, "table.looks");
    let line = Vec2::new(ui.available_width(), 32.0);
    let layout = egui::Layout::left_to_right(egui::Align::Center);
    ui.allocate_ui_with_layout(line, layout, |ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        asked = look_controls(ui, theme, texts, t, range);
    });
    ui.add_space(6.0);
    if let Some(act) = align_controls(ui, theme, texts, t, range) {
        asked = Some(act);
    }
    ui.add_space(6.0);
    if ui.button(plain(texts, "table.no_fill")).clicked() {
        asked = Some(Act::Fill(None));
    }
    title(ui, "table.borders");
    if let Some(act) = border_controls(ui, theme, texts, pen) {
        asked = Some(act);
    }
    title(ui, "table.room");
    if let Some(act) = spacing_controls(ui, theme, texts, t) {
        asked = Some(act);
    }
    let mut header = t.header;
    ui.horizontal(|ui| {
        if widgets::toggle(ui, theme, &mut header).changed() {
            asked = Some(Act::Header(header));
        }
        ui.label(plain(texts, "table.header"));
    });
    asked
}

/// The colour the chosen cells are ringed with.
pub(crate) fn ring(theme: &Theme) -> Color32 {
    theme.c("accent")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(rows: usize, cols: usize) -> TableNode {
        let style = CellStyle::new("Inter", 16.0, crate::colors::hex(0));
        TableNode::new(rows, cols, (100.0, 40.0), &style)
    }

    fn place(t: &TableNode) -> Placement {
        let (w, h) = gafira_table::size(t);
        Placement::new(50.0, 60.0, w, h)
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    // S81-02: the box grows with a row and its first corner stays, also
    // turned and mirrored.
    #[test]
    fn s81_02_the_corner_stays_as_the_box_grows() {
        let t = table(2, 2);
        for (turn, flip) in [(0.0, false), (30.0, false), (90.0, true)] {
            let mut p = place(&t);
            p.rotation_deg = turn;
            p.flip.x = flip;
            let (next, box_of, now) =
                change(&t, &p, Range::cell(1, 0), &Act::RowAfter).expect("a row");
            assert_eq!(next.rows.len(), 3);
            assert!(near(box_of.h, 120.0) && near(box_of.w, 200.0));
            let corner = |p: &Placement| p.to_transform() * kurbo::Point::ZERO;
            assert!((corner(&p) - corner(&box_of)).hypot() < 1e-6, "{turn}");
            assert_eq!(now, Range::cell(2, 0), "the new row is chosen");
        }
    }

    // S81-03: a row grows so its words fit, and a narrower column makes its
    // rows taller.
    #[test]
    fn s81_03_a_row_grows_with_its_words() {
        let t = table(2, 2);
        let long = "una frase bastante larga que no cabe en una sola línea".to_owned();
        let (next, box_of, _) =
            change(&t, &place(&t), Range::cell(0, 0), &Act::Words((0, 0), long)).expect("words");
        assert!(next.rows[0] > 60.0, "{:?}", next.rows);
        assert!(near(next.rows[1], 40.0), "only its own row");
        assert!(near(box_of.h, next.rows[0] + 40.0));
        let before = next.rows[0];
        let (thin, _, _) = change(
            &next,
            &box_of,
            Range::cell(0, 0),
            &Act::ResizeColumn(0, -40.0),
        )
        .expect("narrower");
        assert!(thin.rows[0] > before);
        assert!(near(thin.columns[0], 60.0) && near(thin.columns[1], 100.0));
    }

    // S81-02: a column takes the width of its longest line; a row, the
    // height of its words.
    #[test]
    fn s81_02_sizes_fit_the_words() {
        let mut t = table(2, 2);
        t.cells[0].text = "Hola".to_owned();
        let (next, box_of, _) =
            change(&t, &place(&t), Range::cell(0, 0), &Act::FitColumns).expect("a fit");
        assert!(
            (30.0..70.0).contains(&next.columns[0]),
            "{:?}",
            next.columns
        );
        let layout = Layout::of(&next, (box_of.w, box_of.h));
        assert!(needed(&next, &layout, (0, 0)) < 40.0, "still in one line");
        t.rows = vec![200.0, 40.0];
        let (next, _, _) = change(&t, &place(&t), Range::cell(0, 0), &Act::FitRows).expect("a fit");
        assert!(next.rows[0] < 50.0);
    }

    // S81-08: Tab goes cell by cell, over merges, and ends at the last one;
    // the arrows leave a merge by its far side.
    #[test]
    fn s81_08_moving_between_cells() {
        let mut t = table(2, 3);
        t.merges.push(gafira_doc::CellMerge {
            row: 0,
            col: 0,
            rows: 1,
            cols: 2,
        });
        let at = |cell| Cells::one(PageId::new(1), NodeId::new(2), cell);
        assert_eq!(at((0, 0)).tab(&t, false), Some((0, 2)));
        assert_eq!(at((0, 2)).tab(&t, false), Some((1, 0)));
        assert_eq!(at((1, 2)).tab(&t, false), None);
        assert_eq!(at((0, 0)).tab(&t, true), None);
        assert_eq!(at((0, 0)).step(&t, 0, 1), (0, 2));
        assert_eq!(at((0, 2)).step(&t, 0, -1), (0, 0), "into the merge");
        assert_eq!(at((1, 1)).step(&t, 1, 0), (1, 1), "the edge");
        assert_eq!(at((1, 1)).step(&t, -1, 0), (0, 0));
    }

    // S81-12: the canvas draws the fill, the borders and the words, and
    // leaves out those being written.
    #[test]
    fn s81_12_the_canvas_draws_the_table() {
        let mut t = table(1, 2);
        t.cells[0].style.fill = Some(crate::colors::hex(0x00ff_0000));
        t.cells[1].text = "████".to_owned();
        t.cells[1].text = "HHHH".to_owned();
        let size = gafira_table::size(&t);
        let mut outlines = Outlines::default();
        let drawn = image(&t, size, 1.0, None, &mut outlines).expect("pixels");
        assert_eq!(drawn[(40, 10)], Color32::RED);
        let dark = |image: &ColorImage| {
            image
                .pixels
                .iter()
                .filter(|p| p.a() > 200 && p.r() < 60)
                .count()
        };
        assert!(dark(&drawn) > 20, "the words");
        let without = image(&t, size, 1.0, Some((0, 1)), &mut outlines).expect("pixels");
        assert_eq!(dark(&without), 0);
    }

    // S81-06, S81-11: looks and pasted cells go where they are asked.
    #[test]
    fn s81_11_pasting_grows_the_table_and_chooses_it() {
        let t = table(1, 1);
        let grid = vec![vec!["a".to_owned(), "b".to_owned()]; 3];
        let (next, _, now) =
            change(&t, &place(&t), Range::cell(0, 0), &Act::Paste(grid)).expect("pasted");
        assert_eq!((next.rows.len(), next.columns.len()), (3, 2));
        assert_eq!(now, Range::new((0, 0), (2, 1)));
        assert!(
            change(&next, &place(&next), now, &Act::Bold(false)).is_none(),
            "as it was"
        );
        let (bold, _, _) = change(&next, &place(&next), now, &Act::Bold(true)).expect("bold");
        assert!(bold.cells.iter().all(|c| c.style.bold));
    }
}
