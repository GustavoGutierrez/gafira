# Design: `gafira-ports`

This design fixes the crate's surface, its V5 row and its deliveries. The key moves: `gafira-ports` sits directly above `gafira-raster`, the render contract lives in ports so adapters never see `gafira-render`, and `D1` lands every shared-file change so later deliveries touch only their own module.

Evidence: 02 §1.3, §2.1 to §2.3, §4 intro, §4.5 to §4.16, §5.3, §5.5, §6.1 to §6.4, §7.3, §11, §13, §14.1 to §14.4; 00 (D-02, D-08, D-12); 04 FD-31, FD-55, FD-57, FD-58, FD-64; 05 V-5, FV-02, FV-12, FV-23; ADR 0005, 0007, 0008, 0012; `.github/workflows/ci.yml`; `crates/gafira-doc/src/contract/`, `crates/gafira-raster/src/view.rs`, `crates/gafira-segment/src/lib.rs`, `crates/gafira-testkit/`. ADRs: [0013](../../../docs/adr/0013-ports-edges-and-v5-row.md), [0014](../../../docs/adr/0014-one-error-enum-per-port.md), both accepted by the owner on 2026-11-02.

## Flow

`Flow: full | level: L2 | pack: new capability | mutation: archive | tier T4 at close | verify V2`. The floor is the new V5 row. The coordinator owns this line.

## Quick path

1. Merge `D1` alone.
2. Run `D2`, `D3`, `D4`, `D5a` and `D6` in any order, or in parallel worktrees.
3. Run `D5b` last.
4. T4 at close: `gafira-ports`, then `gafira-doc`, one at a time.

O1 and O2 are closed. S1 to S16 block no delivery and can land at any time.

## Decisions

### P1. V5 row, dependencies, lints

```text
gafira-ports   gafira-geom gafira-color gafira-doc gafira-time gafira-raster
```

| Dependency | Why | Used by |
|---|---|---|
| `gafira-doc` | `Describe`, `Descriptor`, `ParamSchema`, `ParamValues`, `CancelToken`, `Cancelled`, `Progress`, `msg`, `Document`, `Page`, `Node`, `NodeId`, `PageId`, `BlendMode`, `ContentHash` (P5) | Every module |
| `gafira-raster` | `ImageBuf`, `PixelFormat`, `AlphaMask`, `Effect`, `Transition`, `BufferError`, `EffectError` | `render`, `media`, `assets`, aliases |
| `gafira-time` | `Flicks`, `FrameRate`, `SampleRate` | `media`, `render` |
| `gafira-geom` | `IRect`, `Affine` | `render` |
| `gafira-color` | `Color` | `render`; also transitive through `gafira-doc` |
| `serde` (external) | `Locale`, `LanguagePreference` (02 §4.16) | `i18n` |
| `kurbo` (external) | `BezPath` in `RenderOp`; `gafira-geom` does not re-export it | `render` |

Dev-dependencies: `proptest`, `serde_json`. No `gafira-testkit`: tests use local doubles.

Forbidden, by omission from the row:

| Crate | Why |
|---|---|
| `gafira-text`, `gafira-pathops`, `gafira-layout` | No signature needs them. `gafira-text` would bring parley into every adapter |
| `gafira-audio`, `gafira-segment` | `AudioProcessor` and `Segmenter` live there, and their registries are plain `Registry<dyn _>` at the composition root (P4). `gafira-segment` would bring `rayon` |
| `gafira-anim`, `gafira-edit` | No signature needs them |
| `gafira-render`, `gafira-media`, `gafira-engine` | They depend on ports, so any such edge is a cycle |
| `gafira-testkit` | Support crate, see P7 |
| ICU crates | `Region` is an own type (P9). The ARQ-107 dependency check stays on for this crate |

| Question | Decision |
|---|---|
| `clippy.toml` (V12) | Yes, byte-identical. The V12 step requires one for every row except testkit, ports does no I/O, and it costs nothing. ARQ-02 does not bind layer 3; this copy is stricter than the spec |
| `Registry` map | `BTreeMap`, because of that `clippy.toml` (S1) |
| ARQ-107 step | **Must change.** The step reads its crates from the V5 table and greps `Locale` in each (`.github/workflows/ci.yml`, "Layers 0-2 know no language"). `gafira-ports` defines `Locale`, so a plain new row fails CI. `D1` skips only the `Locale` grep for `gafira-ports`, with a comment saying it is layer 3. Its dependency check still runs |
| `forbidden` regex of V5 | No change |
| `#![forbid(unsafe_code)]` | Yes (policy step) |
| ARQ-05 `cargo public-api` | Not added: no such step exists for `gafira-doc` either. `lib.rs` holds only `pub mod`; helpers are `pub(crate)` |

Graph check. `ports -> raster -> doc -> geom, color`, and `ports -> time`. Each of `render`, `media`, `render-cpu`, `render-gpu`, `io-*`, `mux`, `codec-*`, `assets`, `format`, `license`, `i18n` and `platform` depends on ports and lower layers. No cycle in normal edges. The one dev-edge loop is closed by P7.

### P2. Render contract and ARQ-06

`gafira-render-cpu` and `gafira-render-gpu` must not see `gafira-render` (ARQ-06), and `RenderBackend::execute` names `RenderPlan` and `TileCache`, so both move below `gafira-render`.

