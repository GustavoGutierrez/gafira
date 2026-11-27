//! `EditService`: commands, undo, redo and gestures over the session's
//! `History` (02 §5.1, §5.4 line 1359; ARQ-30, ARQ-33).
//!
//! Every change that installs a document bumps the engine's version by one
//! and publishes one `DocumentChanged` with the set of the entry `History`
//! installed (design P14, P15). Nothing else bumps or publishes. The first
//! such bump after the last snapshot marks the session pending for autosave
//! (design P13).

use std::sync::Arc;
use std::time::Duration;

use gafira_doc::msg::CommandLabel;
use gafira_edit::{Applied, Command, DirtySet, GestureId};
use gafira_ports::system::Clock;

use crate::engine::EngineError;
use crate::session::{Session, Shared};

/// Apply `command` to the open session at `clock`'s monotonic time, bump and
/// publish when it changed the document. The one path every service that
/// edits takes (`EditService`, `AssetService`), so each change is one undo
/// entry and one `DocumentChanged` (ARQ-30, design P15).
pub(crate) fn run(
    shared: &Shared,
    clock: &dyn Clock,
    command: Box<dyn Command>,
) -> Result<Applied, EngineError> {
    let at = clock.monotonic();
    let mut cell = shared.lock();
    let session = cell.as_mut().ok_or(EngineError::NoSession)?;
    let applied = session.history.execute(command, at)?;
    if applied == Applied::Changed {
        let dirty = session.history.newest_touches().unwrap_or_default();
        EditService::bumped(session, shared, dirty, at);
    }
    Ok(applied)
}

/// Runs the user's edits against the open document (02 §5.4 line 1359).
pub struct EditService {
    shared: Shared,
    clock: Arc<dyn Clock>,
}

impl EditService {
    pub(crate) fn new(shared: Shared, clock: Arc<dyn Clock>) -> Self {
        Self { shared, clock }
    }

    /// Bump and publish `dirty`, and mark the session pending since `at`
    /// unless an earlier change already did (P13).
    fn bumped(session: &mut Session, shared: &Shared, dirty: DirtySet, at: Duration) {
        session.version = shared.changed(dirty);
        session.autosave.pending_since.get_or_insert(at);
    }

    /// Run `f` over the open session, or refuse with
    /// [`EngineError::NoSession`] before anything runs or is published.
    fn with_session<T>(
        &self,
        f: impl FnOnce(&mut Session, &Shared) -> Result<T, EngineError>,
    ) -> Result<T, EngineError> {
        let mut cell = self.shared.lock();
        let session = cell.as_mut().ok_or(EngineError::NoSession)?;
        f(session, &self.shared)
    }

    /// Apply `command` at the clock's monotonic time (§5.1). `Changed`, merged
    /// into the newest entry or not, publishes the newest entry's set.
    pub fn execute(&mut self, command: Box<dyn Command>) -> Result<Applied, EngineError> {
        run(&self.shared, &*self.clock, command)
    }

    /// Whether there is an entry to undo and one to redo, for the Edit menu
    /// and the shortcuts; `(false, false)` with no session.
    #[must_use]
    pub fn can_undo_redo(&self) -> (bool, bool) {
        self.shared
            .lock()
            .as_ref()
            .map_or((false, false), |session| {
                (session.history.can_undo(), session.history.can_redo())
            })
    }

    /// Undo the newest entry; `Some` publishes the set of the entry undone.
    pub fn undo(&mut self) -> Result<Option<CommandLabel>, EngineError> {
        let at = self.clock.monotonic();
        self.with_session(|session, shared| {
            let label = session.history.undo()?;
            if label.is_some() {
                let dirty = session.history.undone_touches().unwrap_or_default();
                Self::bumped(session, shared, dirty, at);
            }
            Ok(label)
        })
    }

    /// Redo the newest undone entry; `Some` publishes the set of the entry
    /// redone.
    pub fn redo(&mut self) -> Result<Option<CommandLabel>, EngineError> {
        let at = self.clock.monotonic();
        self.with_session(|session, shared| {
            let label = session.history.redo()?;
            if label.is_some() {
                let dirty = session.history.newest_touches().unwrap_or_default();
                Self::bumped(session, shared, dirty, at);
            }
            Ok(label)
        })
    }

    /// Open a gesture: its commands merge into one entry (ARQ-33).
    pub fn begin_gesture(&mut self) -> Result<GestureId, EngineError> {
        self.with_session(|session, _| Ok(session.history.begin_gesture()))
    }

