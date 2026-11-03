//! Language types and ports (design P4, P8, P9; 02 §4.16, §14.4).

use std::borrow::Cow;
use std::time::Duration;

use gafira_doc::msg::{Message, MessageKey, Unit};
use gafira_ports::common::UnixTime;
use gafira_ports::i18n::{
    DateStyle, FormatLocale, LanguagePreference, Locale, LocaleResolver, LocaleSource, NumberStyle,
    Region, ResolvedLocale, SystemLocale, SystemLocaleSource, Translator,
};

// P8 `LanguagePreference::parse` (D-08, 02 §14.4).
#[test]
fn p8_preference_parses_exact_values_and_defaults_to_system() {
    let table = [
        ("system", LanguagePreference::System),
        ("es", LanguagePreference::Es),
        ("en", LanguagePreference::En),
        ("ES", LanguagePreference::System),
        ("En", LanguagePreference::System),
        ("System", LanguagePreference::System),
        (" es", LanguagePreference::System),
        ("es-MX", LanguagePreference::System),
        ("fr", LanguagePreference::System),
        ("", LanguagePreference::System),
    ];
    for (text, want) in table {
        assert_eq!(LanguagePreference::parse(text), want, "{text:?}");
    }
    assert_eq!(LanguagePreference::default(), LanguagePreference::System);
}

#[test]
fn p8_preference_as_str_round_trips_through_parse() {
    for (pref, text) in [
        (LanguagePreference::System, "system"),
        (LanguagePreference::Es, "es"),
        (LanguagePreference::En, "en"),
    ] {
        assert_eq!(pref.as_str(), text);
        assert_eq!(LanguagePreference::parse(text), pref);
    }
}

#[test]
fn p8_preference_serde_is_the_lowercase_string() {
    for (pref, json) in [
        (LanguagePreference::System, "\"system\""),
        (LanguagePreference::Es, "\"es\""),
        (LanguagePreference::En, "\"en\""),
    ] {
        assert_eq!(serde_json::to_string(&pref).unwrap(), json);
        assert_eq!(
            serde_json::from_str::<LanguagePreference>(json).unwrap(),
            pref
        );
    }
    // An unknown text reads as System, never an error (S11).
    for json in ["\"ES\"", "\"fr\"", "\"\""] {
        assert_eq!(
            serde_json::from_str::<LanguagePreference>(json).unwrap(),
            LanguagePreference::System,
            "{json}"
        );
    }
    // A non-string value is a deserialize error (S11).
    for json in ["1", "null", "true", "[]", "{}"] {
        assert!(
            serde_json::from_str::<LanguagePreference>(json).is_err(),
            "{json}"
        );
    }
}

#[test]
fn p8_locale_serde_is_es_and_en() {
    assert_eq!(serde_json::to_string(&Locale::Es).unwrap(), "\"es\"");
    assert_eq!(serde_json::to_string(&Locale::En).unwrap(), "\"en\"");
    assert_eq!(
        serde_json::from_str::<Locale>("\"es\"").unwrap(),
        Locale::Es
    );
    assert_eq!(
        serde_json::from_str::<Locale>("\"en\"").unwrap(),
        Locale::En
    );
    assert!(serde_json::from_str::<Locale>("\"fr\"").is_err());
}

// P8 `Region::new`, acceptance of 02 §4.16.
#[test]
fn p8_region_accepts_two_letters_or_three_digits() {
    for (input, stored) in [
        ("mx", "MX"),
        ("MX", "MX"),
        ("Mx", "MX"),
        ("419", "419"),
        ("us", "US"),
    ] {
        let region = Region::new(input).unwrap_or_else(|| panic!("{input:?} should be valid"));
        assert_eq!(region.as_str(), stored, "{input:?}");
    }
    for input in [
        "", "m", "mex", "41", "4190", "m1", "1m", "ñx", "m x", "4 9", "٤١٩", "mx ",
    ] {
        assert_eq!(Region::new(input), None, "{input:?}");
    }
}

#[test]
fn p8_region_equality_ignores_input_case() {
    assert_eq!(Region::new("mx"), Region::new("MX"));
    assert_ne!(Region::new("mx"), Region::new("us"));
    assert_ne!(Region::new("419"), Region::new("41").or(Region::new("420")));
}

