# Verify report: domain-doc

## D1 (branch `feat/domain-doc-1`, range `369e3d5..6e6e013`, 7 commits)

**Verdict: PASS** (D1 only). CRITICAL 0, WARNING 3, SUGGESTION 3. The change is not complete (D2a to D6 remain), so `next` = tdd-apply for D2a.

Verified 2026-10-24 on toolchain 1.93.1, clippy 0.1.93, cargo-deny 0.20.2, git 2.34.1. Each commit was checked out with `git worktree add --detach` under `/tmp`; negative runs used a scratch worktree of HEAD, removed afterwards. The repository tree was not modified.

### 1. Gates per commit

| Commit | Subject | fmt | clippy `--workspace --all-targets --locked -D warnings` | test `--workspace --locked` (passed) | V5 | V11 | V12 | ARQ-07 | `cargo deny --locked check` |
|---|---|---|---|---|---|---|---|---|---|
| fb679a5 | build(lint): ban HashMap and HashSet in domain clippy policy | 0 | 0 | 0 (154) | 0 | 0 | 0 (old body) | 0 | 0 |
| cb125d9 | feat(doc): add gafira-doc crate skeleton | 0 | 0 | 0 (154) | 0 | 0 | 0 (hardened, 3 copies) | 0 | 0 |
| 4e2377e | feat(doc): add NodeId and PageId | 0 | 0 | 0 (158) | 0 | 0 | 0 | 0 | 0 |
| d8d0908 | feat(doc): add Dpi | 0 | 0 | 0 (162) | 0 | 0 | 0 | 0 | 0 |
| 1989f90 | feat(doc): add PageSize with pixel limits | 0 | 0 | 0 (173) | 0 | 0 | 0 | 0 | 0 |
| 0238e25 | feat(doc): add BlendMode | 0 | 0 | 0 (177) | 0 | 0 | 0 | 0 | 0 |
| 6e6e013 | docs(sdd): record domain-doc D1 apply progress | 0 | 0 | 0 (177) | 0 | 0 | 0 | 0 | 0 |

The CI steps were extracted from each commit's own `.github/workflows/ci.yml` with a YAML parser and run under `bash --noprofile --norc -eo pipefail`. The clippy logs show `Checking gafira-doc` from cb125d9 on, so no stale cache was involved. Total passed includes the `compile_fail` doctest. "Hardened V12 timing" holds: fb679a5 runs the old 3-line body (no `gafira-doc` V5 row yet), and cb125d9 runs the hardened body with the V5 row.

### 2. Requirements and scenarios

#### workspace-tooling

