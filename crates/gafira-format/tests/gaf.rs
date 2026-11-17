//! Spec 019: the `.gaf` container of D-68.

use std::collections::BTreeMap;
use std::io::{self, Cursor, Read, Write};
use std::sync::Mutex;

use gafira_doc::{AssetRef, ContentHash, Document, ProjectId};
use gafira_format::{FORMAT_VERSION, GafRepository, Limits, MIMETYPE, read_gaf, write_gaf};
use gafira_ports::repository::{ProjectRepository, RepositoryError};
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::ReadSeek;
use serde_json::{Value, json};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// Contents kept in memory, for the tests that need no disk.
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

/// The bytes of the embedded `clip.mp4` of the golden.
const CLIP: &[u8] = b"not really an mp4";

fn clip_hex() -> String {
    AssetRef::new(ContentHash(*blake3::hash(CLIP).as_bytes())).hex()
}

/// A store with the embedded clip in it.
fn store() -> MemoryStore {
    let store = MemoryStore::default();
    store.put(&mut &CLIP[..]).expect("puts");
    store
}

/// The timeline golden: a page, a sequence and two assets, which covers
/// every part of the layout. Its embedded clip is renamed to the hash of
/// [`CLIP`].
fn document() -> Document {
    let text = include_str!("../../gafira-doc/tests/golden/timeline.json").replace(
        "abababababababababababababababababababababababababababababababab",
        &clip_hex(),
    );
    Document::from_json(text.as_bytes())
        .expect("the golden loads")
        .with_project_id(ProjectId::from_u128(0xfeed))
}

fn gaf(document: &Document) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    write_gaf(&mut bytes, document, &[], None, &store()).expect("writes");
    bytes.into_inner()
}

fn entries(bytes: &[u8]) -> Vec<String> {
    let zip = ZipArchive::new(Cursor::new(bytes)).expect("a zip");
    zip.file_names().map(str::to_owned).collect::<Vec<_>>()
}

fn entry(bytes: &[u8], name: &str) -> Value {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("a zip");
    let mut text = String::new();
    zip.by_name(name)
        .expect(name)
        .read_to_string(&mut text)
        .expect("text");
    serde_json::from_str(&text).expect("json")
}

fn read(bytes: Vec<u8>) -> Result<gafira_ports::repository::LoadedProject, RepositoryError> {
    read_gaf(
        Cursor::new(bytes),
        Limits::default(),
        &MemoryStore::default(),
    )
}

/// A ZIP built entry by entry, for the hostile cases.
fn crafted(parts: &[(&str, CompressionMethod, &[u8])]) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, method, bytes) in parts {
        zip.start_file(
            *name,
            SimpleFileOptions::default().compression_method(*method),
        )
        .expect("an entry");
        zip.write_all(bytes).expect("bytes");
    }
    zip.finish().expect("a zip").into_inner()
}

/// The entries of a valid file, with `name` replaced by `with`.
fn tampered(name: &str, with: &Value) -> Vec<u8> {
    replaced(name, &serde_json::to_vec(with).expect("json"))
}

/// The entries of a valid file, with the bytes of `name` replaced.
fn replaced(name: &str, with: &[u8]) -> Vec<u8> {
    let bytes = gaf(&document());
    let mut parts: Vec<(String, Vec<u8>)> = Vec::new();
    let mut zip = ZipArchive::new(Cursor::new(&bytes)).expect("a zip");
    for index in 0..zip.len() {
        let mut file = zip.by_index(index).expect("an entry");
        let mut data = Vec::new();
        file.read_to_end(&mut data).expect("bytes");
        let data = if file.name() == name {
            with.to_vec()
        } else {
            data
        };
        parts.push((file.name().to_owned(), data));
    }
    let parts: Vec<(&str, CompressionMethod, &[u8])> = parts
        .iter()
        .map(|(n, d)| {
            let method = if n == "mimetype" {
                CompressionMethod::Stored
            } else {
                CompressionMethod::Deflated
            };
            (n.as_str(), method, d.as_slice())
        })
        .collect();
    crafted(&parts)
}

