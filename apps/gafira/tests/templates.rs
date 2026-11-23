//! Spec 091, S91-09: the templates the app carries open, are templates of a
//! known kind, carry a thumbnail and come in Spanish and in English.

use std::collections::BTreeSet;
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};

use gafira_doc::ContentHash;
use gafira_format::{Limits, read_gaf, read_template_info};
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::ReadSeek;

/// A store that holds nothing: the templates embed no asset.
struct Empty;

impl ContentStore for Empty {
    fn put(&self, _src: &mut dyn io::Read) -> Result<ContentHash, StoreError> {
        Err(StoreError::Io(io::Error::other("no asset expected")))
    }

    fn open(&self, hash: ContentHash) -> Result<Box<dyn ReadSeek + Send>, StoreError> {
        Err(StoreError::NotFound { hash })
    }

    fn contains(&self, _hash: ContentHash) -> Result<bool, StoreError> {
        Ok(false)
    }
}

fn gafs(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("the folder").flatten() {
        let path = entry.path();
        if path.is_dir() {
            gafs(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "gaf") {
            out.push(path);
        }
    }
}

#[test]
fn s91_09_the_bundled_templates_open_with_kind_thumbnail_and_both_languages() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/templates");
    let mut paths = Vec::new();
    gafs(&root, &mut paths);
    assert!(paths.len() >= 20, "{} templates", paths.len());
    let mut folders = BTreeSet::new();
    let mut designs: BTreeSet<(String, String)> = BTreeSet::new();
    for path in &paths {
        let bytes = std::fs::read(path).expect("reads");
        let info = read_template_info(Cursor::new(&bytes), Limits::default())
            .expect("a .gaf")
            .unwrap_or_else(|| panic!("{} is a template", path.display()));
        let tags = &info.meta.tags;
        assert!(
            tags.iter().any(|t| t.starts_with("size.")),
            "{} has a kind",
            path.display()
        );
        assert!(
            tags.iter().any(|t| t.starts_with("title:")),
            "{} has a name",
            path.display()
        );
        let lang = tags
            .iter()
            .find_map(|t| t.strip_prefix("lang:"))
            .unwrap_or_else(|| panic!("{} has a language", path.display()));
        assert!(
            info.thumbnail.is_some(),
            "{} has a thumbnail",
            path.display()
        );
        let loaded = read_gaf(Cursor::new(&bytes), Limits::default(), &Empty).expect("it opens");
        assert!(!loaded.document.pages().is_empty());
        let folder = path.parent().expect("a folder").to_path_buf();
        let stem = path.file_name().expect("a name").to_string_lossy();
        let design = stem.split('.').next().expect("a design").to_owned();
        // File names are English: lowercase words and hyphens.
        assert!(
            design.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
            "{design}"
        );
        designs.insert((format!("{}/{design}", folder.display()), lang.to_owned()));
        folders.insert(folder);
    }
    assert!(folders.len() >= 5, "{} categories", folders.len());
    let names: BTreeSet<&String> = designs.iter().map(|(d, _)| d).collect();
    for name in names {
        for lang in ["es", "en"] {
            assert!(
                designs.contains(&(name.clone(), lang.to_owned())),
                "{name} in {lang}"
            );
        }
    }
}
