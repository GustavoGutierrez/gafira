//! `RestoreVersion`: puts a version's content back into the document
//! (02 §7.1 «Versiones», spec 031). The document keeps its identity; undo
//! gives back what it had.

use std::sync::Arc;

use gafira_doc::msg::{CommandLabel, MessageKey};
use gafira_doc::{AssetTable, DocMut, Document, Dpi, Page, Sequence};

use crate::{Applied, Command, CommandError, DirtySet};

/// The content a restore replaces: everything but the schema and the
/// project's identity.
#[derive(Clone, Debug, PartialEq)]
struct Content {
    next_id: u64,
    dpi: Dpi,
    pages: Vec<Arc<Page>>,
    sequences: Vec<Arc<Sequence>>,
    assets: AssetTable,
}

impl Content {
    fn of(document: &Document) -> Self {
        Self {
            next_id: document.next_id(),
            dpi: document.dpi(),
            pages: document.pages().to_vec(),
            sequences: document.sequences().to_vec(),
            assets: document.assets().clone(),
        }
    }

    /// The same pages, sequences, assets and dpi; the id counter does not
    /// count, it is not content.
    fn same_as(&self, other: &Content) -> bool {
        self.dpi == other.dpi
            && self.pages == other.pages
            && self.sequences == other.sequences
            && self.assets == other.assets
    }

    /// Every page and node it holds.
    fn touches(&self) -> DirtySet {
        DirtySet {
            pages: self.pages.iter().map(|page| page.id).collect(),
            nodes: self
                .pages
                .iter()
                .flat_map(|page| page.nodes.keys().copied())
                .collect(),
        }
    }

    /// Install the pages, sequences, assets and dpi into `doc`.
    fn install(&self, doc: &mut DocMut<'_>) {
        doc.set_dpi(self.dpi);
        doc.pages_mut().clone_from(&self.pages);
        doc.sequences_mut().clone_from(&self.sequences);
        doc.assets_mut().clone_from(&self.assets);
    }
}

/// Replaces the pages, the sequences, the asset table and the dpi with
/// those of `version`. The id counter only moves up, past every id of both
/// contents, so no id is handed out twice; the project id stays.
#[derive(Clone, Debug)]
pub struct RestoreVersion {
    version: Content,
    /// What the last `apply` did, for `revert` and `touches`.
    state: State,
}

/// What the last `apply` of a [`RestoreVersion`] did.
#[derive(Clone, Debug)]
enum State {
    NotApplied,
    /// The document already held the version's content.
    Unchanged,
    /// The content `apply` replaced.
    Replaced(Content),
}

impl RestoreVersion {
    /// Restores the content of `version`, a valid document of the same
    /// project.
    pub fn new(version: &Document) -> Self {
        Self {
            version: Content::of(version),
            state: State::NotApplied,
        }
    }
}

