//! `SetData`: replaces the connected data of bulk create (spec 104).

use std::any::Any;
use std::sync::Arc;

use gafira_doc::msg::{CommandLabel, MessageKey};
use gafira_doc::{DataSet, DocMut};

use crate::{Applied, Command, CommandError, DirtySet, MergeKey};

/// Puts `data` as the document's connected data (none removes it). Edits
/// of the table made close together merge into one undo step, so typing a
/// cell is one step; the rows are shared by `Arc`, so a step copies no row
/// that did not change.
#[derive(Clone, Debug)]
pub struct SetData {
    data: Option<Arc<DataSet>>,
    old: Option<Option<Arc<DataSet>>>,
}

impl SetData {
    #[must_use]
    pub fn new(data: Option<Arc<DataSet>>) -> Self {
        Self { data, old: None }
    }
}

impl Command for SetData {
    fn label(&self) -> CommandLabel {
        MessageKey("edit.set_data")
    }

    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        let old = doc.as_doc().data_arc().cloned();
        if old.as_deref() == self.data.as_deref() {
            // A revert after it puts back the same table.
            if self.old.is_none() {
                self.old = Some(old);
            }
            return Ok(Applied::Unchanged);
        }
        if self.data.as_deref().and_then(DataSet::problem).is_some() {
            return Err(CommandError::InvalidValue(
                "the connected data is not valid",
            ));
        }
        doc.set_data(self.data.clone());
        // Merged commands keep the first old value.
        if self.old.is_none() {
            self.old = Some(old);
        }
        Ok(Applied::Changed)
    }

    fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError> {
        let old = self.old.clone().ok_or(CommandError::NotApplied)?;
        doc.set_data(old);
        Ok(())
    }

    fn merge_key(&self) -> Option<MergeKey> {
        Some(MergeKey {
            kind: "set_data",
            page: None,
            nodes: Vec::new(),
            gesture: None,
        })
    }

    fn merge(&mut self, next: Box<dyn Command>) -> Result<(), Box<dyn Command>> {
        if !(&*next as &dyn Any).is::<SetData>() {
            return Err(next);
        }
        let any: Box<dyn Any + Send + Sync> = next;
        match any.downcast::<SetData>() {
            Ok(other) => {
                self.data = other.data;
                Ok(())
            }
            Err(_) => unreachable!("probed above"),
        }
    }

    fn touches(&self) -> DirtySet {
        DirtySet::default()
    }
}
