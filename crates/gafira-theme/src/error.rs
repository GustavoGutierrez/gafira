//! Why a tokens file was rejected (02 §8.2). Paths are dotted token paths such
//! as `modes.dark.link`: stable technical identifiers, never translated (ADR 0012).

use std::fmt;

use gafira_doc::msg;
use gafira_doc::msg::{Message, ToMessage};

/// A tokens file that does not load or does not pass validation.
#[derive(Clone, Debug, PartialEq)]
pub enum ThemeError {
    /// The text is not JSON.
    Json { detail: String },
    /// `schema` is newer than this build reads.
    SchemaTooNew { found: u64, supported: u64 },
    /// `schema` is not a version this build knows.
    SchemaUnknown { found: u64 },
    /// A required token is absent.
    MissingToken { path: String },
    /// A token has the wrong JSON type or an out-of-range number.
    BadValue { path: String },
    /// A colour is neither `#RRGGBB` nor a `{reference}`.
    BadColor { path: String, value: String },
    /// A `{reference}` names a token that does not exist.
    UnresolvedReference { path: String, target: String },
    /// A chain of `{reference}`s comes back to itself.
    ReferenceCycle { path: String },
    /// A token exists in one mode and not in the other.
    ModeMismatch { token: String, missing_in: String },
    /// A contrast pair uses a usage other than those of `contrast.min`.
    UnknownUsage { group: String, usage: String },
    /// A declared contrast pair is below the minimum of its usage.
    Contrast {
        group: String,
        fg: String,
        bg: String,
        ratio: f64,
        min: f64,
    },
    /// A declared pair resolves to colours listed in `contrast.rejected`.
    RejectedPairDeclared {
        group: String,
        fg: String,
        bg: String,
    },
    /// A pair of `contrast.rejected` meets the text minimum, so it is not a rejection.
    RejectedPairPasses { fg: String, bg: String },
    /// A duration of `motion.reduced` is not zero.
    ReducedMotionNotZero { name: String },
    /// A shadow is neither `none` nor `x y blur rgba(r,g,b,a)`.
    BadShadow { path: String, value: String },
    /// A `type.family` names no embedded font.
    UnknownFamily { family: String },
}

impl fmt::Display for ThemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json { detail } => write!(f, "the tokens file is not JSON: {detail}"),
            Self::SchemaTooNew { found, supported } => {
                write!(f, "tokens schema {found} is newer than {supported}")
            }
            Self::SchemaUnknown { found } => write!(f, "tokens schema {found} is unknown"),
            Self::MissingToken { path } => write!(f, "token {path} is missing"),
            Self::BadValue { path } => write!(f, "token {path} has an invalid value"),
            Self::BadColor { path, value } => write!(f, "token {path}: {value:?} is not a colour"),
            Self::UnresolvedReference { path, target } => {
                write!(f, "token {path} refers to {target}, which does not exist")
            }
            Self::ReferenceCycle { path } => write!(f, "token {path} is part of a reference cycle"),
            Self::ModeMismatch { token, missing_in } => {
                write!(f, "token {token} is missing in mode {missing_in}")
            }
            Self::UnknownUsage { group, usage } => {
                write!(f, "contrast pair in {group} has unknown usage {usage}")
            }
            Self::Contrast {
                group,
                fg,
                bg,
                ratio,
                min,
            } => write!(f, "{group}: {fg} on {bg} is {ratio:.2}:1, below {min}:1"),
            Self::RejectedPairDeclared { group, fg, bg } => {
                write!(f, "{group}: {fg} on {bg} is a rejected pair")
            }
            Self::RejectedPairPasses { fg, bg } => {
                write!(f, "rejected pair {fg} on {bg} meets the text minimum")
            }
            Self::ReducedMotionNotZero { name } => write!(f, "motion.reduced.{name} is not 0"),
            Self::BadShadow { path, value } => write!(f, "token {path}: {value:?} is not a shadow"),
            Self::UnknownFamily { family } => write!(f, "font family {family} is not embedded"),
        }
    }
}

impl std::error::Error for ThemeError {}