impl Command for RestoreVersion {
    fn label(&self) -> CommandLabel {
        MessageKey("edit.restore_version")
    }

    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        let before = Content::of(doc.as_doc());
        if before.same_as(&self.version) {
            self.state = State::Unchanged;
            return Ok(Applied::Unchanged);
        }
        self.version.install(doc);
        doc.raise_next_id(self.version.next_id);
        self.state = State::Replaced(before);
        Ok(Applied::Changed)
    }

    /// Puts back what `apply` replaced; after an `Unchanged` apply there is
    /// nothing to put back.
    fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError> {
        match std::mem::replace(&mut self.state, State::NotApplied) {
            State::NotApplied => Err(CommandError::NotApplied),
            State::Unchanged => Ok(()),
            State::Replaced(before) => {
                before.install(doc);
                doc.restore_next_id(before.next_id);
                Ok(())
            }
        }
    }

    /// Every page and node of the content before and after: all of them may
    /// have changed. Nothing after an `Unchanged` apply.
    fn touches(&self) -> DirtySet {
        match &self.state {
            State::Replaced(before) => before.touches().union(&self.version.touches()),
            State::NotApplied | State::Unchanged => DirtySet::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use gafira_doc::{
        AssetEntry, AssetRef, AssetStorage, ContentHash, ElementBase, GroupNode, Node, NodeId,
        NodeKind, PageId, PageSize, Placement, ProjectId,
    };
    use gafira_geom::{Length, Unit};

    use super::*;
    use crate::History;

    fn size(px: f64) -> PageSize {
        PageSize {
            width: Length::new(px, Unit::Px),
            height: Length::new(px, Unit::Px),
        }
    }

    /// A document of one page `page` holding one group `node`, counter
    /// `next_id`, with an asset when `asset` is set.
    fn document(page: u64, node: u64, next_id: u64, dpi: u32, asset: bool) -> Document {
        let mut sheet = Page::new(PageId::new(page), format!("p{page}"), size(100.0));
        let group = Node::new(
            ElementBase::new(NodeId::new(node), Placement::new(0.0, 0.0, 10.0, 10.0)),
            NodeKind::Group(GroupNode {}),
        );
        sheet.nodes.insert(NodeId::new(node), Arc::new(group));
        sheet.roots.push(NodeId::new(node));
        let mut document = Document::from_parts(next_id, Dpi::new(dpi), vec![Arc::new(sheet)])
            .expect("a valid document")
            .with_project_id(ProjectId::from_u128(7));
        if asset {
            let token = gafira_doc::EditToken::__new_for_gafira_edit_only();
            document.edit(&token).assets_mut().insert(
                AssetRef::new(ContentHash([1; 32])),
                AssetEntry {
                    name: "a.png".to_owned(),
                    storage: AssetStorage::Embedded,
                },
            );
        }
        document
    }

    /// The version: page 1 with node 2, counter 3, 300 dpi, one asset; under
    /// another project id, which must not travel.
    fn version() -> Document {
        document(1, 2, 3, 300, true).with_project_id(ProjectId::from_u128(9))
    }

    #[test]
    fn s31_08_the_version_content_replaces_the_document_and_undo_gives_it_back() {
        let current = document(5, 6, 7, 96, false);
        let mut history = History::new(current.clone()).expect("valid");
        let applied = history
            .execute(
                Box::new(RestoreVersion::new(&version())),
                Default::default(),
            )
            .expect("applies");
        assert_eq!(applied, Applied::Changed);
        let restored = history.document();
        assert_eq!(restored.pages(), version().pages());
        assert_eq!(restored.assets(), version().assets());
        assert_eq!(restored.dpi(), Dpi::new(300));
        assert_eq!(restored.project_id(), ProjectId::from_u128(7), "kept");
        assert_eq!(restored.next_id(), 7, "never lowered");
        assert!(restored.validate().is_ok());

        history.undo().expect("undoes");
        assert_eq!(**history.document(), current);
        history.redo().expect("redoes");
        assert_eq!(history.document().pages(), version().pages());
    }

    #[test]
    fn s31_08_the_counter_rises_past_the_version_ids() {
        // The current counter (3 after undoing everything) is below the
        // version's ids; restoring must not hand id 4 out again.
        let current = document(1, 2, 3, 96, false);
        let ahead = document(4, 5, 6, 96, false);
        let mut history = History::new(current.clone()).expect("valid");
        history
            .execute(Box::new(RestoreVersion::new(&ahead)), Default::default())
            .expect("applies");
        assert_eq!(history.document().next_id(), 6);
        history.undo().expect("undoes");
        assert_eq!(history.document().next_id(), 3, "undo lowers it back");
    }

    #[test]
    fn s31_08_the_same_content_changes_nothing() {
        let current = version().with_project_id(ProjectId::from_u128(7));
        let mut history = History::new(current).expect("valid");
        let applied = history
            .execute(
                Box::new(RestoreVersion::new(&version())),
                Default::default(),
            )
            .expect("applies");
        assert_eq!(applied, Applied::Unchanged);
        assert!(!history.can_undo());
    }

    #[test]
    fn s31_08_it_touches_every_page_and_node_of_both_contents() {
        let token = gafira_doc::EditToken::__new_for_gafira_edit_only();
        let mut current = document(5, 6, 7, 96, false);
        let mut restore = RestoreVersion::new(&version());
        restore.apply(&mut current.edit(&token)).expect("applies");
        let touched = restore.touches();
        let pages: Vec<u64> = touched.pages.iter().map(|p| p.get()).collect();
        let nodes: Vec<u64> = touched.nodes.iter().map(|n| n.get()).collect();
        assert_eq!(pages, [1, 5]);
        assert_eq!(nodes, [2, 6]);
    }

    #[test]
    fn s31_08_revert_puts_back_the_exact_document() {
        // History undo uses snapshots; a compacted entry relies on revert.
        let token = gafira_doc::EditToken::__new_for_gafira_edit_only();
        let ahead = document(4, 5, 6, 300, true);
        for original in [document(1, 2, 3, 96, false), document(8, 9, 10, 96, false)] {
            let mut current = original.clone();
            let mut restore = RestoreVersion::new(&ahead);
            let mut doc = current.edit(&token);
            assert_eq!(restore.apply(&mut doc).expect("applies"), Applied::Changed);
            restore.revert(&mut doc).expect("reverts");
            assert_eq!(current, original);
        }
    }

    #[test]
    fn s31_08_revert_before_apply_is_refused() {
        let token = gafira_doc::EditToken::__new_for_gafira_edit_only();
        let mut current = document(5, 6, 7, 96, false);
        let mut restore = RestoreVersion::new(&version());
        assert!(matches!(
            restore.revert(&mut current.edit(&token)),
            Err(CommandError::NotApplied)
        ));
    }
}
