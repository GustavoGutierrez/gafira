//! `CatalogTranslator`: messages by key with parameters and plurals, the
//! fallback chain of 02 §14.6 and locale-aware values (02 §14.5).

use std::borrow::Cow;
use std::time::Duration;

use gafira_doc::msg::{Message, MessageKey, MsgArg, Unit};
use gafira_ports::common::UnixTime;
use gafira_ports::i18n::{DateStyle, FormatLocale, Locale, NumberStyle, Translator};
use icu_plurals::{PluralCategory, PluralRules};

use crate::catalog::{Catalog, Entry};
use crate::format;

/// What the chain shows when no language has the key (02 §14.6, step 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fallback {
    /// `⟦key⟧`: development builds and the `pseudo` feature.
    Development,
    /// The other published language, then `…`: the `dist` configuration.
    Distribution,
}

impl Fallback {
    const BUILD: Self = if cfg!(any(debug_assertions, feature = "pseudo")) {
        Self::Development
    } else {
        Self::Distribution
    };
}

/// The neutral text of a key that no catalog has, in `dist`.
pub(crate) const MISSING: &str = "…";

/// Translates messages with the embedded Spanish and English catalogs.
#[derive(Clone, Debug)]
pub struct CatalogTranslator {
    es: Catalog,
    en: Catalog,
    fallback: Fallback,
}

impl Default for CatalogTranslator {
    fn default() -> Self {
        Self::new()
    }
}

impl CatalogTranslator {
    /// The embedded catalogs, with the fallback of this build.
    #[must_use]
    pub fn new() -> Self {
        Self::with(
            Catalog::embedded(Locale::Es),
            Catalog::embedded(Locale::En),
            Fallback::BUILD,
        )
    }

    pub(crate) const fn with(es: Catalog, en: Catalog, fallback: Fallback) -> Self {
        Self { es, en, fallback }
    }

    const fn catalog(&self, locale: Locale) -> &Catalog {
        match locale {
            Locale::Es => &self.es,
            Locale::En => &self.en,
        }
    }

    /// Requested language, then English, then (in `dist`) the other language.
    fn lookup(&self, locale: Locale, key: &str) -> Option<(Locale, &'static Entry)> {
        let other = match locale {
            Locale::Es => Locale::En,
            Locale::En => Locale::Es,
        };
        let mut chain = vec![locale, Locale::En];
        if self.fallback == Fallback::Distribution {
            chain.push(other);
        }
        chain
            .into_iter()
            .find_map(|l| self.catalog(l).get(key).map(|e| (l, e)))
    }

    fn missing(&self, key: &str) -> String {
        match self.fallback {
            Fallback::Development => format!("⟦{key}⟧"),
            Fallback::Distribution => MISSING.to_string(),
        }
    }

    /// The template of `msg` in the chain, with its plural form chosen by the
    /// first `Count` argument; `Err` holds the text of a missing key.
    pub(crate) fn template(&self, locale: Locale, msg: &Message) -> Result<&'static str, String> {
        let Some((found, entry)) = self.lookup(locale, msg.key.0) else {
            return Err(self.missing(msg.key.0));
        };
        Ok(match entry {
            Entry::Text(t) => t,
            Entry::Plural(p) => {
                let count = msg.args.iter().find_map(|(_, a)| match a {
                    MsgArg::Count(n) => Some(*n),
                    _ => None,
                });
                count.map_or(p.other, |n| p.form(plural_category(found, n)))
            }
        })
    }

    /// One catalog value used as a pattern (`unit.px`, `format.percent`).
    fn pattern(&self, locale: Locale, key: &'static str) -> String {
        match self.template(
            locale,
            &Message {
                key: MessageKey(key),
                args: Vec::new(),
            },
        ) {
            Ok(t) => t.to_string(),
            Err(missing) => missing,
        }
    }

    /// Replaces every `{name}` that has an argument; others stay literal.
    pub(crate) fn render(
        &self,
        f: &FormatLocale,
        template: &str,
        args: &[(&'static str, MsgArg)],
    ) -> String {
        let mut out = String::with_capacity(template.len());
        let mut rest = template;
        while let Some(start) = rest.find('{') {
            out.push_str(&rest[..start]);
            let after = &rest[start + 1..];
            let Some(end) = after.find('}') else {
                rest = &rest[start..];
                break;
            };
            let name = &after[..end];
            match args.iter().find(|(n, _)| *n == name) {
                Some((_, arg)) => out.push_str(&self.argument(f, arg)),
                None => {
                    out.push('{');
                    out.push_str(name);
                    out.push('}');
                }
            }
            rest = &after[end + 1..];
        }
        out.push_str(rest);
        out
    }

    fn argument(&self, f: &FormatLocale, arg: &MsgArg) -> String {
        match arg {
            MsgArg::Text(t) => t.clone(),
            MsgArg::Count(n) => format::format_decimal(f, &(*n).into()),
            MsgArg::Int(n) => format::format_decimal(f, &(*n).into()),
            MsgArg::Decimal(v) => self.number(f, *v, NumberStyle::Decimal),
            MsgArg::Quantity(v, unit) => self.quantity(f, *v, *unit),
            MsgArg::Timestamp(t) => self.date(f, UnixTime(*t), DateStyle::Short),
            MsgArg::Elapsed(s) => {
                self.relative(f, Duration::from_secs(u64::try_from(*s).unwrap_or(0)))
            }
            // One level: the named message's own text, without its parameters.
            MsgArg::Key(key) => match self.template(
                f.locale,
                &Message {
                    key: *key,
                    args: Vec::new(),
                },
            ) {
                Ok(t) => t.to_string(),
                Err(missing) => missing,
            },
        }
    }

    fn plural_text(&self, f: &FormatLocale, key: &'static str, count: u64) -> String {
        let msg = Message {
            key: MessageKey(key),
            args: vec![("count", MsgArg::Count(count))],
        };
        match self.template(f.locale, &msg) {
            Ok(t) => self.render(f, t, &msg.args),
            Err(missing) => missing,
        }
    }
}

/// The CLDR cardinal category of `n` in `locale`.
fn plural_category(locale: Locale, n: u64) -> PluralCategory {
    let icu = format::icu_locale(&FormatLocale {
        locale,
        region: None,
    });
    PluralRules::try_new_cardinal((&icu).into())
        .map_or(PluralCategory::Other, |r| r.category_for(n))
}

const fn unit_key(unit: Unit) -> &'static str {
    match unit {
        Unit::Px => "unit.px",
        Unit::Mm => "unit.mm",
        Unit::Cm => "unit.cm",
        Unit::In => "unit.in",
        Unit::Pt => "unit.pt",
        // Percent is formatted as `NumberStyle::Percent`; this arm is not used.
        Unit::Percent => "format.percent",
        Unit::Bytes => "unit.bytes",
        Unit::Seconds => "unit.seconds",
        Unit::Fps => "unit.fps",
        Unit::Db => "unit.db",
        Unit::Lufs => "unit.lufs",
        Unit::Degrees => "unit.degrees",
    }
}

