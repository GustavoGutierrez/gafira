#![forbid(unsafe_code)]
// No console behind the window of a release (spec 090).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

//! `gafira`: the desktop app (02 §2.1). Composition only: it builds the
//! ports from the adapters, the engine over them, and hands both to the
//! shell (spec 018).

mod brand;
mod templates;

use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gafira_assets::FsStore;
use gafira_engine::{
    EngineConfig, EventChannel, ExportPorts, Gafira, Ports, Registries, SettingsPorts, SystemPorts,
    Waker,
};
use gafira_format::{GafRepository, Limits, read_thumbnail};
use gafira_i18n::D08LocaleResolver;
use gafira_platform::{
    EnvLocale, ImportedFiles, JsonSettings, RandomIds, RecentProjects, StdFileSystem,
    StderrNotifier, SystemClock, SystemProcesses,
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

/// The export settings kept per format, in `<data_dir>/export-presets.json`
/// (spec 074): `{"pdf": {"compress": true, "scale": 2.0}}`.
struct Presets {
    file: std::path::PathBuf,
}

impl Presets {
    fn read(&self) -> serde_json::Map<String, serde_json::Value> {
        std::fs::read(&self.file)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }
}

impl gafira_ui::ExportPresets for Presets {
    fn load(&self, format: &str) -> Option<Vec<(String, gafira_ui::PresetValue)>> {
        use gafira_ui::PresetValue;
        let all = self.read();
        let kept = all.get(format)?.as_object()?;
        Some(
            kept.iter()
                .filter_map(|(id, value)| {
                    let value = match value {
                        serde_json::Value::Bool(v) => PresetValue::Bool(*v),
                        serde_json::Value::String(v) => PresetValue::Choice(v.clone()),
                        serde_json::Value::Number(n) if n.is_f64() => {
                            PresetValue::Float(n.as_f64()?)
                        }
                        serde_json::Value::Number(n) => PresetValue::Int(n.as_i64()?),
                        _ => return None,
                    };
                    Some((id.clone(), value))
                })
                .collect(),
        )
    }

    fn save(&self, format: &str, values: Option<Vec<(String, gafira_ui::PresetValue)>>) {
        use gafira_ui::PresetValue;
        let mut all = self.read();
        match values {
            Some(values) => {
                let kept: serde_json::Map<String, serde_json::Value> = values
                    .into_iter()
                    .filter_map(|(id, value)| {
                        let value = match value {
                            PresetValue::Bool(v) => serde_json::Value::Bool(v),
                            PresetValue::Choice(v) => serde_json::Value::String(v),
                            PresetValue::Float(v) => {
                                serde_json::Value::Number(serde_json::Number::from_f64(v)?)
                            }
                            PresetValue::Int(v) => serde_json::Value::Number(v.into()),
                        };
                        Some((id, value))
                    })
                    .collect();
                all.insert(format.to_owned(), serde_json::Value::Object(kept));
            }
            None => {
                all.remove(format);
            }
        }
        let written = serde_json::to_vec_pretty(&all)
            .map_err(std::io::Error::other)
            .and_then(|bytes| {
                if let Some(folder) = self.file.parent() {
                    std::fs::create_dir_all(folder)?;
                }
                std::fs::write(&self.file, bytes)
            });
        if let Err(error) = written {
            eprintln!("export presets: {error}");
        }
    }
}

/// Biblioteca over `<data_dir>/library.json` (spec 068).
struct Library {
    file: ImportedFiles,
}

impl gafira_ui::LibraryStore for Library {
    fn existing(&self) -> Vec<gafira_ui::LibraryEntry> {
        let list = self.file.existing().unwrap_or_else(|error| {
            eprintln!("library: {error}");
            self.file.load()
        });
        list.into_iter()
            .map(|entry| gafira_ui::LibraryEntry {
                path: entry.path,
                design: entry.design,
                added: entry.added,
            })
            .collect()
    }

    fn record(&self, file: &std::path::Path, design: bool) {
        if let Err(error) = self.file.record(file, design) {
            eprintln!("library: {error}");
        }
    }

    fn remove(&self, entries: &[(std::path::PathBuf, bool)]) {
        if let Err(error) = self.file.remove(entries) {
            eprintln!("library: {error}");
        }
    }
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

/// The project file the system passed (a double click on a `.gaf`, spec
/// 078): the first argument that is not a flag or a flag's value.
fn project_argument() -> Option<std::path::PathBuf> {
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--lang" {
            args.next();
            continue;
        }
        if arg.to_string_lossy().starts_with("--") {
            continue;
        }
        let path = std::path::PathBuf::from(arg);
        return path.is_file().then_some(path);
    }
    None
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
    // What a package is checked with (spec 090).
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("gafira {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let data_dir =
        gafira_platform::data_dir().unwrap_or_else(|| std::env::temp_dir().join("gafira"));
    let resolver = Arc::new(D08LocaleResolver::new(Arc::new(EnvLocale)));
    let locale: Option<Locale> = language_flag().map(|pref| resolver.resolve(pref).locale);
    // Spec 062: the fonts installed on the system, looked for in the
    // background so the window opens at once; the export reads a face of
    // theirs only when a text uses it.
    let system_fonts = gafira_ui::SystemFonts::new();
    let scanning = system_fonts.clone();
    std::thread::spawn(move || {
        let faces = gafira_platform::system_fonts()
            .into_iter()
            .map(|face| gafira_ui::SystemFont {
                family: face.family,
                bold: face.bold,
                italic: face.italic,
                path: face.path,
            })
            .collect();
        scanning.set(faces);
    });
    let for_export = system_fonts.clone();
    let brand_fonts = system_fonts.clone();
    let catalog = Arc::new(gafira_photos::Catalog::new(Box::new(
        gafira_photos::online::UreqHttp::new(env!("CARGO_PKG_VERSION")),
    )));
    gafira_ui::run(gafira_ui::UiConfig {
        open: project_argument(),
        start_monitor: gafira_platform::pointer_monitor().map(|m| (m.x, m.y, m.width, m.height)),
        locale,
        version: env!("CARGO_PKG_VERSION"),
        recent: Box::new(Recent {
            file: RecentProjects::at(data_dir.join("recent.json")),
            clock: SystemClock::new(),
        }),
        library: Box::new(Library {
            file: ImportedFiles::at(data_dir.join("library.json")),
        }),
        export_presets: Box::new(Presets {
            file: data_dir.join("export-presets.json"),
        }),
        // Spec 039: «Mostrar en carpeta» opens the system file manager.
        show_in_folder: Box::new(gafira_platform::show_in_folder),
        system_fonts,
        // Spec 093: the brand kits, in the data folder.
        brand: Box::new(brand::Brand::new(data_dir.join("brand"), brand_fonts)),
        // Spec 091: the templates the app carries, written out once.
        templates: Some(Arc::new(templates::Templates {
            dir: data_dir.join("templates").join("bundled"),
        })),
        // D-111: Openverse and Wikimedia Commons, and the illustrations of
        // Iconify (spec 072), without an API key.
        photos: Some(Arc::clone(&catalog) as Arc<dyn gafira_ports::photos::PhotoCatalog>),
        illustrations: Some(catalog as Arc<dyn gafira_ports::photos::IllustrationCatalog>),
        // D-115: the image models, the light ones inside and the others
        // downloaded when the person asks.
        vision: Some(Arc::new(gafira_vision::Vision::new(
            data_dir.join("models"),
            Box::new(gafira_vision::online::UreqFetch::new(env!(
                "CARGO_PKG_VERSION"
            ))),
            gafira_vision::detect(),
        ))),
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
                // Spec 032: the image assets an export draws decode in io-image.
                ExportPorts::new(
                    export_pages(clock.clone(), for_export.clone()),
                    Arc::new(gafira_io_image::RasterDecoder::new()),
                ),
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
fn export_pages(clock: Arc<SystemClock>, system: gafira_ui::SystemFonts) -> Arc<dyn PageRenderers> {
    // The interface fonts, then every face of the library under its name
    // (spec 062): a face of the library replaces the interface subset of
    // the same family.
    let fonts = gafira_theme::fonts()
        .iter()
        .fold(FontBook::new(), |book, font| {
            book.with(font.family, font.bytes)
        });
    let fonts = gafira_theme::library().iter().fold(fonts, |book, face| {
        book.with(
            &gafira_theme::face_key(face.family, face.bold, face.italic),
            face.bytes,
        )
    });
    // A face of the system is read the first time a text asks for it.
    let fonts = fonts.with_loader(Arc::new(move |key: &str| {
        let bytes = std::fs::read(system.path(key)?).ok()?;
        Some(Arc::from(bytes))
    }));
    Arc::new(GraphPageRenderers::new(
        Box::new(|| Box::new(CpuBackend::new())),
        clock,
        Arc::new(fonts),
    ))
}

/// The formats of this build: PNG and PDF export (spec 030), JPEG and
/// WebP (spec 039), in the order the export dialog lists them.
fn registries() -> Registries {
    let mut registries = Registries::new();
    let exporters: [Arc<dyn Exporter>; 8] = [
        Arc::new(gafira_io_image::PngExporter::new()),
        Arc::new(gafira_io_image::JpegExporter::new()),
        Arc::new(gafira_io_image::WebpExporter::new()),
        // Spec 087: AVIF.
        Arc::new(gafira_io_image::AvifExporter::new()),
        Arc::new(gafira_io_pdf::PdfExporter::new()),
        // Spec 074: SVG, vectors and real text.
        Arc::new(gafira_io_svg::SvgExporter::new()),
        // Spec 075: PowerPoint, editable.
        Arc::new(gafira_io_pptx::PptxExporter::new()),
        // Spec 081: the tables, for Excel.
        Arc::new(gafira_io_xlsx::XlsxExporter::new()),
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
