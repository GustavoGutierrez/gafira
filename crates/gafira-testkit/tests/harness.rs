//! The harness itself: it accepts sound commands and names the broken law.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use gafira_doc::msg::{CommandLabel, MessageKey};
use gafira_doc::{DocMut, Document, Dpi, NodeId, PageId};
use gafira_edit::{Applied, Command, CommandError, DirtySet};
use gafira_testkit::{prop_command, try_prop_command};
use proptest::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Flaw {
    Sound,
    /// Rejects its input with `Err` before changing anything.
    SoundRejects,
    /// Changes nothing and reports `Unchanged`.
    SoundUnchanged,
    /// Reports `Unchanged`, but its revert changes the page.
    UnchangedBadRevert,
    /// Sets a node opacity and lists the node.
    SoundNode,
    ForgetsRevert,
    DriftsOnReapply,
    InvalidResult,
    SilentNode,
    GhostTouch,
    FailsAfterMutating,
    /// Copies the page on write, changes nothing, then fails.
    FailsAfterCopying,
}

/// Toggles the `locked` flag of the first page and, for node flaws, changes
/// the opacity of its first root.
#[derive(Clone, Debug)]
struct Probe {
    flaw: Flaw,
    page: Option<PageId>,
    node: Option<NodeId>,
    old_locked: Option<bool>,
    old_name: Option<String>,
    old_dpi: Option<Dpi>,
    old_opacity: Option<f32>,
    applications: u32,
}

impl Probe {
    fn new(flaw: Flaw, doc: &Document) -> Self {
        let first = doc.pages().first();
        Self {
            flaw,
            page: first.map(|page| page.id),
            node: first.and_then(|page| page.roots.first().copied()),
            old_locked: None,
            old_name: None,
            old_dpi: None,
            old_opacity: None,
            applications: 0,
        }
    }

    fn changes_node(&self) -> bool {
        matches!(self.flaw, Flaw::SilentNode | Flaw::SoundNode)
    }
}

impl Command for Probe {
    fn label(&self) -> CommandLabel {
        MessageKey("test.probe")
    }

    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        self.applications += 1;
        let Some(page_id) = self.page else {
            return Ok(Applied::Unchanged);
        };
        match self.flaw {
            Flaw::SoundRejects => return Err(CommandError::InvalidValue("probe")),
            Flaw::SoundUnchanged | Flaw::UnchangedBadRevert => return Ok(Applied::Unchanged),
            _ => {}
        }
        if self.changes_node() {
            if let Some(id) = self.node {
                let node = doc.node_mut(page_id, id).expect("generated target exists");
                self.old_opacity = Some(node.base.opacity);
                node.base.opacity = if node.base.opacity == 0.5 { 0.25 } else { 0.5 };
            }
            return Ok(Applied::Changed);
        }
        self.old_dpi = Some(doc.as_doc().dpi());
        if self.flaw == Flaw::FailsAfterCopying {
            doc.page_mut(page_id).expect("generated target exists");
            return Err(CommandError::InvalidValue("probe"));
        }
        if self.flaw == Flaw::InvalidResult {
            doc.set_dpi(Dpi::new(0));
        }
        let page = doc.page_mut(page_id).expect("generated target exists");
        self.old_locked = Some(page.locked);
        self.old_name = Some(page.name.clone());
        page.locked = !page.locked;
        if self.flaw == Flaw::DriftsOnReapply {
            page.name = format!("applied {}", self.applications);
        }
        if self.flaw == Flaw::FailsAfterMutating {
            return Err(CommandError::InvalidValue("probe"));
        }
        Ok(Applied::Changed)
    }

    fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError> {
        let Some(page_id) = self.page else {
            return Ok(());
        };
        if self.flaw == Flaw::UnchangedBadRevert {
            let page = doc.page_mut(page_id).expect("target exists");
            page.locked = !page.locked;
            return Ok(());
        }
        if self.flaw == Flaw::ForgetsRevert {
            return Ok(());
        }
        if let (Some(id), Some(opacity)) = (self.node, self.old_opacity) {
            doc.node_mut(page_id, id)
                .expect("target exists")
                .base
                .opacity = opacity;
        }
        if let (Some(locked), Some(name), Some(dpi)) =
            (self.old_locked, self.old_name.take(), self.old_dpi)
        {
            let page = doc.page_mut(page_id).expect("target exists");
            (page.locked, page.name) = (locked, name);
            doc.set_dpi(dpi);
        }
        Ok(())
    }

    fn touches(&self) -> DirtySet {
        let mut dirty = DirtySet::default();
        match (self.flaw, self.page, self.node) {
            (Flaw::SoundNode, _, Some(id)) => {
                dirty.nodes.insert(id);
            }
            (Flaw::SilentNode | Flaw::SoundNode, _, _) => {}
            (flaw, Some(page), _) => {
                dirty.pages.insert(page);
                if flaw == Flaw::GhostTouch {
                    dirty.nodes.insert(NodeId::new(u64::MAX));
                }
            }
            _ => {}
        }
        dirty
    }
}

