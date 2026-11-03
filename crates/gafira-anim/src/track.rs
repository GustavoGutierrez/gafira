use std::fmt;

use gafira_time::Flicks;
use serde::{Deserialize, Serialize};

use crate::{Easing, Interpolate};

/// A value pinned at an instant, with the curve that leaves it (ARQ-16).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Keyframe<T> {
    pub at: Flicks,
    pub value: T,
    pub out: Easing,
}

/// The invariant a [`KeyTrack`] enforces (ARQ-18).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyTrackRule {
    /// The track has at least one key.
    AtLeastOneKey,
    /// `at` is strictly increasing.
    AtStrictlyIncreasing,
    /// Every `out` is a valid easing.
    ValidEasing,
    /// Every `value` is finite in all its components.
    FiniteValue,
}

impl KeyTrackRule {
    /// The rule as a short phrase.
    pub fn name(self) -> &'static str {
        match self {
            Self::AtLeastOneKey => "at least one key",
            Self::AtStrictlyIncreasing => "at strictly increasing",
            Self::ValidEasing => "valid easing",
            Self::FiniteValue => "finite value",
        }
    }
}

/// A broken [`KeyTrack`] invariant: the rule and the offending key index.
///
/// `index` is `None` only for [`KeyTrackRule::AtLeastOneKey`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyTrackError {
    pub rule: KeyTrackRule,
    pub index: Option<usize>,
}

impl fmt::Display for KeyTrackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "KeyTrack rule '{}' violated", self.rule.name())?;
        match self.index {
            Some(index) => write!(f, " at key index {index}"),
            None => write!(f, " (0 keys)"),
        }
    }
}

impl std::error::Error for KeyTrackError {}

/// Keys sorted by time. The invariant holds by construction and when read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    try_from = "KeyTrackWire<T>",
    bound(deserialize = "T: Interpolate + Deserialize<'de>")
)]
pub struct KeyTrack<T> {
    keys: Vec<Keyframe<T>>,
}

/// Unvalidated transport shape of a [`KeyTrack`].
#[derive(Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub(crate) struct KeyTrackWire<T> {
    pub(crate) keys: Vec<Keyframe<T>>,
}

impl<T: Interpolate> TryFrom<KeyTrackWire<T>> for KeyTrack<T> {
    type Error = KeyTrackError;

    fn try_from(wire: KeyTrackWire<T>) -> Result<Self, KeyTrackError> {
        Self::new(wire.keys)
    }
}

impl<T: Interpolate> KeyTrack<T> {
    /// Builds a track, checking the invariant. Keys are never sorted or fixed.
    ///
    /// # Errors
    ///
    /// Returns the first rule broken, with the index of the offending key.
    pub fn new(keys: Vec<Keyframe<T>>) -> Result<Self, KeyTrackError> {
        validate(&keys)?;
        Ok(Self { keys })
    }

    /// The keys, in strictly increasing `at` order.
    pub fn keys(&self) -> &[Keyframe<T>] {
        &self.keys
    }
}

fn validate<T: Interpolate>(keys: &[Keyframe<T>]) -> Result<(), KeyTrackError> {
    let fail = |rule, index| {
        Err(KeyTrackError {
            rule,
            index: Some(index),
        })
    };
    if keys.is_empty() {
        return Err(KeyTrackError {
            rule: KeyTrackRule::AtLeastOneKey,
            index: None,
        });
    }
    for (i, key) in keys.iter().enumerate() {
        if i > 0 && key.at <= keys[i - 1].at {
            return fail(KeyTrackRule::AtStrictlyIncreasing, i);
        }
        if !key.out.is_valid() {
            return fail(KeyTrackRule::ValidEasing, i);
        }
        if !key.value.is_finite() {
            return fail(KeyTrackRule::FiniteValue, i);
        }
    }
    Ok(())
}
