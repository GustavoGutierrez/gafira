//! The `Command` contract (02 §4.3).

use std::any::Any;
use std::fmt::Debug;

use gafira_doc::msg::CommandLabel;
#[cfg(test)]
use gafira_doc::msg::MessageKey;
use gafira_doc::{DocMut, NodeId, PageId};

use crate::{CommandError, DirtySet};

/// Whether an applied command changed the document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Applied {
    /// The document changed and the command is an undo step.
    Changed,
    /// The document already held the result; nothing to undo.
    Unchanged,
}

/// Names one drag or similar user gesture, so its commands can merge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GestureId(pub u64);

/// Identifies commands that may merge into one history entry.
///
/// Two commands merge only when their keys are equal; `History` decides when
/// (same gesture, or inside the merge window).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeKey {
    /// The command kind and property, for example `"set_property.placement"`.
    pub kind: &'static str,
    /// The page the targets belong to, if the command works inside one page.
    pub page: Option<PageId>,
    /// The target nodes, in command order.
    pub nodes: Vec<NodeId>,
    /// The gesture that produced the command, if any.
    pub gesture: Option<GestureId>,
}

/// One undoable edit of a document.
///
/// `apply` and `revert` mutate through the capability view [`DocMut`]. A
/// failed `apply` MUST leave the document equal to its prior state.
pub trait Command: Any + Send + Sync + Debug {
    /// The key of the message shown as «Undo {label}».
    fn label(&self) -> CommandLabel;

    /// Applies the command; `Unchanged` when nothing would change.
    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError>;

    /// Restores the state before the last successful `apply`.
    fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError>;

    /// The key under which this command may merge with a later one.
    fn merge_key(&self) -> Option<MergeKey> {
        None
    }

    /// Absorbs `next`, or hands it back untouched when it cannot.
    ///
    /// An override recovers the concrete type of `next` by probing it by
    /// reference first (`(&*next as &dyn Any).is::<Own>()`); a failed
    /// by-value downcast cannot give the box back.
    fn merge(&mut self, next: Box<dyn Command>) -> Result<(), Box<dyn Command>> {
        Err(next)
    }

    /// The pages and nodes this command changes.
    fn touches(&self) -> DirtySet;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Does nothing and keeps the default `merge`.
    #[derive(Debug)]
    struct Noop;

    impl Command for Noop {
        fn label(&self) -> CommandLabel {
            MessageKey("test.noop")
        }
        fn apply(&mut self, _: &mut DocMut<'_>) -> Result<Applied, CommandError> {
            Ok(Applied::Unchanged)
        }
        fn revert(&mut self, _: &mut DocMut<'_>) -> Result<(), CommandError> {
            Ok(())
        }
        fn touches(&self) -> DirtySet {
            DirtySet::default()
        }
    }

    /// Adds up the values of the commands merged into it.
    #[derive(Debug)]
    struct Sum(u32);

    impl Command for Sum {
        fn label(&self) -> CommandLabel {
            MessageKey("test.sum")
        }
        fn apply(&mut self, _: &mut DocMut<'_>) -> Result<Applied, CommandError> {
            Ok(Applied::Unchanged)
        }
        fn revert(&mut self, _: &mut DocMut<'_>) -> Result<(), CommandError> {
            Ok(())
        }
        fn touches(&self) -> DirtySet {
            DirtySet::default()
        }
        fn merge(&mut self, next: Box<dyn Command>) -> Result<(), Box<dyn Command>> {
            if !(&*next as &dyn Any).is::<Sum>() {
                return Err(next);
            }
            let any: Box<dyn Any + Send + Sync> = next;
            match any.downcast::<Sum>() {
                Ok(other) => {
                    self.0 += other.0;
                    Ok(())
                }
                Err(_) => unreachable!("probed above"),
            }
        }
    }

    fn assert_send_sync<T: Send + Sync + ?Sized>() {}

    #[test]
    fn a_boxed_command_is_send_and_sync() {
        assert_send_sync::<dyn Command>();
        assert_send_sync::<Box<dyn Command>>();
    }

    #[test]
    fn default_merge_hands_next_back_unchanged() {
        let mut first = Noop;
        let second: Box<dyn Command> = Box::new(Sum(7));
        let before: *const dyn Command = &*second;
        let back = first.merge(second).expect_err("the default never merges");
        assert!(std::ptr::addr_eq(before, &*back));
    }

    #[test]
    fn merge_recovers_its_own_type_and_returns_any_other_intact() {
        let mut first = Sum(1);
        assert!(first.merge(Box::new(Sum(2))).is_ok());
        assert_eq!(first.0, 3);

        let other: Box<dyn Command> = Box::new(Noop);
        let before: *const dyn Command = &*other;
        let back = first.merge(other).expect_err("another type is refused");
        assert!(std::ptr::addr_eq(before, &*back));
        assert_eq!(first.0, 3);
    }
}
