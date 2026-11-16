//! ARQ-105 (S14-11): the embedded fonts cover both UI languages. Every
//! character of the Spanish and English catalogs, plus the list of ARQ-105,
//! has a glyph in the `cmap` of Inter and of Manrope.

use gafira_theme::fonts;
use skrifa::{FontRef, MetadataProvider};

const CATALOGS: [(&str, &str); 2] = [
    ("es.toml", include_str!("../../../assets/i18n/es.toml")),
    ("en.toml", include_str!("../../../assets/i18n/en.toml")),
];

const ARQ_105: &str = "¿¡ñÑáéíóúüÁÉÍÓÚÜ«»“”‘’—–…×°";

/// The characters a font must draw: the catalogs' text, escapes included.
fn required() -> Vec<(String, char)> {
    let mut out: Vec<(String, char)> = ARQ_105
        .chars()
        .map(|c| ("ARQ-105".to_string(), c))
        .collect();
    for (name, text) in CATALOGS {
        for c in text.chars().filter(|c| !c.is_control()) {
            out.push((name.to_string(), c));
        }
        // The unit patterns spell their no-break space as a TOML escape.
        if text.contains("\\u00A0") {
            out.push((name.to_string(), '\u{a0}'));
        }
    }
    out.sort();
    out.dedup();
    out
}

#[test]
fn arq_105_embedded_fonts_cover_the_catalogs_and_the_spanish_set() {
    let needed = required();
    assert!(needed.len() > 60, "{}", needed.len());
    for font in fonts() {
        let face = FontRef::new(font.bytes).expect("a valid font");
        let charmap = face.charmap();
        let missing: Vec<String> = needed
            .iter()
            .filter(|(_, c)| charmap.map(*c).is_none())
            .map(|(from, c)| format!("{c:?} (U+{:04X}, {from})", u32::from(*c)))
            .collect();
        assert!(missing.is_empty(), "{} lacks {missing:?}", font.family);
    }
}

#[test]
fn arq_105_the_check_reports_a_character_the_fonts_lack() {
    // U+27E6 (the pseudo-localization bracket) is outside the embedded subsets.
    for font in fonts() {
        let face = FontRef::new(font.bytes).expect("a valid font");
        assert!(face.charmap().map('⟦').is_none(), "{}", font.family);
        assert!(face.charmap().map('ñ').is_some(), "{}", font.family);
    }
}
