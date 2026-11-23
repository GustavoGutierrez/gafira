//! The `assets/<blake3-hex>.<ext>` entries of a `.gaf` (02 §7.1, spec 026).

use gafira_doc::AssetRef;
use zip::CompressionMethod;

/// The folder of the embedded assets.
pub(crate) const FOLDER: &str = "assets/";

/// The extension written when an asset's name has none that is usable.
const FALLBACK: &str = "bin";

/// The longest extension an entry name may carry.
const MAX_EXT: usize = 10;

/// Formats whose bytes are already compressed: DEFLATE would only cost time.
const COMPRESSED: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "gif", "avif", "heic", "mp4", "m4v", "m4a", "mov", "webm", "mkv",
    "mp3", "aac", "ogg", "opus", "flac", "zip", "gz", "woff", "woff2",
];

/// An extension an entry name may carry: 1 to 10 lowercase ASCII letters or
/// digits.
fn usable(ext: &str) -> bool {
    (1..=MAX_EXT).contains(&ext.len())
        && ext
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

/// The extension of the entry for an asset called `name`: its own, in
/// lowercase, or `bin` when it has none that is usable.
pub(crate) fn extension(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .filter(|ext| usable(ext))
        .unwrap_or_else(|| FALLBACK.to_owned())
}

/// The entry name of an asset.
pub(crate) fn entry_name(reference: AssetRef, name: &str) -> String {
    format!("{FOLDER}{}.{}", reference.hex(), extension(name))
}

/// How an asset with this extension is stored.
pub(crate) fn method(ext: &str) -> CompressionMethod {
    if COMPRESSED.contains(&ext) {
        CompressionMethod::Stored
    } else {
        CompressionMethod::Deflated
    }
}

/// The reference an `assets/` entry name carries, or `None` when the name is
/// not exactly `assets/<64 lowercase hex>.<ext>`.
pub(crate) fn parse(entry: &str) -> Option<AssetRef> {
    let (hex, ext) = entry.strip_prefix(FOLDER)?.split_once('.')?;
    let reference = AssetRef::from_hex(hex)?;
    (reference.hex() == hex && usable(ext)).then_some(reference)
}

#[cfg(test)]
mod tests {
    use gafira_doc::ContentHash;

    use super::*;

    #[test]
    fn s26_05_the_extension_comes_from_the_name() {
        assert_eq!(extension("Clip.MP4"), "mp4");
        assert_eq!(extension("a.tar.gz"), "gz");
        assert_eq!(extension("README"), "bin");
        assert_eq!(extension("odd.m p4"), "bin");
        assert_eq!(extension("trailing."), "bin");
        assert_eq!(extension("long.abcdefghijk"), "bin");
        assert_eq!(method("png"), CompressionMethod::Stored);
        assert_eq!(method("svg"), CompressionMethod::Deflated);
    }

    #[test]
    fn s26_09_only_exact_entry_names_parse() {
        let reference = AssetRef::new(ContentHash([0xAB; 32]));
        let name = entry_name(reference, "x.png");
        assert_eq!(parse(&name), Some(reference));
        assert_eq!(parse(&name.to_uppercase()), None);
        assert_eq!(parse(&format!("assets/{}.p.ng", reference.hex())), None);
        assert_eq!(parse("assets/ab.png"), None);
        assert_eq!(parse(&name.replace("assets/", "pages/")), None);
    }
}
