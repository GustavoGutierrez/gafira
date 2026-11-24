//! Abrir, Guardar, Guardar como y Guardar copia (03 §2.2, 02 §5.4): the
//! system file dialog and the engine calls behind them.

use std::path::{Path, PathBuf};

use gafira_doc::msg::Message;
use gafira_engine::{Gafira, SaveRequest};

/// What the Archivo menu, Inicio or a shortcut asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileAction {
    Open,
    Save,
    SaveAs,
    SaveCopy,
}

/// What came of an action, for the shell.
pub(crate) enum Outcome {
    /// Nothing changed: the dialog was cancelled.
    Cancelled,
    /// The action ran.
    Done,
    /// The action failed; the message is for the status line, the error for
    /// the log.
    Failed(Message, String),
}

/// `path` with the `.gaf` extension, added when the person typed none.
pub(crate) fn with_gaf(path: PathBuf) -> PathBuf {
    if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("gaf"))
    {
        path
    } else {
        let mut name = path.into_os_string();
        name.push(".gaf");
        PathBuf::from(name)
    }
}

/// The name of the project in the top bar: the file name without `.gaf`.
pub(crate) fn project_name(path: Option<&Path>) -> Option<String> {
    path.and_then(Path::file_stem)
        .map(|stem| stem.to_string_lossy().into_owned())
}

fn dialog(window: Option<&eframe::Frame>) -> rfd::FileDialog {
    picker(window).add_filter("Gafira", &["gaf"])
}

/// What an action of the Archivo menu comes to once its dialog closed.
pub(crate) enum Plan {
    /// «Abrir» ran here (or its dialog was cancelled).
    Opened(Outcome),
    /// A save to run on its thread (spec 036).
    Save(SaveRequest),
    /// The save dialog was cancelled.
    Cancelled,
}

/// The new file of a project, from the system save dialog: the session's
/// folder and name are proposed.
pub(crate) fn pick_new(engine: &Gafira, window: Option<&eframe::Frame>) -> Option<PathBuf> {
    let start = engine.projects().path();
    let mut picker = dialog(window);
    if let Some(folder) = start.as_deref().and_then(Path::parent) {
        picker = picker.set_directory(folder);
    }
    if let Some(name) = project_name(start.as_deref()) {
        picker = picker.set_file_name(format!("{name}.gaf"));
    }
    picker.save_file().map(with_gaf)
}

/// `action`: the dialog first when it needs a path, then «Abrir» here or
/// the save to run.
pub(crate) fn plan(engine: &Gafira, action: FileAction, window: Option<&eframe::Frame>) -> Plan {
    match action {
        FileAction::Open => Plan::Opened(match dialog(window).pick_file() {
            None => Outcome::Cancelled,
            Some(path) => match engine.projects().open(&path) {
                Ok(()) => Outcome::Done,
                Err(e) => Outcome::Failed(gafira_doc::msg!("file.open_failed"), e.to_string()),
            },
        }),
        FileAction::Save if engine.projects().path().is_some() => Plan::Save(SaveRequest::Save),
        FileAction::Save | FileAction::SaveAs | FileAction::SaveCopy => {
            match pick_new(engine, window) {
                None => Plan::Cancelled,
                Some(path) if action == FileAction::SaveCopy => {
                    Plan::Save(SaveRequest::SaveCopy(path))
                }
                Some(path) => Plan::Save(SaveRequest::SaveAs(path)),
            }
        }
    }
}

/// The extensions of the images Gafira imports (spec 087).
pub(crate) const IMAGES: [&str; 5] = ["png", "jpg", "jpeg", "webp", "avif"];

/// What the image `path` is, whatever its name says.
fn reader(path: &std::path::Path) -> Option<image::ImageReader<std::io::BufReader<std::fs::File>>> {
    image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()
}

/// The size of the image `path`.
pub(crate) fn dimensions(path: &std::path::Path) -> Option<(u32, u32)> {
    reader(path)?.into_dimensions().ok()
}

/// The image `path`.
pub(crate) fn open(path: &std::path::Path) -> Option<image::DynamicImage> {
    reader(path)?.decode().ok()
}

/// A system file dialog over Gafira's window, when it is known, so the
/// window manager puts it in front (owner report: some opened behind).
pub(crate) fn picker(window: Option<&eframe::Frame>) -> rfd::FileDialog {
    let picker = rfd::FileDialog::new();
    match window {
        Some(window) => picker.set_parent(window),
        None => picker,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s19_10_the_gaf_extension_is_added_once() {
        assert_eq!(with_gaf("/w/poster".into()), PathBuf::from("/w/poster.gaf"));
        assert_eq!(
            with_gaf("/w/poster.gaf".into()),
            PathBuf::from("/w/poster.gaf")
        );
        assert_eq!(
            with_gaf("/w/poster.GAF".into()),
            PathBuf::from("/w/poster.GAF")
        );
        assert_eq!(
            with_gaf("/w/poster.png".into()),
            PathBuf::from("/w/poster.png.gaf")
        );
        assert_eq!(
            project_name(Some(Path::new("/w/Mi cartel.gaf"))).as_deref(),
            Some("Mi cartel")
        );
        assert_eq!(project_name(None), None);
    }
}
