# gafira-ports: the ports and their registries

## Status

| Area | Status |
|---|---|
| Port traits in the thirteen modules of the crate | Implemented |
| `Registry<T>` and `RegistryError` | Implemented |
| One error enum per port family (ADR 0014) | Implemented |
| `PluginHost` | **Planned**: arrives with `gafira-plugin` (`specs/02-arquitectura-software.md:825-833`) |
| `LibraryRepository`, `SettingsRepository` | **Planned**: `specs/02-arquitectura-software.md:1343` |
| Frame-source trait for the `VideoNode` kind | **Planned**: `docs/adr/0015-layer-3-sibling-seams.md:23` |
| `Project`, `AssetRef` and `AssetTable` as the root of the unified model | **Planned**: `specs/02-arquitectura-software.md:199-215` (ARQ-55); `ProjectRepository` loads a `Document` today |

## Purpose

Layer-3 port traits and registries: traits and plain data only, with no I/O and no global
state. Adapters implement them; the engine consumes them (`crates/gafira-ports/src/lib.rs:1-5`).
The crate declares thirteen public modules and re-exports nothing at its root, so every item
is reached by its module path (`lib.rs:5`, `lib.rs:9-21`).

| Aspect | Value |
|---|---|
| Internal dependencies | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-time`, `gafira-raster` |
| External dependencies | `serde`, `kurbo` |
| `unsafe` | Forbidden by `#![forbid(unsafe_code)]` (`lib.rs:7`) |
| Layer-3 row in the workspace table | `specs/02-arquitectura-software.md:100`; edges fixed by ADR 0013 |

The edge set is the one ADR 0013 records, and CI enforces it: the V5 row reads
`gafira-ports gafira-geom gafira-color gafira-doc gafira-time gafira-raster`
(`.github/workflows/ci.yml:107`). ARQ-02 does not bind layer 3, yet the crate keeps a
`clippy.toml` byte-identical to the canonical copy that disallows `std::fs`, `std::net`,
thread spawning, `std::time::SystemTime::now`, `std::env`, `std::process`, `HashMap` and
`HashSet` (`crates/gafira-ports/clippy.toml:6-62`; ARQ-10; ADR 0013 decision 2).

Layering rules and the V5 gate that reads this crate's dependency edges are documented in
`02-architecture.md`.

## Module map

| Module | Ports and types | Registry |
|---|---|---|
| `assets` | `AssetProvider`, `AssetQuery`, `AssetPage`, `AssetItem` | `AssetRegistry` |
| `assist` | `TextAssistant`, `RewriteRequest`, `AssistReq`, `Suggestion` | `TextAssistantRegistry` |
| `common` | `Confidence`, `LegalClass`, `Bytes`, `UnixTime` | none |
| `export` | `Exporter`, `ExportSink`, `PageRenderer`, `ExportCaps` | `FormatRegistry` |
| `format` | `Importer`, `FormatDescriptor`, `ByteSignature`, `Imported` | `FormatRegistry` |
| `i18n` | `SystemLocaleSource`, `LocaleResolver`, `Translator`, `Locale` | none |
| `license` | `LicenseVerifier`, `LicenseStatus`, `LicenseKey` | none |
| `media` | `Container`, `Demuxer`, `Muxer`, `Codec`, `Decoder`, `Encoder` | `CodecRegistry` |
| `registry` | `Registry<T>`, `RegistryError`, `Describe` | the generic base |
| `render` | `RenderBackend`, `TileCache`, `Effect`, `Transition`, `RenderOp` | `EffectRegistry`, `TransitionRegistry` |
| `repository` | `ProjectRepository`, `LoadedProject` | none |
| `store` | `ContentStore` | none |
| `system` | `FileSystem`, `Clock`, `Notifier`, `ReadSeek`, `WriteSeek`, `FileWrite` | none |

## The registry

Every family that adapters extend is a `Registry<T>`; the crate ships no global one
(ARQ-31, `crates/gafira-ports/src/registry.rs:37-38`), so the composition root owns each
registry and passes it down.

```rust
// crates/gafira-ports/src/registry.rs:40-43
pub struct Registry<T: ?Sized> {
    items: Vec<Arc<T>>,
    by_id: BTreeMap<&'static str, usize>,
}
```

