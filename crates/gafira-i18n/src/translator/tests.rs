use gafira_doc::msg;
use gafira_ports::i18n::Region;

use super::*;
use crate::catalog::Plural;

const NBSP: char = '\u{a0}';

fn t() -> CatalogTranslator {
    CatalogTranslator::new()
}

fn fl(locale: Locale, region: Option<&str>) -> FormatLocale {
    FormatLocale {
        locale,
        region: region.and_then(Region::new),
    }
}

// Incomplete test catalogs for ARQ-99: `only.en` is missing in es, `only.es`
// in en, and `in.neither` in both.
static TEST_ES: &[(&str, Entry)] = &[
    ("both.here", Entry::Text("es: both")),
    ("only.es", Entry::Text("es: only")),
];
static TEST_EN: &[(&str, Entry)] = &[
    ("both.here", Entry::Text("en: both")),
    ("only.en", Entry::Text("en: only")),
];

fn test_translator(fallback: Fallback) -> CatalogTranslator {
    CatalogTranslator::with(Catalog::new(TEST_ES), Catalog::new(TEST_EN), fallback)
}

/// `^[a-z0-9_]+(\.[a-z0-9_]+)+$`, the key pattern of ARQ-99.
fn looks_like_a_key(text: &str) -> bool {
    let segments: Vec<&str> = text.split('.').collect();
    segments.len() >= 2
        && segments.iter().all(|s| {
            !s.is_empty()
                && s.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        })
}

// S15-03
#[test]
fn s15_03_text_substitutes_parameters() {
    let m = msg!(
        "settings.language.system_now",
        language = Text("Español".into()),
        tag = Text("es-MX".into())
    );
    assert_eq!(t().text(Locale::Es, &m), "Sistema · ahora: Español (es-MX)");
    assert_eq!(t().text(Locale::En, &m), "System · now: Español (es-MX)");
    // A parameter without its argument stays literal.
    let m = msg!(
        "settings.language.system_now",
        language = Text("English".into())
    );
    assert_eq!(t().text(Locale::En, &m), "System · now: English ({tag})");
    // No parameters: the catalog text itself.
    assert!(matches!(
        t().text(Locale::Es, &msg!("brand.slogan")),
        Cow::Borrowed("Haz visibles tus ideas.")
    ));
    assert_eq!(
        t().text(Locale::En, &msg!("brand.slogan")),
        "Bring your ideas to life."
    );
}

#[test]
fn s15_03_has_is_exact_per_language() {
    assert!(t().has(Locale::Es, MessageKey("export.done")));
    assert!(t().has(Locale::En, MessageKey("export.done")));
    assert!(!t().has(Locale::Es, MessageKey("no.such_key")));
    let tt = test_translator(Fallback::Development);
    assert!(!tt.has(Locale::Es, MessageKey("only.en")));
    assert!(tt.has(Locale::En, MessageKey("only.en")));
}

// S15-04
#[test]
fn s15_04_plural_forms_follow_cldr() {
    let m = |n| msg!("export.pages_selected", count = Count(n));
    assert_eq!(t().text(Locale::Es, &m(1)), "1 página seleccionada");
    assert_eq!(t().text(Locale::En, &m(1)), "1 page selected");
    assert_eq!(t().text(Locale::Es, &m(3)), "3 páginas seleccionadas");
    assert_eq!(t().text(Locale::En, &m(3)), "3 pages selected");
    assert_eq!(t().text(Locale::En, &m(0)), "0 pages selected");
    assert_eq!(
        t().text(Locale::Es, &m(1_000_000)),
        "1.000.000 de páginas seleccionadas"
    );
    assert_eq!(
        t().text(Locale::En, &m(1_000_000)),
        "1,000,000 pages selected"
    );
    // Without a Count, `other`.
    assert_eq!(
        t().text(Locale::En, &msg!("export.pages_selected")),
        "{count} pages selected"
    );
}

// S15-06 (ARQ-99)
#[test]
fn s15_06_missing_in_spanish_falls_back_to_english() {
    for fallback in [Fallback::Development, Fallback::Distribution] {
        let tt = test_translator(fallback);
        assert_eq!(tt.text(Locale::Es, &msg!("only.en")), "en: only");
        assert_eq!(tt.text(Locale::Es, &msg!("both.here")), "es: both");
    }
}

