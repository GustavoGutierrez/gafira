//! Black-box serde tests on JSON fixtures, loaded without validation in D3.

use gafira_doc::{Document, LoadError};
use serde_json::Value;

const FULL: &str = include_str!("golden/full.json");

fn load(v: &Value) -> Result<Document, LoadError> {
    Document::from_json(v.to_string().as_bytes())
}

fn full() -> Value {
    serde_json::from_str(FULL).unwrap()
}

/// Removes the value at a JSON pointer and returns it.
fn remove(v: &mut Value, pointer: &str) -> Option<Value> {
    let (parent, key) = pointer.rsplit_once('/').unwrap();
    match v.pointer_mut(parent)? {
        Value::Object(m) => m.remove(key),
        Value::Array(a) => Some(a.remove(key.parse().ok()?)),
        _ => None,
    }
}

#[test]
fn missing_required_fields_are_errors() {
    assert!(load(&full()).is_ok());
    let rows = [
        "/schema_version",
        "/next_id",
        "/pages",
        "/pages/0/id",
        "/pages/0/size",
        "/pages/0/size/width/value",
        "/pages/0/size/width/unit",
        "/pages/0/nodes/0/base",
        "/pages/0/nodes/0/kind",
        "/pages/0/nodes/0/base/id",
        "/pages/0/nodes/0/base/placement",
        "/pages/0/nodes/0/base/placement/x",
        "/pages/0/nodes/0/base/placement/y",
        "/pages/0/nodes/0/base/placement/w",
        "/pages/0/nodes/0/base/placement/h",
        "/pages/0/nodes/0/kind/type",
        "/pages/0/nodes/1/kind/geometry",
        "/pages/0/nodes/1/kind/fill/space",
        "/pages/0/nodes/1/kind/fill/r",
        "/pages/0/nodes/2/kind/type_id",
        "/pages/0/nodes/2/kind/schema_version",
        "/pages/0/nodes/2/kind/payload",
    ];
    for row in rows {
        let mut v = full();
        assert!(remove(&mut v, row).is_some(), "{row} is not in the fixture");
        assert!(matches!(load(&v), Err(LoadError::Json(_))), "{row}");
    }
}

#[test]
fn unknown_vocabulary_is_an_error() {
    let rows = [
        (r#""type":"group""#, r#""type":"text""#),
        (r#""type":"group""#, r#""type":"Group""#),
        (r#""geometry":"ellipse""#, r#""geometry":"star""#),
        (r#""blend":"normal""#, r#""blend":"Multiply""#),
        (r#""unit":"cm""#, r#""unit":"parsec""#),
        (r#""space":"srgb""#, r#""space":"rec2020""#),
    ];
    for (from, to) in rows {
        let text = FULL.replacen(from, to, 1);
        assert_ne!(text, FULL, "{from}");
        assert!(
            matches!(
                Document::from_json(text.as_bytes()),
                Err(LoadError::Json(_))
            ),
            "{to}"
        );
    }
}

#[test]
fn unknown_fields_are_ignored_and_dropped() {
    let pointers = [
        "",
        "/pages/0",
        "/pages/0/size",
        "/pages/0/nodes/0",
        "/pages/0/nodes/0/base",
        "/pages/0/nodes/0/base/placement",
        "/pages/0/nodes/0/base/placement/flip",
        "/pages/0/nodes/1/kind",
        "/pages/0/nodes/1/kind/fill",
        "/pages/0/nodes/2/kind",
    ];
    let mut v = full();
    for p in pointers {
        let o = v.pointer_mut(p).unwrap().as_object_mut().unwrap();
        o.insert("zz_extra".to_string(), serde_json::json!({"n": [1]}));
    }
    assert!(v.to_string().contains("zz_extra"));
    let d = load(&v).unwrap();
    assert_eq!(d, Document::from_json(FULL.as_bytes()).unwrap());
    let out = d.to_json().unwrap();
    assert!(!out.contains("zz_extra"));
    assert_eq!(out, FULL);
}

#[test]
fn saving_normalizes_numbers_node_order_and_payload() {
    let base = |id: u64| {
        format!(
            r#"{{"base":{{"id":{id},"placement":{{"x":3,"y":0,"w":1,"h":1}}}},"kind":{{"type":"group"}}}}"#
        )
    };
    let ext = r#"{"base":{"id":9,"placement":{"x":0,"y":0,"w":1,"h":1}},"kind":{"type":"extension","type_id":"t","schema_version":1,"payload":{"b":1,"a":{"z":0,"y":1}}}}"#;
    let text = format!(
        r#"{{"schema_version":1,"next_id":10,"pages":[{{"id":1,"size":{{"width":{{"value":1,"unit":"px"}},"height":{{"value":1,"unit":"px"}}}},"roots":[7,2,9],"nodes":[{},{},{ext}]}}]}}"#,
        base(7),
        base(2)
    );
    let out = Document::from_json(text.as_bytes())
        .unwrap()
        .to_json()
        .unwrap();
    assert!(out.contains(r#""x":3.0"#) && !out.contains(r#""x":3,"#));
    let (two, seven) = (
        out.find(r#""id":2,"#).unwrap(),
        out.find(r#""id":7,"#).unwrap(),
    );
    assert!(two < seven);
    assert!(out.contains(r#""payload":{"a":{"y":1,"z":0},"b":1}"#));
    assert!(out.contains(r#""roots":[7,2,9]"#));
}