    /// Close the open gesture.
    pub fn end_gesture(&mut self) -> Result<(), EngineError> {
        self.with_session(|session, _| {
            session.history.end_gesture();
            Ok(())
        })
    }

    /// An immutable snapshot of the open document (02 §5.5: snapshots, never
    /// a shared mutable document), or `None` with no session. The UI draws
    /// from it and changes the document only through
    /// [`EditService::execute`] (spec 018).
    #[must_use]
    pub fn document(&self) -> Option<Arc<gafira_doc::Document>> {
        self.shared
            .lock()
            .as_ref()
            .map(|session| Arc::clone(session.history.document()))
    }
}

#[cfg(test)]
mod tests {
    use gafira_edit::{Applied, CommandError, DirtySet};

    use crate::engine::{EngineError, Gafira};
    use crate::event::EngineEvent;
    use crate::fakes::*;
    use crate::session::DocVersion;

    fn changed(version: u64, dirty: DirtySet) -> EngineEvent {
        EngineEvent::DocumentChanged {
            version: DocVersion(version),
            dirty,
        }
    }

    /// An engine after `new` (version 1) and one appended page (version 2),
    /// with both events drained.
    fn engine_with_one_page() -> (Gafira, gafira_doc::PageId) {
        let mut engine = open_engine();
        let page = append_page(&mut engine);
        assert_eq!(drain(&engine).len(), 1, "the append");
        (engine, page)
    }

    // ARQ-30
    #[test]
    fn ac1_a_changed_execute_bumps_once_with_the_newest_set() {
        let mut engine = open_engine();
        let page = append_page(&mut engine);
        assert_eq!(drain(&engine), vec![changed(2, pages(&[page]))]);
    }

    // ARQ-30
    #[test]
    fn ac1_a_merged_execute_bumps_once_with_the_merged_set() {
        let (mut engine, page) = engine_with_one_page();
        for name in ["a", "b"] {
            let applied = engine.edit().execute(rename(page, name, None));
            assert_eq!(applied.ok(), Some(Applied::Changed));
        }
        assert_eq!(
            drain(&engine),
            vec![changed(3, pages(&[page])), changed(4, pages(&[page]))]
        );
        assert!(engine.edit().undo().expect("an undo").is_some());
        assert_eq!(page_name(&mut engine), NAME, "both renames were one entry");
    }

    // ARQ-30
    #[test]
    fn ac1_ac4_undo_bumps_once_with_the_undone_set() {
        let (mut engine, first) = engine_with_one_page();
        let second = append_page(&mut engine);
        drain(&engine);

        assert!(engine.edit().undo().expect("an undo").is_some());
        assert_eq!(drain(&engine), vec![changed(4, pages(&[second]))]);
        assert_eq!(
            engine.edit().document().expect("a session").pages()[0].id,
            first
        );
    }

    // ARQ-30
    #[test]
    fn ac1_ac4_redo_bumps_once_with_the_redone_set() {
        let (mut engine, first) = engine_with_one_page();
        let second = append_page(&mut engine);
        assert!(engine.edit().undo().expect("an undo").is_some());
        assert!(engine.edit().undo().expect("an undo").is_some());
        drain(&engine);

        assert!(engine.edit().redo().expect("a redo").is_some());
        assert_eq!(drain(&engine), vec![changed(6, pages(&[first]))]);
        assert!(engine.edit().redo().expect("a redo").is_some());
        assert_eq!(drain(&engine), vec![changed(7, pages(&[second]))]);
    }

    /// After `op`, nothing was published and the next bump is `expected`:
    /// `op` neither bumped nor published.
    fn assert_no_bump(mut engine: Gafira, expected: u64, op: impl FnOnce(&mut Gafira)) {
        op(&mut engine);
        assert_eq!(drain(&engine), Vec::new(), "nothing published");
        let page = append_page(&mut engine);
        assert_eq!(drain(&engine), vec![changed(expected, pages(&[page]))]);
    }

    // ARQ-30
    #[test]
    fn ac2_an_unchanged_execute_neither_bumps_nor_publishes() {
        let (engine, page) = engine_with_one_page();
        assert_no_bump(engine, 3, |engine| {
            let applied = engine.edit().execute(rename(page, NAME, None));
            assert_eq!(applied.ok(), Some(Applied::Unchanged));
        });
    }