| Scenario | Evidence | Status |
|---|---|---|
| V5 Clean tree (N0/D-N0) | xtrace shows `cargo tree -p gafira-geom`, `-p gafira-color`, `-p gafira-doc`; rc 0 (`ci.yml:75-101`, row `ci.yml:82`) | covered |
| N1 forbidden internal edge | `gafira-geom.workspace = true` in color: rc 1 "gafira-color must not depend on gafira-geom"; after revert rc 0 | covered |
| N2 misspelled row | rc 101: "gafira-color has clippy.toml but no V5 row" + "package ID specification `gafira-colour` did not match any packages" | covered |
| N3/D-N3 missing row | color row deleted: rc 1 "gafira-color has clippy.toml but no V5 row"; doc row deleted: rc 1 "gafira-doc has clippy.toml but no V5 row" | covered |
| N4 forbidden third party | `serde|` prepended to `forbidden`: rc 1, reports `serde v1.0.229`, `serde_json`, `serde_core` lines | covered |
| N5 prefixed names | `^(egui|...|ureq)` matches `wgpu-core v0.1.0`, `tokio-util v0.7.0`, `winit-x v1.0.0`; no match on `foo-wgpu v1.0.0`, `serde v1.0.0` | covered |
| Crate without clippy.toml (known limitation) | scratch `crates/scratch` with forbid and no clippy.toml: V5 rc 0. **Recorded here as required: the limitation is real and not met by this change** (deferred to `xtask check-layers`) | covered (limitation recorded) |
| Doc may depend on geom and color | `gafira-*` set = {doc, geom, color}; direct normal deps = geom, color, serde, serde_json; V5 rc 0 | covered |
| D-N1 shrunk doc row | row `gafira-doc gafira-geom`: rc 1 "gafira-doc must not depend on gafira-color" | covered |
| D-N2 forced edge to gafira-cli | warning "ignoring invalid dependency `gafira-cli` which is missing a lib target", rc 1 "gafira-doc must not depend on gafira-cli" | covered |
| V12 Copies identical | `diff -q` geom/color rc 0, geom/doc rc 0 | covered |
| H-N2 drifted copy | extra line in doc copy: rc 1 "...gafira-doc/clippy.toml son distintos"; in color copy: rc 1 | covered |
| H-N3 ban removed everywhere | rc 1 "lacks the std::collections::HashMap ban under disallowed-types"; old V12 (fb679a5 body) rc 0 on the same tree (RED) | covered |
| H-N5 commented ban | rc 1 same message; old V12 rc 0 | covered |
| H-N6 ban under `disallowed-methods` | both entries moved into `disallowed-methods` in all copies: rc 1 same message; old V12 rc 0 | covered |
| Hardened V12 timing | see section 1 (fb679a5 old body rc 0, cb125d9 hardened rc 0) | covered |
| H-N4 too few copies | doc and color copies removed: rc 1 "expected >= 3 domain clippy.toml copies, found 1" | covered |
| V11 missing forbid | removed from `gafira-doc/src/lib.rs`: rc 1 "missing forbid(unsafe_code): crates/gafira-doc/src/lib.rs"; same for `gafira-color` | covered |
| Workspace membership | `cargo metadata --no-deps`: gafira-color and gafira-doc are members, edition 2024, rust-version 1.93, publish `[]`; root `Cargo.toml` declares `gafira-doc = { path = "crates/gafira-doc" }` | covered |
| Disallowed I/O flagged (geom) | scratch `std::fs::read`: clippy rc 101 `disallowed_methods` | covered |
| H-N1 HashMap/HashSet in geom, color, doc | scratch lib bindings: each crate fails with "use of a disallowed type `std::collections::HashMap`/`HashSet`" and notes "ARQ-10: unstable iteration order; use BTreeMap/BTreeSet"; a `#[cfg(test)]`-only `HashSet` also fails (`lib test`), so `--all-targets` covers tests; no "does not refer to an existing" warning | covered |
| BTreeMap allowed | scratch `BTreeMap` in doc: clippy rc 0 | covered |
| Entries present in all copies | each of the 3 copies has 1 HashMap and 1 HashSet entry (`clippy.toml:45-46`); V12 rc 0 | covered |
| Clean tree | fmt and clippy rc 0 on every commit | covered |
| `rustfmt.toml` exists | present at repo root | covered |

V13 (D5) is out of scope for D1.

#### document-model

