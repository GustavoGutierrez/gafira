use std::sync::Arc;
use std::time::Duration;

use gafira_doc::msg::{CommandLabel, MessageKey};
use gafira_doc::{DocMut, Document, Page, PageSize};
use gafira_edit::{Applied, Command, CommandError, DirtySet, History};
use gafira_geom::{Length, Unit};

/// A command written outside `gafira-edit`: it mutates only through `DocMut`.
#[derive(Debug)]
struct AddPage;

impl Command for AddPage {
    fn label(&self) -> CommandLabel {
        MessageKey("fixture.add_page")
    }

    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
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

    fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError> {
        doc.pages_mut().pop();
        Ok(())
    }

    fn touches(&self) -> DirtySet {
        DirtySet::default()
    }
}

fn main() {
    let mut history = History::new(Document::default()).unwrap();
    assert!(history.document().pages().is_empty());

    history.execute(Box::new(AddPage), Duration::ZERO).unwrap();
    assert_eq!(history.document().pages().len(), 1);

    history.undo().unwrap();
    assert!(history.document().pages().is_empty());

    history.redo().unwrap();
    assert_eq!(history.document().pages().len(), 1);
}
