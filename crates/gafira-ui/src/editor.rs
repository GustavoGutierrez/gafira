//! The regions of the Design editor (03 §7.1, §7.3): top bar, tab rail, side
//! panel, canvas with the page, inspector and bottom bar.

use egui::{Align, Align2, Color32, Layout, Pos2, Rect, RichText, Stroke, Vec2};
use gafira_theme::Shadow;

use crate::format::{StartFormat, fit_zoom};
use crate::shell::Go;
use crate::style::{Theme, color32, px};
use crate::text::Texts;
use crate::widgets;

/// The tabs of the rail, in the order of 03 §2.3.
const TABS: [&str; 6] = [
    "tab.templates",
    "tab.pieces",
    "tab.text",
    "tab.identity",
    "tab.library",
    "tab.drawing",
];

/// The margin around the page on the canvas, in logical px.
const CANVAS_MARGIN: f32 = 48.0;

/// The editor's view state; the document arrives with the engine.
pub(crate) struct Editor {
    format: StartFormat,
    tab: usize,
    /// A factor over the fitting zoom, changed with Ctrl + wheel.
    zoom_factor: f32,
}

impl Editor {
    pub(crate) fn new(format: StartFormat) -> Self {
        Self {
            format,
            tab: 0,
            zoom_factor: 1.0,
        }
    }

