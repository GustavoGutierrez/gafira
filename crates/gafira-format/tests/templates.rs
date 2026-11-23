//! Spec 091 (D-81): the `template` object of the manifest, written and read
//! back, and read without opening the project.

use std::collections::BTreeMap;
use std::io::{self, Cursor, Read, Write};
use std::sync::Mutex;

use gafira_doc::{ContentHash, Document, PageId, ProjectId};
use gafira_format::{Limits, read_gaf, read_template_info, write_gaf, write_template};
use gafira_ports::repository::{TemplateMeta, Thumbnail};
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::ReadSeek;
use gafira_raster::{ImageBuf, PixelBuf};
use serde_json::Value;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

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

/// A document of one page (id 1) and nothing else.
fn document() -> Document {
    Document::from_json(include_bytes!("../../gafira-doc/tests/golden/minimal.json"))
        .expect("the golden loads")
        .with_project_id(ProjectId::from_u128(0xbeef))
}

fn meta() -> TemplateMeta {
    TemplateMeta {
        tags: vec!["size.ig_square".to_owned(), "lang:es".to_owned()],
        page_format: "social".to_owned(),
        style: "minimal".to_owned(),
        color: "#FF6B57".to_owned(),
    }
}

fn thumbnail() -> Thumbnail {
    Thumbnail {
        page: PageId::new(1),
        image: ImageBuf::new(1, 1, PixelBuf::Rgba8(vec![255, 0, 0, 255])).expect("an image"),
    }
}

fn template_gaf(thumb: Option<&Thumbnail>) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    write_template(
        &mut bytes,
        &document(),
        thumb,
        &meta(),
        &MemoryStore::default(),
    )
    .expect("writes");
    bytes.into_inner()
}

fn manifest(bytes: &[u8]) -> Value {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("a zip");
    let mut text = String::new();
    zip.by_name("manifest.json")
        .expect("a manifest")
        .read_to_string(&mut text)
        .expect("text");
    serde_json::from_str(&text).expect("json")
}

/// `bytes` with its manifest's `template` replaced by `template`.
fn with_template(bytes: &[u8], template: Value) -> Vec<u8> {
    let mut source = ZipArchive::new(Cursor::new(bytes)).expect("a zip");
    let mut out = ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..source.len() {
        let mut entry = source.by_index(index).expect("an entry");
        let name = entry.name().expect("name").into_owned();
        let mut data = Vec::new();
        entry.read_to_end(&mut data).expect("bytes");
        if name == "manifest.json" {
            let mut value: Value = serde_json::from_slice(&data).expect("json");
            value["template"] = template.clone();
            data = serde_json::to_vec(&value).expect("json");
        }
        out.start_file(
            name,
            SimpleFileOptions::default().compression_method(entry.compression()),
        )
        .expect("starts");
        out.write_all(&data).expect("writes");
    }
    out.finish().expect("finishes").into_inner()
}

#[test]
fn s91_01_the_manifest_carries_the_template_in_camel_case() {
    let value = manifest(&template_gaf(None));
    let template = &value["template"];
    assert_eq!(template["tags"][0], "size.ig_square");
    assert_eq!(template["pageFormat"], "social");
    assert_eq!(template["style"], "minimal");
    assert_eq!(template["color"], "#FF6B57");
}

#[test]
fn s91_01_a_project_has_no_template() {
    let mut bytes = Cursor::new(Vec::new());
    write_gaf(&mut bytes, &document(), &[], None, &MemoryStore::default()).expect("writes");
    let bytes = bytes.into_inner();
    assert!(manifest(&bytes).get("template").is_none());
    let info = read_template_info(Cursor::new(bytes), Limits::default()).expect("reads");
    assert!(info.is_none());
}

#[test]
fn s91_02_the_template_and_its_thumbnail_are_read_back() {
    let info = read_template_info(
        Cursor::new(template_gaf(Some(&thumbnail()))),
        Limits::default(),
    )
    .expect("reads")
    .expect("a template");
    assert_eq!(info.meta, meta());
    let thumb = info.thumbnail.expect("a thumbnail");
    assert_eq!((thumb.width, thumb.height), (1, 1));
}

#[test]
fn s91_01_a_template_still_opens_as_a_document() {
    let loaded = read_gaf(
        Cursor::new(template_gaf(None)),
        Limits::default(),
        &MemoryStore::default(),
    )
    .expect("opens");
    assert_eq!(loaded.document.pages().len(), 1);
}

#[test]
fn s91_01_a_malformed_template_is_ignored_and_the_file_opens() {
    let bad = [
        serde_json::json!("social"),
        serde_json::json!({ "tags": "x", "pageFormat": "social", "style": "a", "color": "#FFFFFF" }),
        serde_json::json!({ "tags": [], "pageFormat": "banner", "style": "a", "color": "#FFFFFF" }),
        serde_json::json!({ "tags": [], "pageFormat": "social", "style": "a", "color": "red" }),
        serde_json::json!({ "tags": [1], "pageFormat": "social", "style": "a", "color": "#FFFFFF" }),
    ];
    for template in bad {
        let bytes = with_template(&template_gaf(None), template.clone());
        let info =
            read_template_info(Cursor::new(bytes.clone()), Limits::default()).expect("reads");
        assert!(info.is_none(), "{template} is not a template");
        read_gaf(
            Cursor::new(bytes),
            Limits::default(),
            &MemoryStore::default(),
        )
        .expect("the project still opens");
    }
}