| Type | Options | Decision |
|---|---|---|
| `RenderPlan` | (a) data in ports; (b) new crate; (c) associated type, which breaks `dyn RenderBackend` for `BackendSelector` | (a). `gafira-render` builds it, backends read it. A new crate fails ADR-01 |
| `RenderOp` | (a) closed enum; (b) `#[non_exhaustive]`, which forces a `_` arm that silently skips an op | (a). A new variant must break both backends at compile time |
| Effects in a plan | (a) `Arc<dyn Effect>` plus resolved params; (b) an id, plus a registry inside each backend | (a). The plan is self-contained |
| `TileCache` | (a) concrete LRU in ports; (b) trait in ports, implemented by `gafira-render` | (b). 02 §2.1 gives caches to `gafira-render`, and ports must not implement (S2). A GPU backend keeps its textures privately, by `TileKey`, and frees them in `trim` |
| `FrameCx` | (a) `Box<dyn Any>`; (b) associated type, which breaks `dyn`; (c) plain handle | (c). All methods take `&mut self`, so a backend holds its own state by `FrameId` |
| `Deadline` | (a) `Instant`; (b) monotonic `Duration`, as `History` in 02 §5.1 | (b), from `Clock::monotonic` |
| Late render | `Err`, or a report | Report. 02 §6.2 says the frame presents its best mip, so `RenderReport` counts what was left |
| `BackendSelector` | ports or `gafira-render` | `gafira-render` (02 §2.1, §11.1). Nothing here |

```rust
pub struct RenderPlan { pub quality: RenderQuality, pub tasks: Vec<TileTask> } // in priority order (02 §6.1)
pub struct TileTask { pub key: TileKey, pub priority: Priority, pub ops: Vec<RenderOp> }
pub struct TileKey { pub content: ContentKey, pub level: u8, pub column: u32, pub row: u32 }
pub struct ContentKey(pub u128);                 // computed by gafira-render
pub enum Priority { Interactive, Visible, Prefetch, Background } // 02 §6.3
pub enum RenderQuality { Preview, Export }       // 02 §6.2

pub enum RenderOp {
    PushLayer,
    PopLayer { blend: BlendMode, opacity: f32 },
    FillPath { path: Arc<BezPath>, color: Color, transform: Affine },
    DrawImage { image: Arc<ImageBuf>, transform: Affine },
    ApplyMask { mask: Arc<AlphaMask> },
    ApplyEffect { effect: Arc<dyn Effect>, params: ParamValues, scale: f32 },
    ApplyTransition { transition: Arc<dyn Transition>, params: ParamValues, progress: f32 },
}

pub trait TileCache: Send {
    fn get(&self, key: &TileKey) -> Option<Arc<ImageBuf>>;
    fn put(&mut self, key: TileKey, tile: Arc<ImageBuf>);
}

pub trait RenderBackend: Send {
    fn caps(&self) -> BackendCaps;
    fn begin_frame(&mut self, target: &TargetDesc) -> Result<FrameCx, RenderError>;
    fn execute(&mut self, cx: &mut FrameCx, plan: &RenderPlan, cache: &mut dyn TileCache,
               deadline: Deadline, cancel: &CancelToken) -> Result<RenderReport, RenderError>;
    fn read_back(&mut self, cx: &FrameCx, region: IRect) -> Result<ImageBuf, RenderError>;
    fn memory_usage(&self) -> MemoryUsage;
    fn trim(&mut self, budget: Bytes);
}
```

- `RenderOp` is the first cut from the nodes of 02 §6.1. Text reaches it as contours through `FillPath`. Hinted glyphs for Preview are a later variant, added by the `gafira-render` design. Ops carry no `Debug`, because `dyn Effect` has none.
- Blend formulas stay in `gafira-raster`. O5 of the layer-2 design must be ruled before `gafira-render-cpu`, not before this crate.

### P3. Scope boundary

| Lands now | Why |
|---|---|
| `Registry<T>`, `FormatRegistry`, `CodecRegistry`, aliases (P4) | §2.1 row, ARQ-26, ARQ-77 |
| `Importer`, `Exporter`, `PageRenderer` | §2.1 row. `PageRenderer` is new (S5) |
| `RenderBackend`, `TileCache` | §2.1 row, ARQ-22 |
| `Container`, `Demuxer`, `Muxer`, `Codec`, `Decoder`, `Encoder` | §2.1 row |
| `AssetProvider`, `ContentStore`, `ProjectRepository` | §2.1 row, §11.2 lists them as MVP |
| `FileSystem`, `Clock`, `Notifier` | §2.1 row, ARQ-50, ARQ-93, 02 §6.4, §8.4 |
| `LicenseVerifier` | §2.1 row, ARQ-90 to ARQ-95 |
| `TextAssistant`, `TextAssistantRegistry` | ARQ-26 requires an empty registry in the MVP build; FD-64 places the trait here. Plain code, no feature |
| `Translator`, `LocaleResolver`, `SystemLocaleSource` and §4.16 types | §2.1 row, ARQ-100 |

| Stays out, with no stub | Arrives with |
|---|---|
| `PluginHost` and its 7 types (§4.12) | `gafira-plugin`, Fase 3. ARQ-24 is Fase 3, and `Contributions` and `PluginRequest` depend on an ABI that does not exist (S6) |
| `LibraryRepository`, `SettingsRepository` (§5.3) | `gafira-assets`, `gafira-engine`. No signature, not in the §2.1 row. `LanguagePreference::parse` lands now, so the §14.4 rule has a home |
| Version operations of `ProjectRepository` | `gafira-format` |
| `ExportRequest::builder`, `EncoderConfig::builder` | `gafira-engine`, `gafira-media`. No rule to validate yet. Fields are public |
| `best_encoder`, sequence export scope | `gafira-media` |
| Font, audio device, hardware and lock ports of `gafira-platform` | `gafira-platform`. 02 names no trait |
| Fakes (`FakeLicenseVerifier`, `FakeSystemLocaleSource`, `InMemoryRepository`, a fake `FileSystem`, a fake `Clock`) | First consumer, under P7 |
| `extension-demo` (ARQ-20) | `gafira-engine` era: it needs a registered exporter and provider |
| `NodeExtension`, `NodeRegistry` | Plugin phase; `render_into` names `RenderGraphBuilder`, which is above ports |

