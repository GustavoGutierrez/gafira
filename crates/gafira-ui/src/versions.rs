//! «Versiones» in the editor's top bar and Archivo → «Guardar versión…»
//! (03 §2.2, 02 §7.1; spec 031): the list of the project's versions, newest
//! first, each with «Restaurar», and the dialog that asks a version's name.

use std::time::{SystemTime, UNIX_EPOCH};

use egui::RichText;
use gafira_engine::Gafira;
use gafira_ports::repository::{VersionId, VersionInfo, VersionKind};

use crate::recent;
use crate::style::Theme;
use crate::text::Texts;
use crate::widgets;

/// Which of the two dialogs is open, and what went wrong last.
#[derive(Default)]
pub(crate) struct Versions {
    /// The list is on display.
    list: bool,
    /// The name being typed, while «Guardar versión» is open.
    naming: Option<String>,
    /// The text of the last failure, for the open dialog.
    failure: Option<String>,
}

/// One line of the list: the version, its name and when it was made.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Row {
    pub(crate) id: VersionId,
    pub(crate) name: String,
    pub(crate) when: String,
}

/// The lines of the list for `infos` at `now`: newest first; an automatic
/// version without a name reads «Automática».
pub(crate) fn rows(infos: &[VersionInfo], now: i64, texts: &Texts) -> Vec<Row> {
    infos
        .iter()
        .rev()
        .map(|info| Row {
            id: info.id,
            name: if info.name.is_empty() && info.kind == VersionKind::Auto {
                texts.t(&gafira_doc::msg!("versions.auto"))
            } else {
                info.name.clone()
            },
            when: texts.t(&gafira_doc::msg!(
                "versions.when",
                date = Timestamp(info.created.0),
                ago = Text(texts.relative(recent::age(info.created.0, now)))
            )),
        })
        .collect()
}

impl Versions {
    /// Open the list (the top bar's «Versiones»).
    pub(crate) fn open_list(&mut self) {
        self.list = true;
        self.failure = None;
    }

    /// Open «Guardar versión» with an empty name.
    pub(crate) fn ask_name(&mut self) {
        self.naming = Some(String::new());
        self.failure = None;
    }

