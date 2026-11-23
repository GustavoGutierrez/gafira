//! The D-08 language rule and the D-64 format region (02 §14.4).

use std::sync::Arc;

use gafira_ports::i18n::{
    FormatLocale, LanguagePreference, Locale, LocaleResolver, LocaleSource, Region, ResolvedLocale,
    SystemLocaleSource,
};
use icu_locale_core::LanguageIdentifier;

/// Resolves a [`LanguagePreference`] with the operating system's tags: any
/// Spanish → `Es`; English and every other language → `En`; no usable tag →
/// `En` with [`LocaleSource::Fallback`] (D-08).
pub struct D08LocaleResolver {
    system: Arc<dyn SystemLocaleSource>,
}

impl D08LocaleResolver {
    #[must_use]
    pub fn new(system: Arc<dyn SystemLocaleSource>) -> Self {
        Self { system }
    }
}

impl LocaleResolver for D08LocaleResolver {
    fn resolve(&self, pref: LanguagePreference) -> ResolvedLocale {
        let sys = self.system.system_locale();
        let (locale, source) = match pref {
            LanguagePreference::Es => (Locale::Es, LocaleSource::Manual),
            LanguagePreference::En => (Locale::En, LocaleSource::Manual),
            LanguagePreference::System => {
                let tag = sys.ui.clone().unwrap_or_default();
                match sys.ui.as_deref().and_then(parse_tag) {
                    Some(id) if id.language.as_str() == "es" => {
                        (Locale::Es, LocaleSource::System { tag })
                    }
                    Some(_) => (Locale::En, LocaleSource::System { tag }),
                    None => (Locale::En, LocaleSource::Fallback),
                }
            }
        };
        ResolvedLocale {
            locale,
            formats: format_locale(locale, sys.formats.as_deref()),
            source,
        }
    }
}

/// Parses an OS tag: `_` becomes `-`, `.codeset` and `@modifier` go, and
/// `C`/`POSIX` mean no language. `es_CO.UTF-8`, `es_ES@euro`, `ES-mx` and
/// `es-419` parse; `C.UTF-8`, `""` and garbage do not.
pub(crate) fn parse_tag(tag: &str) -> Option<LanguageIdentifier> {
    let tag = tag.trim();
    let end = tag.find(['.', '@']).unwrap_or(tag.len());
    let tag = tag[..end].replace('_', "-");
    if tag.is_empty() || tag.eq_ignore_ascii_case("c") || tag.eq_ignore_ascii_case("posix") {
        return None;
    }
    let id = LanguageIdentifier::try_from_str(&tag).ok()?;
    // `und` (no language) is as good as no tag.
    (!id.language.is_unknown()).then_some(id)
}

/// The region of `formats` when its language is the UI language (D-64);
/// otherwise none, so CLDR's default for the language applies.
pub(crate) fn format_locale(locale: Locale, formats: Option<&str>) -> FormatLocale {
    let wanted = match locale {
        Locale::Es => "es",
        Locale::En => "en",
    };
    let region = formats
        .and_then(parse_tag)
        .filter(|id| id.language.as_str() == wanted)
        .and_then(|id| id.region)
        .and_then(|r| Region::new(r.as_str()));
    FormatLocale { locale, region }
}

#[cfg(test)]
mod tests {
    use gafira_ports::i18n::SystemLocale;

    use super::*;

    struct FakeSystem(SystemLocale);

    impl SystemLocaleSource for FakeSystem {
        fn system_locale(&self) -> SystemLocale {
            self.0.clone()
        }
    }

    fn resolver(ui: Option<&str>, formats: Option<&str>) -> D08LocaleResolver {
        D08LocaleResolver::new(Arc::new(FakeSystem(SystemLocale {
            ui: ui.map(str::to_string),
            formats: formats.map(str::to_string),
        })))
    }