A later addition to this crate is a small delivery of the crate that needs it. It reopens no decision here.

### P4. Ports and type homes

"exists" means the type is on `develop` today. Everything else in `gafira-ports`, by module.

| Module | Port or type | Home | Note |
|---|---|---|---|
| any | `Describe`, `Descriptor`, `ParamSchema`, `ParamValues`, `CancelToken`, `Cancelled`, `Progress` | `gafira_doc::contract`, exists | |
| any | `Message`, `MessageKey`, `ToMessage`, `msg::Unit` | `gafira_doc::msg`, exists | Not `gafira_geom::Unit` |
| `registry` | `Registry<T>`, `RegistryError` | ports | P8 |
| `common` | `Confidence`, `LegalClass`, `Bytes`, `UnixTime` | ports | `UnixTime(pub i64)` in seconds, as `MsgArg::Timestamp` |
| `system` | `ReadSeek`, `WriteSeek`, `FileWrite`, `FileSystem`, `Clock`, `Notifier` | ports | Blanket impls for the first two |
| `store` | `ContentStore`, `StoreError` | ports | |
| any | `ContentHash` | `gafira-doc`, new | P5 |
| `format` | `FormatDescriptor`, `ByteSignature`, `FormatKind`, `Importer`, `ImportOptions`, `ImportCx`, `Imported`, `ImportedAsset`, `ImportError`, `FormatRegistry` | ports | |
| `export` | `Exporter`, `ExportCaps`, `ExportRequest`, `ExportScope`, `ExportIssue`, `IssueSeverity`, `ExportJob`, `ExportSink`, `ExportReport`, `ExportError`, `PageRenderer`, `PageRenderRequest` | ports | |
| `render` | `RenderBackend`, `TileCache`, `RenderPlan`, `TileTask`, `TileKey`, `ContentKey`, `RenderOp`, `Priority`, `RenderQuality`, `BackendCaps`, `BackendKind`, `TargetDesc`, `FrameCx`, `FrameId`, `Deadline`, `RenderReport`, `MemoryUsage`, `RenderError` | ports | P2 |
| any | `ImageBuf`, `AlphaMask`, `PixelFormat`, `Effect`, `Transition`, `BufferError`, `EffectError` | `gafira-raster`, exists | |
| any | `IRect`, `Affine` | `gafira-geom`, exists | |
| any | `Flicks`, `FrameRate`, `SampleRate` | `gafira-time`, exists | |
| `media` | `Container`, `Demuxer`, `Muxer`, `Codec`, `Decoder`, `Encoder`, `MediaError`, `MuxConfig`, `MuxReport`, `StreamInfo`, `StreamIdx`, `Packet`, `SeekMode`, `CodecId`, `CodecCaps`, `CodecParams`, `EncoderConfig`, `Frame`, `VideoFrame`, `AudioBuf`, `CodecRegistry` | ports | |
| `assets` | `AssetProvider`, `AssetKind`, `AssetQuery`, `AssetItem`, `AssetPage`, `ProviderAssetId`, `Materialized`, `AssetError`, `AssetRegistry` | ports | |
| `repository` | `ProjectRepository`, `LoadedProject`, `RepositoryError` | ports | Over `Document`: S15 |
| `license` | `LicenseVerifier`, `LicenseStatus`, `LicenseKey`, `LicenseError`, `ReasonCode` | ports | O1 (a) |
| `assist` | `TextAssistant`, `AssistReq`, `RewriteRequest`, `Suggestion`, `AssistError`, `TextAssistantRegistry` | ports | |
| `i18n` | `Locale`, `LanguagePreference`, `SystemLocale`, `SystemLocaleSource`, `ResolvedLocale`, `LocaleSource`, `LocaleResolver`, `FormatLocale`, `Region`, `Translator`, `NumberStyle`, `DateStyle` | ports | P9 |
| `render` | `EffectRegistry`, `TransitionRegistry` | ports | `pub type` over raster traits |
| none | `SegmenterRegistry`, `AudioRegistry` | Composition root | `Registry<dyn Segmenter>`: an alias here would add a forbidden edge |
| none | `ColorRegistry` | Omitted | `ColorTransform` has no `Describe`; no consumer |
| none | `BackendSelector`, `Planner`, `RenderGraph` | `gafira-render` | |
| none | `Ports`, `Registries`, `Waker`, `EngineEvent`, `UserError` | `gafira-engine` | |
| none | `SecretStore` | `gafira-license` | §13.2: internal, not a port |

Shapes 02 leaves open:

```rust
pub struct ImportCx<'a> {
    pub store: &'a dyn ContentStore, pub progress: &'a dyn Progress,
    pub cancel: &'a CancelToken, pub warnings: Vec<Message>,
}
pub enum Imported { Asset(ImportedAsset), Nodes { roots: Vec<NodeId>, nodes: Vec<Node> }, Pages(Vec<Page>) }
pub struct ImportedAsset { pub hash: ContentHash, pub mime: String }
pub struct ImportOptions { pub params: ParamValues }

pub struct ExportRequest { pub format_id: String, pub scope: ExportScope, pub options: ParamValues }
pub enum ExportScope { Pages(Vec<PageId>), Nodes { page: PageId, nodes: Vec<NodeId> } } // ranges resolved by the caller
pub struct ExportJob<'a> {
    pub request: &'a ExportRequest, pub document: &'a Document,
    pub store: &'a dyn ContentStore, pub pages: &'a dyn PageRenderer,
}
pub trait ExportSink {                           // temp file and rename live in its implementation (02 §6.4)
    fn create(&mut self, name: &str) -> std::io::Result<Box<dyn WriteSeek + Send>>;
}
pub trait PageRenderer: Send + Sync {            // implemented by gafira-render
    fn render(&self, req: &PageRenderRequest, cancel: &CancelToken) -> Result<ImageBuf, RenderError>;
}
pub struct PageRenderRequest { pub page: PageId, pub time: Flicks, pub scale: f32, pub region: IRect, pub transparent: bool }

pub trait ContentStore: Send + Sync {
    fn put(&self, src: &mut dyn std::io::Read) -> Result<ContentHash, StoreError>;
    fn open(&self, hash: ContentHash) -> Result<Box<dyn ReadSeek + Send>, StoreError>;
    fn contains(&self, hash: ContentHash) -> Result<bool, StoreError>;
}
pub trait FileWrite: std::io::Write + std::io::Seek + Send { fn sync_all(&mut self) -> std::io::Result<()>; }
pub trait FileSystem: Send + Sync {
    fn open(&self, path: &Path) -> std::io::Result<Box<dyn ReadSeek + Send>>;
    fn create(&self, path: &Path) -> std::io::Result<Box<dyn FileWrite>>;
    fn replace(&self, from: &Path, to: &Path) -> std::io::Result<()>; // atomic, per OS (02 §7.2 step 3), syncs the directory
    fn remove(&self, path: &Path) -> std::io::Result<()>;
    fn exists(&self, path: &Path) -> std::io::Result<bool>;
    fn create_dir_all(&self, path: &Path) -> std::io::Result<()>;
    fn read_dir(&self, path: &Path) -> std::io::Result<Vec<PathBuf>>;
}
pub trait Clock: Send + Sync { fn now(&self) -> UnixTime; fn monotonic(&self) -> Duration; }
pub trait Notifier: Send + Sync { fn notify(&self, title: &str, body: &str); } // best effort, text already translated (02 §6.4)

pub trait ProjectRepository: Send + Sync {
    fn load(&self, path: &Path) -> Result<LoadedProject, RepositoryError>;
    fn save(&self, path: &Path, document: &Document) -> Result<(), RepositoryError>;
}
pub struct LoadedProject { pub document: Document, pub read_only: bool } // ARQ-55

pub struct VideoFrame { pub pts: Flicks, pub image: ImageBuf }
pub struct AudioBuf { /* pts, sample_rate, planar channels: private, equal lengths (P8) */ }
pub struct Packet { pub stream: StreamIdx, pub pts: Flicks, pub dts: Flicks, pub keyframe: bool, pub data: Vec<u8> }

pub struct ExportCaps { pub transparency: bool, pub multipage: bool, pub animation: bool,
                        pub cmyk: bool, pub bleed: bool, pub notes: bool }  // Default: all false
pub struct CodecCaps { pub decode: bool, pub encode: bool, pub hardware: bool, pub legal: LegalClass }
pub enum LegalClass { RoyaltyFree, OsLicensed, Encumbered }
pub enum Confidence { None, Low, Medium, High }    // Ord: declaration order
pub struct ByteSignature { pub offset: usize, pub bytes: &'static [u8] }
```

Rules for all of these:

- Every public trait is object-safe, and a test in the module's `tests/` file proves it by naming `dyn Trait`.
- Public structs that adapters build have public fields and are not `#[non_exhaustive]`, which would stop construction outside the crate. Struct fields added later use `..Default::default()` where `Default` exists.
- `VideoFrame` holds RGBA in `ImageBuf`, the only owned pixel type. A planar form is a `gafira-media` decision (XP2).

### P5. `ContentHash`

`ContentStore` needs it now. It is persisted through `AssetRef`, so ADR 0008 rule 1 puts it in `gafira-doc`, not ports. `D1` adds `pub struct ContentHash(pub [u8; 32])` with derives `Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash`. No serde, no methods: the wire form arrives with `AssetTable` (ADR 0006). `gafira-doc` is therefore a touched crate and owes T4 at close. The type adds no mutable code, and that T4 needs no fix round for it.

### P6. Errors

ADR 0014: one hand-written enum per port family, in the port's module, each with `ToMessage` (ADR 0012).

| Enum | Key area | Variants |
|---|---|---|
| `RegistryError` | `error.registry` | `DuplicateId { id }` |
| `StoreError` | `error.store` | `NotFound { hash }`, `Io(io::Error)` |
| `ImportError` | `error.import` | `Unsupported`, `Corrupt { detail }`, `Store(StoreError)`, `Io(io::Error)`, `Cancelled(Cancelled)` |
| `ExportError` | `error.export` | `Render(RenderError)`, `Media(MediaError)`, `Store(StoreError)`, `Io(io::Error)`, `Cancelled(Cancelled)` |
| `RenderError` | `error.render` | `DeviceLost`, `OutOfMemory`, `AdapterMissing`, `DriverBlocked`, `Effect(EffectError)`, `Buffer(BufferError)`, `Cancelled(Cancelled)` |
| `MediaError` | `error.media` | `UnsupportedCodec { codec }`, `Corrupt { detail }`, `SystemCodecUnavailable`, `ChannelMismatch { expected, actual, channel }`, `Io(io::Error)`, `Cancelled(Cancelled)` |
| `AssetError` | `error.asset` | `NotFound { id }`, `Store(StoreError)`, `Io(io::Error)` |
| `RepositoryError` | `error.repository` | `Corrupt { detail }`, `TooNew { found, supported }`, `Io(io::Error)` |
| `LicenseError` | `error.license` | `Network`, `Rejected(ReasonCode)`, `Storage` |
| `AssistError` | `error.assist` | `ModelMissing`, `Cancelled(Cancelled)` |

