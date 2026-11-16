//! Layer-4 adapter `gafira-ui` (02 §2.1, spec 017): the egui/eframe desktop
//! shell of Gafira.
//!
//! This first delivery draws the Startup screen (03 §7.10, D-09), Inicio
//! (03 §7.2) and the regions of the Design editor (03 §7.1, §7.3) with the
//! tokens of `gafira-theme` and the texts of `gafira-i18n`. Every visible text
//! comes from the catalogs by key (ARQ-97); no colour is written in code.
//! The engine is not wired yet: the editor shows the page of the chosen
//! format, and the next delivery connects it through the Facade.

#![forbid(unsafe_code)]

mod editor;
mod format;
mod home;
mod shell;
mod splash;
mod style;
mod text;
mod widgets;

pub use shell::{UiConfig, run};
