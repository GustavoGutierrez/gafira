//! «Ayuda» of the top bar (owner request): the keyboard shortcuts, the
//! licences of Gafira and of what it ships, the website, reporting a
//! problem and «Acerca de».

use egui::{KeyboardShortcut, RichText};
use gafira_doc::msg::{Message, MessageKey};

use crate::style::Theme;
use crate::text::Texts;

/// The website of Gafira.
pub(crate) const WEBSITE: &str = "https://gafira.com";

/// Where problems are reported.
pub(crate) const ISSUES: &str = "https://github.com/GustavoGutierrez/gafira/issues";

/// The notice of the third-party works Gafira ships, and its own licence.
const NOTICES: &str = include_str!("../../../THIRD_PARTY_NOTICES.md");
const LICENSE: &str = include_str!("../../../LICENSE");

/// A dialog of «Ayuda».
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Dialog {
    Shortcuts,
    Notices,
}

/// The shortcuts of a group: its title's key and each action's key with
/// its keys.
pub(crate) type Group = (&'static str, Vec<(&'static str, KeyboardShortcut)>);

fn t(texts: &Texts, key: &'static str) -> String {
    texts.t(&Message {
        key: MessageKey(key),
        args: Vec::new(),
    })
}

/// Draws `dialog`, if any, over the window; it closes with its button or a
/// click outside it.
pub(crate) fn show(
    ctx: &egui::Context,
    theme: &Theme,
    texts: &Texts,
    dialog: &mut Option<Dialog>,
    groups: &[Group],
) {
    let Some(which) = *dialog else {
        return;
    };
    let title = match which {
        Dialog::Shortcuts => t(texts, "help.shortcuts"),
        Dialog::Notices => t(texts, "help.notices"),
    };
    let mut open = true;
    let screen = ctx.content_rect();
    egui::Window::new(title)
        .id(egui::Id::new("help.dialog"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .fixed_size(egui::vec2(
            560.0_f32.min(screen.width() - 80.0),
            (screen.height() - 160.0).max(240.0),
        ))
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| match which {
                    Dialog::Shortcuts => shortcuts(ui, theme, texts, groups),
                    Dialog::Notices => notices(ui, theme, texts),
                });
        });
    if !open {
        *dialog = None;
    }
}

/// Every shortcut, by group, written in the keys of this system.
fn shortcuts(ui: &mut egui::Ui, theme: &Theme, texts: &Texts, groups: &[Group]) {
    for (title, items) in groups {
        ui.add_space(8.0);
        ui.label(
            RichText::new(t(texts, title))
                .font(theme.font("ui-label"))
                .color(theme.c("text.secondary")),
        );
        ui.add_space(4.0);
        egui::Grid::new(*title)
            .num_columns(2)
            .spacing([24.0, 6.0])
            .striped(true)
            .show(ui, |ui| {
                for (action, keys) in items {
                    ui.label(RichText::new(t(texts, action)).font(theme.font("body")));
                    ui.label(
                        RichText::new(ui.ctx().format_shortcut(keys))
                            .font(theme.font("body"))
                            .color(theme.c("text.secondary")),
                    );
                    ui.end_row();
                }
            });
    }
}

/// The licence of Gafira and the notice of what it ships.
fn notices(ui: &mut egui::Ui, theme: &Theme, texts: &Texts) {
    ui.label(
        RichText::new(t(texts, "help.license_title"))
            .font(theme.font("ui-label"))
            .color(theme.c("text")),
    );
    ui.label(
        RichText::new(t(texts, "help.license_text"))
            .font(theme.font("body-sm"))
            .color(theme.c("text.secondary")),
    );
    egui::CollapsingHeader::new(t(texts, "help.license_full"))
        .id_salt("help.license")
        .show(ui, |ui| {
            ui.label(RichText::new(LICENSE).font(theme.font("caption")));
        });
    ui.add_space(12.0);
    ui.label(RichText::new(NOTICES).font(theme.font("body-sm")));
}

#[cfg(test)]
mod tests {
    use super::*;

    // Owner request: the licence and the notice Gafira ships are the files
    // of the repository, whole.
    #[test]
    fn the_notices_are_the_files_of_the_repository() {
        assert!(LICENSE.contains("GNU GENERAL PUBLIC LICENSE"));
        assert!(NOTICES.contains("Open Font License"));
        assert!(WEBSITE.starts_with("https://") && ISSUES.starts_with("https://"));
    }
}
