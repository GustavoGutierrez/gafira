//! Neutral message keys (02 §14.2, ARQ-107): no language, no translated text.

/// The key of a catalog message, for example `"edit.set_property"`.
///
/// Keys are ASCII `snake_case` segments joined by dots (02 §14.2). They are
/// not validated here and are never serialized into a document.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct MessageKey(pub &'static str);

/// The label of a command, shown as «Undo {label}».
pub type CommandLabel = MessageKey;

/// Display units of a quantity argument. Not `gafira_geom::Unit`: no conversion.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Unit {
    Px,
    Mm,
    Cm,
    In,
    Pt,
    Percent,
    Bytes,
    Seconds,
    Fps,
    Db,
    Lufs,
    Degrees,
}

/// A typed argument of a message; the UI formats it for the active locale.
#[derive(Clone, Debug, PartialEq)]
pub enum MsgArg {
    Text(String),
    Count(u64),
    Int(i64),
    Decimal(f64),
    Quantity(f64, Unit),
    Timestamp(i64),
    Elapsed(i64),
    Key(MessageKey),
}

/// A message key with named arguments, never a translated text (ARQ-102).
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub key: MessageKey,
    pub args: Vec<(&'static str, MsgArg)>,
}

/// Implemented by every error type: a key with parameters, never prose.
pub trait ToMessage {
    fn to_message(&self) -> Message;
}

/// True for two or three dot-separated segments of `[a-z][a-z0-9_]*`.
#[must_use]
pub const fn is_valid_key(key: &str) -> bool {
    let bytes = key.as_bytes();
    let mut segments = 0;
    let mut at_start = true;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if at_start {
            if !b.is_ascii_lowercase() {
                return false;
            }
            segments += 1;
            at_start = false;
        } else if b == b'.' {
            at_start = true;
        } else if !(b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') {
            return false;
        }
        i += 1;
    }
    !at_start && segments >= 2 && segments <= 3
}

/// Builds a [`Message`] from a literal key and named arguments, checking the
/// key grammar ([`is_valid_key`]) at compile time.
///
/// ```
/// use gafira_doc::msg;
/// use gafira_doc::msg::MsgArg;
///
/// let m = msg!("export.pages_selected", count = Count(3));
/// assert_eq!(m.args, vec![("count", MsgArg::Count(3))]);
/// ```
///
/// An invalid key does not compile:
///
/// ```compile_fail
/// let _ = gafira_doc::msg!("Not A Key");
/// ```
#[macro_export]
macro_rules! msg {
    ($key:literal $(, $name:ident = $variant:ident ( $($value:expr),* $(,)? ))* $(,)?) => {{
        const _: () = assert!($crate::msg::is_valid_key($key));
        $crate::msg::Message {
            key: $crate::msg::MessageKey($key),
            args: ::std::vec![$((
                ::core::stringify!($name),
                $crate::msg::MsgArg::$variant($($value),*)
            )),*],
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    // ARQ-102
    #[test]
    fn arq_102_every_arg_variant_and_unit_builds_and_compares() {
        let units = [
            Unit::Px,
            Unit::Mm,
            Unit::Cm,
            Unit::In,
            Unit::Pt,
            Unit::Percent,
            Unit::Bytes,
            Unit::Seconds,
            Unit::Fps,
            Unit::Db,
            Unit::Lufs,
            Unit::Degrees,
        ];
        let mut args = vec![
            MsgArg::Text("t".into()),
            MsgArg::Count(1),
            MsgArg::Int(-1),
            MsgArg::Decimal(0.5),
            MsgArg::Timestamp(2),
            MsgArg::Elapsed(3),
            MsgArg::Key(MessageKey("a.b")),
        ];
        args.extend(units.iter().map(|u| MsgArg::Quantity(1.0, *u)));
        let msg = Message {
            key: MessageKey("a.b"),
            args: args.iter().cloned().map(|a| ("n", a)).collect(),
        };
        assert_eq!(msg.clone(), msg);
        let other = Message {
            key: MessageKey("a.c"),
            ..msg.clone()
        };
        assert_ne!(other, msg);
        assert_ne!(
            MsgArg::Quantity(1.0, Unit::Px),
            MsgArg::Quantity(1.0, Unit::Mm)
        );
    }

    #[test]
    fn a_label_is_a_copyable_key() {
        const L: CommandLabel = MessageKey("edit.set_property");
        let copy = L;
        assert_eq!(L, MessageKey("edit.set_property"));
        assert_eq!(copy, L);
        assert_ne!(L, MessageKey("edit.insert_nodes"));
    }
}
