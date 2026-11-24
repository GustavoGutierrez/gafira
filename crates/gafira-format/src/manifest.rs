//! `manifest.json` (02 §7.1, 008 §5, D-68): camelCase keys.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use gafira_ports::repository::{TEMPLATE_PAGE_FORMATS, TemplateMeta};

use crate::{FORMAT, FORMAT_VERSION};

/// The manifest of a document with `pages`, `sequences` and `assets`
/// (hash to entry) as its JSON holds them; `sizes` gives the bytes of each
/// embedded asset by its hex hash; `versions` lists the ids of the
/// versions in file order (spec 031).
pub(crate) fn build(
    schema: u64,
    project_id: &Value,
    pages: &[Value],
    sequences: &[Value],
    assets: &Value,
    sizes: &BTreeMap<String, u64>,
    versions: &[Value],
) -> Value {
    let names: Map<String, Value> = assets
        .as_object()
        .map(|table| {
            table
                .iter()
                .map(|(hash, entry)| {
                    let mut item = Map::new();
                    item.insert(
                        "name".to_owned(),
                        entry.get("name").cloned().unwrap_or(Value::Null),
                    );
                    if let Some(bytes) = sizes.get(hash) {
                        item.insert("bytes".to_owned(), Value::from(*bytes));
                    }
                    (hash.clone(), Value::Object(item))
                })
                .collect()
        })
        .unwrap_or_default();
    json!({
        "format": FORMAT,
        "formatVersion": FORMAT_VERSION,
        "schemaVersion": schema,
        "generator": { "app": "Gafira", "version": env!("CARGO_PKG_VERSION") },
        "projectId": project_id,
        // Every medium is embedded by default (02 §7.1); linked media are
        // Fase 2, so a file this build writes is always complete.
        "profile": "portable-full",
        "features": { "required": [], "optional": [] },
        "packageCompleteness": "complete",
        "missingResources": [],
        "pages": pages,
        "sequences": sequences,
        "assets": names,
        "versions": versions,
    })
}

/// The `template` object of the manifest (D-81, spec 091).
pub(crate) fn template_value(meta: &TemplateMeta) -> Value {
    json!({
        "tags": meta.tags,
        "pageFormat": meta.page_format,
        "style": meta.style,
        "color": meta.color,
    })
}

/// The template a manifest describes, when its `template` is well formed:
/// a list of words, a known page format, a style and a `#RRGGBB` colour.
/// Anything else is no template, and the file is an ordinary project.
pub(crate) fn template(manifest: &Value) -> Option<TemplateMeta> {
    let value = manifest.get("template")?.as_object()?;
    let tags = value
        .get("tags")?
        .as_array()?
        .iter()
        .map(|tag| tag.as_str().map(str::to_owned))
        .collect::<Option<Vec<_>>>()?;
    let page_format = value.get("pageFormat")?.as_str()?;
    if !TEMPLATE_PAGE_FORMATS.contains(&page_format) {
        return None;
    }
    let style = value.get("style")?.as_str()?;
    let color = value.get("color")?.as_str()?;
    let hex = color.strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some(TemplateMeta {
        tags,
        page_format: page_format.to_owned(),
        style: style.to_owned(),
        color: color.to_owned(),
    })
}

/// The major number of a `formatVersion` (`"1.0.0"` → 1).
pub(crate) fn major(version: &str) -> Option<u64> {
    version.split('.').next()?.parse().ok()
}

/// The capabilities a file requires that this build does not know.
pub(crate) fn unknown_required(manifest: &Value) -> Vec<String> {
    const KNOWN: &[&str] = &[];
    manifest
        .pointer("/features/required")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .filter(|name| !KNOWN.contains(name))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s19_02_major_of_the_format_version() {
        assert_eq!(major("1.0.0"), Some(1));
        assert_eq!(major("2.3.1"), Some(2));
        assert_eq!(major("x"), None);
    }
}
