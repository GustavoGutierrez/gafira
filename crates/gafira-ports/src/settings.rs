//! The user's settings and where they persist (02 §5.3, §14.4; spec 009).
//!
//! 02 §5.3 names `SettingsRepository` without a signature; spec 009 fixes it:
//! the whole [`Settings`] value is loaded and saved at once.

use std::fmt;
use std::io;

use gafira_doc::msg;
use gafira_doc::msg::{Message, ToMessage};
use serde::{Deserialize, Serialize};

use crate::i18n::LanguagePreference;

/// What the user chose in Settings. Persisted as JSON, for example
/// `{"language": "system"}` (02 §14.4). A missing key reads as its default.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The interface language: `"system"` (default), `"es"` or `"en"`.
    pub language: LanguagePreference,
    /// Light or dark interface (D-82, spec 021): `"light"` (default),
    /// `"dark"` or `"system"` (spec 034).
    pub appearance: Appearance,
}

/// The interface mode (03 §3, D-82). Any text other than `"dark"` reads as
/// light, so a file from a newer build never fails to load.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    Dark,
    /// Follow the operating system (03 §7.8, spec 034).
    System,
    // Last, because `serde(other)` must be: unknown text reads as light.
    #[default]
    #[serde(other)]
    Light,
}

impl Settings {
    /// These settings with `appearance` in place of the current one.
    #[must_use]
    pub const fn with_appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// These settings with `language` in place of the current one.
    #[must_use]
    pub const fn with_language(mut self, language: LanguagePreference) -> Self {
        self.language = language;
        self
    }
}

/// Why a [`SettingsRepository`] call failed.
#[derive(Debug)]
pub enum SettingsError {
    /// The stored settings cannot be read. `detail` is library text.
    Corrupt { detail: String },
    /// The backing storage failed.
    Io(io::Error),
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Corrupt { detail } => write!(f, "corrupt settings: {detail}"),
            Self::Io(e) => write!(f, "settings I/O error: {e}"),
        }
    }
}

impl std::error::Error for SettingsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Corrupt { .. } => None,
            Self::Io(e) => Some(e),
        }
    }
}

impl ToMessage for SettingsError {
    fn to_message(&self) -> Message {
        match self {
            Self::Corrupt { .. } => msg!("error.settings.corrupt"),
            Self::Io(_) => msg!("error.settings.io"),
        }
    }
}

/// Loads and saves the user's settings. Adapters implement it (layer 4).
pub trait SettingsRepository: Send + Sync {
    /// Reads the stored settings. A store that does not exist yet is
    /// [`Settings::default`], not an error.
    ///
    /// # Errors
    /// [`SettingsError::Corrupt`] when the stored value cannot be read, for
    /// example a `language` that is not text; [`SettingsError::Io`] when the
    /// storage fails.
    fn load(&self) -> Result<Settings, SettingsError>;

    /// Writes `settings` and leaves them durable before it returns `Ok`.
    ///
    /// # Errors
    /// [`SettingsError::Io`] when the storage fails.
    fn save(&self, settings: &Settings) -> Result<(), SettingsError>;
}