    // ARQ-30
    #[test]
    fn ac2_a_failed_execute_neither_bumps_nor_publishes() {
        let (engine, _) = engine_with_one_page();
        assert_no_bump(engine, 3, |engine| {
            let missing = gafira_doc::PageId::new(99);
            let error = engine.edit().execute(rename(missing, "x", None));
            assert!(matches!(
                error,
                Err(EngineError::Command(CommandError::MissingPage(id))) if id == missing
            ));
        });
    }

    // ARQ-30
    #[test]
    fn ac2_an_empty_undo_or_redo_neither_bumps_nor_publishes() {
        assert_no_bump(open_engine(), 2, |engine| {
            assert!(engine.edit().undo().expect("an undo").is_none());
        });
        let (engine, _) = engine_with_one_page();
        assert_no_bump(engine, 3, |engine| {
            assert!(engine.edit().redo().expect("a redo").is_none());
        });
    }

    // ARQ-33
    #[test]
    fn ac2_gestures_neither_bump_nor_publish() {
        let (engine, _) = engine_with_one_page();
        assert_no_bump(engine, 3, |engine| {
            engine.edit().begin_gesture().expect("a gesture");
            engine.edit().end_gesture().expect("the gesture ends");
        });
    }

    // ARQ-30
    #[test]
    fn ac3_the_version_strictly_increases_across_a_second_new() {
        let mut engine = engine_over(quiet_channel());
        engine.projects().new_document().expect("a new document");
        let first = append_page(&mut engine);
        append_page(&mut engine);
        assert!(engine.edit().undo().expect("an undo").is_some());
        assert!(engine.edit().redo().expect("a redo").is_some());
        engine
            .edit()
            .execute(rename(first, "a", None))
            .expect("a rename");
        engine.projects().new_document().expect("a second new");
        append_page(&mut engine);
        assert!(engine.edit().undo().expect("an undo").is_some());

        let versions: Vec<u64> = drain(&engine)
            .into_iter()
            .map(|event| match event {
                EngineEvent::DocumentChanged { version, .. } => version.get(),
                other => panic!("unexpected {other:?}"),
            })
            .collect();
        assert_eq!(versions, (1..=9).collect::<Vec<_>>());
    }

    // ARQ-33
    #[test]
    fn ac5_a_gesture_of_many_steps_is_one_entry() {
        let (mut engine, page) = engine_with_one_page();
        let gesture = engine.edit().begin_gesture().expect("a gesture");
        for name in ["a", "b", "c", "d"] {
            let applied = engine.edit().execute(rename(page, name, Some(gesture)));
            assert_eq!(applied.ok(), Some(Applied::Changed));
        }
        engine.edit().end_gesture().expect("the gesture ends");
        assert_eq!(page_name(&mut engine), "d");
        assert_eq!(drain(&engine).len(), 4, "one bump per merged step");

        assert!(engine.edit().undo().expect("an undo").is_some());
        assert_eq!(page_name(&mut engine), NAME, "back to before the gesture");
        assert_eq!(drain(&engine), vec![changed(7, pages(&[page]))]);
        assert_eq!(
            engine.edit().document().expect("a session").pages().len(),
            1,
            "the page insert is still there"
        );
    }

    // ARQ-30
    #[test]
    fn ac6_new_drops_the_history_before_it() {
        let (mut engine, _) = engine_with_one_page();
        engine.projects().new_document().expect("a second new");
        drain(&engine);

        assert!(engine.edit().undo().expect("an undo").is_none());
        assert!(engine.edit().redo().expect("a redo").is_none());
        assert_eq!(drain(&engine), Vec::new());
        assert!(
            engine
                .edit()
                .document()
                .expect("a session")
                .pages()
                .is_empty()
        );
    }

    // ARQ-30
    #[test]
    fn ac7_without_a_session_every_edit_call_is_refused_and_publishes_nothing() {
        let mut engine = engine_over(quiet_channel());
        let edit = engine.edit();
        let page = gafira_doc::PageId::new(1);
        let refused = [
            edit.execute(rename(page, "x", None)).map(|_| ()),
            edit.undo().map(|_| ()),
            edit.redo().map(|_| ()),
            edit.begin_gesture().map(|_| ()),
            edit.end_gesture(),
        ];
        for (call, result) in refused.into_iter().enumerate() {
            assert!(
                matches!(result, Err(EngineError::NoSession)),
                "call {call}: {result:?}"
            );
        }
        assert_eq!(drain(&engine), Vec::new());
    }
}
