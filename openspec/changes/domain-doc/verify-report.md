# Verify report: domain-doc

## D1 (branch `feat/domain-doc-1`, range `369e3d5..6e6e013`, 7 commits)

Note: the hashes in this D1 section predate the rebase onto the `chore` commit. Mapping: fb679a5..d8d0908 is now d380fd8..f064203 (PR 1a); code trees are unchanged, only `.agents/agents/archive.md` differs.

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

## D2a (branch `feat/domain-doc-2a`, range `feat/domain-doc-1b` (9905723)..4bac4df, 3 commits)

Scope: `document-model` requirements "Placement and to_transform (D2a)", "ElementBase (D2a)" and the D2a part of "Value policy and equality (D2a, D2b)". No `document-serde` requirement is tagged D2a. Design rows: file map `design.md:95`, size `design.md:351`, commit plan `design.md:386`, test plan T2.1/T2.2 `design.md:411-412`.

### 1. Gates per commit

Each commit was extracted with `git archive` into its own directory `/tmp/gafira-verify-d2a-<sha>` and built with its own `CARGO_TARGET_DIR`. The first run shared one target dir, and cargo reused stale artifacts because `git archive` sets file mtimes to the commit time. That run was discarded. CI steps V5, V11 ("Domain crates forbid unsafe code") and V12 were taken from each commit's own `.github/workflows/ci.yml` (the file did not change in the range) and run under `bash --noprofile --norc -eo pipefail`.

| Commit | fmt | clippy `-D warnings` | test `--locked` | `cargo deny check` | V5 | V11 | V12 | gafira-doc lib tests |
|---|---|---|---|---|---|---|---|---|
| adcb117 `feat(doc): add Placement and its affine transform` | 0 | 0 | 0 (186 ok) | 0 | 0 | 0 | 0 | placement 9 |
| 38139cd `feat(doc): add ElementBase` | 0 | 0 | 0 (189 ok) | 0 | 0 | 0 | 0 | placement 9, element 3 |
| 4bac4df `docs(sdd): record domain-doc D2a apply progress` | 0 | 0 | 0 (189 ok) | 0 | 0 | 0 | 0 | placement 9, element 3 |

`cargo deny check`: advisories, bans, licenses and sources ok on all three. All gates pass on every prefix.

### 2. Requirements and scenarios

#### document-model: Placement and to_transform (D2a) (`spec.md:176`)

| Clause / scenario | Evidence | Status |
|---|---|---|
| Public fields `x,y,w,h,rotation_deg,skew: f64`, `flip: Flip {x,y}` | `placement.rs:7-34` | covered |
| `new` defaults (rotation 0, skew 0, flip false) | `new_sets_defaults` (`placement.rs:88`, compares bits); mutants rotation 1.0, skew -0.0 and flip.x true are killed | covered |
| Plain placement (`spec.md:180`) | `plain_placement` (`placement.rs:97`) | covered |
| Quarter turn clockwise (`spec.md:185`) | `quarter_turn_clockwise` (`placement.rs:104`); mutant `rotate(-θ)` killed | covered |
| Flip (`spec.md:190`) | `flip` (`placement.rs:111`); mutants "axes swapped" and "flip.y ignored" killed | covered |
| Skew 45 degrees (`spec.md:195`) | `skew_45_degrees` (`placement.rs:120`); mutants "no to_radians", "negated tan" and "y shear" killed | covered |
| Center is a fixed point (`spec.md:200`) | property `center_is_a_fixed_point` (`placement.rs:155`), skew drawn in `-89..89` | covered (see SUGGESTION 2) |
| Non-finite does not panic (`spec.md:205`) | `non_finite_does_not_panic` (`placement.rs:127`) | covered for "no panic"; the "non-finite coefficients" clause has no test (WARNING 2) |
| Values retained (`spec.md:210`) | `values_are_retained` (`placement.rs:136`, `to_bits`) | covered |
| `w`/`h` size the box and do not scale it | mutant adding `scale_non_uniform(w, 1)` killed by 4 tests | covered |
| Pivot on the box center | mutants "translate to (x,y)" and "drop translate(-hw,-hh)" killed by 5 tests | covered |
| **Composition order, right to left: translate(-half), mirror, shear, rotate, translate(center)** | Code is correct (`placement.rs:57-66`). The mutants **swap shear/mirror** and **swap rotate/shear** both SURVIVE: every reference test exercises one feature at a time | **missing** (CRITICAL 1) |
| Uses only `gafira_geom::Affine` | `placement.rs:3` | covered |

