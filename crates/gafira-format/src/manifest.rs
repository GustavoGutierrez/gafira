//! `manifest.json` (02 §7.1, 008 §5, D-68): camelCase keys.

use serde_json::{Map, Value, json};

use crate::{FORMAT, FORMAT_VERSION};

/// The manifest of a document with `pages`, `sequences` and `assets`
/// (hash to entry) as its JSON holds them.
pub(crate) fn build(
    schema: u64,
    project_id: &Value,
    pages: &[Value],
    sequences: &[Value],
    assets: &Value,
) -> Value {
    let names: Map<String, Value> = assets
        .as_object()
        .map(|table| {
            table
                .iter()
                .map(|(hash, entry)| {
                    (
                        hash.clone(),
                        json!({ "name": entry.get("name").cloned().unwrap_or(Value::Null) }),
                    )
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
        "versions": [],
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
