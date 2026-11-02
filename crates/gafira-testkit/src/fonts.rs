//! Test fonts for the text stack (ARQ-115).
//!
//! The bytes are embedded with `include_bytes!`, because domain crates may not
//! read files under `--all-targets` (ARQ-02). They are subsets of Inter,
//! Manrope, Noto Sans Arabic and Noto Emoji; the recipe and the licence of each
//! are in `assets/LICENSES.toml`.

/// One embedded test font.
#[derive(Clone, Copy, Debug)]
pub struct TestFont {
    /// Path of the file from the repository root, as `assets/LICENSES.toml` writes it.
    pub path: &'static str,
    /// The whole font file.
    pub bytes: &'static [u8],
}

/// Inter subset: Latin. It has no `liga` feature.
pub const INTER: TestFont = TestFont {
    path: "assets/fonts/test/Inter-test.ttf",
    bytes: include_bytes!("../../../assets/fonts/test/Inter-test.ttf"),
};

/// Manrope subset: Latin, with the optional `liga` feature.
pub const MANROPE: TestFont = TestFont {
    path: "assets/fonts/test/Manrope-test.ttf",
    bytes: include_bytes!("../../../assets/fonts/test/Manrope-test.ttf"),
};

/// Noto Sans Arabic subset: Arabic with joining forms.
pub const NOTO_SANS_ARABIC: TestFont = TestFont {
    path: "assets/fonts/test/NotoSansArabic-test.ttf",
    bytes: include_bytes!("../../../assets/fonts/test/NotoSansArabic-test.ttf"),
};

/// Noto Emoji subset: monochrome outlines for a few emoji.
pub const NOTO_EMOJI: TestFont = TestFont {
    path: "assets/fonts/test/NotoEmoji-test.ttf",
    bytes: include_bytes!("../../../assets/fonts/test/NotoEmoji-test.ttf"),
};

/// Every test font.
pub const ALL: [TestFont; 4] = [INTER, MANROPE, NOTO_SANS_ARABIC, NOTO_EMOJI];
