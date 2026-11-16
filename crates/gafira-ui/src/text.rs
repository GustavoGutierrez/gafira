//! The texts of the shell, by key (ARQ-97).

use gafira_doc::msg::Message;
use gafira_i18n::CatalogTranslator;
use gafira_ports::i18n::{Locale, Translator};

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
        "file.new",
        "file.open",
        "file.save",
        "file.save_as",
        "file.save_copy",
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
        "nav.search",
        "menu.file",
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
        "inspector.x",
        "inspector.y",
        "inspector.width",
        "inspector.height",
        "pieces.shapes",
        "pieces.rect",
        "pieces.ellipse",
        "bottom.notes",
        "bottom.pages",
        "format.percent",
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
