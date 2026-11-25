//! Spec 100 §5, AC-10, AC-11: a 3D model travels inside the `.gaf` with its file, and two
//! elements of one model share that file while each keeps its own view.

use std::io::Cursor;

use gafira_assets::FsStore;
use gafira_doc::{AssetRef, Document, NodeKind};
use gafira_format::{Limits, read_gaf, write_gaf};
use gafira_ports::store::ContentStore;
use serde_json::json;

const CUBE: &[u8] = b"v -1 -1 -1\nv 1 -1 -1\nv 1 1 -1\nv -1 1 -1\nf 1 2 3 4\n";

fn node(id: u64, asset: &str, yaw: f64) -> serde_json::Value {
    json!({
        "base": {"id": id, "name": "Cubo", "placement": {"x": 10.0, "y": 10.0, "w": 80.0, "h": 80.0,
            "rotation_deg": 0.0, "skew": 0.0, "flip": {"x": false, "y": false}},
            "opacity": 1.0, "blend": "normal", "visible": true, "locked": false},
        "kind": {"type": "model3d", "asset": asset, "format": "obj",
            "rotation": [0.0, 0.0, 0.0, 1.0], "position": [0.0, 0.0, 0.0], "scale": [1.0, 1.0, 1.0],
            "camera": {"projection": "perspective", "yaw": yaw, "elevation": 15.0, "zoom": 1.0, "fov": 35.0},
            "lighting": "studio"},
        "children": []
    })
}

#[test]
fn s100_models_travel_inside_the_gaf_and_share_their_file() {
    let dir = std::env::temp_dir().join(format!("gafira-format-3d-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let store = FsStore::new(dir.join("a"));
    let hash = store.put(&mut &CUBE[..]).expect("puts the model");
    let hex = AssetRef::new(hash).hex();
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("../../gafira-doc/tests/golden/minimal.json"))
            .expect("json");
    value["next_id"] = json!(4);
    value["pages"][0]["roots"] = json!([2, 3]);
    value["pages"][0]["nodes"] = json!([node(2, &hex, -30.0), node(3, &hex, 90.0)]);
    value["assets"][&hex] = json!({"name": "cubo.obj", "storage": "embedded"});
    let document =
        Document::from_json(&serde_json::to_vec(&value).expect("json")).expect("a valid document");

    let mut bytes = Cursor::new(Vec::new());
    write_gaf(&mut bytes, &document, &[], None, &store).expect("writes");
    let bytes = bytes.into_inner();
    let zip = zip::ZipArchive::new(Cursor::new(&bytes)).expect("a zip");
    let models: Vec<String> = zip
        .file_names()
        .filter_map(Result::ok)
        .map(|n| n.into_owned())
        .filter(|n| n.starts_with("assets/"))
        .collect();
    // One file for both elements, named by its hash.
    assert_eq!(models, [format!("assets/{hex}.obj")]);

    // Reopened into an empty store, without the original file anywhere else.
    let other = FsStore::new(dir.join("b"));
    let loaded = read_gaf(Cursor::new(bytes), Limits::default(), &other).expect("reads");
    assert!(other.contains(hash).expect("store"), "the model came back");
    let page = &loaded.document.pages()[0];
    let yaws: Vec<f64> = page
        .roots
        .iter()
        .map(|id| match &page.node(*id).expect("node").kind {
            NodeKind::Model3d(m) => m.camera.yaw,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(yaws, [-30.0, 90.0], "each element keeps its own view");
    let _ = std::fs::remove_dir_all(&dir);
}
