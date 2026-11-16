//! Writing a `.gaf` (02 §7.1, D-68).

use std::collections::BTreeMap;
use std::io::{self, Seek, SeekFrom, Write};

use gafira_doc::{AssetRef, AssetStorage, Document};
use gafira_ports::repository::RepositoryError;
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::ReadSeek;
use serde_json::Value;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipWriter};

use crate::{MIMETYPE, assets, manifest};

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

/// Write `document` as a `.gaf` into `sink`: `mimetype` first and stored,
/// then `manifest.json`, `document.json`, `pages/<id>.json` in page order,
/// `sequences/<id>.json` and one `assets/<blake3-hex>.<ext>` per embedded
/// asset in hash order, with the bytes `store` holds (spec 026).
///
/// # Errors
/// [`RepositoryError::Io`] when the sink fails, the document does not
/// serialise or the store does not have an embedded asset.
pub fn write_gaf<W: Write + Seek>(
    sink: W,
    document: &Document,
    store: &dyn ContentStore,
) -> Result<(), RepositoryError> {
    let embedded: Vec<(AssetRef, String)> = document
        .assets()
        .iter()
        .filter(|(_, entry)| entry.storage == AssetStorage::Embedded)
        .map(|(reference, entry)| (*reference, assets::entry_name(*reference, &entry.name)))
        .collect();
    let mut sizes = BTreeMap::new();
    for (reference, _) in &embedded {
        let size = open(store, *reference)?
            .seek(SeekFrom::End(0))
            .map_err(RepositoryError::Io)?;
        sizes.insert(reference.hex(), size);
    }

    let mut whole = serde_json::to_value(document).map_err(io)?;
    let root = whole
        .as_object_mut()
        .ok_or_else(|| io("a document serialises to an object"))?;
    let pages = take_list(root, "pages");
    let sequences = take_list(root, "sequences");
    let table = root.get("assets").cloned().unwrap_or(Value::Null);
    let schema = root
        .get("schema_version")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let project_id = root.get("project_id").cloned().unwrap_or(Value::Null);
    let ids = |items: &[Value]| -> Vec<Value> {
        items
            .iter()
            .map(|item| item.get("id").cloned().unwrap_or(Value::Null))
            .collect()
    };
    let manifest = manifest::build(
        schema,
        &project_id,
        &ids(&pages),
        &ids(&sequences),
        &table,
        &sizes,
    );

    let mut zip = ZipWriter::new(sink);
    zip.start_file("mimetype", options(CompressionMethod::Stored))
        .map_err(io)?;
    zip.write_all(MIMETYPE.as_bytes())
        .map_err(RepositoryError::Io)?;
    let json = |zip: &mut ZipWriter<W>, name: String, value: &Value| {
        zip.start_file(name, options(CompressionMethod::Deflated))
            .map_err(io)?;
        let bytes = serde_json::to_vec(value).map_err(io)?;
        zip.write_all(&bytes).map_err(RepositoryError::Io)
    };
    json(&mut zip, "manifest.json".to_owned(), &manifest)?;
    json(&mut zip, "document.json".to_owned(), &whole)?;
    for page in &pages {
        json(&mut zip, format!("pages/{}.json", entry_id(page)), page)?;
    }
    for sequence in &sequences {
        json(
            &mut zip,
            format!("sequences/{}.json", entry_id(sequence)),
            sequence,
        )?;
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
