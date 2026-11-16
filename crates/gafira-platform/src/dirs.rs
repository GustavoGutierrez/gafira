//! The app's data folder (`datos_app` of 02 §7.2), per OS.

use std::path::PathBuf;

/// `$XDG_DATA_HOME/gafira` or `~/.local/share/gafira` on Linux,
/// `%APPDATA%\Gafira` on Windows, `~/Library/Application Support/Gafira` on
/// macOS. `None` when the environment names no home.
#[must_use]
pub fn data_dir() -> Option<PathBuf> {
    let var = |name: &str| {
        std::env::var_os(name)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    if cfg!(target_os = "windows") {
        var("APPDATA").map(|base| base.join("Gafira"))
    } else if cfg!(target_os = "macos") {
        var("HOME").map(|home| home.join("Library/Application Support/Gafira"))
    } else {
        var("XDG_DATA_HOME")
            .or_else(|| var("HOME").map(|home| home.join(".local/share")))
            .map(|base| base.join("gafira"))
    }
}