| Requirement / scenario | Evidence | Status |
|---|---|---|
| Purity: forbid unsafe | `crates/gafira-doc/src/lib.rs:6` | covered |
| Purity: manifest | `crates/gafira-doc/Cargo.toml:9-16` (geom, color, serde `rc`, serde_json; dev proptest) | covered |
| Normal dependency tree | `gafira-*` set {doc, geom, color}; 4 direct deps; 0 lines matching UI/engine/IO prefixes | covered |
| Dev tree bounded | dev additions = proptest v1.12.0 and its transitive crates (rand, rand_core, rand_xorshift, chacha20, getrandom, libc, bitflags, num-traits, regex-syntax, unarray, ...); `rusty-fork`/`tempfile` count 0 | covered |
| No kurbo, no new package | `grep -r kurbo crates/gafira-doc` empty; `git diff 369e3d5..HEAD -- Cargo.lock` adds only `name = "gafira-doc"` | covered |
| I/O and unsafe rejected | scratch `std::fs::read` in doc: `disallowed_methods`; scratch `unsafe {}`: `cargo check` error "usage of an `unsafe` block" | covered |
| Public value types are Serialize + Deserialize | NodeId, PageId, Dpi, PageSize, BlendMode derive both; `Axis` and `PageSizeError` do not (see W2) | covered for value types, see W2 |
| NodeId/PageId distinct types | compile_fail doctest `ids.rs:42-48`; sanity mutation (call with `NodeId`) makes the doctest fail with "Test compiled successfully, but it's marked `compile_fail`", so the doctest is meaningful | covered |
| Order, accessors, Display | `ids::tests::order_accessors_and_display` (`ids.rs:59`) | covered |
| Transparent wire form | `ids::tests::transparent_wire_form` (`ids.rs:70`), including `u64::MAX` for both types | covered |
| Copy/Eq/Ord/Hash | derives `ids.rs:10-12`; `ids::tests::hash_and_copy_are_available` (`ids.rs:92`) | covered |
| Dpi Default | `dpi::tests::default_is_96_and_valid` (`dpi.rs:49`) | covered |
| Dpi Range edges | `dpi::tests::range_edges` (`dpi.rs:57`); mutations `MAX = 10_001` and `MIN = 0` both fail it | covered |
| Dpi Wire form (+ any u32 accepted) | `dpi::tests::wire_form` (`dpi.rs:73`, includes 0 and `u32::MAX`) | covered |
| Dpi non-integer rejected | `dpi::tests::non_integer_wire_values_are_rejected` (`dpi.rs:85`), all 6 inputs | covered |
| PageSize bounds inclusive | `page_size::tests::bounds_are_inclusive` (`page_size.rs:137`, bit comparison); mutation `px < MAX_PX` fails it and `bounds_reached_through_a_physical_unit` | covered |
| One step outside, axis named | `one_step_outside_the_bounds_names_the_axis` (`page_size.rs:152`) | covered |
| Width before height | `width_is_checked_before_height` (`page_size.rs:174`); mutation swapping the order fails it | covered |
| Bounds through a physical unit | `bounds_reached_through_a_physical_unit` (`page_size.rs:189`) | covered |
| A4 at 300 DPI | `a4_at_300_dpi` (`page_size.rs:210`) (see S1) | covered |
| A0 follows the DPI | `a0_follows_the_document_dpi` (`page_size.rs:224`), 351 -> `OutOfRange{Height}` near 16430.7 | covered |
| Zero, negative, non-finite | `zero_negative_and_non_finite_sides` (`page_size.rs:241`) | covered |
| Overflow and invalid DPI | `overflowing_conversion_and_invalid_dpi` (`page_size.rs:270`) | covered |
| Display names the axis, Error | `errors_display_the_axis` (`page_size.rs:283`); `impl Error` `page_size.rs:76`; `#[non_exhaustive]` `page_size.rs:41` | covered |
| Unit round trip (property) | `unit_round_trip` (`page_size.rs:311-325`), all `Unit::ALL`, DPI 1..=10 000, px 1..=16384, rel < 1e-6 | covered |
| PageSize wire form | `page_size::tests::wire_form` (`page_size.rs:298`), exact JSON | covered |
| No own unit math | `to_px`/`check_axis` call only `Length::to_px(f64::from(dpi.get()))` (`page_size.rs:85-105`) | covered |
| BlendMode exhaustive set | `index` with no wildcard (`blend.rs:93-111`), `all_is_exhaustive_and_distinct` (`blend.rs:115`); enum not `#[non_exhaustive]` | covered |
| BlendMode wire names | `wire_names` (`blend.rs:126`); mutation to `snake_case` fails it | covered |
| Unknown/mis-cased names | `unknown_or_miscased_names_are_rejected` (`blend.rs:135`) | covered |
| BlendMode default | `default_is_normal` (`blend.rs:142`) | covered |

