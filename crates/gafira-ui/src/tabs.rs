//! The strip of document tabs over the window (spec 111): one tab per open
//! project with its file name and a mark when it has unsaved changes, a
//! close button on each and «+» to open or create another.

use egui::{Align2, Color32, Pos2, Rect, Sense, Stroke, Vec2};

use crate::icons::{self, Icon, Weight};
use crate::style::Theme;
use crate::text::Texts;

/// What the strip asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TabAsk {
    /// Show the tab of this session.
    Show(String),
    /// Close the tab of this session.
    Close(String),
    /// Go to Inicio to open or create another project.
    New,
}

/// One tab as the strip draws it.
pub(crate) struct TabView {
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) modified: bool,
    pub(crate) active: bool,
}

/// The height of the strip, in px.
const HEIGHT: f32 = 34.0;

/// The widest and narrowest a tab gets, in px.
const WIDEST: f32 = 220.0;
const NARROWEST: f32 = 96.0;

/// Draws the strip of `tabs` at the top of the window; what was asked.
pub(crate) fn strip(
    ctx: &egui::Context,
    theme: &Theme,
    texts: &Texts,
    tabs: &[TabView],
) -> Option<TabAsk> {
    let mut asked = None;
    egui::TopBottomPanel::top("shell.tabs")
        .exact_height(HEIGHT)
        .frame(egui::Frame::NONE.fill(theme.c("topbar.bg")))
        .show(ctx, |ui| {
            // The empty part of the strip moves the window, as the top bar.
            let whole = ui.max_rect();
            let drag = ui.interact(whole, egui::Id::new("shell.tabs.drag"), Sense::drag());
            if drag.drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            let room = (whole.width() - 48.0).max(NARROWEST);
            let count = tabs.len().max(1) as f32;
            let width = (room / count).clamp(NARROWEST, WIDEST);
            egui::ScrollArea::horizontal()
                .id_salt("shell.tabs.scroll")
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        ui.add_space(6.0);
                        for tab in tabs {
                            if let Some(ask) = one(ui, theme, tab, width) {
                                asked = Some(ask);
                            }
                        }
                        let plus = ui
                            .add(
                                egui::Button::image(icons::image(
                                    ui.ctx(),
                                    Icon::PageAdd,
                                    Weight::Regular,
                                    16.0,
                                    theme.c("topbar.fg.muted"),
                                ))
                                .frame(false)
                                .min_size(Vec2::new(32.0, HEIGHT - 6.0)),
                            )
                            .on_hover_text(plain(texts, "tabs.new"));
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
fn one(ui: &mut egui::Ui, theme: &Theme, tab: &TabView, width: f32) -> Option<TabAsk> {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, HEIGHT - 6.0), Sense::click());
    let rect = rect.translate(Vec2::new(0.0, 3.0));
    let hovered = response.hovered();
    let fill = if tab.active {
        theme.c("bg")
    } else if hovered {
        theme.c("topbar.hover")
    } else {
        Color32::TRANSPARENT
    };
    let painter = ui.painter();
    painter.rect(
        rect,
        egui::CornerRadius {
            nw: 8,
            ne: 8,
            sw: 0,
            se: 0,
        },
        fill,
        Stroke::NONE,
        egui::StrokeKind::Inside,
    );
    let ink = if tab.active {
        theme.c("text")
    } else {
        theme.c("topbar.fg")
    };
    // The close button on the right; the unsaved mark takes its place
    // until the pointer is over the tab.
    let close = Rect::from_center_size(
        Pos2::new(rect.right() - 16.0, rect.center().y),
        Vec2::splat(18.0),
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
            Icon::Delete,
            Weight::Regular,
            ink,
        );
    }
    // The name, cut with an ellipsis to fit before the close button.
    let room = (close.left() - rect.left() - 18.0).max(10.0);
    let font = theme.font("ui-label");
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
    if close_hit.clicked() || response.middle_clicked() {
        Some(TabAsk::Close(tab.key.clone()))
    } else if response.clicked() {
        Some(TabAsk::Show(tab.key.clone()))
    } else {
        None
    }
}

fn plain(texts: &Texts, key: &'static str) -> String {
    texts.t(&gafira_doc::msg::Message {
        key: gafira_doc::msg::MessageKey(key),
        args: Vec::new(),
    })
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
