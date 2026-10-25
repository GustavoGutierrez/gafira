//! `History`: the owner of the session document and its undo stack (ADR-03).

use std::sync::Arc;
use std::time::Duration;

use gafira_doc::msg::CommandLabel;
use gafira_doc::{Document, EditToken, ValidationError};

use crate::{Applied, Command, CommandError};

/// One undo step: the command and the document before and after it.
#[derive(Debug)]
struct Entry {
    command: Box<dyn Command>,
    before: Arc<Document>,
    after: Arc<Document>,
}

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
}

impl History {
    /// A history that starts from `doc`, which must be valid.
    pub fn new(doc: Document) -> Result<Self, ValidationError> {
        doc.validate()?;
        Ok(Self {
            doc: Arc::new(doc),
            undo: Vec::new(),
            redo: Vec::new(),
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
    /// `at` is the caller's monotonic time, used by gesture merging.
    pub fn execute(
        &mut self,
        mut cmd: Box<dyn Command>,
        _at: Duration,
    ) -> Result<Applied, CommandError> {
        let (after, applied) = self.apply_to_copy(&mut *cmd)?;
        if applied == Applied::Changed {
            self.redo.clear();
            let before = std::mem::replace(&mut self.doc, Arc::clone(&after));
            self.undo.push(Entry {
                command: cmd,
                before,
                after,
            });
        }
        Ok(applied)
    }

    /// The document `cmd` would produce, without installing it or touching
    /// the undo stack.
    pub fn preview(&self, mut cmd: Box<dyn Command>) -> Result<Arc<Document>, CommandError> {
        self.apply_to_copy(&mut *cmd).map(|(doc, _)| doc)
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
    /// Installs the stored snapshot and calls no command method; `None` when
    /// there is nothing to undo.
    pub fn undo(&mut self) -> Result<Option<CommandLabel>, CommandError> {
        let Some(entry) = self.undo.pop() else {
            return Ok(None);
        };
        self.doc = Arc::clone(&entry.before);
        self.debug_check();
        let label = entry.command.label();
        self.redo.push(entry);
        Ok(Some(label))
    }

    /// Reinstalls the document the last undone entry produced.
    ///
    /// Installs the stored snapshot and calls no command method; `None` when
    /// there is nothing to redo.
    pub fn redo(&mut self) -> Result<Option<CommandLabel>, CommandError> {
        let Some(entry) = self.redo.pop() else {
            return Ok(None);
        };
        self.doc = Arc::clone(&entry.after);
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
        let token = EditToken::__new_for_gafira_edit_only();
        let mut copy = Arc::clone(&self.doc);
        let applied = cmd.apply(&mut Arc::make_mut(&mut copy).edit(&token))?;
        debug_assert!(
            copy.validate().is_ok(),
            "{cmd:?} left the document invalid: {:?}",
            copy.validate()
        );
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

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn a_history_is_send_and_sync() {
        assert_send_sync::<History>();
    }
}