/// Byte prefixes in powers of 1000 (PROPUESTA, spec 015).
const BYTE_UNITS: [&str; 4] = [
    "unit.bytes",
    "unit.kilobytes",
    "unit.megabytes",
    "unit.gigabytes",
];

impl Translator for CatalogTranslator {
    fn text(&self, locale: Locale, msg: &Message) -> Cow<'_, str> {
        match self.template(locale, msg) {
            Ok(t) if !t.contains('{') => Cow::Borrowed(t),
            Ok(t) => {
                let f = FormatLocale {
                    locale,
                    region: None,
                };
                Cow::Owned(self.render(&f, t, &msg.args))
            }
            Err(missing) => Cow::Owned(missing),
        }
    }

    fn has(&self, locale: Locale, key: MessageKey) -> bool {
        self.catalog(locale).get(key.0).is_some()
    }

    fn number(&self, f: &FormatLocale, v: f64, style: NumberStyle) -> String {
        match style {
            NumberStyle::Decimal => format::format_number(f, v, 2, false),
            NumberStyle::Tabular => format::format_number(f, v, 2, true),
            NumberStyle::Percent => {
                let value = format::format_number(f, v, 1, false);
                self.pattern(f.locale, "format.percent")
                    .replace("{value}", &value)
            }
        }
    }

    fn date(&self, _f: &FormatLocale, t: UnixTime, style: DateStyle) -> String {
        format::iso_date(t.0, style)
    }

    fn relative(&self, f: &FormatLocale, delta: Duration) -> String {
        let secs = delta.as_secs();
        match secs {
            0..60 => self.pattern(f.locale, "time.just_now"),
            60..3_600 => self.plural_text(f, "time.minutes_ago", secs / 60),
            3_600..86_400 => self.plural_text(f, "time.hours_ago", secs / 3_600),
            _ => self.plural_text(f, "time.days_ago", secs / 86_400),
        }
    }

    fn quantity(&self, f: &FormatLocale, v: f64, unit: Unit) -> String {
        match unit {
            Unit::Percent => self.number(f, v, NumberStyle::Percent),
            Unit::Bytes => {
                let mut scaled = v;
                let mut step = 0;
                while scaled.abs() >= 1000.0 && step + 1 < BYTE_UNITS.len() {
                    scaled /= 1000.0;
                    step += 1;
                }
                let digits = if step == 0 { 0 } else { 1 };
                let value = format::format_number(f, scaled, digits, false);
                self.pattern(f.locale, BYTE_UNITS[step])
                    .replace("{value}", &value)
            }
            _ => {
                let value = format::format_number(f, v, 2, false);
                self.pattern(f.locale, unit_key(unit))
                    .replace("{value}", &value)
            }
        }
    }
}

#[cfg(test)]
mod tests;
