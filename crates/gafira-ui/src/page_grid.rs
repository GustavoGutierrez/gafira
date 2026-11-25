//! Vista de cuadrícula (spec 102): every page of the design as a tile, in rows. A page is
//! chosen with a click (Ctrl adds or removes one, Shift a run), opened with a double click,
//! moved by dragging it (the others make room and a gap shows where it lands), and a «+»
//! between two pages adds one there. A bar at the top selects all, adds, duplicates,
//! deletes and hides.

use std::collections::{BTreeMap, BTreeSet};

use egui::{Align2, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use gafira_doc::PageId;

use crate::charts::plain;
use crate::icons::{self, Icon, Weight};
use crate::style::Theme;
use crate::text::Texts;

/// The picture box of a tile, its label row, and the gaps between tiles, in points.
const TILE: Vec2 = Vec2::new(200.0, 140.0);
const LABEL: f32 = 30.0;
const GAP_X: f32 = 48.0;
const GAP_Y: f32 = 24.0;
const MARGIN: f32 = 40.0;
const TOP: f32 = 72.0;

/// A page as the grid shows it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GridPage {
    pub(crate) id: PageId,
    /// Changes whenever the page's content changes: its picture is drawn again.
    pub(crate) version: usize,
    pub(crate) hidden: bool,
    pub(crate) locked: bool,
    /// Width over height of the page: its tile takes its shape.
    pub(crate) aspect: f32,
    /// The page's title, and its size as the menu writes it («1080 × 1080 px»).
    pub(crate) name: String,
    pub(crate) size: String,
}

/// The tile of a page of `aspect` in its slot at `min`: as large as fits, centred.
fn tile_in(min: Pos2, aspect: f32) -> Rect {
    let aspect = if aspect.is_finite() && aspect > 0.0 {
        aspect
    } else {
        TILE.x / TILE.y
    };
    let size = if aspect >= TILE.x / TILE.y {
        Vec2::new(TILE.x, TILE.x / aspect)
    } else {
        Vec2::new(TILE.y * aspect, TILE.y)
    };
    Rect::from_center_size(Rect::from_min_size(min, TILE).center(), size)
}

/// What the grid asks of the editor; each is one undo step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum GridAsk {
    /// Move a page so it ends at this index.
    MoveTo(PageId, usize),
    /// An empty page at this index.
    AddAt(usize),
    Duplicate(Vec<PageId>),
    Delete(Vec<PageId>),
    Hide(Vec<PageId>, bool),
    Lock(PageId, bool),
    Rename(PageId, String),
    /// Leave the grid on this page.
    Open(PageId),
}

/// The state of the grid between frames.
#[derive(Default)]
pub(crate) struct Grid {
    pub(crate) selected: BTreeSet<PageId>,
    /// The last page clicked: the anchor of Shift and the page the editor shows on leaving.
    pub(crate) last: Option<PageId>,
    pictures: BTreeMap<PageId, (usize, egui::TextureHandle)>,
    /// The page being dragged and where in its tile it was grabbed.
    drag: Option<(PageId, Vec2)>,
    /// The page whose «⋯» menu is open, where it opens, and the title being written.
    menu: Option<(PageId, Pos2, String)>,
}

/// Where the page dragged would land: the index among the other pages whose slot centre
/// is nearest `at`, or after the last.
pub(crate) fn drop_index(slots: &[Pos2], at: Pos2) -> usize {
    let nearest = slots
        .iter()
        .enumerate()
        .min_by(|a, b| (*a.1 - at).length_sq().total_cmp(&(*b.1 - at).length_sq()));
    match nearest {
        None => 0,
        // Past the middle of the nearest slot the page goes after it.
        Some((i, c)) => {
            if at.x > c.x {
                i + 1
            } else {
                i
            }
        }
    }
}

/// The index of `n` items in rows of `cols`: column and row.
fn cell(i: usize, cols: usize) -> (usize, usize) {
    (i % cols, i / cols)
}

