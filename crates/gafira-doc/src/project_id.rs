//! The project's identity (D-63, spec 012): a 128-bit UUID minted when the
//! project is created and kept in the `.gaf`, so the recovery slots and
//! the versions folder of 02 §7.2 find their project after a rename or a move.
//!
//! `gafira-doc` mints nothing: the bits come from a port of the engine, so the
//! crate stays free of randomness and of a UUID dependency. The wire form is
//! the hyphenated lowercase text of RFC 9562 (`8-4-4-4-12` hex digits); upper
//! case reads too and is written back lower case. The nil id says "no
//! identity yet": a document written before the field loads with it, and the
//! engine gives such a document a fresh id when it installs it.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The identity of a project (D-63).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectId(u128);

impl ProjectId {
    /// The nil id: the project has no identity yet.
    pub const NIL: Self = Self(0);

    /// Wraps raw bits without validation.
    #[must_use]
    pub const fn from_u128(bits: u128) -> Self {
        Self(bits)
    }

    /// The raw bits.
    #[must_use]
    pub const fn as_u128(self) -> u128 {
        self.0
    }

    /// Whether this is [`ProjectId::NIL`].
    #[must_use]
    pub const fn is_nil(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hex = format!("{:032x}", self.0);
        write!(
            f,
            "{}-{}-{}-{}-{}",
            &hex[0..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..32]
        )
    }
}

/// Text that is not a hyphenated UUID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectIdError;

impl fmt::Display for ProjectIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a project id is 32 hex digits grouped 8-4-4-4-12")
    }
}

impl std::error::Error for ProjectIdError {}

/// Where the four hyphens of the text form sit.
const HYPHENS: [usize; 4] = [8, 13, 18, 23];

impl FromStr for ProjectId {
    type Err = ProjectIdError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let bytes = text.as_bytes();
        if bytes.len() != 36 {
            return Err(ProjectIdError);
        }
        let mut bits: u128 = 0;
        for (at, byte) in bytes.iter().enumerate() {
            if HYPHENS.contains(&at) {
                if *byte != b'-' {
                    return Err(ProjectIdError);
                }
                continue;
            }
            let digit = char::from(*byte).to_digit(16).ok_or(ProjectIdError)?;
            bits = (bits << 4) | u128::from(digit);
        }
        Ok(Self(bits))
    }
}

impl Serialize for ProjectId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ProjectId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "0123abcd-4567-89ef-0a1b-2c3d4e5f6789";
    const BITS: u128 = 0x0123_abcd_4567_89ef_0a1b_2c3d_4e5f_6789;

    #[test]
    fn s12_01_text_form_is_hyphenated_lowercase_hex() {
        assert_eq!(ProjectId::from_u128(BITS).to_string(), TEXT);
        assert_eq!(
            ProjectId::NIL.to_string(),
            "00000000-0000-0000-0000-000000000000"
        );
        assert_eq!(
            ProjectId::from_u128(u128::MAX).to_string(),
            "ffffffff-ffff-ffff-ffff-ffffffffffff"
        );
    }

    #[test]
    fn s12_01_text_reads_back_and_upper_case_reads_too() {
        assert_eq!(TEXT.parse(), Ok(ProjectId::from_u128(BITS)));
        assert_eq!(TEXT.to_uppercase().parse(), Ok(ProjectId::from_u128(BITS)));
        let max: ProjectId = "ffffffff-ffff-ffff-ffff-ffffffffffff"
            .parse()
            .expect("max reads");
        assert_eq!(max.as_u128(), u128::MAX);
    }

    #[test]
    fn s12_01_malformed_text_is_refused() {
        for bad in [
            "",
            "0123abcd45678-9ef-0a1b-2c3d4e5f6789",
            "0123abcd-4567-89ef-0a1b-2c3d4e5f678",
            "0123abcd-4567-89ef-0a1b-2c3d4e5f67890",
            "0123abcg-4567-89ef-0a1b-2c3d4e5f6789",
            "0123abcd 4567-89ef-0a1b-2c3d4e5f6789",
            "0123abcd-4567-89ef-0a1b+2c3d4e5f6789",
            "+123abcd-4567-89ef-0a1b-2c3d4e5f6789",
            "0123abcd-4567-89ef-0a1b-2c3d4e5f678é",
        ] {
            assert_eq!(bad.parse::<ProjectId>(), Err(ProjectIdError), "{bad:?}");
        }
    }

    #[test]
    fn s12_01_wire_form_is_the_text_and_round_trips() {
        let id = ProjectId::from_u128(BITS);
        let json = serde_json::to_string(&id).expect("writes");
        assert_eq!(json, format!("\"{TEXT}\""));
        assert_eq!(serde_json::from_str::<ProjectId>(&json).expect("reads"), id);
        assert!(serde_json::from_str::<ProjectId>("\"nope\"").is_err());
        assert!(serde_json::from_str::<ProjectId>("7").is_err());
    }

    #[test]
    fn s12_01_nil_is_the_default_and_the_only_nil() {
        assert!(ProjectId::default().is_nil());
        assert_eq!(ProjectId::default(), ProjectId::NIL);
        assert!(!ProjectId::from_u128(1).is_nil());
        assert!(!ProjectId::from_u128(1 << 127).is_nil());
    }
}