#[test]
fn s19_01_the_layout_is_the_one_of_d68() {
    let bytes = gaf(&document());
    assert_eq!(
        entries(&bytes),
        [
            "mimetype",
            "manifest.json",
            "document.json",
            "pages/1.json",
            "sequences/3.json",
            format!("assets/{}.mp4", clip_hex()).as_str(),
        ]
    );
    let mut zip = ZipArchive::new(Cursor::new(&bytes)).expect("a zip");
    let mut first = zip.by_index(0).expect("mimetype");
    assert_eq!(first.compression(), CompressionMethod::Stored);
    let mut text = String::new();
    first.read_to_string(&mut text).expect("text");
    assert_eq!(text, MIMETYPE);
    drop(first);

    let document_json = entry(&bytes, "document.json");
    assert!(document_json.get("pages").is_none() && document_json.get("sequences").is_none());
    assert_eq!(
        document_json["project_id"],
        "00000000-0000-0000-0000-00000000feed"
    );
    assert_eq!(entry(&bytes, "pages/1.json")["id"], 1);
}

#[test]
fn s19_02_the_manifest_carries_the_fields_of_d68_in_camel_case() {
    let manifest = entry(&gaf(&document()), "manifest.json");
    assert_eq!(manifest["format"], "gafira-project");
    assert_eq!(manifest["formatVersion"], FORMAT_VERSION);
    assert_eq!(manifest["schemaVersion"], 1);
    assert_eq!(
        manifest["projectId"],
        "00000000-0000-0000-0000-00000000feed"
    );
    assert_eq!(manifest["profile"], "portable-full");
    assert_eq!(manifest["packageCompleteness"], "complete");
    assert_eq!(
        manifest["features"],
        json!({ "required": [], "optional": [] })
    );
    assert_eq!(manifest["pages"], json!([1]));
    assert_eq!(manifest["sequences"], json!([3]));
    assert_eq!(manifest["generator"]["app"], "Gafira");
    let assets = manifest["assets"].as_object().expect("assets");
    assert_eq!(assets.len(), 2);
    assert!(assets.values().any(|a| a["name"] == "clip.mp4"));
}

#[test]
fn s19_03_a_document_round_trips_and_writes_the_same_bytes_twice() {
    let document = document();
    let bytes = gaf(&document);
    assert_eq!(bytes, gaf(&document), "deterministic");
    let loaded = read(bytes).expect("reads");
    assert_eq!(loaded.document, document);
    assert!(!loaded.read_only);
}

#[test]
fn s19_04_not_a_gaf_is_corrupt() {
    let manifest = serde_json::to_vec(&json!({})).expect("json");
    for (case, bytes) in [
        ("not a zip", b"hello".to_vec()),
        (
            "no mimetype first",
            crafted(&[("manifest.json", CompressionMethod::Stored, &manifest)]),
        ),
        (
            "compressed mimetype",
            crafted(&[("mimetype", CompressionMethod::Deflated, MIMETYPE.as_bytes())]),
        ),
        ("wrong mimetype", replaced("mimetype", b"application/zip")),
        (
            "mimetype with a tail",
            replaced("mimetype", format!("{MIMETYPE}\n").as_bytes()),
        ),
        (
            "no manifest",
            crafted(&[("mimetype", CompressionMethod::Stored, MIMETYPE.as_bytes())]),
        ),
    ] {
        assert!(
            matches!(read(bytes), Err(RepositoryError::Corrupt { .. })),
            "{case}"
        );
    }
}

#[test]
fn s19_05_unsafe_entry_names_are_refused() {
    for name in [
        "../evil.json",
        "/abs.json",
        "a/../../b.json",
        "c:\\x.json",
        "pages\\1.json",
        "a//b.json",
    ] {
        let bytes = crafted(&[
            ("mimetype", CompressionMethod::Stored, MIMETYPE.as_bytes()),
            (name, CompressionMethod::Stored, b"{}"),
        ]);
        let error = read(bytes).expect_err(name);
        assert!(
            matches!(&error, RepositoryError::Corrupt { detail } if detail.contains("unsafe")),
            "{name}: {error:?}"
        );
    }
}

#[test]
fn s19_06_a_zip_bomb_is_refused() {
    let zeros = vec![0_u8; 8 * 1024 * 1024];
    let bytes = crafted(&[
        ("mimetype", CompressionMethod::Stored, MIMETYPE.as_bytes()),
        ("pages/9.json", CompressionMethod::Deflated, &zeros),
    ]);
    let error = read(bytes).expect_err("a bomb");
    assert!(
        matches!(&error, RepositoryError::Corrupt { detail } if detail.contains("expands")),
        "{error:?}"
    );
}

