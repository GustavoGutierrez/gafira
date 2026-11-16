//! Writing a `.gaf` (02 §7.1, D-68).

use std::io::{Seek, Write};

use gafira_doc::Document;
use gafira_ports::repository::RepositoryError;
use serde_json::Value;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipWriter};

use crate::{MIMETYPE, manifest};

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

/// Write `document` as a `.gaf` into `sink`: `mimetype` first and stored,
/// then `manifest.json`, `document.json`, `pages/<id>.json` in page order and
/// `sequences/<id>.json`.
///
/// # Errors
/// [`RepositoryError::Io`] when the sink fails or the document does not
/// serialise.
pub fn write_gaf<W: Write + Seek>(sink: W, document: &Document) -> Result<(), RepositoryError> {
    let mut whole = serde_json::to_value(document).map_err(io)?;
    let root = whole
        .as_object_mut()
        .ok_or_else(|| io("a document serialises to an object"))?;
    let pages = take_list(root, "pages");
    let sequences = take_list(root, "sequences");
    let assets = root.get("assets").cloned().unwrap_or(Value::Null);
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
    let manifest = manifest::build(schema, &project_id, &ids(&pages), &ids(&sequences), &assets);

    let mut zip = ZipWriter::new(sink);
    zip.start_file("mimetype", options(CompressionMethod::Stored))
        .map_err(io)?;
    zip.write_all(MIMETYPE.as_bytes())
        .map_err(RepositoryError::Io)?;
    let mut json = |name: String, value: &Value| -> Result<(), RepositoryError> {
        zip.start_file(name, options(CompressionMethod::Deflated))
            .map_err(io)?;
        let bytes = serde_json::to_vec(value).map_err(io)?;
        zip.write_all(&bytes).map_err(RepositoryError::Io)
    };
    json("manifest.json".to_owned(), &manifest)?;
    json("document.json".to_owned(), &whole)?;
    for page in &pages {
        json(format!("pages/{}.json", entry_id(page)), page)?;
    }
    for sequence in &sequences {
        json(format!("sequences/{}.json", entry_id(sequence)), sequence)?;
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
