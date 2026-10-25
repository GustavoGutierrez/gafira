//! The error type of command application.

use std::fmt;

use gafira_doc::{NodeId, PageId, ValidationError};

/// Why a command could not be applied or reverted.
///
/// A failed `apply` leaves the document as it was (ARQ-13).
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum CommandError {
    /// The target page does not exist.
    MissingPage(PageId),
    /// The target node does not exist in its page.
    MissingNode(NodeId),
    /// The node is not a `Group`, so it cannot hold children.
    NotAContainer(NodeId),
    /// An insertion or move index is above the length of the target list.
    IndexOutOfRange { index: usize, len: usize },
    /// Moving the node would put it under itself or one of its descendants.
    WouldCreateCycle(NodeId),
    /// A value is not finite, outside its range, or not valid for the target.
    InvalidValue(&'static str),
    /// The change would leave the document invalid.
    InvalidDocument(ValidationError),
    /// No further node or page id can be allocated.
    IdSpaceExhausted,
    /// `revert` was called on a command that was never applied.
    NotApplied,
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPage(page) => write!(f, "page {page} does not exist"),
            Self::MissingNode(node) => write!(f, "node {node} does not exist"),
            Self::NotAContainer(node) => write!(f, "node {node} is not a group"),
            Self::IndexOutOfRange { index, len } => {
                write!(f, "index {index} is out of range for length {len}")
            }
            Self::WouldCreateCycle(node) => write!(f, "moving node {node} would create a cycle"),
            Self::InvalidValue(what) => write!(f, "invalid value for {what}"),
            Self::InvalidDocument(error) => {
                write!(f, "the change leaves the document invalid: {error}")
            }
            Self::IdSpaceExhausted => f.write_str("the document id space is exhausted"),
            Self::NotApplied => f.write_str("the command was reverted without being applied"),
        }
    }
}

impl std::error::Error for CommandError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_contract<E: std::error::Error + fmt::Display + Clone + PartialEq>() {}

    #[test]
    fn an_error_is_cloneable_comparable_and_an_error() {
        assert_contract::<CommandError>();
    }
}