#[test]
fn s19_07_parts_that_disagree_with_the_manifest_are_corrupt() {
    let mut manifest = entry(&gaf(&document()), "manifest.json");
    manifest["pages"] = json!([1, 42]);
    assert!(
        matches!(
            read(tampered("manifest.json", &manifest)),
            Err(RepositoryError::Corrupt { .. })
        ),
        "missing page"
    );

    let mut manifest = entry(&gaf(&document()), "manifest.json");
    manifest["projectId"] = json!("00000000-0000-0000-0000-000000000001");
    assert!(
        matches!(
            read(tampered("manifest.json", &manifest)),
            Err(RepositoryError::Corrupt { .. })
        ),
        "other project"
    );

    let mut page = entry(&gaf(&document()), "pages/1.json");
    page["id"] = json!(7);
    assert!(
        matches!(
            read(tampered("pages/1.json", &page)),
            Err(RepositoryError::Corrupt { .. })
        ),
        "other id"
    );

    let mut manifest = entry(&gaf(&document()), "manifest.json");
    manifest["format"] = json!("something-else");
    assert!(
        matches!(
            read(tampered("manifest.json", &manifest)),
            Err(RepositoryError::Corrupt { .. })
        ),
        "format"
    );
}

#[test]
fn s19_08_a_newer_container_or_an_unknown_capability_opens_read_only() {
    let mut manifest = entry(&gaf(&document()), "manifest.json");
    manifest["formatVersion"] = json!("2.0.0");
    assert!(
        read(tampered("manifest.json", &manifest))
            .expect("reads")
            .read_only,
        "major 2"
    );

    let mut manifest = entry(&gaf(&document()), "manifest.json");
    manifest["formatVersion"] = json!("1.4.0");
    assert!(
        !read(tampered("manifest.json", &manifest))
            .expect("reads")
            .read_only,
        "minor bump"
    );

    let mut manifest = entry(&gaf(&document()), "manifest.json");
    manifest["features"]["required"] = json!(["holograms"]);
    assert!(
        read(tampered("manifest.json", &manifest))
            .expect("reads")
            .read_only,
        "unknown capability"
    );

    let mut manifest = entry(&gaf(&document()), "manifest.json");
    manifest["schemaVersion"] = json!(2);
    assert!(matches!(
        read(tampered("manifest.json", &manifest)),
        Err(RepositoryError::TooNew {
            found: 2,
            supported: 1
        })
    ));
}

