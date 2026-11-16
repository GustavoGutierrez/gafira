//! Locale-aware numbers with ICU4X (D-64, 02 §14.5) and the interim dates.

use gafira_ports::i18n::{DateStyle, FormatLocale, Locale};
use icu_decimal::DecimalFormatter;
use icu_decimal::input::Decimal;

/// The ICU locale of `f`: the language plus the region when there is one.
pub(crate) fn icu_locale(f: &FormatLocale) -> icu_locale_core::Locale {
    let language = match f.locale {
        Locale::Es => "es",
        Locale::En => "en",
    };
    let tag = match f.region {
        Some(r) => format!("{language}-{}", r.as_str()),
        None => language.to_string(),
    };
    icu_locale_core::Locale::try_from_str(&tag)
        .or_else(|_| icu_locale_core::Locale::try_from_str(language))
        .unwrap_or(icu_locale_core::Locale::UNKNOWN)
}

/// `v` rounded to `digits` fraction digits; `pad` keeps trailing zeros.
/// `None` for a non-finite value.
pub(crate) fn decimal(v: f64, digits: i16, pad: bool) -> Option<Decimal> {
    if !v.is_finite() {
        return None;
    }
    // `Display` of an f64 is the shortest round-trip text and never uses an exponent.
    let mut d = Decimal::try_from_str(&format!("{v}")).ok()?;
    d.round(-digits);
    if d.absolute.is_zero() {
        // No "-0" after rounding a tiny negative value.
        d = Decimal::from(0);
    }
    if pad {
        d.absolute.pad_end(-digits);
    } else {
        d.absolute.trim_end();
    }
    Some(d)
}

/// The CLDR symbol of a non-finite value (`∞`, `-∞`, `NaN`).
pub(crate) fn non_finite(v: f64) -> &'static str {
    if v.is_nan() {
        "NaN"
    } else if v > 0.0 {
        "∞"
    } else {
        "-∞"
    }
}

/// `d` with the separators and grouping of `f` (CLDR through ICU4X).
pub(crate) fn format_decimal(f: &FormatLocale, d: &Decimal) -> String {
    let locale = icu_locale(f);
    match DecimalFormatter::try_new((&locale).into(), Default::default()) {
        Ok(formatter) => formatter.format(d).to_string(),
        // Unreachable with compiled data; the plain digits are still correct.
        Err(_) => d.to_string(),
    }
}

/// `v` rounded to `digits` fraction digits and formatted for `f`.
pub(crate) fn format_number(f: &FormatLocale, v: f64, digits: i16, pad: bool) -> String {
    match decimal(v, digits, pad) {
        Some(d) => format_decimal(f, &d),
        None => non_finite(v).to_string(),
    }
}

/// Interim, until the dates spec (015 «No alcance»): ISO 8601 in UTC, the same
/// in every language. `Short` is `YYYY-MM-DD`; `Long` adds `HH:MM UTC`.
pub(crate) fn iso_date(seconds: i64, style: DateStyle) -> String {
    let days = seconds.div_euclid(86_400);
    let secs = seconds.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    match style {
        DateStyle::Short => format!("{y:04}-{m:02}-{d:02}"),
        DateStyle::Long => format!(
            "{y:04}-{m:02}-{d:02} {:02}:{:02} UTC",
            secs / 3600,
            secs % 3600 / 60
        ),
    }
}

/// Days since 1970-01-01 to a proleptic Gregorian date (H. Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use gafira_ports::i18n::Region;

    use super::*;

    fn fl(locale: Locale, region: Option<&str>) -> FormatLocale {
        FormatLocale {
            locale,
            region: region.and_then(Region::new),
        }
    }

    // S15-07 (D-64): the values of the 02 §14.5 table.
    #[test]
    fn s15_07_decimal_separators_and_grouping_follow_cldr() {
        let es = fl(Locale::Es, None);
        let es_mx = fl(Locale::Es, Some("MX"));
        let en = fl(Locale::En, None);
        let cases = [
            (&es, 1.5, "1,5"),
            (&es, 1234.5, "1234,5"),
            (&es, 12345.5, "12.345,5"),
            (&es_mx, 1234.5, "1,234.5"),
            (&en, 1.5, "1.5"),
            (&en, 1234.5, "1,234.5"),
            (&en, 12345.5, "12,345.5"),
            (&en, -14.0, "-14"),
            (&en, 0.004, "0"),
            (&en, -0.004, "0"),
        ];
        for (f, v, want) in cases {
            assert_eq!(format_number(f, v, 2, false), want, "{f:?} {v}");
        }
        assert_eq!(format_number(&en, 1.5, 2, true), "1.50");
        assert_eq!(format_number(&en, f64::NAN, 2, false), "NaN");
        assert_eq!(format_number(&en, f64::INFINITY, 2, false), "∞");
        assert_eq!(format_number(&en, f64::NEG_INFINITY, 2, false), "-∞");
    }

    // S15-12: the declared interim.
    #[test]
    fn s15_12_dates_are_iso_8601_utc() {
        assert_eq!(iso_date(0, DateStyle::Short), "1970-01-01");
        assert_eq!(iso_date(0, DateStyle::Long), "1970-01-01 00:00 UTC");
        // 2026-03-12 14:05:59 UTC
        assert_eq!(
            iso_date(1_773_324_359, DateStyle::Long),
            "2026-03-12 14:05 UTC"
        );
        assert_eq!(iso_date(951_782_400, DateStyle::Short), "2000-02-29");
        assert_eq!(iso_date(-1, DateStyle::Long), "1969-12-31 23:59 UTC");
    }
}