impl ToMessage for ThemeError {
    fn to_message(&self) -> Message {
        match self {
            Self::Json { .. } => msg!("error.theme.json"),
            Self::SchemaTooNew { found, supported } => msg!(
                "error.theme.schema_too_new",
                found = Int(i64::try_from(*found).unwrap_or(i64::MAX)),
                supported = Int(i64::try_from(*supported).unwrap_or(i64::MAX))
            ),
            Self::SchemaUnknown { found } => msg!(
                "error.theme.schema_unknown",
                found = Int(i64::try_from(*found).unwrap_or(i64::MAX))
            ),
            Self::MissingToken { path } => {
                msg!("error.theme.missing_token", path = Text(path.clone()))
            }
            Self::BadValue { path } => msg!("error.theme.bad_value", path = Text(path.clone())),
            Self::BadColor { path, .. } => msg!("error.theme.bad_color", path = Text(path.clone())),
            Self::UnresolvedReference { path, target } => msg!(
                "error.theme.unresolved_reference",
                path = Text(path.clone()),
                target = Text(target.clone())
            ),
            Self::ReferenceCycle { path } => {
                msg!("error.theme.reference_cycle", path = Text(path.clone()))
            }
            Self::ModeMismatch { token, missing_in } => msg!(
                "error.theme.mode_mismatch",
                token = Text(token.clone()),
                mode = Text(missing_in.clone())
            ),
            Self::UnknownUsage { group, usage } => msg!(
                "error.theme.unknown_usage",
                group = Text(group.clone()),
                usage = Text(usage.clone())
            ),
            Self::Contrast {
                group,
                fg,
                bg,
                ratio,
                min,
            } => msg!(
                "error.theme.contrast",
                group = Text(group.clone()),
                fg = Text(fg.clone()),
                bg = Text(bg.clone()),
                ratio = Decimal(*ratio),
                min = Decimal(*min)
            ),
            Self::RejectedPairDeclared { group, fg, bg } => msg!(
                "error.theme.rejected_pair_declared",
                group = Text(group.clone()),
                fg = Text(fg.clone()),
                bg = Text(bg.clone())
            ),
            Self::RejectedPairPasses { fg, bg } => msg!(
                "error.theme.rejected_pair_passes",
                fg = Text(fg.clone()),
                bg = Text(bg.clone())
            ),
            Self::ReducedMotionNotZero { name } => {
                msg!("error.theme.reduced_motion", name = Text(name.clone()))
            }
            Self::BadShadow { path, .. } => {
                msg!("error.theme.bad_shadow", path = Text(path.clone()))
            }
            Self::UnknownFamily { family } => {
                msg!("error.theme.unknown_family", family = Text(family.clone()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use gafira_doc::msg::is_valid_key;

    use super::*;

    fn s() -> String {
        "x".to_string()
    }

    // S14-12: one key per variant, in the `theme` area.
    #[test]
    fn s14_12_every_variant_has_a_valid_theme_key() {
        let all = [
            ThemeError::Json { detail: s() },
            ThemeError::SchemaTooNew {
                found: 2,
                supported: 1,
            },
            ThemeError::SchemaUnknown { found: 0 },
            ThemeError::MissingToken { path: s() },
            ThemeError::BadValue { path: s() },
            ThemeError::BadColor {
                path: s(),
                value: s(),
            },
            ThemeError::UnresolvedReference {
                path: s(),
                target: s(),
            },
            ThemeError::ReferenceCycle { path: s() },
            ThemeError::ModeMismatch {
                token: s(),
                missing_in: s(),
            },
            ThemeError::UnknownUsage {
                group: s(),
                usage: s(),
            },
            ThemeError::Contrast {
                group: s(),
                fg: s(),
                bg: s(),
                ratio: 1.0,
                min: 4.5,
            },
            ThemeError::RejectedPairDeclared {
                group: s(),
                fg: s(),
                bg: s(),
            },
            ThemeError::RejectedPairPasses { fg: s(), bg: s() },
            ThemeError::ReducedMotionNotZero { name: s() },
            ThemeError::BadShadow {
                path: s(),
                value: s(),
            },
            ThemeError::UnknownFamily { family: s() },
        ];
        let mut keys: Vec<&str> = all.iter().map(|e| e.to_message().key.0).collect();
        for key in &keys {
            assert!(is_valid_key(key), "{key}");
            assert!(key.starts_with("error.theme."), "{key}");
        }
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), all.len(), "two variants share a key");
        let m = ThemeError::SchemaTooNew {
            found: 2,
            supported: 1,
        }
        .to_message();
        assert_eq!(
            m.args,
            vec![
                ("found", gafira_doc::msg::MsgArg::Int(2)),
                ("supported", gafira_doc::msg::MsgArg::Int(1))
            ]
        );
    }
}
