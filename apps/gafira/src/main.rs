#![forbid(unsafe_code)]

//! `gafira`: the desktop app (02 §2.1). Composition only: it builds the
//! ports from the adapters, the engine over them, and hands both to the
//! shell (spec 018).

use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gafira_assets::FsStore;
use gafira_engine::{
    EngineConfig, EventChannel, Gafira, Ports, Registries, SettingsPorts, SystemPorts, Waker,
};
use gafira_format::{GafRepository, Limits, read_thumbnail};
use gafira_i18n::D08LocaleResolver;
use gafira_platform::{
    EnvLocale, JsonSettings, RandomIds, RecentProjects, StdFileSystem, StderrNotifier, SystemClock,
    SystemProcesses,
};
use gafira_ports::export::{Exporter, PageRenderers};
use gafira_ports::i18n::{LanguagePreference, Locale, LocaleResolver};
use gafira_ports::license::NoLicensing;
use gafira_ports::repository::RepositoryError;
use gafira_ports::store::ContentStore;
use gafira_ports::system::Clock;
use gafira_render::{FontBook, GraphPageRenderers};
use gafira_render_cpu::CpuBackend;

/// «Recientes» of Inicio over `<data_dir>/recent.json` (spec 025).
struct Recent {
    file: RecentProjects,
    clock: SystemClock,
}

impl gafira_ui::RecentStore for Recent {
    fn list(&self) -> Vec<gafira_ui::RecentProject> {
        let list = self.file.existing().unwrap_or_else(|error| {
            eprintln!("recent: {error}");
            let mut list = self.file.load();
            list.retain(|project| project.path.exists());
            list
        });
        list.into_iter()
            .map(|project| gafira_ui::RecentProject {
                path: project.path,
                name: project.name,
                opened_at: project.opened_at,
            })
            .collect()
    }

    fn record(&self, path: &Path, name: &str) {
        let project = gafira_platform::RecentProject {
            path: path.to_path_buf(),
            name: name.to_owned(),
            opened_at: self.clock.now().0,
        };
        if let Err(error) = self.file.record(project) {
            eprintln!("recent: {error}");
        }
    }

    /// Spec 033: `thumbs/<first page>.webp` of the file, read without
    /// opening the project.
    fn thumbnail(&self, path: &Path) -> Option<gafira_ui::RecentThumbnail> {
        let read = std::fs::File::open(path)
            .map_err(RepositoryError::Io)
            .and_then(|file| read_thumbnail(BufReader::new(file), Limits::default()));
        match read {
            Ok(thumb) => thumb.map(|thumb| gafira_ui::RecentThumbnail {
                width: thumb.width,
                height: thumb.height,
                rgba: thumb.rgba,
            }),
            Err(error) => {
                eprintln!("recent thumbnail {}: {error}", path.display());
                None
            }
        }
    }
}

/// `--lang es|en` overrides the language for this run without saving it.
fn language_flag() -> Option<LanguagePreference> {
    let args: Vec<String> = std::env::args().collect();
    match args
        .windows(2)
        .find(|w| w[0] == "--lang")
        .map(|w| w[1].as_str())
    {
        Some("es") => Some(LanguagePreference::Es),
        Some("en") => Some(LanguagePreference::En),
        _ => None,
    }
}

fn main() -> eframe::Result {
    let data_dir =
        gafira_platform::data_dir().unwrap_or_else(|| std::env::temp_dir().join("gafira"));
    let resolver = Arc::new(D08LocaleResolver::new(Arc::new(EnvLocale)));
    let locale: Option<Locale> = language_flag().map(|pref| resolver.resolve(pref).locale);
    gafira_ui::run(gafira_ui::UiConfig {
        locale,
        version: env!("CARGO_PKG_VERSION"),
        recent: Box::new(Recent {
            file: RecentProjects::at(data_dir.join("recent.json")),
            clock: SystemClock::new(),
        }),
        engine: Box::new(move |waker: Arc<dyn Waker>| {
            let clock = Arc::new(SystemClock::new());
            // Spec 026: imported and opened assets live in the content store,
            // and the repository reads and writes their bytes through it.
            let store: Arc<dyn ContentStore> = Arc::new(FsStore::new(store_dir(&data_dir)));
            let ports = Ports::new(
                Arc::new(GafRepository::new(Limits::default(), Arc::clone(&store))),
                store,
                SystemPorts::new(
                    Arc::new(StdFileSystem),
                    clock.clone(),
                    Arc::new(RandomIds),
                    Arc::new(SystemProcesses::new(data_dir.join("instances"))),
                ),
                Arc::new(StderrNotifier),
                // D-62: the first version ships without a licence.
                Arc::new(NoLicensing),
                SettingsPorts::new(
                    Arc::new(JsonSettings::at(settings_file(&data_dir))),
                    resolver,
                ),
                export_pages(clock.clone()),
            );
            Gafira::new(
                ports,
                registries(),
                EngineConfig::new(data_dir)?,
                EventChannel::new(waker, clock),
            )
        }),
    })
}

/// The pages of an export (spec 030): the render graph on the CPU backend,
/// with the fonts the theme embeds.
fn export_pages(clock: Arc<SystemClock>) -> Arc<dyn PageRenderers> {
    let fonts = gafira_theme::fonts()
        .iter()
        .fold(FontBook::new(), |book, font| {
            book.with(font.family, font.bytes)
        });
    Arc::new(GraphPageRenderers::new(
        Box::new(|| Box::new(CpuBackend::new())),
        clock,
        Arc::new(fonts),
    ))
}

/// The formats of this build: PNG and PDF export (spec 030).
fn registries() -> Registries {
    let mut registries = Registries::new();
    let exporters: [Arc<dyn Exporter>; 2] = [
        Arc::new(gafira_io_image::PngExporter::new()),
        Arc::new(gafira_io_pdf::PdfExporter::new()),
    ];
    for exporter in exporters {
        if let Err(error) = registries.formats.register_exporter(exporter) {
            eprintln!("formats: {error}");
        }
    }
    registries
}

fn settings_file(data_dir: &Path) -> PathBuf {
    data_dir.join("settings.json")
}

fn store_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("store")
}
