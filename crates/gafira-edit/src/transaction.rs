//! `Transaction`: many commands, one history entry, all or nothing (ADR-03).

use gafira_doc::DocMut;
use gafira_doc::msg::CommandLabel;

use crate::{Applied, Command, CommandError, DirtySet};

/// A Composite command that applies its children in order.
///
/// If a child fails, the children applied before it are reverted in reverse
/// order and the failing child's error is returned, so the document ends as
/// it started.
#[derive(Debug)]
pub struct Transaction {
    label: CommandLabel,
    commands: Vec<Box<dyn Command>>,
}

impl Transaction {
    /// A transaction that applies `commands` in the given order.
    pub fn new(label: CommandLabel, commands: Vec<Box<dyn Command>>) -> Self {
        Self { label, commands }
    }
}

impl Command for Transaction {
    fn label(&self) -> CommandLabel {
        self.label
    }

    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        let mut result = Applied::Unchanged;
        for i in 0..self.commands.len() {
            match self.commands[i].apply(doc) {
                Ok(Applied::Changed) => result = Applied::Changed,
                Ok(Applied::Unchanged) => {}
                Err(error) => {
                    for done in self.commands[..i].iter_mut().rev() {
                        // The children applied cleanly, so their revert is
                        // expected to succeed; the failing child's error is
                        // the one to report.
                        let _ = done.revert(doc);
                    }
                    return Err(error);
                }
            }
        }
        Ok(result)
    }

    fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError> {
        for command in self.commands.iter_mut().rev() {
            command.revert(doc)?;
        }
        Ok(())
    }

    fn touches(&self) -> DirtySet {
        self.commands
            .iter()
            .fold(DirtySet::default(), |all, c| all.union(&c.touches()))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use gafira_doc::msg::MessageKey;
    use gafira_doc::{Document, Dpi, EditToken, NodeId, PageId};

    use super::*;

    type Log = Arc<Mutex<Vec<String>>>;

    /// What a test step does to the document.
    #[derive(Debug)]
    enum Effect {
        SetDpi(u32),
        Fail,
        Nothing,
    }

    /// A test-only command that logs its calls and optionally sets the DPI.
    #[derive(Debug)]
    struct Step {
        name: &'static str,
        log: Log,
        effect: Effect,
        previous: Option<Dpi>,
        dirty: DirtySet,
    }

    impl Step {
        fn new(name: &'static str, log: &Log, effect: Effect) -> Box<Self> {
            Box::new(Self {
                name,
                log: Arc::clone(log),
                effect,
                previous: None,
                dirty: DirtySet::default(),
            })
        }

        fn touching(name: &'static str, log: &Log, pages: &[u64], nodes: &[u64]) -> Box<Self> {
            let mut step = Self::new(name, log, Effect::Nothing);
            step.dirty = DirtySet {
                pages: pages.iter().copied().map(PageId::new).collect(),
                nodes: nodes.iter().copied().map(NodeId::new).collect(),
            };
            step
        }
    }

    impl Command for Step {
        fn label(&self) -> CommandLabel {
            MessageKey("test.step")
        }

        fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
            self.log
                .lock()
                .unwrap()
                .push(format!("apply {}", self.name));
            match self.effect {
                Effect::Fail => Err(CommandError::InvalidValue(self.name)),
                Effect::Nothing => Ok(Applied::Unchanged),
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
            }
        }

        fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError> {
            self.log
                .lock()
                .unwrap()
                .push(format!("revert {}", self.name));
            if let Some(old) = self.previous {
                doc.set_dpi(old);
            }
            Ok(())
        }

        fn touches(&self) -> DirtySet {
            self.dirty.clone()
        }
    }

    fn label() -> CommandLabel {
        MessageKey("test.transaction")
    }

    fn calls(log: &Log) -> Vec<String> {
        log.lock().unwrap().clone()
    }

    #[test]
    fn a_failing_child_rolls_back_the_earlier_ones() {
        let log = Log::default();
        let token = EditToken::__new_for_gafira_edit_only();
        let mut doc = Document::default();
        let before = doc.clone();
        let mut tx = Transaction::new(
            label(),
            vec![
                Step::new("a", &log, Effect::SetDpi(300)),
                Step::new("b", &log, Effect::SetDpi(600)),
                Step::new("c", &log, Effect::Fail),
            ],
        );
        let err = tx.apply(&mut doc.edit(&token)).unwrap_err();
        assert_eq!(err, CommandError::InvalidValue("c"));
        assert_eq!(doc, before);
        assert_eq!(
            calls(&log),
            ["apply a", "apply b", "apply c", "revert b", "revert a"]
        );
    }

    #[test]
    fn revert_undoes_the_children_in_reverse_order() {
        let log = Log::default();
        let token = EditToken::__new_for_gafira_edit_only();
        let mut doc = Document::default();
        let before = doc.clone();
        let mut tx = Transaction::new(
            label(),
            vec![
                Step::new("1", &log, Effect::SetDpi(300)),
                Step::new("2", &log, Effect::SetDpi(600)),
            ],
        );
        assert_eq!(tx.apply(&mut doc.edit(&token)), Ok(Applied::Changed));
        assert_eq!(doc.dpi(), Dpi::new(600));
        tx.revert(&mut doc.edit(&token)).unwrap();
        assert_eq!(calls(&log), ["apply 1", "apply 2", "revert 2", "revert 1"]);
        assert_eq!(doc, before);
    }

    #[test]
    fn touches_is_the_union_of_the_children() {
        let log = Log::default();
        let tx = Transaction::new(
            label(),
            vec![
                Step::touching("a", &log, &[], &[3]),
                Step::touching("b", &log, &[1], &[5]),
            ],
        );
        let touched = tx.touches();
        let pages: Vec<u64> = touched.pages.iter().map(|p| p.get()).collect();
        let nodes: Vec<u64> = touched.nodes.iter().map(|n| n.get()).collect();
        assert_eq!((pages, nodes), (vec![1], vec![3, 5]));
    }

    #[test]
    fn it_reports_changed_only_when_a_child_changed() {
        let token = EditToken::__new_for_gafira_edit_only();
        let rows = [
            (vec![], Applied::Unchanged),
            (vec![Effect::Nothing, Effect::Nothing], Applied::Unchanged),
            (vec![Effect::SetDpi(96)], Applied::Unchanged),
            (vec![Effect::Nothing, Effect::SetDpi(300)], Applied::Changed),
            (vec![Effect::SetDpi(300), Effect::Nothing], Applied::Changed),
        ];
        for (effects, expected) in rows {
            let log = Log::default();
            let commands = effects
                .into_iter()
                .map(|e| Step::new("s", &log, e) as Box<dyn Command>)
                .collect();
            let mut tx = Transaction::new(label(), commands);
            let mut doc = Document::default();
            assert_eq!(tx.apply(&mut doc.edit(&token)), Ok(expected));
        }
    }

    #[test]
    fn it_has_its_own_label_and_never_merges() {
        let tx = Transaction::new(label(), Vec::new());
        assert_eq!(tx.label(), label());
        assert_eq!(tx.merge_key(), None);
    }
}
