//! The strip of document tabs over the window (spec 111): one tab per open
//! project with its file name and a mark when it has unsaved changes, a
//! close button on each and «+» to open or create another.

use egui::{Align2, Color32, Pos2, Rect, Sense, Stroke, Vec2};

use crate::icons::{self, Icon, Weight};
use crate::style::Theme;

/// What the strip asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TabAsk {
    /// Show the tab of this session.
    Show(String),
    /// Close the tab of this session.
    Close(String),
    /// Go to Inicio to open or create another project.
    New,
    /// Put the tab of this session at this place of the strip.
    Move(String, usize),
    /// Close every tab (right-click menu).
    CloseAll,
    /// Close every tab but this one.
    CloseOthers(String),
    /// «Abrir…» a file in a new tab.
    Open,
    /// Show the file of this session in its folder.
    Reveal(String),
}

/// The words of the strip, in the language of the interface.
#[derive(Clone)]
pub(crate) struct Words {
    pub(crate) new: String,
    pub(crate) close: String,
    pub(crate) close_all: String,
    pub(crate) close_others: String,
    pub(crate) new_document: String,
    pub(crate) open: String,
    pub(crate) reveal: String,
}

/// One tab as the strip draws it.
#[derive(Clone)]
pub(crate) struct TabView {
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) modified: bool,
    pub(crate) active: bool,
    /// Whether it lives in a file, to show in its folder.
    pub(crate) saved: bool,
}

/// The height of the strip, in px.
const HEIGHT: f32 = 30.0;

/// The widest and narrowest a tab gets, in px.
const WIDEST: f32 = 240.0;
const NARROWEST: f32 = 96.0;

/// What the shell hands the screens for the strip: the tabs and the hint
/// of «+».
#[derive(Clone)]
struct Offer {
    tabs: std::sync::Arc<Vec<TabView>>,
    words: std::sync::Arc<Words>,
}

fn offer_id() -> egui::Id {
    egui::Id::new("shell.tabs.offer")
}

fn ask_id() -> egui::Id {
    egui::Id::new("shell.tabs.ask")
}

/// The shell offers `tabs` for this frame; with none, the strip hides.
pub(crate) fn offer(ctx: &egui::Context, tabs: Vec<TabView>, words: Words) {
    ctx.data_mut(|d| {
        if tabs.is_empty() {
            d.remove::<Offer>(offer_id());
        } else {
            d.insert_temp(
                offer_id(),
                Offer {
                    tabs: std::sync::Arc::new(tabs),
                    words: std::sync::Arc::new(words),
                },
            );
        }
    });
}

/// What the strip was asked this frame, taken once.
pub(crate) fn take_ask(ctx: &egui::Context) -> Option<TabAsk> {
    ctx.data_mut(|d| {
        let ask = d.get_temp::<TabAsk>(ask_id());
        d.remove::<TabAsk>(ask_id());
        ask
    })
}

/// The strip right under a screen's top bar, when the shell offered tabs
/// (owner request: under the menus, not over them). Every top bar calls it
/// after its own panel.
pub(crate) fn under_topbar(ctx: &egui::Context, theme: &Theme) {
    let Some(offer) = ctx.data(|d| d.get_temp::<Offer>(offer_id())) else {
        return;
    };
    if let Some(ask) = strip(ctx, theme, &offer.tabs, &offer.words) {
        ctx.data_mut(|d| d.insert_temp(ask_id(), ask));
    }
}

