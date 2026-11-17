//! The texts of the shell, by key (ARQ-97).

use std::time::Duration;

use gafira_doc::msg::Message;
use gafira_i18n::CatalogTranslator;
use gafira_ports::i18n::{FormatLocale, Locale, Translator};

/// The catalogs and the language in force.
pub(crate) struct Texts {
    translator: CatalogTranslator,
    pub(crate) locale: Locale,
}

impl Texts {
    pub(crate) fn new(locale: Locale) -> Self {
        Self {
            translator: CatalogTranslator::new(),
            locale,
        }
    }

    /// The text of `message` in the language in force.
    pub(crate) fn t(&self, message: &Message) -> String {
        self.translator.text(self.locale, message).into_owned()
    }

    /// «hace 2 h»: how long ago, in the language in force (spec 025).
    pub(crate) fn relative(&self, delta: Duration) -> String {
        let formats = FormatLocale {
            locale: self.locale,
            region: None,
        };
        self.translator.relative(&formats, delta)
    }
}

#[cfg(test)]
mod tests {
    use gafira_doc::msg::MessageKey;

    use super::*;

    /// Every key the shell shows (kept in step with the screens by review;
    /// a key the catalogs lack fails the build of `gafira-i18n` only when it
    /// is defined, so this list is what catches a typo in a screen).
    const USED: &[&str] = &[
        "app.name",
        "brand.slogan",
        "splash.version",
        "home.question",
        "home.start",
        "home.templates",
        "home.recent",
        "home.see_all",
        "home.empty",
        "home.open",
        "time.just_now",
        "time.minutes_ago",
        "time.hours_ago",
        "time.days_ago",
        "file.new",
        "file.open",
        "file.save",
        "file.save_as",
        "file.save_copy",
        "file.save_version",
        "file.saving",
        "file.busy",
        "versions.title",
        "versions.name_title",
        "versions.name_hint",
        "versions.save",
        "versions.cancel",
        "versions.restore",
        "versions.close",
        "versions.auto",
        "versions.when",
        "versions.empty",
        "versions.save_failed",
        "versions.restore_failed",
        "file.save_failed",
        "file.open_failed",
        "recovery.title",
        "recovery.recover",
        "recovery.open_saved",
        "recovery.discard",
        "recovery.confirm_discard",
        "recovery.unnamed",
        "recovery.failed",
        "leave.title",
        "leave.body",
        "leave.save",
        "leave.discard",
        "leave.cancel",
        "home.format.social",
        "home.format.presentation",
        "home.format.print",
        "home.format.video_vertical",
        "home.format.video_horizontal",
        "home.format.custom",
        "home.size.social",
        "home.size.presentation",
        "home.size.print",
        "home.size.video_vertical",
        "home.size.video_horizontal",
        "home.size.custom",
        "nav.home",
        "nav.projects",
        "nav.templates",
        "nav.settings",
        "settings.title",
        "settings.back",
        "settings.general",
        "settings.privacy",
        "settings.privacy_text",
        "settings.about",
        "settings.fonts",
        "settings.font_license",
        "settings.appearance.title",
        "settings.appearance.light",
        "settings.appearance.dark",
        "settings.appearance.system",
        "settings.language.title",
        "settings.language.system",
        "settings.language.system_now",
        "settings.language.es",
        "settings.language.en",
        "settings.language.help",
        "nav.search",
        "menu.file",
        "view.dark_mode",
        "menu.edit",
        "menu.view",
        "menu.size",
        "editor.untitled",
        "editor.saved",
        "editor.versions",
        "editor.export",
        "editor.page",
        "editor.page_of",
        "editor.search_templates",
        "editor.unsaved",
        "editor.undo",
        "editor.redo",
        "editor.add_page",
        "tab.templates",
        "tab.pieces",
        "tab.text",
        "tab.identity",
        "tab.library",
        "tab.drawing",
        "inspector.position",
        "inspector.layers",
        "inspector.image",
        "inspector.empty",
        "layers.up",
        "layers.down",
        "layers.empty",
        "layers.group",
        "layers.element",
        "layers.text",
        "layers.image",
        "library.import_image",
        "library.import_failed",
        "text.add_title",
        "text.add_subtitle",
        "text.add_body",
        "text.title",
        "text.subtitle",
        "text.body",
        "inspector.text",
        "inspector.font_size",
        "inspector.text_color",
        "inspector.x",
        "inspector.y",
        "inspector.width",
        "inspector.height",
        "inspector.fill",
        "inspector.opacity",
        "inspector.no_fill",
        "inspector.outline",
        "inspector.no_outline",
        "inspector.outline_width",
        "inspector.corner_radius",
        "pieces.shapes",
        "pieces.rect",
        "pieces.ellipse",
        "bottom.notes",
        "bottom.pages",
        "format.percent",
        "unit.px",
        "export.done",
        "export.title",
        "export.format",
        "export.png",
        "export.pdf",
        "export.scale",
        "export.scale_1x",
        "export.scale_2x",
        "export.transparent",
        "export.resolution",
        "export.action",
        "export.failed",
        "export.not_allowed",
        "export.no_pages",
        "export.selection_unsupported",
        "export.running",
        "export.cancel",
        "export.cancelled",
        "export.busy",
    ];

    #[test]
    fn s17_03_every_key_the_shell_shows_exists_in_both_languages() {
        let translator = CatalogTranslator::new();
        for key in USED {
            for locale in [Locale::Es, Locale::En] {
                assert!(
                    translator.has(locale, MessageKey(key)),
                    "{key} ({locale:?})"
                );
            }
        }
    }

    #[test]
    fn s17_03_the_same_key_reads_in_each_language() {
        let message = gafira_doc::msg!("home.start");
        assert_eq!(Texts::new(Locale::Es).t(&message), "Empezar a diseñar");
        assert_eq!(Texts::new(Locale::En).t(&message), "Start designing");
    }
}
