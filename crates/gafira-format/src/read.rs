//! Reading a `.gaf`, treating it as hostile (008 §11).

use std::collections::BTreeSet;
use std::io::{self, Read, Seek};
use std::sync::Arc;

use gafira_doc::{AssetRef, AssetStorage, ContentHash, Document};
use gafira_ports::repository::{LoadedProject, RepositoryError, Version};
use gafira_ports::store::{ContentStore, StoreError};
use serde_json::Value;
use zip::ZipArchive;
use zip::result::ZipError;

use crate::{FORMAT, Limits, MIMETYPE, assets, manifest, thumbs, versions};

pub(crate) fn corrupt(detail: impl Into<String>) -> RepositoryError {
    RepositoryError::Corrupt {
        detail: detail.into(),
    }
}

pub(crate) fn zip_error(error: ZipError) -> RepositoryError {
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
/// Every embedded asset is checked against the hash its entry name carries
/// and then put into `store`; a mismatch, or an embedded asset of the table
/// without its entry, is corrupt (spec 026).
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
    store: &dyn ContentStore,
) -> Result<LoadedProject, RepositoryError> {
    let mut zip = ZipArchive::new(source).map_err(zip_error)?;
    if zip.len() > limits.max_entries {
        return Err(corrupt(format!(
            "{} entries, more than {}",
            zip.len(),
            limits.max_entries
        )));
    }
    let mut embedded: Vec<(usize, AssetRef)> = Vec::new();
    for index in 0..zip.len() {
        let entry = zip.by_index_raw(index).map_err(zip_error)?;
        let name = entry.name().map_err(zip_error)?;
        if !safe_name(&name) {
            return Err(corrupt(format!("unsafe entry name {:?}", name)));
        }
        let (size, packed) = (entry.size(), entry.compressed_size().max(1));
        if size > 1024 * 1024 && size / packed > limits.max_ratio {
            return Err(corrupt(format!("{} expands {}x", name, size / packed)));
        }
        if name.starts_with(assets::FOLDER) && !entry.is_dir() {
            let reference = assets::parse(&name)
                .ok_or_else(|| corrupt(format!("malformed asset name {:?}", name)))?;
            if size > limits.max_asset_bytes {
                return Err(corrupt(format!(
                    "{} is larger than {} bytes",
                    name, limits.max_asset_bytes
                )));
            }
            embedded.push((index, reference));
        }
        if name.starts_with(thumbs::FOLDER) && !entry.is_dir() {
            if thumbs::parse(&name).is_none() {
                return Err(corrupt(format!("malformed thumbnail name {:?}", name)));
            }
            if size > limits.max_thumb_bytes {
                return Err(corrupt(format!(
                    "{} is larger than {} bytes",
                    name, limits.max_thumb_bytes
                )));
            }
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

    let document = assemble(&mut zip, "", &manifest, limits, |root| {
        if root.get("project_id") == manifest.get("projectId") {
            Ok(())
        } else {
            Err(corrupt(
                "the manifest and the document name different projects",
            ))
        }
    })?;
    let versions = read_versions(&mut zip, &manifest, limits)?;

    let present: BTreeSet<AssetRef> = embedded.iter().map(|(_, r)| *r).collect();
    let documents =
        std::iter::once(&document).chain(versions.iter().map(|version| version.document.as_ref()));
    for doc in documents {
        if let Some((missing, _)) = doc
            .assets()
            .iter()
            .find(|(r, e)| e.storage == AssetStorage::Embedded && !present.contains(r))
        {
            return Err(corrupt(format!("embedded asset {missing} has no entry")));
        }
    }
    for (index, reference) in embedded {
        import(&mut zip, index, reference, limits, store)?;
    }
    Ok(LoadedProject {
        document,
        read_only,
        versions,
    })
}

/// The document under `prefix`: `<prefix>document.json`, which `check`
/// sees first, with the pages and the sequences `listed` names by id, in
/// its order, validated as a whole.
fn assemble<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    prefix: &str,
    listed: &Value,
    limits: Limits,
    check: impl FnOnce(&serde_json::Map<String, Value>) -> Result<(), RepositoryError>,
) -> Result<Document, RepositoryError> {
    let name = format!("{prefix}document.json");
    let mut document = json(zip, &name, limits)?;
    let root = document
        .as_object_mut()
        .ok_or_else(|| corrupt(format!("{name} is not an object")))?;
    check(root)?;
    let pages = parts(zip, listed, prefix, "pages", limits)?;
    let sequences = parts(zip, listed, prefix, "sequences", limits)?;
    root.insert("pages".to_owned(), Value::Array(pages));
    root.insert("sequences".to_owned(), Value::Array(sequences));
    let bytes = serde_json::to_vec(&document).map_err(|e| corrupt(e.to_string()))?;
    Document::from_json(&bytes).map_err(|e| corrupt(format!("{name}: {e}")))
}

/// The versions the manifest lists, in its order (spec 031): each one must
/// have its entry in `versions/index.json`, at the same place and with a
/// distinct id, and its folder must hold a valid document of the schema the
/// entry names. A manifest without `versions` has none.
fn read_versions<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    manifest: &Value,
    limits: Limits,
) -> Result<Vec<Version>, RepositoryError> {
    let ids = match manifest.get("versions") {
        None => return Ok(Vec::new()),
        Some(list) => list
            .as_array()
            .ok_or_else(|| corrupt("the manifest's versions is not a list"))?
            .iter()
            .map(|id| {
                id.as_u64()
                    .ok_or_else(|| corrupt("a version id is not a number"))
            })
            .collect::<Result<Vec<u64>, _>>()?,
    };
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let distinct: BTreeSet<u64> = ids.iter().copied().collect();
    if distinct.len() != ids.len() {
        return Err(corrupt("the manifest lists a version twice"));
    }
    let index = json(zip, versions::INDEX, limits)?;
    let entries = index
        .as_array()
        .ok_or_else(|| corrupt("the version index is not a list"))?;
    if entries.len() != ids.len() {
        return Err(corrupt(
            "the version index and the manifest list different versions",
        ));
    }
    let mut found = Vec::with_capacity(ids.len());
    for (value, id) in entries.iter().zip(ids) {
        let entry = versions::parse_entry(value, id).map_err(corrupt)?;
        let listed = serde_json::json!({ "pages": entry.pages, "sequences": entry.sequences });
        let document = assemble(zip, &versions::prefix(id), &listed, limits, |_| Ok(()))?;
        if document.schema_version() != entry.info.schema {
            return Err(corrupt(format!(
                "version {id} names schema {} but holds {}",
                entry.info.schema,
                document.schema_version()
            )));
        }
        found.push(Version {
            info: entry.info,
            document: Arc::new(document),
        });
    }
    Ok(found)
}

