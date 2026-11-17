//! Layer-4 adapter `gafira-ui` (02 §2.1, spec 017): the egui/eframe desktop
//! shell of Gafira.
//!
//! This first delivery draws the Startup screen (03 §7.10, D-09), Inicio
//! (03 §7.2) and the regions of the Design editor (03 §7.1, §7.3) with the
//! tokens of `gafira-theme` and the texts of `gafira-i18n`. Every visible text
//! comes from the catalogs by key (ARQ-97); no colour is written in code.
//! Since spec 018 the editor runs on the engine's Facade: «Empezar a
//! diseñar» creates the document, the canvas draws its snapshot, and undo,
//! redo and «Añadir página» go through `EditService` (02 §5.5).

#![forbid(unsafe_code)]

mod canvas;
mod dialogs;
mod editor;
mod export;
mod files;
mod format;
mod home;
mod recent;
mod saving;
mod settings;
mod shell;
mod splash;
mod style;
mod text;
mod versions;
mod widgets;

pub use recent::{RecentProject, RecentStore, RecentThumbnail};
pub use shell::{EngineFactory, UiConfig, run};
