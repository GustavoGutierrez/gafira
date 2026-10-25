# Document Model Specification (delta for C3 `edit-history`)

## Purpose

Adds the minimal `msg` module to `gafira-doc` so that `Command::label` returns the final `CommandLabel` type from day one (02 §14.2; `design.md` Q11). The rest of `msg` (`Message`, `MsgArg`, `ToMessage`, `msg!`) stays for the i18n change. All other `document-model` requirements are unchanged.

## ADDED Requirements

### Requirement: Message key for command labels (ARQ-107) (D1, Q11)

`gafira-doc` MUST expose a public module `msg` with `#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct MessageKey(pub &'static str)` and `pub type CommandLabel = MessageKey`, exactly as 02 §14.2 writes them. The module MUST NOT contain text in any language, a `Locale`, or literals other than keys (ARQ-107). Keys follow the 02 §14.2 convention (ASCII `snake_case` segments joined by dots); this change does not validate keys at runtime (the catalog check is `xtask check-i18n`, later). `MessageKey` MUST NOT be serialized into the document.

#### Scenario: Label is a key (D1, unit)
- GIVEN `const L: CommandLabel = MessageKey("edit.set_property");`
- WHEN it is compared with `MessageKey("edit.set_property")` and copied
- THEN they are equal and the copy is still usable (the type is `Copy`)

#### Scenario: No serde on MessageKey (D1, command)
- GIVEN `crates/gafira-doc/src/msg.rs`
- WHEN it is searched for `Serialize` and `Deserialize`
- THEN neither appears
