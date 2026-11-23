//! Spec 031: versions inside the `.gaf` (02 §7.1 «Versiones», D-67, D-68).

use std::collections::BTreeMap;
use std::io::{self, Cursor, Read, Write};
use std::sync::{Arc, Mutex};

use gafira_doc::{AssetRef, ContentHash, Document, ProjectId};
use gafira_format::{Limits, read_gaf, write_gaf};
use gafira_ports::common::UnixTime;
use gafira_ports::repository::{
    LoadedProject, RepositoryError, Version, VersionId, VersionInfo, VersionKind,
};
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::ReadSeek;
use serde_json::{Value, json};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// Contents kept in memory.
#[derive(Default)]
struct MemoryStore(Mutex<BTreeMap<ContentHash, Vec<u8>>>);

impl ContentStore for MemoryStore {
    fn put(&self, src: &mut dyn io::Read) -> Result<ContentHash, StoreError> {
        let mut bytes = Vec::new();
        src.read_to_end(&mut bytes).map_err(StoreError::Io)?;
        let hash = ContentHash(*blake3::hash(&bytes).as_bytes());
        self.0.lock().expect("a lock").insert(hash, bytes);
        Ok(hash)
    }

    fn open(&self, hash: ContentHash) -> Result<Box<dyn ReadSeek + Send>, StoreError> {
        let bytes = self.0.lock().expect("a lock").get(&hash).cloned();
        let bytes = bytes.ok_or(StoreError::NotFound { hash })?;
        Ok(Box::new(Cursor::new(bytes)))
    }

    fn contains(&self, hash: ContentHash) -> Result<bool, StoreError> {
        Ok(self.0.lock().expect("a lock").contains_key(&hash))
    }
}

/// The clip the document and both versions embed.
const CLIP: &[u8] = b"not really an mp4";
/// An image only the automatic version embeds.
const EXTRA: &[u8] = b"not really a png";

fn hex(bytes: &[u8]) -> String {
    AssetRef::new(ContentHash(*blake3::hash(bytes).as_bytes())).hex()
}

fn store() -> MemoryStore {
    let store = MemoryStore::default();
    store.put(&mut &CLIP[..]).expect("puts");
    store.put(&mut &EXTRA[..]).expect("puts");
    store
}

/// The timeline golden (one page, one sequence) with the clip embedded,
/// `edit` applied to its JSON.
fn golden(edit: impl FnOnce(&mut Value)) -> Document {
    let text = include_str!("../../gafira-doc/tests/golden/timeline.json").replace(
        "abababababababababababababababababababababababababababababababab",
        &hex(CLIP),
    );
    let mut value: Value = serde_json::from_str(&text).expect("json");
    edit(&mut value);
    Document::from_json(&serde_json::to_vec(&value).expect("json"))
        .expect("the golden loads")
        .with_project_id(ProjectId::from_u128(0xfeed))
}

fn version(id: u64, name: &str, kind: VersionKind, document: Document) -> Version {
    Version {
        info: VersionInfo {
            id: VersionId(id),
            name: name.to_owned(),
            kind,
            created: UnixTime(1_700_000_000 + 60 * id as i64),
            app: "0.1.0".to_owned(),
            schema: document.schema_version(),
        },
        document: Arc::new(document),
    }
}

/// The document, a named version of it and an automatic one that also
/// embeds [`EXTRA`].
fn project() -> (Document, Vec<Version>) {
    let document = golden(|_| {});
    let named = version(1, "Primera idea", VersionKind::Named, golden(|_| {}));
    let auto = version(
        2,
        "",
        VersionKind::Auto,
        golden(|value| {
            value["dpi"] = json!(300);
            value["assets"][hex(EXTRA)] = json!({ "name": "extra.png", "storage": "embedded" });
        }),
    );
    (document, vec![named, auto])
}

fn gaf(document: &Document, versions: &[Version]) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    write_gaf(&mut bytes, document, versions, None, &store()).expect("writes");
    bytes.into_inner()
}

fn read(bytes: Vec<u8>, store: &dyn ContentStore) -> Result<LoadedProject, RepositoryError> {
    read_gaf(Cursor::new(bytes), Limits::default(), store)
}