#[test]
fn s15_06_missing_everywhere_shows_the_key_in_development() {
    let tt = test_translator(Fallback::Development);
    assert_eq!(tt.text(Locale::Es, &msg!("in.neither")), "⟦in.neither⟧");
    assert_eq!(tt.text(Locale::En, &msg!("in.neither")), "⟦in.neither⟧");
    // English requested, only Spanish has it: development does not use the other language.
    assert_eq!(tt.text(Locale::En, &msg!("only.es")), "⟦only.es⟧");
}

#[test]
fn s15_06_dist_uses_the_other_language_then_an_ellipsis_never_a_key() {
    let tt = test_translator(Fallback::Distribution);
    assert_eq!(tt.text(Locale::En, &msg!("only.es")), "es: only");
    assert_eq!(tt.text(Locale::Es, &msg!("in.neither")), MISSING);
    assert_eq!(tt.text(Locale::En, &msg!("in.neither")), MISSING);
    for key in ["both.here", "only.es", "only.en", "in.neither"] {
        let m = Message {
            key: MessageKey(key),
            args: Vec::new(),
        };
        for locale in [Locale::Es, Locale::En] {
            let text = tt.text(locale, &m);
            assert!(!looks_like_a_key(&text), "{key} in {locale:?} gave {text}");
        }
    }
    assert!(looks_like_a_key("only.es") && !looks_like_a_key("es: only"));
}

#[test]
fn s15_06_this_build_uses_the_development_fallback() {
    // Tests build with debug assertions; `dist` builds take the other arm.
    assert_eq!(Fallback::BUILD, Fallback::Development);
}

// S15-07 (D-64)
#[test]
fn s15_07_number_styles_and_region() {
    let (es, es_mx, en) = (
        fl(Locale::Es, None),
        fl(Locale::Es, Some("MX")),
        fl(Locale::En, None),
    );
    assert_eq!(t().number(&es, 12345.5, NumberStyle::Decimal), "12.345,5");
    assert_eq!(t().number(&es_mx, 1234.5, NumberStyle::Decimal), "1,234.5");
    assert_eq!(t().number(&en, 1234.567, NumberStyle::Decimal), "1,234.57");
    assert_eq!(t().number(&es, 1.5, NumberStyle::Tabular), "1,50");
    assert_eq!(t().number(&en, 3.0, NumberStyle::Tabular), "3.00");
    assert_eq!(
        t().number(&es, 50.0, NumberStyle::Percent),
        format!("50{NBSP}%")
    );
    // Half-even, the ICU default: 12.25 → 12.2, 12.35 → 12.4.
    assert_eq!(t().number(&en, 12.25, NumberStyle::Percent), "12.2%");
    assert_eq!(t().number(&en, 12.35, NumberStyle::Percent), "12.4%");
    assert_eq!(t().number(&en, 100.0, NumberStyle::Percent), "100%");
    assert_eq!(t().number(&en, f64::NAN, NumberStyle::Decimal), "NaN");
}

// S15-08
#[test]
fn s15_08_quantities_use_the_unit_patterns() {
    let (es, en) = (fl(Locale::Es, None), fl(Locale::En, None));
    assert_eq!(t().quantity(&es, 0.5, Unit::Seconds), format!("0,5{NBSP}s"));
    assert_eq!(t().quantity(&en, 0.5, Unit::Seconds), format!("0.5{NBSP}s"));
    assert_eq!(
        t().quantity(&en, 1080.0, Unit::Px),
        format!("1,080{NBSP}px")
    );
    assert_eq!(t().quantity(&es, 1080.0, Unit::Px), format!("1080{NBSP}px"));
    assert_eq!(
        t().quantity(&es, 1_200_000.0, Unit::Bytes),
        format!("1,2{NBSP}MB")
    );
    assert_eq!(
        t().quantity(&en, 1_200_000.0, Unit::Bytes),
        format!("1.2{NBSP}MB")
    );
    assert_eq!(t().quantity(&en, 999.0, Unit::Bytes), format!("999{NBSP}B"));
    assert_eq!(
        t().quantity(&en, 3e12, Unit::Bytes),
        format!("3,000{NBSP}GB")
    );
    assert_eq!(
        t().quantity(&en, -14.0, Unit::Lufs),
        format!("-14{NBSP}LUFS")
    );
    assert_eq!(t().quantity(&en, 90.0, Unit::Degrees), "90°");
    assert_eq!(t().quantity(&es, 25.0, Unit::Percent), format!("25{NBSP}%"));
    for unit in [Unit::Mm, Unit::Cm, Unit::In, Unit::Pt, Unit::Fps, Unit::Db] {
        let text = t().quantity(&en, 2.0, unit);
        assert!(
            text.starts_with("2\u{a0}") && !text.contains('⟦'),
            "{unit:?}: {text}"
        );
    }
}

