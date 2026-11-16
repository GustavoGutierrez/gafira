//! Layer-4 adapter `gafira-format` (02 §2.1, §7.1; spec 019): reads and
//! writes the `.gaf` project file and implements `ports::ProjectRepository`.
//!
//! The container is the hybrid layout of D-68: a ZIP whose first entry is the
//! uncompressed `mimetype`, a camelCase `manifest.json` (008 §5), a
//! `document.json` with everything but the pages and the sequences, and one
//! JSON per page and per sequence in the snake_case of `gafira-doc`. Times are
//! flicks (D-69), hashes BLAKE3 (`ContentHash`). Reading treats the file as
//! hostile (008 §11): every entry name is checked, sizes are capped and a
//! compression ratio that only a ZIP bomb has is refused. Embedded assets
//! travel as `assets/<blake3-hex>.<ext>` and come from, and go to, a
//! `ports::ContentStore` (spec 026); each one is checked against its hash.
//! Versions live in `versions/`: an index and one folder per version with
//! its own `document.json`, pages and sequences; their assets share
//! `assets/` with the document, by hash (spec 031).

#![forbid(unsafe_code)]

mod assets;
mod limits;
mod manifest;
mod read;
mod repository;
mod versions;
mod write;

pub use limits::Limits;
pub use read::read_gaf;
pub use repository::GafRepository;
pub use write::write_gaf;

/// The exact text of the `mimetype` entry (02 §7.1).
pub const MIMETYPE: &str = "application/vnd.gafira.project+zip";
/// The `format` of the manifest (D-68).
pub const FORMAT: &str = "gafira-project";
/// The container version this build writes (D-68).
pub const FORMAT_VERSION: &str = "1.0.0";
