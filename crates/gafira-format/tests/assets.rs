//! Spec 026: embedded assets travel in `assets/<blake3-hex>.<ext>`.

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use gafira_assets::FsStore;
use gafira_doc::{AssetRef, ContentHash, Document, ProjectId};
use gafira_format::{Limits, read_gaf, write_gaf};
use gafira_ports::repository::{LoadedProject, RepositoryError};
use gafira_ports::store::ContentStore;
use serde_json::{Value, json};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// A store root under the system temporary folder, removed on drop.
struct Root(PathBuf);

impl Root {
    fn new(case: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("gafira-format-{case}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        Self(path)
    }

    fn store(&self) -> FsStore {
        FsStore::new(&self.0)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for entry in entries {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            found.extend(files_under(&path));
        } else {
            found.push(path);
        }
    }
    found
}

fn clip() -> Vec<u8> {
    (0..5000_u32).map(|i| (i * 7 % 256) as u8).collect()
}

fn wave() -> Vec<u8> {
    b"RIFF....WAVEfmt ".repeat(200)
}

fn hex(hash: ContentHash) -> String {
    AssetRef::new(hash).hex()
}

/// The timeline golden with its embedded clip renamed to the real hash of
/// [`clip`], an embedded `score-take2.wav` and the linked `score.wav`.
fn document(clip_hash: ContentHash, wave_hash: ContentHash) -> Document {
    let text = include_str!("../../gafira-doc/tests/golden/timeline.json").replace(
        "abababababababababababababababababababababababababababababababab",
        &hex(clip_hash),
    );
    let mut value: Value = serde_json::from_str(&text).expect("json");
    value["assets"][hex(wave_hash)] = json!({ "name": "Score-Take2.WAV", "storage": "embedded" });
    Document::from_json(&serde_json::to_vec(&value).expect("json"))
        .expect("the golden loads")
        .with_project_id(ProjectId::from_u128(0xfeed))
}

/// A store holding both embedded contents, and the document naming them.
fn filled(root: &Root) -> (FsStore, Document, ContentHash, ContentHash) {
    let store = root.store();
    let clip_hash = store.put(&mut clip().as_slice()).expect("puts the clip");
    let wave_hash = store.put(&mut wave().as_slice()).expect("puts the wave");
    let document = document(clip_hash, wave_hash);
    (store, document, clip_hash, wave_hash)
}

fn gaf(document: &Document, store: &dyn ContentStore) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    write_gaf(&mut bytes, document, &[], store).expect("writes");
    bytes.into_inner()
}

fn read(bytes: Vec<u8>, store: &dyn ContentStore) -> Result<LoadedProject, RepositoryError> {
    read_gaf(Cursor::new(bytes), Limits::default(), store)
}

/// The entries of `bytes` in order, each with its method and its data.
fn unpacked(bytes: &[u8]) -> Vec<(String, CompressionMethod, Vec<u8>)> {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("a zip");
    (0..zip.len())
        .map(|index| {
            let mut file = zip.by_index(index).expect("an entry");
            let mut data = Vec::new();
            file.read_to_end(&mut data).expect("bytes");
            (file.name().to_owned(), file.compression(), data)
        })
        .collect()
}

fn packed(parts: &[(String, CompressionMethod, Vec<u8>)]) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, method, data) in parts {
        zip.start_file(
            name.as_str(),
            SimpleFileOptions::default().compression_method(*method),
        )
        .expect("an entry");
        zip.write_all(data).expect("bytes");
    }
    zip.finish().expect("a zip").into_inner()
}

fn corrupt(result: Result<LoadedProject, RepositoryError>) -> bool {
    matches!(result, Err(RepositoryError::Corrupt { .. }))
}