| Item | Contract |
|---|---|
| `new`, `Default` | An empty registry; it works for `T` without `Describe` |
| `register(item)` | Requires `T: Describe`; stores the item and indexes it by its static id |
| `get(id)` | The registered item for an id, or `None` |
| `iter` | Items in registration order, which is what discovery iterates |
| `len`, `is_empty` | Number of registered items |

`register` returns `Result<(), RegistryError>` and its only variant is
`DuplicateId { id: &'static str }`. A duplicate id is rejected and the registry is left
unchanged (`registry.rs:80-83`, `registry.rs:13-16`). Registration order is part of the
contract because format discovery and codec tie-breaking read it.

## Render ports

`RenderBackend` is the boundary to a raster implementation. It reports `BackendCaps`
(`kind` CPU or GPU, maximum texture, supported formats, whether it runs effects), opens a
frame over a `TargetDesc`, executes a `RenderPlan`, reads pixels back and exposes its own
memory accounting:

| Item | Contract |
|---|---|
| `caps` | Static capabilities of the adapter (render.rs:282) |
| `begin_frame(target)` | A `FrameCx` for the target, or a `RenderError` |
| `execute(cx, plan, cache, deadline, cancel)` | A `RenderReport`, never a partial failure |
| `read_back(cx, region)` | An `ImageBuf` of the requested region |
| `memory_usage()` | `MemoryUsage { host, device }` in bytes |
| `trim(budget)` | Release memory down to the budget (ARQ-44) |

The tile, region and origin contract is the part a backend must not improvise, and ARQ-149
fixes it: `RenderOp` values stay in page coordinates, `TileTask.region` is a half-open
page-pixel `IRect` already clipped to the request viewport, and `TargetDesc.origin` is the
surface's top-left corner in page pixels (`crates/gafira-ports/src/render.rs:100-105`,
`render.rs:150-156`). The backend therefore never needs to know the page size, and a tile
never depends on where the surface sits.

`TileKey` is the content key: a `ContentKey(u128)` for the rendered content plus level,
column and row, so the tile cache is content-addressed (ARQ-142, `render.rs:31-45`).
`TileCache` is the adapter the backend writes through: `get`, `put`, `bytes` and
`trim(budget)`, all in bytes, which is the governor contract of ARQ-44 (`render.rs:112-121`).

Cancellation and lateness are distinct. `execute` receives a `CancelToken` and reports
cancellation as `RenderError::Cancelled`, checked at least once per tile and per frame
(ARQ-139); a frame that could not finish inside its `Deadline` is not an error at all but a
`RenderReport { done, left }` whose `is_complete` is false (`render.rs:195-210`,
`render.rs:291-294`). `read_back` returns `Rgba8` or `RgbaF32` pixels and never `Alpha8`
(`render.rs:155-157`, `render.rs:308`). Recovery from a lost device or adapter lives in the
error variants — `DeviceLost`, `OutOfMemory`, `AdapterMissing`, `DriverBlocked` — which is
how ARQ-22 degrades GPU to CPU without touching the document.

`PageRenderer` is the narrow port used outside the interactive pipeline: it takes a
`PageRenderRequest` (page, time, scale, region, transparency) and returns a composed
`ImageBuf`, reporting cancellation as `RenderError::Cancelled` (`export.rs:129-142`).
`Effect` and `Transition` are the pluggable render extensions; their registries are
`Registry<dyn Effect>` and `Registry<dyn Transition>` (`render.rs:24-27`).

## Format ports

`Importer` and `Exporter` share one `FormatDescriptor` (`id`, extensions, MIME, byte
signatures, `FormatKind`, `LegalClass`). An importer adds `sniff`, which returns a
`Confidence`, and `import`, which reads from a `ReadSeek` and returns `Imported` as an asset,
nodes or pages (`format.rs:166-177`; FD-31). An exporter declares `capabilities`,
`options_schema`, `validate` and `export` (`export.rs:225-243`).

| Port | What it must guarantee |
|---|---|
| `ExportCaps` | Declares transparency, multipage, animation, CMYK, bleed and notes; `contains` answers a requirement set |
| `ExportIssue`, `IssueSeverity` | Validation before writing; `blocks()` separates a hard stop from a warning (FD-58) |
| `ExportScope` | Page, selection or range (FD-57) |
| `ExportSink` | `create(name)` hands back a `WriteSeek`; the caller owns the destination |
| `Exporter` | Lives in an adapter and never sees `gafira-render` (`export.rs:4-6`, ARQ-06) |

