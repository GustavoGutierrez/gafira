#![forbid(unsafe_code)]

//! `gafira`: the desktop app (02 §2.1). Composition only: it builds the
//! ports from the adapters, the engine over them, and hands both to the
//! shell (spec 018).

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gafira_doc::{ContentHash, Document};
use gafira_engine::{
    EngineConfig, EventChannel, Gafira, Ports, Registries, SettingsPorts, SystemPorts, Waker,
};
use gafira_i18n::D08LocaleResolver;
use gafira_platform::{
    EnvLocale, JsonSettings, RandomIds, StdFileSystem, StderrNotifier, SystemClock,
};
use gafira_ports::i18n::{LanguagePreference, Locale, LocaleResolver};
use gafira_ports::license::NoLicensing;
use gafira_ports::repository::{LoadedProject, ProjectRepository, RepositoryError};
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::ReadSeek;

/// The `.gaf` reader and writer arrive with `gafira-format` (spec 008,
/// D-68); until then opening and saving say so instead of pretending.
struct FormatPending;

fn pending() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "the .gaf format is not in this build yet",
    )
}

impl ProjectRepository for FormatPending {
    fn load(&self, _path: &Path) -> Result<LoadedProject, RepositoryError> {
        Err(RepositoryError::Io(pending()))
    }

    fn save(&self, _path: &Path, _document: &Document) -> Result<(), RepositoryError> {
        Err(RepositoryError::Io(pending()))
    }
}

/// The content store arrives with `gafira-assets`; imports fail until then.
struct StorePending;

impl ContentStore for StorePending {
    fn put(&self, _src: &mut dyn io::Read) -> Result<ContentHash, StoreError> {
        Err(StoreError::Io(pending()))
    }

    fn open(&self, _hash: ContentHash) -> Result<Box<dyn ReadSeek + Send>, StoreError> {
        Err(StoreError::Io(pending()))
    }

    fn contains(&self, _hash: ContentHash) -> Result<bool, StoreError> {
        Ok(false)
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
        engine: Box::new(move |waker: Arc<dyn Waker>| {
            let clock = Arc::new(SystemClock::new());
            let ports = Ports::new(
                Arc::new(FormatPending),
                Arc::new(StorePending),
                SystemPorts::new(Arc::new(StdFileSystem), clock.clone(), Arc::new(RandomIds)),
                Arc::new(StderrNotifier),
                // D-62: the first version ships without a licence.
                Arc::new(NoLicensing),
                SettingsPorts::new(
                    Arc::new(JsonSettings::at(settings_file(&data_dir))),
                    resolver,
                ),
            );
            Gafira::new(
                ports,
                Registries::new(),
                EngineConfig::new(data_dir)?,
                EventChannel::new(waker, clock),
            )
        }),
    })
}

fn settings_file(data_dir: &Path) -> PathBuf {
    data_dir.join("settings.json")
}
