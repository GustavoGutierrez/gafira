//! Ajustes (03 §7.8, §16.1; spec 034, spec 092): sections with their icons
//! on the left, content as cards on the right. General holds the language, Apariencia the interface mode,
//! Privacidad its promise and Acerca de the version and the bundled fonts.
//! Every change applies at once, with no «Aplicar» button (DIS-75).

use egui::{Align, Color32, Layout, Rect, RichText, Sense, Stroke, Vec2};
use gafira_ports::i18n::{LanguagePreference, Locale, LocaleSource, ResolvedLocale};
use gafira_ports::settings::{Appearance, Settings};

use crate::icons::{self, Icon, Weight};
use crate::style::{DISPLAY, Theme};
use crate::text::Texts;
use crate::widgets;

/// The sections this delivery has, in the order of 03 §7.8.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Section {
    General,
    Appearance,
    Privacy,
    /// The installed asset packages (spec 097).
    Packages,
    About,
}

impl Section {
    const ALL: [Self; 5] = [
        Self::General,
        Self::Appearance,
        Self::Privacy,
        Self::Packages,
        Self::About,
    ];

    fn icon(self) -> Icon {
        match self {
            Self::General => Icon::Language,
            Self::Appearance => Icon::Identity,
            Self::Privacy => Icon::Privacy,
            Self::Packages => Icon::Package,
            Self::About => Icon::About,
        }
    }

    fn title(self) -> gafira_doc::msg::Message {
        match self {
            Self::General => gafira_doc::msg!("settings.general"),
            Self::Appearance => gafira_doc::msg!("settings.appearance.title"),
            Self::Privacy => gafira_doc::msg!("settings.privacy"),
            Self::Packages => gafira_doc::msg!("packs.settings_title"),
            Self::About => gafira_doc::msg!("settings.about"),
        }
    }
}

/// What the person changed this frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Change {
    None,
    Back,
    Language(LanguagePreference),
    Appearance(Appearance),
    /// Choose a `.gafpack` to install (spec 097).
    InstallPack,
    /// Remove the installed package with this id (spec 097).
    Uninstall(String),
}

/// The open Ajustes screen.
pub(crate) struct SettingsView {
    pub(crate) section: Section,
    /// The package whose «Desinstalar» was pressed once, waiting for the
    /// confirmation.
    pub(crate) confirm: Option<String>,
}

impl Default for SettingsView {
    fn default() -> Self {
        Self {
            section: Section::General,
            confirm: None,
        }
    }
}

/// The name of a language, always in its own language (03 §16.1).
fn language_name(locale: Locale) -> gafira_doc::msg::Message {
    match locale {
        Locale::Es => gafira_doc::msg!("settings.language.es"),
        Locale::En => gafira_doc::msg!("settings.language.en"),
    }
}

/// «Sistema · ahora: Español (es-MX)», or «… English (en)» when the system
/// gave no language (03 §16.1).
pub(crate) fn system_label(texts: &Texts, system: &ResolvedLocale) -> String {
    let tag = match &system.source {
        LocaleSource::System { tag } => tag.clone(),
        LocaleSource::Manual | LocaleSource::Fallback => match system.locale {
            Locale::Es => "es".to_owned(),
            Locale::En => "en".to_owned(),
        },
    };
    texts.t(&gafira_doc::msg!(
        "settings.language.system_now",
        language = Text(texts.t(&language_name(system.locale))),
        tag = Text(tag)
    ))
}