/// Names the first page after a value; merging keeps the later value
/// (sound) or adds the values (broken).
#[derive(Clone, Debug)]
struct Dial {
    value: u32,
    page: Option<PageId>,
    adds_on_merge: bool,
    /// Fails on a page that an earlier dial already named.
    rejects_dialed_page: bool,
    old: Option<String>,
}

impl Command for Dial {
    fn label(&self) -> CommandLabel {
        MessageKey("test.dial")
    }

    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        let Some(id) = self.page else {
            return Ok(Applied::Unchanged);
        };
        if self.rejects_dialed_page
            && doc
                .as_doc()
                .page(id)
                .is_some_and(|page| page.name.starts_with("dial "))
        {
            return Err(CommandError::InvalidValue("dialed"));
        }
        let page = doc.page_mut(id).expect("generated target exists");
        self.old = Some(std::mem::replace(
            &mut page.name,
            format!("dial {}", self.value),
        ));
        Ok(Applied::Changed)
    }

    fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError> {
        if let (Some(id), Some(old)) = (self.page, self.old.take()) {
            doc.page_mut(id).expect("target exists").name = old;
        }
        Ok(())
    }

    fn merge(&mut self, next: Box<dyn Command>) -> Result<(), Box<dyn Command>> {
        if !(&*next as &dyn std::any::Any).is::<Dial>() {
            return Err(next);
        }
        let any: Box<dyn std::any::Any + Send + Sync> = next;
        let Ok(next) = any.downcast::<Dial>() else {
            unreachable!("probed above")
        };
        self.value = if self.adds_on_merge {
            self.value + next.value
        } else {
            next.value
        };
        Ok(())
    }

    fn touches(&self) -> DirtySet {
        let mut dirty = DirtySet::default();
        dirty.pages.extend(self.page);
        dirty
    }
}

fn probe(flaw: Flaw) -> impl Fn(&Document) -> Just<Probe> {
    move |doc| Just(Probe::new(flaw, doc))
}

fn dial(
    adds_on_merge: bool,
    rejects_dialed_page: bool,
) -> impl Fn(&Document) -> BoxedStrategy<Dial> {
    move |doc| {
        let page = doc.pages().first().map(|page| page.id);
        (1..=100_u32)
            .prop_map(move |value| Dial {
                value,
                page,
                adds_on_merge,
                rejects_dialed_page,
                old: None,
            })
            .boxed()
    }
}

#[test]
fn sound_commands_pass_every_law() {
    prop_command(probe(Flaw::Sound));
    prop_command(probe(Flaw::SoundNode));
    prop_command(dial(false, false));
    prop_command(probe(Flaw::SoundRejects));
    prop_command(probe(Flaw::SoundUnchanged));
}

#[test]
fn each_broken_law_is_named_with_a_counterexample() {
    let rows = [
        (Flaw::ForgetsRevert, 1),
        (Flaw::DriftsOnReapply, 2),
        (Flaw::InvalidResult, 3),
        (Flaw::SilentNode, 4),
        (Flaw::GhostTouch, 4),
        (Flaw::FailsAfterMutating, 5),
        (Flaw::FailsAfterCopying, 5),
        (Flaw::UnchangedBadRevert, 1),
    ];
    for (flaw, law) in rows {
        let violation = try_prop_command(probe(flaw)).expect_err("the flaw is caught");
        assert_eq!(violation.law, law, "{flaw:?}: {violation}");
        assert!(violation.counterexample.contains("Probe"), "{violation}");
    }
    let violation = try_prop_command(dial(true, false)).expect_err("the broken merge is caught");
    assert_eq!(violation.law, 6, "{violation}");
}

#[test]
#[should_panic(expected = "law 1 broken")]
fn prop_command_panics_on_a_broken_law() {
    prop_command(probe(Flaw::ForgetsRevert));
}

#[test]
fn merge_is_not_compared_when_the_sequence_cannot_apply() {
    // The second dial fails on the page the first one named, so there is no
    // sequence to compare the (broken) merge with.
    assert_eq!(try_prop_command(dial(true, true)), Ok(()));
}

#[test]
fn it_runs_a_thousand_cases_by_default() {
    if std::env::var_os("PROPTEST_CASES").is_some() {
        return; // the override is in effect
    }
    let calls = Arc::new(AtomicU32::new(0));
    let counter = Arc::clone(&calls);
    prop_command(move |doc| {
        counter.fetch_add(1, Ordering::Relaxed);
        Just(Probe::new(Flaw::Sound, doc))
    });
    // The strategy is built twice per case: the command and the next one.
    assert_eq!(calls.load(Ordering::Relaxed), 2_000);
}
