//! Inicio (03 §7.2): «¿Qué quieres crear hoy?», the formats, the main action
//! and the recent projects, with the thumbnail of each one's first page
//! (spec 033).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use egui::{Align, Layout, RichText, Sense, Stroke, Vec2};

use crate::format::StartFormat;
use crate::recent::{self, RecentProject, RecentThumbnail, ThumbCache};
use crate::shell::Go;
use crate::style::{DISPLAY, Theme};
use crate::text::Texts;
use crate::widgets;

/// The state of Inicio: the format the person picked and the recent
/// projects with their thumbnails, read as Inicio appears.
pub(crate) struct Home {
    picked: StartFormat,
    recent: Vec<RecentProject>,
    thumbs: BTreeMap<PathBuf, RecentThumbnail>,
}

/// The textures of the thumbnails Inicio shows; the shell keeps them from
/// one visit of Inicio to the next (S33-07).
pub(crate) type Thumbs = ThumbCache<egui::TextureHandle>;

impl Home {
    pub(crate) fn new(
        recent: Vec<RecentProject>,
        thumbs: BTreeMap<PathBuf, RecentThumbnail>,
    ) -> Self {
        Self {
            picked: StartFormat::Social,
            recent,
            thumbs,
        }
    }

    /// Inicio at `now` (seconds since the Unix epoch, for the relative
    /// times of «Recientes»), with the thumbnails' textures in `textures`.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        now: i64,
        textures: &mut Thumbs,
    ) -> Go {
        textures.keep_only(self.thumbs.keys().map(PathBuf::as_path));
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
                if widgets::on_topbar(ui, theme, texts.t(&gafira_doc::msg!("nav.settings")), false)
                    .clicked()
                {
                    go = Go::Settings;
                }
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
                let shown = recent::shown(&self.recent);
                if shown.is_empty() {
                    ui.label(
                        RichText::new(texts.t(&gafira_doc::msg!("home.empty")))
                            .font(theme.font("body"))
                            .color(theme.c("text.muted")),
                    );
                } else {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::splat(12.0);
                        for project in shown {
                            let picture = self.thumbs.get(&project.path).map(|thumb| {
                                let texture = textures.get(&project.path, thumb, |thumb| {
                                    upload(ctx, &project.path, thumb)
                                });
                                (texture.clone(), (thumb.width, thumb.height))
                            });
                            if recent_card(ui, theme, texts, project, now, picture.as_ref()) {
                                go = Go::OpenRecent(project.path.clone());
                            }
                        }
                    });
                }
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

/// The texture of the thumbnail of the project at `path`.
fn upload(ctx: &egui::Context, path: &Path, thumb: &RecentThumbnail) -> egui::TextureHandle {
    let size = [thumb.width as usize, thumb.height as usize];
    ctx.load_texture(
        format!("recent:{}", path.display()),
        egui::ColorImage::from_rgba_unmultiplied(size, &thumb.rgba),
        egui::TextureOptions::LINEAR,
    )
}

/// One recent project: its thumbnail, whole and centred on the
/// placeholder, or the placeholder alone when it has none (spec 033); its
/// name and how long ago it was opened; `true` when clicked.
fn recent_card(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    project: &RecentProject,
    now: i64,
    picture: Option<&(egui::TextureHandle, (u32, u32))>,
) -> bool {
    let frame = egui::Frame::NONE
        .fill(theme.c("bg"))
        .stroke(Stroke::new(1.0, theme.c("border")))
        .corner_radius(theme.radius_md())
        .inner_margin(egui::Margin::same(8));
    frame
        .show(ui, |ui| {
            ui.set_width(176.0);
            ui.vertical(|ui| {
                let (thumb, _) = ui.allocate_exact_size(Vec2::new(176.0, 110.0), Sense::hover());
                ui.painter()
                    .rect_filled(thumb, theme.radius_md(), theme.c("surface"));
                if let Some((texture, (width, height))) = picture {
                    let (w, h) = recent::fit(*width, *height, (thumb.width(), thumb.height()));
                    ui.painter().image(
                        texture.id(),
                        egui::Rect::from_center_size(thumb.center(), Vec2::new(w, h)),
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
                ui.add_space(4.0);
                ui.label(
                    RichText::new(&project.name)
                        .font(theme.font("ui-label"))
                        .color(theme.c("text")),
                );
                ui.label(
                    RichText::new(texts.relative(recent::age(project.opened_at, now)))
                        .font(theme.font("caption"))
                        .color(theme.c("text.muted")),
                );
            });
        })
        .response
        .interact(Sense::click())
        .clicked()
}