/// Draw Ajustes over the whole window; `system` is what «Sistema» resolves
/// to now (spec 092: a top bar with the logo and «Volver», the sections
/// with their icons, and each option as a card).
pub(crate) fn show(
    ctx: &egui::Context,
    theme: &Theme,
    texts: &Texts,
    view: &mut SettingsView,
    current: Settings,
    system: &ResolvedLocale,
    (version, packs): (&str, &[crate::InstalledPack]),
) -> Change {
    let mut change = Change::None;
    widgets::topbar(ctx, theme, "settings.topbar", |ui| {
        ui.add_space(4.0);
        widgets::topbar_logo(ui, 26.0);
        ui.add_space(28.0);
        let back = texts.t(&gafira_doc::msg!("settings.back"));
        if widgets::nav_item(ui, theme, Icon::Back, &back, false).clicked() {
            change = Change::Back;
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let title = texts.t(&gafira_doc::msg!("settings.title"));
            widgets::nav_item(ui, theme, Icon::Settings, &title, true);
        });
    });
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        change = Change::Back;
    }
    egui::SidePanel::left("settings.sections")
        .exact_width(260.0)
        .resizable(false)
        .show_separator_line(false)
        .frame(
            egui::Frame::NONE
                .fill(theme.c("bg"))
                .inner_margin(egui::Margin {
                    left: 20,
                    right: 12,
                    top: 28,
                    bottom: 20,
                }),
        )
        .show(ctx, |ui| {
            ui.label(
                RichText::new(texts.t(&gafira_doc::msg!("settings.title")))
                    .font(egui::FontId::new(
                        28.0,
                        egui::FontFamily::Name(DISPLAY.into()),
                    ))
                    .color(theme.c("text")),
            );
            ui.add_space(20.0);
            ui.spacing_mut().item_spacing.y = 2.0;
            for section in Section::ALL {
                let on = view.section == section;
                let row = crate::create::side_row(
                    ui,
                    theme,
                    on,
                    &texts.t(&section.title()),
                    |ui, rect| {
                        let weight = if on { Weight::Fill } else { Weight::Regular };
                        icons::paint(ui.painter(), rect, section.icon(), weight, theme.c("text"));
                    },
                );
                if row.clicked() {
                    view.section = section;
                }
            }
        });
    egui::CentralPanel::default()
        .frame(
            egui::Frame::NONE
                .fill(theme.c("bg"))
                .inner_margin(egui::Margin {
                    left: 16,
                    right: 32,
                    top: 28,
                    bottom: 0,
                }),
        )
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_max_width(760.0);
                    let next = match view.section {
                        Section::General => general(ui, theme, texts, current, system),
                        Section::Appearance => appearance(ui, theme, texts, current),
                        Section::Privacy => {
                            privacy(ui, theme, texts);
                            Change::None
                        }
                        Section::Packages => packages(ui, theme, texts, packs, &mut view.confirm),
                        Section::About => {
                            about(ui, theme, texts, version);
                            Change::None
                        }
                    };
                    if next != Change::None {
                        change = next;
                    }
                    ui.add_space(32.0);
                });
        });
    change
}

/// The title of a section and a line that says what it holds.
fn section_header(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    title: gafira_doc::msg::Message,
    hint: gafira_doc::msg::Message,
) {
    ui.label(
        RichText::new(texts.t(&title))
            .font(theme.font("h3"))
            .color(theme.c("text")),
    );
    ui.add_space(4.0);
    ui.label(
        RichText::new(texts.t(&hint))
            .font(theme.font("body-sm"))
            .color(theme.c("text.secondary")),
    );
    ui.add_space(18.0);
}

