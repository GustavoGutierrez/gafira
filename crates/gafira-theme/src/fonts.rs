//! Inter and Manrope, embedded with their OFL text (DIS-05, 02 §8.2).
//!
//! Provisional files: the tracked variable subsets of `assets/fonts/test/`
//! (Latin, Latin-1 and Latin Extended-A, typographic punctuation), the only
//! font files with an entry in `assets/LICENSES.toml`. The product files are
//! an open owner decision (spec 014); swapping them means changing the paths
//! below and registering the new files.

/// A font file compiled into the binary.
#[derive(Clone, Copy, Debug)]
pub struct EmbeddedFont {
    /// The family name the tokens use in `type.family`.
    pub family: &'static str,
    /// The repository path of the file, for the licence register.
    pub path: &'static str,
    pub bytes: &'static [u8],
    /// The SIL Open Font License text shipped with the font.
    pub license: &'static str,
}

const ALL: [EmbeddedFont; 2] = [
    EmbeddedFont {
        family: "Inter",
        path: "assets/fonts/test/Inter-test.ttf",
        bytes: include_bytes!("../../../assets/fonts/test/Inter-test.ttf"),
        license: include_str!("../../../assets/fonts/test/OFL-inter.txt"),
    },
    EmbeddedFont {
        family: "Manrope",
        path: "assets/fonts/test/Manrope-test.ttf",
        bytes: include_bytes!("../../../assets/fonts/test/Manrope-test.ttf"),
        license: include_str!("../../../assets/fonts/test/OFL-manrope.txt"),
    },
];

/// Every embedded font, Inter first.
#[must_use]
pub fn fonts() -> &'static [EmbeddedFont] {
    &ALL
}

/// The embedded font of `family`, if there is one.
#[must_use]
pub fn font(family: &str) -> Option<&'static EmbeddedFont> {
    ALL.iter().find(|f| f.family == family)
}

#[cfg(test)]
mod tests {
    use super::*;

    // S14-10
    #[test]
    fn s14_10_inter_and_manrope_are_embedded_with_their_licence() {
        let families: Vec<&str> = fonts().iter().map(|f| f.family).collect();
        assert_eq!(families, ["Inter", "Manrope"]);
        for f in fonts() {
            assert!(f.bytes.len() > 1000, "{}", f.family);
            assert!(f.license.contains("SIL OPEN FONT LICENSE"), "{}", f.family);
        }
        assert_eq!(font("Manrope").map(|f| f.family), Some("Manrope"));
        assert!(font("Roboto").is_none());
    }
}
