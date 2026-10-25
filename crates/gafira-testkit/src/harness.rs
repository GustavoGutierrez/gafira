//! `prop_command`: the property harness every core command runs (02 §10).
//!
//! Given a generated valid document and generated commands for it, the
//! harness checks six laws:
//!
//! 1. `apply; revert` gives a document equal to the original;
//! 2. `apply; revert; apply` gives a document equal to the first `apply`;
//! 3. after a successful `apply`, `validate()` passes;
//! 4. `touches` is exact: every page or node that changed is listed, and
//!    every listed page or node exists in the original or in the result;
//! 5. on `Err`, the document is pointer-equal to the original;
//! 6. when two commands merge, applying the merged command equals applying
//!    the two in sequence.
//!
//! A page counts as changed when its own fields or its `roots` differ; a
//! change confined to its nodes is reported on those nodes (`DirtySet`).
//! A node counts as changed when its `Arc` is not pointer-equal, or when it
//! exists on one side only.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use gafira_doc::{Document, EditToken, Node, NodeId, Page};
use gafira_edit::{Command, DirtySet};
use proptest::prelude::*;
use proptest::test_runner::{Config, TestError, TestRunner};

use crate::document;

/// Cases per run unless `PROPTEST_CASES` says otherwise.
const DEFAULT_CASES: u32 = 1_000;

/// A broken law, with the smallest input proptest found for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LawViolation {
    /// The number of the law, 1 to 6.
    pub law: u8,
    /// What differed.
    pub detail: String,
    /// The minimized `(document, command, next command)`, as `Debug` text.
    pub counterexample: String,
}

impl fmt::Display for LawViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "law {} broken: {}\nminimized input: {}",
            self.law, self.detail, self.counterexample
        )
    }
}

impl std::error::Error for LawViolation {}

/// Runs the six laws over generated documents and commands; panics with the
/// minimized counterexample on the first broken law.
///
/// `commands` builds a strategy of commands that target the given document.
/// It is called twice per case: the second command feeds law 6.
pub fn prop_command<C, S>(commands: impl Fn(&Document) -> S)
where
    C: Command + Clone,
    S: Strategy<Value = C>,
{
    if let Err(violation) = try_prop_command(commands) {
        panic!("{violation}");
    }
}

/// Like [`prop_command`], but returns the broken law instead of panicking.
pub fn try_prop_command<C, S>(commands: impl Fn(&Document) -> S) -> Result<(), LawViolation>
where
    C: Command + Clone,
    S: Strategy<Value = C>,
{
    let cases = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(DEFAULT_CASES);
    let mut runner = TestRunner::new(Config {
        cases,
        failure_persistence: None,
        ..Config::default()
    });
    let input = document().prop_flat_map(move |doc| {
        let (first, second) = (commands(&doc), commands(&doc));
        (Just(doc), first, second)
    });
    match runner.run(&input, |(doc, first, second)| {
        check(&doc, &first, &second)
            .map_err(|(law, detail)| TestCaseError::fail(format!("law {law} broken: {detail}")))
    }) {
        Ok(()) => Ok(()),
        Err(TestError::Fail(_, (doc, first, second))) => {
            let (law, detail) = check(&doc, &first, &second)
                .expect_err("a shrunk failing input fails again: the laws are deterministic");
            Err(LawViolation {
                law,
                detail,
                counterexample: format!("{doc:?}\n{first:?}\n{second:?}"),
            })
        }
        Err(TestError::Abort(reason)) => panic!("proptest aborted: {reason}"),
    }
}

type Broken = (u8, String);

/// Applies `command` to a copy of `doc`.
fn applied<C: Command + Clone>(doc: &Document, command: &C) -> (Document, C, Result<(), Broken>) {
    let token = EditToken::__new_for_gafira_edit_only();
    let (mut out, mut command) = (doc.clone(), command.clone());
    let result = command
        .apply(&mut out.edit(&token))
        .map(|_| ())
        .map_err(|e| (5, format!("apply failed: {e}")));
    (out, command, result)
}