// S15-09
#[test]
fn s15_09_relative_times() {
    let (es, en) = (fl(Locale::Es, None), fl(Locale::En, None));
    let secs = Duration::from_secs;
    assert_eq!(t().relative(&es, secs(30)), "ahora mismo");
    assert_eq!(t().relative(&en, secs(59)), "just now");
    assert_eq!(t().relative(&es, secs(60)), "hace 1 min");
    assert_eq!(t().relative(&en, secs(7_200)), "2 h ago");
    assert_eq!(t().relative(&es, secs(7_200)), "hace 2 h");
    assert_eq!(t().relative(&es, secs(86_400)), "hace 1 día");
    assert_eq!(t().relative(&en, secs(86_400)), "1 day ago");
    assert_eq!(t().relative(&en, secs(3 * 86_400)), "3 days ago");
}

// S15-10
#[test]
fn s15_10_typed_arguments_are_formatted_for_the_locale() {
    static ES: &[(&str, Entry)] = &[("test.args", Entry::Text("{n}|{i}|{d}|{q}|{e}|{s}|{k}|{x}"))];
    let tt = CatalogTranslator::with(Catalog::new(ES), Catalog::new(ES), Fallback::Development);
    let m = msg!(
        "test.args",
        n = Count(12345),
        i = Int(-5),
        d = Decimal(1.5),
        q = Quantity(0.5, Unit::Seconds),
        e = Elapsed(7200),
        s = Timestamp(0),
        k = Key(MessageKey("test.args")),
        x = Text("Café".into())
    );
    let text = tt.text(Locale::Es, &m);
    assert_eq!(
        text,
        // The test catalog has neither `unit.seconds` nor `time.hours_ago`; the
        // Key argument shows the raw template of `test.args`, one level deep.
        "12.345|-5|1,5|⟦unit.seconds⟧|⟦time.hours_ago⟧|1970-01-01|{n}|{i}|{d}|{q}|{e}|{s}|{k}|{x}|Café"
    );
    // A Key is translated one level, in the same language.
    let m = msg!(
        "settings.language.title",
        unused = Key(MessageKey("brand.slogan"))
    );
    assert_eq!(t().text(Locale::En, &m), "Language");
    // Sorted by key, as the generated tables are.
    static KEYED: &[(&str, Entry)] = &[
        ("test.action", Entry::Text("mover")),
        ("test.undo", Entry::Text("Deshacer {action}")),
    ];
    let tt = CatalogTranslator::with(
        Catalog::new(KEYED),
        Catalog::new(KEYED),
        Fallback::Development,
    );
    let m = msg!("test.undo", action = Key(MessageKey("test.action")));
    assert_eq!(tt.text(Locale::Es, &m), "Deshacer mover");
}

#[test]
fn s15_10_a_plural_count_picks_the_form_and_is_formatted() {
    static ES: &[(&str, Entry)] = &[(
        "test.items",
        Entry::Plural(Plural {
            zero: None,
            one: Some("{count} uno"),
            two: None,
            few: None,
            many: None,
            other: "{count} varios",
        }),
    )];
    let tt = CatalogTranslator::with(Catalog::new(ES), Catalog::new(ES), Fallback::Development);
    assert_eq!(
        tt.text(Locale::Es, &msg!("test.items", count = Count(1))),
        "1 uno"
    );
    assert_eq!(
        tt.text(Locale::En, &msg!("test.items", count = Count(1234))),
        "1,234 varios"
    );
}

// S15-12
#[test]
fn s15_12_date_is_the_iso_interim_in_both_languages() {
    for locale in [Locale::Es, Locale::En] {
        let f = fl(locale, None);
        assert_eq!(t().date(&f, UnixTime(0), DateStyle::Short), "1970-01-01");
        assert_eq!(
            t().date(&f, UnixTime(0), DateStyle::Long),
            "1970-01-01 00:00 UTC"
        );
    }
}

#[test]
fn translator_is_a_send_sync_trait_object() {
    let boxed: Box<dyn Translator> = Box::new(CatalogTranslator::default());
    assert_eq!(
        boxed.text(Locale::En, &msg!("export.done")),
        "Your design is ready to share"
    );
}
