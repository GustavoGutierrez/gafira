//! Ajustes (03 §7.8, §16.1; spec 034): sections on the left, content on the
//! right. General holds the language, Apariencia the interface mode,
//! Privacidad its promise and Acerca de the version and the bundled fonts.
//! Every change applies at once, with no «Aplicar» button (DIS-75).

use egui::RichText;
use gafira_ports::i18n::{LanguagePreference, Locale, LocaleSource, ResolvedLocale};
use gafira_ports::settings::{Appearance, Settings};

use crate::style::{Theme, px};
use crate::text::Texts;
use crate::widgets;

/// The sections this delivery has, in the order of 03 §7.8.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Section {
    General,
    Appearance,
    Privacy,
    About,
}

impl Section {
    const ALL: [Self; 4] = [Self::General, Self::Appearance, Self::Privacy, Self::About];

    fn title(self) -> gafira_doc::msg::Message {
        match self {
            Self::General => gafira_doc::msg!("settings.general"),
            Self::Appearance => gafira_doc::msg!("settings.appearance.title"),
            Self::Privacy => gafira_doc::msg!("settings.privacy"),
            Self::About => gafira_doc::msg!("settings.about"),
        }
    }
}

/// What the person changed this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Change {
    None,
    Back,
    Language(LanguagePreference),
    Appearance(Appearance),
}

/// The open Ajustes screen.
pub(crate) struct SettingsView {
    pub(crate) section: Section,
}

impl Default for SettingsView {
    fn default() -> Self {
        Self {
            section: Section::General,
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
/// to now.
pub(crate) fn show(
    ctx: &egui::Context,
    theme: &Theme,
    texts: &Texts,
    view: &mut SettingsView,
    current: Settings,
    system: &ResolvedLocale,
    version: &str,
) -> Change {
    let mut change = Change::None;
    widgets::topbar(ctx, theme, "settings.topbar", |ui| {
        if widgets::on_topbar(ui, theme, texts.t(&gafira_doc::msg!("settings.back")), true)
            .clicked()
        {
            change = Change::Back;
        }
        ui.add_space(16.0);
        ui.label(
            RichText::new(texts.t(&gafira_doc::msg!("settings.title")))
                .font(theme.font("ui-label"))
                .color(theme.c("topbar.fg")),
        );
    });
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        change = Change::Back;
    }
    egui::SidePanel::left("settings.sections")
        .exact_width(220.0)
        .resizable(false)
        .frame(
            egui::Frame::NONE
                .fill(theme.c("surface"))
                .inner_margin(egui::Margin::same(16)),
        )
        .show(ctx, |ui| {
            for section in Section::ALL {
                let label = RichText::new(texts.t(&section.title())).font(theme.font("ui-label"));
                if ui
                    .selectable_label(view.section == section, label)
                    .clicked()
                {
                    view.section = section;
                }
            }
        });
    egui::CentralPanel::default()
        .frame(
            egui::Frame::NONE
                .fill(theme.c("bg"))
                .inner_margin(egui::Margin::same(32)),
        )
        .show(ctx, |ui| {
            ui.set_max_width(px(560));
            match view.section {
                Section::General => {
                    heading(
                        ui,
                        theme,
                        texts,
                        gafira_doc::msg!("settings.language.title"),
                    );
                    let options = [
                        (LanguagePreference::System, system_label(texts, system)),
                        (
                            LanguagePreference::Es,
                            texts.t(&gafira_doc::msg!("settings.language.es")),
                        ),
                        (
                            LanguagePreference::En,
                            texts.t(&gafira_doc::msg!("settings.language.en")),
                        ),
                    ];
                    for (preference, label) in options {
                        if ui.radio(current.language == preference, label).clicked()
                            && current.language != preference
                        {
                            change = Change::Language(preference);
                        }
                    }
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(texts.t(&gafira_doc::msg!("settings.language.help")))
                            .font(theme.font("body-sm"))
                            .color(theme.c("text.secondary")),
                    );
                }
                Section::Appearance => {
                    heading(
                        ui,
                        theme,
                        texts,
                        gafira_doc::msg!("settings.appearance.title"),
                    );
                    for (appearance, key) in [
                        (
                            Appearance::Light,
                            gafira_doc::msg!("settings.appearance.light"),
                        ),
                        (
                            Appearance::Dark,
                            gafira_doc::msg!("settings.appearance.dark"),
                        ),
                        (
                            Appearance::System,
                            gafira_doc::msg!("settings.appearance.system"),
                        ),
                    ] {
                        if ui
                            .radio(current.appearance == appearance, texts.t(&key))
                            .clicked()
                            && current.appearance != appearance
                        {
                            change = Change::Appearance(appearance);
                        }
                    }
                }
                Section::Privacy => {
                    heading(ui, theme, texts, gafira_doc::msg!("settings.privacy"));
                    ui.label(texts.t(&gafira_doc::msg!("settings.privacy_text")));
                }
                Section::About => {
                    heading(ui, theme, texts, gafira_doc::msg!("app.name"));
                    ui.label(texts.t(&gafira_doc::msg!(
                        "splash.version",
                        version = Text(version.to_owned())
                    )));
                    ui.add_space(16.0);
                    heading(ui, theme, texts, gafira_doc::msg!("settings.fonts"));
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
                                    ui.label(
                                        RichText::new(font.license).font(theme.font("caption")),
                                    );
                                });
                        });
                    }
                }
            }
        });
    change
}

fn heading(ui: &mut egui::Ui, theme: &Theme, texts: &Texts, key: gafira_doc::msg::Message) {
    ui.label(
        RichText::new(texts.t(&key))
            .font(theme.font("h3"))
            .color(theme.c("text")),
    );
    ui.add_space(12.0);
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
