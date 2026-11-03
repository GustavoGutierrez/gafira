//! Optional text assistants (02 §4.15, FD-64). Local and off by default: the
//! MVP registers none (ARQ-26).

use std::fmt;

use gafira_doc::contract::{CancelToken, Cancelled, Describe, Progress};
use gafira_doc::msg;
use gafira_doc::msg::{Message, ToMessage};

use crate::registry::Registry;

/// What an assistant needs to run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AssistReq {
    /// RAM the model needs, in bytes.
    pub ram_bytes: u64,
    /// True when it needs a GPU.
    pub needs_gpu: bool,
    /// Size of the installed model, in bytes.
    pub model_bytes: u64,
}

/// The tone asked of a rewrite.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    /// Keep the tone of the text.
    #[default]
    Same,
    /// More formal.
    Formal,
    /// More casual.
    Casual,
}

/// The length asked of a rewrite.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Length {
    /// Keep the length.
    #[default]
    Same,
    /// Shorter.
    Shorter,
    /// Longer.
    Longer,
}

/// What to do with the text in [`TextAssistant::rewrite`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RewriteRequest {
    /// The tone.
    pub tone: Tone,
    /// The length.
    pub length: Length,
    /// Language tag of the result; `None` keeps the language of the text.
    pub language: Option<String>,
}

/// One proposed rewrite. Never applied on its own: the user picks one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggestion {
    /// The proposed text.
    pub text: String,
}

/// Why a [`TextAssistant`] call failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssistError {
    /// The user has not installed the model.
    ModelMissing,
    /// The caller cancelled.
    Cancelled(Cancelled),
}

impl fmt::Display for AssistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModelMissing => f.write_str("assistant model is not installed"),
            Self::Cancelled(c) => fmt::Display::fmt(c, f),
        }
    }
}

impl std::error::Error for AssistError {}

impl ToMessage for AssistError {
    fn to_message(&self) -> Message {
        match self {
            Self::ModelMissing => msg!("error.assist.model_missing"),
            Self::Cancelled(c) => c.to_message(),
        }
    }
}

/// Rewrites text locally, without network. Adapters implement it (Fase 3).
pub trait TextAssistant: Describe + Send + Sync {
    /// What this assistant needs to run.
    fn requirements(&self) -> AssistReq;

    /// Proposes rewrites of `text`.
    ///
    /// # Errors
    /// [`AssistError::ModelMissing`] when the model is not installed,
    /// [`AssistError::Cancelled`] when `cancel` fires.
    fn rewrite(
        &self,
        text: &str,
        req: &RewriteRequest,
        progress: &dyn Progress,
        cancel: &CancelToken,
    ) -> Result<Vec<Suggestion>, AssistError>;
}

/// The assistants of a build. `default()` is empty: the MVP registers none.
pub type TextAssistantRegistry = Registry<dyn TextAssistant>;
