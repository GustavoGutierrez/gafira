//! The compiled catalogs: sorted static tables with binary search (02 §14.5).

use gafira_ports::i18n::Locale;
use icu_plurals::PluralCategory;

/// The forms of a plural message, by CLDR cardinal category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Plural {
    pub zero: Option<&'static str>,
    pub one: Option<&'static str>,
    pub two: Option<&'static str>,
    pub few: Option<&'static str>,
    pub many: Option<&'static str>,
    pub other: &'static str,
}

impl Plural {
    /// The form of `category`, or `other` when the catalog lacks it (02 §14.5).
    pub fn form(&self, category: PluralCategory) -> &'static str {
        let form = match category {
            PluralCategory::Zero => self.zero,
            PluralCategory::One => self.one,
            PluralCategory::Two => self.two,
            PluralCategory::Few => self.few,
            PluralCategory::Many => self.many,
            PluralCategory::Other => None,
        };
        form.unwrap_or(self.other)
    }

    #[cfg(test)]
    /// Whether the catalog spells out `category`.
    pub fn has(&self, category: PluralCategory) -> bool {
        match category {
            PluralCategory::Zero => self.zero.is_some(),
            PluralCategory::One => self.one.is_some(),
            PluralCategory::Two => self.two.is_some(),
            PluralCategory::Few => self.few.is_some(),
            PluralCategory::Many => self.many.is_some(),
            PluralCategory::Other => true,
        }
    }

    #[cfg(test)]
    pub fn forms(&self) -> impl Iterator<Item = &'static str> {
        [
            self.zero,
            self.one,
            self.two,
            self.few,
            self.many,
            Some(self.other),
        ]
        .into_iter()
        .flatten()
    }
}

/// One catalog value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Entry {
    Text(&'static str),
    Plural(Plural),
}

impl Entry {
    #[cfg(test)]
    pub fn forms(&self) -> Vec<&'static str> {
        match self {
            Self::Text(t) => vec![t],
            Self::Plural(p) => p.forms().collect(),
        }
    }
}

include!(concat!(env!("OUT_DIR"), "/catalogs.rs"));

/// The messages of one language, sorted by key.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Catalog {
    entries: &'static [(&'static str, Entry)],
}

impl Catalog {
    pub const fn new(entries: &'static [(&'static str, Entry)]) -> Self {
        Self { entries }
    }

    /// The embedded catalog of `locale`.
    pub const fn embedded(locale: Locale) -> Self {
        match locale {
            Locale::Es => Self::new(ES),
            Locale::En => Self::new(EN),
        }
    }

    pub fn get(&self, key: &str) -> Option<&'static Entry> {
        let entries: &'static [(&'static str, Entry)] = self.entries;
        entries
            .binary_search_by(|(k, _)| (*k).cmp(key))
            .ok()
            .map(|i| &entries[i].1)
    }

    #[cfg(test)]
    pub fn entries(&self) -> &'static [(&'static str, Entry)] {
        self.entries
    }
}

#[cfg(test)]
/// The `{name}` parameters of a template, in order of appearance.
pub(crate) fn params(template: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else { break };
        out.push(&after[..end]);
        rest = &after[end + 1..];
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use gafira_doc::msg::is_valid_key;
    use icu_locale_core::locale;
    use icu_plurals::PluralRules;

    use super::*;

    fn param_set(entry: &Entry) -> BTreeSet<&'static str> {
        entry.forms().into_iter().flat_map(params).collect()
    }

    // S15-05 (part of ARQ-98): the generated tables agree with each other.
    #[test]
    fn s15_05_es_and_en_have_the_same_keys_and_parameters() {
        let (es, en) = (Catalog::embedded(Locale::Es), Catalog::embedded(Locale::En));
        let keys = |c: &Catalog| c.entries().iter().map(|(k, _)| *k).collect::<Vec<_>>();
        assert_eq!(keys(&es), keys(&en));
        assert!(!es.entries().is_empty());
        for (key, entry) in es.entries() {
            assert!(is_valid_key(key), "{key}");
            let other = en.get(key).expect("same keys");
            assert_eq!(param_set(entry), param_set(other), "{key}");
            for form in entry.forms().into_iter().chain(other.forms()) {
                assert!(!form.is_empty(), "{key} has an empty form");
            }
        }
    }

    #[test]
    fn s15_05_tables_are_sorted_so_binary_search_finds_every_key() {
        for locale in [Locale::Es, Locale::En] {
            let c = Catalog::embedded(locale);
            assert!(c.entries().windows(2).all(|w| w[0].0 < w[1].0));
            for (key, entry) in c.entries() {
                assert_eq!(c.get(key), Some(entry), "{key}");
            }
            assert_eq!(c.get("no.such_key"), None);
        }
    }

    // S15-04 (part of ARQ-98): every plural has the categories CLDR requires.
    #[test]
    fn s15_04_plurals_have_every_category_their_language_requires() {
        let cases = [(Locale::Es, locale!("es")), (Locale::En, locale!("en"))];
        for (locale, icu) in cases {
            let rules = PluralRules::try_new_cardinal(icu.into()).expect("plural data");
            let required: Vec<PluralCategory> = rules.categories().collect();
            let mut plurals = 0;
            for (key, entry) in Catalog::embedded(locale).entries() {
                if let Entry::Plural(p) = entry {
                    plurals += 1;
                    for c in &required {
                        assert!(p.has(*c), "{locale:?} {key} lacks {c:?}");
                    }
                }
            }
            assert!(plurals > 0);
        }
        let es = PluralRules::try_new_cardinal(locale!("es").into()).expect("data");
        assert!(
            es.categories().any(|c| c == PluralCategory::Many),
            "CLDR es has `many`"
        );
    }

    #[test]
    fn s15_04_a_missing_category_falls_back_to_other() {
        let p = Plural {
            zero: None,
            one: Some("one"),
            two: None,
            few: None,
            many: None,
            other: "other",
        };
        assert_eq!(p.form(PluralCategory::One), "one");
        assert_eq!(p.form(PluralCategory::Many), "other");
        assert_eq!(p.form(PluralCategory::Other), "other");
    }

    #[test]
    fn params_lists_names_in_order() {
        assert_eq!(params("{a} and {b}{a}"), ["a", "b", "a"]);
        assert!(params("no params").is_empty());
        assert_eq!(params("open { only"), Vec::<&str>::new());
    }
}