`FormatRegistry` is a single registry for both directions, with `register_importer`,
`register_exporter`, `importers`, `exporters`, `exporters_with` and the discovery entry
points `resolve_import` and `import` (`format.rs:191-330`). Discovery implements rules 1 to 6
of section 4.5 of the architecture spec: extension, last extension, case folding, signature
sniffing through `ByteSignature::matches` and the confidence order, with sorted iteration so
the outcome is deterministic.

## Media ports

Media is written as a push-and-pull pipeline so no adapter owns a thread: `Container`
probes a head, opens a `Demuxer` or creates a `Muxer`; `Demuxer` lists streams, reads
`Packet`s and seeks; `Muxer` adds streams, writes packets and finishes with a `MuxReport`;
`Codec` builds a `Decoder` or an `Encoder`, and both exchange `send` and `receive`
(`crates/gafira-ports/src/media.rs:293-404`).

`CodecRegistry::best_decoder` encodes the licence policy of D-02: an encumbered codec is
never selected, and among the rest the order is hardware, then OS-licensed, then
royalty-free, with registration order breaking a tie (`media.rs:448-452`). ARQ-77 — the
binary carries no H.264, HEVC or AAC codec of its own — is what the policy protects.

## Platform ports

| Port | What it demands |
|---|---|
| `FileSystem` | `open`, `create`, `replace`, `remove`, `exists`, `create_dir_all`, `read_dir`, all `std::io::Result` |
| `FileSystem::replace` | Atomic rename plus a directory sync, the mechanism behind ARQ-50 and step 3 of §7.2 (system.rs:47) |
| `Clock` | `now` as `UnixTime`; `monotonic` as a `Duration` that never goes back |
| `Notifier` | `notify(title, body)` with text already translated, best effort |
| `ReadSeek`, `WriteSeek`, `FileWrite` | Stream traits with blanket implementations over `std`; `FileWrite` adds `sync_all` |

`Clock` is the only source of time for the application (ARQ-138), and its monotonic half is
what `Deadline` is measured against; tests inject a fake clock to make state transitions
deterministic (ARQ-93). `Notifier` is deliberately weak: it carries no locale and no error,
because the message is formatted by `Translator` before it arrives (D-08).

## Document, content and asset ports

`ProjectRepository` loads and saves a document by path. `load` returns `LoadedProject`, which
carries a `read_only` flag: a file written by a newer schema loads read-only and is never
overwritten, and `RepositoryError::TooNew { found, supported }` reports the version pair
(`crates/gafira-ports/src/repository.rs:14-23`, `repository.rs:68-83`; ARQ-55). Until the
unified model lands, the payload is a `Document`, not a `Project`.

`ContentStore` is the content-addressed byte store: `put` returns a `ContentHash`, `open`
returns a `ReadSeek` and `contains` answers without reading; `StoreError` is `NotFound` or
`Io` (`store.rs:14-71`). `AssetProvider` is the library port: `kinds`, `search` over an
`AssetQuery` (offset and limit, where zero means no limit), `thumbnail` bounded by the
longest side and `materialize`, which copies the asset into a `ContentStore` and reports the
hash and MIME (`assets.rs:126-155`).

## Locale, text assistance and license ports

`i18n` fixes the language vocabulary of layer 3 while the domain stays language-free
(ARQ-107): `Locale` is `Es` or `En` and nothing else, `LanguagePreference` adds `System` and
parses from and serializes to its own name, and `SystemLocaleSource::system_locale` is the
port through which the platform reports the system UI and format locales
(`crates/gafira-ports/src/i18n.rs:14-141`; RF-65, RF-66, D-08). `LocaleResolver` turns a
preference into a `ResolvedLocale` with a `LocaleSource` of manual, system or fallback, and
`Translator` formats text, numbers, dates, relative times and quantities for a
`FormatLocale` (`i18n.rs:145-156`; ARQ-100).

`TextAssistant` is the optional local rewrite port: `requirements` reports RAM, GPU and model
size, and `rewrite` returns suggestions for a tone, a length and a language, given `Progress`
and a `CancelToken` (`assist.rs:101-120`). A suggestion is never applied on its own — the
user picks one (`assist.rs:59`) — and the port is local, off by default and outside the MVP
(ARQ-26, FD-64). Its errors are `ModelMissing` and `Cancelled`.