- `detail` is library text: `Display` only, never an argument (ADR 0012 rule 3).
- `Io` keeps the `io::Error`, as `FormatError` in 02 §7.3, so these enums derive `Debug` only. `to_message` gives no argument for it.
- Wrapped errors delegate their `to_message`.

### P7. Fakes and the dev-edge loop

02 §4.14, §4.16 and §5.3 place fakes in `gafira-testkit`. A plain `testkit -> ports` edge means `testkit -> raster`, which breaks the V5 testkit row and ADR 0007. It also rebuilds ports and testkit on every `gafira-raster` mutant, because raster has `testkit` as a dev-dependency.

Decision (ADR 0013): `gafira-testkit` gains an optional feature `ports = ["dep:gafira-ports", "dep:gafira-raster"]`, off by default, with a `fakes` module behind it. Both dependencies are optional, so the default graph of testkit has neither. Consumers ask for it in their `dev-dependencies`. The V5 row keeps checking the default graph, so it does not change. Spec text stays true. Not built here: it lands with the first fake.

Rules, from XP1:

| Rule | Why (XP1) |
|---|---|
| `gafira-raster` never enables the `ports` feature and never depends on `gafira-ports`, in any section | Otherwise `cargo mutants -p gafira-raster` builds ports and testkit for every mutant |
| A fake that takes or returns raster types is used only from integration tests (`crates/<crate>/tests/`) or from other crates, never from a unit test in `gafira-raster/src` | Inside raster's unit tests, the loop `raster [dev] -> testkit -> ports -> raster` gives two copies of `gafira_raster`, and the types do not match (E0308) |
| Other crates enable it freely | Their tests, and raster's integration tests, see one copy |

`gafira-testkit/examples/extension-demo` (ARQ-20) uses a dev-dependency of testkit, which V5 does not walk.

### P8. Logic that needs tests

Traits have no behaviour. This does. "Crit" follows `AGENTS.md`.

| Item | Rule | Crit |
|---|---|---|
| `Registry::register` | A repeated `descriptor().id` gives `DuplicateId` and leaves the registry unchanged | Yes |
| `Registry::get`, `iter`, `len`, `is_empty` | `iter` is in registration order. Unknown id gives `None`. `new` needs no `Describe` bound | Yes |
| `ByteSignature::matches` | `head[offset..offset + len] == bytes`. A `head` too short, an overflowing offset or empty `bytes` gives `false`, with no panic | Yes |
| `FormatDescriptor::sniff_signatures` | Any match gives `High`, otherwise `None` | Yes |
| `FormatRegistry::resolve_import(path, head)` | Below | Yes |
| `FormatRegistry::import(path, head, src, opt, cx)` | Below | Yes |
| `FormatRegistry::register_importer`, `register_exporter` | A repeated id in the same list gives `DuplicateId`. The same id as importer and exporter is valid (`.gafira`) | Yes |
| `ExportCaps::contains(required)` | True when every flag set in `required` is set in `self`. Each of the 6 flags tested alone | Yes |
| `FormatRegistry::exporters_with(required)` | Those whose `capabilities()` contain `required`, in registration order | Yes |
| `ExportIssue::blocks` | `Error` blocks, `Warning` does not (FD-58) | Yes |
| `CodecRegistry::best_decoder(id)` | Below | Yes |
| `AudioBuf::new` | Unequal channel lengths give `ChannelMismatch` naming the first bad channel. No channels gives an empty buffer | Yes |
| `Deadline::has_passed(now)` | `Unbounded` never. `At(t)` when `now >= t` | Yes |
| `RenderReport::is_complete` | True when `left == 0` | Yes |
| `LanguagePreference::parse`, `as_str`, serde | `system`, `es`, `en`, exact. Anything else, `ES` included, reads as `System` (§14.4). Serialized form is that string. A non-string JSON value is a deserialize error (S11) | Yes |
| `Locale` serde | `es`, `en` | Yes |
| `Region::new` | 2 ASCII letters, stored upper case, or 3 ASCII digits. Otherwise `None`. `as_str` round-trips | Yes |
| `LicenseStatus::can_export` | True only for `Active` and `OfflineGrace` (D-12, O1) | Yes |
| `LicenseKey` | `Debug` prints no key byte; no `Display`; `expose()` gives the key (ARQ-94) | Yes |
| `to_message` of every enum | One fixture per variant: `is_valid_key` holds, the key has the area of P6, argument names are the field names, `detail` never appears (ARQ-102, ADR 0012) | Yes |
| `Display` of errors | Not critical | No |
| Dyn-compatibility tests, plain data | Not critical | No |

`resolve_import` returns `Vec<Arc<dyn Importer>>`:

1. `ext` is the last extension of the file name, ASCII lower-cased. A missing or non-UTF-8 extension gives no match.
2. Keep importers listing `ext`, and drop any with `sniff(head) == None`.
3. If none remain, take every importer and drop any with `sniff(head) == None`. A PNG named `.jpg` still opens (S3).
4. Sort by confidence, highest first. A stable sort keeps registration order on a tie.

`import` runs that list:

