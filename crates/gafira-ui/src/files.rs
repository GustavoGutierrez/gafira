//! Abrir, Guardar, Guardar como y Guardar copia (03 §2.2, 02 §5.4): the
//! system file dialog and the engine calls behind them.

use std::path::{Path, PathBuf};

use gafira_doc::msg::Message;
use gafira_engine::Gafira;

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

fn dialog() -> rfd::FileDialog {
    rfd::FileDialog::new().add_filter("Gafira", &["gaf"])
}

/// Run `action`: the dialog first when it needs a path, then the engine.
pub(crate) fn run(engine: &Gafira, action: FileAction) -> Outcome {
    let failed_save = || gafira_doc::msg!("file.save_failed");
    match action {
        FileAction::Open => match dialog().pick_file() {
            None => Outcome::Cancelled,
            Some(path) => match engine.projects().open(&path) {
                Ok(()) => Outcome::Done,
                Err(e) => Outcome::Failed(gafira_doc::msg!("file.open_failed"), e.to_string()),
            },
        },
        FileAction::Save if engine.projects().path().is_some() => match engine.projects().save() {
            Ok(()) => Outcome::Done,
            Err(e) => Outcome::Failed(failed_save(), e.to_string()),
        },
        FileAction::Save | FileAction::SaveAs | FileAction::SaveCopy => {
            let start = engine.projects().path();
            let mut picker = dialog();
            if let Some(folder) = start.as_deref().and_then(Path::parent) {
                picker = picker.set_directory(folder);
            }
            if let Some(name) = project_name(start.as_deref()) {
                picker = picker.set_file_name(format!("{name}.gaf"));
            }
            let Some(path) = picker.save_file().map(with_gaf) else {
                return Outcome::Cancelled;
            };
            let result = if action == FileAction::SaveCopy {
                engine.projects().save_copy(&path)
            } else {
                engine.projects().save_as(&path)
            };
            match result {
                Ok(()) => Outcome::Done,
                Err(e) => Outcome::Failed(failed_save(), e.to_string()),
            }
        }
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