// ARQ-100 (types): a resolver double maps tags through the port types.
struct FakeSource(SystemLocale);

impl SystemLocaleSource for FakeSource {
    fn system_locale(&self) -> SystemLocale {
        self.0.clone()
    }
}

struct Resolver<'a>(&'a dyn SystemLocaleSource);

impl LocaleResolver for Resolver<'_> {
    fn resolve(&self, pref: LanguagePreference) -> ResolvedLocale {
        let manual = |locale| ResolvedLocale {
            locale,
            formats: FormatLocale {
                locale,
                region: None,
            },
            source: LocaleSource::Manual,
        };
        match pref {
            LanguagePreference::Es => manual(Locale::Es),
            LanguagePreference::En => manual(Locale::En),
            LanguagePreference::System => match self.0.system_locale().ui {
                Some(tag) if tag.starts_with("es") => ResolvedLocale {
                    locale: Locale::Es,
                    formats: FormatLocale {
                        locale: Locale::Es,
                        region: Region::new("419"),
                    },
                    source: LocaleSource::System { tag },
                },
                _ => ResolvedLocale {
                    locale: Locale::En,
                    formats: FormatLocale {
                        locale: Locale::En,
                        region: None,
                    },
                    source: LocaleSource::Fallback,
                },
            },
        }
    }
}

#[test]
fn arq_100_resolver_double_maps_tags_through_dyn_ports() {
    let source: Box<dyn SystemLocaleSource> = Box::new(FakeSource(SystemLocale {
        ui: Some("es-419".to_owned()),
        formats: None,
    }));
    let resolver: Box<dyn LocaleResolver + '_> = Box::new(Resolver(&*source));
    let system = resolver.resolve(LanguagePreference::System);
    assert_eq!(system.locale, Locale::Es);
    assert_eq!(
        system.formats.region.map(|r| r.as_str().to_owned()),
        Some("419".to_owned())
    );
    assert_eq!(
        system.source,
        LocaleSource::System {
            tag: "es-419".to_owned()
        }
    );
    // A manual choice ignores the system.
    assert_eq!(resolver.resolve(LanguagePreference::En).locale, Locale::En);
    assert_eq!(
        resolver.resolve(LanguagePreference::En).source,
        LocaleSource::Manual
    );
    let none = Resolver(&FakeSource(SystemLocale::default())).resolve(LanguagePreference::System);
    assert_eq!(none.source, LocaleSource::Fallback);
    assert_eq!(none.locale, Locale::En);
}

// ARQ-20, ARQ-31: `Translator` is object-safe and takes the locale per call.
struct Echo;

impl Translator for Echo {
    fn text(&self, _: Locale, msg: &Message) -> Cow<'_, str> {
        Cow::Owned(msg.key.0.to_owned())
    }
    fn has(&self, locale: Locale, key: MessageKey) -> bool {
        locale == Locale::Es && key.0 == "a.b"
    }
    fn number(&self, _: &FormatLocale, v: f64, _: NumberStyle) -> String {
        v.to_string()
    }
    fn date(&self, _: &FormatLocale, t: UnixTime, _: DateStyle) -> String {
        t.0.to_string()
    }
    fn relative(&self, _: &FormatLocale, delta: Duration) -> String {
        delta.as_secs().to_string()
    }
    fn quantity(&self, _: &FormatLocale, v: f64, _: Unit) -> String {
        v.to_string()
    }
}

#[test]
fn arq_20_translator_is_object_safe() {
    let t: Box<dyn Translator> = Box::new(Echo);
    let f = FormatLocale {
        locale: Locale::En,
        region: None,
    };
    assert!(t.has(Locale::Es, MessageKey("a.b")));
    assert!(!t.has(Locale::En, MessageKey("a.b")));
    assert_eq!(t.number(&f, 1.5, NumberStyle::Decimal), "1.5");
    assert_eq!(t.date(&f, UnixTime(7), DateStyle::Short), "7");
    assert_eq!(t.relative(&f, Duration::from_secs(3)), "3");
    assert_eq!(t.quantity(&f, 2.0, Unit::Mm), "2");
    let m = Message {
        key: MessageKey("a.b"),
        args: vec![],
    };
    assert_eq!(t.text(Locale::Es, &m), "a.b");
}
