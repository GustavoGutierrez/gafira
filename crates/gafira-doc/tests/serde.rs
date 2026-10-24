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

const EXTENSION: &str = include_str!("golden/extension.json");

/// Loads the extension golden with its payload text replaced, re-saved.
fn resave_payload(payload: &str) -> Result<String, LoadError> {
    let start = EXTENSION.find(r#""payload":"#).unwrap() + r#""payload":"#.len();
    let end = EXTENSION.rfind(r#"},"children""#).unwrap();
    let text = format!("{}{payload}{}", &EXTENSION[..start], &EXTENSION[end..]);
    let out = Document::from_json(text.as_bytes())?.to_json().unwrap();
    Ok(out[start..out.len() - (EXTENSION.len() - end)].to_string())
}

#[test]
fn payload_numbers_and_keys_get_one_canonical_spelling() {
    let rows = [
        (
            r#"{"b":1,"a":{"z":0,"y":1}}"#,
            r#"{"a":{"y":1,"z":0},"b":1}"#,
        ),
        ("1.10", "1.1"),
        ("1", "1"),
        ("1.0", "1.0"),
        ("10E-1", "1.0"),
        ("18446744073709551615", "18446744073709551615"),
        ("18446744073709551616", "1.8446744073709552e+19"),
        (r#"{"a":1,"a":2}"#, r#"{"a":2}"#),
    ];
    for (input, want) in rows {
        let got = resave_payload(input).unwrap();
        assert_eq!(got, want, "{input}");
        assert_eq!(resave_payload(&got).unwrap(), got, "{input} is not stable");
    }
}

#[test]
fn unrepresentable_payload_number_is_a_json_error() {
    assert!(matches!(resave_payload("1e400"), Err(LoadError::Json(_))));
    assert!(matches!(resave_payload("-1e400"), Err(LoadError::Json(_))));
}

const MINIMAL: &str = include_str!("golden/minimal.json");

fn is_json_error(bytes: &[u8]) -> bool {
    matches!(Document::from_json(bytes), Err(LoadError::Json(_)))
}

#[test]
fn every_prefix_of_the_minimal_golden_is_a_json_error() {
    assert!(Document::from_json(MINIMAL.as_bytes()).is_ok());
    for n in 0..MINIMAL.len() {
        assert!(is_json_error(&MINIMAL.as_bytes()[..n]), "prefix {n}");
    }
}

#[test]
fn wrong_types_are_json_errors() {
    for text in ["null", "[]", r#""x""#, "7", "true"] {
        assert!(is_json_error(text.as_bytes()), "{text}");
    }
    let rows = [
        ("/next_id", r#""7""#),
        ("/pages", "{}"),
        ("/pages/0/locked", r#""yes""#),
        ("/pages/0/nodes/0/base/name", "5"),
        ("/pages/0/nodes/0/base/opacity", "null"),
        ("/pages/0/nodes/0/children", "{}"),
    ];
    for (pointer, wrong) in rows {
        let mut v = full();
        *v.pointer_mut(pointer).unwrap() = serde_json::from_str(wrong).unwrap();
        assert!(matches!(load(&v), Err(LoadError::Json(_))), "{pointer}");
    }
}

#[test]
fn bad_numbers_are_json_errors() {
    let bad = ["-1", "1.5", "1e3", "null", "18446744073709551616"];
    let fields = [
        r#""next_id":20"#,
        r#""dpi":300"#,
        r#""id":10,"#,
        r#""id":5,"#,
        r#""roots":[2,3,4]"#,
    ];
    for field in fields {
        let (key, old) = field.split_once(':').unwrap();
        for b in bad {
            let new = if field.starts_with(r#""roots""#) {
                format!("{key}:[{b},3,4]")
            } else {
                format!("{key}:{b}{}", if old.ends_with(',') { "," } else { "" })
            };
            let text = FULL.replacen(field, &new, 1);
            assert_ne!(text, FULL, "{field}");
            assert!(is_json_error(text.as_bytes()), "{new}");
        }
    }
    let dpi_overflow = FULL.replacen(r#""dpi":300"#, r#""dpi":4294967296"#, 1);
    assert!(is_json_error(dpi_overflow.as_bytes()));
    let float_overflow = FULL.replacen(r#""value":21.0"#, r#""value":1e400"#, 1);
    assert_ne!(float_overflow, FULL);
    assert!(is_json_error(float_overflow.as_bytes()));
}

#[test]
fn duplicate_struct_fields_are_json_errors() {
    let rows = [
        (r#""next_id":20,"#, r#""next_id":20,"next_id":21,"#),
        (r#""opacity":0.5,"#, r#""opacity":0.5,"opacity":0.25,"#),
        (
            r#""geometry":"ellipse","#,
            r#""geometry":"ellipse","geometry":"rect","#,
        ),
    ];
    for (from, to) in rows {
        let text = FULL.replacen(from, to, 1);
        assert_ne!(text, FULL, "{from}");
        assert!(is_json_error(text.as_bytes()), "{to}");
    }
}

#[test]
fn finite_out_of_range_values_survive_serde_by_bits() {
    use gafira_doc::{ElementBase, GroupNode, Node, NodeId, NodeKind, Placement};
    let mut base = ElementBase::new(NodeId::new(5), Placement::new(0.0, 0.0, 1.0, 1.0));
    base.opacity = 1.5;
    base.placement.rotation_deg = 1e300;
    let node = Node::new(base, NodeKind::Group(GroupNode {}));
    let json = serde_json::to_string(&node).unwrap();
    let back: Node = serde_json::from_str(&json).unwrap();
    assert_eq!(back.base.opacity.to_bits(), 1.5f32.to_bits());
    assert_eq!(
        back.base.placement.rotation_deg.to_bits(),
        1e300f64.to_bits()
    );
    assert_eq!(back, node);
}
