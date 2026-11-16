#![forbid(unsafe_code)]

//! `gafira`: the desktop app (02 §2.1). Composition only: it resolves the
//! interface language and hands it to the shell.

use std::sync::Arc;

use gafira_i18n::D08LocaleResolver;
use gafira_ports::i18n::{LanguagePreference, LocaleResolver, SystemLocale, SystemLocaleSource};

/// The language of the process environment (`LC_ALL`, `LC_MESSAGES`, `LANG`),
/// until `gafira-platform` reads it from each OS.
struct EnvLocale;

impl SystemLocaleSource for EnvLocale {
    fn system_locale(&self) -> SystemLocale {
        let read = |names: &[&str]| {
            names
                .iter()
                .find_map(|name| std::env::var(name).ok().filter(|v| !v.is_empty()))
                .map(|v| v.split('.').next().unwrap_or_default().replace('_', "-"))
        };
        SystemLocale {
            ui: read(&["LC_ALL", "LC_MESSAGES", "LANG"]),
            formats: read(&["LC_ALL", "LC_NUMERIC", "LANG"]),
        }
    }
}

/// `--lang es|en` overrides the system language, as Ajustes will (D-08).
fn preference() -> LanguagePreference {
    let args: Vec<String> = std::env::args().collect();
    match args
        .windows(2)
        .find(|w| w[0] == "--lang")
        .map(|w| w[1].as_str())
    {
        Some("es") => LanguagePreference::Es,
        Some("en") => LanguagePreference::En,
        _ => LanguagePreference::System,
    }
}

fn main() -> eframe::Result {
    let resolver = D08LocaleResolver::new(Arc::new(EnvLocale));
    let resolved = resolver.resolve(preference());
    gafira_ui::run(gafira_ui::UiConfig {
        locale: resolved.locale,
        version: env!("CARGO_PKG_VERSION"),
    })
}
