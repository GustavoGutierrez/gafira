//! Writing a `.gaf` (02 §7.1, D-68).

use std::collections::BTreeMap;
use std::io::{self, Read, Seek, SeekFrom, Write};

use gafira_doc::{AssetRef, AssetStorage, Document};
use gafira_ports::repository::{RepositoryError, TemplateMeta, Thumbnail, Version, VersionKind};
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::ReadSeek;
use serde_json::{Value, json};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipWriter};

use crate::read::{check_mimetype, corrupt, json, zip_error};
use crate::{FORMAT, Limits, MIMETYPE, assets, manifest, thumbs, versions};

fn io(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> RepositoryError {
    RepositoryError::Io(std::io::Error::other(error))
}

/// Every entry gets the same timestamp, so the same document always writes
/// the same bytes.
fn options(method: CompressionMethod) -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(method)
        .last_modified_time(DateTime::default())
}

/// The bytes of an embedded asset, from the store.
fn open(
    store: &dyn ContentStore,
    reference: AssetRef,
) -> Result<Box<dyn ReadSeek + Send>, RepositoryError> {
    store.open(reference.as_hash()).map_err(|e| match e {
        StoreError::NotFound { .. } => RepositoryError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            format!("embedded asset {reference} is not in the content store"),
        )),
        StoreError::Io(e) => RepositoryError::Io(e),
    })
}

/// A document's JSON as the container holds it: the root without its pages
/// and sequences, and those apart.
struct Split {
    root: Value,
    pages: Vec<Value>,
    sequences: Vec<Value>,
}

impl Split {
    fn of(document: &Document) -> Result<Self, RepositoryError> {
        let mut root = serde_json::to_value(document).map_err(io)?;
        let object = root
            .as_object_mut()
            .ok_or_else(|| io("a document serialises to an object"))?;
        let pages = take_list(object, "pages");
        let sequences = take_list(object, "sequences");
        Ok(Self {
            root,
            pages,
            sequences,
        })
    }

    fn page_ids(&self) -> Vec<Value> {
        ids(&self.pages)
    }

    fn sequence_ids(&self) -> Vec<Value> {
        ids(&self.sequences)
    }
}

fn ids(items: &[Value]) -> Vec<Value> {
    items
        .iter()
        .map(|item| item.get("id").cloned().unwrap_or(Value::Null))
        .collect()
}

/// The `versions/index.json` entry of `version`, whose document is `split`
/// (02 §7.1, spec 031). The ids of its pages and sequences give their order,
/// as the manifest's do for the document.
fn index_entry(version: &Version, split: &Split) -> Value {
    let info = &version.info;
    json!({
        "id": info.id.0,
        "name": info.name,
        "kind": match info.kind {
            VersionKind::Named => versions::NAMED,
            VersionKind::Auto => versions::AUTO,
        },
        "created": info.created.0,
        "app": info.app,
        "schema": info.schema,
        "pages": split.page_ids(),
        "sequences": split.sequence_ids(),
    })
}

/// Write `document` and its `versions` as a `.gaf` into `sink`: `mimetype`
/// first and stored, then `manifest.json`, `document.json`,
/// `pages/<id>.json` in page order and `sequences/<id>.json`; with any
/// version, `versions/index.json` and, per version in order, its
/// `versions/<id>/document.json`, `pages/` and `sequences/` (spec 031);
/// with a `thumbnail`, `thumbs/<page_id>.webp`, stored, unless it cannot
/// be encoded: then it is left out and the write goes on (spec 033);
/// last, one `assets/<blake3-hex>.<ext>` per embedded asset of the document
/// or of any version, once per hash and in hash order, with the bytes
/// `store` holds (spec 026).
///
/// # Errors
/// [`RepositoryError::Io`] when the sink fails, the document does not
/// serialise or the store does not have an embedded asset.
pub fn write_gaf<W: Write + Seek>(
    sink: W,
    document: &Document,
    versions: &[Version],
    thumbnail: Option<&Thumbnail>,
    store: &dyn ContentStore,
) -> Result<(), RepositoryError> {
    write(sink, document, versions, thumbnail, None, store)
}

/// [`write_gaf`] of a template (D-81, spec 091): `document` with no
/// versions, and `meta` as the manifest's `template`.
///
/// # Errors
/// Those of [`write_gaf`].
pub fn write_template<W: Write + Seek>(
    sink: W,
    document: &Document,
    thumbnail: Option<&Thumbnail>,
    meta: &TemplateMeta,
    store: &dyn ContentStore,
) -> Result<(), RepositoryError> {
    write(sink, document, &[], thumbnail, Some(meta), store)
}