    // S15-01 (ARQ-100): every tag of the 02 §14.4 table, and more.
    #[test]
    fn s15_01_d08_rule_on_the_tag_table() {
        let spanish = [
            "es",
            "es-ES",
            "es-MX",
            "es-CO",
            "es-AR",
            "es-419",
            "es_CL.UTF-8",
            "es_ES@euro",
            "ES-mx",
            "es_US",
            " es-PE ",
        ];
        let english_or_other = [
            "en",
            "en-US",
            "en-GB",
            "en-AU",
            "en_IN.UTF-8",
            "fr-FR",
            "pt-BR",
            "de-DE",
            "it",
            "zh-Hans-CN",
            "ja",
            "ca-ES",
            "eu-ES",
            "gl-ES",
            "pt_PT.UTF-8",
            "ast-ES",
        ];
        let fallback = [
            "", "C", "POSIX", "C.UTF-8", "c", "posix", "!!", "und", "12-34",
        ];
        let mut count = 0;
        for tag in spanish {
            let r = resolver(Some(tag), None).resolve(LanguagePreference::System);
            assert_eq!(r.locale, Locale::Es, "{tag:?}");
            assert_eq!(
                r.source,
                LocaleSource::System {
                    tag: tag.to_string()
                },
                "{tag:?}"
            );
            count += 1;
        }
        for tag in english_or_other {
            let r = resolver(Some(tag), None).resolve(LanguagePreference::System);
            assert_eq!(r.locale, Locale::En, "{tag:?}");
            assert_eq!(
                r.source,
                LocaleSource::System {
                    tag: tag.to_string()
                },
                "{tag:?}"
            );
            count += 1;
        }
        for tag in fallback {
            let r = resolver(Some(tag), None).resolve(LanguagePreference::System);
            assert_eq!(
                (r.locale, r.source),
                (Locale::En, LocaleSource::Fallback),
                "{tag:?}"
            );
            count += 1;
        }
        let r = resolver(None, None).resolve(LanguagePreference::System);
        assert_eq!((r.locale, r.source), (Locale::En, LocaleSource::Fallback));
        assert!(
            count >= 30,
            "ARQ-100 asks for at least 30 tags, got {count}"
        );
    }

    #[test]
    fn s15_01_manual_choices_ignore_the_system() {
        for ui in [Some("es-MX"), Some("fr-FR"), None] {
            let r = resolver(ui, None);
            let es = r.resolve(LanguagePreference::Es);
            let en = r.resolve(LanguagePreference::En);
            assert_eq!((es.locale, es.source), (Locale::Es, LocaleSource::Manual));
            assert_eq!((en.locale, en.source), (Locale::En, LocaleSource::Manual));
        }
    }

    // S15-02 (D-64)
    #[test]
    fn s15_02_format_region_only_when_its_language_matches() {
        let mx = Region::new("MX");
        let r = resolver(Some("es-CO"), Some("es_MX.UTF-8")).resolve(LanguagePreference::System);
        assert_eq!(
            r.formats,
            FormatLocale {
                locale: Locale::Es,
                region: mx
            }
        );
        let r = resolver(Some("es-ES"), Some("fr-FR")).resolve(LanguagePreference::System);
        assert_eq!(
            r.formats,
            FormatLocale {
                locale: Locale::Es,
                region: None
            }
        );
        let r = resolver(Some("fr-FR"), Some("en-GB")).resolve(LanguagePreference::En);
        assert_eq!(
            r.formats,
            FormatLocale {
                locale: Locale::En,
                region: Region::new("GB")
            }
        );
        let r = resolver(Some("es-MX"), None).resolve(LanguagePreference::Es);
        assert_eq!(r.formats.region, None);
        let r = resolver(Some("es"), Some("es-419")).resolve(LanguagePreference::System);
        assert_eq!(r.formats.region, Region::new("419"));
        let r = resolver(Some("es"), Some("es")).resolve(LanguagePreference::System);
        assert_eq!(r.formats.region, None);
    }
}
