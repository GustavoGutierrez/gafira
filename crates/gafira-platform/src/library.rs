//! The files imported into Gafira, for the Biblioteca panel (spec 068): a
//! JSON list in the data folder, `<data_dir>/library.json`, newest first,
//! written atomically.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// How many files the list keeps.
pub const MAX_IMPORTED: usize = 200;

/// A file imported into Gafira (spec 068).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedFile {
    pub path: PathBuf,
    /// Imported as a design (the outline of a frame) rather than as an
    /// image: the tab it shows in. An SVG can be both.
    pub design: bool,
    /// When it was last imported, in seconds since the Unix epoch.
    pub added: i64,
}

#[derive(Default, Serialize, Deserialize)]
struct LibraryFile {
    /// The first form of the list: paths alone.
    #[serde(default, skip_serializing)]
    files: Vec<PathBuf>,
    #[serde(default)]
    entries: Vec<ImportedFile>,
}

/// Whether `path` is a design when nothing else says: SVG and PDF.
fn design_by_name(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "svg" | "pdf"))
}

/// The imported files in a JSON file.
pub struct ImportedFiles {
    path: PathBuf,
}

impl ImportedFiles {
    #[must_use]
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    /// The list, newest first, without repeats. A missing or unreadable
    /// file reads as empty: the list is a convenience, never a failure.
    #[must_use]
    pub fn load(&self) -> Vec<ImportedFile> {
        fs::read(&self.path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<LibraryFile>(&bytes).ok())
            .map(|file| {
                let mut list = file.entries;
                list.extend(file.files.into_iter().map(|path| ImportedFile {
                    design: design_by_name(&path),
                    path,
                    added: 0,
                }));
                tidy(list)
            })
            .unwrap_or_default()
    }

    /// Put `file` in front of the list, as a design or an image, now.
    ///
    /// # Errors
    /// When the list cannot be written.
    pub fn record(&self, file: &Path, design: bool) -> io::Result<()> {
        let added = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX));
        let mut list = vec![ImportedFile {
            path: file.to_path_buf(),
            design,
            added,
        }];
        list.extend(self.load());
        self.write(tidy(list))
    }

    /// Take these entries (path and tab) out of the list; the files stay
    /// on the disk.
    ///
    /// # Errors
    /// When the list cannot be written.
    pub fn remove(&self, gone: &[(PathBuf, bool)]) -> io::Result<()> {
        let mut list = self.load();
        list.retain(|entry| !gone.contains(&(entry.path.clone(), entry.design)));
        self.write(list)
    }

    /// The list without the files that are gone; it is rewritten when one
    /// went, so a deleted file never shows again (spec 068).
    ///
    /// # Errors
    /// When the pruned list cannot be written.
    pub fn existing(&self) -> io::Result<Vec<ImportedFile>> {
        let list = self.load();
        let before = list.len();
        let kept: Vec<ImportedFile> = list.into_iter().filter(|e| e.path.is_file()).collect();
        if kept.len() != before {
            self.write(kept.clone())?;
        }
        Ok(kept)
    }

    fn write(&self, entries: Vec<ImportedFile>) -> io::Result<()> {
        let file = LibraryFile {
            files: Vec::new(),
            entries,
        };
        let text = serde_json::to_vec_pretty(&file).map_err(io::Error::other)?;
        if let Some(folder) = self.path.parent() {
            fs::create_dir_all(folder)?;
        }
        let mut tmp = self.path.clone().into_os_string();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        fs::write(&tmp, text)?;
        fs::rename(&tmp, &self.path)
    }
}

/// `list` without repeats of a path in a tab (the first stays), cut to
/// [`MAX_IMPORTED`].
fn tidy(list: Vec<ImportedFile>) -> Vec<ImportedFile> {
    let mut out: Vec<ImportedFile> = Vec::new();
    for entry in list {
        if out.len() == MAX_IMPORTED {
            break;
        }
        if !out
            .iter()
            .any(|kept| kept.path == entry.path && kept.design == entry.design)
        {
            out.push(entry);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("gafira-library-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a folder");
        dir
    }

    fn paths(list: &[ImportedFile]) -> Vec<(PathBuf, bool)> {
        list.iter().map(|e| (e.path.clone(), e.design)).collect()
    }

    // S68-01: the newest import goes first; importing again moves it to the
    // front once; an SVG can be in both tabs; a deleted file leaves the list.
    #[test]
    fn s68_01_newest_first_once_and_only_existing_files() {
        let dir = scratch("order");
        let (a, b) = (dir.join("a.png"), dir.join("b.svg"));
        fs::write(&a, b"a").expect("a");
        fs::write(&b, b"b").expect("b");
        let files = ImportedFiles::at(dir.join("library.json"));
        assert!(files.load().is_empty(), "no file yet");
        files.record(&a, false).expect("records");
        files.record(&b, true).expect("records");
        files.record(&b, false).expect("records");
        files.record(&a, false).expect("records");
        assert_eq!(
            paths(&files.load()),
            [(a.clone(), false), (b.clone(), false), (b.clone(), true)]
        );
        assert!(files.load()[0].added > 0, "when it came in");
        files.remove(&[(b.clone(), true)]).expect("removes");
        assert_eq!(
            paths(&files.load()),
            [(a.clone(), false), (b.clone(), false)]
        );
        assert!(b.is_file(), "the file stays on the disk");
        fs::remove_file(&b).expect("deleted");
        assert_eq!(
            paths(&files.existing().expect("prunes")),
            [(a.clone(), false)]
        );
        fs::remove_dir_all(dir).expect("clean up");
    }

    // S68-01: a broken list reads as empty, the first form (paths alone) is
    // still read, and the list never grows past its limit.
    #[test]
    fn s68_01_broken_old_or_long_lists() {
        let dir = scratch("limit");
        fs::write(dir.join("library.json"), b"{ not json").expect("broken");
        let files = ImportedFiles::at(dir.join("library.json"));
        assert!(files.load().is_empty());
        fs::write(
            dir.join("library.json"),
            br#"{"files":["/x/a.png","/x/m.pdf"]}"#,
        )
        .expect("old");
        assert_eq!(
            paths(&files.load()),
            [
                (PathBuf::from("/x/a.png"), false),
                (PathBuf::from("/x/m.pdf"), true)
            ]
        );
        let long: Vec<ImportedFile> = (0..MAX_IMPORTED + 5)
            .map(|i| ImportedFile {
                path: PathBuf::from(format!("/x/{i}.png")),
                design: false,
                added: 0,
            })
            .collect();
        assert_eq!(tidy(long).len(), MAX_IMPORTED);
        fs::remove_dir_all(dir).expect("clean up");
    }
}
