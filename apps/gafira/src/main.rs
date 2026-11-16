#![forbid(unsafe_code)]

//! `gafira`: the desktop app (02 §2.1). Composition only: it builds the
//! ports from the adapters, the engine over them, and hands both to the
//! shell (spec 018).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use gafira_assets::FsStore;
use gafira_engine::{
    EngineConfig, EventChannel, Gafira, Ports, Registries, SettingsPorts, SystemPorts, Waker,
};
use gafira_format::{GafRepository, Limits};
use gafira_i18n::D08LocaleResolver;
use gafira_platform::{
    EnvLocale, JsonSettings, RandomIds, RecentProjects, StdFileSystem, StderrNotifier, SystemClock,
    SystemProcesses,
};
use gafira_ports::i18n::{LanguagePreference, Locale, LocaleResolver};
use gafira_ports::license::NoLicensing;
use gafira_ports::store::ContentStore;
use gafira_ports::system::Clock;

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

fn store_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("store")
}
