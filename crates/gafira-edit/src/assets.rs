//! `InsertAsset`: adds an entry to the document's asset table (02 §3.1;
//! spec 011). The reference is the content hash, so the same content
//! imported twice is one entry and the second insert changes nothing.

use gafira_doc::msg::{CommandLabel, MessageKey};
use gafira_doc::{AssetEntry, AssetRef, DocMut};

use crate::{Applied, Command, CommandError, DirtySet};

/// Adds `entry` under `reference` unless the table already has that content.
#[derive(Clone, Debug)]
pub struct InsertAsset {
    reference: AssetRef,
    entry: AssetEntry,
    inserted: bool,
}

impl InsertAsset {
    /// Inserts `entry` under `reference`, the hash of its content.
    pub fn new(reference: AssetRef, entry: AssetEntry) -> Self {
        Self {
            reference,
            entry,
            inserted: false,
        }
    }
}

impl Command for InsertAsset {
    fn label(&self) -> CommandLabel {
        MessageKey("edit.insert_asset")
    }

    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        if doc.as_doc().asset(&self.reference).is_some() {
            self.inserted = false;
            return Ok(Applied::Unchanged);
        }
        doc.assets_mut().insert(self.reference, self.entry.clone());
        self.inserted = true;
        Ok(Applied::Changed)
    }

    /// Removes the entry `apply` inserted; an `Unchanged` apply left nothing
    /// to remove.
    fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError> {
        if !self.inserted {
            return Err(CommandError::NotApplied);
        }
        doc.assets_mut().remove(&self.reference);
        self.inserted = false;
        Ok(())
    }

    /// The asset table is neither a page nor a node: nothing to repaint.
    fn touches(&self) -> DirtySet {
        DirtySet::default()
    }
}

#[cfg(test)]
mod tests {
    use gafira_doc::{AssetEntry, AssetRef, AssetStorage, ContentHash, Document};

    use super::*;
    use crate::History;

    fn entry(name: &str) -> AssetEntry {
        AssetEntry {
            name: name.to_owned(),
            storage: AssetStorage::Embedded,
        }
    }

    fn reference(byte: u8) -> AssetRef {
        AssetRef::new(ContentHash([byte; 32]))
    }

    fn history() -> History {
        History::new(Document::default()).expect("the default document is valid")
    }

    #[test]
    fn s11_01_a_new_hash_is_inserted_and_undo_removes_it() {
        let mut history = history();
        let applied = history
            .execute(
                Box::new(InsertAsset::new(reference(1), entry("a.png"))),
                Default::default(),
            )
            .expect("applies");
        assert_eq!(applied, Applied::Changed);
        assert_eq!(
            history.document().asset(&reference(1)),
            Some(&entry("a.png"))
        );

        history.undo().expect("undoes");
        assert!(history.document().assets().is_empty());

        history.redo().expect("redoes");
        assert_eq!(
            history.document().asset(&reference(1)),
            Some(&entry("a.png"))
        );
    }

    #[test]
    fn s11_01_a_known_hash_changes_nothing_and_keeps_the_first_entry() {
        let mut history = history();
        history
            .execute(
                Box::new(InsertAsset::new(reference(1), entry("a.png"))),
                Default::default(),
            )
            .expect("applies");
        let again = history
            .execute(
                Box::new(InsertAsset::new(reference(1), entry("copy.png"))),
                Default::default(),
            )
            .expect("applies");
        assert_eq!(again, Applied::Unchanged);
        assert_eq!(history.document().assets().len(), 1);
        assert_eq!(
            history.document().asset(&reference(1)),
            Some(&entry("a.png"))
        );
    }

    #[test]
    fn s11_01_revert_removes_what_apply_inserted() {
        let token = gafira_doc::EditToken::__new_for_gafira_edit_only();
        let mut document = Document::default();
        let mut insert = InsertAsset::new(reference(1), entry("a.png"));
        let mut doc = document.edit(&token);
        assert_eq!(insert.apply(&mut doc).expect("applies"), Applied::Changed);
        insert.revert(&mut doc).expect("reverts");
        assert!(document.assets().is_empty());
    }

    #[test]
    fn s11_01_revert_before_apply_is_refused() {
        let token = gafira_doc::EditToken::__new_for_gafira_edit_only();
        let mut document = Document::default();
        let mut doc = document.edit(&token);
        let mut insert = InsertAsset::new(reference(1), entry("a.png"));
        assert!(matches!(
            insert.revert(&mut doc),
            Err(CommandError::NotApplied)
        ));
        assert_eq!(insert.touches(), DirtySet::default());
    }
}
