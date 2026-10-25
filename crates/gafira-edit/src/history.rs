//! `History`: the owner of the session document and its undo stack (ADR-03).

use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

use gafira_doc::msg::CommandLabel;
use gafira_doc::{DocMut, Document, EditToken, ValidationError};

use crate::{Applied, Command, CommandError, GestureId, budget};

/// The documents on both sides of an entry's command.
#[derive(Debug)]
struct Snapshots {
    before: Arc<Document>,
    after: Arc<Document>,
}

/// One undo step: the command, its snapshots while it still holds them, and
/// the estimate of what they hold alone (Q7). A compacted entry has no
/// snapshots and counts nothing.
#[derive(Debug)]
struct Entry {
    command: Box<dyn Command>,
    snaps: Option<Snapshots>,
    bytes: usize,
    /// When the entry's newest command ran (the caller's clock).
    at: Duration,
}

/// Commands with equal keys merge when they are less than this apart.
const MERGE_WINDOW: Duration = Duration::from_millis(500);

/// The default memory budget: 512 MiB (ADR-03).
pub const DEFAULT_BUDGET: usize = 512 * 1024 * 1024;

/// The session document and the commands that produced it.
///
/// The only place where commands run against the document. Every change
/// happens on a copy that replaces the current document only on success, so
/// a failed command leaves it as it was.
#[derive(Debug)]
pub struct History {
    doc: Arc<Document>,
    undo: Vec<Entry>,
    redo: Vec<Entry>,
    /// The most the entries' snapshots may estimate to before the oldest are
    /// compacted.
    budget: usize,
    /// The gesture between `begin_gesture` and `end_gesture`.
    open_gesture: Option<GestureId>,
    next_gesture: u64,
    /// Whether the newest entry may absorb the next command; an undo or
    /// redo ends the run.
    mergeable: bool,
}

impl History {
    /// A history that starts from `doc`, which must be valid.
    pub fn new(doc: Document) -> Result<Self, ValidationError> {
        Self::with_budget(doc, DEFAULT_BUDGET)
    }

    /// A history that keeps about `budget` estimated bytes of snapshots.
    pub fn with_budget(doc: Document, budget: usize) -> Result<Self, ValidationError> {
        doc.validate()?;
        Ok(Self {
            doc: Arc::new(doc),
            undo: Vec::new(),
            redo: Vec::new(),
            budget,
            open_gesture: None,
            next_gesture: 0,
            mergeable: false,
        })
    }

    /// The current document.
    pub fn document(&self) -> &Arc<Document> {
        &self.doc
    }

    /// Applies `cmd` to a copy of the document. On `Changed` the copy
    /// becomes the current document and `cmd` an undo step; on `Unchanged`
    /// nothing is recorded; on `Err` the document is as it was.
    ///
    /// `at` is the caller's monotonic time. A changed command with the same
    /// merge key as the newest entry is absorbed into it when both belong to
    /// the open gesture, or when no gesture is involved and `at` is less
    /// than 500 ms after that entry's last command.
    ///
    /// When the snapshots' estimate then exceeds the budget, the oldest
    /// entries drop their snapshots and keep their commands.
    pub fn execute(
        &mut self,
        mut cmd: Box<dyn Command>,
        at: Duration,
    ) -> Result<Applied, CommandError> {
        let (after, applied) = self.apply_to_copy(&mut *cmd)?;
        if applied == Applied::Changed {
            self.redo.clear();
            let before = std::mem::replace(&mut self.doc, Arc::clone(&after));
            if let Err(cmd) = self.merge_into_newest(cmd, &after, at) {
                let bytes = budget::unshared_bytes(&before, &after);
                self.undo.push(Entry {
                    command: cmd,
                    snaps: Some(Snapshots { before, after }),
                    bytes,
                    at,
                });
                self.mergeable = true;
            }
            self.compact();
        }
        Ok(applied)
    }

    /// Drops the snapshots of the oldest entries until the estimate fits the
    /// budget; the newest entry always keeps its own.
    fn compact(&mut self) {
        let mut total = self.estimated_bytes();
        let older = self.undo.len().saturating_sub(1);
        for entry in self.undo.iter_mut().take(older) {
            if total <= self.budget {
                break;
            }
            if entry.snaps.take().is_some() {
                total -= entry.bytes;
                entry.bytes = 0;
            }
        }
    }