/// A card: the surface with its border, `add` inside.
fn card<R>(ui: &mut egui::Ui, theme: &Theme, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::NONE
        .fill(theme.c("surface"))
        .stroke(Stroke::new(1.0, theme.c("border")))
        .corner_radius(12.0)
        .inner_margin(egui::Margin::same(20))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// An option as a card: its name, a line under it, a radio mark; lit with
/// the brand ring when chosen. `true` when clicked.
fn choice(
    ui: &mut egui::Ui,
    theme: &Theme,
    size: Vec2,
    on: bool,
    title: &str,
    detail: &str,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let hover = ui.ctx().animate_bool(response.id, response.hovered());
    let fill = if on {
        theme.c("selection.bg")
    } else {
        theme
            .c("surface")
            .lerp_to_gamma(theme.c("secondary.bg"), hover)
    };
    ui.painter().rect_filled(rect, 12.0, fill);
    let stroke = if on {
        Stroke::new(2.0, theme.c("accent"))
    } else {
        Stroke::new(1.0, theme.c("border"))
    };
    ui.painter()
        .rect_stroke(rect, 12.0, stroke, egui::StrokeKind::Inside);
    radio_mark(
        ui,
        theme,
        egui::pos2(rect.right() - 22.0, rect.top() + 22.0),
        on,
    );
    let width = rect.width() - 56.0;
    let title = ui.painter().layout(
        title.to_owned(),
        theme.font("ui-label"),
        theme.c("text"),
        width,
    );
    let title_h = title.size().y;
    ui.painter().galley(
        rect.left_top() + Vec2::new(16.0, 14.0),
        title,
        theme.c("text"),
    );
    let detail = ui.painter().layout(
        detail.to_owned(),
        theme.font("caption"),
        theme.c("text.secondary"),
        rect.width() - 32.0,
    );
    ui.painter().galley(
        rect.left_top() + Vec2::new(16.0, 20.0 + title_h),
        detail,
        theme.c("text.secondary"),
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.clicked()
}

/// A round radio mark at `center`, filled when on.
fn radio_mark(ui: &egui::Ui, theme: &Theme, center: egui::Pos2, on: bool) {
    if on {
        ui.painter().circle_filled(center, 9.0, theme.c("accent"));
        ui.painter().circle_filled(center, 3.5, theme.c("surface"));
    } else {
        ui.painter()
            .circle_stroke(center, 8.0, Stroke::new(1.5, theme.c("border.strong")));
    }
}

/// General: the language, as three cards.
fn general(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    current: Settings,
    system: &ResolvedLocale,
) -> Change {
    let mut change = Change::None;
    section_header(
        ui,
        theme,
        texts,
        gafira_doc::msg!("settings.language.title"),
        gafira_doc::msg!("settings.language.hint"),
    );
    // «Sistema · ahora: Español (es-CO)» without its first word, which
    // the card already shows.
    let now = system_label(texts, system);
    let now = now
        .split_once(" · ")
        .map_or(now.clone(), |(_, rest)| rest.to_owned());
    let options = [
        (
            LanguagePreference::System,
            texts.t(&gafira_doc::msg!("settings.language.system")),
            now,
        ),
        (
            LanguagePreference::Es,
            texts.t(&gafira_doc::msg!("settings.language.es")),
            texts.t(&gafira_doc::msg!("settings.language.es_detail")),
        ),
        (
            LanguagePreference::En,
            texts.t(&gafira_doc::msg!("settings.language.en")),
            texts.t(&gafira_doc::msg!("settings.language.en_detail")),
        ),
    ];
    let gap = 12.0;
    let width = ((ui.available_width() - 2.0 * gap) / 3.0).clamp(160.0, 240.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for (preference, title, detail) in options {
            let on = current.language == preference;
            if choice(ui, theme, Vec2::new(width, 92.0), on, &title, &detail) && !on {
                change = Change::Language(preference);
            }
        }
    });
    ui.add_space(14.0);
    ui.label(
        RichText::new(texts.t(&gafira_doc::msg!("settings.language.help")))
            .font(theme.font("body-sm"))
            .color(theme.c("text.secondary")),
    );
    change
}

/// Apariencia: Claro, Oscuro and Sistema, each with a small picture of
/// the interface in its colours.
fn appearance(ui: &mut egui::Ui, theme: &Theme, texts: &Texts, current: Settings) -> Change {
    let mut change = Change::None;
    section_header(
        ui,
        theme,
        texts,
        gafira_doc::msg!("settings.appearance.title"),
        gafira_doc::msg!("settings.appearance.hint"),
    );
    let options = [
        (
            Appearance::Light,
            Icon::Light,
            gafira_doc::msg!("settings.appearance.light"),
        ),
        (
            Appearance::Dark,
            Icon::Dark,
            gafira_doc::msg!("settings.appearance.dark"),
        ),
        (
            Appearance::System,
            Icon::System,
            gafira_doc::msg!("settings.appearance.system"),
        ),
    ];
    let gap = 16.0;
    let width = ((ui.available_width() - 2.0 * gap) / 3.0).clamp(170.0, 240.0);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for (appearance, icon, key) in options {
            let on = current.appearance == appearance;
            if theme_card(ui, theme, (width, appearance, icon), on, &texts.t(&key)) && !on {
                change = Change::Appearance(appearance);
            }
        }
    });
    change
}

