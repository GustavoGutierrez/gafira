//! `Reparent`: nodes under a new parent or to the roots (ARQ-81, ADR-03).

use gafira_doc::msg::{CommandLabel, MessageKey};
use gafira_doc::{DocMut, NodeId, PageId};

use crate::moves::{self, Record};
use crate::{Applied, Command, CommandError, DirtySet};

/// Moves nodes of one page under a new `Group` parent, or to the roots.
///
/// The moved nodes keep their document order. A node listed together with
/// one of its descendants moves once, as the ancestor.
#[derive(Clone, Debug)]
pub struct Reparent {
    page: PageId,
    nodes: Vec<NodeId>,
    parent: Option<NodeId>,
    index: usize,
    record: Option<Record>,
}

impl Reparent {
    /// Moves `nodes` of `page` under `parent` (`None` for the roots) at
    /// `index` of that list once the moved nodes are out of it.
    pub fn new(page: PageId, nodes: Vec<NodeId>, parent: Option<NodeId>, index: usize) -> Self {
        Self {
            page,
            nodes,
            parent,
            index,
            record: None,
        }
    }
}

impl Command for Reparent {
    fn label(&self) -> CommandLabel {
        MessageKey("edit.reparent")
    }

    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        let page = doc
            .as_doc()
            .page(self.page)
            .ok_or(CommandError::MissingPage(self.page))?;
        let planned = moves::plan(page, &self.nodes, self.parent, self.index)?;
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
            None => DirtySet {
                nodes: self.nodes.iter().copied().collect(),
                ..DirtySet::default()
            },
        }
    }
}