fn unpacked(bytes: &[u8]) -> Vec<(String, CompressionMethod, Vec<u8>)> {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("a zip");
    (0..zip.len())
        .map(|index| {
            let mut file = zip.by_index(index).expect("an entry");
            let mut data = Vec::new();
            file.read_to_end(&mut data).expect("bytes");
            (
                file.name().expect("name").into_owned(),
                file.compression(),
                data,
            )
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

fn entry(bytes: &[u8], name: &str) -> Value {
    let parts = unpacked(bytes);
    let (_, _, data) = parts
        .iter()
        .find(|(entry, _, _)| entry == name)
        .unwrap_or_else(|| panic!("{name} is missing"));
    serde_json::from_slice(data).expect("json")
}

/// The versioned file with the JSON entry `name` changed by `edit`.
fn tampered(name: &str, edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let (document, versions) = project();
    let mut parts = unpacked(&gaf(&document, &versions));
    let part = parts
        .iter_mut()
        .find(|(entry, _, _)| entry == name)
        .unwrap_or_else(|| panic!("{name} is missing"));
    let mut value: Value = serde_json::from_slice(&part.2).expect("json");
    edit(&mut value);
    part.2 = serde_json::to_vec(&value).expect("json");
    packed(&parts)
}

/// The versioned file without the entry `name`.
fn without(name: &str) -> Vec<u8> {
    let (document, versions) = project();
    let mut parts = unpacked(&gaf(&document, &versions));
    parts.retain(|(entry, _, _)| entry != name);
    packed(&parts)
}

fn corrupt(bytes: Vec<u8>) -> bool {
    matches!(
        read(bytes, &MemoryStore::default()),
        Err(RepositoryError::Corrupt { .. })
    )
}

#[test]
fn s31_02_each_version_has_its_folder_and_its_index_entry() {
    let (document, versions) = project();
    let bytes = gaf(&document, &versions);
    let names: Vec<String> = unpacked(&bytes).into_iter().map(|(n, _, _)| n).collect();
    // The assets come last, once per hash and in hash order.
    let mut assets = [
        format!("assets/{}.mp4", hex(CLIP)),
        format!("assets/{}.png", hex(EXTRA)),
    ];
    assets.sort();
    assert_eq!(&names[12..], assets);
    assert_eq!(
        names[..12],
        [
            "mimetype".to_owned(),
            "manifest.json".to_owned(),
            "document.json".to_owned(),
            "pages/1.json".to_owned(),
            "sequences/3.json".to_owned(),
            "versions/index.json".to_owned(),
            "versions/1/document.json".to_owned(),
            "versions/1/pages/1.json".to_owned(),
            "versions/1/sequences/3.json".to_owned(),
            "versions/2/document.json".to_owned(),
            "versions/2/pages/1.json".to_owned(),
            "versions/2/sequences/3.json".to_owned(),
        ]
    );
    assert_eq!(entry(&bytes, "manifest.json")["versions"], json!([1, 2]));
    assert_eq!(
        entry(&bytes, "versions/index.json"),
        json!([
            {
                "id": 1, "name": "Primera idea", "kind": "named", "created": 1_700_000_060,
                "app": "0.1.0", "schema": 1, "pages": [1], "sequences": [3]
            },
            {
                "id": 2, "name": "", "kind": "auto", "created": 1_700_000_120,
                "app": "0.1.0", "schema": 1, "pages": [1], "sequences": [3]
            }
        ])
    );
    let version_json = entry(&bytes, "versions/2/document.json");
    assert!(version_json.get("pages").is_none() && version_json.get("sequences").is_none());
    assert_eq!(version_json["dpi"], 300);
}

#[test]
fn s31_02_versions_round_trip_and_write_the_same_bytes_twice() {
    let (document, versions) = project();
    let bytes = gaf(&document, &versions);
    assert_eq!(bytes, gaf(&document, &versions), "deterministic");
    let loaded = read(bytes, &MemoryStore::default()).expect("reads");
    assert_eq!(loaded.document, document);
    assert_eq!(loaded.versions, versions);
}

#[test]
fn s31_02_without_versions_nothing_under_versions_is_written() {
    let (document, _) = project();
    let bytes = gaf(&document, &[]);
    assert!(
        unpacked(&bytes)
            .iter()
            .all(|(name, _, _)| !name.starts_with("versions/"))
    );
    assert_eq!(entry(&bytes, "manifest.json")["versions"], json!([]));
    assert!(
        read(bytes, &MemoryStore::default())
            .expect("reads")
            .versions
            .is_empty()
    );
}

#[test]
fn s31_03_assets_are_shared_by_hash_and_a_version_only_one_is_kept() {
    let (document, versions) = project();
    let bytes = gaf(&document, &versions);
    let clips = unpacked(&bytes)
        .iter()
        .filter(|(name, _, _)| name.contains(&hex(CLIP)))
        .count();
    assert_eq!(clips, 1, "the clip of the document and both versions");
    let fresh = MemoryStore::default();
    read(bytes, &fresh).expect("reads");
    let extra = ContentHash(*blake3::hash(EXTRA).as_bytes());
    assert!(fresh.contains(extra).expect("asks"), "the version's image");
    assert_eq!(
        fresh
            .0
            .lock()
            .expect("a lock")
            .get(&extra)
            .map(Vec::as_slice),
        Some(EXTRA)
    );
}

#[test]
fn s31_04_an_index_that_disagrees_with_the_manifest_is_corrupt() {
    assert!(corrupt(
        tampered("manifest.json", |m| m["versions"] = json!([1]))
    ));
    assert!(corrupt(
        tampered("manifest.json", |m| m["versions"] = json!([2, 1]))
    ));
    assert!(corrupt(
        tampered("manifest.json", |m| m["versions"] = json!(["1", 2]))
    ));
    assert!(corrupt(tampered("versions/index.json", |i| *i = json!({}))));
    assert!(corrupt(without("versions/index.json")));
}

#[test]
fn s31_04_a_repeated_version_id_is_corrupt() {
    let bytes = {
        let (document, versions) = project();
        let mut parts = unpacked(&gaf(&document, &versions));
        for (name, _, data) in &mut parts {
            let mut value: Value = serde_json::from_slice(data).unwrap_or(Value::Null);
            if name == "manifest.json" {
                value["versions"] = json!([1, 1]);
            } else if name == "versions/index.json" {
                value[1]["id"] = json!(1);
            } else {
                continue;
            }
            *data = serde_json::to_vec(&value).expect("json");
        }
        packed(&parts)
    };
    assert!(corrupt(bytes));
}

#[test]
fn s31_04_malformed_index_entries_are_corrupt() {
    assert!(corrupt(tampered("versions/index.json", |i| {
        i[0]["kind"] = json!("manual");
    })));
    assert!(corrupt(tampered("versions/index.json", |i| {
        i[0]["name"] = json!("x".repeat(257));
    })));
    assert!(corrupt(tampered("versions/index.json", |i| {
        i[0]["created"] = json!("yesterday");
    })));
    assert!(corrupt(tampered("versions/index.json", |i| {
        i[0]["schema"] = json!(Document::SCHEMA_VERSION + 1);
    })));
    assert!(corrupt(tampered("versions/index.json", |i| {
        i[0]["schema"] = json!(0);
    })));
    assert!(corrupt(tampered("versions/index.json", |i| {
        i[0]["pages"] = json!([9]);
    })));
    // A name of exactly 256 characters is fine.
    let fine = tampered("versions/index.json", |i| {
        i[0]["name"] = json!("é".repeat(256));
    });
    let loaded = read(fine, &MemoryStore::default()).expect("reads");
    assert_eq!(loaded.versions[0].info.name.chars().count(), 256);
}

#[test]
fn s31_04_version_parts_that_do_not_hold_their_ids_are_corrupt() {
    assert!(corrupt(tampered("versions/1/pages/1.json", |p| {
        p["id"] = json!(9);
    })));
    assert!(corrupt(tampered("versions/2/sequences/3.json", |s| {
        s["id"] = json!(9);
    })));
    assert!(corrupt(without("versions/2/document.json")));
    assert!(corrupt(tampered("versions/1/document.json", |d| {
        *d = json!([]);
    })));
}

#[test]
fn s31_04_a_version_asset_missing_from_the_file_is_corrupt() {
    assert!(corrupt(without(&format!("assets/{}.png", hex(EXTRA)))));
}