impl Grid {
    /// Picks `id` with a click: alone, added or removed with Ctrl, or the run from the
    /// last one with Shift.
    pub(crate) fn pick(&mut self, id: PageId, order: &[PageId], (ctrl, shift): (bool, bool)) {
        if shift
            && let Some(last) = self.last
            && let (Some(a), Some(b)) = (
                order.iter().position(|p| *p == last),
                order.iter().position(|p| *p == id),
            )
        {
            let (lo, hi) = (a.min(b), a.max(b));
            self.selected.extend(order[lo..=hi].iter().copied());
        } else if ctrl {
            if !self.selected.remove(&id) {
                self.selected.insert(id);
            }
        } else {
            self.selected = BTreeSet::from([id]);
        }
        self.last = Some(id);
    }

    /// The selected pages in the design's order.
    fn chosen(&self, pages: &[GridPage]) -> Vec<PageId> {
        pages
            .iter()
            .map(|p| p.id)
            .filter(|p| self.selected.contains(p))
            .collect()
    }

    /// Draws the grid over `area`. `picture` draws a page's thumbnail; one new picture is
    /// drawn a frame so the grid never stalls.
    pub(crate) fn show(
        &mut self,
        ui: &mut egui::Ui,
        (theme, texts): (&Theme, &Texts),
        pages: &[GridPage],
        picture: &mut dyn FnMut(PageId) -> Option<egui::ColorImage>,
    ) -> Vec<GridAsk> {
        let mut asks = Vec::new();
        let order: Vec<PageId> = pages.iter().map(|p| p.id).collect();
        self.selected.retain(|p| order.contains(p));
        self.pictures.retain(|p, _| order.contains(p));
        let area = ui.max_rect();
        ui.painter().rect_filled(area, 0.0, theme.c("workspace"));

        // One new or outdated picture a frame.
        if let Some(page) = pages.iter().find(|p| {
            self.pictures
                .get(&p.id)
                .is_none_or(|(v, _)| *v != p.version)
        }) {
            if let Some(image) = picture(page.id) {
                let texture = ui.ctx().load_texture(
                    format!("grid-{:?}", page.id),
                    image,
                    egui::TextureOptions::LINEAR,
                );
                self.pictures.insert(page.id, (page.version, texture));
            }
            ui.ctx().request_repaint();
        }

        // Keys: Delete removes the chosen pages, Ctrl+A chooses all.
        let (delete, all) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::Delete),
                i.modifiers.command && i.key_pressed(egui::Key::A),
            )
        });
        if all {
            self.selected = order.iter().copied().collect();
        }
        if delete && !self.selected.is_empty() {
            asks.push(GridAsk::Delete(self.chosen(pages)));
        }

        let step = Vec2::new(TILE.x + GAP_X, TILE.y + LABEL + GAP_Y);
        let cols = (((area.width() - 2.0 * MARGIN + GAP_X) / step.x).floor() as usize).max(1);
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let mods = ui.input(|i| (i.modifiers.command, i.modifiers.shift));

        egui::ScrollArea::vertical()
            .id_salt("page_grid")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // The pages laid out without the one dragged, a gap where it would land.
                let dragged = self.drag.map(|(id, _)| id);
                let others: Vec<&GridPage> =
                    pages.iter().filter(|p| Some(p.id) != dragged).collect();
                let count = others.len() + usize::from(dragged.is_some()) + 1;
                let rows = count.div_ceil(cols);
                let width = cols as f32 * step.x - GAP_X;
                let height = TOP + rows as f32 * step.y + MARGIN;
                let (content, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
                let left = content.left() + ((content.width() - width) / 2.0).max(MARGIN);
                let at = |i: usize| {
                    let (c, r) = cell(i, cols);
                    Pos2::new(
                        left + c as f32 * step.x,
                        content.top() + TOP + r as f32 * step.y,
                    )
                };
                let slots: Vec<Pos2> = (0..others.len())
                    .map(|i| Rect::from_min_size(at(i), TILE).center())
                    .collect();
                let target = match (dragged, pointer) {
                    (Some(_), Some(p)) => Some(drop_index(&slots, p).min(others.len())),
                    _ => None,
                };
                let shown_at = |i: usize| match target {
                    Some(t) if i >= t => at(i + 1),
                    _ => at(i),
                };

                let painter = ui.painter().clone();
                let accent = theme.c("primary.bg");
                for (i, page) in others.iter().enumerate() {
                    let tile = tile_in(shown_at(i), page.aspect);
                    let response = ui.interact(
                        tile,
                        ui.id().with(("tile", page.id)),
                        Sense::click_and_drag(),
                    );
                    let on = self.selected.contains(&page.id);
                    painter.rect_filled(tile, 4.0, theme.c("surface"));
                    if let Some((_, texture)) = self.pictures.get(&page.id) {
                        draw_picture(&painter, texture, tile, page.hidden);
                    }
                    let stroke = if on {
                        Stroke::new(2.5, accent)
                    } else if response.hovered() {
                        Stroke::new(1.5, theme.c("border.strong"))
                    } else {
                        Stroke::new(1.0, theme.c("border"))
                    };
                    painter.rect_stroke(tile, 4.0, stroke, egui::StrokeKind::Outside);
                    // The page number, and an eye crossed out on a hidden page.
                    let number = order.iter().position(|p| *p == page.id).unwrap_or(i) + 1;
                    let label = Pos2::new(tile.left() + 2.0, tile.bottom() + 16.0);
                    let galley = painter.text(
                        label,
                        Align2::LEFT_CENTER,
                        number.to_string(),
                        theme.font("ui-label"),
                        if on {
                            theme.c("text")
                        } else {
                            theme.c("text.secondary")
                        },
                    );
                    if page.hidden {
                        let eye = Rect::from_center_size(
                            Pos2::new(galley.right() + 14.0, label.y),
                            Vec2::splat(16.0),
                        );
                        icons::paint(
                            &painter,
                            eye,
                            Icon::Hidden,
                            Weight::Regular,
                            theme.c("text.muted"),
                        );
                    }
                    if response.double_clicked() {
                        asks.push(GridAsk::Open(page.id));
                    } else if response.clicked() {
                        self.pick(page.id, &order, mods);
                    }
                    if response.drag_started()
                        && let Some(p) = pointer
                    {
                        if !self.selected.contains(&page.id) {
                            self.pick(page.id, &order, (false, false));
                        }
                        self.drag = Some((page.id, p - Rect::from_min_size(shown_at(i), TILE).min));
                    }
                    // «⋯» on the tile under the pointer, or with its menu open.
                    let menu_open = self.menu.as_ref().is_some_and(|(p, _, _)| *p == page.id);
                    let hovering = pointer.is_some_and(|p| tile.contains(p));
                    let mut on_dots = false;
                    if (hovering || menu_open) && self.drag.is_none() {
                        let dots = Rect::from_min_size(
                            Pos2::new(tile.right() - 34.0, tile.top() + 6.0),
                            Vec2::new(28.0, 20.0),
                        );
                        let hit =
                            ui.interact(dots, ui.id().with(("dots", page.id)), Sense::click());
                        on_dots = hit.hovered();
                        painter.rect_filled(
                            dots,
                            6.0,
                            if hit.hovered() || menu_open {
                                accent
                            } else {
                                theme.c("topbar.bg")
                            },
                        );
                        for dx in [-6.0, 0.0, 6.0] {
                            painter.circle_filled(
                                dots.center() + Vec2::new(dx, 0.0),
                                1.8,
                                Color32::WHITE,
                            );
                        }
                        if hit.on_hover_text(plain(texts, "grid.menu")).clicked() {
                            self.menu = if menu_open {
                                None
                            } else {
                                Some((
                                    page.id,
                                    dots.left_bottom() + Vec2::new(0.0, 6.0),
                                    page.name.clone(),
                                ))
                            };
                            if !self.selected.contains(&page.id) {
                                self.pick(page.id, &order, (false, false));
                            }
                        }
                    }
                    if response.hovered() && self.drag.is_none() && !on_dots {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                    }
                }

                // The gap where the dragged page lands, and the page following the pointer.
                if let (Some(id), Some(t), Some(p)) = (dragged, target, pointer) {
                    let aspect = pages.iter().find(|q| q.id == id).map_or(1.0, |q| q.aspect);
                    let gap = tile_in(at(t), aspect);
                    painter.rect_filled(gap, 4.0, theme.c("secondary.bg.hover"));
                    painter.rect_stroke(
                        gap,
                        4.0,
                        Stroke::new(1.0, theme.c("border")),
                        egui::StrokeKind::Inside,
                    );
                    painter.line_segment(
                        [
                            Pos2::new(gap.left() - 12.0, gap.top() - 8.0),
                            Pos2::new(gap.left() - 12.0, gap.bottom() + 8.0),
                        ],
                        Stroke::new(2.0, accent),
                    );
                    let grab = self.drag.map_or(Vec2::ZERO, |(_, g)| g);
                    let aspect = pages.iter().find(|q| q.id == id).map_or(1.0, |q| q.aspect);
                    let ghost = tile_in(p - grab, aspect);
                    let top = ui.ctx().layer_painter(egui::LayerId::new(
                        egui::Order::Tooltip,
                        egui::Id::new("grid.ghost"),
                    ));
                    top.rect_filled(
                        ghost.translate(Vec2::new(0.0, 8.0)),
                        10.0,
                        Color32::from_black_alpha(40),
                    );
                    top.rect_filled(ghost, 10.0, theme.c("surface"));
                    if let Some((_, texture)) = self.pictures.get(&id) {
                        draw_picture(&top, texture, ghost, false);
                    }
                    top.rect_stroke(
                        ghost,
                        10.0,
                        Stroke::new(2.5, accent),
                        egui::StrokeKind::Outside,
                    );
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    if ui.input(|i| i.pointer.any_released()) {
                        let from = order.iter().position(|p| *p == id);
                        if from != Some(t) {
                            asks.push(GridAsk::MoveTo(id, t));
                        }
                        self.drag = None;
                    }
                } else if self.drag.is_some() && !ui.input(|i| i.pointer.any_down()) {
                    self.drag = None;
                }

                // «+» between two pages of a row, where the pointer is (spec 102).
                if self.drag.is_none() {
                    for i in 1..others.len() {
                        let (c, _) = cell(i, cols);
                        if c == 0 {
                            continue;
                        }
                        // The room between the two pictures as drawn.
                        let before = tile_in(at(i - 1), others[i - 1].aspect);
                        let after = tile_in(at(i), others[i].aspect);
                        let gap = Rect::from_min_max(
                            Pos2::new(before.right(), before.top().min(after.top())),
                            Pos2::new(after.left(), before.bottom().max(after.bottom())),
                        );
                        if pointer.is_some_and(|p| gap.contains(p)) {
                            let button = Rect::from_center_size(gap.center(), Vec2::splat(32.0));
                            let response =
                                ui.interact(button, ui.id().with(("add", i)), Sense::click());
                            painter.rect_filled(
                                button,
                                8.0,
                                if response.hovered() {
                                    theme.c("secondary.bg.hover")
                                } else {
                                    theme.c("secondary.bg")
                                },
                            );
                            icons::paint(
                                &painter,
                                button.shrink(7.0),
                                Icon::ZoomIn,
                                Weight::Regular,
                                theme.c("text"),
                            );
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            if response
                                .on_hover_text(plain(texts, "grid.add_here"))
                                .clicked()
                            {
                                asks.push(GridAsk::AddAt(i));
                            }
                        }
                    }
                    // A tile that adds a page at the end.
                    let end = Rect::from_min_size(at(others.len()), TILE);
                    let response = ui.interact(end, ui.id().with("add_end"), Sense::click());
                    painter.rect_filled(
                        end,
                        10.0,
                        if response.hovered() {
                            theme.c("secondary.bg.pressed")
                        } else {
                            theme.c("secondary.bg.hover")
                        },
                    );
                    icons::paint(
                        &painter,
                        Rect::from_center_size(end.center(), Vec2::splat(26.0)),
                        Icon::ZoomIn,
                        Weight::Regular,
                        theme.c("text"),
                    );
                    if response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if response
                        .on_hover_text(plain(texts, "grid.add_page"))
                        .clicked()
                    {
                        asks.push(GridAsk::AddAt(order.len()));
                    }
                }
            });

        asks.extend(self.bar(ui.ctx(), (theme, texts), area, pages));
        asks.extend(self.page_menu(ui.ctx(), (theme, texts), pages));
        asks
    }

    /// The «⋯» menu of a page: its title and size, duplicate, delete, add a page after,
    /// hide and lock (S102-07).
    fn page_menu(
        &mut self,
        ctx: &egui::Context,
        (theme, texts): (&Theme, &Texts),
        pages: &[GridPage],
    ) -> Vec<GridAsk> {
        let mut asks = Vec::new();
        let Some((id, at, mut title)) = self.menu.clone() else {
            return asks;
        };
        let Some(index) = pages.iter().position(|p| p.id == id) else {
            self.menu = None;
            return asks;
        };
        let page = pages[index].clone();
        let mut close = false;
        let area = egui::Area::new(egui::Id::new("grid.page_menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(at)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.set_min_width(260.0);
                        ui.set_max_width(260.0);
                        crate::widgets::menu_style(ui, theme);
                        // The title, written in place: Enter or leaving the field keeps it.
                        let hint = texts.t(&gafira_doc::msg!(
                            "grid.page_number",
                            number = Count((index + 1) as u64)
                        ));
                        let field = ui.add(
                            crate::widgets::field(&mut title, hint)
                                .font(theme.font("ui-label"))
                                .desired_width(244.0),
                        );
                        if field.lost_focus() && title.trim() != page.name.trim() {
                            asks.push(GridAsk::Rename(id, title.trim().to_owned()));
                        }
                        ui.label(
                            RichText::new(&page.size)
                                .font(theme.font("caption"))
                                .color(theme.c("text.secondary")),
                        );
                        ui.separator();
                        let item = |ui: &mut egui::Ui,
                                    icon: Icon,
                                    key: &'static str,
                                    keys: Option<String>,
                                    on: bool| {
                            crate::editor::menu_item(ui, theme, (icon, plain(texts, key)), keys, on)
                        };
                        if item(ui, Icon::PageCopy, "grid.duplicate_page", None, true) {
                            asks.push(GridAsk::Duplicate(vec![id]));
                            close = true;
                        }
                        if item(
                            ui,
                            Icon::Delete,
                            "grid.delete_page",
                            Some(plain(texts, "layers.delete_key")),
                            pages.len() > 1,
                        ) {
                            asks.push(GridAsk::Delete(vec![id]));
                            close = true;
                        }
                        ui.separator();
                        if item(ui, Icon::PageAdd, "grid.add_page", None, true) {
                            asks.push(GridAsk::AddAt(index + 1));
                            close = true;
                        }
                        let (icon, key) = if page.hidden {
                            (Icon::Visible, "grid.show_page")
                        } else {
                            (Icon::Hidden, "grid.hide_page")
                        };
                        if item(ui, icon, key, None, true) {
                            asks.push(GridAsk::Hide(vec![id], !page.hidden));
                            close = true;
                        }
                        let (icon, key) = if page.locked {
                            (Icon::Unlocked, "grid.unlock_page")
                        } else {
                            (Icon::Locked, "grid.lock_page")
                        };
                        if item(ui, icon, key, None, true) {
                            asks.push(GridAsk::Lock(id, !page.locked));
                            close = true;
                        }
                    });
            });
        // A click elsewhere or Esc closes it.
        let outside = ctx.input(|i| i.pointer.any_pressed())
            && ctx
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|p| !area.response.rect.contains(p));
        let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        if close
            || esc
            || (outside
                && !ctx
                    .input(|i| i.pointer.interact_pos())
                    .is_some_and(|p| (p - at).length() < 30.0))
        {
            if !close && title.trim() != page.name.trim() {
                asks.push(GridAsk::Rename(id, title.trim().to_owned()));
            }
            self.menu = None;
        } else if let Some(menu) = self.menu.as_mut() {
            menu.2 = title;
        }
        asks
    }

    /// The bar at the top: select all, add after the selection, duplicate, delete, hide.
    fn bar(
        &mut self,
        ctx: &egui::Context,
        (theme, texts): (&Theme, &Texts),
        area: Rect,
        pages: &[GridPage],
    ) -> Vec<GridAsk> {
        let mut asks = Vec::new();
        let id = egui::Id::new("grid.bar");
        let width = ctx.memory(|m| m.area_rect(id)).map_or(320.0, |r| r.width());
        let chosen = self.chosen(pages);
        let any = !chosen.is_empty();
        egui::Area::new(id)
            .order(egui::Order::Foreground)
            .fixed_pos(Pos2::new(area.center().x - width / 2.0, area.top() + 14.0))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .corner_radius(theme.radius_md())
                    .inner_margin(egui::Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let all = egui::Button::new(
                                RichText::new(plain(texts, "grid.select_all"))
                                    .font(theme.font("ui-label")),
                            )
                            .frame(false);
                            if ui.add(all).clicked() {
                                self.selected = pages.iter().map(|p| p.id).collect();
                            }
                            let tool =
                                |ui: &mut egui::Ui, icon: Icon, key: &'static str, on: bool| {
                                    ui.add_enabled_ui(on, |ui| {
                                        crate::widgets::IconButton::new(icon, plain(texts, key))
                                            .small()
                                            .show(ui, theme)
                                    })
                                    .inner
                                    .clicked()
                                        && on
                                };
                            if tool(ui, Icon::PageAdd, "grid.add_page", true) {
                                let after =
                                    pages.iter().rposition(|p| self.selected.contains(&p.id));
                                asks.push(GridAsk::AddAt(after.map_or(pages.len(), |i| i + 1)));
                            }
                            if tool(ui, Icon::PageCopy, "grid.duplicate", any) {
                                asks.push(GridAsk::Duplicate(chosen.clone()));
                            }
                            if tool(
                                ui,
                                Icon::Delete,
                                "grid.delete",
                                any && chosen.len() < pages.len(),
                            ) {
                                asks.push(GridAsk::Delete(chosen.clone()));
                            }
                            let hidden = any
                                && pages
                                    .iter()
                                    .filter(|p| self.selected.contains(&p.id))
                                    .all(|p| p.hidden);
                            let (icon, key) = if hidden {
                                (Icon::Visible, "grid.show")
                            } else {
                                (Icon::Hidden, "grid.hide")
                            };
                            if tool(ui, icon, key, any) {
                                asks.push(GridAsk::Hide(chosen.clone(), !hidden));
                            }
                        });
                    });
            });
        asks
    }
}