/// Checks the asset at `index` against its hash, then puts it into `store`.
/// Its bytes are read twice so that a mismatch never reaches the store.
fn import<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    index: usize,
    reference: AssetRef,
    limits: Limits,
    store: &dyn ContentStore,
) -> Result<(), RepositoryError> {
    let mismatch = || {
        corrupt(format!(
            "the bytes of asset {reference} do not match its hash"
        ))
    };
    let mut hasher = blake3::Hasher::new();
    let entry = zip.by_index(index).map_err(zip_error)?;
    let read =
        io::copy(&mut entry.take(limits.max_asset_bytes + 1), &mut hasher).map_err(asset_error)?;
    if read > limits.max_asset_bytes
        || ContentHash(*hasher.finalize().as_bytes()) != reference.as_hash()
    {
        return Err(mismatch());
    }
    let mut entry = zip.by_index(index).map_err(zip_error)?;
    let stored = store.put(&mut entry).map_err(|e| match e {
        StoreError::Io(e) => RepositoryError::Io(e),
        StoreError::NotFound { .. } => mismatch(),
    })?;
    if stored != reference.as_hash() {
        return Err(mismatch());
    }
    Ok(())
}

/// A failed read of a ZIP entry: a bad checksum or a broken stream is the
/// file's fault, anything else the disk's.
fn asset_error(error: io::Error) -> RepositoryError {
    match error.kind() {
        io::ErrorKind::InvalidData | io::ErrorKind::UnexpectedEof => corrupt(error.to_string()),
        _ => RepositoryError::Io(error),
    }
}

pub(crate) fn check_mimetype<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
) -> Result<(), RepositoryError> {
    let mut first = zip.by_index(0).map_err(zip_error)?;
    if first.name().map_err(zip_error)? != "mimetype"
        || first.compression() != zip::CompressionMethod::Stored
    {
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

pub(crate) fn json<R: Read + Seek>(
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

/// The pages or the sequences `listed` names (the manifest, or a version's
/// index entry), in its order, from under `prefix`.
fn parts<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    listed: &Value,
    prefix: &str,
    folder: &str,
    limits: Limits,
) -> Result<Vec<Value>, RepositoryError> {
    let ids = listed
        .get(folder)
        .and_then(Value::as_array)
        .ok_or_else(|| corrupt(format!("{prefix}: no {folder} are listed")))?;
    ids.iter()
        .map(|id| {
            let id = id
                .as_u64()
                .ok_or_else(|| corrupt(format!("a {folder} id is not a number")))?;
            let name = format!("{prefix}{folder}/{id}.json");
            let item = json(zip, &name, limits)?;
            if item.get("id").and_then(Value::as_u64) != Some(id) {
                return Err(corrupt(format!("{name} holds another id")));
            }
            Ok(item)
        })
        .collect()
}