#### document-model: ElementBase (D2a) (`spec.md:215`)

| Clause / scenario | Evidence | Status |
|---|---|---|
| Fields `id, name: Option<String>, placement, opacity: f32, blend, visible, locked`, all bare | `element.rs:9-25` | covered |
| Defaults (`spec.md:219`) | `defaults` (`element.rs:47`, opacity compared by bits). Mutants on opacity (0.9999999), blend, visible, locked, name and placement are all killed | covered |
| Colors are `gafira_color::Color` bare values | no color field in D2a (first use `ShapeNode.fill`, D2b) | n/a in D2a |

#### document-model: Value policy and equality (D2a, D2b) (`spec.md:295`)

| Scenario | Evidence | Status |
|---|---|---|
| Out-of-range values are representable (`spec.md:299`) | `out_of_range_values_are_representable` (`element.rs:59`): opacity 1.5 by bits, `Dpi::new(0).get() == 0`, NaN `x` checked only with `is_nan()` | covered (SUGGESTION 1) |
| IEEE equality: zeros (`spec.md:304`) | `ieee_equality` (`placement.rs:144`): `==` holds and bits differ | covered |
| IEEE equality: "NaN node is not equal to itself" | `ieee_equality` (`Placement`) and `nan_node_is_not_equal_to_itself` (`element.rs:68`, on `ElementBase`) | partial by design: `Node` is D2b. See "Deviations" |
| Constructors infallible, no clamp or normalize | `Placement::new` is `const fn` (`placement.rs:38`), `ElementBase::new` (`element.rs:29`); neither returns `Result` | covered |

#### document-serde

No requirement is tagged D2a.

#### Deviations claimed in apply-progress

- **No serde derives on `Placement`, `Flip` or `ElementBase` until D3.** The wire requirements in `document-serde` are all tagged D3, and the design budgets "serde attributes across modules" in D3 (`design.md:356`, commit plan `design.md:388`). That timing is allowed. The D1 requirement "Purity and dependencies" (`spec.md:24`), however, says without condition that every public value type MUST implement `Serialize` and `Deserialize`. That clause is false at every D2a prefix (WARNING 1). No functionality is missing: nothing in D2a consumes serde for these types.
- **NaN-equality scenario tested on `ElementBase`.** The requirement is tagged (D2a, D2b) and `Node` only exists in D2b, so the D2a part is satisfied. The `Node` part stays open and must be re-checked in D2b. This is a carried obligation, not a gap.

### 3. Code rules

- `#![forbid(unsafe_code)]` is still present (`lib.rs`), and V11 passes.
- No `#[allow]`, `unsafe`, `HashMap`, `HashSet`, `unwrap` or `expect` in the added lines.
- Added code and comments are ASCII-only English.
- Commit subjects are conventional (`feat(doc)`, `feat(doc)`, `docs(sdd)`), and no message has an AI attribution or `Co-Authored-By` line.
- No `proptest-regressions` file is tracked or left in the tree.
- New modules are exported from `lib.rs` in the commit that adds them, per the design's commit rule (`design.md:393`).

### 4. Size

