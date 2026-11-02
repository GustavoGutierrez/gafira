//! ADR 0005: `gafira_doc::contract` imports only `crate::msg`, `gafira_color`,
//! `serde` and `std` (ARQ-01 layer rule). The sources are embedded, so no file
//! is read at run time.
//!
//! A new file under `src/contract/` must be added to `SOURCES`.

const SOURCES: &[(&str, &str)] = &[
    ("mod.rs", include_str!("../src/contract/mod.rs")),
    ("describe.rs", include_str!("../src/contract/describe.rs")),
    ("params.rs", include_str!("../src/contract/params.rs")),
];

/// Roots a `use` may start from, besides the modules the file declares.
const ALLOWED_ROOTS: &[&str] = &["std", "serde", "gafira_color", "crate", "self", "super"];
/// What may follow `crate::`.
const ALLOWED_CRATE_PATHS: &[&str] = &["msg", "contract"];

fn strip_comments(src: &str) -> String {
    src.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The identifier that starts `s`.
fn ident(s: &str) -> &str {
    let end = s.find(|c| !is_ident(c)).unwrap_or(s.len());
    &s[..end]
}

/// Every path a `use` tree names, with the braces expanded.
fn expand(tree: &str) -> Vec<String> {
    let tree: String = tree.chars().filter(|c| !c.is_whitespace()).collect();
    fn go(prefix: &str, tree: &str, out: &mut Vec<String>) {
        let Some(open) = tree.find('{') else {
            out.push(format!("{prefix}{tree}"));
            return;
        };
        let head = &tree[..open];
        let close = tree.rfind('}').unwrap_or(tree.len());
        let (mut depth, mut start) = (0, open + 1);
        let inner = &tree[open + 1..close];
        let mut parts = Vec::new();
        for (i, c) in inner.char_indices() {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                ',' if depth == 0 => {
                    parts.push(&tree[start..open + 1 + i]);
                    start = open + 2 + i;
                }
                _ => {}
            }
        }
        parts.push(&tree[start..close]);
        for p in parts.into_iter().filter(|p| !p.is_empty()) {
            go(&format!("{prefix}{head}"), p, out);
        }
    }
    let mut out = Vec::new();
    go("", &tree, &mut out);
    out
}

/// What breaks the import rule in `src`, one line of text per fault.
fn violations(src: &str) -> Vec<String> {
    let code = strip_comments(src);
    let mut found = Vec::new();

    let declared: Vec<&str> = code
        .split(';')
        .filter_map(|s| {
            let s = s.trim().strip_prefix("pub ").unwrap_or(s.trim());
            s.strip_prefix("mod ").map(ident)
        })
        .collect();

    for stmt in code.split(';') {
        let stmt = stmt.trim();
        let stmt = stmt.strip_prefix("pub ").unwrap_or(stmt);
        if stmt.starts_with("extern crate") {
            found.push(format!("`{stmt}`"));
        }
        let Some(tree) = stmt.strip_prefix("use ") else {
            continue;
        };
        for path in expand(tree) {
            let path = path.trim_start_matches("::");
            let root = ident(path);
            if !ALLOWED_ROOTS.contains(&root) && !declared.contains(&root) {
                found.push(format!("use of `{path}`"));
            }
            if let Some(rest) = path.strip_prefix("crate::")
                && !ALLOWED_CRATE_PATHS.contains(&ident(rest))
            {
                found.push(format!("use of `{path}`"));
            }
        }
    }

    // Paths written out in code, with no `use`.
    let mut rest = code.as_str();
    while let Some(at) = rest.find("crate::") {
        let before = rest[..at].chars().next_back();
        let after = &rest[at + "crate::".len()..];
        // `crate::{..}` is a `use` tree, already checked path by path above.
        let in_tree = after.starts_with('{');
        if !in_tree && !before.is_some_and(is_ident) && !ALLOWED_CRATE_PATHS.contains(&ident(after))
        {
            found.push(format!("path `crate::{}`", ident(after)));
        }
        rest = after;
    }
    let mut rest = code.as_str();
    while let Some(at) = rest.find("gafira_") {
        let before = rest[..at].chars().next_back();
        let name = ident(&rest[at..]);
        if !before.is_some_and(is_ident) && name != "gafira_color" {
            found.push(format!("path `{name}`"));
        }
        rest = &rest[at + "gafira_".len()..];
    }
    found
}

#[test]
fn adr_0005_contract_imports_only_msg_color_serde_and_std() {
    for (name, src) in SOURCES {
        assert_eq!(violations(src), Vec::<String>::new(), "{name}");
    }
}

#[test]
fn adr_0005_the_checker_names_each_kind_of_forbidden_import() {
    let rejected = [
        "use crate::Node;",
        "use crate::{msg::Message, Page};",
        "use crate::document::Document;",
        "use crate::msg::Message;\nuse crate::NodeId;",
        "fn f() { let _ = crate::Document::new(); }",
        "use gafira_geom::Rect;",
        "use gafira_doc::Page;",
        "fn f(_: gafira_geom::Rect) {}",
        "use serde_json::Value;",
        "use kurbo::Point;",
        "extern crate gafira_geom;",
        "use ::gafira_geom::Rect;",
    ];
    for src in rejected {
        assert!(!violations(src).is_empty(), "should flag {src:?}");
    }
    let accepted = [
        "use std::fmt;",
        "use std::collections::BTreeMap;",
        "use gafira_color::Color;",
        "use serde::{Deserialize, Serialize};",
        "use crate::msg::{Message, MessageKey, MsgArg, ToMessage};",
        "use crate::{msg::Message, contract::Descriptor};",
        "mod describe;\npub use describe::{Describe, Descriptor};",
        "// use crate::Node;\n/// see gafira_geom\nfn f() {}",
        "fn f() { let _ = crate::msg::MessageKey(\"a.b\"); let _ = gafira_color::Color::default; }",
    ];
    for src in accepted {
        assert_eq!(
            violations(src),
            Vec::<String>::new(),
            "should accept {src:?}"
        );
    }
}
