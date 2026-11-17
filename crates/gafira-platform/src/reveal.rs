//! «Mostrar en carpeta» (03 §7.5, 02 §6.4, spec 039): the system file
//! manager opens on the folder of an exported file.

use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

/// The program and the arguments that show `path` in the file manager of
/// this system: `xdg-open` on its folder in Linux, `explorer /select,` in
/// Windows and `open -R` in macOS (both select the file).
#[must_use]
pub fn reveal_command(path: &Path) -> (&'static str, Vec<OsString>) {
    if cfg!(target_os = "windows") {
        let mut select = OsString::from("/select,");
        select.push(path.as_os_str());
        ("explorer", vec![select])
    } else if cfg!(target_os = "macos") {
        ("open", vec!["-R".into(), path.as_os_str().to_owned()])
    } else {
        let folder = path
            .parent()
            .filter(|folder| !folder.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        ("xdg-open", vec![folder.as_os_str().to_owned()])
    }
}

/// Open the file manager on the folder of `path`. The process is started
/// and left to run: this returns at once, and a thread reaps it when it
/// ends.
///
/// # Errors
/// The I/O error when the program cannot be started.
pub fn show_in_folder(path: &Path) -> io::Result<()> {
    let (program, args) = reveal_command(path);
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    std::thread::Builder::new()
        .name("gafira-reveal".to_owned())
        .spawn(move || {
            let _ = child.wait();
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    // S39-07: Linux opens the folder of the file, never the file itself.
    #[cfg(target_os = "linux")]
    #[test]
    fn s39_07_linux_opens_the_folder_of_the_file() {
        let (program, args) = reveal_command(&PathBuf::from("/work/out/poster.png"));
        assert_eq!(program, "xdg-open");
        assert_eq!(args, [OsString::from("/work/out")]);
        let (_, bare) = reveal_command(Path::new("poster.png"));
        assert_eq!(bare, [OsString::from(".")]);
    }

    // S39-07: Windows and macOS select the file in its folder.
    #[cfg(not(target_os = "linux"))]
    #[test]
    fn s39_07_the_file_is_selected_in_its_folder() {
        let path = PathBuf::from("poster.png");
        let (program, args) = reveal_command(&path);
        assert!(matches!(program, "explorer" | "open"));
        assert!(
            args.last()
                .is_some_and(|arg| arg.to_string_lossy().ends_with("poster.png"))
        );
    }
}