#[test]
fn s19_09_the_repository_writes_a_synced_file_and_reads_it_back() {
    let dir = std::env::temp_dir().join(format!("gafira-format-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join("poster.gaf");
    let repository = GafRepository::new(Limits::default(), std::sync::Arc::new(store()));
    repository.save(&path, &document(), &[]).expect("saves");
    assert_eq!(repository.load(&path).expect("loads").document, document());
    assert!(matches!(
        repository.load(&dir.join("missing.gaf")),
        Err(RepositoryError::Io(_))
    ));
    std::fs::remove_dir_all(dir).expect("clean up");
}

// S38-01: the outline and the corner radius of a shape survive the `.gaf`;
// the golden without them still loads and round-trips (S19-03).
#[test]
fn s38_01_an_outlined_rounded_shape_round_trips_through_the_gaf() {
    let text = document().to_json().expect("json");
    let mut json: Value = serde_json::from_str(&text).expect("json");
    json["pages"][0]["nodes"][0]["kind"] = json!({
        "type": "shape",
        "geometry": "rect",
        "fill": null,
        "stroke": {
            "color": { "space": "srgb", "r": 0.0, "g": 0.0, "b": 1.0, "a": 1.0 },
            "width": 3.5
        },
        "corner_radius": 9.0,
    });
    let outlined = Document::from_json(&serde_json::to_vec(&json).expect("bytes"))
        .expect("the outlined document loads");
    let loaded = read(gaf(&outlined)).expect("reads");
    assert_eq!(loaded.document, outlined);
    let back: Value =
        serde_json::from_str(&loaded.document.to_json().expect("json")).expect("json");
    let kind = &back["pages"][0]["nodes"][0]["kind"];
    assert_eq!(kind["stroke"]["width"], json!(3.5));
    assert_eq!(kind["corner_radius"], json!(9.0));
}

// S41-19: the rotation and both flips of an element survive the `.gaf`;
// a placement without them loads unturned and unflipped.
#[test]
fn s41_19_a_turned_flipped_element_round_trips_through_the_gaf() {
    let text = document().to_json().expect("json");
    let mut json: Value = serde_json::from_str(&text).expect("json");
    let placement = &mut json["pages"][0]["nodes"][0]["base"]["placement"];
    placement["rotation_deg"] = json!(292.5);
    placement["flip"] = json!({ "x": true, "y": true });
    let turned = Document::from_json(&serde_json::to_vec(&json).expect("bytes"))
        .expect("the turned document loads");
    let loaded = read(gaf(&turned)).expect("reads");
    assert_eq!(loaded.document, turned);
    let back: Value =
        serde_json::from_str(&loaded.document.to_json().expect("json")).expect("json");
    let placement = &back["pages"][0]["nodes"][0]["base"]["placement"];
    assert_eq!(placement["rotation_deg"], json!(292.5));
    assert_eq!(placement["flip"], json!({ "x": true, "y": true }));

    let mut plain: Value = serde_json::from_str(&text).expect("json");
    let placement = plain["pages"][0]["nodes"][0]["base"]["placement"]
        .as_object_mut()
        .expect("an object");
    placement.remove("rotation_deg");
    placement.remove("flip");
    let old = Document::from_json(&serde_json::to_vec(&plain).expect("bytes"))
        .expect("a placement without them loads");
    let loaded = read(gaf(&old)).expect("reads");
    let back: Value =
        serde_json::from_str(&loaded.document.to_json().expect("json")).expect("json");
    let placement = &back["pages"][0]["nodes"][0]["base"]["placement"];
    assert_eq!(placement["rotation_deg"], json!(0.0));
    assert_eq!(placement["flip"], json!({ "x": false, "y": false }));
}

// S43-19: groups, nested and with a turned child, round-trip through the
// `.gaf` with their children lists, in schema version 1: the format has
// had `group` nodes since spec 019, so spec 043 needs no new version.
#[test]
fn s43_19_nested_groups_round_trip_through_the_gaf() {
    let mut json: Value = serde_json::from_str(&document().to_json().expect("json")).expect("json");
    let rect = |id: u64, x: f64, rotation: f64| {
        json!({
            "base": {"id": id, "placement": {"x": x, "y": 10.0, "w": 40.0, "h": 20.0,
                "rotation_deg": rotation}},
            "kind": {"type": "shape", "geometry": "rect", "fill": null},
        })
    };
    let group = |id: u64, children: &[u64]| {
        json!({
            "base": {"id": id, "name": "Grupo", "opacity": 0.5,
                "placement": {"x": 0.0, "y": 0.0, "w": 100.0, "h": 40.0}},
            "kind": {"type": "group"},
            "children": children,
        })
    };
    json["next_id"] = json!(30);
    let page = &mut json["pages"][0];
    page["roots"] = json!([2]);
    page["nodes"] = json!([
        group(2, &[20, 21]),
        rect(20, 0.0, 0.0),
        group(21, &[22]),
        rect(22, 60.0, 45.0),
    ]);
    let grouped = Document::from_json(&serde_json::to_vec(&json).expect("bytes"))
        .expect("the grouped document loads");
    let loaded = read(gaf(&grouped)).expect("reads");
    assert_eq!(loaded.document, grouped);
    assert_eq!(loaded.document.schema_version(), 1);
    let page = &loaded.document.pages()[0];
    let ids = |id: u64| -> Vec<u64> {
        let node = page.node(gafira_doc::NodeId::new(id)).expect("a node");
        node.children.iter().map(|c| c.get()).collect()
    };
    assert_eq!(ids(2), [20, 21]);
    assert_eq!(ids(21), [22]);
    let turned = page.node(gafira_doc::NodeId::new(22)).expect("a node");
    assert_eq!(turned.base.placement.rotation_deg, 45.0);
}