#[test]
fn s26_05_saving_writes_the_embedded_assets_with_their_bytes() {
    let root = Root::new("write");
    let (store, document, clip_hash, wave_hash) = filled(&root);
    let bytes = gaf(&document, &store);
    let parts = unpacked(&bytes);
    let assets: Vec<_> = parts
        .iter()
        .filter(|(name, _, _)| name.starts_with("assets/"))
        .collect();
    let clip_name = format!("assets/{}.mp4", hex(clip_hash));
    let wave_name = format!("assets/{}.wav", hex(wave_hash));
    assert_eq!(assets.len(), 2, "the linked score has no entry");
    let find = |name: &str| {
        assets
            .iter()
            .find(|(n, _, _)| n == name)
            .unwrap_or_else(|| panic!("{name} in {parts:?}"))
    };
    let (_, method, data) = find(&clip_name);
    assert_eq!(
        *method,
        CompressionMethod::Stored,
        "mp4 is already compressed"
    );
    assert_eq!(data, &clip());
    let (_, method, data) = find(&wave_name);
    assert_eq!(*method, CompressionMethod::Deflated, "wav is not");
    assert_eq!(data, &wave());

    let manifest: Value = serde_json::from_slice(
        &parts
            .iter()
            .find(|(n, _, _)| n == "manifest.json")
            .expect("a manifest")
            .2,
    )
    .expect("json");
    let table = &manifest["assets"];
    assert_eq!(table[hex(clip_hash)]["name"], "clip.mp4");
    assert_eq!(table[hex(clip_hash)]["bytes"], clip().len());
    assert_eq!(table[hex(wave_hash)]["bytes"], wave().len());
    let linked = &table["2222222222222222222222222222222222222222222222222222222222222222"];
    assert_eq!(linked["name"], "score.wav");
    assert!(linked.get("bytes").is_none(), "linked bytes live outside");
}

#[test]
fn s26_06_an_embedded_asset_round_trips_into_a_fresh_store() {
    let root = Root::new("trip");
    let (store, document, clip_hash, wave_hash) = filled(&root);
    let bytes = gaf(&document, &store);
    assert_eq!(bytes, gaf(&document, &store), "deterministic");

    let fresh = Root::new("trip-fresh");
    let fresh_store = fresh.store();
    let loaded = read(bytes, &fresh_store).expect("reads");
    assert_eq!(loaded.document, document);
    for (hash, expected) in [(clip_hash, clip()), (wave_hash, wave())] {
        let mut back = Vec::new();
        fresh_store
            .open(hash)
            .expect("in the fresh store")
            .read_to_end(&mut back)
            .expect("reads");
        assert_eq!(back, expected);
    }
}

#[test]
fn s26_07_tampered_asset_bytes_are_corrupt_and_stay_out_of_the_store() {
    let root = Root::new("tamper");
    let (store, document, clip_hash, _) = filled(&root);
    let mut parts = unpacked(&gaf(&document, &store));
    let name = format!("assets/{}.mp4", hex(clip_hash));
    let slot = parts
        .iter_mut()
        .find(|(n, _, _)| *n == name)
        .expect("the clip entry");
    slot.2[10] ^= 0xFF;

    let fresh = Root::new("tamper-fresh");
    assert!(corrupt(read(packed(&parts), &fresh.store())));
    assert_eq!(files_under(&fresh.0), Vec::<PathBuf>::new());
}

#[test]
fn s26_08_an_embedded_asset_missing_from_the_file_is_corrupt() {
    let root = Root::new("missing");
    let (store, document, clip_hash, _) = filled(&root);
    let name = format!("assets/{}.mp4", hex(clip_hash));
    let parts: Vec<_> = unpacked(&gaf(&document, &store))
        .into_iter()
        .filter(|(n, _, _)| *n != name)
        .collect();
    let fresh = Root::new("missing-fresh");
    assert!(corrupt(read(packed(&parts), &fresh.store())));
}

#[test]
fn s26_09_malformed_asset_names_are_corrupt() {
    let root = Root::new("names");
    let (store, document, clip_hash, _) = filled(&root);
    let parts = unpacked(&gaf(&document, &store));
    let good = hex(clip_hash);
    for bad in [
        "assets/abcd.png".to_owned(),
        format!("assets/{}.mp4", good.to_uppercase()),
        format!("assets/{good}"),
        format!("assets/{good}."),
        format!("assets/{good}.m p4"),
        format!("assets/x/{good}.mp4"),
    ] {
        let mut with = parts.clone();
        with.push((bad.clone(), CompressionMethod::Stored, clip()));
        let fresh = Root::new("names-fresh");
        assert!(corrupt(read(packed(&with), &fresh.store())), "{bad}");
    }
}

#[test]
fn s26_10_saving_without_the_bytes_in_the_store_fails() {
    let root = Root::new("absent");
    let (_, document, _, _) = filled(&root);
    let empty = Root::new("absent-empty");
    let mut sink = Cursor::new(Vec::new());
    assert!(matches!(
        write_gaf(&mut sink, &document, &[], &empty.store()),
        Err(RepositoryError::Io(_))
    ));
}
