//! The `ports` feature of the testkit (ADR 0013, ARQ-112): the default tree
//! carries neither `gafira-ports` nor `gafira-raster`, and the feature turns
//! the port doubles on for a consumer.

use std::process::Command;

/// The `cargo tree` of `gafira-testkit` with the given features.
fn tree(features: &[&str]) -> String {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = Command::new(cargo);
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            "tree",
            "-p",
            "gafira-testkit",
            "-e",
            "normal,build",
            "--locked",
        ])
        .args(features);
    let out = cmd.output().expect("cargo tree runs");
    assert!(
        out.status.success(),
        "cargo tree {features:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

// ARQ-112, ADR 0013: the default tree of the testkit carries neither
// `gafira-ports` nor `gafira-raster`, so the doubles cost nothing when the
// feature is off.
#[test]
fn arq_112_default_testkit_tree_carries_neither_ports_nor_raster() {
    let tree = tree(&[]);
    assert!(!tree.contains("gafira-ports"), "{tree}");
    assert!(!tree.contains("gafira-raster"), "{tree}");
}

// ARQ-112, ADR 0013: the `ports` feature brings the port crates in, so a
// consumer that asks for the doubles gets them.
#[test]
fn arq_112_the_ports_feature_brings_the_port_crates_in() {
    let tree = tree(&["--features", "ports"]);
    assert!(tree.contains("gafira-ports"), "{tree}");
    assert!(tree.contains("gafira-raster"), "{tree}");
}

// The nine doubles are public and reachable from a consumer when the feature
// is on. The test does not run them: they are stubs until the render and the
// media streams fill them.
#[cfg(feature = "ports")]
#[test]
fn arq_112_the_port_doubles_are_reachable_with_the_feature_on() {
    use gafira_testkit::ports::clock::FakeClock;
    use gafira_testkit::ports::codec::FakeCodec;
    use gafira_testkit::ports::container::FakeContainer;
    use gafira_testkit::ports::decoder::FakeDecoder;
    use gafira_testkit::ports::encoder::FakeEncoder;
    use gafira_testkit::ports::file_system::FakeFileSystem;
    use gafira_testkit::ports::muxer::FakeMuxer;
    use gafira_testkit::ports::page_renderer::FakePageRenderer;
    use gafira_testkit::ports::render_backend::FakeRenderBackend;

    let _ = (
        FakeClock,
        FakeCodec,
        FakeContainer,
        FakeDecoder,
        FakeEncoder,
        FakeFileSystem,
        FakeMuxer,
        FakePageRenderer,
        FakeRenderBackend,
    );
}
