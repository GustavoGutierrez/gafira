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

/// The time types the domain must not name outside the data of the timeline:
/// the instant travels apart, as a separate argument, and the domain owns no
/// clock (ARQ-136).
const TIME_TYPES: [&str; 5] = ["Flicks", "FrameRate", "Instant", "Duration", "SystemTime"];

/// The clock types: the domain names none of them in any file, the timeline's
/// data files included. `Flicks` and `FrameRate` are data; these three are a
/// clock (ARQ-136).
const CLOCK_TYPES: [&str; 3] = ["Instant", "Duration", "SystemTime"];

/// The data types of the timeline: only the sources that hold the timeline of
/// 02 §3.1 may name them (ARQ-136).
const TIMELINE_TYPES: [&str; 2] = ["Flicks", "FrameRate"];

/// The sources that hold the data of the timeline of 02 §3.1: the domain
/// names the time types there and nowhere else (ARQ-136).
const TIMELINE_SOURCES: [&str; 4] = [
    "page.rs",
    "timeline/clip.rs",
    "timeline/sequence.rs",
    "timeline/track.rs",
];

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
    "image.rs",
    "image_look.rs",
    "json.rs",
    "background.rs",
    "chart.rs",
    "effect.rs",
    "frame.rs",
    "gradient.rs",
    "layout.rs",
    "line.rs",
    "msg.rs",
    "node.rs",
    "page.rs",
    "page_size.rs",
    "timeline/mod.rs",
    "timeline/asset.rs",
    "timeline/clip.rs",
    "timeline/error.rs",
    "timeline/sequence.rs",
    "timeline/track.rs",
    "placement.rs",
    "project_id.rs",
    "qr.rs",
    "table.rs",
    "text.rs",
    "text_effect.rs",
    "validate.rs",
    "visit.rs",
];

/// True when `source` names `name` as a word of its own.
fn names_type(source: &str, name: &str) -> bool {
    source
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .any(|word| word == name)
}

/// The time types `path` names in its code that it may not name (ARQ-136).
///
/// A timeline source may name the two data types of the timeline; a clock
/// type is refused everywhere, because the instant travels apart from the
/// measurement and the domain owns no clock. A source outside the timeline
/// names none of the five.
fn time_violations(path: &str, source: &str) -> Vec<&'static str> {
    let code = code_only(source);
    let banned: &[&str] = if TIMELINE_SOURCES.contains(&path) {
        &CLOCK_TYPES
    } else {
        &TIME_TYPES
    };
    banned
        .iter()
        .copied()
        .filter(|name| names_type(&code, name))
        .collect()
}

