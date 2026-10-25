//! Neutral message keys (02 §14.2, ARQ-107): no language, no translated text.

/// The key of a catalog message, for example `"edit.set_property"`.
///
/// Keys are ASCII `snake_case` segments joined by dots (02 §14.2). They are
/// not validated here and are never serialized into a document.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct MessageKey(pub &'static str);

/// The label of a command, shown as «Undo {label}».
pub type CommandLabel = MessageKey;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_label_is_a_copyable_key() {
        const L: CommandLabel = MessageKey("edit.set_property");
        let copy = L;
        assert_eq!(L, MessageKey("edit.set_property"));
        assert_eq!(copy, L);
        assert_ne!(L, MessageKey("edit.insert_nodes"));
    }
}