/// Draws the strip of `tabs`; what was asked.
fn strip(ctx: &egui::Context, theme: &Theme, tabs: &[TabView], words: &Words) -> Option<TabAsk> {
    let mut asked = None;
    egui::TopBottomPanel::top("shell.tabs")
        .exact_height(HEIGHT)
        .frame(
            egui::Frame::NONE
                .fill(theme.c("surface.sunken"))
                .stroke(Stroke::new(1.0, theme.c("border"))),
        )
        .show(ctx, |ui| {
            let whole = ui.max_rect();
            let room = (whole.width() - 48.0).max(NARROWEST);
            let count = tabs.len().max(1) as f32;
            let width = (room / count).clamp(NARROWEST, WIDEST);
            egui::ScrollArea::horizontal()
                .id_salt("shell.tabs.scroll")
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        let left = ui.cursor().left();
                        for (index, tab) in tabs.iter().enumerate() {
                            match one(ui, theme, (tab, words), width) {
                                Some(TabAsk::Move(key, _)) => {
                                    // Held and carried sideways: the place
                                    // under the pointer (owner request).
                                    let x = ui.input(|i| i.pointer.latest_pos()).map(|p| p.x);
                                    let to = x.map_or(index, |x| {
                                        (((x - left) / width).floor().max(0.0) as usize)
                                            .min(tabs.len() - 1)
                                    });
                                    if to != index {
                                        asked = Some(TabAsk::Move(key, to));
                                    }
                                }
                                Some(ask) => asked = Some(ask),
                                None => {}
                            }
                        }
                        ui.add_space(4.0);
                        let plus = ui
                            .add(
                                egui::Button::image(icons::image(
                                    ui.ctx(),
                                    Icon::PageAdd,
                                    Weight::Regular,
                                    15.0,
                                    theme.c("text.secondary"),
                                ))
                                .frame(false)
                                .min_size(Vec2::new(30.0, HEIGHT)),
                            )
                            .on_hover_text(&words.new);
                        if plus.clicked() {
                            asked = Some(TabAsk::New);
                        }
                    });
                });
        });
    asked
}

/// One tab, `width` across: its name, the unsaved mark and the close
/// button; what was asked.
fn one(
    ui: &mut egui::Ui,
    theme: &Theme,
    (tab, words): (&TabView, &Words),
    width: f32,
) -> Option<TabAsk> {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(width, HEIGHT), Sense::click_and_drag());
    let hovered = response.hovered();
    let fill = if tab.active {
        theme.c("bg")
    } else if hovered {
        theme.c("surface")
    } else {
        Color32::TRANSPARENT
    };
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, fill);
    // A line between tabs; the active one is marked on its top edge.
    painter.vline(
        rect.right(),
        rect.y_range().shrink(7.0),
        Stroke::new(1.0, theme.c("border")),
    );
    if tab.active {
        painter.hline(
            rect.x_range(),
            rect.top() + 1.0,
            Stroke::new(2.0, theme.c("primary.bg")),
        );
    }
    let ink = if tab.active {
        theme.c("text")
    } else {
        theme.c("text.secondary")
    };
    // The close button on the right; the unsaved mark takes its place
    // until the pointer is over the tab.
    let close = Rect::from_center_size(
        Pos2::new(rect.right() - 15.0, rect.center().y),
        Vec2::splat(16.0),
    );
    let close_hit = ui.interact(close, response.id.with("close"), Sense::click());
    if tab.modified && !hovered && !close_hit.hovered() {
        painter.circle_filled(close.center(), 4.0, ink);
    } else {
        if close_hit.hovered() {
            painter.rect_filled(close, 4.0, ink.gamma_multiply(0.15));
        }
        icons::paint(
            painter,
            close.shrink(3.0),
            Icon::Close,
            Weight::Regular,
            ink,
        );
    }
    // The name, cut with an ellipsis to fit before the close button.
    let room = (close.left() - rect.left() - 18.0).max(10.0);
    let font = theme.font("body-sm");
    let fits = |text: &str| {
        painter
            .layout_no_wrap(text.to_owned(), font.clone(), ink)
            .size()
            .x
            <= room
    };
    let shown = if fits(&tab.name) {
        tab.name.clone()
    } else {
        let chars: Vec<char> = tab.name.chars().collect();
        (1..chars.len())
            .rev()
            .map(|n| format!("{}…", chars[..n].iter().collect::<String>()))
            .find(|s| fits(s))
            .unwrap_or_else(|| "…".to_owned())
    };
    painter.text(
        Pos2::new(rect.left() + 12.0, rect.center().y),
        Align2::LEFT_CENTER,
        shown,
        font,
        ink,
    );
    if hovered || close_hit.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let response = response.on_hover_text(&tab.name);
    // The right-click menu of a tab (owner request).
    let mut chosen = None;
    response.context_menu(|ui| {
        let key = || tab.key.clone();
        for (label, ask) in [
            (&words.close, TabAsk::Close(key())),
            (&words.close_all, TabAsk::CloseAll),
            (&words.close_others, TabAsk::CloseOthers(key())),
        ] {
            if ui.button(label).clicked() {
                chosen = Some(ask);
                ui.close();
            }
        }
        ui.separator();
        for (label, ask) in [
            (&words.new_document, TabAsk::New),
            (&words.open, TabAsk::Open),
        ] {
            if ui.button(label).clicked() {
                chosen = Some(ask);
                ui.close();
            }
        }
        ui.separator();
        if ui
            .add_enabled(tab.saved, egui::Button::new(&words.reveal))
            .clicked()
        {
            chosen = Some(TabAsk::Reveal(key()));
            ui.close();
        }
    });
    if chosen.is_some() {
        return chosen;
    }
    if response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        // The strip works out the place under the pointer.
        return Some(TabAsk::Move(tab.key.clone(), 0));
    }
    if close_hit.clicked() || response.middle_clicked() {
        Some(TabAsk::Close(tab.key.clone()))
    } else if response.clicked() {
        Some(TabAsk::Show(tab.key.clone()))
    } else {
        None
    }
}

