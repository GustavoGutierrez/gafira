//! The Biblioteca panel (spec 068): the files imported into Gafira, newest
//! first, as previews in two tabs, «Imágenes» (PNG, JPEG, and SVG brought in
//! as vector shapes) and «Diseños» (SVG and PDF brought in as the outlines
//! of frames), with a search by name, ticks, a menu and details.

use std::path::{Path, PathBuf};

/// A file of Biblioteca: where it is, whether it came in as a design (the
/// outline of a frame) or as an image, and when, in seconds since the Unix
/// epoch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryEntry {
    pub path: PathBuf,
    pub design: bool,
    pub added: i64,
}

/// Where the imported files are remembered (the app's `library.json`).
pub trait LibraryStore {
    /// The imported files that still exist, newest first.
    fn existing(&self) -> Vec<LibraryEntry>;
    /// Remember `file` as just imported, as a design or an image.
    fn record(&self, file: &Path, design: bool);
    /// Take these files (path and tab) out of the list; they stay on disk.
    fn remove(&self, entries: &[(PathBuf, bool)]);
}

/// The tabs of Biblioteca.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Tab {
    #[default]
    Images,
    Designs,
}

/// Whether Biblioteca shows `file` at all, by its extension.
pub(crate) fn known(file: &Path) -> bool {
    file.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        matches!(
            e.to_ascii_lowercase().as_str(),
            "png" | "jpg" | "jpeg" | "svg" | "pdf"
        )
    })
}

/// The entries of `tab` whose name holds `query` (any case), in order: an
/// entry shows in the tab it came in as (spec 068).
pub(crate) fn shown(files: &[LibraryEntry], tab: Tab, query: &str) -> Vec<LibraryEntry> {
    let query = query.trim().to_lowercase();
    files
        .iter()
        .filter(|entry| known(&entry.path) && entry.design == (tab == Tab::Designs))
        .filter(|entry| {
            query.is_empty()
                || entry
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.to_lowercase().contains(&query))
        })
        .cloned()
        .collect()
}

/// What the panel keeps between frames: the tab, the search, the files
/// read from the store (`None` until read again), the files ticked and the
/// one whose details show.
#[derive(Default)]
pub(crate) struct View {
    pub(crate) tab: Tab,
    pub(crate) query: String,
    pub(crate) files: Option<Vec<LibraryEntry>>,
    pub(crate) ticked: Vec<(PathBuf, bool)>,
    pub(crate) info: Option<LibraryEntry>,
    /// The file whose «···» menu is open; it stays open until an option or
    /// a click elsewhere.
    pub(crate) menu: Option<(PathBuf, bool)>,
}

/// A date of `seconds` since the Unix epoch as day/month/year (UTC).
pub(crate) fn date(seconds: i64) -> String {
    // Days to the civil date (Howard Hinnant's algorithm).
    let z = seconds.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{day:02}/{month:02}/{year}")
}

/// A size in bytes as MB with two decimals, or as whole KB.
pub(crate) fn bytes(size: u64) -> String {
    const MB: u64 = 1024 * 1024;
    if size >= MB {
        let hundredths = (size * 100 + MB / 2) / MB;
        format!("{}.{:02} MB", hundredths / 100, hundredths % 100)
    } else {
        format!("{} KB", ((size + 512) / 1024).max(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, design: bool) -> LibraryEntry {
        LibraryEntry {
            path: PathBuf::from(path),
            design,
            added: 0,
        }
    }

    // S68-02, S68-03: an entry shows in the tab it came in as (an SVG as
    // vector shapes is an image, as a frame a design); the search keeps the
    // names that hold it, in any case, in order; other files never show.
    #[test]
    fn s68_03_tabs_and_search() {
        let files = [
            entry("/x/Playa.png", false),
            entry("/x/perro.pdf", true),
            entry("/x/playa-2.jpg", false),
            entry("/x/sol.png", false),
            entry("/x/logo.svg", false),
            entry("/x/logo.svg", true),
            entry("/x/notes.txt", false),
        ];
        let names = |tab, query| {
            shown(&files, tab, query)
                .iter()
                .map(|e| {
                    e.path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_owned()
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            names(Tab::Images, ""),
            ["Playa.png", "playa-2.jpg", "sol.png", "logo.svg"]
        );
        assert_eq!(names(Tab::Images, " PLAYA "), ["Playa.png", "playa-2.jpg"]);
        assert_eq!(names(Tab::Designs, ""), ["perro.pdf", "logo.svg"]);
        assert_eq!(names(Tab::Designs, "per"), ["perro.pdf"]);
        assert!(names(Tab::Designs, "sol").is_empty());
    }

    // S68-06: the details read a date and a size the way people do.
    #[test]
    fn s68_06_date_and_size() {
        assert_eq!(date(0), "01/01/1970");
        assert_eq!(date(1_795_219_200), "21/11/2026");
        assert_eq!(bytes(3_051_356), "2.91 MB");
        assert_eq!(bytes(2048), "2 KB");
        assert_eq!(bytes(10), "1 KB");
    }
}