    /// Draw the open dialog, if any, and run what the person chose; the
    /// name of the version to save when they pressed «Guardar» (the shell
    /// saves it, spec 036).
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        engine: &Gafira,
        theme: &Theme,
        texts: &Texts,
    ) -> Option<String> {
        if let Some(name) = self.naming.take() {
            let (naming, save) = self.name_dialog(ctx, theme, texts, name);
            self.naming = naming;
            return save;
        }
        if self.list {
            self.list_dialog(ctx, engine, theme, texts);
        }
        None
    }

    /// The name dialog: the name while it stays open, and the name to save
    /// when the person pressed «Guardar».
    fn name_dialog(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        mut name: String,
    ) -> (Option<String>, Option<String>) {
        let mut save = false;
        let mut cancel = false;
        let response = egui::Modal::new(egui::Id::new("versions.name")).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.label(
                RichText::new(texts.t(&gafira_doc::msg!("versions.name_title")))
                    .font(theme.font("h3")),
            );
            ui.add_space(8.0);
            let field = ui.add(
                egui::TextEdit::singleline(&mut name)
                    .hint_text(texts.t(&gafira_doc::msg!("versions.name_hint")))
                    .desired_width(f32::INFINITY),
            );
            field.request_focus();
            let ready = !name.trim().is_empty();
            if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && ready {
                save = true;
            }
            if let Some(failure) = &self.failure {
                ui.label(RichText::new(failure).color(theme.c("primary.bg")));
            }
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                let primary = ui.add_enabled_ui(ready, |ui| {
                    widgets::primary(ui, theme, texts.t(&gafira_doc::msg!("versions.save")))
                });
                save |= primary.inner.clicked();
                cancel |=
                    widgets::secondary(ui, theme, texts.t(&gafira_doc::msg!("versions.cancel")))
                        .clicked();
            });
        });
        if cancel || response.should_close() {
            return (None, None);
        }
        if !save {
            return (Some(name), None);
        }
        // The shell saves it on its thread (spec 036); a failure is its
        // notice.
        (None, Some(name))
    }

    /// The list of versions, newest first, with «Restaurar» on each.
    fn list_dialog(&mut self, ctx: &egui::Context, engine: &Gafira, theme: &Theme, texts: &Texts) {
        let rows = rows(&engine.projects().list_versions(), unix_now(), texts);
        let mut restore = None;
        let mut close = false;
        let response = egui::Modal::new(egui::Id::new("versions.list")).show(ctx, |ui| {
            ui.set_width(480.0);
            ui.label(
                RichText::new(texts.t(&gafira_doc::msg!("versions.title"))).font(theme.font("h3")),
            );
            if let Some(failure) = &self.failure {
                ui.label(RichText::new(failure).color(theme.c("primary.bg")));
            }
            ui.add_space(12.0);
            if rows.is_empty() {
                ui.label(texts.t(&gafira_doc::msg!("versions.empty")));
            }
            egui::ScrollArea::vertical()
                .max_height(360.0)
                .show(ui, |ui| {
                    for row in &rows {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&row.name).font(theme.font("ui-label")));
                                ui.label(
                                    RichText::new(&row.when)
                                        .font(theme.font("caption"))
                                        .color(theme.c("text.muted")),
                                );
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if widgets::secondary(
                                        ui,
                                        theme,
                                        texts.t(&gafira_doc::msg!("versions.restore")),
                                    )
                                    .clicked()
                                    {
                                        restore = Some(row.id);
                                    }
                                },
                            );
                        });
                        ui.add_space(8.0);
                    }
                });
            ui.add_space(12.0);
            close |= widgets::secondary(ui, theme, texts.t(&gafira_doc::msg!("versions.close")))
                .clicked();
        });
        if let Some(id) = restore {
            match engine.projects().restore_version(id) {
                Ok(()) => close = true,
                Err(error) => {
                    eprintln!("restore version: {error}");
                    self.failure = Some(texts.t(&gafira_doc::msg!("versions.restore_failed")));
                }
            }
        }
        if close || response.should_close() {
            self.list = false;
            self.failure = None;
        }
    }
}

/// Seconds since the Unix epoch, for «hace 2 h».
fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

#[cfg(test)]
mod tests {
    use gafira_ports::common::UnixTime;
    use gafira_ports::i18n::Locale;

    use super::*;

    fn info(id: u64, name: &str, kind: VersionKind, created: i64) -> VersionInfo {
        VersionInfo {
            id: VersionId(id),
            name: name.to_owned(),
            kind,
            created: UnixTime(created),
            app: "0.1.0".to_owned(),
            schema: 1,
        }
    }

    // S31-10
    #[test]
    fn s31_10_the_list_runs_newest_first_and_names_the_automatic_ones() {
        let day = 1_763_251_200; // 2025-11-16 00:00 UTC
        let infos = [
            info(1, "Primera idea", VersionKind::Named, day),
            info(2, "", VersionKind::Auto, day + 3_600),
        ];
        let texts = Texts::new(Locale::Es);
        let rows = rows(&infos, day + 7_200, &texts);
        assert_eq!(
            rows,
            [
                Row {
                    id: VersionId(2),
                    name: "Automática".to_owned(),
                    when: "2025-11-16 · hace 1 h".to_owned(),
                },
                Row {
                    id: VersionId(1),
                    name: "Primera idea".to_owned(),
                    when: "2025-11-16 · hace 2 h".to_owned(),
                },
            ]
        );
        let english = super::rows(&infos[1..], day + 7_200, &Texts::new(Locale::En));
        assert_eq!(english[0].name, "Automatic");
    }
}
