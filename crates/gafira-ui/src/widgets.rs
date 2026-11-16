//! The buttons and bars of the component catalog (03 §8) the shell uses.

use egui::{Button, Response, RichText, Ui, Vec2};

use crate::style::{Theme, px};

/// The coral button: one per view (DIS-15).
pub(crate) fn primary(ui: &mut Ui, theme: &Theme, text: String) -> Response {
    let height = px(theme.tokens.size.control.lg);
    let button = Button::new(
        RichText::new(text)
            .font(theme.font("ui-label"))
            .color(theme.c("primary.fg")),
    )
    .fill(theme.c("primary.bg"))
    .corner_radius(theme.radius_md())
    .min_size(Vec2::new(0.0, height));
    ui.add(button)
}

/// A secondary action.
pub(crate) fn secondary(ui: &mut Ui, theme: &Theme, text: String) -> Response {
    let height = px(theme.tokens.size.control.lg);
    let button = Button::new(
        RichText::new(text)
            .font(theme.font("ui-label"))
            .color(theme.c("secondary.fg")),
    )
    .fill(theme.c("secondary.bg"))
    .corner_radius(theme.radius_md())
    .min_size(Vec2::new(0.0, height));
    ui.add(button)
}

/// A flat button on the graphite top bar.
pub(crate) fn on_topbar(ui: &mut Ui, theme: &Theme, text: String, active: bool) -> Response {
    let color = if active {
        theme.c("topbar.fg")
    } else {
        theme.c("topbar.fg.muted")
    };
    let button = Button::new(
        RichText::new(text)
            .font(theme.font("ui-label"))
            .color(color),
    )
    .frame(false)
    .min_size(Vec2::new(0.0, px(theme.tokens.size.control.md)));
    ui.add(button)
}

/// The graphite top bar of every screen (03 §7.1: 48 px).
pub(crate) fn topbar(ctx: &egui::Context, theme: &Theme, id: &str, add: impl FnOnce(&mut Ui)) {
    egui::TopBottomPanel::top(id.to_owned())
        .exact_height(px(theme.tokens.size.topbar))
        .frame(
            egui::Frame::NONE
                .fill(theme.c("topbar.bg"))
                .inner_margin(egui::Margin::symmetric(12, 0)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(add);
        });
}
