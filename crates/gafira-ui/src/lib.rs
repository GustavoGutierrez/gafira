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

mod app_icon;
mod arrange;
mod brand;
mod brand_panel;
mod brand_screen;
mod canvas;
mod chart_templates;
mod charts;
mod clipboard;
mod collage;
mod colors;
mod create;
mod crop;
mod design_art;
mod design_sizes;
mod dialogs;
mod draw;
mod editor;
mod elements;
mod eraser;
mod export;
mod files;
mod format;
mod frames;
mod gradients;
mod group;
mod home;
mod icons;
mod illustrations;
mod image_panel;
mod layers;
mod library;
mod lines;
mod looks;
mod magnet;
mod pairings;
mod photos;
mod qr;
mod recent;
mod recolor;
mod resize;
mod rotation;
mod ruler;
mod saving;
mod selection;
mod settings;
mod shapes;
mod sheets;
mod shell;
mod splash;
mod style;
mod system_fonts;
mod table_templates;
mod tables;
mod template_fit;
mod template_panel;
mod templates;
mod text;
mod text_effects;
mod textbox;
mod transfer;
mod versions;
mod vision;
mod widgets;
mod zones;
mod zoom;

pub use brand::{
    BRAND_TAG, BrandColor, BrandFiles, BrandKit, BrandPalette, BrandStore, BrandTextStyle,
    parse_hex as parse_brand_hex,
};
pub use export::{ExportPresets, PresetValue};
pub use library::{LibraryEntry, LibraryStore};
pub use recent::{RecentProject, RecentStore, RecentThumbnail};
pub use shell::{EngineFactory, ShowInFolder, UiConfig, run};
pub use system_fonts::{SystemFont, SystemFonts};
pub use templates::{TemplateListing, TemplateStore};

/// The guides a page of the size named `key` starts with in «Crear un
/// diseño» (spec 076): the folds, locked, and the safe margins. The
/// templates of that size carry the same ones (spec 091).
#[must_use]
pub fn size_guides(key: &str) -> Option<gafira_doc::GuideSet> {
    design_sizes::by_key(key).map(design_sizes::DesignSize::guides)
}
