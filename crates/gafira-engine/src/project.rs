//! `ProjectService`: the session's lifecycle (02 §5.4 line 1357).
//!
//! `E3b` brings `new_document` (the `new` of §5.4) and `is_modified`; `open`
//! and `save` are `E3c`.

use gafira_doc::{Document, Dpi};
use gafira_edit::{DirtySet, History};

use crate::engine::EngineError;
use crate::session::{Session, Shared};

/// Creates, opens and saves the session's document (02 §5.4 line 1357).
pub struct ProjectService {
    shared: Shared,
}

impl ProjectService {
    pub(crate) fn over(shared: Shared) -> Self {
        Self { shared }
    }

    /// Replace the session with an empty document and a fresh history.
    ///
    /// The old history is dropped: nothing made before `new` can be undone
    /// after it. The engine does not ask first; the UI does, with
    /// [`ProjectService::is_modified`] (design P15). Bumps the version and
    /// publishes a `DocumentChanged` that lists every page and node of the new
    /// document, so an observer drops what it holds for any other page.
    ///
    /// This is the `new` of 02 §5.4 line 1357; the name differs because
    /// clippy's `new_ret_no_self` reserves `new` for constructors.
    pub fn new_document(&self) -> Result<(), EngineError> {
        let document = Document::from_parts(1, Dpi::DEFAULT, Vec::new())?;
        let dirty = every_page_and_node(&document);
        let history = History::new(document)?;
        let mut cell = self.shared.lock();
        let clean_version = self.shared.changed(dirty);
        *cell = Some(Session {
            history,
            clean_version,
        });
        Ok(())
    }

    /// Whether the session holds changes no `new_document`, `open` or `save`
    /// has accounted for: its version differs from its clean version. Undoing back
    /// to the clean content still counts as modified (conservative, P15).
    /// `false` with no session: there is nothing to lose.
    #[must_use]
    pub fn is_modified(&self) -> bool {
        self.shared
            .lock()
            .as_ref()
            .is_some_and(|session| session.clean_version != self.shared.version())
    }
}

/// Every page and every node of `document`.
fn every_page_and_node(document: &Document) -> DirtySet {
    DirtySet {
        pages: document.pages().iter().map(|page| page.id).collect(),
        nodes: document
            .pages()
            .iter()
            .flat_map(|page| page.nodes.keys().copied())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use gafira_edit::DirtySet;

    use crate::event::EngineEvent;
    use crate::fakes::*;
    use crate::session::DocVersion;

    // ARQ-30
    #[test]
    fn ac1_ac4_new_bumps_once_and_lists_the_whole_empty_document() {
        let engine = engine_over(quiet_channel());
        engine.projects().new_document().expect("a new document");
        assert_eq!(
            drain(&engine),
            vec![EngineEvent::DocumentChanged {
                version: DocVersion(1),
                dirty: DirtySet::default(),
            }]
        );
    }

    // ARQ-30
    #[test]
    fn ac8_modified_after_an_edit_and_after_undoing_it() {
        let mut engine = engine_over(quiet_channel());
        assert!(!engine.projects().is_modified(), "no session");
        engine.projects().new_document().expect("a new document");
        assert!(!engine.projects().is_modified(), "after new");

        append_page(&mut engine);
        assert!(engine.projects().is_modified(), "after an edit");

        assert!(engine.edit().undo().expect("an undo").is_some());
        assert!(engine.projects().is_modified(), "after undoing back");

        engine.projects().new_document().expect("a second new");
        assert!(!engine.projects().is_modified(), "after a second new");
    }
}
