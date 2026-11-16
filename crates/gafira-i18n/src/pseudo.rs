//! `PseudoTranslator` (02 §14.6, ARQ-103): accented, bracketed and expanded
//! English that reveals literal text, concatenations and truncation. Behind the
//! `pseudo` feature: development and tests only, never in `dist`.

use std::borrow::Cow;
use std::time::Duration;

use gafira_doc::msg::{Message, MessageKey, Unit};
use gafira_ports::common::UnixTime;
use gafira_ports::i18n::{DateStyle, FormatLocale, Locale, NumberStyle, Translator};

use crate::translator::CatalogTranslator;

/// Shows every message as pseudo-localized English; values as `CatalogTranslator`.
#[derive(Clone, Debug, Default)]
pub struct PseudoTranslator {
    inner: CatalogTranslator,
}

impl PseudoTranslator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Translator for PseudoTranslator {
    fn text(&self, _locale: Locale, msg: &Message) -> Cow<'_, str> {
        match self.inner.template(Locale::En, msg) {
            Ok(template) => {
                let f = FormatLocale {
                    locale: Locale::En,
                    region: None,
                };
                Cow::Owned(self.inner.render(&f, &pseudo(template), &msg.args))
            }
            Err(missing) => Cow::Owned(missing),
        }
    }

    fn has(&self, locale: Locale, key: MessageKey) -> bool {
        self.inner.has(locale, key)
    }

    fn number(&self, f: &FormatLocale, v: f64, style: NumberStyle) -> String {
        self.inner.number(f, v, style)
    }

    fn date(&self, f: &FormatLocale, t: UnixTime, style: DateStyle) -> String {
        self.inner.date(f, t, style)
    }

    fn relative(&self, f: &FormatLocale, delta: Duration) -> String {
        self.inner.relative(f, delta)
    }

    fn quantity(&self, f: &FormatLocale, v: f64, unit: Unit) -> String {
        self.inner.quantity(f, v, unit)
    }
}

/// An accented variant of an ASCII letter; other characters are kept.
fn accent(c: char) -> char {
    const LOWER: &str = "áƀçðéƒĝĥíĵķľmñöþqŕšţüvŵxýž";
    const UPPER: &str = "ÅƁÇÐËƑĜĤÍĴĶĽMÑÖÞQŔŠŢÜVŴXÝŽ";
    let pick = |table: &str, base: u8| {
        let index = usize::from(u8::try_from(c).unwrap_or(base) - base);
        table.chars().nth(index).unwrap_or(c)
    };
    match c {
        'a'..='z' => pick(LOWER, b'a'),
        'A'..='Z' => pick(UPPER, b'A'),
        _ => c,
    }
}

/// The expansion of 02 §14.6 for a text of `len` characters, in characters.
fn padding(len: usize) -> usize {
    let percent = match len {
        0..=10 => 100,
        11..=20 => 60,
        21..=50 => 40,
        _ => 30,
    };
    (len * percent).div_ceil(100)
}

/// `⟦` + the template with accented letters (parameters intact) + `·` padding + `⟧`.
pub(crate) fn pseudo(template: &str) -> String {
    let len = template.chars().count();
    let mut out = String::with_capacity(template.len() * 2 + 8);
    out.push('⟦');
    let mut in_param = false;
    for c in template.chars() {
        match c {
            '{' => in_param = true,
            '}' => in_param = false,
            _ => {}
        }
        out.push(if in_param || c == '}' { c } else { accent(c) });
    }
    out.extend(std::iter::repeat_n('·', padding(len)));
    out.push('⟧');
    out
}

#[cfg(test)]
mod tests {
    use gafira_doc::msg;

    use super::*;

    // S15-11
    #[test]
    fn s15_11_pseudo_accents_brackets_and_expands() {
        assert_eq!(pseudo("Export"), "⟦Ëxþöŕţ······⟧");
        let p = pseudo("{count} pages selected");
        assert!(p.starts_with("⟦{count} "), "{p}");
        assert!(!p.contains("pages"), "{p}");
        assert!(p.ends_with('⟧'));
    }

    #[test]
    fn s15_11_expansion_follows_the_length_bands() {
        assert_eq!(padding(0), 0);
        assert_eq!(padding(10), 10);
        assert_eq!(padding(11), 7);
        assert_eq!(padding(20), 12);
        assert_eq!(padding(21), 9);
        assert_eq!(padding(50), 20);
        assert_eq!(padding(51), 16);
        let dots = |s: &str| s.chars().filter(|c| *c == '·').count();
        assert_eq!(dots(&pseudo("abcdefghijk")), 7);
    }

    #[test]
    fn s15_11_translator_substitutes_parameters_into_the_pseudo_text() {
        let t = PseudoTranslator::new();
        let text = t.text(Locale::Es, &msg!("export.pages_selected", count = Count(3)));
        assert!(text.starts_with("⟦3 þáĝéš šéľéçţéð"), "{text}");
        assert!(text.ends_with('⟧'));
        assert_eq!(t.text(Locale::En, &msg!("no.such_key")), "⟦no.such_key⟧");
        let f = FormatLocale {
            locale: Locale::Es,
            region: None,
        };
        assert_eq!(t.number(&f, 1.5, NumberStyle::Decimal), "1,5");
        assert!(t.has(Locale::Es, MessageKey("export.done")));
    }
}
