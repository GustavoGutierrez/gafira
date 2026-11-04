//! The shape of the visitor contract (ARQ-134), the absence of a time in the
//! measurement context (ARQ-136) and the seam it leaves to the crates above
//! the domain (ARQ-20, ARQ-137).
//!
//! This test crate is not `gafira-doc`: what it writes is what a crate above
//! the domain writes.

use std::path::Path;
use std::sync::Arc;

use gafira_doc::visit::{NodeVisitor, PageView, WalkOrder, walk};
use gafira_doc::{
    ElementBase, ExtensionNode, GroupNode, Node, NodeId, NodeKind, Page, PageId, PageSize,
    Placement, ShapeGeometry, ShapeNode,
};
use gafira_geom::{Length, Unit};

/// The text of the block that starts at `marker`, from its opening brace to
/// the matching closing one.
fn block(source: &str, marker: &str) -> String {
    let start = source.find(marker).expect("the marker is in the source");
    let rest = &source[start + marker.len()..];
    let open = rest.find('{').expect("the block opens with a brace");
    let mut depth = 0usize;
    for (at, c) in rest[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return rest[open..open + at + 1].to_string();
                }
            }
            _ => {}
        }
    }
    panic!("the block that starts at {marker} is not closed");
}

/// The time types the domain must not name: the instant travels apart, as a
/// separate argument, and the domain owns no clock (ARQ-136).
const TIME_TYPES: [&str; 5] = ["Flicks", "FrameRate", "Instant", "Duration", "SystemTime"];

/// The Rust sources of the domain crate, as paths relative to `src/`.
///
/// The sources are read when the test is compiled, never from the disk:
/// `gafira-doc` is a pure crate and its `clippy.toml` forbids the file system
/// (ARQ-02).
macro_rules! domain_sources {
    ($($path:literal),* $(,)?) => {
        &[$(($path, include_str!(concat!("../src/", $path)))),*]
    };
}

/// Every Rust source file of the domain crate.
const DOMAIN_SOURCES: &[(&str, &str)] = domain_sources![
    "lib.rs",
    "arena.rs",
    "blend.rs",
    "content_hash.rs",
    "contract/mod.rs",
    "contract/cancel.rs",
    "contract/describe.rs",
    "contract/params.rs",
    "document.rs",
    "dpi.rs",
    "edit.rs",
    "element.rs",
    "error.rs",
    "ids.rs",
    "json.rs",
    "layout.rs",
    "msg.rs",
    "node.rs",
    "page.rs",
    "page_size.rs",
    "placement.rs",
    "validate.rs",
    "visit.rs",
];

/// True when `source` names `name` as a word of its own.
fn names_type(source: &str, name: &str) -> bool {
    source
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .any(|word| word == name)
}

/// The names of the fields the struct that starts at `marker` declares.
fn fields(source: &str, marker: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in block(source, marker).lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
            continue;
        }
        let Some((head, _)) = line.split_once(':') else {
            continue;
        };
        let Some(name) = head.split_whitespace().last() else {
            continue;
        };
        if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            names.push(name.to_string());
        }
    }
    names
}

/// The modules a source file declares with `mod <name>;`.
fn declared_modules(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = source;
    while let Some(at) = rest.find("mod ") {
        let after = &rest[at + "mod ".len()..];
        let name: String = after
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        // A declaration without a body: `mod name;`.
        if !name.is_empty() && after[name.len()..].trim_start().starts_with(';') {
            names.push(name);
        }
        rest = after;
    }
    names
}

// ARQ-134: the trait declares one `visit_*` method per `NodeKind` variant,
// plus the mandatory `visit_node`.
#[test]
fn arq_134_the_trait_declares_one_method_per_node_kind() {
    let source = include_str!("../src/visit.rs");
    let declaration = block(source, "pub trait NodeVisitor");
    let mut methods = Vec::new();
    let mut rest = declaration.as_str();
    while let Some(at) = rest.find("fn visit_") {
        rest = &rest[at + "fn visit_".len()..];
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        methods.push(format!("visit_{name}"));
    }
    assert_eq!(
        methods,
        [
            "visit_node",
            "visit_group",
            "visit_shape",
            "visit_extension"
        ],
        "the trait declares {methods:?}"
    );
}

