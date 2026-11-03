# 0014. One error enum per port family in `gafira-ports`

- Status: accepted (owner confirmation 2026-11-02)
- Date: 2026-11-02
- Related: ARQ-102, 02 §7.3, ADR 0012, change `gafira-ports`

## Context

02 §7.3 asks for one error enum per crate, then names `ImportError`, `ExportError`, `MediaError`, `RenderError` and `PluginError`. Each sits in a port signature, so each must live in `gafira-ports`, along with `AssetError`, `LicenseError` and `AssistError`. ADR 0012 sets `<area>` to the crate name or a shared area, which would put every key under `error.ports`.

## Decision

1. Each port family has its own hand-written enum in its module of `gafira-ports`. This is an exception to "one enum per crate" for layer 3 ports only.
2. Key areas are the family, not the crate: `import`, `export`, `render`, `media`, `asset`, `store`, `repository`, `license`, `assist`, `registry`. All other rules of ADR 0012 apply: `Cancelled` delegates, `to_message` has no `_` arm, library text stays in `Display`.
3. An enum may wrap another with `From`, and its `to_message` delegates, as with `BufferError` (02 §7.3).
4. I/O failures keep `std::io::Error`, as `FormatError` does. These enums derive `Debug` only and give the key no argument.

## Consequences

- Adapters return the enum their port names, and do not need one of their own for that surface. An adapter's internal enum, such as `FormatError`, maps into it.
- Keys read by meaning in catalogs: `error.render.device_lost`, not `error.ports.render_device_lost`.
- Tests compare these enums by variant, not with `assert_eq!`.
- ARQ-102's fixture test covers ten more enums.

## Alternatives considered

- **One `PortError` enum.** Couples unrelated adapters. `RenderBackend` could return a licence error.
- **Enums in the adapter crates.** A port signature cannot name a type above it.
- **Associated `type Error` on each trait.** Breaks `dyn`, on which every registry relies.
- **`error.ports.<variant>`.** Fits ADR 0012 literally, but three-segment keys cannot hold both family and variant without folding.