`LicenseVerifier` keeps the licence decision local except for activation: `status` is a pure
function of `UnixTime` with no network, `activate` and `validate` are the only calls that may
reach the network, and `deactivate` releases the seat
(`crates/gafira-ports/src/license.rs:108-133`; FD-05, RNF-04). `LicenseStatus` has five
values — `Active`, `OfflineGrace`, `Expired`, `Unlicensed`, `Invalid` — with no trial state
(D-12), and `can_export` is true only for `Active` and `OfflineGrace` (D-55). `LicenseKey`
has a redacted `Debug` and no `Display`, so the key cannot reach a log by accident
(ARQ-94, `license.rs:57-79`).

## Errors and cancellation

Each port family owns one error enum, as ADR 0014 requires
(`specs/02-arquitectura-software.md:1590`); errors of lower layers travel as message keys
with parameters (ARQ-102).

| Error | Family | Variants |
|---|---|---|
| `RegistryError` | generic | `DuplicateId` |
| `StoreError` | content store | `NotFound`, `Io` |
| `AssetError` | assets | `NotFound`, `Store`, `Io` |
| `RepositoryError` | repository | `Corrupt`, `TooNew`, `Io` |
| `ImportError` | import | `Unsupported`, `Corrupt`, `Store`, `Io`, `Cancelled` |
| `ExportError` | export | `Render`, `Media`, `Store`, `Io`, `Cancelled` |
| `RenderError` | render | `DeviceLost`, `OutOfMemory`, `AdapterMissing`, `DriverBlocked`, `Effect`, `Buffer`, `Cancelled` |
| `MediaError` | media | `UnsupportedCodec`, `Corrupt`, `SystemCodecUnavailable`, `ChannelMismatch`, `Io`, `Cancelled` |
| `LicenseError` | licence | `Network`, `Rejected`, `Storage` |
| `AssistError` | assist | `ModelMissing`, `Cancelled` |

`Cancelled` is not defined here. It is the unit struct declared by `gafira-doc` in
`crates/gafira-doc/src/contract/cancel.rs:64`, re-exported as
`gafira_doc::contract::Cancelled`, next to the `CancelToken` that carries the request and the
`Progress` trait that reports it (`cancel.rs:14`, `cancel.rs:87`). It is the one cancelled
value of every engine (ADR 0012), and ports take borrowed tokens and wrap the result, so
cancellation crosses a trait boundary without the port inventing its own flag:

```rust
// crates/gafira-ports/src/render.rs:212-224, 248
pub enum RenderError {
    // ...
    Cancelled(Cancelled),
}

impl From<Cancelled> for RenderError {
    fn from(c: Cancelled) -> Self { Self::Cancelled(c) }
}
```

The same pattern, with the same `From<Cancelled>` conversion, exists in `ExportError`
(`export.rs:154`, `export.rs:206`), `ImportError` (`format.rs:120`, `format.rs:147`),
`MediaError` (`media.rs:32`, `media.rs:67`) and `AssistError` (`assist.rs:70`,
`assist.rs:85`). The methods that accept a token are `RenderBackend::execute`,
`PageRenderer::render`, `Exporter::export`, `Importer::import` and
`TextAssistant::rewrite`.

## Tests

Integration tests live in `crates/gafira-ports/tests/`, one file per module, and each test
name carries the requirement ID it proves.

| File | Contract pinned |
|---|---|
| `registry.rs` | Empty registry without `Describe`, registration order, duplicate id rejected |
| `render.rs` | Backend through `dyn`, late frame is a report, cancel is an error, `read_back` formats, `TileCache` budget |
| `export.rs` | `ExportCaps`, `ExportIssue::blocks`, object safety of `Exporter`, `ExportSink`, `PageRenderer` |
| `format.rs` | Signature table, sniff confidence, extension and sorting rules, duplicate importer id |
| `media.rs` | Decoder selection policy, `Cancelled` mapping, `AudioBuf` channel mismatch, dyn compatibility |
| `system.rs` | Atomic save trace, filesystem operations through `dyn`, injectable `Clock`, `Notifier` |
| `repository.rs`, `store.rs`, `assets.rs` | Object safety through `dyn`, document round trip, store and asset errors |
| `i18n.rs`, `license.rs`, `assist.rs`, `common.rs` | Preference parsing, `can_export`, key redaction, rewrite cancellation, `Confidence` order |

Object safety is tested explicitly because every port is consumed as `dyn Trait` by the
engine; the `Error` messages are pinned against `ToMessage` keys, which is how ARQ-102 is
enforced per family.