/// One theme: its picture, its icon and name and a radio mark.
fn theme_card(
    ui: &mut egui::Ui,
    theme: &Theme,
    (width, appearance, icon): (f32, Appearance, Icon),
    on: bool,
    name: &str,
) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(width, width * 0.62 + 52.0), Sense::click());
    let hover = ui.ctx().animate_bool(response.id, response.hovered());
    let fill = if on {
        theme.c("selection.bg")
    } else {
        theme
            .c("surface")
            .lerp_to_gamma(theme.c("secondary.bg"), hover)
    };
    ui.painter().rect_filled(rect, 12.0, fill);
    let picture = Rect::from_min_size(
        rect.min + Vec2::splat(10.0),
        Vec2::new(width - 20.0, width * 0.62 - 10.0),
    );
    match appearance {
        Appearance::Light => preview(ui, theme, picture, false, picture),
        Appearance::Dark => preview(ui, theme, picture, true, picture),
        Appearance::System => {
            let half = picture.center().x;
            preview(
                ui,
                theme,
                picture,
                false,
                Rect::from_min_max(picture.min, egui::pos2(half, picture.max.y)),
            );
            preview(
                ui,
                theme,
                picture,
                true,
                Rect::from_min_max(egui::pos2(half, picture.min.y), picture.max),
            );
        }
    }
    let stroke = if on {
        Stroke::new(2.0, theme.c("accent"))
    } else {
        Stroke::new(1.0, theme.c("border"))
    };
    ui.painter()
        .rect_stroke(rect, 12.0, stroke, egui::StrokeKind::Inside);
    let row = rect.bottom() - 24.0;
    icons::paint(
        ui.painter(),
        Rect::from_center_size(egui::pos2(rect.left() + 22.0, row), Vec2::splat(18.0)),
        icon,
        if on { Weight::Fill } else { Weight::Regular },
        theme.c("text"),
    );
    ui.painter().text(
        egui::pos2(rect.left() + 40.0, row),
        egui::Align2::LEFT_CENTER,
        name,
        theme.font("ui-label"),
        theme.c("text"),
    );
    radio_mark(ui, theme, egui::pos2(rect.right() - 22.0, row), on);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.clicked()
}

/// A small picture of the editor in the light or dark colours, drawn in
/// `full` and shown only inside `clip`: the top bar, the rail, the side
/// panel, the canvas with its page, and the coral «Exportar».
fn preview(ui: &egui::Ui, theme: &Theme, full: Rect, dark: bool, clip: Rect) {
    let mode = theme.tokens.mode(dark);
    let c = |token: &str| mode.get(token).map_or(Color32::GRAY, crate::style::color32);
    let painter = ui.painter().with_clip_rect(clip);
    let r = |x0: f32, y0: f32, x1: f32, y1: f32| {
        Rect::from_min_max(
            egui::pos2(
                full.left() + full.width() * x0,
                full.top() + full.height() * y0,
            ),
            egui::pos2(
                full.left() + full.width() * x1,
                full.top() + full.height() * y1,
            ),
        )
    };
    painter.rect_filled(full, 8.0, c("workspace"));
    painter.rect_filled(
        r(0.0, 0.0, 1.0, 0.14),
        egui::CornerRadius {
            nw: 8,
            ne: 8,
            sw: 0,
            se: 0,
        },
        c("topbar.bg"),
    );
    painter.rect_filled(r(0.80, 0.03, 0.96, 0.11), 3.0, c("primary.bg"));
    painter.rect_filled(
        r(0.0, 0.14, 0.09, 1.0),
        egui::CornerRadius {
            nw: 0,
            ne: 0,
            sw: 8,
            se: 0,
        },
        c("topbar.bg"),
    );
    painter.rect_filled(r(0.09, 0.14, 0.34, 1.0), 0.0, c("bg"));
    for k in 0..3 {
        let y = 0.22 + k as f32 * 0.18;
        painter.rect_filled(r(0.12, y, 0.31, y + 0.13), 3.0, c("surface.sunken"));
    }
    painter.rect_filled(r(0.45, 0.24, 0.88, 0.86), 2.0, Color32::WHITE);
    painter.rect_filled(r(0.52, 0.34, 0.80, 0.42), 2.0, c("text"));
    painter.rect_filled(r(0.52, 0.48, 0.70, 0.52), 2.0, c("accent"));
    painter.circle_filled(
        r(0.70, 0.62, 0.80, 0.76).center(),
        full.height() * 0.06,
        c("primary.bg"),
    );
}

