//! Inicio (03 §7.2): «¿Qué quieres crear hoy?», the formats, the main action
//! and the recent projects.

use egui::{Align, Layout, RichText, Sense, Stroke, Vec2};

use crate::format::StartFormat;
use crate::shell::Go;
use crate::style::{DISPLAY, Theme};
use crate::text::Texts;
use crate::widgets;

/// The state of Inicio: the format the person picked.
pub(crate) struct Home {
    picked: StartFormat,
}

impl Default for Home {
    fn default() -> Self {
        Self {
            picked: StartFormat::Social,
        }
    }
}

impl Home {
    pub(crate) fn show(&mut self, ctx: &egui::Context, theme: &Theme, texts: &Texts) -> Go {
        let mut go = Go::Stay;
        widgets::topbar(ctx, theme, "home.topbar", |ui| {
            ui.label(
                RichText::new(texts.t(&gafira_doc::msg!("app.name")))
                    .font(egui::FontId::new(
                        20.0,
                        egui::FontFamily::Name(DISPLAY.into()),
                    ))
                    .color(theme.c("topbar.fg")),
            );
            ui.add_space(24.0);
            widgets::on_topbar(ui, theme, texts.t(&gafira_doc::msg!("nav.home")), true);
            widgets::on_topbar(ui, theme, texts.t(&gafira_doc::msg!("nav.projects")), false);
            widgets::on_topbar(
                ui,
                theme,
                texts.t(&gafira_doc::msg!("nav.templates")),
                false,
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                widgets::on_topbar(ui, theme, texts.t(&gafira_doc::msg!("nav.settings")), false);
                ui.add_space(16.0);
                ui.label(
                    RichText::new(texts.t(&gafira_doc::msg!("nav.search")))
                        .font(theme.font("caption"))
                        .color(theme.c("topbar.fg.muted")),
                );
            });
        });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(theme.c("bg"))
                    .inner_margin(egui::Margin::same(48)),
            )
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(texts.t(&gafira_doc::msg!("brand.slogan")))
                        .font(theme.font("h1"))
                        .color(theme.c("text")),
                );
                ui.label(
                    RichText::new(texts.t(&gafira_doc::msg!("home.question")))
                        .font(theme.font("body-lg"))
                        .color(theme.c("text.secondary")),
                );
                ui.add_space(24.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(12.0);
                    for format in StartFormat::ALL {
                        if self.card(ui, theme, texts, format) {
                            self.picked = format;
                        }
                    }
                });
                ui.add_space(24.0);
                ui.horizontal(|ui| {
                    if widgets::primary(ui, theme, texts.t(&gafira_doc::msg!("home.start")))
                        .clicked()
                    {
                        go = Go::Editor(self.picked);
                    }
                    ui.add_space(12.0);
                    widgets::secondary(ui, theme, texts.t(&gafira_doc::msg!("home.templates")));
                    ui.add_space(12.0);
                    if widgets::secondary(ui, theme, texts.t(&gafira_doc::msg!("home.open")))
                        .clicked()
                    {
                        go = Go::File(crate::files::FileAction::Open);
                    }
                });
                ui.add_space(40.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(texts.t(&gafira_doc::msg!("home.recent")))
                            .font(theme.font("h3"))
                            .color(theme.c("text")),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(texts.t(&gafira_doc::msg!("home.see_all")))
                                .font(theme.font("body-sm"))
                                .color(theme.c("link")),
                        );
                    });
                });
                ui.add_space(12.0);
                ui.label(
                    RichText::new(texts.t(&gafira_doc::msg!("home.empty")))
                        .font(theme.font("body"))
                        .color(theme.c("text.muted")),
                );
            });
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::N)) {
            go = Go::Editor(self.picked);
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::O)) {
            go = Go::File(crate::files::FileAction::Open);
        }
        go
    }

    /// One format card; `true` when clicked.
    fn card(&self, ui: &mut egui::Ui, theme: &Theme, texts: &Texts, format: StartFormat) -> bool {
        let picked = self.picked == format;
        let stroke = if picked {
            Stroke::new(2.0, theme.c("accent"))
        } else {
            Stroke::new(1.0, theme.c("border"))
        };
        let frame = egui::Frame::NONE
            .fill(theme.c("surface"))
            .stroke(stroke)
            .corner_radius(theme.radius_md())
            .inner_margin(egui::Margin::same(16));
        let response = frame
            .show(ui, |ui| {
                ui.set_min_size(Vec2::new(132.0, 84.0));
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(texts.t(&format.name()))
                            .font(theme.font("ui-label"))
                            .color(theme.c("text")),
                    );
                    ui.label(
                        RichText::new(texts.t(&format.size_label()))
                            .font(theme.font("caption"))
                            .color(theme.c("text.muted")),
                    );
                });
            })
            .response
            .interact(Sense::click());
        response.clicked()
    }
}