    pub(crate) fn show(&mut self, ctx: &egui::Context, theme: &Theme, texts: &Texts) -> Go {
        let mut go = Go::Stay;
        widgets::topbar(ctx, theme, "editor.topbar", |ui| {
            if widgets::on_topbar(ui, theme, texts.t(&gafira_doc::msg!("nav.home")), true).clicked()
            {
                go = Go::Home;
            }
            ui.add_space(12.0);
            for key in [
                gafira_doc::msg!("menu.file"),
                gafira_doc::msg!("menu.edit"),
                gafira_doc::msg!("menu.view"),
                gafira_doc::msg!("menu.size"),
            ] {
                widgets::on_topbar(ui, theme, texts.t(&key), false);
            }
            ui.add_space(24.0);
            ui.label(
                RichText::new(texts.t(&gafira_doc::msg!("editor.untitled")))
                    .font(theme.font("ui-label"))
                    .color(theme.c("topbar.fg")),
            );
            ui.add_space(16.0);
            ui.label(
                RichText::new(texts.t(&gafira_doc::msg!("editor.saved")))
                    .font(theme.font("caption"))
                    .color(theme.c("topbar.fg.muted")),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                widgets::primary(ui, theme, texts.t(&gafira_doc::msg!("editor.export")));
                ui.add_space(8.0);
                widgets::on_topbar(
                    ui,
                    theme,
                    texts.t(&gafira_doc::msg!("editor.versions")),
                    false,
                );
            });
        });
        egui::TopBottomPanel::bottom("editor.bottombar")
            .exact_height(px(theme.tokens.size.bottombar))
            .frame(
                egui::Frame::NONE
                    .fill(theme.c("bg"))
                    .stroke(Stroke::new(1.0, theme.c("border")))
                    .inner_margin(egui::Margin::symmetric(12, 0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(texts.t(&gafira_doc::msg!("bottom.notes")));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(texts.t(&gafira_doc::msg!("bottom.pages")));
                        ui.add_space(16.0);
                        ui.label(texts.t(&gafira_doc::msg!(
                            "editor.page_of",
                            current = Count(1),
                            total = Count(1)
                        )));
                        ui.add_space(16.0);
                        let zoom = self.zoom_shown(ctx, theme);
                        ui.label(texts.t(&gafira_doc::msg!(
                            "format.percent",
                            value = Text(format!("{:.0}", zoom * 100.0))
                        )));
                    });
                });
            });
        self.rail(ctx, theme, texts);
        self.side_panel(ctx, theme, texts);
        self.inspector(ctx, theme, texts);
        self.canvas(ctx, theme, texts);
        go
    }

    fn rail(&mut self, ctx: &egui::Context, theme: &Theme, texts: &Texts) {
        egui::SidePanel::left("editor.rail")
            .exact_width(px(theme.tokens.size.rail))
            .resizable(false)
            .frame(egui::Frame::NONE.fill(theme.c("topbar.bg")))
            .show(ctx, |ui| {
                ui.add_space(8.0);
                for (index, key) in TABS.iter().enumerate() {
                    let active = index == self.tab;
                    let (rect, response) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 56.0),
                        egui::Sense::click(),
                    );
                    if response.hovered() {
                        ui.painter().rect_filled(rect, 0.0, theme.c("topbar.hover"));
                    }
                    if active {
                        let bar = Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height()));
                        ui.painter().rect_filled(bar, 0.0, theme.c("primary.bg"));
                    }
                    let color = if active {
                        theme.c("topbar.fg")
                    } else {
                        theme.c("topbar.fg.muted")
                    };
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        texts.t(&gafira_doc::msg::Message {
                            key: gafira_doc::msg::MessageKey(key),
                            args: Vec::new(),
                        }),
                        theme.font("caption"),
                        color,
                    );
                    if response.clicked() {
                        self.tab = index;
                    }
                }
            });
    }

    fn side_panel(&self, ctx: &egui::Context, theme: &Theme, texts: &Texts) {
        egui::SidePanel::left("editor.side")
            .default_width(px(theme.tokens.size.panel_left))
            .width_range(280.0..=480.0)
            .frame(
                egui::Frame::NONE
                    .fill(theme.c("bg"))
                    .stroke(Stroke::new(1.0, theme.c("border")))
                    .inner_margin(egui::Margin::same(16)),
            )
            .show(ctx, |ui| {
                let mut query = String::new();
                ui.add(
                    egui::TextEdit::singleline(&mut query)
                        .hint_text(texts.t(&gafira_doc::msg!("editor.search_templates")))
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(16.0);
                let card = Vec2::new((ui.available_width() - 12.0) / 2.0, 120.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(12.0);
                    for _ in 0..6 {
                        let (rect, _) = ui.allocate_exact_size(card, egui::Sense::hover());
                        ui.painter().rect(
                            rect,
                            theme.radius_md(),
                            theme.c("surface"),
                            Stroke::new(1.0, theme.c("border")),
                            egui::StrokeKind::Inside,
                        );
                    }
                });
            });
    }

    fn inspector(&self, ctx: &egui::Context, theme: &Theme, texts: &Texts) {
        egui::SidePanel::right("editor.inspector")
            .exact_width(px(theme.tokens.size.panel_right))
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(theme.c("surface"))
                    .stroke(Stroke::new(1.0, theme.c("border")))
                    .inner_margin(egui::Margin::same(16)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (index, key) in [
                        gafira_doc::msg!("inspector.position"),
                        gafira_doc::msg!("inspector.layers"),
                        gafira_doc::msg!("inspector.image"),
                    ]
                    .iter()
                    .enumerate()
                    {
                        let color = if index == 0 {
                            theme.c("text")
                        } else {
                            theme.c("text.muted")
                        };
                        ui.label(
                            RichText::new(texts.t(key))
                                .font(theme.font("ui-label"))
                                .color(color),
                        );
                        ui.add_space(8.0);
                    }
                });
                ui.add_space(24.0);
                ui.label(
                    RichText::new(texts.t(&gafira_doc::msg!("inspector.empty")))
                        .font(theme.font("body-sm"))
                        .color(theme.c("text.muted")),
                );
            });
    }

    /// The zoom on display: the fitting zoom of the last canvas size times
    /// the factor.
    fn zoom_shown(&self, ctx: &egui::Context, theme: &Theme) -> f32 {
        let area = ctx.content_rect().size()
            - Vec2::new(
                px(theme.tokens.size.rail
                    + theme.tokens.size.panel_left
                    + theme.tokens.size.panel_right),
                px(theme.tokens.size.topbar + theme.tokens.size.bottombar),
            );
        (fit_zoom(self.format.page_px(), (area.x, area.y), CANVAS_MARGIN) * self.zoom_factor)
            .clamp(0.05, 32.0)
    }

    fn canvas(&mut self, ctx: &egui::Context, theme: &Theme, texts: &Texts) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme.c("workspace")))
            .show(ctx, |ui| {
                let area = ui.max_rect();
                let (scroll, command) =
                    ui.input(|i| (i.smooth_scroll_delta.y, i.modifiers.command));
                if command && scroll != 0.0 && ui.rect_contains_pointer(area) {
                    self.zoom_factor = (self.zoom_factor * (1.0 + scroll / 400.0)).clamp(0.1, 32.0);
                }
                let (w, h) = self.format.page_px();
                let zoom = (fit_zoom((w, h), (area.width(), area.height()), CANVAS_MARGIN)
                    * self.zoom_factor)
                    .clamp(0.05, 32.0);
                let page = Rect::from_center_size(area.center(), Vec2::new(w * zoom, h * zoom));
                let painter = ui.painter_at(area);
                if let Some(Shadow::Drop {
                    x,
                    y,
                    blur,
                    color,
                    alpha,
                }) = theme.tokens.shadow.get(&1).copied()
                {
                    let shadow = egui::epaint::Shadow {
                        offset: [to_i8(x), to_i8(y)],
                        blur: to_u8(blur),
                        spread: 0,
                        color: color32(color).gamma_multiply(alpha),
                    };
                    painter.add(shadow.as_shape(page, 0.0));
                }
                // The page colour belongs to the document (white by default,
                // 03 §7.1), not to the theme.
                painter.rect_filled(page, 0.0, Color32::WHITE);
                painter.text(
                    Pos2::new(page.min.x, page.min.y - 8.0),
                    Align2::LEFT_BOTTOM,
                    texts.t(&gafira_doc::msg!("editor.page", number = Count(1))),
                    theme.font("caption"),
                    theme.c("text.secondary"),
                );
            });
    }
}

/// A shadow offset as egui's `i8`; the tokens hold small whole numbers.
fn to_i8(value: f32) -> i8 {
    let mut out: i8 = 0;
    while f32::from(out) < value.min(127.0) {
        out += 1;
    }
    out
}

/// A shadow blur as egui's `u8`.
fn to_u8(value: f32) -> u8 {
    let mut out: u8 = 0;
    while f32::from(out) < value.min(255.0) {
        out += 1;
    }
    out
}
