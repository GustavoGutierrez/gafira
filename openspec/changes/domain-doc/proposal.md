# Proposal: Domain Doc (C2b, the document model)

## Intent

`gafira-edit` (C3), the `.gafira` format, render and UI all need one pure, serializable, validated document model (02 §3). None exists today: `gafira-geom` and `gafira-color` give units, transforms and colors, but nothing to put them in. This change adds `gafira-doc` with the first product slice of `Document -> Page -> Node`. It makes three guarantees from day one: deterministic bytes (ARQ-10), unknown extensions that survive a load and save (ARQ-11), and invariants checked on load (ARQ-13). It also adds a compile-visible edit capability, so that only `gafira-edit` can mutate a document (part of ARQ-30).

## Change Chain

| Order | Change | Content |
|---|---|---|
| C1 | `workspace-scaffold` (merged) | Workspace, CI, `gafira-geom`, `gafira-cli --version` |
| C2a | `domain-color` (merged, including PR #5 `Color` + property tests, `main` 75f7e78) | `gafira-color`, table-driven V5 |
| C2b | `domain-doc` (this) | `gafira-doc` model, serde, `validate()`, `EditToken`. Absorbs the former "C2c doc invariants" |
| C3 | `edit-history` | `gafira-edit`, `Command`, `History`, `trybuild` for ARQ-30, `debug_assert` after each command |

C2b MUST NOT decide `Command`/`History` APIs. It only provides `Document::edit(&EditToken) -> DocMut` as the single mutation entry point that C3 builds on.

## Scope

### In Scope
- `NodeId(u64)`, `PageId(u64)`. One serialized `next_id` counter, unique across nodes and pages.
- Root `Document` with private fields: `schema_version: u32 = 1`, `next_id`, `dpi: Dpi` and `pages: Vec<Arc<Page>>`.
- `Dpi`: a `u32` newtype from 1 to 10 000, default 96. **Proposed decision (design A6, PROPOSAL)**; 02 does not fix a DPI range.
- `Page { id, name, size: PageSize, locked, hidden, roots: Vec<NodeId>, nodes }`. `PageSize { width: Length, height: Length }` must resolve to between 1 and 16 384 px at the document DPI (FD-01c). Example: A0 (841 x 1189 mm) at 300 DPI is 14 043 px and is accepted; A0 is accepted up to 350 DPI and rejected from 351 DPI.
- Arena `BTreeMap<NodeId, Arc<Node>>` (**PROPOSAL**, see amendments). It is serialized as an array sorted by id. A duplicate id inside one page is rejected while reading.
- Loading always validates; there is no unvalidated load path.
- `Node { base, kind, children }`. `ElementBase { id, name, placement, opacity, blend, visible, locked }` with bare values.
- `Placement { x, y, w, h, rotation_deg, skew, flip }` in `f64`, plus `to_transform() -> Affine`. `Affine` is imported from `gafira_geom`, never from `kurbo`.
- `NodeKind`, tagged by `type`: `Group`, `Shape` (`Rect` | `Ellipse`, with one optional solid `Color` fill) and `Extension { type_id, schema_version, payload: serde_json::Value }`.
- `BlendMode` (closed enum).
- Deterministic serde: `#[serde(default)]` on optional fields, no `deny_unknown_fields`, the serde `rc` feature for `Arc`, and canonical golden files for ARQ-11.
- `validate() -> Result<(), ValidationError>` (hand-written error). It checks:
  - ids are unique across nodes and pages, and `next_id > max(id)`;
  - children and roots exist;
  - only `Group` has children, every non-root node has one parent, and there are no cycles;
  - `PageSize` is within its limits, and values are finite and in range;
  - an `Extension` payload is nested no deeper than 64 levels (`ExtensionTooDeep`), because `serde_json` cannot reload more than about 121 levels.
- `EditToken`: not `Clone` or `Copy`, with a `#[doc(hidden)]` constructor. `Document::edit(&mut self, &EditToken) -> DocMut`.
- Tooling:
  - a V5 row `gafira-doc gafira-geom gafira-color`;
  - `HashMap`/`HashSet` added to `disallowed-types` in every `clippy.toml` (byte-identical copies, V12);
  - a CI step that fails if `EditToken` appears outside `gafira-doc`, `gafira-edit` and `gafira-testkit`.
- A bounded proptest document generator. Properties: the ARQ-10 round trip, generated documents validate, and corrupted documents fail.

### Out of Scope
- `Sequence`/`Track`/`Clip`, `AssetTable`, styles, identities, masks, effects, `timing`, text/image/frame nodes, `Path` (`BezPath` is not exposed by geom).
- `Animatable` (`gafira-anim`), `gafira-time`, `NodeBehavior`/`NodeVisitor`, the `msg` module, `grid`, `notes`, `bleed`, guides.
- `trybuild` (ARQ-30, C3), ARQ-12/14/15/32/33, the ARQ-13 fuzzer in `gafira-format`, and `xtask check-layers`.
- `gafira-cli validate`: deferred to the `gafira-format` change, because no file reader exists yet (pending holder confirmation, question 5).
- Any edit to `docs/`, and any change to `gafira-geom`/`gafira-color` code.

## Capabilities

### New Capabilities
- `document-model`: ids, the root `Document`, `Page`, `PageSize`, `Dpi`, `Node`, `ElementBase`, `NodeKind`, `Placement`, `BlendMode`, the arena.
- `document-serde`: the wire format, determinism (ARQ-10), forward compatibility, `Extension` round trip (ARQ-11).
- `document-validation`: `validate()`, the `ValidationError` taxonomy, ARQ-13 structural invariants.
- `edit-capability`: `EditToken`, `Document::edit`, `DocMut`, the rule on who may build a token.

### Modified Capabilities
- `workspace-tooling`: a V5 row for `gafira-doc`, the `HashMap`/`HashSet` ban in `clippy.toml`, and the `EditToken` CI step. The base spec lives in C1/C2a change folders until they are archived.

## Approach

Strict TDD. New dependencies: none from third parties. Normal dependencies are `serde` (+`rc`), `serde_json`, `gafira-geom` and `gafira-color`; `proptest` is dev-only. `Color` stays a bare value, so that a future untagged `Animatable::Static` stays compatible.

### Delivery plan
With the real size factor of about ×1.9, the forecast is about 4 517 measured lines (re-cut during apply). The work is a stacked chain of ten sub-deliveries (D1, D2a, D2b, D3a, D3b, D4a, D4b, D5, D6a, D6b; D1 also shipped as PR 1a and 1b), and every commit passes the gates on its own.

| # | Delivery | Forecast | Depends on |
|---|---|---|---|
| D1 | Crate skeleton, V5 row, `HashMap` ban, `NodeId`/`PageId`, `Dpi`, `PageSize` (+ px resolution), `BlendMode` | ~595 | `main` (75f7e78) |
| D2a | `Placement` + `to_transform`, `ElementBase` | ~335 | D1 |
| D2b | `NodeKind`, `Node`, `Page`, `Document`, arena, `next_id`, `#[cfg(test)]` `test_document` helper with its first test | ~465 | D2a |
| D3 | Wire format (sorted arena array), `rc`, defaults, `Extension`/`Value`, canonical goldens | ~690 | D2b |
| D4a | `ValidationError` (18 variants), passes P1–P3, parse-side depth boundaries | ~507 | D3b |
| D4b | pass P4 and `ExtensionTooDeep`, validating load (`try_from`, `LoadError::Invalid`), `from_parts` | ~387 | D4a |
| D5 | `validate()` passes P5–P6, `EditToken`, `edit`/`DocMut`, the `EditToken` CI step + negative runs | ~633 | D4b |
| D6a | Bounded generator, corruption generators, deterministic guards | ~540 | D5 |
| D6b | ARQ-10/13 properties (Q1–Q5), mutation runs, CI runtime record | ~365 | D6a |

Measure with `git diff --shortstat` after D2a. If the ratio is above ×1.3 of the forecast, re-cut the remaining deliveries before you continue.

### PROPOSAL deviations from 02 (design must confirm)
- §3.2: `transform: Animatable<Transform2D>` becomes a bare `Placement`, and `opacity` is a bare `f32`. `Placement` is decomposed and derives an `Affine`.
- §2.2: `gafira-doc` depends only on `geom` and `color` for now, because `anim` and `time` do not exist yet.
- §3.1: the arena is a `BTreeMap`, not a `HashMap`.
- §3.1: the root is `Document` with private fields, not `Project`, and pages are `Vec<Arc<Page>>`.
- `Dpi` is a `u32` from 1 to 10 000 (A6); 02 sets no range.

### Amendments to `docs/specs/02` (PROPOSAL, not applied here)
1. §3.1: `NodeArena` becomes a `BTreeMap<NodeId, Arc<Node>>`. The current `HashMap` contradicts ARQ-10 ("stable order"). Lookup becomes O(log n).
2. ARQ-11: the payload is preserved as a value, and payloads in canonical form are preserved byte for byte. `serde_json::Value` sorts keys and normalizes number spelling. The alternative is `Box<RawValue>` (the `raw_value` feature), which keeps the exact bytes but has no `PartialEq` and costs registry ergonomics. Design picks one.
3. ARQ-30: until C3, enforcement is an `EditToken` capability plus a CI search. `trybuild` comes in C3.

## Affected Areas

| Area | Impact |
|---|---|
| `crates/gafira-doc/**` (+ `clippy.toml`) | New |
| `crates/gafira-{geom,color}/clippy.toml` | Modified (`HashMap`/`HashSet` ban; no code change) |
| `Cargo.toml` (`gafira-doc` path only), `Cargo.lock` | Modified. serde `rc` is enabled in `crates/gafira-doc/Cargo.toml`, not in the root entry. |
| `.github/workflows/ci.yml` (V5 row, `EditToken` step) | Modified |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Size overrun (C2a was ×1.92) | High | Ten sub-deliveries of ≤ 700 lines each; measure after D2a; cut stacked branches at commit boundaries |
| The `EditToken` grep or V5 passes silently | Med | `set -euo pipefail`; a positive control (the definition must be found); manual negative runs per C2a N0–N7 |
| 10 000 proptest cases are slow on 3 OS | Med | Bounded node and page counts; CI case count in design; `PROPTEST_CASES` runs locally |
| `Value` normalization versus "byte for byte" in ARQ-11 | Med | Amendment 2; goldens written in canonical form |
| Deep `Extension` payloads that save but cannot be reloaded | Med | `ExtensionTooDeep` at 64 levels, below the ~121 reload limit of `serde_json` |
| The serde `rc` feature unifies across the workspace | Low | Design A3: enabled in the crate manifest only; the root `[workspace.dependencies]` entry is unchanged |

## Rollback Plan

Revert the change's commits per delivery, newest first. In full: delete `crates/gafira-doc/`, the root dependency line and its `Cargo.lock` lines, the V5 row and the `EditToken` step. Restoring the `clippy.toml` copies is optional, because the ban is harmless. No persisted user data exists yet.

## Success Criteria

- [ ] `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` and `cargo deny check` pass after every delivery.
- [ ] CI is green on Windows, Linux and macOS (ARQ-03).
- [ ] `cargo tree -p gafira-doc -e normal,build` has `gafira-geom` and `gafira-color` as its only `gafira-*` crates and no forbidden prefix. V5 fails on a forced `gafira-doc -> gafira-cli` edge.
- [ ] The `EditToken` step fails on a forced use outside the allowed crates.
- [ ] `serialize(deserialize(serialize(d))) == serialize(d)` holds for generated documents, and the canonical `Extension` golden survives a round trip with identical bytes.
- [ ] `validate()` rejects each of these with a distinct error:
  - a duplicate id;
  - a dangling child;
  - a cycle;
  - a second parent;
  - children on a `Shape`;
  - a `PageSize` outside 1 to 16 384 px (A0 accepted at 350 DPI, rejected at 351 DPI);
  - `next_id <= max(id)`;
  - an `Extension` payload deeper than 64 levels (`ExtensionTooDeep`).
- [ ] Reading a page with a duplicate node id fails, and every load runs `validate()`.

## Proposal question round

The questions were not asked interactively, because the executor cannot prompt. Each one carries the default that design and specs adopted; the holder confirms or corrects:

1. **Page limits:** default: 1 to 16 384 px measured at the document DPI (one DPI per document). A0 is accepted up to 350 DPI.
2. **Out-of-range data:** default: reject (opacity outside [0,1], non-finite values, `Dpi` outside 1 to 10 000); no clamping on load.
3. **Unknown extensions:** default: identical bytes only for canonical payloads; other payloads keep their meaning (`Value`, not `RawValue`).
4. **Root name:** default: `Document` (02 says `Project`). Still open: does a lock on a page cascade to its nodes in the data model, or only in the UI?
5. **CLI:** default: `gafira-cli validate` (promised for C2b in C1/C2a) is deferred to the `gafira-format` change. Pending confirmation.

Assumptions to review: the document is rejected on the first invariant failure (not with a list of errors); `Shape` has a fill only (no stroke); `Group` is the only kind of node with children.