fn check<C: Command + Clone>(original: &Document, first: &C, second: &C) -> Result<(), Broken> {
    let token = EditToken::__new_for_gafira_edit_only();
    let (after, mut command, result) = applied(original, first);
    if result.is_err() {
        // Law 5: a failed apply changes nothing.
        return if same_document(original, &after) {
            Ok(())
        } else {
            Err((5, "a failed apply changed the document".into()))
        };
    }
    after
        .validate()
        .map_err(|e| (3, format!("the document is invalid after apply: {e}")))?;
    check_touches(original, &after, &command.touches()).map_err(|d| (4, d))?;

    let mut reverted = after.clone();
    command
        .revert(&mut reverted.edit(&token))
        .map_err(|e| (1, format!("revert failed: {e}")))?;
    if reverted != *original {
        return Err((1, "apply; revert differs from the original".into()));
    }
    command
        .apply(&mut reverted.edit(&token))
        .map_err(|e| (2, format!("the second apply failed: {e}")))?;
    if reverted != after {
        return Err((
            2,
            "apply; revert; apply differs from the first apply".into(),
        ));
    }
    check_merge(original, first, second)
}

/// Law 6, for commands whose `merge` accepts the next command.
fn check_merge<C: Command + Clone>(
    original: &Document,
    first: &C,
    second: &C,
) -> Result<(), Broken> {
    let token = EditToken::__new_for_gafira_edit_only();
    let mut merged = first.clone();
    if merged.merge(Box::new(second.clone())).is_err() {
        return Ok(());
    }
    let (mut sequence, mut first, mut second) = (original.clone(), first.clone(), second.clone());
    if first.apply(&mut sequence.edit(&token)).is_err()
        || second.apply(&mut sequence.edit(&token)).is_err()
    {
        return Ok(());
    }
    let mut together = original.clone();
    merged
        .apply(&mut together.edit(&token))
        .map_err(|e| (6, format!("the merged command failed: {e}")))?;
    if together != sequence {
        return Err((6, "the merged command differs from the sequence".into()));
    }
    Ok(())
}

/// Equal in value, and sharing every page with `original`.
fn same_document(original: &Document, other: &Document) -> bool {
    original == other
        && original
            .pages()
            .iter()
            .zip(other.pages())
            .all(|(a, b)| Arc::ptr_eq(a, b))
}

/// The page without its arena.
fn skeleton(page: &Page) -> Page {
    let mut bare = page.clone();
    bare.nodes.clear();
    bare
}

fn nodes_of(doc: &Document) -> BTreeMap<NodeId, &Arc<Node>> {
    doc.pages()
        .iter()
        .flat_map(|page| page.nodes.iter())
        .map(|(id, node)| (*id, node))
        .collect()
}

fn check_touches(before: &Document, after: &Document, dirty: &DirtySet) -> Result<(), String> {
    let page_ids = before
        .pages()
        .iter()
        .chain(after.pages())
        .map(|page| page.id)
        .collect::<std::collections::BTreeSet<_>>();
    for id in &page_ids {
        let changed = match (before.page(*id), after.page(*id)) {
            (Some(b), Some(a)) => skeleton(b) != skeleton(a),
            _ => true,
        };
        if changed && !dirty.pages.contains(id) {
            return Err(format!("page {id} changed but is not in touches"));
        }
    }
    if let Some(id) = dirty.pages.iter().find(|id| !page_ids.contains(id)) {
        return Err(format!("touches lists page {id}, which does not exist"));
    }
    let (old, new) = (nodes_of(before), nodes_of(after));
    let node_ids = old
        .keys()
        .chain(new.keys())
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    for id in &node_ids {
        let changed = match (old.get(id), new.get(id)) {
            (Some(b), Some(a)) => !Arc::ptr_eq(b, a),
            _ => true,
        };
        if changed && !dirty.nodes.contains(id) {
            return Err(format!("node {id} changed but is not in touches"));
        }
    }
    if let Some(id) = dirty.nodes.iter().find(|id| !node_ids.contains(id)) {
        return Err(format!("touches lists node {id}, which does not exist"));
    }
    Ok(())
}
