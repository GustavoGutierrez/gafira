//! Layer-4 adapter `gafira-platform` (02 §2.1, spec 018): the operating
//! system behind the ports the engine needs to run in the desktop app —
//! files, time, project identities, processes, the system language, notices,
//! the settings file, the recent projects (spec 025), the app's data folder
//! and «Mostrar en carpeta» (spec 039).

#![forbid(unsafe_code)]

#[cfg(not(any(
    all(target_arch = "x86_64", target_os = "windows"),
    all(target_arch = "x86_64", target_os = "linux"),
    all(target_arch = "aarch64", target_os = "macos"),
)))]
compile_error!("Gafira targets only x86_64 Windows, x86_64 Linux and aarch64 macOS (ARQ-03)");

mod clock;
mod dirs;
mod fonts;
mod fs;
mod ids;
mod locale;
mod notify;
mod processes;
mod recent;
mod reveal;
mod settings;

pub use clock::SystemClock;
pub use dirs::data_dir;
pub use fonts::{SystemFace, face_of, font_dirs, scan_fonts, system_fonts};
pub use fs::StdFileSystem;
pub use ids::RandomIds;
pub use locale::EnvLocale;
pub use notify::StderrNotifier;
pub use processes::SystemProcesses;
pub use recent::{MAX_RECENT, RecentProject, RecentProjects, push};
pub use reveal::{reveal_command, show_in_folder};
pub use settings::JsonSettings;