1. An empty list gives `Unsupported`.
2. Before each try, seek `src` to 0 and truncate `cx.warnings` to its length at the call. A failed seek returns `Io`.
3. The first `Ok` is the result.
4. `Err(Cancelled)` returns at once; nothing else is tried.
5. Any other `Err` moves to the next importer. When all fail, the first error is returned, because it comes from the most likely format.

`best_decoder(id)`, from 02 §4.8 and D-02:

1. Eligible: `codec_id() == id`, `caps().decode`, and `legal` is not `Encumbered`.
2. Order: `hardware` first, then `OsLicensed`, then `RoyaltyFree`. Registration order on a tie (S9).
3. None eligible gives `None`.

`ParamSchema::validate_definition` needs nothing here. It is tested in `gafira-doc`, and C12 of `engine-contracts` rejects a run-time call from a registry: each exporter crate tests its own `options_schema()`. `BackendSelector` is `gafira-render`'s.

### P9. i18n details

- `Region` is an own `Copy` type, not `icu_locale_core`'s, so adapters and the engine get `FormatLocale` without ICU. `es-419` needs the 3-digit form.
- `gafira-i18n` converts at its edge.
- Derives as in 02 §4.16, with `#[serde(rename_all = "lowercase")]` on `Locale`. `LanguagePreference` has hand-written serde (P8).

## Acceptance

| ID | What this change proves | Test or command | Delivery |
|---|---|---|---|
| ARQ-01 | Row of P1 holds | V5 step | `D1` |
| ARQ-05 | Public surface is `pub mod` plus public items; helpers `pub(crate)`. `cargo public-api` stays pending | Review | All |
| ARQ-06 | Backends can compile against ports alone: a test double of `RenderBackend` in `tests/` uses only `gafira-ports` and lower crates | `tests/render.rs` | `D3` |
| ARQ-20 | Traits are public and object-safe. `extension-demo` pending | `dyn` tests | All |
| ARQ-22 | `DeviceLost`, `OutOfMemory`, `AdapterMissing` and `DriverBlocked` exist and are told apart. Fallback pending in `gafira-render` | `tests/render.rs` | `D3` |
| ARQ-26 | `TextAssistantRegistry::default()` is empty; a test double registers | `tests/assist.rs` | `D6` |
| ARQ-31 | `Registry` is built by value; no `static` in the crate | `git grep`, CI pending | `D1` |
| ARQ-50 | `FileSystem` can express the write, sync and replace trace; a recording double runs it | `tests/system.rs` | `D1` |
| ARQ-77 | `best_decoder` never returns `Encumbered`. Registry walk pending | `tests/media.rs` | `D4` |
| ARQ-93 | `Clock` double is injectable. State machine pending in `gafira-license` | `tests/system.rs` | `D1` |
| ARQ-94 | `LicenseKey` `Debug` leaks nothing | `tests/license.rs` | `D2` |
| ARQ-100 | Types let a resolver double map tags. Table pending in `gafira-i18n` | `tests/i18n.rs` | `D2` |
| ARQ-102 | P8 row, for all ten enums. Catalogue check pending | Per module | Each |
| ARQ-107 | Layers 0 to 2 unchanged; ports exempt only from the `Locale` grep; negative control below | ARQ-107 step | `D1` |
| FD-31 (technique) | Resolve and fallback rules of P8 | `tests/format.rs` | `D5a` |
| FD-57, FD-58 (types) | `ExportScope`, `ExportIssue::blocks` | `tests/export.rs` | `D5b` |
| FD-64 | Trait in `gafira-ports` | `tests/assist.rs` | `D6` |
| V-5 | `LegalClass` has the 3 classes | `tests/media.rs` | `D1`, `D4` |

Tests name their ID in the test name or a comment. Where no ID applies, the name cites the rule, for example `p8_import_cancel_stops_trying`.

`D1` negative controls, run by `verify` on a scratch export: a plain row without the ARQ-107 change fails the step; `icu_plurals` added to `gafira-ports` fails it; `Locale` added to `gafira-raster` fails it.

## Delivery table

Lines are authored forecast, with x1.4 in brackets. Forecasts include tests. `AU1` and `RS1` ran about 3 times over their forecast, so if a delivery passes 700 lines the coordinator stops and asks; the split lines below are ready.

| Delivery | Scope | Specs covered | Depends on | Mode | Est. lines |
|---|---|---|---|---|---|
| `D1` | Crate, shared files, `ContentHash`, `registry`, `common`, `system`, `store`, module stubs | ARQ-01, ARQ-31, ARQ-50, ARQ-93, ARQ-107, V-5 (type) | None | serial | 300 (420) |
| `D2` | `i18n.rs`, `license.rs` | ARQ-94, ARQ-100, ARQ-102 | `D1` | parallel | 380 (530) |
| `D3` | `render.rs` | ARQ-06, ARQ-22, ARQ-102 | `D1` | parallel | 280 (390) |
| `D4` | `media.rs` | ARQ-77, V-5, ARQ-102 | `D1` | parallel | 380 (530) |
| `D5a` | `format.rs`: importers, `FormatRegistry` import side | FD-31, ARQ-102 | `D1` | parallel | 340 (475) |
| `D5b` | `export.rs`, exporter side of `FormatRegistry` | FD-57, FD-58, ARQ-102 | `D3`, `D4`, `D5a` | serial | 260 (365) |
| `D6` | `assets.rs`, `repository.rs`, `assist.rs` | ARQ-26, FD-64, ARQ-102 | `D1` | parallel | 280 (390) |

Total 2 220 (3 100). Largest: 380 (530). Split lines: `D2` into `i18n` and `license`; `D4` before `CodecRegistry`.