/// `source` with its comments and its string literals removed: what is left
/// is code. A type is named in code; the word for it in a doc comment, in an
/// assertion message or in a string is prose (ARQ-136).
fn code_only(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '/' if chars.peek() == Some(&'/') => {
                chars.next();
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut depth = 1usize;
                while depth > 0 {
                    match chars.next() {
                        Some('/') if chars.peek() == Some(&'*') => {
                            chars.next();
                            depth += 1;
                        }
                        Some('*') if chars.peek() == Some(&'/') => {
                            chars.next();
                            depth -= 1;
                        }
                        Some(_) => {}
                        None => break,
                    }
                }
            }
            '"' => skip_string(&mut chars),
            'r' => {
                if !skip_raw(&mut chars) {
                    out.push(c);
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// Consumes a string literal whose opening quote was already read.
fn skip_string(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            '"' => return,
            _ => {}
        }
    }
}

/// Consumes a raw string, `r"..."` or `r#"..."#`, when one starts here; the
/// `r` was already read. False when this is a raw identifier (`r#type`).
fn skip_raw(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> bool {
    let mut probe = chars.clone();
    let mut hashes = 0usize;
    while probe.peek() == Some(&'#') {
        probe.next();
        hashes += 1;
    }
    if probe.peek() != Some(&'"') {
        return false;
    }
    for _ in 0..hashes {
        chars.next();
    }
    chars.next();
    loop {
        match chars.next() {
            Some('"') => {
                let mut seen = 0usize;
                while seen < hashes && chars.peek() == Some(&'#') {
                    chars.next();
                    seen += 1;
                }
                if seen == hashes {
                    return true;
                }
            }
            Some(_) => {}
            // An unclosed raw string: nothing more can hide after it.
            None => return true,
        }
    }
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
            "visit_text",
            "visit_image",
            "visit_line",
            "visit_frame",
            "visit_qr",
            "visit_table",
            "visit_chart",
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
        10,
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
// engine of a higher layer. `gafira-time` is layer 0 — below the domain, never
// above it — and the edge to it is the one ARQ-136 names as expected and
// bounded, so it is not part of this ban.
#[test]
fn arq_137_the_domain_depends_on_no_crate_above_it() {
    let manifest = include_str!("../Cargo.toml");
    for banned in [
        "gafira-render",
        "gafira-media",
        "gafira-layout",
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
        let found = time_violations(path, text);
        assert!(
            found.is_empty(),
            "src/{path} names the time type(s) {found:?}, outside what ARQ-136 allows there"
        );
    }
}

// ARQ-136: a source that holds the timeline's data may name `Flicks` and
// `FrameRate`, and still may not name a clock. The whole file is not exempt:
// the rule reads what the file names, not where it lives.
#[test]
fn arq_136_a_clock_type_inside_a_timeline_source_is_refused() {
    for (path, probe, refused) in [
        (
            "page.rs",
            "fn stamp() -> std::time::Instant { todo!() }",
            "Instant",
        ),
        (
            "timeline/clip.rs",
            "fn age() -> std::time::Duration { todo!() }",
            "Duration",
        ),
        (
            "timeline/track.rs",
            "fn start() -> std::time::SystemTime { todo!() }",
            "SystemTime",
        ),
    ] {
        assert_eq!(
            time_violations(path, probe),
            [refused],
            "src/{path} must refuse the clock type {refused}"
        );
    }

    // The two data types are legal in a timeline source, and nowhere else.
    for name in TIMELINE_TYPES {
        let names_it = format!("use gafira_time::{name};");
        assert!(
            time_violations("page.rs", &names_it).is_empty(),
            "a timeline source may name the data type {name}"
        );
        assert_eq!(
            time_violations("validate.rs", &names_it),
            [name],
            "a source outside the timeline may not name {name}"
        );
    }

    // The five names are exactly the two data types and the three clock
    // types: a name in one list and not the other is a hole in the rule.
    let mut listed: Vec<&str> = TIMELINE_TYPES
        .iter()
        .chain(CLOCK_TYPES.iter())
        .copied()
        .collect();
    listed.sort_unstable();
    let mut expected = TIME_TYPES.to_vec();
    expected.sort_unstable();
    assert_eq!(listed, expected);
}

// ARQ-136: the guard reads the code, not the words around it. A `Duration` in
// a comment or a string is prose; the field `duration` and the identifier
// `durations` are not the type. Only the type in code counts.
#[test]
fn arq_136_the_guard_tells_a_type_from_a_word() {
    let prose = concat!(
        "// An Instant is not named in this file.\n",
        "/// The page duration, and a Duration is prose here.\n",
        "/* SystemTime and Duration, in a block comment. */\n",
        "fn note(duration: i64, durations: &str) -> i64 {\n",
        "    let _ = \"Duration\";\n",
        "    let _ = r#\"Instant\"#;\n",
        "    duration + durations.len() as i64\n",
        "}\n",
    );
    assert!(
        time_violations("page.rs", prose).is_empty(),
        "prose and identifiers are not types: {:?}",
        time_violations("page.rs", prose)
    );
    assert_eq!(
        time_violations("page.rs", "use std::time::Duration;"),
        ["Duration"]
    );
    assert_eq!(
        time_violations("page.rs", "fn wait(clock: std::time::Instant) {}"),
        ["Instant"]
    );
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