#### document-serde

No requirement is tagged D1 (all are D3, D4 or D6). The D1 wire forms are covered above.

### 3. Code rules (AGENTS.md)

- `#![forbid(unsafe_code)]`: `crates/gafira-doc/src/lib.rs:6`. OK.
- No `#[allow]` in `crates/gafira-doc`: grep empty. OK.
- No `HashMap`/`HashSet` in code (only in the `clippy.toml` ban lines); tests use `BTreeSet`. `std::hash::DefaultHasher` in `ids.rs:98` is not a banned type. OK.
- English in code, comments and commits. OK.
- Conventional commits (`build(lint)`, `feat(doc)`, `docs(sdd)`); no `Co-Authored-By` or AI attribution in the range (grep rc 1). OK.

### 4. Size

`git diff --shortstat chore/lite-sdd-methodology..HEAD` = 13 files, +813 -1. Excluding `Cargo.lock` and `openspec` (the design's metric): 11 files, +766 -1. Forecast ~595, ratio 1.29.

| Prefix ending at | Measured (excl. lock/openspec) |
|---|---|
| fb679a5 | 4 |
| cb125d9 | 92 |
| 4e2377e | 200 |
| d8d0908 | 292 |
| 1989f90 | 620 |
| 0238e25 / 6e6e013 | 767 |

Every prefix passes all gates (section 1).

### CRITICAL

None.

### WARNING

1. **Size over the ~700 guideline.** 766 (813 with lock and openspec) is above the AGENTS.md "split when it exceeds ~700" rule and the design's ≤ ~700 planning target. It is below the design's 800 hard limit (`design.md`, "Measurement and re-cut rule" item 2), and the ratio 1.29 is just under the ×1.3 recompute trigger, so the design does not force a split. Owner decision. If the owner wants a split, the cut is **after 1989f90**: PR1 = fb679a5..1989f90 (620), PR2 = 0238e25 + 6e6e013 (147 + progress note). The alternative cut after d8d0908 gives 292 / 475. Both prefixes pass the gates. `page_size.rs` alone is +326 against a forecast of 171 (×1.9), which drives the overrun.
2. **Spec vs design on serde for error types.** `document-model/spec.md:24` says every public value type MUST implement `Serialize` and `Deserialize`. `Axis` (`page_size.rs:23`) and `PageSizeError` (`page_size.rs:42`) are public and implement neither. The code follows `design.md:211-213` (`Axis`: Copy, Eq; `PageSizeError`: Display + Error). `ConversionError` from geom would also need serde before `PageSizeError` could have it. Recommended fix: make the spec text exclude error types ("every public value type other than error types and `Document`"). Alternatively, add serde before D4 embeds `PageSizeError` in `ValidationError`.
3. **Stale tooling claims.** `openspec/config.yaml` (testing context) and `apply-progress.md` both say `cargo deny check` is not available. `cargo-deny 0.20.2` is installed, and `cargo deny --locked check` passes on all 7 commits (section 1). Update both files so later deliveries run the check.

### SUGGESTION

1. `a4_at_300_dpi` (`page_size.rs:218-219`) checks only the width for "stored lengths unchanged" and "unrounded values returned". Assert the height too.
2. `a0_follows_the_document_dpi` (`page_size.rs:229-230`) checks only `Ok(())` at 300 and 350 DPI. An approximate assertion (about 14043 and 16383.86 px through `to_px`) would pin the scenario's numbers.
3. `apply-progress.md` says "Six commits", but the D1 range has seven (the progress commit is the seventh). Cosmetic.

### Result

- Verdict: **PASS** for D1 (0 CRITICAL, 3 WARNING, 3 SUGGESTION).
- Next: owner decides on WARNING 1 (keep one PR or cut after 1989f90). Then **tdd-apply D2a**. Archive waits for D2a to D6.