/// Privacidad: what stays on the computer and when Gafira goes online.
fn privacy(ui: &mut egui::Ui, theme: &Theme, texts: &Texts) {
    section_header(
        ui,
        theme,
        texts,
        gafira_doc::msg!("settings.privacy"),
        gafira_doc::msg!("settings.privacy_hint"),
    );
    card(ui, theme, |ui| {
        for (icon, key) in [
            (Icon::Privacy, gafira_doc::msg!("settings.privacy_text")),
            (Icon::Language, gafira_doc::msg!("settings.privacy_online")),
        ] {
            ui.horizontal_top(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::hover());
                icons::paint(ui.painter(), rect, icon, Weight::Regular, theme.c("accent"));
                ui.add_space(8.0);
                ui.add(
                    egui::Label::new(
                        RichText::new(texts.t(&key))
                            .font(theme.font("body"))
                            .color(theme.c("text")),
                    )
                    .wrap(),
                );
            });
            ui.add_space(12.0);
        }
    });
}

/// Acerca de: the logo, the version and what Gafira carries.
/// «Paquetes» (spec 097, §8.3): «Instalar paquete…» and each installed
/// package with its thumbnail, name and description in the language of
/// the interface, type, version, declared author, size and what it holds;
/// «Desinstalar» asks once more before it removes it.
fn packages(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    packs: &[crate::InstalledPack],
    confirm: &mut Option<String>,
) -> Change {
    let mut change = Change::None;
    let t = |key: &'static str| {
        texts.t(&gafira_doc::msg::Message {
            key: gafira_doc::msg::MessageKey(key),
            args: Vec::new(),
        })
    };
    section_header(
        ui,
        theme,
        texts,
        gafira_doc::msg!("packs.settings_title"),
        gafira_doc::msg!("packs.settings_hint"),
    );
    if widgets::primary_icon(ui, theme, Icon::Package, t("packs.install_pack")).clicked() {
        change = Change::InstallPack;
    }
    ui.add_space(16.0);
    if packs.is_empty() {
        card(ui, theme, |ui| {
            ui.label(
                RichText::new(t("packs.none_installed"))
                    .font(theme.font("body"))
                    .color(theme.c("text.secondary")),
            );
        });
        return change;
    }
    let spanish = texts.locale == Locale::Es;
    for pack in packs {
        let m = &pack.manifest;
        card(ui, theme, |ui| {
            ui.horizontal_top(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(88.0), Sense::hover());
                crate::pack_dialog::checkerboard(ui, theme, rect);
                if let Some(image) = crate::brand_screen::file_image(ui.ctx(), &pack.thumbnail()) {
                    crate::brand_screen::paint_fitted(ui, image, rect);
                }
                ui.add_space(14.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(m.name.get(spanish))
                            .font(theme.font("ui-label"))
                            .color(theme.c("text")),
                    );
                    ui.add(
                        egui::Label::new(
                            RichText::new(m.description.get(spanish))
                                .font(theme.font("body-sm"))
                                .color(theme.c("text")),
                        )
                        .wrap(),
                    );
                    let kind = crate::packs::PackType::parse(&m.package_type)
                        .map(|k| t(k.key()))
                        .unwrap_or_default();
                    let assets = m.assets.len();
                    ui.label(
                        RichText::new(format!(
                            "{kind} · {} {} · {} · {}",
                            t("packs.version"),
                            m.version,
                            t("packs.assets_count").replace("{n}", &assets.to_string()),
                            crate::pack_dialog::size_text(pack.size)
                        ))
                        .font(theme.font("caption"))
                        .color(theme.c("text.secondary")),
                    );
                    ui.label(
                        RichText::new(crate::pack_dialog::author_line(
                            texts,
                            &m.author.name,
                            &pack.signer,
                        ))
                        .font(theme.font("caption"))
                        .color(theme.c("text.secondary")),
                    );
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if confirm.as_deref() == Some(m.id.as_str()) {
                            ui.label(
                                RichText::new(t("packs.uninstall_ask"))
                                    .font(theme.font("body-sm"))
                                    .color(theme.c("text")),
                            );
                            if widgets::primary(ui, theme, t("packs.uninstall")).clicked() {
                                change = Change::Uninstall(m.id.clone());
                                *confirm = None;
                            }
                            if widgets::secondary(ui, theme, t("packs.cancel")).clicked() {
                                *confirm = None;
                            }
                        } else if widgets::secondary(ui, theme, t("packs.uninstall")).clicked() {
                            *confirm = Some(m.id.clone());
                        }
                    });
                });
            });
        });
        ui.add_space(12.0);
    }
    change
}

