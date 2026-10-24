# Design: Domain Doc (C2b, the document model)

## Technical Approach

Add `crates/gafira-doc`, a layer-1 domain crate that is pure, serializable and validated. Normal dependencies: `gafira-geom`, `gafira-color`, `serde` (+ `rc`, declared in this crate only) and `serde_json` (for `Extension` payloads). Dev-only: `proptest`. No new third-party crate enters `Cargo.lock`.

The crate follows the C1/C2a patterns:
- `#![forbid(unsafe_code)]` (V11);
- a `clippy.toml` byte-identical to the canonical copy (V12), which now also bans `HashMap`/`HashSet`;
- constructors that do not validate (C1 D8); invariants live in one place, `Document::validate()`;
- IEEE `PartialEq` on floats, bit comparison (`to_bits()`) in tests;
- hand-written errors (C1 D12), `#[non_exhaustive]`;
- exhaustive enums for closed vocabularies (C2a D7).

Shape of the model (02 §3, first product slice):

    Document { schema_version, next_id, dpi, pages: Vec<Arc<Page>> }   (fields private)
      └─ Page { id, name, size, locked, hidden, roots: Vec<NodeId>, nodes: BTreeMap<NodeId, Arc<Node>> }
           └─ Node { base: ElementBase, kind: NodeKind, children: Vec<NodeId> }
                ElementBase { id, name, placement: Placement, opacity: f32, blend, visible, locked }
                NodeKind = Group(GroupNode) | Shape(ShapeNode) | Extension(ExtensionNode)

Mutation of a `Document` in place needs an `EditToken` (`Document::edit(&mut self, &EditToken) -> DocMut`). Only `gafira-edit` (C3), `gafira-testkit` (future) and `gafira-doc`'s own tests may name the token; a CI step with a positive control enforces that until `trybuild` arrives in C3.

**Specs**: `specs/*/spec.md` and this design are aligned on the same decisions (see "Spec Alignment").

## Architecture Decisions