fn write<W: Write + Seek>(
    sink: W,
    document: &Document,
    versions: &[Version],
    thumbnail: Option<&Thumbnail>,
    template: Option<&TemplateMeta>,
    store: &dyn ContentStore,
) -> Result<(), RepositoryError> {
    // The document names an asset first; a version adds only the ones the
    // document does not have.
    let mut embedded: BTreeMap<AssetRef, String> = BTreeMap::new();
    for doc in std::iter::once(document).chain(versions.iter().map(|v| v.document.as_ref())) {
        for (reference, entry) in doc.assets() {
            if entry.storage == AssetStorage::Embedded {
                embedded
                    .entry(*reference)
                    .or_insert_with(|| assets::entry_name(*reference, &entry.name));
            }
        }
    }
    let mut sizes = BTreeMap::new();
    for reference in embedded.keys() {
        let size = open(store, *reference)?
            .seek(SeekFrom::End(0))
            .map_err(RepositoryError::Io)?;
        sizes.insert(reference.hex(), size);
    }

    let main = Split::of(document)?;
    let table = main.root.get("assets").cloned().unwrap_or(Value::Null);
    let schema = main
        .root
        .get("schema_version")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let project_id = main.root.get("project_id").cloned().unwrap_or(Value::Null);
    let version_ids: Vec<Value> = versions.iter().map(|v| Value::from(v.info.id.0)).collect();
    let mut manifest = manifest::build(
        schema,
        &project_id,
        &main.page_ids(),
        &main.sequence_ids(),
        &table,
        &sizes,
        &version_ids,
    );
    if let (Some(meta), Some(fields)) = (template, manifest.as_object_mut()) {
        fields.insert("template".to_owned(), manifest::template_value(meta));
    }
    let splits = versions
        .iter()
        .map(|version| Split::of(&version.document))
        .collect::<Result<Vec<_>, _>>()?;

    let thumb = thumbnail.and_then(|t| thumbs::encode(t).map(|bytes| (t.page, bytes)));

    let mut zip = ZipWriter::new(sink);
    zip.start_file("mimetype", options(CompressionMethod::Stored))
        .map_err(io)?;
    zip.write_all(MIMETYPE.as_bytes())
        .map_err(RepositoryError::Io)?;
    json_entry(&mut zip, "manifest.json".to_owned(), &manifest)?;
    write_parts(&mut zip, "", &main)?;
    if !versions.is_empty() {
        let index: Vec<Value> = versions
            .iter()
            .zip(&splits)
            .map(|(version, split)| index_entry(version, split))
            .collect();
        json_entry(&mut zip, versions::INDEX.to_owned(), &Value::Array(index))?;
        for (version, split) in versions.iter().zip(&splits) {
            write_parts(&mut zip, &versions::prefix(version.info.id.0), split)?;
        }
    }
    if let Some((page, bytes)) = thumb {
        zip.start_file(thumbs::name(page), options(CompressionMethod::Stored))
            .map_err(io)?;
        zip.write_all(&bytes).map_err(RepositoryError::Io)?;
    }
    for (reference, name) in &embedded {
        let ext = name.rsplit_once('.').map_or("", |(_, ext)| ext);
        let size = sizes.get(&reference.hex()).copied().unwrap_or(0);
        let options = options(assets::method(ext)).large_file(size >= u64::from(u32::MAX));
        zip.start_file(name.as_str(), options).map_err(io)?;
        let mut bytes = open(store, *reference)?;
        io::copy(&mut bytes, &mut zip).map_err(RepositoryError::Io)?;
    }
    zip.finish().map_err(io)?;
    Ok(())
}

fn json_entry<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    name: String,
    value: &Value,
) -> Result<(), RepositoryError> {
    zip.start_file(name, options(CompressionMethod::Deflated))
        .map_err(io)?;
    let bytes = serde_json::to_vec(value).map_err(io)?;
    zip.write_all(&bytes).map_err(RepositoryError::Io)
}

/// `<prefix>document.json`, then the pages and the sequences under `prefix`.
fn write_parts<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    prefix: &str,
    split: &Split,
) -> Result<(), RepositoryError> {
    json_entry(zip, format!("{prefix}document.json"), &split.root)?;
    for page in &split.pages {
        json_entry(zip, format!("{prefix}pages/{}.json", entry_id(page)), page)?;
    }
    for sequence in &split.sequences {
        json_entry(
            zip,
            format!("{prefix}sequences/{}.json", entry_id(sequence)),
            sequence,
        )?;
    }
    Ok(())
}

fn take_list(root: &mut serde_json::Map<String, Value>, key: &str) -> Vec<Value> {
    match root.remove(key) {
        Some(Value::Array(items)) => items,
        _ => Vec::new(),
    }
}

/// The id of a page or a sequence as its file name.
pub(crate) fn entry_id(item: &Value) -> String {
    item.get("id").map(Value::to_string).unwrap_or_default()
}

/// Copy the `.gaf` in `source` to `sink` with `meta` as its `template`
/// (D-81, spec 093): every entry but the manifest is copied as it is, in
/// its order and without packing it again; the manifest gets `template`,
/// in place of any it had. «Guardar como plantilla de marca» and «Subir
/// plantilla» make a template this way.
///
/// # Errors
/// [`RepositoryError::Corrupt`] when `source` is not a `.gaf` within
/// `limits`, [`RepositoryError::Io`] when a stream fails.
pub fn set_template<R: Read + Seek, W: Write + Seek>(
    source: R,
    sink: W,
    meta: &TemplateMeta,
    limits: Limits,
) -> Result<(), RepositoryError> {
    let mut zip = zip::ZipArchive::new(source).map_err(zip_error)?;
    if zip.len() > limits.max_entries {
        return Err(corrupt(format!(
            "{} entries, more than {}",
            zip.len(),
            limits.max_entries
        )));
    }
    check_mimetype(&mut zip)?;
    let mut manifest = json(&mut zip, "manifest.json", limits)?;
    if manifest.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err(corrupt("the manifest is not a gafira-project"));
    }
    if let Some(fields) = manifest.as_object_mut() {
        fields.insert("template".to_owned(), manifest::template_value(meta));
    }
    let mut out = ZipWriter::new(sink);
    for index in 0..zip.len() {
        let entry = zip.by_index_raw(index).map_err(zip_error)?;
        if entry.name().map_err(zip_error)? == "manifest.json" {
            drop(entry);
            json_entry(&mut out, "manifest.json".to_owned(), &manifest)?;
        } else {
            out.raw_copy_file(entry).map_err(io)?;
        }
    }
    out.finish().map_err(io)?;
    Ok(())
}