/// `order` with `key` moved to place `to` (spec 111).
pub(crate) fn moved(order: &mut Vec<String>, key: &str, to: usize) {
    if let Some(at) = order.iter().position(|k| k == key) {
        let key = order.remove(at);
        order.insert(to.min(order.len()), key);
    }
}

/// The tab after (`forward`) or before the one of `active` in `order`,
/// going round; `None` with fewer than two tabs.
pub(crate) fn next(order: &[String], active: &str, forward: bool) -> Option<String> {
    if order.len() < 2 {
        return None;
    }
    let at = order.iter().position(|k| k == active)?;
    let n = order.len();
    let to = if forward {
        (at + 1) % n
    } else {
        (at + n - 1) % n
    };
    order.get(to).cloned()
}

/// The tab to show after the one at `index` of a strip closes: the one on
/// its left, or else the first.
pub(crate) fn after_close(order: &[String], index: usize) -> Option<String> {
    order
        .get(index.saturating_sub(1))
        .or_else(|| order.first())
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(n: usize) -> Vec<String> {
        (0..n).map(|k| format!("k{k}")).collect()
    }

    // S111-09: Ctrl+Tab goes round the tabs both ways.
    #[test]
    fn s111_09_tabs_go_round() {
        let order = keys(3);
        assert_eq!(next(&order, "k0", true).as_deref(), Some("k1"));
        assert_eq!(next(&order, "k2", true).as_deref(), Some("k0"));
        assert_eq!(next(&order, "k0", false).as_deref(), Some("k2"));
        assert_eq!(next(&keys(1), "k0", true), None);
        assert_eq!(next(&order, "zz", true), None);
    }

    // Owner request: a tab held and carried sideways takes its new place.
    #[test]
    fn s111_07_a_tab_is_carried_to_its_place() {
        let mut order = keys(4);
        moved(&mut order, "k0", 2);
        assert_eq!(order, ["k1", "k2", "k0", "k3"]);
        moved(&mut order, "k3", 0);
        assert_eq!(order, ["k3", "k1", "k2", "k0"]);
        moved(&mut order, "k1", 9);
        assert_eq!(order, ["k3", "k2", "k0", "k1"]);
        moved(&mut order, "zz", 0);
        assert_eq!(order.len(), 4);
    }

    // S111-07: closing a tab shows the one on its left, or the first.
    #[test]
    fn s111_07_closing_shows_the_left_neighbour() {
        // `order` without the closed tab, which was at `index`.
        let order = keys(3);
        assert_eq!(after_close(&order, 2).as_deref(), Some("k1"));
        assert_eq!(after_close(&order, 0).as_deref(), Some("k0"));
        assert_eq!(after_close(&[], 0), None);
    }
}