    /// Hands `cmd` to the newest entry when the merge rules allow it; gives
    /// it back otherwise.
    fn merge_into_newest(
        &mut self,
        cmd: Box<dyn Command>,
        after: &Arc<Document>,
        at: Duration,
    ) -> Result<(), Box<dyn Command>> {
        let open = self.open_gesture;
        let mergeable = self.mergeable;
        let Some(entry) = self.undo.last_mut().filter(|_| mergeable) else {
            return Err(cmd);
        };
        let (Some(old), Some(new)) = (entry.command.merge_key(), cmd.merge_key()) else {
            return Err(cmd);
        };
        let allowed = old == new
            && match old.gesture {
                Some(gesture) => open == Some(gesture),
                None => at.saturating_sub(entry.at) < MERGE_WINDOW,
            };
        if !allowed {
            return Err(cmd);
        }
        // The newest entry is never compacted, so this holds.
        let Some(snaps) = entry.snaps.as_mut() else {
            return Err(cmd);
        };
        entry.command.merge(cmd)?;
        snaps.after = Arc::clone(after);
        entry.bytes = budget::unshared_bytes(&snaps.before, after);
        entry.at = at;
        Ok(())
    }

    /// The document `cmd` would produce, without installing it or touching
    /// the undo stack.
    pub fn preview(&self, mut cmd: Box<dyn Command>) -> Result<Arc<Document>, CommandError> {
        self.apply_to_copy(&mut *cmd).map(|(doc, _)| doc)
    }

    /// Opens a gesture and returns its id.
    pub fn begin_gesture(&mut self) -> GestureId {
        let id = GestureId(self.next_gesture);
        self.next_gesture += 1;
        self.open_gesture = Some(id);
        id
    }

    /// Closes the open gesture.
    pub fn end_gesture(&mut self) {
        self.open_gesture = None;
    }

    /// The estimated bytes the entries' snapshots hold alone (Q7).
    pub fn estimated_bytes(&self) -> usize {
        self.undo.iter().chain(&self.redo).map(|e| e.bytes).sum()
    }

    /// How many entries can be undone.
    pub fn len(&self) -> usize {
        self.undo.len()
    }

    /// Whether no entry can be undone.
    pub fn is_empty(&self) -> bool {
        self.undo.is_empty()
    }

    /// Whether there is an entry to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Whether there is an undone entry to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Restores the document from before the newest entry and returns its label.
    ///
    /// An entry that still holds its snapshots installs the stored one and
    /// calls no command method. A compacted entry reverts its command on a
    /// copy of the current document, and on `Err` nothing changes. `None`
    /// when there is nothing to undo.
    pub fn undo(&mut self) -> Result<Option<CommandLabel>, CommandError> {
        let Some(mut entry) = self.undo.pop() else {
            return Ok(None);
        };
        let restored = match &entry.snaps {
            Some(snaps) => Ok(Arc::clone(&snaps.before)),
            None => on_copy(&self.doc, |doc| entry.command.revert(doc)).map(|(copy, ())| {
                debug_valid(&copy, &entry.command);
                copy
            }),
        };
        match restored {
            Ok(doc) => self.doc = doc,
            Err(error) => {
                self.undo.push(entry);
                return Err(error);
            }
        }
        self.mergeable = false;
        self.debug_check();
        let label = entry.command.label();
        self.redo.push(entry);
        Ok(Some(label))
    }

    /// Reinstalls the document the last undone entry produced.
    ///
    /// An entry that still holds its snapshots installs the stored one and
    /// calls no command method. A compacted entry applies its command again
    /// to a copy of the current document; `Unchanged` counts as success
    /// because the document then already holds the result. On `Err` nothing
    /// changes. `None` when there is nothing to redo.
    pub fn redo(&mut self) -> Result<Option<CommandLabel>, CommandError> {
        let Some(mut entry) = self.redo.pop() else {
            return Ok(None);
        };
        let restored = match &entry.snaps {
            Some(snaps) => Ok(Arc::clone(&snaps.after)),
            None => on_copy(&self.doc, |doc| entry.command.apply(doc)).map(|(copy, _)| {
                debug_valid(&copy, &entry.command);
                copy
            }),
        };
        match restored {
            Ok(doc) => self.doc = doc,
            Err(error) => {
                self.redo.push(entry);
                return Err(error);
            }
        }
        self.mergeable = false;
        self.debug_check();
        let label = entry.command.label();
        self.undo.push(entry);
        Ok(Some(label))
    }

