//! Language ports and types (02 §4.16).

use std::borrow::Cow;
use std::time::Duration;

use gafira_doc::msg::{Message, MessageKey, Unit};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::common::UnixTime;

/// A published UI language (D-08). Serialized as `"es"` or `"en"`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    Es,
    En,
}

/// The language choice in Settings (02 §14.4).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LanguagePreference {
    #[default]
    System,
    Es,
    En,
}

impl LanguagePreference {
    #[must_use]
    pub fn parse(text: &str) -> Self {
        match text {
            "es" => Self::Es,
            "en" => Self::En,
            _ => Self::System,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Es => "es",
            Self::En => "en",
        }
    }
}

impl Serialize for LanguagePreference {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for LanguagePreference {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        Ok(Self::parse(&text))
    }
}

/// A region subtag: 2 ASCII letters or 3 ASCII digits.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Region {
    len: u8,
    bytes: [u8; 3],
}

impl Region {
    #[must_use]
    pub fn new(text: &str) -> Option<Self> {
        let raw = text.as_bytes();
        let mut bytes = [0u8; 3];
        match raw.len() {
            2 if raw.iter().all(u8::is_ascii_alphabetic) => {
                bytes[..2].copy_from_slice(raw);
                bytes.make_ascii_uppercase();
                Some(Self { len: 2, bytes })
            }
            3 if raw.iter().all(u8::is_ascii_digit) => {
                bytes.copy_from_slice(raw);
                Some(Self { len: 3, bytes })
            }
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        // The bytes are ASCII by construction; the fallback is unreachable.
        std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or_default()
    }
}

/// Number and date conventions of a locale.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FormatLocale {
    pub locale: Locale,
    pub region: Option<Region>,
}

/// Locale tags as the operating system gives them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SystemLocale {
    pub ui: Option<String>,
    pub formats: Option<String>,
}

/// Where a resolved locale came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocaleSource {
    Manual,
    System { tag: String },
    Fallback,
}

/// The outcome of resolving a [`LanguagePreference`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedLocale {
    pub locale: Locale,
    pub formats: FormatLocale,
    pub source: LocaleSource,
}

/// How a number is shown.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum NumberStyle {
    Decimal,
    Percent,
    Tabular,
}

/// How a date is shown.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DateStyle {
    Short,
    Long,
}

/// Reads the operating system's locale tags (`gafira-platform`).
pub trait SystemLocaleSource: Send + Sync {
    fn system_locale(&self) -> SystemLocale;
}

/// Maps a preference to a locale (`gafira-i18n`).
pub trait LocaleResolver: Send + Sync {
    fn resolve(&self, pref: LanguagePreference) -> ResolvedLocale;
}

/// Turns messages and values into text (`gafira-i18n`).
pub trait Translator: Send + Sync {
    fn text(&self, locale: Locale, msg: &Message) -> Cow<'_, str>;
    fn has(&self, locale: Locale, key: MessageKey) -> bool;
    fn number(&self, f: &FormatLocale, v: f64, style: NumberStyle) -> String;
    fn date(&self, f: &FormatLocale, t: UnixTime, style: DateStyle) -> String;
    fn relative(&self, f: &FormatLocale, delta: Duration) -> String;
    fn quantity(&self, f: &FormatLocale, v: f64, unit: Unit) -> String;
}
