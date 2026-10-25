//! `Reorder`: one node to a new index among its siblings (ARQ-81, ADR-03).

use gafira_doc::msg::{CommandLabel, MessageKey};
use gafira_doc::{DocMut, NodeId, PageId};

use crate::moves::{self, Record};
use crate::{Applied, Command, CommandError, DirtySet};

/// Moves one node to `index` inside its current sibling list.
#[derive(Clone, Debug)]
pub struct Reorder {
    page: PageId,
    node: NodeId,
    index: usize,
    record: Option<Record>,
}

impl Reorder {
    /// Moves `node` of `page` so it ends at `index` (`0..siblings`).
    pub fn new(page: PageId, node: NodeId, index: usize) -> Self {
        Self {
            page,
            node,
            index,
            record: None,
        }
    }
}

impl Command for Reorder {
    fn label(&self) -> CommandLabel {
        MessageKey("edit.reorder")
    }

    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        let page = doc
            .as_doc()
            .page(self.page)
            .ok_or(CommandError::MissingPage(self.page))?;
        let (parent, _) = moves::places(page)
            .get(&self.node)
            .copied()
            .ok_or(CommandError::MissingNode(self.node))?;
        let len = moves::list(page, parent).map_or(0, Vec::len);
        if self.index >= len {
            return Err(CommandError::IndexOutOfRange {
                index: self.index,
                len,
            });
        }
        let planned = moves::plan(page, &[self.node], parent, self.index)?;
        let applied = planned.is_some();
        let record = planned.unwrap_or_default();
        record.write(doc, self.page)?;
        self.record = Some(record);
        Ok(if applied {
            Applied::Changed
        } else {
            Applied::Unchanged
        })
    }

    fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError> {
        let record = self.record.as_ref().ok_or(CommandError::NotApplied)?;
        record.revert(doc, self.page)
    }

    fn touches(&self) -> DirtySet {
        match &self.record {
            Some(record) => record.touches(self.page),
            None => {
                // Unspecified before a successful apply (edit-commands: touches is defined after it); filled in place,
                // not built with a struct expression, so cargo-mutants has no field to delete in this fallback.
                let mut set = DirtySet::default();
                set.nodes.insert(self.node);
                set
            }
        }
    }
}