    /// Runs `cmd` on a copy that shares every page and node it does not touch.
    fn apply_to_copy(
        &self,
        cmd: &mut dyn Command,
    ) -> Result<(Arc<Document>, Applied), CommandError> {
        let (copy, applied) = on_copy(&self.doc, |doc| cmd.apply(doc))?;
        debug_valid(&copy, cmd);
        Ok((copy, applied))
    }

    /// ARQ-13: the installed document is valid (debug builds only).
    fn debug_check(&self) {
        debug_assert!(
            self.doc.validate().is_ok(),
            "an installed snapshot is invalid: {:?}",
            self.doc.validate()
        );
    }
}

/// Runs `run` on a copy of `doc` that shares every page and node it does not
/// touch; the copy is dropped by the caller on `Err`.
fn on_copy<T>(
    doc: &Arc<Document>,
    run: impl FnOnce(&mut DocMut<'_>) -> Result<T, CommandError>,
) -> Result<(Arc<Document>, T), CommandError> {
    let token = EditToken::__new_for_gafira_edit_only();
    let mut copy = Arc::clone(doc);
    let out = run(&mut Arc::make_mut(&mut copy).edit(&token))?;
    Ok((copy, out))
}

/// ARQ-13: a command must leave a valid document (debug builds only).
fn debug_valid(doc: &Document, cmd: &dyn Debug) {
    debug_assert!(
        doc.validate().is_ok(),
        "{cmd:?} left the document invalid: {:?}",
        doc.validate()
    );
}

#[cfg(test)]
#[path = "history_budget_tests.rs"]
mod budget_tests;

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use gafira_doc::msg::MessageKey;
    use gafira_doc::{DocMut, Dpi, Node, Page, PageSize};
    use gafira_doc::{ElementBase, GroupNode, NodeKind, Placement};
    use gafira_geom::{Length, Unit};

    use super::*;
    use crate::DirtySet;

    /// What a test command does when applied.
    #[derive(Clone, Copy, Debug)]
    enum Effect {
        SetDpi(u32),
        /// Sets the DPI and reports `Changed` the first time only, like a
        /// merged drag that ended where it started.
        ChangedOnce(u32),
        Fail,
        /// Leaves a node nobody can reach and reports `Changed`.
        LeaveOrphan,
    }

    /// How often a command's `apply` and `revert` ran.
    #[derive(Debug, Default)]
    struct Calls {
        applies: AtomicUsize,
        reverts: AtomicUsize,
    }

    impl Calls {
        fn counts(&self) -> (usize, usize) {
            (
                self.applies.load(Ordering::SeqCst),
                self.reverts.load(Ordering::SeqCst),
            )
        }
    }

    #[derive(Debug)]
    struct Probe {
        effect: Effect,
        previous: Option<Dpi>,
        calls: Arc<Calls>,
    }

    impl Probe {
        fn boxed(effect: Effect) -> Box<dyn Command> {
            Self::counted(effect, &Arc::default())
        }

        fn counted(effect: Effect, calls: &Arc<Calls>) -> Box<dyn Command> {
            Box::new(Self {
                effect,
                previous: None,
                calls: Arc::clone(calls),
            })
        }
    }

    impl Command for Probe {
        fn label(&self) -> CommandLabel {
            MessageKey("test.probe")
        }

        fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
            self.calls.applies.fetch_add(1, Ordering::SeqCst);
            match self.effect {
                Effect::ChangedOnce(value) => {
                    let first = self.previous.is_none();
                    self.previous = Some(doc.as_doc().dpi());
                    doc.set_dpi(Dpi::new(value));
                    Ok(if first {
                        Applied::Changed
                    } else {
                        Applied::Unchanged
                    })
                }
                Effect::Fail => {
                    // Touch the copy first, so a leak would show.
                    doc.set_dpi(Dpi::new(1));
                    Err(CommandError::InvalidValue("probe"))
                }
                Effect::SetDpi(value) => {
                    let old = doc.as_doc().dpi();
                    self.previous = Some(old);
                    doc.set_dpi(Dpi::new(value));
                    Ok(if old.get() == value {
                        Applied::Unchanged
                    } else {
                        Applied::Changed
                    })
                }
                Effect::LeaveOrphan => {
                    let page = doc.as_doc().pages()[0].id;
                    let id = doc.alloc_node_id().expect("ids are available");
                    let node = Node::new(
                        ElementBase::new(id, Placement::new(0.0, 0.0, 1.0, 1.0)),
                        NodeKind::Group(GroupNode {}),
                    );
                    doc.page_mut(page)
                        .expect("the page exists")
                        .nodes
                        .insert(id, Arc::new(node));
                    Ok(Applied::Changed)
                }
            }
        }

        fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError> {
            self.calls.reverts.fetch_add(1, Ordering::SeqCst);
            if let Some(old) = self.previous {
                doc.set_dpi(old);
            }
            Ok(())
        }

        fn touches(&self) -> DirtySet {
            DirtySet::default()
        }
    }

    fn history() -> History {
        History::new(Document::default()).expect("the default document is valid")
    }

    fn at(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }

    fn set_dpi(history: &mut History, value: u32) -> Result<Applied, CommandError> {
        history.execute(Probe::boxed(Effect::SetDpi(value)), at(0))
    }

    #[test]
    fn it_refuses_an_invalid_document() {
        let token = gafira_doc::EditToken::__new_for_gafira_edit_only();
        let mut doc = Document::default();
        doc.edit(&token).set_dpi(Dpi::new(0));
        assert!(History::new(doc).is_err());
    }

    #[test]
    fn readers_keep_their_snapshot_when_a_command_runs() {
        let mut history = history();
        let before = Arc::clone(history.document());
        assert_eq!(set_dpi(&mut history, 300), Ok(Applied::Changed));
        assert_eq!(before.dpi(), Dpi::new(96));
        assert_eq!(history.document().dpi(), Dpi::new(300));
        assert!(!Arc::ptr_eq(&before, history.document()));
    }

    #[test]
    fn a_failed_command_leaves_the_session_untouched_and_usable() {
        let mut history = history();
        set_dpi(&mut history, 300).unwrap();
        let before = Arc::clone(history.document());
        let result = history.execute(Probe::boxed(Effect::Fail), at(0));
        assert_eq!(result, Err(CommandError::InvalidValue("probe")));
        assert!(Arc::ptr_eq(&before, history.document()));
        assert_eq!(history.len(), 1);
        assert_eq!(set_dpi(&mut history, 600), Ok(Applied::Changed));
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn an_unchanged_command_is_not_recorded() {
        let mut history = history();
        set_dpi(&mut history, 300).unwrap();
        let before = Arc::clone(history.document());
        assert_eq!(set_dpi(&mut history, 300), Ok(Applied::Unchanged));
        assert_eq!(history.len(), 1);
        assert!(history.can_undo());
        assert!(Arc::ptr_eq(&before, history.document()));
    }

    #[test]
    fn preview_returns_the_result_without_installing_it() {
        let mut history = history();
        set_dpi(&mut history, 300).unwrap();
        let before = Arc::clone(history.document());
        let shown = history.preview(Probe::boxed(Effect::SetDpi(600))).unwrap();
        assert_eq!(shown.dpi(), Dpi::new(600));
        assert!(Arc::ptr_eq(&before, history.document()));
        assert_eq!(history.len(), 1);
        assert!(!history.can_redo());
    }

    #[test]
    fn a_failed_preview_returns_the_error() {
        let history = history();
        let result = history.preview(Probe::boxed(Effect::Fail));
        assert_eq!(result.unwrap_err(), CommandError::InvalidValue("probe"));
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "left the document invalid")]
    fn a_command_that_breaks_the_document_trips_the_debug_assert() {
        let token = gafira_doc::EditToken::__new_for_gafira_edit_only();
        let mut doc = Document::default();
        {
            let mut edit = doc.edit(&token);
            let id = edit.alloc_page_id().unwrap();
            let size = PageSize {
                width: Length::new(10.0, Unit::Px),
                height: Length::new(10.0, Unit::Px),
            };
            edit.pages_mut()
                .push(Arc::new(Page::new(id, String::new(), size)));
        }
        let mut history = History::new(doc).unwrap();
        let _ = history.execute(Probe::boxed(Effect::LeaveOrphan), at(0));
    }

    fn label() -> Option<CommandLabel> {
        Some(MessageKey("test.probe"))
    }

    #[test]
    fn undo_restores_the_exact_snapshot() {
        let mut history = history();
        let s0 = Arc::clone(history.document());
        set_dpi(&mut history, 300).unwrap();
        assert_eq!(history.undo(), Ok(label()));
        assert!(Arc::ptr_eq(&s0, history.document()));
        assert_eq!(
            (history.len(), history.can_undo(), history.can_redo()),
            (0, false, true)
        );
    }

    #[test]
    fn redo_reinstalls_the_exact_state_the_command_produced() {
        let mut history = history();
        set_dpi(&mut history, 300).unwrap();
        let s1 = Arc::clone(history.document());
        history.undo().unwrap();
        assert_eq!(history.redo(), Ok(label()));
        assert!(Arc::ptr_eq(&s1, history.document()));
        assert_eq!(
            (history.len(), history.can_undo(), history.can_redo()),
            (1, true, false)
        );
    }

    #[test]
    fn entries_undo_newest_first_and_redo_oldest_first() {
        let mut history = history();
        for dpi in [100, 200, 300] {
            set_dpi(&mut history, dpi).unwrap();
        }
        let mut seen = Vec::new();
        for _ in 0..3 {
            history.undo().unwrap();
            seen.push(history.document().dpi().get());
        }
        for _ in 0..3 {
            history.redo().unwrap();
            seen.push(history.document().dpi().get());
        }
        assert_eq!(seen, [200, 100, 96, 100, 200, 300]);
    }

    #[test]
    fn undo_and_redo_call_no_command_method() {
        let calls = Arc::new(Calls::default());
        let mut history = history();
        history
            .execute(Probe::counted(Effect::SetDpi(300), &calls), at(0))
            .unwrap();
        for _ in 0..1_000 {
            history.undo().unwrap();
            assert_eq!(history.document().dpi(), Dpi::new(96));
            history.redo().unwrap();
            assert_eq!(history.document().dpi(), Dpi::new(300));
        }
        assert_eq!(calls.counts(), (1, 0));
        assert_eq!(history.document().dpi(), Dpi::new(300));
    }

    #[test]
    fn with_nothing_to_undo_or_redo_both_return_none() {
        let mut history = history();
        let before = Arc::clone(history.document());
        assert_eq!(history.undo(), Ok(None));
        assert_eq!(history.redo(), Ok(None));
        assert!(Arc::ptr_eq(&before, history.document()));
    }

    #[test]
    fn a_new_change_clears_redo() {
        let mut history = history();
        set_dpi(&mut history, 100).unwrap();
        set_dpi(&mut history, 200).unwrap();
        history.undo().unwrap();
        set_dpi(&mut history, 300).unwrap();
        assert!(!history.can_redo());
        assert_eq!(history.len(), 2);
        assert_eq!(history.redo(), Ok(None));
    }

    #[test]
    fn unchanged_and_failed_commands_keep_redo() {
        let mut history = history();
        set_dpi(&mut history, 100).unwrap();
        history.undo().unwrap();
        assert_eq!(set_dpi(&mut history, 96), Ok(Applied::Unchanged));
        assert!(history.can_redo());
        assert!(history.execute(Probe::boxed(Effect::Fail), at(0)).is_err());
        assert!(history.can_redo());
    }

    #[test]
    fn redo_does_not_depend_on_what_a_reapply_would_report() {
        let mut history = history();
        history
            .execute(Probe::boxed(Effect::ChangedOnce(300)), at(0))
            .unwrap();
        let s1 = Arc::clone(history.document());
        history.undo().unwrap();
        assert_eq!(history.redo(), Ok(label()));
        assert!(Arc::ptr_eq(&s1, history.document()));
        assert_eq!(history.len(), 1);
    }

    /// Sets the DPI and merges with any other `Nudge` of the same lane.
    #[derive(Debug)]
    struct Nudge {
        to: u32,
        lane: u64,
        gesture: Option<GestureId>,
        previous: Option<Dpi>,
    }

    impl Nudge {
        fn boxed(to: u32, lane: u64, gesture: Option<GestureId>) -> Box<dyn Command> {
            Box::new(Self {
                to,
                lane,
                gesture,
                previous: None,
            })
        }
    }

    impl Command for Nudge {
        fn label(&self) -> CommandLabel {
            MessageKey("test.nudge")
        }

        fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
            let old = doc.as_doc().dpi();
            self.previous = Some(old);
            doc.set_dpi(Dpi::new(self.to));
            Ok(if old.get() == self.to {
                Applied::Unchanged
            } else {
                Applied::Changed
            })
        }

        fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError> {
            if let Some(old) = self.previous {
                doc.set_dpi(old);
            }
            Ok(())
        }

        fn merge_key(&self) -> Option<crate::MergeKey> {
            Some(crate::MergeKey {
                kind: "test.nudge",
                page: None,
                nodes: vec![gafira_doc::NodeId::new(self.lane)],
                gesture: self.gesture,
            })
        }

        fn merge(&mut self, next: Box<dyn Command>) -> Result<(), Box<dyn Command>> {
            if !(&*next as &dyn std::any::Any).is::<Nudge>() {
                return Err(next);
            }
            let any: Box<dyn std::any::Any + Send + Sync> = next;
            match any.downcast::<Nudge>() {
                Ok(other) if other.merge_key() == self.merge_key() => {
                    self.to = other.to;
                    Ok(())
                }
                Ok(other) => Err(other),
                Err(_) => unreachable!("probed above"),
            }
        }

        fn touches(&self) -> DirtySet {
            DirtySet::default()
        }
    }

    fn nudge(history: &mut History, to: u32, ms: u64) {
        history
            .execute(Nudge::boxed(to, 1, None), at(ms))
            .expect("a nudge applies");
    }

    #[test]
    fn a_gesture_of_many_commands_is_one_entry_that_undoes_to_the_start() {
        let mut history = history();
        set_dpi(&mut history, 50).unwrap();
        let start = Arc::clone(history.document());
        let gesture = history.begin_gesture();
        for step in 0..300_u32 {
            let command = Nudge::boxed(100 + step, 1, Some(gesture));
            // Far apart in time: only the gesture can merge them.
            history
                .execute(command, at(u64::from(step) * 10_000))
                .unwrap();
        }
        history.end_gesture();
        assert_eq!(history.len(), 2);
        assert_eq!(history.document().dpi(), Dpi::new(399));
        assert_eq!(history.undo(), Ok(Some(MessageKey("test.nudge"))));
        assert!(Arc::ptr_eq(&start, history.document()));
        history.redo().unwrap();
        assert_eq!(history.document().dpi(), Dpi::new(399));
    }

    #[test]
    fn each_gesture_gets_a_fresh_id() {
        let mut history = history();
        let first = history.begin_gesture();
        history.end_gesture();
        assert_ne!(first, history.begin_gesture());
    }

    #[test]
    fn equal_keys_less_than_500_ms_apart_merge_and_500_ms_do_not() {
        let mut history = history();
        nudge(&mut history, 100, 1_000);
        nudge(&mut history, 200, 1_499);
        assert_eq!(history.len(), 1);
        nudge(&mut history, 300, 1_998);
        assert_eq!(history.len(), 1, "the window runs from the last command");
        nudge(&mut history, 400, 2_498);
        assert_eq!(history.len(), 2, "exactly 500 ms is outside the window");
        history.undo().unwrap();
        assert_eq!(history.document().dpi(), Dpi::new(300));
        history.undo().unwrap();
        assert_eq!(history.document().dpi(), Dpi::new(96));
    }

    #[test]
    fn a_merged_entry_keeps_its_before_and_takes_the_new_after() {
        let mut history = history();
        nudge(&mut history, 100, 0);
        nudge(&mut history, 200, 10);
        let last = Arc::clone(history.document());
        history.undo().unwrap();
        assert_eq!(history.document().dpi(), Dpi::new(96));
        history.redo().unwrap();
        assert!(Arc::ptr_eq(&last, history.document()));
    }

    #[test]
    fn different_keys_never_merge() {
        let mut history = history();
        let gesture = history.begin_gesture();
        history
            .execute(Nudge::boxed(100, 1, Some(gesture)), at(0))
            .unwrap();
        history
            .execute(Nudge::boxed(200, 2, Some(gesture)), at(1))
            .unwrap();
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn commands_without_a_key_never_merge() {
        let mut history = history();
        history.begin_gesture();
        set_dpi(&mut history, 100).unwrap();
        set_dpi(&mut history, 200).unwrap();
        history
            .execute(Probe::boxed(Effect::SetDpi(300)), at(1))
            .unwrap();
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn a_key_with_a_gesture_never_merges_once_the_gesture_closed() {
        let mut history = history();
        let gesture = history.begin_gesture();
        history
            .execute(Nudge::boxed(100, 1, Some(gesture)), at(0))
            .unwrap();
        history.end_gesture();
        history
            .execute(Nudge::boxed(200, 1, Some(gesture)), at(10))
            .unwrap();
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn a_gesture_from_a_previous_begin_does_not_merge_into_a_new_one() {
        let mut history = history();
        let first = history.begin_gesture();
        history
            .execute(Nudge::boxed(100, 1, Some(first)), at(0))
            .unwrap();
        history.end_gesture();
        let second = history.begin_gesture();
        history
            .execute(Nudge::boxed(200, 1, Some(second)), at(10_000))
            .unwrap();
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn no_merge_into_an_undone_entry_or_after_an_undo() {
        let mut history = history();
        nudge(&mut history, 100, 0);
        history.undo().unwrap();
        nudge(&mut history, 200, 100);
        assert_eq!(history.len(), 1);
        assert!(!history.can_redo());

        // An undo then a redo also breaks the run.
        nudge(&mut history, 300, 150);
        assert_eq!(history.len(), 1);
        history.undo().unwrap();
        history.redo().unwrap();
        nudge(&mut history, 400, 200);
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn an_undo_between_commands_stops_the_merge_into_older_entries() {
        let mut history = history();
        let gesture = history.begin_gesture();
        // Same gesture and 10 ms apart: only the undo can keep lane 1 apart.
        history
            .execute(Nudge::boxed(100, 1, Some(gesture)), at(0))
            .unwrap();
        history
            .execute(Nudge::boxed(200, 2, Some(gesture)), at(10))
            .unwrap();
        history.undo().unwrap();
        history
            .execute(Nudge::boxed(300, 1, Some(gesture)), at(20))
            .unwrap();
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn a_key_with_a_closed_gesture_never_merges_while_another_gesture_is_open() {
        let mut history = history();
        let first = history.begin_gesture();
        history
            .execute(Nudge::boxed(100, 1, Some(first)), at(0))
            .unwrap();
        let second = history.begin_gesture();
        assert_ne!(first, second);
        history
            .execute(Nudge::boxed(200, 1, Some(first)), at(10))
            .unwrap();
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn a_merged_command_that_ends_where_it_started_is_kept() {
        let mut history = history();
        nudge(&mut history, 100, 0);
        nudge(&mut history, 96, 10);
        assert_eq!(history.document().dpi(), Dpi::new(96));
        assert_eq!(history.len(), 1);
        assert!(history.can_undo());
    }

    #[test]
    fn a_command_that_does_not_merge_becomes_a_new_entry() {
        let mut history = history();
        nudge(&mut history, 100, 0);
        // Same key, but the other type refuses to absorb it.
        history
            .execute(Probe::boxed(Effect::SetDpi(200)), at(1))
            .unwrap();
        assert_eq!(history.len(), 2);
        history.undo().unwrap();
        assert_eq!(history.document().dpi(), Dpi::new(100));
    }

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn a_history_is_send_and_sync() {
        assert_send_sync::<History>();
    }
}
