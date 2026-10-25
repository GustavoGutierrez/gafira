//! Undo and redo install stored snapshots whatever the document size
//! (ARQ-32, structural proof; the timed criterion waits for ARQ-40).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use gafira_doc::msg::{CommandLabel, MessageKey};
use gafira_doc::{DocMut, Page, PageSize};
use gafira_edit::{Applied, Command, CommandError, DirtySet, History};
use gafira_geom::{Length, Unit};
use gafira_testkit::document;
use proptest::prelude::*;

/// Adds one page to the document; counts how often it was called.
#[derive(Debug)]
struct AddPage {
    calls: Arc<AtomicUsize>,
}

impl Command for AddPage {
    fn label(&self) -> CommandLabel {
        MessageKey("test.add_page")
    }

    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let id = doc
            .alloc_page_id()
            .map_err(|_| CommandError::IdSpaceExhausted)?;
        let size = PageSize {
            width: Length::new(10.0, Unit::Px),
            height: Length::new(10.0, Unit::Px),
        };
        doc.pages_mut()
            .push(Arc::new(Page::new(id, String::new(), size)));
        Ok(Applied::Changed)
    }

    fn revert(&mut self, _: &mut DocMut<'_>) -> Result<(), CommandError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn touches(&self) -> DirtySet {
        DirtySet::default()
    }
}

proptest! {
    #![proptest_config(ProptestConfig { failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn undo_and_redo_install_the_recorded_snapshots_whatever_the_size(
        doc in document(),
        steps in 1..=3_usize,
    ) {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut history = History::new(doc).unwrap();
        let mut snapshots = vec![Arc::clone(history.document())];
        for _ in 0..steps {
            let command = AddPage { calls: Arc::clone(&calls) };
            history.execute(Box::new(command), Duration::ZERO).unwrap();
            snapshots.push(Arc::clone(history.document()));
        }
        prop_assert_eq!(calls.load(Ordering::SeqCst), steps);

        for expected in snapshots.iter().rev().skip(1) {
            prop_assert!(history.undo().unwrap().is_some());
            prop_assert!(Arc::ptr_eq(expected, history.document()));
        }
        for expected in snapshots.iter().skip(1) {
            prop_assert!(history.redo().unwrap().is_some());
            prop_assert!(Arc::ptr_eq(expected, history.document()));
        }
        prop_assert_eq!(calls.load(Ordering::SeqCst), steps);
    }
}
