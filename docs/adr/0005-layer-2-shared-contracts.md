# 0005. Layer-2 shared contracts live in a `gafira-doc` module

- Status: accepted
- Date: 2026-10-30
- Related: ARQ-01, ARQ-98, ARQ-102, ARQ-110 to ARQ-112, ARQ-119, ADR-01 (02 §2.3), changes `layout-core`, `engine-contracts`

## Context

The layer-2 traits (`Effect`, `Transition`, `AudioProcessor`, `Segmenter`) use types that 02 assigns to no crate. Layer 2 cannot depend on `gafira-ports` (ARQ-01), which owns `Registry<T>` (02 §2.1). 02 §4.17 leaves the location to design.

The set is not uniform. Only six types are shared by engines that do not depend on each other. The rest have one natural owner.

## Decision

A new module `gafira_doc::contract` holds the types shared across engines. Every other type goes to the crate that owns the concept. No new crate.

| Type | Home | Why |
|---|---|---|
| `Describe`, `Descriptor` | `gafira_doc::contract` | Used by raster, audio, segment and `gafira-ports`; labels are `MessageKey` (ARQ-98) |
| `ParamSchema`, `ParamValues` | `gafira_doc::contract` | `ParamValues` is persisted (`EffectInstance`, saved filters), so it cannot sit above `gafira-doc` |
| `CancelToken`, `Progress` | `gafira_doc::contract` | Used by raster, segment, media and engine (ARQ-112) |
| `AudioCrossfade` | `gafira_doc::contract` | Plain enum returned by `Transition::audio()` (raster) and read by audio; neither engine depends on the other |
| `IRect` | `gafira-geom` | Integer sibling of `Rect` |
| `ImageView`, `ImageViewMut`, `EffectCx` | `gafira-raster` | Buffers are raster's job (02 §2.1); `segment -> raster` already exists |
| `AudioBlock`, `ParamSnapshot` | `gafira-audio` | Only the audio chain uses them |

`gafira_doc::contract` imports only `crate::msg`, `gafira-color`, `serde` and `std`. It never imports `Page`, `Node` or `Document`.

## Consequences

- No new row in 02 §2.1 and no new V5 row. `gafira-doc` stays the only crate below the engines that they must share.
- `gafira-doc` gains a concern beyond the document model. The import rule above keeps it separable: moving the module to its own crate later costs one `pub use` and one V5 row, with no API change.
- `gafira-audio` links `gafira-doc`, and so `kurbo` and `serde_json`, without using them at run time. A change in `gafira-doc` rebuilds all five engines; this is already true of four.
- `Registry<T>` stays in `gafira-ports`; only its bound `Describe` moves below it.

## Alternatives considered

- **New layer-0 crate (`gafira-contracts`).** ADR-01 criterion 4 allows it, but it brings little. It must sit below `gafira-doc` because `ParamValues` is persisted, so `msg` must move into it too (02 §2.1, §14.1, §14.2). And it does not remove the doc edge: every engine error implements `ToMessage` (ARQ-102, 02:1269, 02:1814), raster needs `BlendMode::ALL` (ARQ-119, `crates/gafira-doc/src/blend.rs`) and layout needs the page types. Only audio would shed the dependency. A new 02 §2.1 row, a V5 row and a `clippy.toml` copy to save one edge is not worth it now.
- **All twelve types in `gafira_doc::contract`.** Puts pixel and audio buffers in the document crate, against its 02 §2.1 row, with no consumer that needs them there.
- **New crate above `gafira-doc`.** Cannot hold `ParamValues`, and pulls `gafira-doc` anyway.
- **`gafira-ports`.** Layer 3; forbidden by ARQ-01.
