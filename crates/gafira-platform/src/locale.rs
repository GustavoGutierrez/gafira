//! The system language from the process environment (D-08).

use gafira_ports::i18n::{SystemLocale, SystemLocaleSource};

/// Reads `LC_ALL`, `LC_MESSAGES` and `LANG` for the interface, and
/// `LC_ALL`, `LC_NUMERIC` and `LANG` for formats, as BCP 47 tags
/// (`es_MX.UTF-8` → `es-MX`). Windows and macOS read the same variables
/// until their native calls arrive.
#[derive(Clone, Copy, Debug, Default)]
pub struct EnvLocale;

/// `es_MX.UTF-8@euro` → `es-MX`; `C` and `POSIX` say nothing.
fn tag(value: &str) -> Option<String> {
    let base = value.split(['.', '@']).next().unwrap_or_default();
    if base.is_empty() || base == "C" || base == "POSIX" {
        return None;
    }
    Some(base.replace('_', "-"))
}

fn first(names: &[&str]) -> Option<String> {
    names
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .find_map(|value| tag(&value))
}

impl SystemLocaleSource for EnvLocale {
    fn system_locale(&self) -> SystemLocale {
        SystemLocale {
            ui: first(&["LC_ALL", "LC_MESSAGES", "LANG"]),
            formats: first(&["LC_ALL", "LC_NUMERIC", "LANG"]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s18_04_posix_values_become_tags() {
        assert_eq!(tag("es_MX.UTF-8").as_deref(), Some("es-MX"));
        assert_eq!(tag("en_US").as_deref(), Some("en-US"));
        assert_eq!(tag("de_DE@euro").as_deref(), Some("de-DE"));
        assert_eq!(tag("es").as_deref(), Some("es"));
        assert_eq!(tag("C"), None);
        assert_eq!(tag("POSIX"), None);
        assert_eq!(tag(""), None);
    }
}