### Why each mark holds

| Mark | Ground |
|---|---|
| `D1` serial | It alone touches the root `Cargo.toml`, `Cargo.lock`, `ci.yml`, `lib.rs`, the crate `Cargo.toml` and `gafira-doc` |
| `D2`, `D3`, `D4`, `D5a`, `D6` parallel | Each writes only its own `src/<module>.rs` and `tests/<module>.rs`, and imports only `D1` modules and lower crates. `LegalClass` and `Confidence` sit in `common` for this reason |
| `D5b` serial | It edits `format.rs` from `D5a`, and `ExportError` wraps `RenderError` and `MediaError` |

### Shared files

| File | Handling |
|---|---|
| Root `Cargo.toml`, `Cargo.lock` | `D1` only. It adds one path entry. No new third-party crate: `serde` and `kurbo` are already workspace dependencies |
| `crates/gafira-ports/Cargo.toml` | `D1` declares every dependency and dev-dependency, unused or not. Unused dependencies do not fail `clippy` here (layer-2 design, XP6) |
| `crates/gafira-ports/src/lib.rs` | `D1` declares all 13 `pub mod`, no re-exports. Each module file starts as a `//!` line only, so no later delivery edits `lib.rs` |
| `.github/workflows/ci.yml` | `D1` only: the row and the ARQ-107 skip. V12 and the V5 step body do not change |
| `clippy.toml`, `deny.toml` | `clippy.toml` is a new copy. `deny.toml` does not change |
| Change folders | `apply-progress.md`, `verify-report.md` and `verification.json` have fixed names and would collide across worktrees. As layer-2 P2: each parallel delivery gets `specs/changes/gafira-ports-<delivery>/` for those files, and cites this design. `D1` and `D5b` use `specs/changes/gafira-ports/` |
| `.odd/state.json` | One outcome per parallel delivery, named like its folder, and union by `outcome_id` at merge (layer-2 P2) |
| `.cargo/mutants.toml` | T4 only, one crate at a time |

Merge order: `D1`, then first PASS first merged, then `D5b`. T4 runs after the last merge.

## Critical code and hand mutants

Every P8 row marked Crit is critical. Hand mutants below are rule-level; `tdd-apply` applies each, sees a test fail and reverts.

| Delivery | Critical | Hand mutants |
|---|---|---|
| `D1` | `Registry`, `system` blanket impls | (1) A duplicate id replaces the old item. (2) A duplicate id is rejected but the item stays in `items`. (3) `iter` goes in id order, not registration order. (4) `get` of an unknown id returns the first item. (5) The `DuplicateId` key loses its id argument |
| `D2` | `LanguagePreference`, `Locale`, `Region`, `LicenseStatus`, `LicenseKey` | (1) An unknown value reads as `Es`. (2) Parsing is case-insensitive, so `ES` is `Es`. (3) Serialized form is `System`, capitalised. (4) `Region` keeps lower case. (5) `Region` accepts 3 letters. (6) `can_export` is true for `Expired`. (7) `can_export` is false for `OfflineGrace`, breaking ARQ-92. (8) `LicenseKey` `Debug` prints the key |
| `D3` | `Deadline`, `RenderReport`, `to_message` | (1) `has_passed` is strict at `now == t`. (2) `Unbounded` reports passed. (3) `is_complete` ignores `left`. (4) `DeviceLost` and `OutOfMemory` share a key. (5) `Cancelled` gets its own key, not `error.job.cancelled` |
| `D4` | `best_decoder`, `AudioBuf::new` | (1) `Encumbered` is eligible. (2) An encode-only codec is eligible. (3) `RoyaltyFree` ranks above `hardware`. (4) A tie takes the last registered. (5) Another `CodecId` matches. (6) `AudioBuf` checks only the first two channels. (7) The error names the wrong channel |
| `D5a` | Signatures, `resolve_import`, `import` | (1) A short `head` panics. (2) Offset overflow panics. (3) Empty `bytes` matches. (4) Extension compare is case-sensitive. (5) A `None` sniff stays in the list. (6) No fallback to all importers. (7) Sort ascending. (8) Unstable tie order. (9) No seek between tries. (10) `Cancelled` tries the next importer. (11) The last error is returned, not the first. (12) Warnings of a failed try are kept |
| `D5b` | `contains`, `exporters_with`, `blocks`, duplicates | (1) `contains` tests only `transparency`. (2) `contains` is reversed, `required ⊇ self`. (3) `Warning` blocks. (4) `exporters_with` drops registration order. (5) Importer and exporter share one id list, rejecting `.gafira` |
| `D6` | `to_message`, `TextAssistantRegistry` | (1) `detail` leaks into an argument. (2) `TooNew` swaps `found` and `supported`. (3) The default registry is not empty |

T4 at close: `gafira-ports`, `gafira-doc`. Plain data has few mutants, so T4 is cheap on ports.

## Open questions (owner)

None open. The record:

| # | Question | Options and tradeoffs | Recommendation |
|---|---|---|---|
| O1 | D-12 removes the trial and lets people use the app unlicensed with export blocked; 02 §4.14 has `Trial` and `can_edit`, and ARQ-92, ARQ-93 and ARQ-95 follow it (open item 42). What does `LicenseStatus` expose while the list of blocked functions is pending? | (a) No `Trial`; `can_export` only. Applies D-12 exactly, and adds a predicate per blocked function later. (b) Keep `can_edit` with its 02 meaning: contradicts "se puede usar sin licencia". (c) Copy 02 as written: contradicts D-12, which prevails | Closed 2026-11-02: owner chose (a). `D2` builds it; `Unlicensed` stays a state |
| O2 | Accept ADR 0013 and ADR 0014? | Accepting makes the row, the testkit feature and the error rule binding. They stay `proposed` until then; `D1` can run on them | Closed 2026-11-02: owner chose (a), both accepted |

