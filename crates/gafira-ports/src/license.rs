//! License verification (02 §4.14).

use std::fmt;
use std::time::Duration;

use gafira_doc::msg;
use gafira_doc::msg::{Message, ToMessage};

use crate::common::UnixTime;

/// Why the server rejected a license.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReasonCode {
    Revoked,
    UnknownKey,
    DeviceLimit,
    TokenTampered,
}

/// The state of the license (D-12: no trial; unlicensed use is allowed).
///
/// `NotRequired` is licensing switched off for the whole build (D-58): the
/// composition root injects [`NoLicensing`] instead of `gafira-license`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LicenseStatus {
    Active {
        expires_at: UnixTime,
        activation_id: String,
    },
    OfflineGrace {
        token_expires_at: UnixTime,
        retry_after: Duration,
    },
    Expired,
    Unlicensed,
    Invalid {
        reason: ReasonCode,
    },
    /// Licensing is off in this build: nothing is locked, nothing to activate.
    NotRequired,
}

impl LicenseStatus {
    #[must_use]
    pub const fn can_export(&self) -> bool {
        matches!(
            self,
            Self::Active { .. } | Self::OfflineGrace { .. } | Self::NotRequired
        )
    }

    /// Whether this build asks for a licence at all, so the UI shows the
    /// activation screen and the licence row of Settings (D-58).
    #[must_use]
    pub const fn is_required(&self) -> bool {
        !matches!(self, Self::NotRequired)
    }
}

/// A license key. `Debug` never shows it (ARQ-94) and there is no `Display`.
///
/// ```
/// use gafira_ports::license::LicenseKey;
/// assert_eq!(LicenseKey::new("k").expose(), "k");
/// ```
///
/// ```compile_fail
/// use gafira_ports::license::LicenseKey;
/// let _ = format!("{}", LicenseKey::new("k"));
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct LicenseKey(String);

impl fmt::Debug for LicenseKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LicenseKey(<redacted>)")
    }
}

impl LicenseKey {
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

/// Why a license call failed.
#[derive(Debug, PartialEq, Eq)]
pub enum LicenseError {
    Network,
    Rejected(ReasonCode),
    Storage,
    /// Licensing is off in this build (D-58); there is nothing to change.
    Disabled,
}

impl fmt::Display for LicenseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network => f.write_str("license server unreachable"),
            Self::Rejected(_) => f.write_str("license rejected"),
            Self::Storage => f.write_str("license storage failed"),
            Self::Disabled => f.write_str("licensing is disabled in this build"),
        }
    }
}

impl std::error::Error for LicenseError {}

impl ToMessage for LicenseError {
    fn to_message(&self) -> Message {
        match self {
            Self::Network => msg!("error.license.network"),
            Self::Rejected(_) => msg!("error.license.rejected"),
            Self::Storage => msg!("error.license.storage"),
            Self::Disabled => msg!("error.license.disabled"),
        }
    }
}

/// Checks and changes the license. `gafira-license` implements it.
pub trait LicenseVerifier: Send + Sync {
    /// Reads and verifies the stored token. No network.
    fn status(&self, now: UnixTime) -> LicenseStatus;

    /// Activates a key (network).
    ///
    /// # Errors
    /// [`LicenseError`] when the call fails or the server rejects the key.
    fn activate(
        &self,
        key: &LicenseKey,
        label: Option<&str>,
    ) -> Result<LicenseStatus, LicenseError>;

    /// Re-validates the token (network).
    ///
    /// # Errors
    /// [`LicenseError`] when the call fails.
    fn validate(&self, now: UnixTime) -> Result<LicenseStatus, LicenseError>;

    /// Releases this device (network).
    ///
    /// # Errors
    /// [`LicenseError`] when the call fails.
    fn deactivate(&self) -> Result<(), LicenseError>;
}

/// Licensing switched off (D-58): the verifier the composition root injects
/// when the build ships without a licence. No network, no storage.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoLicensing;

impl LicenseVerifier for NoLicensing {
    fn status(&self, _now: UnixTime) -> LicenseStatus {
        LicenseStatus::NotRequired
    }

    fn activate(
        &self,
        _key: &LicenseKey,
        _label: Option<&str>,
    ) -> Result<LicenseStatus, LicenseError> {
        Err(LicenseError::Disabled)
    }

    fn validate(&self, _now: UnixTime) -> Result<LicenseStatus, LicenseError> {
        Err(LicenseError::Disabled)
    }

    fn deactivate(&self) -> Result<(), LicenseError> {
        Err(LicenseError::Disabled)
    }
}
