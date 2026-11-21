//! The two dialogs over any screen: Recuperación tras fallo (03 §7.9) and
//! «¿Guardar los cambios?» (03 §6.1).

use egui::RichText;
use gafira_engine::RecoveryEntry;

use crate::files::project_name;
use crate::style::Theme;
use crate::text::Texts;
use crate::widgets;

/// What the person chose in the recovery dialog.
pub(crate) enum RecoveryChoice {
    None,
    Recover(usize),
    OpenSaved(usize),
    AskDiscard(usize),
    Discard(usize),
    CancelDiscard,
}

/// The recovery dialog; `confirming` is the entry whose «Descartar» waits
/// for confirmation.
pub(crate) fn recovery(
    ctx: &egui::Context,
    theme: &Theme,
    texts: &Texts,
    entries: &[RecoveryEntry],
    confirming: Option<usize>,
    failure: Option<&str>,
) -> RecoveryChoice {
    let mut choice = RecoveryChoice::None;
    egui::Modal::new(egui::Id::new("recovery")).show(ctx, |ui| {
        ui.set_width(520.0);
        ui.label(
            RichText::new(texts.t(&gafira_doc::msg!("recovery.title"))).font(theme.font("h3")),
        );
        if let Some(failure) = failure {
            ui.label(RichText::new(failure).color(theme.c("primary.bg")));
        }
        ui.add_space(12.0);
        for (index, entry) in entries.iter().enumerate() {
            let name = project_name(entry.origin.as_deref())
                .unwrap_or_else(|| texts.t(&gafira_doc::msg!("recovery.unnamed")));
            ui.label(RichText::new(name).font(theme.font("ui-label")));
            if confirming == Some(index) {
                ui.label(texts.t(&gafira_doc::msg!("recovery.confirm_discard")));
                ui.horizontal(|ui| {
                    if widgets::secondary(ui, theme, texts.t(&gafira_doc::msg!("recovery.discard")))
                        .clicked()
                    {
                        choice = RecoveryChoice::Discard(index);
                    }
                    if widgets::secondary(ui, theme, texts.t(&gafira_doc::msg!("leave.cancel")))
                        .clicked()
                    {
                        choice = RecoveryChoice::CancelDiscard;
                    }
                });
            } else {
                ui.horizontal(|ui| {
                    if widgets::primary(ui, theme, texts.t(&gafira_doc::msg!("recovery.recover")))
                        .clicked()
                    {
                        choice = RecoveryChoice::Recover(index);
                    }
                    if entry.origin.is_some()
                        && widgets::secondary(
                            ui,
                            theme,
                            texts.t(&gafira_doc::msg!("recovery.open_saved")),
                        )
                        .clicked()
                    {
                        choice = RecoveryChoice::OpenSaved(index);
                    }
                    if widgets::secondary(ui, theme, texts.t(&gafira_doc::msg!("recovery.discard")))
                        .clicked()
                    {
                        choice = RecoveryChoice::AskDiscard(index);
                    }
                });
            }
            ui.add_space(12.0);
        }
    });
    choice
}

/// What the person chose in «¿Guardar los cambios?».
pub(crate) enum LeaveChoice {
    None,
    Save,
    Discard,
    Cancel,
}

pub(crate) fn leave(ctx: &egui::Context, theme: &Theme, texts: &Texts) -> LeaveChoice {
    let mut choice = LeaveChoice::None;
    let response = egui::Modal::new(egui::Id::new("leave")).show(ctx, |ui| {
        ui.set_width(420.0);
        ui.label(RichText::new(texts.t(&gafira_doc::msg!("leave.title"))).font(theme.font("h3")));
        ui.label(texts.t(&gafira_doc::msg!("leave.body")));
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if widgets::primary(ui, theme, texts.t(&gafira_doc::msg!("leave.save"))).clicked() {
                choice = LeaveChoice::Save;
            }
            if widgets::secondary(ui, theme, texts.t(&gafira_doc::msg!("leave.discard"))).clicked()
            {
                choice = LeaveChoice::Discard;
            }
            if widgets::secondary(ui, theme, texts.t(&gafira_doc::msg!("leave.cancel"))).clicked() {
                choice = LeaveChoice::Cancel;
            }
        });
    });
    if response.should_close() && matches!(choice, LeaveChoice::None) {
        choice = LeaveChoice::Cancel;
    }
    choice
}

/// The answer to «¿Cómo quieres importarlo?» (spec 067).
pub(crate) enum SvgImport {
    Waiting,
    Image,
    Frame,
    Cancel,
}

/// «¿Cómo quieres importarlo?»: an SVG comes in as it is, as an image, or
/// as a frame of its outline (spec 067). One question for all the files.
pub(crate) fn svg_import(
    ctx: &egui::Context,
    theme: &Theme,
    texts: &Texts,
    files: &[std::path::PathBuf],
) -> SvgImport {
    let mut choice = SvgImport::Waiting;
    let response = egui::Modal::new(egui::Id::new("svg_import")).show(ctx, |ui| {
        ui.set_width(460.0);
        ui.label(
            RichText::new(texts.t(&gafira_doc::msg!("svg_import.title"))).font(theme.font("h3")),
        );
        let names: Vec<String> = files
            .iter()
            .filter_map(|f| f.file_name().and_then(|n| n.to_str()).map(str::to_owned))
            .collect();
        ui.label(
            RichText::new(names.join(", "))
                .font(theme.font("body-sm"))
                .color(theme.c("text.secondary")),
        );
        ui.add_space(8.0);
        ui.label(texts.t(&gafira_doc::msg!("svg_import.body")));
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if widgets::primary(ui, theme, texts.t(&gafira_doc::msg!("svg_import.image"))).clicked()
            {
                choice = SvgImport::Image;
            }
            if widgets::secondary(ui, theme, texts.t(&gafira_doc::msg!("svg_import.frame")))
                .clicked()
            {
                choice = SvgImport::Frame;
            }
            if widgets::secondary(ui, theme, texts.t(&gafira_doc::msg!("svg_import.cancel")))
                .clicked()
            {
                choice = SvgImport::Cancel;
            }
        });
    });
    if response.should_close() && matches!(choice, SvgImport::Waiting) {
        choice = SvgImport::Cancel;
    }
    choice
}
