//! The `versions/` folder of a `.gaf` (02 §7.1 «Versiones», D-67, spec 031):
//! `versions/index.json` and one `versions/<id>/` per version.

use gafira_doc::Document;
use gafira_ports::common::UnixTime;
use gafira_ports::repository::{VersionId, VersionInfo, VersionKind};
use serde_json::Value;

/// The folder of the versions.
pub(crate) const FOLDER: &str = "versions/";

/// The index of the versions.
pub(crate) const INDEX: &str = "versions/index.json";

/// The `kind` of a named version.
pub(crate) const NAMED: &str = "named";

/// The `kind` of an automatic version.
pub(crate) const AUTO: &str = "auto";

/// The longest name a version may carry, in characters.
pub(crate) const MAX_NAME_CHARS: usize = 256;

/// The longest app version a version may carry, in characters.
const MAX_APP_CHARS: usize = 64;

/// The folder of the version `id`, with its trailing `/`.
pub(crate) fn prefix(id: u64) -> String {
    format!("{FOLDER}{id}/")
}

/// One entry of the index: what the port sees of it, and the ids of its
/// pages and sequences in order.
pub(crate) struct Entry {
    pub(crate) info: VersionInfo,
    pub(crate) pages: Value,
    pub(crate) sequences: Value,
}

/// The index entry `value`, which must name `expected`; the text says what
/// is wrong with it.
pub(crate) fn parse_entry(value: &Value, expected: u64) -> Result<Entry, String> {
    let id = value
        .get("id")
        .and_then(Value::as_u64)
        .ok_or("a version has no numeric id")?;
    if id != expected {
        return Err(format!(
            "the index lists version {id} where the manifest lists {expected}"
        ));
    }
    let text = |key: &str, max: usize| -> Result<String, String> {
        let text = value
            .get(key)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("version {id} has no {key}"))?;
        if text.chars().count() > max {
            return Err(format!("the {key} of version {id} is too long"));
        }
        Ok(text.to_owned())
    };
    let name = text("name", MAX_NAME_CHARS)?;
    let app = text("app", MAX_APP_CHARS)?;
    let kind = match value.get("kind").and_then(Value::as_str) {
        Some(NAMED) => VersionKind::Named,
        Some(AUTO) => VersionKind::Auto,
        _ => return Err(format!("version {id} has an unknown kind")),
    };
    let created = value
        .get("created")
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("version {id} has no numeric created"))?;
    let schema = value
        .get("schema")
        .and_then(Value::as_u64)
        .and_then(|schema| u32::try_from(schema).ok())
        .filter(|schema| (1..=Document::SCHEMA_VERSION).contains(schema))
        .ok_or_else(|| format!("version {id} has an unsupported schema"))?;
    let list = |key: &str| -> Result<Value, String> {
        value
            .get(key)
            .filter(|list| list.is_array())
            .cloned()
            .ok_or_else(|| format!("version {id} lists no {key}"))
    };
    Ok(Entry {
        info: VersionInfo {
            id: VersionId(id),
            name,
            kind,
            created: UnixTime(created),
            app,
            schema,
        },
        pages: list("pages")?,
        sequences: list("sequences")?,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn good() -> Value {
        json!({
            "id": 4, "name": "Final", "kind": "named", "created": -5,
            "app": "0.1.0", "schema": 1, "pages": [1], "sequences": []
        })
    }

    #[test]
    fn s31_04_a_good_entry_parses() {
        let entry = parse_entry(&good(), 4).expect("parses");
        assert_eq!(entry.info.id, VersionId(4));
        assert_eq!(entry.info.name, "Final");
        assert_eq!(entry.info.kind, VersionKind::Named);
        assert_eq!(entry.info.created, UnixTime(-5));
        assert_eq!(entry.info.schema, 1);
        assert_eq!(entry.pages, json!([1]));
        assert_eq!(prefix(4), "versions/4/");
    }

    #[test]
    fn s31_04_each_bad_field_is_refused() {
        assert!(parse_entry(&good(), 5).is_err(), "another id");
        for (key, bad) in [
            ("id", json!("4")),
            ("name", json!(7)),
            ("app", json!("x".repeat(65))),
            ("kind", json!("Named")),
            ("created", json!(1.5)),
            ("schema", json!(u64::from(u32::MAX) + 1)),
            ("pages", json!({})),
            ("sequences", Value::Null),
        ] {
            let mut entry = good();
            entry[key] = bad;
            assert!(parse_entry(&entry, 4).is_err(), "{key}");
        }
        let mut auto = good();
        auto["kind"] = json!("auto");
        auto["app"] = json!("x".repeat(64));
        assert_eq!(
            parse_entry(&auto, 4).expect("parses").info.kind,
            VersionKind::Auto
        );
    }
}
