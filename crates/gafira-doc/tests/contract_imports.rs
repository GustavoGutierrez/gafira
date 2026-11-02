//! ADR 0005: `gafira_doc::contract` imports only `crate::msg`, `gafira_color`,
//! `serde` and `std` (ARQ-01 layer rule). The sources are embedded, so no file
//! is read at run time.
//!
//! A new file under `src/contract/` must be added to `SOURCES`; a test fails
//! until it is (it compares `SOURCES` with the `mod` lines of `mod.rs`).

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

/// Primitive types that start a path (`f64::NAN`).
const PRIMITIVES: &[&str] = &[
    "bool", "char", "str", "f32", "f64", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16",
    "u32", "u64", "u128", "usize",
];

/// Byte offsets of `word` where no identifier character comes just before.
fn word_starts<'a>(code: &'a str, word: &'a str) -> impl Iterator<Item = usize> + 'a {
    code.match_indices(word)
        .map(|(at, _)| at)
        .filter(|&at| !code[..at].chars().next_back().is_some_and(is_ident))
}

/// What breaks the import rule in `src`, one line of text per fault.
fn violations(src: &str) -> Vec<String> {
    let code = strip_comments(src);
    let mut found = Vec::new();

    let declared: Vec<&str> = word_starts(&code, "mod ")
        .map(|at| ident(&code[at + "mod ".len()..]))
        .collect();

    if word_starts(&code, "extern crate").next().is_some() {
        found.push("`extern crate`".to_owned());
    }

    // Every `use`, wherever it stands: after an item, inside a function.
    let mut use_spans = Vec::new();
    // Names a `use` brings in (`use std::fmt;` lets `fmt::Result` follow).
    let mut imported: Vec<String> = Vec::new();
    for at in word_starts(&code, "use ") {
        let tree = code[at + "use ".len()..].split(';').next().unwrap_or("");
        use_spans.push(at..at + "use ".len() + tree.len());
        for path in expand(tree) {
            let path = path.trim_start_matches("::");
            imported.push(ident(path.rsplit("::").next().unwrap_or("")).to_owned());
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

    // Paths written out in code, with no `use`: the root of every `a::b`
    // chain. A type or variant (`ParamKind::Float`) starts with a capital.
    for (at, _) in code.match_indices("::") {
        if use_spans.iter().any(|span| span.contains(&at)) {
            continue;
        }
        let head = &code[..at];
        let start = head
            .rfind(|c: char| !(is_ident(c) || c == ':'))
            .map_or(0, |i| {
                i + head[i..].chars().next().map_or(1, char::len_utf8)
            });
        let chain = head[start..].trim_start_matches(':');
        let root = ident(chain);
        if root.is_empty() || chain.contains("::") || root.starts_with(char::is_uppercase) {
            continue;
        }
        if PRIMITIVES.contains(&root)
            || declared.contains(&root)
            || imported.iter().any(|i| i == root)
        {
            continue;
        }
        if ALLOWED_ROOTS.contains(&root) && root != "crate" {
            continue;
        }
        if root == "crate" {
            let after = &code[at + 2..];
            if !after.starts_with('{') && !ALLOWED_CRATE_PATHS.contains(&ident(after)) {
                found.push(format!("path `crate::{}`", ident(after)));
            }
            continue;
        }
        found.push(format!("path `{root}::`"));
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
        "fn f() { let _ = serde_json::Value::Null; }",
        "fn f(_: ::serde_json::Value) {}",
        "fn f() {}\nuse serde_json::Value;",
        "fn f() { if x { } use serde_json::Value; }",
        "struct S {}use kurbo::Point;",
        "fn f() { use kurbo::Point; use serde_json::Value; }",
        "fn f() { let _ = kurbo::Point::ORIGIN; }",
        "use crate::msg::Message;\nfn f() { let _ = crate::Document::new(); }",
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
        "fn f() { let _ = f64::NAN; let _: std::fmt::Result = Ok(()); let _ = Vec::<u8>::new(); }",
        "mod inner;\nfn f() { inner::g(); self::h(); super::k(); }",
        "fn f() {}\nuse std::fmt;\nfn g() { use serde::Serialize; }",
    ];
    for src in accepted {
        assert_eq!(
            violations(src),
            Vec::<String>::new(),
            "should accept {src:?}"
        );
    }
}

#[test]
fn adr_0005_sources_lists_every_module_mod_rs_declares() {
    // A file that `mod.rs` does not declare is not compiled, so the
    // declarations are the complete list (ARQ-02 forbids reading the disk).
    let (_, mod_rs) = SOURCES[0];
    let code = strip_comments(mod_rs);
    let mut declared: Vec<String> = word_starts(&code, "mod ")
        .map(|at| format!("{}.rs", ident(&code[at + "mod ".len()..])))
        .collect();
    declared.push("mod.rs".to_owned());
    declared.sort();
    let mut listed: Vec<String> = SOURCES.iter().map(|(n, _)| (*n).to_owned()).collect();
    listed.sort();
    assert_eq!(listed, declared, "SOURCES must list every module of mod.rs");
}