fn about(ui: &mut egui::Ui, theme: &Theme, texts: &Texts, version: &str) {
    section_header(
        ui,
        theme,
        texts,
        gafira_doc::msg!("settings.about"),
        gafira_doc::msg!("settings.about_hint"),
    );
    egui::Frame::NONE
        .fill(theme.c("topbar.bg"))
        .corner_radius(12.0)
        .inner_margin(egui::Margin::same(24))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            widgets::topbar_logo(ui, 40.0);
            ui.add_space(12.0);
            ui.label(
                RichText::new(texts.t(&gafira_doc::msg!("settings.slogan")))
                    .font(theme.font("body"))
                    .color(theme.c("topbar.fg")),
            );
            ui.label(
                RichText::new(texts.t(&gafira_doc::msg!(
                    "splash.version",
                    version = Text(version.to_owned())
                )))
                .font(theme.font("caption"))
                .color(theme.c("topbar.fg.muted")),
            );
        });
    ui.add_space(16.0);
    card(ui, theme, |ui| {
        ui.label(
            RichText::new(texts.t(&gafira_doc::msg!("settings.fonts")))
                .font(theme.font("ui-label"))
                .color(theme.c("text")),
        );
        ui.add_space(6.0);
        let mut families: Vec<&str> = gafira_theme::library().iter().map(|f| f.family).collect();
        families.dedup();
        ui.label(
            RichText::new(texts.t(&gafira_doc::msg!(
                "settings.fonts_library",
                count = Text(families.len().to_string())
            )))
            .font(theme.font("body-sm"))
            .color(theme.c("text.secondary")),
        );
        ui.add_space(8.0);
        for font in gafira_theme::fonts() {
            egui::CollapsingHeader::new(format!(
                "{} · {}",
                font.family,
                texts.t(&gafira_doc::msg!("settings.font_license"))
            ))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(240.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(font.license).font(theme.font("caption")));
                    });
            });
        }
    });
    ui.add_space(16.0);
    card(ui, theme, |ui| {
        ui.label(
            RichText::new(texts.t(&gafira_doc::msg!("settings.included")))
                .font(theme.font("ui-label"))
                .color(theme.c("text")),
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new(texts.t(&gafira_doc::msg!("settings.included_text")))
                .font(theme.font("body-sm"))
                .color(theme.c("text.secondary")),
        );
    });
}

/// Whether `appearance` draws dark, given the system's mode if it is known.
pub(crate) fn is_dark(appearance: Appearance, system_dark: Option<bool>) -> bool {
    match appearance {
        Appearance::Dark => true,
        Appearance::Light => false,
        Appearance::System => system_dark.unwrap_or(false),
    }
}

#[cfg(test)]
mod tests {
    use gafira_ports::i18n::FormatLocale;

    use super::*;

    fn resolved(locale: Locale, source: LocaleSource) -> ResolvedLocale {
        ResolvedLocale {
            locale,
            formats: FormatLocale {
                locale,
                region: None,
            },
            source,
        }
    }

    #[test]
    fn s34_03_the_system_option_names_the_language_in_its_own_tongue() {
        let es_ui = Texts::new(Locale::Es);
        let en_ui = Texts::new(Locale::En);
        let mexico = resolved(
            Locale::Es,
            LocaleSource::System {
                tag: "es-MX".to_owned(),
            },
        );
        assert_eq!(
            system_label(&es_ui, &mexico),
            "Sistema · ahora: Español (es-MX)"
        );
        assert_eq!(
            system_label(&en_ui, &mexico),
            "System · now: Español (es-MX)"
        );
        let nothing = resolved(Locale::En, LocaleSource::Fallback);
        assert_eq!(
            system_label(&es_ui, &nothing),
            "Sistema · ahora: English (en)"
        );
    }

    #[test]
    fn s34_04_system_appearance_follows_the_system_and_defaults_to_light() {
        assert!(is_dark(Appearance::Dark, Some(false)));
        assert!(!is_dark(Appearance::Light, Some(true)));
        assert!(is_dark(Appearance::System, Some(true)));
        assert!(!is_dark(Appearance::System, Some(false)));
        assert!(
            !is_dark(Appearance::System, None),
            "unknown system mode reads light"
        );
    }
}
