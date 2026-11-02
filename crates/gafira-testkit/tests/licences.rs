//! ARQ-115 and ARQ-75 (partial file): every test font has a licence entry in
//! `assets/LICENSES.toml`, wherever it lives, and the splash image has its own.
//!
//! Integration tests may read files: this crate has no `clippy.toml`.

use std::fs;
use std::path::{Path, PathBuf};

use gafira_testkit::fonts;

const FONT_EXTENSIONS: [&str; 5] = ["ttf", "otf", "ttc", "woff", "woff2"];

/// One `[[asset]]` table of the licence file, as `key = "value"` pairs.
#[derive(Debug, Default)]
struct Entry {
    fields: Vec<(String, String)>,
}

impl Entry {
    fn get(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A line parser for the subset of TOML the licence file uses.
fn parse_entries(text: &str) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    for line in text.lines().map(str::trim) {
        if line == "[[asset]]" {
            entries.push(Entry::default());
        } else if let (Some(entry), Some((key, value))) = (entries.last_mut(), line.split_once('='))
        {
            let value = value.trim().trim_matches('"').to_string();
            let key = key.trim().trim_matches('"').to_string();
            entry.fields.push((key, value));
        }
    }
    entries
}

fn read_entries() -> Vec<Entry> {
    // A missing file is "no entries", so every font is reported as unlicensed.
    let text = fs::read_to_string(repo_root().join("assets/LICENSES.toml")).unwrap_or_default();
    parse_entries(&text)
}

/// Font files under `dir`, as repository-relative paths with `/`.
fn font_files(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(read) = fs::read_dir(dir) else { return };
    for item in read.flatten() {
        let path = item.path();
        let name = item.file_name();
        if path.is_dir() {
            if name != "target" && name != ".git" {
                font_files(root, &path, out);
            }
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| FONT_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        {
            let rel = path.strip_prefix(root).unwrap_or(&path);
            let parts: Vec<_> = rel.iter().map(|p| p.to_string_lossy()).collect();
            out.push(parts.join("/"));
        }
    }
}

/// The fonts that have no entry with that `path`.
fn unlicensed(fonts: &[String], entries: &[Entry]) -> Vec<String> {
    fonts
        .iter()
        .filter(|f| !entries.iter().any(|e| e.get("path") == Some(f.as_str())))
        .cloned()
        .collect()
}

fn found_fonts() -> Vec<String> {
    let root = repo_root();
    let mut found = Vec::new();
    font_files(&root, &root, &mut found);
    found.sort();
    found
}

#[test]
fn arq_115_every_font_file_in_the_repository_has_a_licence_entry() {
    let found = found_fonts();
    assert!(
        found.len() >= 4,
        "the walk must find the test fonts: {found:?}"
    );
    assert_eq!(
        unlicensed(&found, &read_entries()),
        Vec::<String>::new(),
        "fonts without a licence entry"
    );
}

#[test]
fn arq_115_the_check_names_a_font_that_has_no_entry() {
    let entries = parse_entries("[[asset]]\npath = \"assets/fonts/test/A.ttf\"\n");
    let fonts = ["assets/fonts/test/A.ttf".to_string(), "x/B.otf".to_string()];
    assert_eq!(unlicensed(&fonts, &entries), vec!["x/B.otf".to_string()]);
}

#[test]
fn arq_115_embedded_fonts_are_the_files_on_disk() {
    let found = found_fonts();
    for font in fonts::ALL {
        assert!(
            found.iter().any(|f| f == font.path),
            "{} is not on disk",
            font.path
        );
        let disk = fs::read(repo_root().join(font.path)).expect("font file");
        assert_eq!(disk, font.bytes, "{}", font.path);
    }
    assert_eq!(
        found.len(),
        fonts::ALL.len(),
        "a font on disk is not exposed: {found:?}"
    );
}

#[test]
fn arq_115_font_entries_record_licence_upstream_commit_and_subset_command() {
    let entries = read_entries();
    for font in fonts::ALL {
        let entry = entries
            .iter()
            .find(|e| e.get("path") == Some(font.path))
            .unwrap_or_else(|| panic!("no entry for {}", font.path));
        assert_eq!(entry.get("license"), Some("OFL-1.1"), "{}", font.path);
        for key in ["upstream", "commit", "subset_command", "license_file"] {
            assert!(
                entry.get(key).is_some_and(|v| !v.is_empty()),
                "{} lacks {key}",
                font.path
            );
        }
        let license_file = entry.get("license_file").unwrap_or_default();
        assert!(
            repo_root().join(license_file).is_file(),
            "{license_file} is missing"
        );
    }
}

#[test]
fn arq_75_splash_image_has_an_entry_with_the_seven_d09_fields() {
    let entries = read_entries();
    let entry = entries
        .iter()
        .find(|e| e.get("path") == Some("assets/splash_screen.jpg"))
        .expect("no entry for assets/splash_screen.jpg");
    for key in [
        "origin",
        "tool",
        "license_basis",
        "ai_notice",
        "copyright_note",
        "provenance",
        "similarity_check",
    ] {
        assert!(
            entry.get(key).is_some_and(|v| !v.is_empty()),
            "splash lacks {key}"
        );
    }
    // The similarity check is still pending (item 6 of 00), and the file says so.
    assert!(
        entry
            .get("similarity_check")
            .unwrap_or_default()
            .contains("VERIFICAR")
    );
}

#[test]
fn arq_75_every_entry_points_to_a_file_that_exists() {
    let entries = read_entries();
    assert!(
        entries.len() >= 5,
        "four fonts and the splash image: {entries:?}"
    );
    for entry in entries {
        let path = entry.get("path").expect("entry without path");
        assert!(repo_root().join(path).is_file(), "{path} does not exist");
    }
}

/// The fields 05 FV-38 asks of every entry.
const FV_38_FIELDS: [&str; 7] = [
    "path",
    "hash",
    "license",
    "autoría",
    "origin",
    "exige atribución",
    "modificado",
];

/// The FV-38 fields the entry lacks or leaves empty.
fn missing_fields(entry: &Entry) -> Vec<&'static str> {
    FV_38_FIELDS
        .into_iter()
        .filter(|k| entry.get(k).is_none_or(str::is_empty))
        .collect()
}

#[test]
fn arq_75_every_entry_has_the_fv_38_fields_and_a_blake3_hash() {
    let entries = read_entries();
    assert!(entries.len() >= 5);
    for entry in &entries {
        let path = entry.get("path").unwrap_or("?");
        assert_eq!(missing_fields(entry), Vec::<&str>::new(), "{path}");
        let hash = entry.get("hash").unwrap_or_default();
        assert!(
            hash.len() == 64 && hash.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
            "{path}: hash is not 64 lowercase hex digits"
        );
    }
}

#[test]
fn arq_75_the_check_fails_when_a_field_is_removed_or_empty() {
    for removed in FV_38_FIELDS {
        let mut text = String::from("[[asset]]\n");
        for key in FV_38_FIELDS.into_iter().filter(|k| *k != removed) {
            text.push_str(&format!("\"{key}\" = \"x\"\n"));
        }
        let entries = parse_entries(&text);
        assert_eq!(missing_fields(&entries[0]), vec![removed]);
    }
    let empty = parse_entries("[[asset]]\npath = \"\"\n");
    assert!(missing_fields(&empty[0]).contains(&"path"));
}

#[test]
fn arq_75_test_fonts_are_modified_and_record_the_year_of_their_own_name_table() {
    // Year of name ID 0 in each subset, which differs from upstream's OFL.txt for two fonts.
    let table = [
        ("Inter-test.ttf", "Copyright 2016 The Inter Project Authors"),
        (
            "Manrope-test.ttf",
            "Copyright 2019 The Manrope Project Authors",
        ),
        (
            "NotoSansArabic-test.ttf",
            "Copyright 2022 The Noto Project Authors",
        ),
        ("NotoEmoji-test.ttf", "Copyright 2013 Google LLC"),
    ];
    let entries = read_entries();
    for (file, author) in table {
        let path = format!("assets/fonts/test/{file}");
        let entry = entries
            .iter()
            .find(|e| e.get("path") == Some(path.as_str()))
            .unwrap_or_else(|| panic!("no entry for {path}"));
        assert!(
            entry.get("autoría").unwrap_or_default().starts_with(author),
            "{path}"
        );
        assert_eq!(entry.get("modificado"), Some("true"), "{path}");
        assert_eq!(entry.get("exige atribución"), Some("true"), "{path}");
    }
}

#[test]
fn arq_75_the_splash_licence_is_propia_and_the_open_provenance_item_is_not_stated_as_done() {
    let entries = read_entries();
    let entry = entries
        .iter()
        .find(|e| e.get("path") == Some("assets/splash_screen.jpg"))
        .expect("no entry for the splash");
    assert_eq!(entry.get("license"), Some("Propia"));
    // Item 6 (b) of 00-decisiones is VERIFICAR: the register must not say the original is kept.
    let provenance = entry.get("provenance").unwrap_or_default();
    assert!(provenance.contains("VERIFICAR"), "{provenance}");
    assert!(!provenance.contains("is kept"), "{provenance}");
}