Owner decisions touched, none changed: D-02 (`best_decoder`), D-08 (`Locale`, §14.4 rule), D-12 (O1). O5 of the layer-2 design blocks `gafira-render-cpu`, not this crate.

## Needs explorer

| # | Claim | Check | Blocks |
|---|---|---|---|
| XP1 | Resolved, confirmed with limits, by `explorer` on a scratch workspace. (1) The default testkit tree has no ports or raster. (2) Workspace tests and the dev loop build, but a unit test in `gafira-raster/src` using a fake that returns raster types fails with E0308: two copies of `gafira_raster`. Integration tests in `crates/gafira-raster/tests/` and tests in other crates work. (3) `cargo mutants -p gafira-raster` does not build ports while raster neither enables the feature nor depends on ports. (4) The V5 cargo-tree check passes | Feature `ports = ["dep:gafira-ports", "dep:gafira-raster"]`. Rules in P7 | Nothing |
| XP2 | `VideoFrame` in RGBA `ImageBuf` meets CAL-16 and the 1 GB cache of 02 §6.5. At 1080p it costs 8 MB per frame | Measure with `gafira-media` | `gafira-media` design. Nothing here |

## Needs specifier

None blocks a delivery. `tdd-apply` builds to this design; 02 then follows.

| # | Where | Problem | Proposed wording |
|---|---|---|---|
| S1 | 02 §4, `Registry` | `HashMap` is banned in the crate | `by_id: BTreeMap<&'static str, usize>` |
| S2 | 02 §4.6, §6.1 | `&mut TileCache` names a type no crate can own | "`TileCache` es un trait de `gafira-ports`, implementado en `gafira-render`; `execute` recibe `&mut dyn TileCache`." Add `RenderPlan`, `RenderOp` and `FrameCx` to §4.6 |
| S3 | 02 §4.5 | No rule for a wrong or missing extension, a tie, or a cancel | "Si ningún importador con esa extensión reconoce la cabecera, se prueban todos. Empate: orden de registro. Se vuelve al byte 0 antes de cada intento. `Cancelled` detiene los intentos. Si todos fallan, se devuelve el primer error." |
| S4 | 02 §6.4, 05 FV-23 | `ExportJob<'_>` borrows, so it cannot be persisted | "Se persiste `ExportRequest`; `ExportJob` es la vista de una ejecución." Needs serde on `ParamValues`, which has none today |
| S5 | 02 §2.1, §4.5 | An `Exporter` in an adapter cannot reach `gafira-render` for pixels (ARQ-06) | Add `PageRenderer` to the ports row and to `ExportJob` |
| S6 | 02 §2.1 | The ports row lists `PluginHost` with no phase | "`PluginHost` llega con `gafira-plugin` (Fase 3)." Add `TileCache`, `PageRenderer` and `Demuxer`, `Muxer` to the row |
| S7 | 02 §7.3, ARQ-102 | "Un enum de error por crate" lists six enums for ports and omits `AssetError`, `LicenseError`, `AssistError`, `StoreError`, `RepositoryError`, `RegistryError` | ADR 0014 wording. Relates to open item 45 |
| S8 | 02 §11.1 vs §4.8, ARQ-77 | `MediaRegistry` against `CodecRegistry` | Keep `CodecRegistry`, add `Registry<dyn Container>`. Drop `ColorRegistry` until a consumer exists |
| S9 | 02 §4.8, 05 FV-02 | "Hardware, then free software" leaves out OS software codecs, such as AudioToolbox AAC | "Orden: hardware; códec del SO sin hardware; libre de regalías. `Encumbered` nunca." |
| S10 | 02 §4.14, §13.3, ARQ-92, ARQ-93, ARQ-95 | D-12 (item 42) | After O1: drop `Trial`, replace `can_edit` |
| S11 | 02 §14.4, §4.16 | Serde forms of `Locale` are unstated, and "valor desconocido" does not say whether a non-string counts | "`es`, `en`. Solo un texto desconocido se lee como `system`." State that `Region` is own, 2 letters or 3 digits |
| S12 | 02 §2.2, §4.14, §4.16, §5.3 | "Sin testkit → raster" against fakes in testkit | "Sin esa arista en el grafo por defecto. Los dobles de puertos están tras la feature `ports` de `gafira-testkit`." |
| S13 | 02 §2.1 raster row, §4.6 | "u8/f16/f32" and `f16/f32`, but `PixelFormat` has no `f16` | "f16 solo es interno de la GPU; `read_back` da `Rgba8` o `RgbaF32`." |
| S14 | ARQ-06 | The list omits `gafira-mux`, `gafira-platform`, `gafira-theme`; the §2.2 diagram draws adapters under the engine | Add them; use "todos los adaptadores salvo `gafira-ui`" |
| S15 | 02 §3.1, §5.3 | `Project` exists only in specs; the code has `Document`, and `AssetRef`, `AssetTable` do not exist | State that `ProjectRepository` works on `Document` until `Project` lands |
| S16 | ARQ-107 | The row binds layers 0 to 2, and the CI step reads every V5 row | Note that V5 also lists layer 3 crates, exempt from the `Locale` search |

## Next

`tdd-apply` for `D1`. `specifier` takes S1 to S16 in one pass; S10 follows O1 (a), and S12 adds the P7 rules.