| # | Decision | Choice | Rejected | Rationale |
|---|---|---|---|---|
| A1 | Node arena | `BTreeMap<NodeId, Arc<Node>>` per page. On the wire the arena is a JSON **array** of nodes in ascending id order (not a JSON object keyed by id). Deserialization fails on a duplicate id inside one array. In memory the key must equal `node.base.id` (`validate` checks it). | `HashMap` (02 §3.1): the iteration order varies per process, which breaks ARQ-10, and the type is now banned. `imbl`/`im`/`rpds`: MPL-2.0, already in `deny.toml [bans]` (ADR-03). `Vec<Node>` indexed by position: ids are unstable on delete, and lookup is O(n) or needs a remap. `IndexMap`: a new dependency, and the bytes would depend on edit history. A JSON object keyed by `"id"`: serde silently keeps the last of two duplicate keys, so a duplicate could never be detected. | Gives stable order, a lookup in O(log n), and `Arc` sharing for ADR-03 (`Arc::make_mut` along the edited path). The array form lets a duplicate be detected, and stores no redundant key. Cost: cloning a `Page` costs O(n) `Arc` bumps; ARQ-32 is measured in C3 (risk R5). Amendment 1 to 02 §3.1 is confirmed. |
| A2 | `pages` | `Vec<Arc<Page>>` | `Vec<Page>` | ADR-03 names `Arc<Page>`. Cloning a document for a history snapshot then costs O(pages). Changing this after C3 consumes it would break the API, so it is fixed now. |
| A3 | serde `rc` | `serde = { workspace = true, features = ["rc"] }` in `crates/gafira-doc/Cargo.toml` only. The root `[workspace.dependencies]` entry is unchanged. | `rc` in `[workspace.dependencies]`; `serialize_with`/`deserialize_with` helpers for `Arc` (~25 lines); no `Arc` in the model | The feature is additive: it only adds `Serialize`/`Deserialize` impls for `Rc`/`Arc`. Declaring it where it is needed documents who needs it. `cargo build -p gafira-geom` alone does not get it, but a workspace build unifies features (resolver 3), which is harmless. Deserializing makes fresh `Arc`s with no deduplication, which is correct because every node appears once. |
| A4 | `Extension` payload | `serde_json::Value`. **Canonical payload** means exactly the output of `serde_json::to_string(&Value)`: keys sorted, no whitespace, shortest round-trip numbers, serde_json escaping. ARQ-11 holds **byte for byte for canonical payloads**, and **by value** for any other payload. | `Box<RawValue>` (`raw_value` feature) | (1) `NodeKind` is internally tagged (`tag = "type"`). serde buffers internally tagged content into its private `Content` type, and `RawValue` cannot be deserialized from it: making `RawValue` work would need either dropping internal tagging or a hand-written `Deserialize` (~80 lines). (2) `RawValue` has no `PartialEq`, so `Node` and `Document` would lose derived equality, which C3 undo properties need. (3) `Value` is easy to generate with proptest. Accepted costs (amendment 2): a non-canonical third-party payload is normalized; with duplicate keys the last one wins; integers outside the i64/u64 range become f64, which loses precision. Guard tests (T3.7) fail if `preserve_order` or `arbitrary_precision` is ever unified in: the first breaks key order (ARQ-10), and the second breaks `f64` fields inside internally tagged enums. **Depth**: serde_json serializes any depth but parses at most 128 levels in total, so a whole document stops reloading once a payload has 122 or more levels. `validate()` therefore rejects payloads deeper than `MAX_EXTENSION_DEPTH = 64` (`ExtensionTooDeep`). This keeps a wide margin, and guarantees that every document that passes `validate()` reloads. |
| A5 | Ids | `NodeId(u64)` and `PageId(u64)`, both `#[serde(transparent)]`. One serialized counter, `next_id`, is shared by nodes and pages. An id is allocated only through `DocMut::alloc_node_id`/`alloc_page_id`, which return the current `next_id` and increment it. Until D5 the allocator behind them, `Document::alloc_id` (`pub(crate)`), is gated `#[cfg(test)]` (see "Dead code before its consumer"). Allocation fails with `IdSpaceExhausted` when `next_id == u64::MAX`. `Document::default()` has `next_id = 1`. Ids are never derived from content. | Recomputing `max + 1` on load: deleting the highest node and reloading would reuse its id, which external references (history, extension payloads, future `ClipSource::Nodes`) could still point at. UUIDs or random ids: need an RNG (ARQ-02), are not deterministic (ARQ-10), and are 128-bit. One counter per kind or per page: "unique across nodes and pages" would then need cross-checks. | With one counter, uniqueness is one set and one rule: `next_id > every id`. An id equal to `u64::MAX` can never be valid, which is consistent with the exhaustion rule. |
| A6 | `Dpi` and page limits | `Dpi(u32)`, in pixels per inch, stored **once per document**. Default 96. Valid range **1..=10 000** (PROPOSAL). Wire format: a bare integer, `96`. `PageSize { width: Length, height: Length }` resolves through `gafira_geom::Length::to_px(f64::from(dpi.get()))`; this crate has no unit math of its own. **Each axis**, in **unrounded** px at the **document DPI**, must lie in `[1.0, 16 384.0]` (inclusive). Example: A0 portrait (841×1189 mm) is accepted up to 350 DPI (14 043 px at 300 DPI, 16 383.86 px at 350) and rejected from 351 DPI (16 430.7 px). | `Dpi(f64)`: no `Eq`, and it has `-0.0`/NaN cases, while no fractional DPI is needed by FD-01. Raw `f64`: C1 D7 deferred the newtype to C2. Limits at a fixed 96 DPI: contradicts FD-01, where the base unit is the logical pixel at the project DPI, and its rationale (GPU texture size, export tiles). Per-page DPI: FD-01 says "per project"; it can be added later. Rounded px: the rounding mode belongs to export (C1: no rounding in the domain). | `u32` gives exact bytes, `Eq`/`Ord`, and a lossless conversion to `f64`. The A0 boundary gives a sharp fixture. The proposal's example ("A0 at 300 DPI is rejected") is wrong and is corrected here. Holder question Q1. |
| A7 | Root name | `Document` | `Project` (02 §3.1) | Every API that C3 consumes uses `Document`: `Command::apply(&mut Document)` (§4.3), `Arc<Document>` snapshots (ADR-03, §6.1) and ARQ-30. In 02, `Project` names the file- and session-level concept (`ProjectMeta` with clock timestamps, `ProjectService`, `project_id`), which belongs to `gafira-format`/`gafira-engine`. Amendment 4 (PROPOSAL): 02 §3.1 shows `Document` as the in-memory aggregate. |
| A8 | Bare values | `opacity: f32`, `placement: Placement` and `fill: Option<Color>` are stored as bare values. **Rule for any type that may later become `Animatable<T>`**: no container-level `#[serde(default)]`, and at least one field must be required. `Placement` requires `x, y, w, h`; `Color` requires `space`. | `Animatable<T>` now (`gafira-anim` does not exist); a container-level default on `Placement` | `Animatable::Static` is untagged and comes first. A bare value written today (`0.5`, `{"x":..}`) is byte-identical to a future `Static`. A keyed track `{"keys":[...]}` must **fail** as `T` so that the untagged enum falls through to `Keyed`: a container default would accept it as a default `Placement` and lose the animation. Test T3.4 pins this rule. Deviation from 02 §3.2 confirmed. |
| A9 | Out-of-range data | **Reject** in `validate()`, never clamp. Rules: every `f64`/`f32` is finite; `w, h >= 0`; `-90 < skew < 90` (degrees); `rotation_deg` is any finite value, not normalized; `opacity` is in `[0, 1]`; a fill must have finite channels and alpha in `[0, 1]`, while RGB outside `[0, 1]` stays legal (extended range, C2a decision 4); an `Extension.type_id` must not be empty. `-0.0` is accepted wherever `0.0` is. | Clamping on load | Clamping changes user data silently, breaks "load then save gives the same bytes" (ARQ-10), and hides the bug that produced the value. A repair mode that is explicit and visible to the user can come later in `gafira-format`. Holder question Q2. |
| A10 | `ValidationError` | Hand-written, `#[non_exhaustive]`, `Clone + Debug + PartialEq` (no `Eq`, because variants carry `f64`), `Display + Error`. There are **18 variants**. `validate()` returns the **first** error in a fixed, documented pass order (see "Validation passes"). The result is deterministic. | `thiserror` (C1 D12); returning all errors (`Vec<ValidationError>`) | One broken id cascades into many derived errors, and nothing consumes a list yet. A `validate_all` can be added later without breaking anything. The fixed order makes property tests exact: each mutation maps to one expected variant. `ToMessage` (ARQ-102) arrives with the `msg` module. |
| A11 | Edit capability | `pub struct EditToken { _private: () }`. Its only constructor is `#[doc(hidden)] pub const fn __new_for_gafira_edit_only() -> EditToken`. No `Clone`, `Copy`, `Default` or `Debug`. `Document::edit(&mut self, _: &EditToken) -> DocMut<'_>` is the only way to get `&mut` access to anything inside a `Document`. `Document` fields are private; the value types below it (`Page`, `Node`, …) have public fields. A CI step (V13) rejects the identifier or the constructor name outside the allowed crates. | Struct literal (blocked by the private field); a cargo feature gating the constructor (features unify across the workspace, so `gafira-ui` would get it as soon as `gafira-edit` enables it); a sealed trait (any crate can implement a public trait whose bound it can name); doing nothing until `trybuild` (C3) | See "EditToken: what Rust guarantees and what it does not". |
| A12 | Children | `children` stays on `Node` (02 §3.2 shape), but only `NodeKind::Group` may have a non-empty list. Every non-root node has exactly one parent, inside the same page. Roots have no parent. There are no cycles and no edges across pages. | `GroupNode { children }` (the type itself would forbid children on a leaf, but it changes 02's `Node` shape and makes every traversal match on the kind); a DAG with several parents (instances or symbols are out of scope) | A uniform traversal for the future `walk`/Visitor. `validate` enforces the rule. `Extension` nodes cannot have children in C2b; a container extension needs a later decision. |
| A13 | Load path | `Document` implements `Deserialize` through `#[serde(try_from = "DocumentWire")]`, and the conversion runs `validate()`, so **no invalid `Document` can come out of deserialization**. `Document::from_json(&[u8]) -> Result<Document, LoadError>` keeps the typed `ValidationError`. `to_json(&self)` writes compact JSON, which is the canonical form. | A plain derive, with validation left to the caller | ARQ-13: "validated after each load" is enforced by the type system rather than by convention. D3 lands `from = "DocumentWire"` (unchecked). D4 switches to `try_from`, which is additive because `LoadError` is `#[non_exhaustive]`. |
| A14 | Wire vocabulary | `NodeKind` tags in snake_case (`group`, `shape`, `extension`), as in 02 §3.2. `ShapeGeometry`: `rect`, `ellipse`. `BlendMode`: the 16 W3C Compositing modes in CSS kebab-case (`normal`, `multiply`, `screen`, `overlay`, `darken`, `lighten`, `color-dodge`, `color-burn`, `hard-light`, `soft-light`, `difference`, `exclusion`, `hue`, `saturation`, `color`, `luminosity`), exhaustive. An unknown value is an error. | snake_case for blend modes; `#[serde(other)]` fallbacks | Same reasoning as C2a D8: SVG export can emit the CSS name directly. A new core kind or mode arrives with a `schema_version` bump and a migration (`gafira-format`). |
| A15 | `gafira-cli validate` | **Deferred** to the `gafira-format` change (Q5). | D7 in this change (~250 measured lines: `lexopt` or hand-written argv parsing, reading a file, mapping exit codes) | No `.gafira` reader exists yet. A raw `document.json` command would freeze a file contract that 02 §7.1 does not have (the ZIP holds one file per page). Consequence: the C1/C2a chain tables that promised `validate` for C2 are out of date. This design records the move; the holder confirms it. C1 D13 (parser choice) moves too. |
| A16 | Page lock | No cascade in the data model. `Page::locked` and `ElementBase::locked` are independent flags. The effective lock is computed by edit/UI (C3+). | Cascading in the model | A cascade is policy, not data. Storing derived state would duplicate it. Q4. |
| A17 | `Placement` → `Affine` | Units are logical px in page space. The node's local box is `[0,w]×[0,h]`. `to_transform()` = `translate(x + w/2, y + h/2) · rotate(rad(rotation_deg)) · skew(tan(rad(skew)), 0) · scale(fx, fy) · translate(−w/2, −h/2)`, written with kurbo's right-to-left `*` on `gafira_geom::Affine`, where `fx = −1` if `flip.x` else `1` (same for `y`). Positive rotation turns +x toward +y, which is clockwise on a y-down screen (as in CSS). Rotation, skew and flip pivot on the box center. `w`/`h` size the box and do not scale it. | Transform2D storage (C1 D9 assigned the decomposed form to C2 as a new type); pivot at the top-left | Center pivot is the convention of editors and of 02 ("rotation_deg"). `Affine` comes only from `gafira_geom` (never from `kurbo` directly). |

### EditToken: what Rust guarantees and what it does not

| Guaranteed by the compiler | NOT guaranteed (and how it is covered) |
|---|---|
| Calling `Document::edit` requires a value of `EditToken`. | `#[doc(hidden)]` hides the constructor from the docs, but it stays callable by any crate. **V13 CI step** rejects the name outside the allowed crates. |
| No crate outside `gafira-doc` can build a token with a struct literal (private field), `Default`, `Clone` or `Copy`. | A holder can lend `&EditToken` anywhere, and type inference lets a callee use it without ever writing the name. V13 also rejects a `pub use … EditToken` re-export in `gafira-edit`. A public `gafira-edit` function that returns `&EditToken` is caught only by review until C3's `trybuild`. |
| With `&Document` or `Arc<Document>` there is no field-level `&mut`. | Whole-value replacement: whoever owns a `Document`, or gets one through `Arc::make_mut`, can write `*doc = other` (built through deserialization or `from_parts`). This does not mutate the session document, which C3's `EditService` owns. It is documented, not prevented. |
| `unsafe` cannot bypass this in any workspace crate (`forbid`, V11). | Building a new valid document from JSON is allowed by design; it is construction, not mutation. |

**`compile_fail` doctests are not evidence for ARQ-30.** On stable, rustdoc does not check the error code in a `compile_fail,E0599` block: a doctest that failed with a different error still passed (validator run). The codes are decorative. Such doctests MAY document intent, but they are never cited as proof. The evidence in C2b is V13 plus the guarantee table above plus review; `trybuild` (C3) is the real compile test.

### Dead code before its consumer

The C2a precedent applies: an item that only tests use until a later commit is gated `#[cfg(test)]`, and the gate is removed in the commit that adds its first non-test consumer. `#[allow(dead_code)]` is never used.

| Item | Gated in | Gate removed in |
|---|---|---|
| `Document::alloc_id` (`pub(crate)`, increments `next_id`) | D2b, commit `add Document root` | D5, commit `add EditToken and DocMut` (`DocMut::alloc_*` consumes it) |
| `test_document(...)` helper (`#[cfg(test)] pub(crate)`, builds a `Document` from parts without validation) | D2b, commit `add Document root`, **together with its first test** ("Clone shares everything") | Never: it is permanently test-only |

The helper is never added in a commit before the test that calls it: an uncalled `#[cfg(test)]` item is still dead code under `clippy --all-targets -D warnings`. D3 reuses it.

`next_id` itself is not dead: the public `next_id()` accessor reads it. `IdSpaceExhausted` is public (`ids.rs`, D2b), so it is never dead code.

## Data Flow

    bytes ──from_json──► DocumentWire ──TryFrom (validate)──► Document ──to_json──► canonical bytes
                              ▲                                   │  &self accessors (read-only)
    serde_json::from_* ───────┘ (Deserialize = same path)         │
                                                                  └─edit(&EditToken)─► DocMut ──► &mut Page / &mut Node (Arc::make_mut)
    PageSize ──Length::to_px(dpi)──► px per axis ──[1, 16 384]──► ok | ValidationError::PageSize
    Placement ──to_transform()──► gafira_geom::Affine

## File Changes

| File | Delivery | Action | Content |
|---|---|---|---|
| `crates/gafira-geom/clippy.toml` | D1 | Modify | +2 entries in `disallowed-types` (below). Canonical copy. |
| `crates/gafira-color/clippy.toml` | D1 | Modify | Same bytes as geom (copied with `cp`, never edited by hand) |
| `crates/gafira-doc/clippy.toml` | D1 | Create | `cp crates/gafira-geom/clippy.toml` |
| `crates/gafira-doc/Cargo.toml` | D1 | Create | The same `[package]` block as color; `[dependencies]` `gafira-geom.workspace`, `gafira-color.workspace`, `serde = { workspace = true, features = ["rc"] }`, `serde_json.workspace`; `[dev-dependencies]` `proptest.workspace`; `[lints] workspace = true` |
| `Cargo.toml` | D1 | Modify | +1 line `gafira-doc = { path = "crates/gafira-doc" }`. `members = ["crates/*", "apps/*"]` already covers the crate. |
| `Cargo.lock` | D1 | Modify | Only the `gafira-doc` package entry (serde_json is already locked as a dev dependency). Committed together with the skeleton. |
| `.github/workflows/ci.yml` | D1, D5 | Modify | D1 commit 2: V5 row and V12 hardening (both together with the `gafira-doc` copy). D5: new V13 `EditToken` step. |
| `crates/gafira-doc/src/lib.rs` | D1+ | Create | Crate docs, `#![forbid(unsafe_code)]`, modules, re-exports |
| `src/ids.rs`, `src/dpi.rs`, `src/page_size.rs`, `src/blend.rs` | D1 | Create | Types + unit tests. `Axis` and `PageSizeError` live in `page_size.rs` (D1). |
| `src/placement.rs`, `src/element.rs` | D2a | Create | `Placement`, `Flip`, `to_transform`, `ElementBase` + unit tests |
| `src/node.rs`, `src/page.rs`, `src/document.rs`; `IdSpaceExhausted` in `src/ids.rs` | D2b | Create / Modify | Node kinds, `Page`, arena, `Document`, read accessors, `#[cfg(test)]` id allocation, the `#[cfg(test)]` helper `test_document` with its first test ("Clone shares everything") + unit tests |
| `src/arena.rs`, `src/json.rs` | D3 | Create | Arena wire as array + duplicate rejection; `DocumentWire`, `from_json`, `to_json`, `LoadError`; in-crate unit tests that build documents with `test_document` (defined in D2b) |
| `tests/serde.rs`, `tests/golden.rs` | D3 | Create | Black-box tests on JSON fixtures only, loaded through `Deserialize` without validation in D3: exact JSON per type, defaults, hostile inputs, serde_json feature guards. Goldens via `include_str!` (`std::fs` is banned in tests too). **Fixture rule**: every fixture is a valid document, so it keeps passing after D4 switches loading to `try_from` and after D5 adds P5–P6. In particular, every node in a fixture's arena is listed in `roots` or in exactly one group's `children` (otherwise `Orphan` would fire from D5), and no fixture has a cycle or a second parent. This includes the defaults and normalization fixtures ("nodes 7 then 2" lists both in `roots`). |
| `tests/golden/{minimal,full,extension,extension_noncanonical}.json` | D3 | Create | Compact, one line, no trailing newline |
| `tests/golden/.gitattributes` | D3 | Create | `*.json -text` (no CRLF conversion on Windows checkouts) |
| `src/error.rs`, `src/validate.rs` | D4 (P1–P4), D5 (P5–P6) | Create / Modify | `ValidationError` (all 18 variants in D4), `NodeField`; passes + table-driven tests. `json.rs` switches to `try_from` in D4; `from_parts` lands in D4. |
| `src/edit.rs`, `tests/edit.rs` | D5 | Create | `EditToken`, `DocMut` (removes the `alloc_id` gate); tests for "Changing the DPI" and "Edit touches one path" |
| `tests/common/mod.rs`, `tests/properties.rs` | D6 | Create | Bounded generator, corruptions, properties Q1–Q5, generator guards |
| `crates/gafira-doc/proptest-regressions/` | D6 | Create if produced | Committed |

### `clippy.toml` (all three copies, byte-identical)

Appended to `disallowed-types`:

    { path = "std::collections::HashMap", reason = "ARQ-10: unstable iteration order; use BTreeMap" },
    { path = "std::collections::HashSet", reason = "ARQ-10: unstable iteration order; use BTreeSet" },

`gafira-geom` and `gafira-color` must take the same lines because V12 requires identical copies. The ban is also correct policy for every domain crate: neither crate uses either type today (checked with grep), and the ban applies to tests as well (`--all-targets`). The `HashSet` ban means `validate` uses `BTreeSet`, which is O(n log n) and acceptable.

### `ci.yml`

**V5**: add one table row; the body is unchanged.

    gafira-doc   gafira-geom gafira-color

**V12, why it changes**: until now V12 only kept the copies consistent. From this change the *content* of the canonical copy matters: the `HashMap` ban protects ARQ-10. The old step would pass silently on any of these:
- the ban removed consistently from all three copies;
- the ban line commented out;
- the ban moved under `disallowed-methods`, where clippy ignores a type;
- a glob that matches nothing.

The hardened body lands in **D1 commit 2**, together with the `gafira-doc` copy, because `n >= 3` cannot hold while only geom and color exist. In D1 commit 1 V12 keeps its current form. New body:

    set -euo pipefail
    canon=crates/gafira-geom/clippy.toml
    # Only the disallowed-types array counts: from its key line to its closing bracket.
    types=$(awk '/^disallowed-types[[:space:]]*=/{on=1} on{print} on && /^\]/{exit}' "$canon")
    [ -n "$types" ] || { echo "::error::$canon has no disallowed-types array" >&2; exit 1; }
    for t in HashMap HashSet; do
      grep -qE "^[[:space:]]*\{ path = \"std::collections::$t\"" <<<"$types" \
        || { echo "::error::$canon lacks the std::collections::$t ban under disallowed-types" >&2; exit 1; }
    done
    n=0
    for c in crates/*/clippy.toml; do
      n=$((n + 1))
      [ "$c" = "$canon" ] || diff -q "$canon" "$c"
    done
    [ "$n" -ge 3 ] || { echo "::error::expected >= 3 domain clippy.toml copies, found $n" >&2; exit 1; }

The anchor `^[[:space:]]*\{` rejects a commented line (`# { path = …`). Extracting the array first rejects a ban placed under `disallowed-methods`. If `awk` fails or the file is missing, the substitution fails and `set -e` stops the step.

**V13 (new, D5)**: `EditToken` only in allowed crates.

    set -euo pipefail
    allowed='^crates/(gafira-doc|gafira-edit|gafira-testkit)/'
    rc=0; hits=$(git grep --untracked -nE 'EditToken|__new_for_gafira_edit_only' -- '*.rs') || rc=$?
    [ "$rc" -le 1 ] || { echo "::error::git grep failed (rc=$rc)" >&2; exit 1; }
    # Positive control: the repo-wide search must see the definition, otherwise it searched the wrong place.
    grep -qE '^crates/gafira-doc/src/[a-z_/]+\.rs:[0-9]+:pub struct EditToken \{' <<<"$hits" \
      || { echo "::error::positive control failed: EditToken definition not found" >&2; exit 1; }
    rc=0; bad=$(grep -vE "$allowed" <<<"$hits") || rc=$?
    case "$rc" in
      0) echo "::error::EditToken outside allowed crates:" >&2; echo "$bad" >&2; exit 1 ;;
      1) ;;
      *) echo "::error::filter grep failed (rc=$rc)" >&2; exit 1 ;;
    esac
    # Public re-exports from gafira-edit. Records are split on ';', so a multi-line
    # `pub use gafira_doc::{\n EditToken,\n};` is one record. A plain directory pathspec is silent when the dir does not exist
    # (a glob pathspec is not: git 2.34 prints "could not open directory" to stderr). `|| true`: grep exits 1 on no match.
    files=$(git ls-files --cached --others --exclude-standard -- crates/gafira-edit | grep '\.rs$' || true)
    status=0
    while IFS= read -r f; do
      [ -n "$f" ] || continue
      out=$(awk 'BEGIN{RS=";"} /pub[[:space:]]+(use|type)[^;]*EditToken/ || /pub[[:space:]]+use[^;]*\*/ {print FILENAME ": " $0}' "$f")
      [ -z "$out" ] || { echo "::error::gafira-edit must not publicly re-export EditToken:" >&2; echo "$out" >&2; status=1; }
    done <<<"$files"
    exit "$status"

Every `grep` exit code is captured explicitly. Exit 1 ("no match") is the only status read as "clean", so any exit above 1 fails the step (`git grep` exits 128 for a broken pattern or a missing repository; the filter `grep` exits 2 for a broken pattern). The positive control also fails if the step runs from a subdirectory, or if the definition is renamed. `\b` is not used, because ERE word boundaries are not portable; matching `EditTokenX` as well is stricter, which is acceptable. Only `*.rs` files are searched, so `openspec/` and `ci.yml` itself never match.

The re-export check catches, in `gafira-edit`:
- `pub use` and `pub type` items that name `EditToken`, including multi-line brace groups;
- every public glob re-export (`pub use …::*`), because a glob would re-export the token without naming it. Banning all public globs in `gafira-edit` is stricter than needed, and cheap.

`pub(crate)` re-exports are deliberately allowed. A failing `awk` stops the step (`set -e` on the substitution). Using `git ls-files` with a plain directory pathspec (verified silent on git 2.34.1 when the directory is missing; a glob pathspec such as `'crates/gafira-edit/*.rs'` is not silent: it prints `warning: could not open directory` to stderr, exit 0) filtered through `grep '\.rs$' || true` avoids that warning and does not depend on it: `|| true` is needed because `grep` exits 1 on no match and the step runs under `pipefail`. A warning on stderr would never change the exit code or the positive control.

**Known limits of V13** (recorded, not met; `trybuild` and review cover them in C3):
1. A two-step alias: `use gafira_doc::EditToken as T;` followed by `pub use self::T;`. The public line does not name the token.
2. A public function or field that hands out `EditToken`/`&EditToken`, or a public wrapper type around it. In `gafira-edit` that is an allowed mention, not a re-export.
3. A re-export generated by a macro.
4. A `;` inside a comment or string literal splits a record, which can hide a match.
5. Type inference in a consumer that never names the token.
6. A re-export of the whole crate or of the module that contains the token, which never names `EditToken`: `pub use gafira_doc as d;` and `pub use gafira_doc::edit;` (the glob check only sees `*`).

## Interfaces / Contracts (illustrative signatures)

```rust
// ids.rs — #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)] #[serde(transparent)]
pub struct NodeId(u64);  impl NodeId { pub const fn new(v: u64) -> Self; pub const fn get(self) -> u64; }
pub struct PageId(u64);  impl PageId { pub const fn new(v: u64) -> Self; pub const fn get(self) -> u64; }
// Display for both: the bare number.
pub struct IdSpaceExhausted;                                        // D2b; Copy, Eq, Display, Error

// dpi.rs — Copy, Eq, Ord, Hash, serde(transparent)
pub struct Dpi(u32);
impl Dpi { pub const DEFAULT: Dpi /* 96 */; pub const MIN: u32 = 1; pub const MAX: u32 = 10_000;
           pub const fn new(v: u32) -> Self /* unvalidated */; pub const fn get(self) -> u32;
           pub const fn is_valid(self) -> bool; }
impl Default for Dpi { /* 96 */ }

// page_size.rs — Clone, Copy, Debug, PartialEq, Serialize, Deserialize
pub struct PageSize { pub width: Length, pub height: Length }
impl PageSize { pub const MIN_PX: f64 = 1.0; pub const MAX_PX: f64 = 16_384.0;
                pub fn to_px(self, dpi: Dpi) -> Result<gafira_geom::Size, ConversionError>;
                pub fn check(self, dpi: Dpi) -> Result<(), PageSizeError>; }
pub enum Axis { Width, Height }                                     // Copy, Eq
#[non_exhaustive] pub enum PageSizeError { Conversion { axis: Axis, error: ConversionError },
                                           OutOfRange { axis: Axis, px: f64 } }   // Display + Error

// blend.rs — Copy, Eq, Hash, Default (= Normal), serde(rename_all = "kebab-case"), exhaustive
pub enum BlendMode { Normal, Multiply, Screen, Overlay, Darken, Lighten, ColorDodge, ColorBurn,
                     HardLight, SoftLight, Difference, Exclusion, Hue, Saturation, Color, Luminosity }
impl BlendMode { pub const ALL: [BlendMode; 16]; }

// placement.rs — Clone, Copy, Debug, PartialEq, Serialize, Deserialize (no container default)
pub struct Placement { pub x: f64, pub y: f64, pub w: f64, pub h: f64,
                       #[serde(default)] pub rotation_deg: f64, #[serde(default)] pub skew: f64,
                       #[serde(default)] pub flip: Flip }
pub struct Flip { pub x: bool, pub y: bool }                         // Copy, Eq, Default
impl Placement { pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self;
                 pub fn to_transform(self) -> gafira_geom::Affine; }

// element.rs — Clone, Debug, PartialEq
pub struct ElementBase { pub id: NodeId, pub name: Option<String>, pub placement: Placement,
                         pub opacity: f32 /* default 1.0 */, pub blend: BlendMode,
                         pub visible: bool /* default true */, pub locked: bool }
impl ElementBase { pub fn new(id: NodeId, placement: Placement) -> Self; }

// node.rs — Clone, Debug, PartialEq
pub struct Node { pub base: ElementBase, pub kind: NodeKind, #[serde(default)] pub children: Vec<NodeId> }
#[serde(tag = "type", rename_all = "snake_case")]                  // exhaustive
pub enum NodeKind { Group(GroupNode), Shape(ShapeNode), Extension(ExtensionNode) }
pub struct GroupNode {}
pub struct ShapeNode { pub geometry: ShapeGeometry, #[serde(default)] pub fill: Option<Color> }
pub enum ShapeGeometry { Rect, Ellipse }                            // snake_case
pub struct ExtensionNode { pub type_id: String, pub schema_version: u32, pub payload: serde_json::Value }
pub const MAX_EXTENSION_DEPTH: usize = 64;  // D4. Depth: a scalar is 0, `[]`/`{}` is 1, `[[1]]` is 2; computed iteratively

// page.rs — Clone, Debug, PartialEq
pub struct Page { pub id: PageId, #[serde(default)] pub name: String, pub size: PageSize,
                  #[serde(default)] pub locked: bool, #[serde(default)] pub hidden: bool,
                  #[serde(default)] pub roots: Vec<NodeId>,
                  #[serde(default, with = "arena")] pub nodes: BTreeMap<NodeId, Arc<Node>> }
impl Page { pub fn new(id: PageId, name: String, size: PageSize) -> Self;
            pub fn node(&self, id: NodeId) -> Option<&Node>; }

// document.rs — Clone, Debug, PartialEq; fields PRIVATE
pub struct Document { schema_version: u32, next_id: u64, dpi: Dpi, pages: Vec<Arc<Page>> }
impl Document {
    pub const SCHEMA_VERSION: u32 = 1;
    pub fn schema_version(&self) -> u32;  pub fn next_id(&self) -> u64;  pub fn dpi(&self) -> Dpi;
    pub fn pages(&self) -> &[Arc<Page>];  pub fn page(&self, id: PageId) -> Option<&Page>;
    pub fn validate(&self) -> Result<(), ValidationError>;                                    // D4
    pub fn from_parts(next_id: u64, dpi: Dpi, pages: Vec<Arc<Page>>) -> Result<Self, ValidationError>; // D4
    pub fn from_json(bytes: &[u8]) -> Result<Self, LoadError>;                                // D3, typed in D4
    pub fn to_json(&self) -> Result<String, serde_json::Error>;                               // D3, compact
    pub fn edit(&mut self, token: &EditToken) -> DocMut<'_>;                                  // D5
    #[cfg(test)] pub(crate) fn alloc_id(&mut self) -> Result<u64, IdSpaceExhausted>;          // D2b; gate removed in D5
}
#[cfg(test)] pub(crate) fn test_document(next_id: u64, dpi: Dpi, pages: Vec<Arc<Page>>) -> Document; // D2b, unvalidated
impl Default for Document { /* schema 1, next_id 1, dpi 96, no pages */ }
#[non_exhaustive] pub enum LoadError { Json(serde_json::Error), Invalid(ValidationError) }  // Display, Error + source

// edit.rs (D5)
pub struct EditToken { _private: () }
impl EditToken { #[doc(hidden)] pub const fn __new_for_gafira_edit_only() -> Self; }
pub struct DocMut<'a> { doc: &'a mut Document }
impl DocMut<'_> {
    pub fn alloc_node_id(&mut self) -> Result<NodeId, IdSpaceExhausted>;
    pub fn alloc_page_id(&mut self) -> Result<PageId, IdSpaceExhausted>;
    pub fn set_dpi(&mut self, dpi: Dpi);
    pub fn pages_mut(&mut self) -> &mut Vec<Arc<Page>>;
    pub fn page_mut(&mut self, id: PageId) -> Option<&mut Page>;                  // Arc::make_mut
    pub fn node_mut(&mut self, page: PageId, id: NodeId) -> Option<&mut Node>;    // make_mut on the path
    pub fn as_doc(&self) -> &Document;
}

// error.rs (D4) — #[non_exhaustive], Clone, Debug, PartialEq, Display, Error; 18 variants, listed in pass order
pub enum ValidationError {
    UnsupportedSchemaVersion { found: u32 }, InvalidDpi { dpi: u32 },
    DuplicateId { id: u64 }, ArenaKeyMismatch { page: PageId, key: NodeId, id: NodeId },
    NextIdNotAboveMax { next_id: u64, max_id: u64 },
    PageSize { page: PageId, error: PageSizeError },
    DanglingRoot { page: PageId, node: NodeId }, DuplicateRoot { page: PageId, node: NodeId },
    NonFinite { node: NodeId, field: NodeField }, OutOfRange { node: NodeId, field: NodeField, value: f64 },
    EmptyExtensionTypeId { node: NodeId }, ExtensionTooDeep { node: NodeId, depth: usize },
    ChildrenOnLeaf { node: NodeId }, DanglingChild { parent: NodeId, child: NodeId },
    MultipleParents { node: NodeId }, RootHasParent { node: NodeId, parent: NodeId },
    Orphan { node: NodeId }, Cycle { node: NodeId },
}
pub enum NodeField { X, Y, W, H, RotationDeg, Skew, Opacity, FillChannel, FillAlpha }
```

**Validation passes** (the first error wins; pages in `pages` order, nodes in ascending id order):

| Pass | Delivery | Checks, with variant positions 1–18 |
|---|---|---|
| P1 Header | D4 | 1 `UnsupportedSchemaVersion` (`schema_version == 1`); 2 `InvalidDpi` (1..=10 000) |
| P2 Ids | D4 | 3 `DuplicateId` (all page and node ids are distinct); 4 `ArenaKeyMismatch` (key equals `base.id`); 5 `NextIdNotAboveMax` (`next_id > max(id)`; an empty document accepts any `next_id`) |
| P3 Pages | D4 | 6 `PageSize` (`PageSize::check(dpi)`); 7 `DanglingRoot`; 8 `DuplicateRoot` |
| P4 Nodes | D4 | Node by node in ascending id; for each node the checks run in the listed order and the first failure ends the pass (a node's check 14 is reached before the next node's check 9). 9 `NonFinite`; 10 `OutOfRange` (A9); 11 `EmptyExtensionTypeId`; **12 `ExtensionTooDeep`** (payload depth > 64); 13 `ChildrenOnLeaf`; 14 `DanglingChild` (every child is in the same arena) |
| P5 Parents | D5 | 15 `MultipleParents` (more than one parent edge); 16 `RootHasParent`; 17 `Orphan` (no parent and not a root) |
| P6 Reachability | D5 | 18 `Cycle`: iterative DFS from the roots (no recursion, so a deep tree cannot overflow the stack); any node not reached. After P5, an unreached node has exactly one parent, so it lies in or below a cycle (a ring, or a subtree hanging off a ring) that no root reaches. `Cycle { node }` names the first such node in ascending id, which is not necessarily on the ring itself. |

Between D4 and D5, `validate()` runs P1–P4 only. A D4 build therefore still accepts cycles and multiple parents. That is acceptable inside a stacked chain, and the D5 tests close the gap.

**Not exposed**: `&mut` access to `Document` fields outside `DocMut`; `DocumentWire`; the arena serde helpers; the validation passes (`pub(crate)`); id allocation outside `DocMut`; `kurbo`; `Transform2D` in the model; `Animatable`; `NodeVisitor`/`walk`; `msg`/`ToMessage`; stroke, `Path` and other node kinds; repair or clamping helpers; any I/O.

## Wire Format (exact)

Key order is the declaration order (serde derive, ARQ-10). Every field is always written, with no `skip_serializing_if`: `None` is written as `null`. The canonical form is compact (`to_json`). Example, pretty-printed here for reading only (the golden files are single-line):

```json
{"schema_version":1,"next_id":5,"dpi":96,"pages":[
 {"id":1,"name":"Cover","size":{"width":{"value":210.0,"unit":"mm"},"height":{"value":297.0,"unit":"mm"}},
  "locked":false,"hidden":false,"roots":[2],"nodes":[
   {"base":{"id":2,"name":null,"placement":{"x":0.0,"y":0.0,"w":200.0,"h":100.0,"rotation_deg":0.0,"skew":0.0,"flip":{"x":false,"y":false}},
            "opacity":1.0,"blend":"normal","visible":true,"locked":false},
    "kind":{"type":"group"},"children":[3,4]},
   {"base":{"id":3,"name":"Sun", "...":"..."},"kind":{"type":"shape","geometry":"ellipse","fill":{"space":"srgb","r":1.0,"g":0.42,"b":0.34,"a":1.0}},"children":[]},
   {"base":{"id":4,"name":null, "...":"..."},"kind":{"type":"extension","type_id":"com.example.sticker","schema_version":2,"payload":{"a":[1,-2,3.5],"z":"é"}},"children":[]}]}]}
```

| Input | Result |
|---|---|
| Missing `name`, `locked`, `hidden`, `visible`, `blend`, `opacity`, `rotation_deg`, `skew`, `flip`, `children`, `roots`, `nodes`, `fill`, `dpi` | Defaults are filled in; a re-save writes them explicitly |
| Unknown extra fields (no `deny_unknown_fields`, 02 §3.5) | Ignored, and **dropped on save**. Only `Extension.payload` round-trips unknown content (ARQ-11). |
| `"x":3` (an integer for an f64 field); nodes out of id order; a non-canonical payload | Accepted and normalized on save (`3.0`, ascending ids, canonical payload) |
| Missing `schema_version`, `next_id`, `id`, `size`, `x/y/w/h`, `kind`, `type`, `type_id`, `schema_version` (extension) or `payload` | Rejected |
| Unknown `type`, `geometry`, `blend`, `unit` or `space`; wrong letter case (`"Group"`) | Rejected |
| `null` in a number field (how serde_json writes NaN/inf), `1e400`, negative ids, `next_id` ≥ 2^64, `dpi` > u32::MAX | Rejected |
| A duplicate struct field; a duplicate node id inside one `nodes` array; trailing characters; invalid UTF-8; a lone surrogate `\ud800` | Rejected (a serde error, never a panic) |
| Nesting beyond serde_json's recursion limit: as a bare `Value`, 127 levels parse and 128 fail; inside a full document, an extension payload of 121 levels parses and one of 122 or more fails | Rejected at parse time (`LoadError::Json`) |
| From D4: an extension payload deeper than 64 levels (65..=121 still parse) | Rejected by validation (`LoadError::Invalid(ExtensionTooDeep)`), the same variant `validate()` returns for a document built in memory |
| From D4: anything else that fails `validate()`, including `schema_version != 1` | Rejected (`LoadError::Invalid`). Read-only opening of a newer schema is a `gafira-format` concern. |

## Delivery Cut (seven stacked deliveries)

The measured factor is ×1.9 on raw estimates, with tests included in the raw numbers; exact files (manifests, `clippy.toml`, CI lines, goldens) count ×1.0. The hard limit is 800 changed lines per delivery, measured with `git diff --shortstat <prev>...HEAD -- . ':!Cargo.lock' ':!openspec'`. The planning target is **≤ ~700 per delivery**, so that each one keeps at least 100 lines of margin.

| D | Row | Raw | Expected measured |
|---|---|---|---|
| D1 | 3× `clippy.toml` (+2, +2, 47 new) / `ci.yml` (V5 +1, V12 +15 −3) / root and crate `Cargo.toml` (1 + 20) | exact | 51 / 19 / 21 |
| | `lib.rs` / `ids.rs` / `dpi.rs` / `page_size.rs` / `blend.rs` | 20 / 60 / 50 / 90 / 45 | 38 / 114 / 95 / 171 / 86 |
| | **D1 total** | | **~595** |
| D2a | `placement.rs` / `element.rs` / `lib.rs` exports | 130 / 45 / exact | 247 / 86 / 2 |
| | **D2a total** | | **~335** |
| D2b | `node.rs` / `page.rs` / `document.rs` (with the `#[cfg(test)]` allocator) / `IdSpaceExhausted` in `ids.rs` | 80 / 50 / 80 / 10 | 152 / 95 / 152 / 19 |
| | `test_document` helper + "Clone shares everything" test | 25 | 48 |
| | **D2b total** | | **~465** |
| D3 | serde attributes across modules / `arena.rs` / `json.rs` | 40 / 40 / 60 | 76 / 76 / 114 |
| | In-crate unit tests using the D2b `test_document` (minimal exact, full, insertion order, NaN not persistable) | 70 | 133 |
| | `tests/serde.rs` (fixtures only) / `tests/golden.rs` / goldens + `.gitattributes` | 110 / 40 / exact | 209 / 76 / 5 |
| | **D3 total** | | **~690** |
| D4 | `error.rs` (18 variants) / `validate.rs` P1–P4 with the depth check / `try_from` switch + `from_parts` / table-driven tests (P1–P4 rows, depth 64/65 and JSON 127/128 boundaries) | 85 / 110 / 15 / 125 | 162 / 209 / 29 / 238 |
| | **D4 total** | | **~640** |
| D5 | `validate.rs` P5–P6 / their table rows + 10 000-deep chain | 50 / 30 | 95 / 57 |
| | `edit.rs` (removes the allocator gate) / `tests/edit.rs` (including "Changing the DPI" and "Edit touches one path") / `ci.yml` V13 | 70 / 70 / exact | 133 / 133 / 45 |
| | **D5 total** | | **~465** |
| D6 | `tests/common/mod.rs` / `tests/properties.rs` (Q1–Q4 and the Q5 corruption-never-panics property) | 170 / 170 | 323 / 323 |
| | **D6 total** | | **~645** |
| | **Change total** | | **~3 835** (595 + 335 + 465 + 690 + 640 + 465 + 645; the earlier six-delivery figure was ~3 578, and the proposal said 2 600–2 900. The increase comes from the V12/V13 hardening, `ExtensionTooDeep`, the unit-test helper and the Q5 property.) |

**Re-balancing compared with the proposal**:
- `BlendMode` moves to D1.
- The old D2 (~730, only 68 lines of margin) splits into D2a (`Placement` + `ElementBase`) and D2b (`Node` + `Page` + `Document` + arena).
- P5–P6 (parents, reachability) move from D4 to D5, which keeps D4 under 700.
- D4 and D5 MUST use table-driven tests: one `(mutation, expected variant)` row per case, about 3 lines each.

**Dependencies**:
- D1 starts from `main`.
- D2b (`ShapeNode.fill`) needs `Color`, which is already on `main` (C2a, PR #5 merged, `main` 75f7e78): D1, D2a and D2b all branch from the same chain with no wait.
- The chain is linear: D1 ← D2a ← D2b ← D3 ← D4 ← D5 ← D6.
- Branches: `feat/domain-doc-1` … `-7`, each based on the previous one.

**Commit order** (each commit passes `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, V5, V6, V11, V12 and, from D5, V13):

| D | Commits |
|---|---|
| D1 | 1 `build(lint): ban HashMap and HashSet in domain clippy policy`: adds the ban to the geom and color copies only. **V12 keeps its current form**; there are two identical copies, and neither crate uses the types. 2 `feat(doc): add gafira-doc crate skeleton`: manifest, `clippy.toml` copy, `lib.rs` with `forbid`, root dependency and `Cargo.lock`, **plus the V5 row and the V12 hardening in the same commit**. A `clippy.toml` without a row fails V5, and `n >= 3` first holds here. 3 `feat(doc): add NodeId and PageId`. 4 `feat(doc): add Dpi`. 5 `feat(doc): add PageSize with pixel limits`. 6 `feat(doc): add BlendMode` |
| D2a | `feat(doc): add Placement and its affine transform` → `feat(doc): add ElementBase` |
| D2b | `feat(doc): add node kinds` → `feat(doc): add Page and node arena` → `feat(doc): add Document root` (adds `IdSpaceExhausted`, `alloc_id` gated `#[cfg(test)]`, and the `#[cfg(test)]` helper `test_document` together with the "Clone shares everything" test that calls it) |
| D3 | `feat(doc): define the document wire format` (attributes and defaults; no helper is added here) → `feat(doc): serialize arenas as sorted arrays` → `feat(doc): add from_json/to_json` (`from = "DocumentWire"`, unchecked) → `test(doc): add canonical goldens and hostile inputs` (JSON fixtures only) |
| D4 | `feat(doc): add ValidationError` (all 18 variants) → `feat(doc): validate header, ids and pages` (P1–P3) → `feat(doc): validate node fields and extension depth` (P4) → `feat(doc): validate on every deserialization` (`try_from`, `LoadError::Invalid`, `from_parts`) |
| D5 | `feat(doc): validate parents and reachability` (P5–P6) → `feat(doc): add EditToken and DocMut` (**removes the `#[cfg(test)]` gate on `alloc_id`**; adds `set_dpi`/`edit`) → `ci: restrict EditToken to the edit crates` (V13) |
| D6 | `test(doc): add bounded document generator` → `test(doc): add ARQ-10 and ARQ-13 properties` |

**Each prefix passes on its own.** A new module is exported from `lib.rs` in the commit that adds it, so public items are never dead code. Crate-private items with no non-test consumer yet follow the gate table in "Dead code before its consumer"; `#[allow]` is never used. V12 hardening and the third copy land together, and the V5 row lands with the `clippy.toml` that requires it. Every D3 fixture is a valid document (every node is a root or a child, see the fixture rule), so D3's tests keep passing after the D4 `try_from` switch and after the D5 passes.

**Measurement and re-cut rule** (replaces the cumulative 1 720 threshold):
1. Measure **every** delivery at its end.
2. If a delivery measures over 800, split it at a commit boundary into two stacked branches before pushing.
3. If a delivery measures more than ×1.3 of its forecast, recompute the forecasts of the remaining deliveries with the observed ratio. Re-cut any remaining delivery whose new forecast exceeds **~700** before starting it. The natural split points are D3 into wire/goldens and hostile inputs, and D6 into the generator and the properties.
4. Never cut a requirement, and never rewrite pushed history.

## Testing Strategy (Strict TDD)

| T | Task | RED | GREEN | REFACTOR / mutation |
|---|---|---|---|---|
| T1.1 (D1 c1) | `HashMap` ban | Scratch `let _m: std::collections::HashMap<u8, u8> = Default::default();` in geom and color: clippy passes before the ban | Ban lines in geom and color; clippy reports "disallowed type"; the old V12 passes (2 identical copies) | Revert the scratch; `git status` clean |
| T1.2 (D1 c2) | Skeleton + V12 hardening | `cargo check -p gafira-doc` → "did not match any packages"; V5 with only a `clippy.toml` → "no V5 row"; H-N3/H-N5/H-N6 pass under the old V12 | Manifest, `lib.rs`, copy, row, hardened V12; V5/V11/V12 pass; the scratch `HashMap` fails clippy in doc; H-N3–H-N6 fail as expected | — |
| T1.3 | Ids | Tests fail to compile | Ordering, `get`/`new`, `Display`, `serde(transparent)` (`5` ↔ `NodeId(5)`), `u64::MAX` survives | — |
| T1.4 | `Dpi` | Same | Default 96; `is_valid` at 0, 1, 10 000, 10 001; wire `96` | Mutation: `MAX = 10_001` fails |
| T1.5 | `PageSize` | Same | A4 at 300 DPI = 2480×3508 after `round()` (FD-01a); 1.0 px and 16 384.0 px accepted; `next_up(16384.0)` and `next_down(1.0)` rejected, per axis, with the axis named; A0 at 350 DPI accepted, at 351 rejected; NaN width → `Conversion` | Mutation: `<` instead of `<=` fails a boundary test |
| T1.6 | `BlendMode` | Same | Exhaustive `match` with no wildcard over `ALL`; the 16 kebab-case names exactly; `"Multiply"` and `"color_dodge"` rejected | — |
| T2.1 (D2a) | `Placement` | Same | Identity placement maps (0,0) → (x,y) and (w,h) → (x+w,y+h); rotating 90° about the center (reference points within `1e-12`); `flip.x` maps the local (0,0) to (x+w, y); skew 45° shears x by y; `to_transform` uses only `gafira_geom::Affine` | Mutation: swap scale and skew, or pivot at the top-left → reference fails |
| T2.2 (D2a) | `ElementBase` | Same | `new` defaults (opacity bits 1.0, visible, Normal); `PartialEq` IEEE (`-0.0 == 0.0`; a NaN placement makes the value unequal to itself) | — |
| T2.3–2.5 (D2b) | Kinds, `Page`, `Document` | Same | `Page::node`; `Document::default`; `page(id)`; `pages()` order; the `#[cfg(test)]` `alloc_id` increments `next_id` and returns `IdSpaceExhausted` at `u64::MAX` (unit tests); `test_document` + **"Clone shares everything"** (`Arc::ptr_eq` across a clone) in the same commit | Measure D2b |
| T3.1 (D3, unit) | Built documents → exact wire | In-crate tests fail to compile | With the D2b `test_document`: **"minimal exact"** (an empty-page document gives exact bytes); **"full"** (a document built in code serializes to the `full` golden); **"insertion order"** (nodes inserted out of order are written in ascending id order); **"NaN not persistable"** (a NaN placement serializes as `null` and the bytes fail to load) | Mutation: arena serialized in insertion order → "insertion order" fails |
| T3.2 (D3, `tests/serde.rs`) | Exact wire per type on fixtures | `tests/serde.rs` fails | Exact compact JSON of each value type (key order, `null` for `None`, kebab and snake names), checked by loading fixtures through `Deserialize` (unchecked in D3) and re-serializing | — |
| T3.3 (D3) | Defaults / forward compatibility | Same | Every defaultable field omitted → defaults; unknown field ignored and dropped; required fields missing → error (one row per field) | Mutation: remove the `visible` default fn → test fails |
| T3.4 (D3) | Hostile inputs | Same | Table of the "Wire Format" rejection rows that apply before D4; each is an `Err`, never a panic. Depth boundaries are in T4.2. | — |
| T3.5 (D3) | Animatable compatibility | Same | `{"keys":[]}` fails as `Placement`, as `Color` and as `f32`; bare `0.5` is an `f32` | Mutation: container `#[serde(default)]` on `Placement` → test fails |
| T3.6 (D3, `tests/golden.rs`) | Goldens (ARQ-11) | `tests/golden.rs` fails | Loading `minimal`, `full` or `extension` through `Deserialize` and re-saving gives **identical bytes**; `extension_noncanonical` (unsorted keys, whitespace, `1E2`, `é`) re-saves as exactly `extension` (normalization documented) | Mutation: rename a tag → goldens fail |
| T3.7 (D3) | serde_json feature guards | Same | `{"b":1,"a":2}` → `{"a":2,"b":1}` (no `preserve_order`); `1.10` → `1.1` (no `arbitrary_precision`); an `f64` inside an extension node keeps its bits through the internally tagged path | — |
| T4.1 (D4) | `ValidationError` | Tests fail to compile | All 18 variants; `Display` names the ids and the field; usable as `&dyn Error`; `LoadError::source()` | — |
| T4.2 (D4) | Passes P1–P4 | Table test fails | One row per P1–P4 variant (14 rows), each a minimal fixture built in-crate; order rows: a document with two defects reports the earlier position. Depth: a payload of depth 64 passes and 65 → `ExtensionTooDeep`; a bare `Value` of 127 levels parses and 128 fails; a document whose payload has 121 levels parses then fails validation, and 122 fails to parse | Mutations: `>=` for `next_id`; `> 65` for the depth limit |
| T4.3 (D4) | Validated deserialization + `from_parts` | `serde_json::from_str::<Document>` accepts an invalid document | `try_from`; `from_json` and `from_parts` return the same variant as `validate`, including `ExtensionTooDeep`; the D3 fixtures still pass | Mutation: revert to `from` → test fails |
| T5.1 (D5) | Passes P5–P6 | Table rows fail | 4 rows (`MultipleParents`, `RootHasParent`, `Orphan`, `Cycle`, all distinct; together with T4.2 these are 18 rows and cover the proposal's seven success criteria); a 10 000-deep chain validates without stack overflow | Mutations: drop P6 → the cycle row fails; count parents per page → the cross-page row fails |
| T5.2 (D5) | `EditToken`/`DocMut` | `tests/edit.rs` fails | Allocation is monotonic and shared by pages and nodes; `u64::MAX` → `IdSpaceExhausted`, with `next_id` unchanged; **"Changing the DPI"** (`set_dpi` to a value that pushes a page over 16 384 px → `validate` returns `PageSize`); **"Edit touches one path"** (`node_mut` leaves untouched nodes and pages shared: `Arc::ptr_eq` after cloning the document). The `alloc_id` gate is removed in this commit. | — |
| T5.3 (D5) | V13 | E1 (below) passes with no step | Step body; E0–E15 as expected | — |
| T6.1 (D6) | Generator | Guard tests fail | Generator plus guards | — |
| T6.2 (D6) | Properties | RED by mutation (below) | Q1–Q5 | Mutations reverted; `git status` clean |

**Bounded generator** (`tests/common/mod.rs`, built through `DocMut` with the token, which is allowed inside `crates/gafira-doc/`):

| Bound | Value |
|---|---|
| Pages | 1..=3 (plus a dedicated empty-document case) |
| Nodes per page | 0..=12; tree depth ≤ 4; only groups get children; the parent of node *i* is drawn from earlier groups, or it is a root, so documents are valid by construction |
| Extension payload | `Value` depth ≤ 3, ≤ 4 entries per array or object, strings ≤ 8 chars including non-ASCII, `"`, `\` and control characters; numbers include i64/u64 extremes, `-0.0` and finite f64 |
| Ids | Allocated from the counter in shuffled order across pages; `next_id` gets an extra gap of 0..=3 |
| Floats | `finite()` from geom (POSITIVE, NEGATIVE, NORMAL, SUBNORMAL, ZERO) for `x`/`y`/`rotation_deg`; `w`/`h` ≥ 0 including `0.0` and `-0.0`; `skew` in (−90, 90) including ±89.999; `opacity` in [0,1] with `Just(0.0)`, `Just(-0.0)`, `Just(1.0)`, subnormals; fill channels in [−4, 4], alpha in [0,1] |
| Page sizes | Units px/mm/cm/in/pt; values chosen so that px lands on 1.0, 16 384.0, near both limits, or random in range; DPI from {1, 72, 96, 300, 600, 10 000} ∪ 1..=10 000 |

**Properties** (`tests/properties.rs`; named Q1–Q5 so they do not clash with validation passes P1–P6):

| ID | Property | Cases |
|---|---|---|
| Q1 | ARQ-10: `to_json(from_json(to_json(d))) == to_json(d)` (bytes), and `from_json(to_json(d))` equals `d` field by field with `to_bits()` | **10 000** (`ProptestConfig::with_cases(10_000)`; the env cannot be read because `std::env` is banned in tests too) |
| Q2 | Every generated document validates | default 256 |
| Q3 | ARQ-13: one corruption from {duplicate id (across two pages, or a page id equal to a node id: an in-page duplicate cannot exist in a `BTreeMap`), key mismatch, `next_id` ≤ max, page size below or above the limit, dangling root, duplicate root, non-finite field, out-of-range opacity/alpha/skew/w/h, empty `type_id`, extension payload deepened to 65 levels, children on a leaf, dangling child, multiple parents, root with a parent, orphan, cycle, bad DPI, schema 2} → `validate` returns **exactly** that variant. **Wire clause**: for a *wire-visible* corruption, `from_json(to_json(d))` is `Err(Invalid(same))`. A corruption is wire-visible when `to_json` writes it and a reader reads the same value back. **Two corruptions are excluded from the wire clause** (they still get the `validate` clause): `ArenaKeyMismatch`, because the arena is written as the node array only, so the key is not on the wire and a reload derives it from `base.id` and succeeds; and `NonFinite`, because `to_json` writes `null` and the reload fails earlier with `LoadError::Json`, never `Invalid(NonFinite)`. Every other corruption is wire-visible. | default 256 |
| Q4 | Canonical payload: `to_string(from_str::<Value>(to_string(v)))` is byte-stable | default 256 |
| Q5 | Corruption never panics: a golden document (`minimal`, `full`, `extension`) with one random single-byte mutation (replace, insert or delete) → `from_json` returns `Err`, or an `Ok` document that passes `validate()`; nothing panics. Goldens come from `include_str!`. | default 256 |

**Guards** (`TestRunner::deterministic()`, 20 000 draws, copied from geom): they prove that the generators reach `-0.0`, subnormals, px exactly 1.0 and 16 384.0, opacity 0.0/1.0/−0.0, depth ≥ 3, an extension node, a group with children, several pages, an empty page, and every corruption variant. Without them the bit-identity and coverage claims would be vacuous. Every bit identity is compared with `to_bits()` and never with `==`.

**Property mutations** (applied one by one and reverted): serialize the arena in insertion order (Q1 must fail on a shuffled build); drop `float_roundtrip` from the workspace `serde_json` (Q1 must fail); remove the cycle pass (Q3); count parents per page instead of per arena (Q3); clamp opacity on read (Q1 on bits or Q3). An undetected mutation is recorded with the reason, as in C2a.

**Runtime**: D6 records the wall time of `cargo test -p gafira-doc --test properties` on each CI runner. If it exceeds 60 s on the slowest runner, reduce the bounds (nodes, payload depth), never the 10 000 cases of Q1.

## Verification Plan

| ID | Command | Locally today? |
|---|---|---|
| Gates | `cargo fmt --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo test --workspace --locked`; `cargo deny check` | Yes (advisories need network) |
| V3 | `cargo metadata --no-deps`: `gafira-doc` has edition 2024, rust_version 1.93, publish [] | Yes |
| V5 | Scratch copy of the committed step (`$SCRATCH/v5.sh`, extracted from `ci.yml` as in C2a) | Yes |
| V5-tree | `cargo tree -p gafira-doc -e normal,build --target all --prefix none -f '{p}'`: only `gafira-geom` and `gafira-color` among `gafira-*`; no forbidden prefix; direct normal deps: geom, color, serde, serde_json | Yes |
| V5-dev | `cargo tree -p gafira-doc -e normal,build,dev …`: no `rusty-fork`/`tempfile` | Yes |
| Features | `cargo tree -p gafira-doc -e features -i serde` shows `rc`; `-i serde_json` shows neither `preserve_order` nor `arbitrary_precision` | Yes |
| V11 / V12 / V13 | Step bodies run from scratch copies; negatives below | Yes |
| Lock | `git diff Cargo.lock` adds only the `gafira-doc` package | Yes |
| CI | Three ARQ-03 targets plus `policy`, green, after each pushed delivery | After push |

**Manual negatives** (`bash --noprofile --norc -eo pipefail "$SCRATCH/<step>.sh"; echo "exit=$?"`, each reverted, `git status` clean):

| Case | Mutation | Expected |
|---|---|---|
| D-N0 | Clean tree | V5 exit 0; xtrace shows `cargo tree -p gafira-doc` ran |
| D-N1 (primary) | Shrink the row to `gafira-doc gafira-geom` | exit 1, `gafira-doc must not depend on gafira-color` |
| D-N2 (secondary, verified) | Add `gafira-cli = { path = "../../apps/gafira-cli" }` to doc's `[dependencies]` (the proposal's forced edge) | Cargo warns "ignoring invalid dependency `gafira-cli` … missing a lib target", but `cargo tree` **does** list the edge (validator run), so V5 exits 1 with `gafira-doc must not depend on gafira-cli`. Revert `Cargo.toml` and `Cargo.lock`. |
| D-N3 | Delete the `gafira-doc` row | exit 1, "has clippy.toml but no V5 row" |
| H-N1 | Scratch `HashMap`/`HashSet` use in each of the three crates | clippy "disallowed type", exit non-zero (×3) |
| H-N2 | Append a line to doc's `clippy.toml` | V12 `diff -q` reports a difference, exit 1 |
| H-N3 | Remove both ban lines from all three copies consistently | V12 exit 1, "lacks the std::collections::HashMap ban" (the positive control; the old V12 exits 0, which is the RED) |
| H-N4 | Remove the doc and color copies (only one left) | V12 exit 1, "expected >= 3" |
| H-N5 | Comment out the `HashMap` ban line (`# { path = "std::collections::HashMap", … },`) in all three copies | V12 exit 1, "lacks the std::collections::HashMap ban under disallowed-types" (the old V12, and an unanchored grep, exit 0) |
| H-N6 | Move both ban lines from `disallowed-types` to `disallowed-methods` in all three copies | V12 exit 1, same message (clippy would silently stop banning the type) |
| E0 | Clean tree | V13 exit 0 |
| E1 | `// EditToken` in `apps/gafira-cli/src/main.rs` | exit 1 naming the file. With no step it passes (the RED). |
| E2 | `__new_for_gafira_edit_only` in a `crates/gafira-color` test | exit 1 |
| E3 | Rename the definition to `pub struct EditTok {` | exit 1, positive control |
| E4 | Break the pattern (`'EditToken('`) / break `allowed` (`'^crates/(gafira-doc'`) | exit 1, "git grep failed" / "filter grep failed" |
| E5 | Run from `crates/` (wrong cwd) / from a non-repository copy | exit 1 (positive control / git rc 128) |
| E6 | Untracked `crates/gafira-edit/src/lib.rs` with `pub use gafira_doc::EditToken as Cap;` | exit 1, re-export |
| E7 | Same file with a multi-line `pub use gafira_doc::{\n    EditToken,\n};` | exit 1, re-export (one `;`-delimited record) |
| E8 | Same file with `pub type Cap = gafira_doc::EditToken;` | exit 1, re-export |
| E9 | Same file with `pub use gafira_doc::*;` | exit 1, public glob |
| E10 | No `crates/gafira-edit/` directory (today's tree) | exit 0, the positive control found the definition, and no git warning on stderr (the plain directory pathspec is silent; a warning, if any, would not change the result) |
| E11 | `use gafira_doc::EditToken as T;` followed by `pub use self::T;` | exit 0: **known limit 1**, recorded as not met |
| E12 | Untracked `crates/gafira-edit/src/lib.rs` with `pub(crate) use gafira_doc::EditToken;` | exit 0 (allowed) |
| E13 | Same file with `pub fn lend(t: &gafira_doc::EditToken) -> &gafira_doc::EditToken { t }` | exit 0: **known limit 2**, recorded as not met |
| E14 | `EditToken` mentioned in `crates/gafira-doc/tests/edit.rs`, a scratch `crates/gafira-testkit/src/lib.rs` and a `gafira-edit` file without re-export | exit 0 (allowed crates) |
| E15 | Same file with `pub use gafira_doc as d;`, and separately `pub use gafira_doc::edit;` | exit 0: **known limit 6**, recorded as not met |
| N6/N7 | V11 without `forbid` in doc's `lib.rs` | exit 1 (existing V11 loop) |

**Not verifiable today**: the `trybuild` proof of ARQ-30 (C3); the ARQ-13 fuzzer (`gafira-format`); per-page files (§7.1); `cargo public-api` (ARQ-05); ARQ-32 timings; the 10 000-case runtime on the three CI runners (only after the first D6 push).

## Not Verifiable at Design Time (check during apply)

| Item | How |
|---|---|
| serde 1.0.229 (split `serde_core`) still has the `rc` feature, and `Arc<T>` derives work | `cargo tree -e features`; T2/T3 compile |
| `RawValue` really fails inside an internally tagged enum (the A4 rationale) | Optional 10-line scratch test outside the repo; record the result in apply-progress |
| `Value` and `f64` fields keep their bits through the `Content` path | T3.7 and Q1 |
| serde derive reports a duplicate struct field inside an internally tagged variant | Row in T3.4 |
| clippy resolves `std::collections::HashMap`/`HashSet` (no "does not refer to an existing" warning) | H-N1, plus a clean `cargo clippy` with no config warnings |
| `git grep --untracked` exit codes and `.gitignore` handling on the ubuntu-24.04 git | E0–E15 locally and the first CI run |
| `awk 'BEGIN{RS=";"}'` behaves the same in mawk (the ubuntu default) and gawk; `git ls-files` with a plain directory pathspec that matches nothing is silent and exits 0 | E7–E10 locally and in CI (the pathspec form is already verified silent on git 2.34.1; a glob pathspec is not) |
| The exact depth thresholds (127/128 for a bare `Value`, 121/122 for a payload inside a document) on the locked serde_json 1.0.154 | T4.2 boundary rows |
| kurbo 0.13 `Affine::skew(kx, ky)`, `rotate` and `scale_non_uniform` signatures | T2.1 compiles |
| The 10 000-case Q1 runtime | D6 timing, recorded per runner |

## Threat Matrix

| Boundary | Applicability | Design response | Planned RED tests |
|---|---|---|---|
| Documentation-like paths | N/A: no file classification or execution | — | — |
| Git repository selection | **Applicable**: V13 runs `git grep` in the step's cwd | No `-C` and no path arguments other than pathspecs. The positive control requires the definition in `crates/gafira-doc/src`, so a wrong cwd or a missing repository fails closed. | E5 (subdirectory and non-repository) |
| Commit state | N/A: read-only search, no commits | `--untracked` and `ls-files --others` cover new files locally; CI sees a full checkout | (E6–E9 cover untracked files) |
| Push state | N/A: no push | — | — |
| PR commands | N/A: no PR automation | — | — |

## Platforms

- No platform-specific code. Everything is `f64`/`u64` arithmetic with no libm calls except `sin`/`cos`/`tan` in `to_transform`, which is never persisted and is tested with tolerances.
- Persisted bytes come only from user values (shortest round-trip printing), so ARQ-10 holds on Windows, Linux and macOS.
- Goldens are single-line files marked `-text`, so a CRLF checkout on Windows cannot change them. They are read with `include_str!` (no `std::fs`).
- The DFS in validation is iterative, so deep trees behave the same on Windows (1 MB main-thread stack) as on Unix.

## Decisions that constrain C3 `edit-history`

1. The root type is `Document`; `Command::apply(&mut Document)` gets that `&mut` from `DocMut` (or C3 wraps it). Only `edit(&EditToken)` hands out mutation.
2. `Vec<Arc<Page>>` + `BTreeMap<NodeId, Arc<Node>>` + `Arc::make_mut` is the sharing model for ADR-03. A page clone is O(n) `Arc` bumps; ARQ-32 must be measured against that.
3. Ids come from one counter that is never reused within a document. A snapshot restore (undo) also restores `next_id`; C3 decides whether ids used on an abandoned redo branch may be reissued.
4. `validate()` returns the first error in pass order P1–P6. C3's `debug_assert` after each command uses it as is.
5. Changing the DPI can make an existing page invalid (the limits are DPI-relative). A `SetDpi` command must validate, or be rejected.
6. `Placement`, `opacity` and `fill` are bare values. `Animatable` wrapping must keep the A8 rule (no container defaults), so that today's files load as `Static`.
7. The generator lives in `crates/gafira-doc/tests/common`. C3 needs it for command properties, so C3 moves it to `gafira-testkit` (already an allowed V13 path).
8. `trybuild` for ARQ-30 replaces or complements V13 in C3. `compile_fail` doctests are not evidence (their error codes are not checked on stable). **Decision left to C3**: trybuild fixtures that emulate an outside crate must name `EditToken`, and V13 fails on any such file outside `crates/(gafira-doc|gafira-edit|gafira-testkit)/`. The fixtures must therefore live under `crates/gafira-edit/tests/ui/`, or V13 must gain an explicit, reviewed exclusion. Fixtures that contain `pub use` of the token would also trip the re-export check and need the same treatment.
9. Only `Group` has children, and locks do not cascade in the data.
10. `MAX_EXTENSION_DEPTH = 64` is a format constant: raising it later must stay below the parse limit (a payload of 121 levels inside a document).

## Spec Alignment

The specs in `specs/*/spec.md` and this design are aligned on the same orchestrator decisions:
- seven deliveries;
- 18 `ValidationError` variants including `ExtensionTooDeep`;
- in-page duplicate ids rejected at parse time;
- the V12/V13 hardening and its known limits;
- `compile_fail` not counted as evidence;
- the D2b helper and the D3 and D5 test placement;
- property names Q1–Q5 (distinct from validation passes P1–P6).

## Migration / Rollout

No migration: there is no persisted user data yet. Roll back per delivery, newest first. In full: delete `crates/gafira-doc/`, the root dependency line and its `Cargo.lock` entry, the V5 row and V13, and restore the old V12 body. Keeping the `HashMap` ban in the geom and color copies is harmless.

## Open Questions (holder)

- [ ] Q1: Page limits at the document DPI (this design), with `Dpi` per document as `u32` in 1..=10 000. Confirm the DPI range.
- [ ] Q2: Reject out-of-range or non-finite values (this design) rather than clamp. Confirm.
- [ ] Q3: ARQ-11 as "byte for byte for canonical payloads, by value otherwise" (`Value`, amendment 2). Confirm.
- [ ] Q4: `Document` as the root name (amendment 4); no lock cascade in the data. Confirm.
- [ ] Q5: `gafira-cli validate` deferred to `gafira-format` (A15), with the C1/C2a chain tables out of date. Confirm, or add D7 (~250 lines).
- [ ] The forecast is ~3 835 lines across seven deliveries, every one at or under ~700. Accept the stacked chain.