// ARQ-134: the dispatch over `NodeKind` is exhaustive and has no catch-all
// arm: a new variant breaks the build here, not in the implementors.
#[test]
fn arq_134_the_dispatch_has_no_catch_all_arm() {
    let source = include_str!("../src/visit.rs");
    let dispatch = block(source, "fn dispatch");
    assert!(
        !dispatch.contains("_ =>"),
        "the dispatch must have no catch-all arm:\n{dispatch}"
    );
    assert_eq!(
        dispatch.matches("NodeKind::").count(),
        3,
        "one arm per NodeKind variant:\n{dispatch}"
    );
    for variant in ["Group", "Shape", "Extension"] {
        assert!(
            dispatch.contains(&format!("NodeKind::{variant}(")),
            "{variant} is not dispatched:\n{dispatch}"
        );
    }
}

// ARQ-137: the domain depends on no crate above it: its manifest names no
// engine of a higher layer.
#[test]
fn arq_137_the_domain_depends_on_no_crate_above_it() {
    let manifest = include_str!("../Cargo.toml");
    for banned in [
        "gafira-render",
        "gafira-media",
        "gafira-layout",
        "gafira-time",
        "gafira-ports",
    ] {
        assert!(
            !manifest.contains(banned),
            "the domain must not depend on {banned}"
        );
    }
}

// ARQ-136: the measurement context carries no time: it holds only the
// borrowed page and the target's DPI, and the domain names no time type.
#[test]
fn arq_136_the_measure_context_carries_no_time() {
    let source = include_str!("../src/visit.rs");
    let context = block(source, "pub struct MeasureCx");

    assert_eq!(
        fields(source, "pub struct MeasureCx"),
        ["page", "dpi"],
        "the context declares only the borrowed page and the target's DPI:\n{context}"
    );

    for name in TIME_TYPES {
        assert!(
            !names_type(&context, name),
            "the context names the time type {name}:\n{context}"
        );
    }

    // Every module the crate declares is examined, so a new one cannot slip
    // into the crate without being checked.
    for (path, text) in DOMAIN_SOURCES {
        for module in declared_modules(text) {
            let dir = Path::new(path).parent().unwrap_or(Path::new(""));
            let files = [
                dir.join(format!("{module}.rs")),
                dir.join(&module).join("mod.rs"),
            ];
            assert!(
                files.iter().any(|f| DOMAIN_SOURCES
                    .iter()
                    .any(|(p, _)| Path::new(p) == f.as_path())),
                "src/{path} declares the module {module}, which the list of sources does not cover: add it"
            );
        }
    }

    for (path, text) in DOMAIN_SOURCES {
        for name in TIME_TYPES {
            assert!(
                !names_type(text, name),
                "the domain names the time type {name} in src/{path}"
            );
        }
    }
}

fn size() -> PageSize {
    PageSize {
        width: Length::new(100.0, Unit::Px),
        height: Length::new(100.0, Unit::Px),
    }
}

/// A page with a group, a shape and an extension, the group holding the other
/// two.
fn page() -> Page {
    let mut p = Page::new(PageId::new(1), "P".to_string(), size());
    for (id, kind) in [
        (1u64, NodeKind::Group(GroupNode {})),
        (2, NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect))),
        (
            3,
            NodeKind::Extension(ExtensionNode {
                type_id: "com.example.sticker".to_string(),
                schema_version: 1,
                payload: serde_json::Value::Null,
            }),
        ),
    ] {
        let base = ElementBase::new(NodeId::new(id), Placement::new(0.0, 0.0, 10.0, 10.0));
        p.nodes
            .insert(NodeId::new(id), Arc::new(Node::new(base, kind)));
    }
    let group = Arc::make_mut(p.nodes.get_mut(&NodeId::new(1)).unwrap());
    group.children = vec![NodeId::new(2), NodeId::new(3)];
    p.roots = vec![NodeId::new(1)];
    p
}

/// A visitor written outside the domain: it counts what it receives.
struct Counter {
    count: usize,
}

impl NodeVisitor for Counter {
    type Out = NodeId;

    fn visit_node(&mut self, n: &Node) -> NodeId {
        self.count += 1;
        n.base.id
    }
}

// ARQ-20: a new operation is a visitor in the crate that needs it: this test
// crate is not `gafira-doc`, and nothing in the domain changed for it.
#[test]
fn arq_20_a_visitor_is_written_outside_the_domain() {
    let p = page();
    let view = PageView::new(&p);
    let mut counter = Counter { count: 0 };
    let visited = walk(&view, &mut counter, WalkOrder::PreOrder);
    assert_eq!(counter.count, 3);
    assert_eq!(
        visited.iter().map(|id| id.get()).collect::<Vec<u64>>(),
        [1, 2, 3]
    );
}
