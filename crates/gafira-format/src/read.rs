//! Reading a `.gaf`, treating it as hostile (008 §11).

use std::io::{Read, Seek};

use gafira_doc::Document;
use gafira_ports::repository::{LoadedProject, RepositoryError};
use serde_json::Value;
use zip::ZipArchive;
use zip::result::ZipError;

use crate::{FORMAT, Limits, MIMETYPE, manifest};

fn corrupt(detail: impl Into<String>) -> RepositoryError {
    RepositoryError::Corrupt {
        detail: detail.into(),
    }
}

fn zip_error(error: ZipError) -> RepositoryError {
    match error {
        ZipError::Io(e) => RepositoryError::Io(e),
        other => corrupt(other.to_string()),
    }
}

/// An entry name a reader may follow: relative, `/`-separated, without `..`
/// or empty segments (ARQ-56, 008 §11).
pub(crate) fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('/')
        && !name.contains('\\')
        && !name.contains(':')
        && name
            .trim_end_matches('/')
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

/// Read the `.gaf` in `source` with `limits`.
///
/// A file of a newer container major, or one that requires a capability
/// this build does not know, loads read-only (ARQ-55). A newer document
/// schema cannot be represented and is [`RepositoryError::TooNew`].
///
/// # Errors
/// [`RepositoryError::Corrupt`] for anything that is not a well-formed `.gaf`
/// within the limits, [`RepositoryError::Io`] when the source fails.
pub fn read_gaf<R: Read + Seek>(
    source: R,
    limits: Limits,
) -> Result<LoadedProject, RepositoryError> {
    let mut zip = ZipArchive::new(source).map_err(zip_error)?;
    if zip.len() > limits.max_entries {
        return Err(corrupt(format!(
            "{} entries, more than {}",
            zip.len(),
            limits.max_entries
        )));
    }
    for index in 0..zip.len() {
        let entry = zip.by_index_raw(index).map_err(zip_error)?;
        if !safe_name(entry.name()) {
            return Err(corrupt(format!("unsafe entry name {:?}", entry.name())));
        }
        let (size, packed) = (entry.size(), entry.compressed_size().max(1));
        if size > 1024 * 1024 && size / packed > limits.max_ratio {
            return Err(corrupt(format!(
                "{} expands {}x",
                entry.name(),
                size / packed
            )));
        }
    }
    check_mimetype(&mut zip)?;
    let manifest = json(&mut zip, "manifest.json", limits)?;
    if manifest.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err(corrupt("the manifest is not a gafira-project"));
    }
    let major = manifest
        .get("formatVersion")
        .and_then(Value::as_str)
        .and_then(manifest::major)
        .ok_or_else(|| corrupt("the manifest has no formatVersion"))?;
    let schema = manifest
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let supported = Document::SCHEMA_VERSION;
    if schema > u64::from(supported) {
        return Err(RepositoryError::TooNew {
            found: u32::try_from(schema).unwrap_or(u32::MAX),
            supported,
        });
    }
    let read_only = major > 1 || !manifest::unknown_required(&manifest).is_empty();

    let mut document = json(&mut zip, "document.json", limits)?;
    let root = document
        .as_object_mut()
        .ok_or_else(|| corrupt("document.json is not an object"))?;
    if root.get("project_id") != manifest.get("projectId") {
        return Err(corrupt(
            "the manifest and the document name different projects",
        ));
    }
    let pages = parts(&mut zip, &manifest, "pages", limits)?;
    let sequences = parts(&mut zip, &manifest, "sequences", limits)?;
    root.insert("pages".to_owned(), Value::Array(pages));
    root.insert("sequences".to_owned(), Value::Array(sequences));
    let bytes = serde_json::to_vec(&document).map_err(|e| corrupt(e.to_string()))?;
    let document = Document::from_json(&bytes).map_err(|e| corrupt(e.to_string()))?;
    Ok(LoadedProject {
        document,
        read_only,
    })
}

fn check_mimetype<R: Read + Seek>(zip: &mut ZipArchive<R>) -> Result<(), RepositoryError> {
    let mut first = zip.by_index(0).map_err(zip_error)?;
    if first.name() != "mimetype" || first.compression() != zip::CompressionMethod::Stored {
        return Err(corrupt("the first entry is not a stored mimetype"));
    }
    let mut text = String::new();
    first
        .by_ref()
        .take(MIMETYPE.len() as u64 + 1)
        .read_to_string(&mut text)
        .map_err(|_| corrupt("the mimetype is not text"))?;
    if text != MIMETYPE {
        return Err(corrupt(format!("unknown mimetype {text:?}")));
    }
    Ok(())
}

fn json<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    name: &str,
    limits: Limits,
) -> Result<Value, RepositoryError> {
    let entry = zip.by_name(name).map_err(|e| match e {
        ZipError::FileNotFound => corrupt(format!("{name} is missing")),
        other => zip_error(other),
    })?;
    if entry.size() > limits.max_json_bytes {
        return Err(corrupt(format!(
            "{name} is larger than {} bytes",
            limits.max_json_bytes
        )));
    }
    let mut bytes = Vec::new();
    entry
        .take(limits.max_json_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(RepositoryError::Io)?;
    serde_json::from_slice(&bytes).map_err(|e| corrupt(format!("{name}: {e}")))
}

/// The pages or the sequences the manifest lists, in its order.
fn parts<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    manifest: &Value,
    folder: &str,
    limits: Limits,
) -> Result<Vec<Value>, RepositoryError> {
    let ids = manifest
        .get(folder)
        .and_then(Value::as_array)
        .ok_or_else(|| corrupt(format!("the manifest lists no {folder}")))?;
    ids.iter()
        .map(|id| {
            let id = id
                .as_u64()
                .ok_or_else(|| corrupt(format!("a {folder} id is not a number")))?;
            let item = json(zip, &format!("{folder}/{id}.json"), limits)?;
            if item.get("id").and_then(Value::as_u64) != Some(id) {
                return Err(corrupt(format!("{folder}/{id}.json holds another id")));
            }
            Ok(item)
        })
        .collect()
}