/// A page's picture contained in `room`, faded when the page is hidden.
fn draw_picture(painter: &egui::Painter, texture: &egui::TextureHandle, room: Rect, hidden: bool) {
    let size = texture.size_vec2();
    let k = (room.width() / size.x).min(room.height() / size.y);
    let rect = Rect::from_center_size(room.center(), size * k);
    // The picture is the tile: no white margin around it.
    let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    let tint = if hidden {
        Color32::WHITE.gamma_multiply(0.45)
    } else {
        Color32::WHITE
    };
    painter.image(texture.id(), rect, uv, tint);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(n: u64) -> Vec<PageId> {
        (1..=n).map(PageId::new).collect()
    }

    // S102-03: a click picks one page, Ctrl adds or removes, Shift takes a run.
    #[test]
    fn s102_03_pages_are_picked() {
        let order = ids(5);
        let mut g = Grid::default();
        g.pick(PageId::new(2), &order, (false, false));
        g.pick(PageId::new(4), &order, (true, false));
        assert_eq!(g.selected, BTreeSet::from([PageId::new(2), PageId::new(4)]));
        g.pick(PageId::new(4), &order, (true, false));
        assert_eq!(g.selected, BTreeSet::from([PageId::new(2)]));
        g.pick(PageId::new(5), &order, (false, true));
        // The run starts at the page clicked last (4): pages 4 and 5 join page 2.
        assert_eq!(
            g.selected,
            BTreeSet::from([PageId::new(2), PageId::new(4), PageId::new(5)])
        );
        g.pick(PageId::new(1), &order, (false, false));
        assert_eq!(g.selected, BTreeSet::from([PageId::new(1)]));
        assert_eq!(g.last, Some(PageId::new(1)));
    }

    // S102-04: the page dragged lands at the nearest slot, before or after it.
    #[test]
    fn s102_04_the_drop_index_follows_the_pointer() {
        let slots = [
            Pos2::new(100.0, 70.0),
            Pos2::new(350.0, 70.0),
            Pos2::new(100.0, 270.0),
        ];
        assert_eq!(drop_index(&slots, Pos2::new(60.0, 70.0)), 0);
        assert_eq!(drop_index(&slots, Pos2::new(140.0, 70.0)), 1);
        assert_eq!(drop_index(&slots, Pos2::new(400.0, 80.0)), 2);
        assert_eq!(drop_index(&slots, Pos2::new(120.0, 280.0)), 3);
        assert_eq!(drop_index(&[], Pos2::new(5.0, 5.0)), 0);
    }
}