`git diff --shortstat feat/domain-doc-1b..HEAD` gives 4 files changed, 271 insertions(+), which matches the reported figure. Without `openspec` it is 3 files, 248 insertions: `placement.rs` +172, `element.rs` +72, `lib.rs` +4. The forecast was ~335 (`design.md:351-352`), so the delivery is under it and far below ~700.

### CRITICAL

1. **The composition order of `to_transform` has no test.** `spec.md:178` requires the transform to MUST equal, right to left, translate(center) · rotate · shear · mirror · translate(-half). Design T2.1 (`design.md:411`) requires the mutation "swap scale and skew" to fail a reference. Two mutants of `placement.rs:62-66` survive the whole suite: swapping `shear` and `mirror`, and swapping `rotate` and `shear`. The scenarios at `spec.md:185-198` each set one feature, and the property at `placement.rs:155` only checks the center, which every ordering fixes. Fix: add one combined reference test. A scratch test checked in the /tmp copy (not committed) kills both mutants: `x=0, y=0, w=2, h=2, rotation_deg=90, skew=45, flip.x=true`, local `(2,2)` maps to `(0,1)` within `1e-12`.

### WARNING

1. **The D1 serde clause is unconditional while D2a types lack serde.** `spec.md:24` says every public value type except `Axis`, `PageSizeError` and `Document` MUST implement `Serialize`/`Deserialize`. `Placement`/`Flip` (`placement.rs:6,18`) and `ElementBase` (`element.rs:9`) implement neither until D3. The design schedules the attributes for D3 (`design.md:356,388`), so this is planned, but the spec text does not say so. Either add "(serde derives land in D3 for D2a/D2b types)" to the requirement, or treat it as a D3 verify item. D3 verify must confirm all three derive serde.
2. **"Non-finite fields give non-finite coefficients" has no test.** `spec.md:178` says so, and the doc comment at `placement.rs:53-54` says so too, but `non_finite_does_not_panic` (`placement.rs:127-133`) discards the result. A mutant that returned a finite fallback, such as identity on non-finite input, would survive. Assert `!a.to_transform().as_coeffs().iter().all(|c| c.is_finite())` (or the geom accessor equivalent) for both cases.
3. **Stale hashes in documents (doc only).** `apply-progress.md:5` cites PR 1a as `fb679a5..d8d0908`, but after the rebase it is `d380fd8..f064203`. The D1 section of this report (section 4 table and WARNING 1) also cites pre-rebase hashes (fb679a5, cb125d9, 4e2377e, d8d0908, 1989f90, 0238e25, 6e6e013), none of which is reachable from the current branches. Update `apply-progress.md:5`, and add a note under the D1 report that those hashes predate the rebase onto the `chore` commit.

### SUGGESTION

1. In `out_of_range_values_are_representable` (`element.rs:59-65`), the scenario says the values "read back bit-identical", but the NaN `x` is checked with `is_nan()`. Compare `e.placement.x.to_bits()` with `f64::NAN.to_bits()`.
2. `center_is_a_fixed_point` (`placement.rs:158`) draws skew in `-89..89`, while the scenario says `(-90, 90)`. Widen it to about `-89.999..89.999`, or note the bound. The relative tolerance with `max(1.0)` is fine.
3. The D2a section of `apply-progress.md` does not record the base hash, a "Status" line or a statement that each commit passed the gates alone (the D1 section has all three). It also does not mention that `cargo deny check` and V5/V11/V12 were not run (verify ran them, all green).

### Result

- Verdict: **FAIL** for D2a (1 CRITICAL, 3 WARNING, 3 SUGGESTION). All gates pass on every commit, and every D2a scenario has a passing test. The failure is the requirement-level composition-order clause, which no test proves, while the design-mandated mutation survives.
- Next: **tdd-apply D2a** to fix CRITICAL 1 (one combined reference test, RED shown by the swap mutants), and preferably WARNING 2 and 3 in the same pass. Then re-verify D2a, then D2b. Archive waits for D2b to D6.
