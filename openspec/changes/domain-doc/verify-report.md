# Verify report: domain-doc

**Commit references.** Commits are cited by subject and branch range (for example `feat/domain-doc-1b..feat/domain-doc-2a`, "feat(doc): add ElementBase") instead of hashes, because a rebase changes hashes. Hashes that remain in the findings text quote what an earlier verify run saw and are history, not current references.


## D1 (branches `feat/domain-doc-1a` and `feat/domain-doc-1b`, range `chore/lite-sdd-methodology..feat/domain-doc-1b`, 7 commits)

Note: the D1 commits were rebased onto the `chore` commit. Commits are cited by subject and branch range (PR 1a = `chore/lite-sdd-methodology..feat/domain-doc-1a`, PR 1b = `feat/domain-doc-1a..feat/domain-doc-1b`). The old and new D1 commits have identical code trees: seven non-code files differ (`.agents/agents/{archive,coordinator,tdd-apply}.md`, `.agents/skills/skill-registry/{SKILL.md,refresh.sh}`, `AGENTS.md`, `CLAUDE.md`), and nothing under `crates/`, `Cargo.*` or `.github/`.

**Verdict: PASS** (D1 only). CRITICAL 0, WARNING 3, SUGGESTION 3. The change is not complete (D2a to D6 remain), so `next` = tdd-apply for D2a.

Verified 2026-10-24 on toolchain 1.93.1, clippy 0.1.93, cargo-deny 0.20.2, git 2.34.1. Each commit was checked out with `git worktree add --detach` under `/tmp`; negative runs used a scratch worktree of HEAD, removed afterwards. The repository tree was not modified.

### 1. Gates per commit

| Subject | fmt | clippy `--workspace --all-targets --locked -D warnings` | test `--workspace --locked` (passed) | V5 | V11 | V12 | ARQ-07 | `cargo deny --locked check` |
|---|---|---|---|---|---|---|---|---|
| build(lint): ban HashMap and HashSet in domain clippy policy | 0 | 0 | 0 (154) | 0 | 0 | 0 (old body) | 0 | 0 |
| feat(doc): add gafira-doc crate skeleton | 0 | 0 | 0 (154) | 0 | 0 | 0 (hardened, 3 copies) | 0 | 0 |
| feat(doc): add NodeId and PageId | 0 | 0 | 0 (158) | 0 | 0 | 0 | 0 | 0 |
| feat(doc): add Dpi | 0 | 0 | 0 (162) | 0 | 0 | 0 | 0 | 0 |
| feat(doc): add PageSize with pixel limits | 0 | 0 | 0 (173) | 0 | 0 | 0 | 0 | 0 |
| feat(doc): add BlendMode | 0 | 0 | 0 (177) | 0 | 0 | 0 | 0 | 0 |
| docs(sdd): record domain-doc D1 apply progress | 0 | 0 | 0 (177) | 0 | 0 | 0 | 0 | 0 |

The CI steps were extracted from each commit's own `.github/workflows/ci.yml` with a YAML parser and run under `bash --noprofile --norc -eo pipefail`. The clippy logs show `Checking gafira-doc` from `feat(doc): add gafira-doc crate skeleton` on, so no stale cache was involved. Total passed includes the `compile_fail` doctest. "Hardened V12 timing" holds: the `build(lint)` commit runs the old 3-line body (no `gafira-doc` V5 row yet), and the crate skeleton commit runs the hardened body with the V5 row.

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
| H-N3 ban removed everywhere | rc 1 "lacks the std::collections::HashMap ban under disallowed-types"; old V12 (body of the `build(lint)` commit) rc 0 on the same tree (RED) | covered |
| H-N5 commented ban | rc 1 same message; old V12 rc 0 | covered |
| H-N6 ban under `disallowed-methods` | both entries moved into `disallowed-methods` in all copies: rc 1 same message; old V12 rc 0 | covered |
| Hardened V12 timing | see section 1 (`build(lint)` commit old body rc 0, crate skeleton commit hardened rc 0) | covered |
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
| No kurbo, no new package | `grep -r kurbo crates/gafira-doc` empty; `git diff chore/lite-sdd-methodology..HEAD -- Cargo.lock` adds only `name = "gafira-doc"` | covered |
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
| `build(lint): ban HashMap and HashSet in domain clippy policy` | 4 |
| `feat(doc): add gafira-doc crate skeleton` | 92 |
| `feat(doc): add NodeId and PageId` | 200 |
| `feat(doc): add Dpi` | 292 |
| `feat(doc): add PageSize with pixel limits` | 620 |
| `feat(doc): add BlendMode` / `docs(sdd): record domain-doc D1 apply progress` | 767 |

Every prefix passes all gates (section 1).

### CRITICAL

None.

### WARNING

1. **Size over the ~700 guideline.** 766 (813 with lock and openspec) is above the AGENTS.md "split when it exceeds ~700" rule and the design's ≤ ~700 planning target. It is below the design's 800 hard limit (`design.md`, "Measurement and re-cut rule" item 2), and the ratio 1.29 is just under the ×1.3 recompute trigger, so the design does not force a split. Owner decision. If the owner wants a split, the cut is **after `feat(doc): add PageSize with pixel limits`**: PR1 = up to that commit (620), PR2 = `feat(doc): add BlendMode` + the progress note (147 + progress note). The alternative cut after `feat(doc): add Dpi` gives 292 / 475. Both prefixes pass the gates. `page_size.rs` alone is +326 against a forecast of 171 (×1.9), which drives the overrun.
2. **Spec vs design on serde for error types.** `document-model/spec.md:24` says every public value type MUST implement `Serialize` and `Deserialize`. `Axis` (`page_size.rs:23`) and `PageSizeError` (`page_size.rs:42`) are public and implement neither. The code follows `design.md:211-213` (`Axis`: Copy, Eq; `PageSizeError`: Display + Error). `ConversionError` from geom would also need serde before `PageSizeError` could have it. Recommended fix: make the spec text exclude error types ("every public value type other than error types and `Document`"). Alternatively, add serde before D4 embeds `PageSizeError` in `ValidationError`.
3. **Stale tooling claims.** `openspec/config.yaml` (testing context) and `apply-progress.md` both say `cargo deny check` is not available. `cargo-deny 0.20.2` is installed, and `cargo deny --locked check` passes on all 7 commits (section 1). Update both files so later deliveries run the check.

### SUGGESTION

1. `a4_at_300_dpi` (`page_size.rs:218-219`) checks only the width for "stored lengths unchanged" and "unrounded values returned". Assert the height too.
2. `a0_follows_the_document_dpi` (`page_size.rs:229-230`) checks only `Ok(())` at 300 and 350 DPI. An approximate assertion (about 14043 and 16383.86 px through `to_px`) would pin the scenario's numbers.
3. `apply-progress.md` says "Six commits", but the D1 range has seven (the progress commit is the seventh). Cosmetic.

### Result

- Verdict: **PASS** for D1 (0 CRITICAL, 3 WARNING, 3 SUGGESTION).
- Next: owner decides on WARNING 1 (keep one PR or cut after `feat(doc): add PageSize with pixel limits`). Then **tdd-apply D2a**. Archive waits for D2a to D6.

## D2a (branch `feat/domain-doc-2a`, range `feat/domain-doc-1b..feat/domain-doc-2a`, first 3 commits up to `docs(sdd): record domain-doc D2a apply progress`)

Scope: `document-model` requirements "Placement and to_transform (D2a)", "ElementBase (D2a)" and the D2a part of "Value policy and equality (D2a, D2b)". No `document-serde` requirement is tagged D2a. Design rows: file map `design.md:95`, size `design.md:351`, commit plan `design.md:386`, test plan T2.1/T2.2 `design.md:411-412`.

### 1. Gates per commit

Each commit was extracted with `git archive` into its own directory `/tmp/gafira-verify-d2a-<sha>` and built with its own `CARGO_TARGET_DIR`. The first run shared one target dir, and cargo reused stale artifacts because `git archive` sets file mtimes to the commit time. That run was discarded. CI steps V5, V11 ("Domain crates forbid unsafe code") and V12 were taken from each commit's own `.github/workflows/ci.yml` (the file did not change in the range) and run under `bash --noprofile --norc -eo pipefail`.

| Commit | fmt | clippy `-D warnings` | test `--locked` | `cargo deny check` | V5 | V11 | V12 | gafira-doc lib tests |
|---|---|---|---|---|---|---|---|---|
| `feat(doc): add Placement and its affine transform` | 0 | 0 | 0 (186 ok) | 0 | 0 | 0 | 0 | placement 9 |
| `feat(doc): add ElementBase` | 0 | 0 | 0 (189 ok) | 0 | 0 | 0 | 0 | placement 9, element 3 |
| `docs(sdd): record domain-doc D2a apply progress` | 0 | 0 | 0 (189 ok) | 0 | 0 | 0 | 0 | placement 9, element 3 |

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

## D2a re-verify (the 4 fix commits from `test(doc): cover combined rotation, skew and flip order` to `docs(sdd): record domain-doc D2a verify fixes`; delivery `feat/domain-doc-1b..feat/domain-doc-2a`)

**Verdict: PASS** (D2a). CRITICAL 0, WARNING 0, SUGGESTION 3. The change is not complete (D2b to D6 remain), so `next` = tdd-apply for D2b.

Verified 2026-10-24 on rustc 1.93.1 and cargo-deny 0.20.2. Each commit was extracted with `git archive` into `/tmp/gafira-reverify-d2a-<sha>`, each with its own `CARGO_TARGET_DIR`. V5, V11 ("Domain crates forbid unsafe code") and V12 were extracted from each commit's own `.github/workflows/ci.yml` (unchanged since the tip of `feat/domain-doc-1b`) and run under `bash --noprofile --norc -eo pipefail`. Mutations ran on a separate copy of the `docs(sdd): record domain-doc D2a verify fixes` commit. The repository tree was not modified.

### 1. Gates per commit

| Commit | fmt | clippy `-D warnings` | test `--locked` | `cargo deny check` | V5 | V11 | V12 | placement/element tests |
|---|---|---|---|---|---|---|---|---|
| `test(doc): cover combined rotation, skew and flip order` | 0 | 0 | 0 (190 ok) | 0 | 0 | 0 | 0 | 13 |
| `test(doc): assert non-finite coefficients and compare NaN by bits` | 0 | 0 | 0 (190 ok) | 0 | 0 | 0 | 0 | 13 |
| `test(doc): widen the skew property to the open interval` | 0 | 0 | 0 (190 ok) | 0 | 0 | 0 | 0 | 13 |
| `docs(sdd): record domain-doc D2a verify fixes` | 0 | 0 | 0 (190 ok) | 0 | 0 | 0 | 0 | 13 |

`cargo deny check` reports advisories, bans, licenses and sources ok on all four. The earlier prefixes (`feat(doc): add Placement and its affine transform`, `feat(doc): add ElementBase` and `docs(sdd): record domain-doc D2a apply progress`) were green in the first D2a run, and no file they touched changed outside the test modules and docs.

### 2. Previous items

| Item | Fix | Evidence | Status |
|---|---|---|---|
| CRITICAL 1: composition order | `combined_rotation_skew_and_flip` (`placement.rs:127-134`), rotation 90, skew 45, flip.x, local (2,2) to (0,1) | Hand check matches `spec.md:178`: centered (1,1), mirror (-1,1), shear (0,1), rotate (-1,0), translate (0,1). Mutant "swap shear/mirror" fails this test only (34 pass, 1 fail). Mutant "swap rotate/shear" fails this test only. Design T2.1 mutation now fails a reference. | closed |
| WARNING 1: serde clause | Sentence appended to "Purity and dependencies" (`specs/document-model/spec.md:24`): serde for `Placement`, `Flip`, `ElementBase` and the other D2 types lands in D3 | Matches the design commit plan (`design.md:388`, D3 "define the document wire format"). D3 verify must still confirm the derives. | closed (D3 carry) |
| WARNING 2: non-finite coefficients | `non_finite_gives_non_finite_coefficients` (`placement.rs:137-143`) replaces `non_finite_does_not_panic` and asserts `any(!is_finite)` for NaN `x` and `+inf` rotation | Mutant "return `Affine::IDENTITY` when any field is non-finite" fails this test only. The test still calls `to_transform()` on both inputs, so the "does not panic" scenario (`spec.md:205`) stays covered. | closed |
| WARNING 3: stale hashes | `apply-progress.md` now cites PR 1a by branch range. A note at `verify-report.md:5` says the D1 hashes predate the rebase. | each old/new D1 commit pair differs only in non-code files (see the corrected note at line 5), so the "code trees unchanged" claim holds. | closed (SUGGESTION 1) |
| SUGGESTION 1: NaN by bits | `element.rs:63` compares `to_bits()` with `f64::NAN.to_bits()`. `element.rs:71-72` adds an is-NaN check and a bits-equal check for the clone. | matches `spec.md:301` "bit-identical" | closed |
| SUGGESTION 2: skew interval | `skew in (-90.0f64).next_up()..90.0` (`placement.rs:168`) | That range is exactly the open interval (-90, 90) of `spec.md:201`: the start is the next f64 above -90, and the end is excluded. `f64::next_up` is stable on 1.93.1, and clippy is clean. It passes with `PROPTEST_CASES=20000`. | closed |
| SUGGESTION 3: apply-progress D2a header | Base hash, a Status line, the per-commit gates statement and the deny/V5/V11/V12 note added (`apply-progress.md:44-46`) | read back | closed |

### 3. Regression check, other D2a requirements

All 13 `placement`/`element` tests pass on every commit. The non-test code in `placement.rs:1-68` and `element.rs:1-40` is byte-identical to the `docs(sdd): record domain-doc D2a apply progress` commit (the diff from it to the `docs(sdd): record domain-doc D2a verify fixes` commit, `-- crates` touches only `mod tests`), so every clause marked covered in the first D2a run still holds. Carried obligations are unchanged: the NaN self-inequality on `Node` must be checked in D2b (`spec.md:304`), and the serde derives on the D2 types in D3.

### 4. Code rules

- Added lines in `crates/` contain no `#[allow]`, `unsafe`, `HashMap`, `HashSet`, `unwrap` or `expect`, and they are ASCII English.
- No `proptest-regressions` file is tracked, left in the tree or created in the mutation copy.
- The four subjects are conventional (`test(doc)` x3, `docs(sdd)`), and no message has a `Co-Authored-By` or AI attribution line.

### 5. Size

`git diff --shortstat feat/domain-doc-1b..HEAD` gives 6 files changed, 392 insertions(+), 2 deletions(-), which matches the reported figure. Without `openspec` it is 3 files, 260 insertions: `placement.rs` +182, `element.rs` +74, `lib.rs` +4. That is under the ~335 forecast and far below ~700.

### CRITICAL

None.

### WARNING

None.

### SUGGESTION

1. The note at `verify-report.md:5` maps only the PR 1a range. The other pre-rebase hashes in the D1 section (`verify-report.md:19,117,128,141`: cb125d9, 4e2377e, 1989f90, 0238e25, 6e6e013) have no mapping. They map to cbdf8cd, bcddca2, 0b33754, c0469c7 and 0fa1185, which could be listed in the note.
2. `apply-progress.md:67` still reports the pre-fix size (3 files, 248 insertions). It is now 260 without `openspec` (392 total).
3. The D2a commit table in `apply-progress.md` lists only the two feature commits. The fix commits ad5a36a, 645a284, ce2d018 and 284bdc4 appear under "follow-ups" by test name, without hashes.

### Result

- Verdict: **PASS** for D2a (0 CRITICAL, 0 WARNING, 3 SUGGESTION). All gates pass on every commit, and all three previous mutants are now killed by the intended tests. Every D2a requirement and scenario is covered.
- Next: **tdd-apply D2b**, which must cover the `Node` part of "IEEE equality". D3 verify must confirm serde on `Placement`, `Flip` and `ElementBase`. Archive waits for D2b to D6.

## D2b (branch `feat/domain-doc-2b`, range `feat/domain-doc-2a..feat/domain-doc-2b`, 5 commits up to `docs(sdd): record domain-doc D2b apply progress`)

**Verdict: FAIL** (D2b). CRITICAL 3, WARNING 3, SUGGESTION 3. `next` = tdd-apply for D2b (fix the CRITICAL items), then re-verify D2b.

Scope: `document-model` requirements "The single id counter (D2b)", "NodeKind (D2b)", "Node, Page and the arena (D2b)", "Document root (D2b; `from_parts` D4)", "Structural sharing (D2b; the edit scenario D5)" and the D2b part of "Value policy and equality (D2a, D2b)". No `document-serde` requirement is tagged D2b; that spec only names the D2b helper `test_document` (`specs/document-serde/spec.md:7`). Design rows: gate table `design.md:66-71`, file map `design.md:96`, signatures `design.md:197,263-265`, size `design.md:353-355`, commit plan `design.md:387`, test plan T2.3-2.5 `design.md:413`.

Verified 2026-10-24 on rustc 1.93.1, cargo-deny 0.20.2, git 2.34.1. Each commit was extracted with `git archive` into its own directory under `/tmp`, each with its own `CARGO_TARGET_DIR`. V5, V11 ("Domain crates forbid unsafe code") and V12 were extracted with a YAML parser from each commit's own `.github/workflows/ci.yml` (unchanged in the range) and run under `bash --noprofile --norc -eo pipefail`. Mutations ran on a separate copy of the `docs(sdd): record domain-doc D2b apply progress` commit. The repository tree was not modified.

### 1. Gates per commit

| Commit | fmt | clippy `--workspace --all-targets --locked -D warnings` | test `--workspace --locked` (passed) | `cargo deny check` | V5 | V11 | V12 | gafira-doc lib tests |
|---|---|---|---|---|---|---|---|---|
| `feat(doc): add node kinds` | 0 | 0 | 0 (195) | 0 | 0 | 0 | 0 | 40 |
| `feat(doc): add Page and node arena` | 0 | 0 | 0 (200) | 0 | 0 | 0 | 0 | 45 |
| `feat(doc): add Document root` | 0 | 0 | 0 (205) | 0 | 0 | 0 | 0 | 50 |
| `docs(sdd): fix D2a size, fix-commit hashes and D1 report hashes` | 0 | 0 | 0 (205) | 0 | 0 | 0 | 0 | 50 |
| `docs(sdd): record domain-doc D2b apply progress` | 0 | 0 | 0 (205) | 0 | 0 | 0 | 0 | 50 |

`cargo deny check`: advisories, bans, licenses and sources ok on all five. Each clippy log shows `Checking gafira-doc` (no stale cache). Every prefix passes every gate.

### 2. Requirements and scenarios

#### The single id counter (D2b) (`specs/document-model/spec.md:65`)

| Clause / scenario | Evidence | Status |
|---|---|---|
| One `next_id: u64` shared by nodes and pages | `document.rs:16`; read by `next_id()` `document.rs:31` | covered |
| `Default`: schema 1, next_id 1, dpi 96, no pages / "Default document" | `Default` `document.rs:60-69`; `default_document` `document.rs:108`. Mutants on each default (next_id 0, dpi 72, schema 2, `SCHEMA_VERSION = 2`) killed | covered |
| Ids not derived from content | no id derivation exists in the code | covered (by review) |
| `IdSpaceExhausted` public, `Copy`, `Eq`, `Display`, `Error` | `ids.rs:55-64`, export `lib.rs:22`; `exhaustion_error_displays_and_is_an_error` `ids.rs:118` checks `Display` and `Error` (removing the `Error` impl fails to compile). Removing `Copy` or `Eq` from the derive survives | **failing** (CRITICAL 2) |
| `#[cfg(test)]` allocator / "Private allocator before DocMut" | `alloc_id` `document.rs:52-58`, gated `#[cfg(test)] pub(crate)` as in `design.md:66,263`; `allocator_increments_then_exhausts` `document.rs:118`. Mutants killed: `saturating_add`, `checked_add(2)`, return the incremented value, `wrapping_add` with reset to 0 on overflow, `next_id()` returning `min(6)`, `test_document` ignoring `next_id` | covered |

#### NodeKind (D2b) (`spec.md:224`)

| Clause / scenario | Evidence | Status |
|---|---|---|
| Closed enum `Group`, `Shape`, `Extension`; `GroupNode` without data; `ShapeGeometry` `Rect`/`Ellipse`; `ExtensionNode { type_id, schema_version: u32, payload: serde_json::Value }` | `node.rs:8-58`, exports `lib.rs:23` | covered |
| "Variant sets" | `variant_sets_are_exhaustive` `node.rs:97` matches both enums without wildcard arms | covered |
| "Shape without fill" | `shape_without_fill` `node.rs:123`; `ShapeNode::new` `node.rs:31` (mutant `fill: Some(..)` fails it) | covered |
| Extension version independent of the document | `extension_keeps_its_own_version_and_payload` `node.rs:142` | covered |

#### Node, Page and the arena (D2b) (`spec.md:238`)

| Clause / scenario | Evidence | Status |
|---|---|---|
| `Node { base, kind, children }` public; `Page` public fields; `nodes: BTreeMap<NodeId, Arc<Node>>` | `node.rs:65-73`, `page.rs:13-29` | covered |
| "Page defaults" | `page_defaults` `page.rs:74` (mutant `hidden: true` killed) | covered |
| "Deterministic iteration" | `iteration_is_in_ascending_id_order` `page.rs:86` | covered |
| "Lookup stays inside the page"; `Page::node(id)` MUST return the node of that page | `lookup_stays_inside_the_page` `page.rs:96`. Every test page holds at most one node, so `Page::node` ignoring `id` and returning the first node of the arena survives the whole suite | **failing** (CRITICAL 3) |
| "Page flags do not cascade" | `page_flags_do_not_cascade` `page.rs:106` | covered |
| Structural violations representable | all `Page`/`Node` fields public and unchecked (`page.rs:14-29`) | covered (by review; see SUGGESTION 2) |

#### Document root (D2b; `from_parts` D4) (`spec.md:262`)

| Clause / scenario | Evidence | Status |
|---|---|---|
| Private fields, `SCHEMA_VERSION = 1` | `document.rs:14-23` | covered |
| `Clone`, `Debug`, `PartialEq` | `document.rs:13`. Removing `Clone` (manual deep or lossy clone) is killed by `clone_shares_everything`. Removing `PartialEq` or `Debug` from the derive survives | **failing** (CRITICAL 2) |
| `schema_version()`, `next_id()` | `document.rs:26,31`; killed mutants above | covered |
| `dpi()` | `document.rs:36`. Only the default DPI is ever read: `dpi()` returning `Dpi::default()`, and `test_document` ignoring its `dpi` argument, both survive | **failing** (CRITICAL 1) |
| `pages()`, `page(PageId)` | `document.rs:41,46`; `page_lookup_and_order` `document.rs:128` (built with `test_document`). Mutants killed: `!=` in the predicate, `first()`, always `None`, `test_document` dropping or reversing pages | covered |
| No `&mut` reachable from `&Document` | every accessor returns `&`/`Copy`; `Page::node` returns `&Node` | covered (by review) |
| "Accessors and page order" | tagged `(D4, unit)` in its title (`spec.md:266`); the purpose line (`spec.md:7`) says scenarios that need `from_parts` carry the later delivery. The deferral is allowed by the tags. `page_lookup_and_order` already checks order 3, 1, 2, lookup and unknown id with `test_document`; D4 verify must see the `from_parts` version | deferred (D4), allowed |
| "from_parts validates" (D4), "Clone is independent" (D5) | later deliveries | n/a in D2b |

#### Structural sharing (D2b) (`spec.md:281`)

| Clause / scenario | Evidence | Status |
|---|---|---|
| "Clone shares everything" (2 pages, 3 nodes in the first, `Arc::ptr_eq`) | `clone_shares_everything` `document.rs:141`; manual deep page clone and lossy clone mutants killed. `cloning_a_page_shares_its_nodes` `page.rs:117` for the `Page` clause | covered |
| "Edit touches one path" | D5 | n/a in D2b |

#### Value policy and equality, D2b part (`spec.md:295`)

| Clause / scenario | Evidence | Status |
|---|---|---|
| "IEEE equality": NaN node not equal to itself (carried from D2a) | `nan_placement_node_is_not_equal_to_itself` `node.rs:154`: `assert_ne!(n, n.clone())` plus bit equality of `x`. Removing `PartialEq` from `Node` fails to compile; a bitwise equality would fail the `assert_ne!` | covered (carry closed) |
| Constructors infallible, no clamping | `Node::new`, `ShapeNode::new`, `Page::new` (`node.rs:77`, `node.rs:31`, `page.rs:33`) store their arguments unchanged | covered |

#### Design checks

- `#[cfg(test)]` gating: `alloc_id` `#[cfg(test)] pub(crate)` (`document.rs:52-53`), `test_document` `#[cfg(test)] pub(crate)` (`document.rs:72-73`), and the `IdSpaceExhausted` import is gated as well (`document.rs:5`). Both land in `feat(doc): add Document root` together with "Clone shares everything", as `design.md:67,387` require. Clippy `--all-targets -D warnings` is clean on every prefix, so neither item is dead code.
- Commit order matches `design.md:387`.

#### Mutation summary (copy of the `docs(sdd): record domain-doc D2b apply progress` commit, `cargo test -p gafira-doc --locked`)

27 mutants: 20 killed, 7 survived. The survivors are `dpi()` returning `Dpi::default()` (M01), `test_document` ignoring `dpi` (M02), `IdSpaceExhausted` without `Copy` (M16) or without `Eq` (M17), `Document` without `PartialEq` (M18) or without `Debug` (M19), and `Page::node` ignoring `id` (M22). Each survivor ran all 50 lib tests green.

### 3. Code rules

- Added lines in `crates/` contain no `#[allow]`, `unsafe`, `HashMap` or `HashSet`; the arena is a `BTreeMap` (`page.rs:3,28`). `#![forbid(unsafe_code)]` stays at `lib.rs:6`.
- Code, comments and commit messages are in English and ASCII. No `proptest-regressions` file is tracked or left in the tree or the mutation copy.
- The five subjects are conventional (`feat(doc)` x3, `docs(sdd)` x2), with no `Co-Authored-By` or AI attribution line.

### 4. Doc fixes in `docs(sdd): fix D2a size, fix-commit hashes and D1 report hashes`

- Correct: the D1 section of this report (`verify-report.md:3,5,15-21,113-118,128,141`) now cites 7f2a523, 7e7cd1f, 9c42c69, 2686a87, 44679cd, dac2b1f, dc7f7a9, all on `feat/domain-doc-1b` and matched by subject. `apply-progress.md:53` cites 2521198, 618a83e, 036bbca, 547e6ba and a5c16fe, all on `feat/domain-doc-2a`. The D2a size `apply-progress.md:69` (3 files, 260 insertions) matches `git diff --shortstat feat/domain-doc-1b..feat/domain-doc-2a -- . ':!openspec'`.
- Not current: see WARNING 2.

### 5. Size

`git diff --shortstat feat/domain-doc-2a..HEAD` gives 7 files changed, 524 insertions(+), 22 deletions(-). Without `openspec`: 5 files, 472 insertions(+), 1 deletion(-). Both match the reported figures. That is ratio 1.02 against the ~465 forecast, under ~700.

### CRITICAL

1. **`Document::dpi()` and the `dpi` argument of `test_document` are untested.** Every test builds with `Dpi::default()` and only `default_document` reads `dpi()` (`document.rs:108`). Mutants `dpi()` -> `Dpi::default()` (`document.rs:36-38`) and `test_document` storing `Dpi::default()` instead of its argument (`document.rs:73-79`) both survive. `test_document` is a permanent helper that D3 relies on for exact bytes, so a dropped field would hide later failures. Fix: build one document with `test_document(_, Dpi::new(300), _)` and assert `dpi().get() == 300`.
2. **Trait clauses of the D2b requirements are unproven.** `spec.md:67` requires `IdSpaceExhausted` to be `Copy` and `Eq`, and `spec.md:264` requires `Document` to be `Debug` and `PartialEq`. Removing `Copy` or `Eq` from `ids.rs:55`, or `PartialEq` or `Debug` from `document.rs:13`, compiles and passes every test. In `exhaustion_error_displays_and_is_an_error` (`ids.rs:118-124`), `let copy = IdSpaceExhausted;` builds a second value and does not copy anything. Fix: one static assertion, for example `fn assert_traits<T: Copy + Eq + std::fmt::Display + std::error::Error>() {}` called for `IdSpaceExhausted`, and a similar `Clone + Debug + PartialEq` check (or a real `assert_eq!`/`format!("{:?}")` on documents) for `Document`. This applies to 025aeb0, which was not written test first.
3. **`Page::node` ignoring its `id` survives.** `spec.md:240` says `Page::node(id)` MUST return the node of that page, but every test page holds at most one node (`page.rs:96-114`, `document.rs:88-105` never calls `node`). Replacing `page.rs:47` with "first node of the arena" passes all tests. The scenario at `spec.md:252` is covered as written, but the clause is not. This is in c14f347, outside the commit that skipped test-first, and is still a surviving mutant on a MUST clause. Fix: insert two or more nodes into the second page in `lookup_stays_inside_the_page` and assert the returned id, plus a `None` for an id missing from a non-empty page.

### WARNING

1. **025aeb0 was not test first (declared).** `apply-progress.md` says the tests and code were written together and RED was shown afterwards with a `wrapping_add` mutant. That mutant is killed here, but CRITICAL 1 and 2 are the gaps that test-first would have exposed. The tdd-apply fix should show RED for each new assertion with the surviving mutants listed above.
2. **Stale hashes left after 931d94b.** These cite pre-rebase commits that are not reachable from any current branch:
   - `apply-progress.md:5` gives PR 1a as `d380fd8..f064203`. It is now `7f2a523..2686a87` (`feat/domain-doc-1a`), as `verify-report.md:5` already says.
   - `apply-progress.md:44` and `verify-report.md:143,239,243` give the D2a base as `feat/domain-doc-1b` 9905723. The branch is now at 9957190.
   - The D2a sections of this report (`verify-report.md:153-155,249-254,263,270`) cite adcb117, 38139cd, 4bac4df, ad5a36a, 645a284, ce2d018, 284bdc4, d380fd8, f064203 and 0fa1185. They are now 67d7dcc, b8cc071, 2e043a9, 2521198, 618a83e, 036bbca, 547e6ba and, for D1, 7f2a523, 2686a87, dc7f7a9. The D2a sections have no mapping note like the one at `verify-report.md:5`.
   - The D1 heading (`verify-report.md:3`) and `apply-progress.md:3` name branch `feat/domain-doc-1`, which no longer exists (`feat/domain-doc-1a` and `-1b` do).
   Fix: rewrite them, or add a mapping note at the top of each D2a section.
3. **The new note at `verify-report.md:5` is inaccurate.** It says "only `.agents/agents/archive.md` differs" between the old and new D1 commits. `git diff` of each old/new pair (for example `6e6e013 dc7f7a9`) now shows 7 files: `.agents/agents/{archive,coordinator,tdd-apply}.md`, `.agents/skills/skill-registry/{SKILL.md,refresh.sh}`, `AGENTS.md` and `CLAUDE.md`. "Code trees unchanged" still holds, since no file under `crates/`, `Cargo.*` or `.github/` differs. Fix: say "only agent, skill and top-level guide files differ".

### SUGGESTION

1. Add a blank line between the last item and `#[cfg(test)] mod tests` in `document.rs:79-81`, `page.rs:48-50` and `node.rs:83-85`. rustfmt accepts the current form, but the other modules use the blank line.
2. "The model MUST allow representing structural violations" (`spec.md:240`) is shown only by public fields. A short test that builds a page whose `roots` names a missing node and a node that lists itself as a child would document it and protect it from a future checked constructor.
3. `apply-progress.md` (D2b) could list the commit subjects next to the table rows, as the D2a section now does.

### Result

- Verdict: **FAIL** for D2b (3 CRITICAL, 3 WARNING, 3 SUGGESTION). All gates pass on every commit, every D2b scenario has a passing test, and the D4 deferral of "Accessors and page order" is allowed by its tag. The failure comes from 7 surviving mutants on D2b MUST clauses: the `dpi` accessor and helper argument, the required traits of `IdSpaceExhausted` and `Document`, and `Page::node` ignoring its id.
- Next: **tdd-apply D2b** to fix CRITICAL 1-3 (test-only changes, RED shown by the listed mutants) and preferably WARNING 2-3. Then re-verify D2b. Archive waits for D3 to D6.

## D2b re-verify (the 6 fix commits from `style(doc): add blank line before test modules` to `docs(sdd): record domain-doc D2b verify fixes and use subjects instead of hashes`; delivery `feat/domain-doc-2a..feat/domain-doc-2b`)

**Verdict: PASS** (D2b). CRITICAL 0, WARNING 1, SUGGESTION 2. The change is not complete (D3 to D6 remain), so `next` = tdd-apply for D3.

Verified 2026-10-24 on rustc 1.93.1, cargo-deny 0.20.2, git 2.34.1. Each of the 6 commits was extracted with `git archive` into `/tmp/gafira-reverify-d2b-<sha>`, each with its own `CARGO_TARGET_DIR`. V5, V11 ("Domain crates forbid unsafe code") and V12 were extracted with a YAML parser from each commit's own `.github/workflows/ci.yml` (no change to `.github`, `Cargo.*` or `deny.toml` in `feat/domain-doc-2a..feat/domain-doc-2b`) and run under `bash --noprofile --norc -eo pipefail`. Mutations ran on a separate copy of the `docs(sdd): record domain-doc D2b verify fixes and use subjects instead of hashes` commit, restored after each mutant (`diff -r` clean at the end). The repository tree was not modified.

### 1. Gates per commit

| Commit | fmt | clippy `--workspace --all-targets --locked -D warnings` | test `--workspace --locked` (passed) | `cargo deny check` | V5 | V11 | V12 | gafira-doc lib tests |
|---|---|---|---|---|---|---|---|---|
| `style(doc): add blank line before test modules` | 0 | 0 | 0 (205) | 0 | 0 | 0 | 0 | 50 |
| `test(doc): cover Document dpi accessor and test_document dpi argument` | 0 | 0 | 0 (206) | 0 | 0 | 0 | 0 | 51 |
| `test(doc): assert required traits of IdSpaceExhausted and Document at compile time` | 0 | 0 | 0 (208) | 0 | 0 | 0 | 0 | 53 |
| `test(doc): look up a non-first node by id in Page::node` | 0 | 0 | 0 (208) | 0 | 0 | 0 | 0 | 53 |
| `test(doc): show structural violations are representable in a page` | 0 | 0 | 0 (209) | 0 | 0 | 0 | 0 | 54 |
| `docs(sdd): record domain-doc D2b verify fixes and use subjects instead of hashes` | 0 | 0 | 0 (209) | 0 | 0 | 0 | 0 | 54 |

`cargo deny check`: advisories, bans, licenses and sources ok on all six. Each clippy log shows `Checking gafira-doc` (no stale cache). The five original D2b commits were green in the first D2b run and their production code is unchanged (section 4).

### 2. Previous items

| Item | Fix | Evidence | Status |
|---|---|---|---|
| CRITICAL 1: `dpi()` and `test_document` dpi | `dpi_is_read_back_from_the_document` (`document.rs:129-134`), `test_document(1, Dpi::new(300), ..)` | M01 (`dpi()` returns `Dpi::default()`) and M02 (`test_document` stores `Dpi::default()`) both fail this test | closed |
| CRITICAL 2: `IdSpaceExhausted` `Copy`/`Eq` | `exhaustion_error_is_copy_and_eq` (`ids.rs:124-130`): `assert_traits::<IdSpaceExhausted>()` with `Copy + Eq + Debug`, plus a real copy (`let copied = e; assert_eq!(e, copied)`) | M16 (no `Copy`) and M17 (no `Eq`) fail to build with E0277 at `ids.rs:126`, the assertion call. `Display`/`Error` stay in `exhaustion_error_displays_and_is_an_error` (`ids.rs:118`): removing the `Error` impl fails to build (E0277), and changing the message fails the test | closed |
| CRITICAL 2: `Document` `Debug`/`PartialEq` | `document_is_clone_debug_and_partial_eq` (`document.rs:119-127`): `assert_traits::<Document>()` with `Clone + Debug + PartialEq`, `assert_eq!(d, d.clone())`, `assert_ne!(d, Document::default())`, `format!("{d:?}")` | M18 (no `PartialEq`) fails to build (E0277 at `document.rs:121`, E0369 at 123-124). M19 (no `Debug`) fails to build (E0277 at `document.rs:121,123-125`). Removing `Clone` fails to build (E0277/E0599). A hand-written `PartialEq` that always returns `true` fails `assert_ne!`. See WARNING 1 for weaker hand-written impls | closed |
| CRITICAL 3: `Page::node` ignores `id` | `lookup_stays_inside_the_page` (`page.rs:97-109`): nodes 4, 9, 12 in the second page, each looked up by id, and missing id 5 returns `None` | M22 (first node) fails it and `structural_violations_are_representable`. New mutants also fail it: last node, ceiling lookup `range(id..)`, floor lookup `range(..=id)` | closed |
| WARNING 1: commit not test first | Each fix has a mutant table in `apply-progress.md` (D2b follow-ups) | Every listed mutant re-run here is killed by the named test | closed |
| WARNING 2: stale hashes | Rule stated at the top of both files (`apply-progress.md:3`, `verify-report.md:3`): commits are cited by subject and branch range. Current references (headings, gate tables, size tables, scope lines) use subjects or ranges | Every quoted subject in both files (25 distinct) matches a commit in `chore/lite-sdd-methodology..feat/domain-doc-2b`. `apply-progress.md` contains no hash. Hashes left in `verify-report.md` (lines 229, 295, 297, 400, 410-422) are inside findings of earlier runs, which `verify-report.md:3` marks as history. Branch `feat/domain-doc-1` is gone from current text; the only mention is the quoted finding at line 420 | closed |
| WARNING 3: D1 rebase note | `verify-report.md:8` says seven non-code files differ and nothing under `crates/`, `Cargo.*` or `.github/` | matches the list measured in the first D2b run | closed |
| SUGGESTION 1: blank line before `mod tests` | `style(doc)` commit: `document.rs:81`, `page.rs:50`, `node.rs:85` | only blank lines added | closed |
| SUGGESTION 2: structural violations | `structural_violations_are_representable` (`page.rs:112-126`): root id 99 missing and listed twice, node 3 lists itself as child | passes on unchanged code, as `apply-progress.md` declares (no RED expected) | closed |
| SUGGESTION 3: subjects in apply-progress | D2b table rows carry subjects, follow-ups listed by subject | read back | closed |

### 3. Mutation summary (copy of the `docs(sdd): record domain-doc D2b verify fixes and use subjects instead of hashes` commit, `cargo test -p gafira-doc --locked`, baseline 54 passed)

23 mutants: 18 killed, 5 survived.

- The 7 previous survivors (M01, M02, M16, M17, M18, M19, M22) are all killed: M01, M02 and M22 by tests, and M16 to M19 at build time by the trait assertions (`ids.rs:126`, `document.rs:121`).
- New killed mutants: `Page::node` returning the last node, the ceiling or the floor entry; `Document` `PartialEq` always `true`; `IdSpaceExhausted` `PartialEq` always `false`; no `Debug`, no `Error` impl or a changed message on `IdSpaceExhausted`; `Document` without `Clone`; `test_document` ignoring `next_id`.
- Survivors: `Page::node` scanning values by `base.id` (equivalent on a well-formed arena, SUGGESTION 1); hand-written `Document` `PartialEq` that ignores `dpi`, compares only `pages`, or compares pages by `Arc::ptr_eq` (WARNING 1); hand-written opaque `Debug` printing only `Document` (Debug content is not specified, so no finding).
- No `proptest-regressions` file was created in the mutation copy or the extracted trees, and none is tracked.

### 4. Regression check and code rules

- `git diff a1de16a..HEAD -- crates` (a1de16a = `docs(sdd): record domain-doc D2b apply progress`) changes production code only by three blank lines (`document.rs:81`, `page.rs:50`, `node.rs:85`). Everything else is inside `mod tests`. Every clause marked covered in the first D2b run still holds, and all 54 lib tests pass on every commit.
- Added lines in `crates/` contain no `#[allow]`, `unsafe`, `HashMap`, `HashSet` or `expect`, and they are ASCII English. `unwrap` appears only in tests.
- The six subjects are conventional (`style(doc)`, `test(doc)` x4, `docs(sdd)`). No message has a `Co-Authored-By` or AI attribution line.
- The size claims in `apply-progress.md` (D2b follow-ups) match: 5 files, 522 insertions(+), 1 deletion(-) without `openspec`; the fix commits add 4 files, 55 insertions(+), 5 deletions(-).

### 5. Size

`git diff --shortstat feat/domain-doc-2a..HEAD` gives 7 files changed, 750 insertions(+), 41 deletions(-). Without `openspec`: 5 files, 522 insertions(+), 1 deletion(-) (`document.rs` +173, `node.rs` +166, `page.rs` +149, `ids.rs` +27, `lib.rs` +8/-1). About 240 of those lines are production code and about 280 are tests.

The code-only figure is what counts. Every earlier delivery in this change was measured that way (`apply-progress.md` D1, D2a and D2b, and this report's D1 and D2a size sections), and the design re-cut rule (`design.md:395-399`) compares against the forecasts, which cover code and tests only (`design.md:353-355`). The other 228 lines are this report and `apply-progress.md`, which a reviewer of the code PR does not need to read line by line. 522 is under ~700 and under the 800 hard limit. The ratio against the ~465 forecast is 1.12, under the x1.3 recompute trigger. No split is needed.

### CRITICAL

None.

### WARNING

1. **`Document` equality is proven to exist, but not to cover every field or to follow IEEE.** `document_is_clone_debug_and_partial_eq` (`document.rs:119-127`) compares a document with its clone and with `Document::default()`, which differs in every field. A hand-written `PartialEq` that ignores `dpi`, compares only `pages`, or compares pages by `Arc::ptr_eq` survives the suite. The last one also breaks "PartialEq on model types MUST be IEEE" (`spec.md:297`) at document level, because a document holding a NaN node would equal its clone. Today the derive (`document.rs:13`) makes all three impossible, so this is not blocking. Each mutant needs a hand-written impl, which is a much larger change than the one-line mutants that were CRITICAL before. Fix: add `assert_ne!` for two documents that differ only in `dpi` and only in `next_id`, and for a document holding a NaN-placement node and its clone.

### SUGGESTION

1. `Page::node` scanning `nodes.values()` for a matching `base.id` gives the same result as the keyed lookup whenever each key equals its node's `base.id`. A test node stored under a key that differs from its `base.id` would pin "lookup by arena key". That arrangement is also a structural violation the model must be able to represent.
2. The historical "Doc fixes" finding of the first D2b run (`verify-report.md:400`) says the D1 section "now cites 7f2a523, ...". The subject rewrite has since replaced those hashes. `verify-report.md:3` already marks old findings as history, but adding "(superseded by the subject rewrite)" there would stop a reader from looking for them.

### Result

- Verdict: **PASS** for D2b (0 CRITICAL, 1 WARNING, 2 SUGGESTION). All gates pass on every commit. All 7 previous survivors are killed, 4 of them at build time by the trait assertions. Production code is unchanged apart from blank lines. Every D2b requirement and scenario is covered.
- Next: the owner decides on pushing and the PR for `feat/domain-doc-2b`, then **tdd-apply D3**. D3 verify must confirm serde on the D2 types. The D4 verify must see "Accessors and page order" through `from_parts`. Archive waits for D3 to D6.

## D3a (branch `feat/domain-doc-3a`, range `feat/domain-doc-2b..feat/domain-doc-3a`, 5 commits from `feat(doc): define the document wire format` to `docs(sdd): record domain-doc D3 progress`)

**Verdict: FAIL** (D3a). CRITICAL 2, WARNING 6, SUGGESTION 2. `next` = tdd-apply for D3a (fix the CRITICAL items), then re-verify D3a, then tdd-apply D3b.

Scope: the D3 requirements of `specs/document-serde/spec.md` ("Wire shape and key order", "Deterministic round trip" D3 part, "Defaults, normalization and forward compatibility", "Animatable compatibility of bare values", "Extension payload survival", "Canonical goldens survive every checkout", "Non-finite and out-of-range numbers" D3 part, the D3 sentence of "Validating load path", "Hostile and corrupt input" D3 part) and the serde note of `specs/document-model/spec.md:24`. D3 was split for size. D3a holds the first three planned commits and a carry-forward test. D3b ("test(doc): add canonical goldens and hostile inputs": goldens, `.gitattributes`, `tests/golden.rs`, `tests/serde.rs`, the `cargo tree` and `git check-attr` commands) is not in this range. Scenarios it owns are listed under "Left for D3b" and are not findings here. Design rows: A13 `design.md:43`, file map `design.md:97-100`, wire format `design.md:314-340`, commit plan `design.md:388`, T3.1-T3.7 `design.md:414-420`.

Verified 2026-10-24 on rustc 1.93.1, cargo-deny 0.20.2, git 2.34.1. Each commit was extracted with `git archive` into its own directory under `/tmp/d3a-verify`, each with its own `CARGO_TARGET_DIR`. The ARQ-07, V11 ("Domain crates forbid unsafe code"), V5 and V12 step bodies were extracted with a YAML parser from each commit's own `.github/workflows/ci.yml` (unchanged in the range) and run under `bash --noprofile --norc -eo pipefail`. Mutations ran on a separate scratch git copy of the `docs(sdd): record domain-doc D3 progress` commit (`cargo test -p gafira-doc --locked`, baseline 77 lib tests passed). The repository tree was not modified apart from this section.

### 1. Gates per commit

| Commit | fmt | clippy `--workspace --all-targets --locked -D warnings` | test `--workspace --locked` (passed) | `cargo deny check` | ARQ-07 | V11 | V5 | V12 | gafira-doc lib tests |
|---|---|---|---|---|---|---|---|---|---|
| `feat(doc): define the document wire format` | 0 | 0 | 0 (220) | 0 | 0 | 0 | 0 | 0 | 65 |
| `feat(doc): serialize arenas as sorted arrays` | 0 | 0 | 0 (224) | 0 | 0 | 0 | 0 | 0 | 69 |
| `feat(doc): add from_json and to_json` | 0 | 0 | 0 (231) | 0 | 0 | 0 | 0 | 0 | 76 |
| `test(doc): show Document equality depends on dpi and page contents` | 0 | 0 | 0 (232) | 0 | 0 | 0 | 0 | 0 | 77 |
| `docs(sdd): record domain-doc D3 progress` | 0 | 0 | 0 (232) | 0 | 0 | 0 | 0 | 0 | 77 |

`cargo deny check`: advisories, bans, licenses and sources ok on all five. Each clippy log shows `Checking gafira-doc` (no stale cache). Every prefix passes every gate. The range changes no manifest, no `Cargo.lock` and no `.github` file.

### 2. Requirements and scenarios

#### Wire shape and key order (D3) (`document-serde/spec.md:22`)

| Scenario / clause | Evidence | Status |
|---|---|---|
| Exact minimal document | `json::tests::minimal_document_has_exact_bytes` (`json.rs:107`), exact string from `spec.md:29`. Killed: M04 (page `locked`/`hidden` swapped), M09 (`to_string_pretty`), M21 (document `schema_version`/`next_id` swapped), M32 (`dpi` skipped) | covered |
| Arena order is ascending id | `page::tests::arena_is_written_as_an_ascending_array` (`page.rs:187`), ids inserted 10, 2, 7, written 2, 7, 10. Killed: M01 (serializer emits `values().rev()`) | covered |
| Node shapes per kind | `node::tests::kinds_have_their_wire_shapes` (`node.rs:177`): group, ellipse with sRGB fill (`space` first), extension, `"fill":null`, each node ends with `"children":[...]`. Killed: M05 (PascalCase tags), M06 (tag key `kind`), M31 (PascalCase geometry) | covered |
| None is written as null | `element::tests::wire_form_writes_every_field_with_null_name` (`element.rs:95`). Killed: M03 (`skip_serializing_if` on `name`) | covered |
| Kebab-case names, bare ids and DPI, `Arc` as bare value | `blend.rs` D1 tests `wire_names` and `unknown_or_miscased_names_are_rejected` kill M07 (snake_case blend); `"blend":"normal"` in `element.rs:95`; bare `"id":1`, `"dpi":96` and bare pages and nodes in `json.rs:107`, `page.rs:172` | covered |

#### Deterministic round trip (ARQ-10) (D3, D6) (`spec.md:46`)

| Scenario / clause | Evidence | Status |
|---|---|---|
| Round trip idempotent (property) | D6 | out of scope |
| Bit identity of numbers | `placement::tests::numbers_keep_their_bits` (`placement.rs:228`), `element::tests::subnormal_opacity_keeps_its_bits` (`element.rs:139`), `json::tests::numbers_keep_their_bits_through_a_document` (`json.rs:141`): opacity `1e-45`, `x = -0.0` with `is_sign_negative`, `w = f64::MAX`, `rotation_deg = 5e-324`, all by `to_bits()` | covered |
| Same bytes from a different build order | `json::tests::round_trip_is_idempotent_and_independent_of_build_order` (`json.rs:121`) | covered |
| Arena order mutation is caught | Deviation: a `BTreeMap` has no insertion order, so the literal mutant cannot be written. The equivalent mutant M01 (reverse emission) is killed by `page.rs:187`. See WARNING 3 | covered (deviation accepted) |
| `from_json(to_json(d))` equals `d` field by field | Only documents with DPI 96 and schema version 1 are round-tripped. M22 (`From<DocumentWire>` drops `dpi`) and M23 (constant `schema_version` 1) survive. See CRITICAL 1 | **failing** |
| `float_roundtrip` kept | root `Cargo.toml:37`; `cargo tree -e features -i serde_json` shows `default`, `float_roundtrip`, `std` only | covered |

#### Defaults, normalization and forward compatibility (D3) (`spec.md:70`)

| Scenario / clause | Evidence | Status |
|---|---|---|
| Defaults applied | `json::tests::defaults_are_applied_and_rewritten_explicitly` (`json.rs:231`), `element::tests::omitted_fields_take_their_defaults` (`element.rs:111`), `placement::tests::optional_fields_default_and_integers_are_accepted` (`placement.rs:205`), `page::tests::page_defaults_required_fields_and_unknown_fields` (`page.rs:220`), `node::tests::kind_defaults_required_fields_and_vocabulary` (`node.rs:219`). Killed: M08 (`dpi` default), M16 (`children`), M17 (`hidden`), M18 (opacity default 0.5), M19 (visible default false), M20 (`rotation_deg`). M15 (`fill` default removed) survives but is equivalent: serde derive already reads a missing `Option` field as `None` | covered |
| `Flip.x`/`Flip.y` default (declared deviation) | Not a spec violation: the required-on-read list at `spec.md:72` is closed and "everything else MUST default", and the design's rejected list (`design.md:332`) does not name them either. A probe confirms `"flip":{}` reads as `Flip { x: false, y: false }`. But no test reads a partial `flip`: M12 and M13 survive. See CRITICAL 2 | **failing** |
| Unknown fields ignored and dropped | page, node, base and kind levels (`page.rs:220`, `node.rs:219`, `element.rs:119`). Document and placement levels are untested (M29, M30 `deny_unknown_fields` survive), and "the output contains none of the extras" is not asserted | partial, rest D3b |
| Normalization on save | `"x":3` loads as `3.0` (`placement.rs:205`), but `"x":3.0` is not asserted on the output; nodes 7 then 2 load and save as 2, 7 (`page.rs:205` plus `page.rs:187`). Loading an unsorted payload from text is untested | partial, rest D3b |
| Missing required field (table) | placement `x`, `y`, `w`, `h` (`placement.rs:211`); base `id`, `placement` (`element.rs:119`); page `id`, `size` (`page.rs:220`); node `base`, kind `type`, `geometry`, `type_id`, extension `schema_version`, `payload` (`node.rs:219`). Missing: document `schema_version`, `next_id`, `pages`, node `kind`, color `space` (M25, M26, M27, M28 survive) | partial, rest D3b |
| Unknown vocabulary (table) | `"type":"text"`, `"Group"`, `"geometry":"star"`, `"space":"rec2020"` (`node.rs:219`), `"blend":"Multiply"` (`element.rs:119`). `"unit":"parsec"` is not tested in this range | partial, rest D3b |

#### Animatable compatibility of bare values (D3) (`spec.md:99`)

| Scenario | Evidence | Status |
|---|---|---|
| Keyed object rejected | `placement::tests::required_fields_and_keyed_objects` (`placement.rs:211`): `{"keys":[]}` fails as `Placement`, `Color` and `f32`; bare `0.5` is an `f32` | covered |
| Bare values | `element.rs:95` (`"opacity":0.5`, flat placement), `node.rs:177` (flat fill, no wrapper, no `keys`) | covered |
| Container default mutation is caught | M14 (`#[derive(Default)]` plus container `#[serde(default)]` on `Placement`) is killed by `placement.rs:211` | covered |

#### Extension payload survival (D3) (`spec.md:118`)

| Scenario | Evidence | Status |
|---|---|---|
| Extension fields kept | `json::tests::extension_fields_survive_and_stay_separate_from_the_document_version` (`json.rs:199`) | covered |
| f64 field in an internally tagged node | `node::tests::extension_payload_null_and_f64_extremes_survive` (`node.rs:249`) puts the extremes in `base.placement`, which is outside the `type`-tagged `kind`. With `arbitrary_precision` switched on in a scratch copy, this test still passes. See WARNING 1 | open, D3b |
| Canonical golden, non-canonical golden, key order and number spelling, integer beyond u64, duplicate payload keys, `1e400`, feature guards (command) | not in this range | D3b |

Feature mutants (scratch `Cargo.toml` edit): `preserve_order` fails `kinds_have_their_wire_shapes` and `extension_fields_survive_and_stay_separate_from_the_document_version`. `arbitrary_precision` fails only `numbers_keep_their_bits_through_a_document` (`json.rs:152`: "invalid type: map, expected f32", from the fill channel `f32::MAX` at `json.rs:149`). Both guards exist today, the second only by accident.

#### Canonical goldens survive every checkout (D3) (`spec.md:167`)

All three scenarios (round-trip goldens, text conversion off, golden shape) are D3b. No golden or `.gitattributes` exists in this range.

#### Non-finite and out-of-range numbers (D3, D4) (`spec.md:186`)

| Scenario | Evidence | Status |
|---|---|---|
| Non-finite not persistable | `json::tests::non_finite_numbers_serialize_as_null_and_do_not_load` (`json.rs:169`): NaN opacity, `+inf` x, `-inf` rotation, NaN channel, NaN page width; serialization does not panic and loading is `LoadError::Json`. The "field is null" check is weak, see WARNING 2 | covered |
| Finite out-of-range survives serde, not load | The serde half (opacity `1.5` through plain `Node` values, by bits) is not in this range; the load half is D4 | D3b / D4 |

#### Validating load path, D3 sentence (`spec.md:202`)

`Document` derives `Deserialize` through `#[serde(from = "DocumentWire")]` (`document.rs:17`), the unchecked D3 form; `try_from` with `validate()` is D4, so the "validation on load" mutant does not apply yet. `DocumentWire` is `pub(crate)` (`json.rs:36`). `from_json` and `to_json` (`json.rs:57`, `json.rs:62`) have the designed signatures. `LoadError` is `#[non_exhaustive]` with only `Json` (the `Invalid` variant is D4), implements `Display` and `Error` with `source()` (`json.rs:10-31`). Killed: M10 (BOM stripped before parsing), M11 (`source()` returns `None`) by `unreadable_bytes_are_json_errors_with_a_source` (`json.rs:214`).

#### Hostile and corrupt input, D3 part (`spec.md:224`)

| Scenario | Evidence | Status |
|---|---|---|
| Empty, truncated, trailing and encoding | `json.rs:214`: empty input, every prefix, trailing `x`, invalid UTF-8, BOM, `\ud800`, each `LoadError::Json` with a source. Prefixes come from a built document, not a golden | covered |
| Duplicate node id inside one page | `page::tests::unordered_nodes_load_and_duplicates_fail` (`page.rs:205`), "duplicate node id 5". Killed: M02 (duplicate check disabled) | covered |
| Nodes out of order are accepted | `page.rs:205` and `page.rs:187` | covered |
| Duplicate struct field | shape `kind` with two `geometry` keys errors (`node.rs:219`), which answers the design's open question. Two `next_id` keys and two `opacity` keys are untested here; a probe shows both error | partial, rest D3b |
| Wrong types | `"nodes":{}`, `"locked":"yes"` (`page.rs:220`), `"visible":"yes"`, `"opacity":null` (`element.rs:119`). `null`, `[]`, `"x"`, `"next_id":"7"`, `"pages":{}`, `"name":5` untested | partial, rest D3b |
| Bad numbers; hostile values (unknown `space` through `from_json`) | not in this range | D3b |
| Recursion limit 127/128; payload depth 121/122 | not in this range (the design puts the boundaries in T4.2, D4) | D3b or D4 |
| Duplicate id across pages, deep payload, deep tree, corruption property | D4, D4, D5, D6 | out of scope |

#### document-model serde note (`document-model/spec.md:24`)

Serde derives now exist on `Flip`, `Placement`, `ElementBase`, `GroupNode`, `ShapeGeometry`, `ShapeNode`, `ExtensionNode`, `NodeKind`, `Node`, `Page` and `Document` (validating `Deserialize` from D4). Error and helper types have none. No dependency changed: `serde` with `rc` stays in the crate manifest only (`crates/gafira-doc/Cargo.toml:12`, A3), `cargo tree -e features -i serde` shows `rc`, and the normal tree adds no new package. Covered.

#### Declared deviations

| Deviation | Judgement |
|---|---|
| `Document` fields `pub(crate)` (`document.rs:19-22`) so `json.rs` can build a `Document` | Accepted. Other crates still cannot see or write the fields, which is what A11 and ARQ-30 need. It widens write access inside the crate; see SUGGESTION 1 |
| "Arena order mutation" replaced by an output-order test | Accepted. M01 is the closest mutant and it is killed. The spec text still describes a mutant that cannot exist; see WARNING 3 |
| `Flip.x`/`Flip.y` default to false on read | Accepted, and required by the spec: `spec.md:72` lists the required fields exhaustively, and "a missing required field MUST be an error" applies only to that list. The behavior has no test; see CRITICAL 2 |

#### Mutation summary

34 mutants (32 source edits and the 2 feature mutants above): 23 killed, 11 survived. M24 as first written (`next_id.max(3)`) was equivalent on the fixtures, which all use `next_id` 3. It was re-run as `next_id + 1` and was killed by four `json::tests`. Survivors: M12, M13 (CRITICAL 2); M22, M23 (CRITICAL 1); M15 (equivalent); M25 to M30 (required fields at document and node level, unknown fields at document and placement level, handed to D3b in WARNING 5). No `proptest-regressions` file was created in any copy, and none is tracked.

### 3. Code rules

- `#![forbid(unsafe_code)]` stays at `lib.rs:6`. The range adds no `#[allow]`, no `unsafe`, no `HashMap` or `HashSet`, no `deny_unknown_fields` and no `skip_serializing_if`.
- Code, comments and identifiers are English. The only non-ASCII characters are the `é` test data at `json.rs:203,207`.
- The five subjects are conventional (`feat(doc)` x3, `test(doc)`, `docs(sdd)`), and no message body has a `Co-Authored-By` or AI attribution line.
- `serde_json` features match the design: the workspace declares `float_roundtrip` only, with no `preserve_order` and no `arbitrary_precision`.

### 4. Size

`git diff --shortstat feat/domain-doc-2b..HEAD -- . ':!openspec'` gives 8 files changed, 611 insertions(+), 15 deletions(-), which matches `apply-progress.md`. That is under ~700 and under the 800 hard limit. The D3a part of the D3 forecast is 399 (serde attributes 76, `arena.rs` 76, `json.rs` 114, in-crate tests 133, `design.md:356-357`), so the observed ratio is 1.53. See WARNING 6.

### CRITICAL

1. **The wire-to-document conversion is not pinned for `dpi` or `schema_version`.** `From<DocumentWire> for Document` (`json.rs:44-52`) may drop `dpi` (M22: `dpi: Dpi::default()`) or force `schema_version` to 1 (M23), and the whole suite still passes, because every round-tripped or loaded document uses DPI 96 and schema version 1. This breaks "`from_json(to_json(d))` MUST equal `d` field by field" (`spec.md:48`) without any test noticing. M23 also matters for D4: if the read version is replaced by 1, `UnsupportedSchemaVersion` can never fire. Fix (test first): round-trip a `test_document` with `Dpi::new(300)` and assert `dpi()`. Then add an in-crate test that converts a parsed `DocumentWire` with `schema_version` 2 and reads back 2. D4 will replace that second test with the `UnsupportedSchemaVersion { found: 2 }` scenario.
2. **The `Flip` sub-field defaults have no test.** `#[serde(default)]` on `Flip.x` and `Flip.y` (`placement.rs:10,13`) is a declared deviation, and the spec requires the behavior (`spec.md:72`, "everything else MUST default"). But removing either attribute (M12, M13) passes the suite, because every test either omits `flip` entirely or writes both keys. Fix: load `{"x":0,"y":0,"w":1,"h":1,"flip":{}}` and expect `Flip::default()`, and load `"flip":{"y":true}` and expect `x == false`.

### WARNING

1. **The claimed guard for "f64 field in an internally tagged node" does not test that path.** `node::tests::extension_payload_null_and_f64_extremes_survive` (`node.rs:249`) puts the extreme values in `base.placement`, which serde reads directly. Only `kind` goes through the buffered internally tagged path. With `arbitrary_precision` on, the test still passes. Today the only test that catches that feature is `json.rs:141`, through the fill channel `f32::MAX` (`json.rs:149`), and that test compares the fill with IEEE `==`, not by bits (the `-0.0` channel is unchecked). D3b must close the scenario (`spec.md:152`): extreme `f64` values inside the extension payload and extreme `f32` channels inside a shape fill, compared by `to_bits()`. `apply-progress.md` should stop listing this scenario as covered.
2. **The "field is null" check in the non-finite test is vacuous for node cases.** `json.rs:190` asserts `json.contains("null")`, but every node already writes `"name":null`. The load failure is the real evidence. Fix: assert the field itself, for example `"opacity":null`, `"x":null`, `"rotation_deg":null`, `"r":null`, `"value":null`.
3. **The spec still describes the arena mutant that cannot exist.** `spec.md:65-68` and the T3.1 mutation column (`design.md:414`) say "serializer mutated to emit insertion order". With a `BTreeMap` there is no insertion order to emit. Amend the spec so the mutant is "the arena serializer emits any order other than ascending, for example reversed", which is what M01 checks.
4. **The required-on-read list does not name fields owned by other crates.** `spec.md:72` makes "everything else" defaultable, but the code (correctly, per `design.md:332` and the owning crates) rejects a `Length` without `unit` and a color without `a` (probe). A literal reading of the spec would require defaults for `size.width`, `size.height`, `value`, `unit` and the channels `r`, `g`, `b`, `a`. Fix the spec text: list them as required, or limit "everything else" to the types this crate owns. No code change is needed.
5. **`apply-progress.md` overstates D3a coverage, and six surviving mutants pass to D3b.** "Unknown fields ignored" is tested at four of six levels (M29 and M30, `deny_unknown_fields` on `DocumentWire` and on `Placement`, survive). "Missing required field" lacks the document fields and node `kind` (M25, M26, M27 and M28, `#[serde(default)]` on `pages`, `next_id`, `schema_version` and `kind`, survive). Neither the dropped-on-save check nor the `"x":3.0` output check exists. These belong to the D3b `tests/serde.rs` table, but the D3b list in `apply-progress.md` does not name them. Each D3b row should kill its mutant.
6. **The re-cut rule asks for new forecasts, and none were recorded.** D3a measures 611 against its 399 share of the forecast, a ratio of 1.53, which is over the x1.3 trigger (`design.md:398`, rule 3). With that ratio, D3b comes to about 445 (fine), but D4 (640) becomes about 980, D5 (465) about 710 and D6 (645) about 985. The rule says to re-cut any delivery whose new forecast exceeds ~700 before it starts. Record the recomputation in `apply-progress.md` and plan the D4 and D6 splits (the design names D6 generator/properties as the natural cut; D4 has none named yet).

### SUGGESTION

1. Keep `Document` fields private and add one `pub(crate)` constructor in `document.rs` that `json.rs` (and later `from_parts`/`try_from`) calls. In-crate writes then have a single entry point, which D4 needs for validation.
2. The hostile-prefix test (`json.rs:214`) cuts a built document. D3b can repeat it over `minimal.json` so the scenario text ("every prefix of a small golden") matches exactly.

### Left for D3b (D3 scenarios not closed by D3a)

- Defaults, normalization and forward compatibility: "Unknown fields ignored and dropped" (document and placement levels, and the dropped-on-save output check); "Normalization on save" (`"x":3.0` in the output, an unsorted payload read from text written canonically); "Missing required field" (document `schema_version`, `next_id`, `pages`; node `kind`; color `space`); "Unknown vocabulary" (`"unit":"parsec"`).
- Extension payload survival: "Canonical golden survives", "Non-canonical golden normalizes", "Key order and number spelling", "Integer beyond u64", "Duplicate keys in a payload", "Unrepresentable payload number", "f64 field in an internally tagged node" (WARNING 1), "Feature guards" (command; this run already shows no `preserve_order`, no `arbitrary_precision`, and `rc` present).
- Canonical goldens survive every checkout: "Round-trip goldens" (including the in-crate test that `full` equals the document built with `test_document`), "Text conversion is off" (command), "Golden shape" (command).
- Non-finite and out-of-range numbers: "Finite out-of-range survives serde, not load", serde half.
- Hostile and corrupt input: "Wrong types" (`null`, `[]`, `"x"`, `"next_id":"7"`, `"pages":{}`, `"name":5`); "Bad numbers" (all rows); "Duplicate struct field" (two `next_id`, two `opacity`); "Hostile values" (unknown `space` through `from_json` gives `LoadError::Json`). "Pure JSON recursion limit" and "Payload depth inside a full document" are untagged in the spec, but the design places them in T4.2 (D4). D3b should either take them or record that D4 owns them.

### Result

- Verdict: **FAIL** for D3a (2 CRITICAL, 6 WARNING, 2 SUGGESTION). All gates, including `cargo deny check`, ARQ-07, V5, V11 and V12, pass on every commit. 23 of 34 mutants are killed. The in-scope failures are the unpinned `dpi`/`schema_version` conversion (M22, M23) and the untested `Flip` sub-field defaults (M12, M13).
- Next: **tdd-apply D3a** for CRITICAL 1 and 2 (and preferably WARNING 1, 2 and 5 in the docs), then re-verify D3a, then **tdd-apply D3b** with the list above and the re-cut forecasts of WARNING 6. Archive waits for D3b to D6.

## D3a re-verify (the 3 fix commits from `test(doc): pin dpi, schema version and partial flip on load` to `docs(sdd): fix domain-doc D3a verify findings`; delivery `feat/domain-doc-2b..feat/domain-doc-3a`)

**Verdict: PASS** (D3a). CRITICAL 0, WARNING 3, SUGGESTION 2. `next` = tdd-apply D3b (with the WARNING items below folded in). Archive waits for D3b to D6.

Verified 2026-10-24 with the same method as the D3a run: each commit extracted with `git archive` into its own directory under `/tmp/d3a-verify`, its own `CARGO_TARGET_DIR`, and the ARQ-07, V11, V5 and V12 step bodies parsed from that commit's own `.github/workflows/ci.yml` (no `.github`, manifest, `Cargo.lock` or `deny.toml` change in the fix commits). Mutations ran on a fresh scratch git copy of `docs(sdd): fix domain-doc D3a verify findings` (`cargo test -p gafira-doc --locked`, baseline 82 lib tests). The repository tree was not modified apart from this section.

### 1. Gates per commit

| Commit | fmt | clippy `--workspace --all-targets --locked -D warnings` | test `--workspace --locked` (passed) | `cargo deny check` | ARQ-07 | V11 | V5 | V12 | gafira-doc lib tests |
|---|---|---|---|---|---|---|---|---|---|
| `test(doc): pin dpi, schema version and partial flip on load` | 0 | 0 | 0 (235) | 0 | 0 | 0 | 0 | 0 | 80 |
| `test(doc): put floats inside the tagged kind and assert the null field` | 0 | 0 | 0 (237) | 0 | 0 | 0 | 0 | 0 | 82 |
| `docs(sdd): fix domain-doc D3a verify findings` | 0 | 0 | 0 (237) | 0 | 0 | 0 | 0 | 0 | 82 |

`cargo deny check`: advisories, bans, licenses and sources ok on all three. Each clippy log shows `Checking gafira-doc`.

### 2. Previous items

| Item | Evidence | Status |
|---|---|---|
| CRITICAL 1 (`dpi`, `schema_version` not pinned) | `json::tests::dpi_survives_the_wire_conversion` (`json.rs:254`, DPI 300 written as `"dpi":300`, read back, whole document equal) kills M22; `json::tests::schema_version_is_read_as_written_by_the_wire_conversion` (`json.rs:264`, version 2 read and re-written byte-exact) kills M23 | fixed |
| CRITICAL 2 (partial `flip`) | `placement::tests::partial_flip_defaults_missing_axes_to_false` (`placement.rs:211`: `{}`, `{"x":true}`, `{"y":true}`) kills M12 and M13 | fixed |
| WARNING 1 (tagged-kind guard) | `node::tests::floats_inside_the_tagged_kind_survive` (`node.rs:264`, `f32::MAX`, `1e-45` by bits, `-0.0` sign) fails with `arbitrary_precision` on (`node.rs:268`, as does `json.rs:152`). `node::tests::f64_payload_inside_the_tagged_kind_survives` (`node.rs:283`) still passes with the feature on; see WARNING 1 below. The scenario stays open for D3b (feature-guard command) | fixed for the shape half |
| WARNING 2 (vacuous null check) | `json.rs:199` asserts the exact field per case. Mutation-checked below | fixed |
| WARNING 3 (arena mutant wording) | `spec.md:67` and the T3.1 row (`design.md:414`) now say reverse output order. `design.md:453` (D6 property mutations) still says "insertion order"; see WARNING 2 below | fixed for D3 |
| WARNING 4 (defaults rule) | `spec.md:72` now limits defaults to the listed fields, adds `flip.x`/`flip.y`, and says every other field (`Length` `value`/`unit`, color channels) must be present. Matches every `#[serde(default)]` in the crate: `json.rs:39`; `element.rs:16,21,24,27,30`; `page.rs:20,25,28,31,34`; `placement.rs:10,13,35,38,41`; `node.rs:28,76` | fixed |
| WARNING 5 (D3b list) | `apply-progress.md:169-177` lists every D3a gap, names M25-M30 with the row that must kill each, records M15 as equivalent and gives the recursion rows to D4 (T4.2) | fixed (one stale line, WARNING 2) |
| WARNING 6 (re-cut forecasts) | not recorded; see WARNING 3 below | open |
| SUGGESTION 1 | skipped with a reason (`apply-progress.md:188`); accepted | — |
| SUGGESTION 2 | D3b | — |

### 3. Mutation summary (`cargo test -p gafira-doc --locked`)

All 32 source mutants re-run, M24 as `next_id + 1`: 25 killed, 7 survive.

- Previously surviving, now killed: M12, M13 (`placement.rs:211`), M22 (`json.rs:254`), M23 (`json.rs:264`). M24 is killed by six `json::tests`, including both new ones.
- M15 (`fill` default removed) survives and is equivalent: `node.rs:222` loads a shape with no `fill` and asserts `None`, and it passes with the attribute removed (serde derive reads a missing `Option` field as `None`).
- M25, M26, M27, M28, M29, M30 survive. Each is named, with its D3b row, at `apply-progress.md:171` and `apply-progress.md:176`. Not a D3a finding.
- No previous kill regressed.

W2 assertions (test-side mutations on `json.rs:169`, each run alone): 9 of 9 fail at the field assertion `json.rs:199` ("<field> not in ..."). Five make the targeted value finite (opacity, `x`, `rotation_deg`, channel `r`, page width). Four keep the document unloadable but move the non-finite value to another field (`rotation_deg` instead of `x`, `a` instead of `r`, page height instead of width, `x` instead of opacity). The old `contains("null")` check plus the load failure would have passed these four, so the new assertions add real coverage.

Feature mutant `arbitrary_precision` (scratch `Cargo.toml` edit): fails `node::tests::floats_inside_the_tagged_kind_survive` and `json::tests::numbers_keep_their_bits_through_a_document`. `node::tests::f64_payload_inside_the_tagged_kind_survives` passes.

No `proptest-regressions` file was created in any copy, and none is tracked.

### 4. Regression check and code rules

- `git diff 'docs(sdd): record domain-doc D3 progress'..HEAD -- crates` touches only `json.rs`, `node.rs` and `placement.rs`, and every hunk is inside `mod tests` (`json.rs:68`, `node.rs:92`, `placement.rs:80`). Production code is unchanged.
- No `#[allow]`, `unsafe`, `HashMap`, `HashSet`, `deny_unknown_fields` or `skip_serializing_if` added. No non-ASCII characters added.
- Subjects are conventional (`test(doc)` x2, `docs(sdd)`). No message has a `Co-Authored-By` or AI attribution line.

### 5. Size

`git diff --shortstat feat/domain-doc-2b..HEAD -- . ':!openspec'` gives 8 files changed, 696 insertions(+), 15 deletions(-). The fix commits add 96 insertions and 11 deletions. 696 is under ~700 and under the 800 hard limit, but `apply-progress.md:165` still gives 611 (WARNING 3).

### CRITICAL

None.

### WARNING

1. **`apply-progress.md:147` says both new node tests fail with `arbitrary_precision`. Only one does.** `floats_inside_the_tagged_kind_survive` (`node.rs:264`) fails. `f64_payload_inside_the_tagged_kind_survives` (`node.rs:283`) passes, because under that feature a `serde_json::Value` stores the number text, so the bits survive anyway. The progress file also misspells the second name (`_survive`). Correct the claim. The extension half of `spec.md:152` can only be guarded by the D3b `cargo tree` feature-guard command.
2. **Two stale lines after the doc fixes.** `apply-progress.md:166` still puts "127/128 nesting, 121/122 payload depth" in D3b's `tests/serde.rs`, which contradicts `apply-progress.md:177` (D4 owns them). `design.md:453` (D6 property mutations) still says "serialize the arena in insertion order". That mutant cannot exist with a `BTreeMap` (WARNING 3 of the D3a run).
3. **The re-cut forecasts (D3a WARNING 6) are still not recorded, and the size line is stale.** D3a now measures 696 against its 399 forecast share, a ratio of 1.74, which is over the x1.3 trigger (`design.md:398`). At that ratio D3b (~290) is about 505, but D4 (640) is about 1115, D5 (465) about 810 and D6 (645) about 1120. All three need a re-cut before they start. Record the ratio, the new forecasts and the planned splits in `apply-progress.md`, and change the size line at `apply-progress.md:165` from 611 to 696. Do this in D3b's first docs commit at the latest.

### SUGGESTION

1. In D3b, check `-0.0` on the fill channel by sign in `json.rs:141`. It is still compared with IEEE `==`, but `node.rs:264` now covers the same value by sign.
2. Correction to the D3a run above: the workspace `serde_json` line with `float_roundtrip` is `Cargo.toml:18`, not `Cargo.toml:37`.

### Result

- Verdict: **PASS** for D3a (0 CRITICAL, 3 WARNING, 2 SUGGESTION). All gates, including `cargo deny check`, ARQ-07, V5, V11 and V12, pass on each of the three fix commits. Both CRITICAL items are fixed, and M12, M13, M22 and M23 are killed. M15 is confirmed equivalent. M25-M30 are assigned to named D3b rows. Production code is unchanged. Size is 696.
- Next: the owner decides on pushing and the PR for `feat/domain-doc-3a`. Then **tdd-apply D3b**, using the list at `apply-progress.md:169-177` plus WARNING 1-3. Archive waits for D3b to D6.

## D3b (branch `feat/domain-doc-3b`, range `feat/domain-doc-3a..feat/domain-doc-3b`, 7 commits from `test(doc): add canonical goldens and round-trip tests` to `docs(sdd): record domain-doc D3b progress`)

**Verdict: PASS** (D3b). CRITICAL 0, WARNING 2, SUGGESTION 4. **D3 complete: PASS.** D3a and D3b together cover every D3-tagged requirement and scenario in `specs/document-serde/spec.md`, plus the serde note at `specs/document-model/spec.md:24`. Only the parts the spec gives to D4, D5 and D6 remain open. `next` = the owner decides on the push and PR for `feat/domain-doc-3b`. Then record the D4 split (WARNING 2), then **tdd-apply D4**. Archive waits for D4 to D6.

Verified 2026-10-24 with the same method as the D3a runs:
- Each commit was extracted with `git archive` into its own directory, with its own `CARGO_TARGET_DIR`.
- The ARQ-07, V11, V5 and V12 step bodies were parsed from that commit's own `.github/workflows/ci.yml`.
- D3b changes no `.github`, `Cargo.toml`, `Cargo.lock`, crate manifest or `deny.toml` file.
- Mutations ran on a scratch git copy of `docs(sdd): record domain-doc D3b progress` with `cargo test -p gafira-doc --locked`. The baseline is 83 lib tests, 3 `golden` tests and 11 `serde` tests.

The only repository change is this section.

### 1. Gates per commit

| Commit | fmt | clippy `--workspace --all-targets --locked -D warnings` | test `--workspace --locked` (passed) | `cargo deny check` | ARQ-07 | V11 | V5 | V12 |
|---|---|---|---|---|---|---|---|---|
| `test(doc): add canonical goldens and round-trip tests` | 0 | 0 | 0 (241) | 0 | 0 | 0 | 0 | 0 |
| `test(doc): cover required fields, vocabulary and unknown fields on load` | 0 | 0 | 0 (245) | 0 | 0 | 0 | 0 | 0 |
| `test(doc): pin extension payload spelling and survival` | 0 | 0 | 0 (247) | 0 | 0 | 0 | 0 | 0 |
| `test(doc): reject hostile and corrupt input on load` | 0 | 0 | 0 (252) | 0 | 0 | 0 | 0 | 0 |
| `test(doc): compare fill channels by bits through a document` | 0 | 0 | 0 (252) | 0 | 0 | 0 | 0 | 0 |
| `docs(sdd): fix domain-doc D3a re-verify warnings` | 0 | 0 | 0 (252) | 0 | 0 | 0 | 0 | 0 |
| `docs(sdd): record domain-doc D3b progress` | 0 | 0 | 0 (252) | 0 | 0 | 0 | 0 | 0 |

`cargo deny check` reports advisories, bans, licenses and sources ok on all seven. Each clippy log shows `Checking gafira-doc`. The unused `const` that apply-progress says was amended away does not appear in any commit.

Manual command checks, re-run on HEAD:
- `git check-attr text -- crates/gafira-doc/tests/golden/*.json` prints `text: unset` for all four goldens.
- `git ls-files --eol` shows `i/none w/none attr/-text` for each golden.
- `cargo tree -p gafira-doc -e features -i serde_json --locked` shows only `default`, `float_roundtrip` and `std`. There is no `preserve_order` and no `arbitrary_precision`.
- `cargo tree -p gafira-doc -e features -i serde --locked` shows `serde feature "rc"`.
- The normal tree has exactly four direct dependencies: `gafira-color`, `gafira-geom`, `serde` and `serde_json`.
- The dev tree adds only `proptest` and its transitive crates. There is no `rusty-fork` and no `tempfile`.

### 2. Requirements and scenarios: the "left for D3b" list

| Item (from the D3a "Left for D3b" list and `apply-progress.md:169-177`) | Test | Status |
|---|---|---|
| Unknown fields ignored and dropped (document and placement levels, dropped on save) | `serde::unknown_fields_are_ignored_and_dropped` (`tests/serde.rs:84`): extras at 10 levels (document, page, size, node, base, placement, flip, shape kind, fill, extension kind); the loaded document equals the plain one; the output has no `zz_extra` and equals `full.json` byte for byte. Kills M29, M30, N04, N05, N06 | covered |
| Normalization on save | `serde::saving_normalizes_numbers_node_order_and_payload` (`tests/serde.rs:111`): `"x":3` is written as `3.0`, nodes 7 then 2 are written 2 then 7 with `roots` `[7,2,9]` kept, and an unsorted payload is written canonically. The fixture is valid: every node is a root | covered |
| Missing required field (document `schema_version`, `next_id`, `pages`; node `kind`; color `space`) | `serde::missing_required_fields_are_errors` (`tests/serde.rs:27`): 22 rows covering all 18 fields of the required list at `spec.md:72`, plus `Length` `value`/`unit` and channel `r`. Each row is `LoadError::Json`, and a missing pointer fails the test. Kills M25, M26, M27, M28 | covered |
| Unknown vocabulary (`"unit":"parsec"`) and Hostile values (unknown `space` through `from_json`) | `serde::unknown_vocabulary_is_an_error` (`tests/serde.rs:61`): `text`, `Group`, `star`, `Multiply`, `parsec` and `rec2020`, each `LoadError::Json` through `Document::from_json` | covered (the D4 rows of "Hostile values" stay with D4) |
| Canonical golden survives | `golden::canonical_goldens_round_trip_byte_for_byte` (`tests/golden.rs:18`), over the `extension.json` payload: nested objects, arrays, integers, floats, strings, booleans and `null` | covered |
| Non-canonical golden normalizes | `golden::noncanonical_golden_normalizes_to_the_canonical_one` (`tests/golden.rs:29`): unsorted keys, spaces and a tab, `1E2` and `é`; the output equals `extension.json`, a second round trip is identical, and the loaded values are equal | covered |
| Key order and number spelling; Integer beyond u64; Duplicate keys in a payload | `serde::payload_numbers_and_keys_get_one_canonical_spelling` (`tests/serde.rs:149`): rows at `:152-161`, each checked stable on a second pass. `18446744073709551616` becomes `1.8446744073709552e+19`, while `18446744073709551615` stays an integer, and `{"a":1,"a":2}` becomes `{"a":2}` | covered |
| Unrepresentable payload number | `serde::unrepresentable_payload_number_is_a_json_error` (`tests/serde.rs:171`): `1e400` and `-1e400` | covered |
| f64 field in an internally tagged node | Shape half: `node::tests::floats_inside_the_tagged_kind_survive` (`node.rs:264`). Extension half: with `arbitrary_precision` on, `golden::canonical_goldens_round_trip_byte_for_byte`, `golden::noncanonical_...` and four `serde` tests fail (see F02 below), so the black-box tests now guard the feature. `node.rs:283` still cannot detect the feature (a `Value` keeps the number text) | covered (see WARNING 1) |
| Feature guards (command) | Manual `cargo tree`, above. It is also guarded by tests: F01, F02 and F03 are killed below | covered (deviation accepted) |
| Round-trip goldens, including `full` equal to the `test_document` build | `tests/golden.rs:18` (minimal, full, extension); `json::tests::full_golden_equals_the_built_document` (`json.rs:263`): the built document serializes to exactly `full.json` and `full.json` loads back equal. The test also kills golden mutants G01, G07, G08, G09 and G12 | covered |
| Text conversion is off (command) | Manual `git check-attr`, above; `crates/gafira-doc/tests/golden/.gitattributes` holds `*.json -text` | covered (deviation accepted) |
| Golden shape (command) | `golden::goldens_have_no_newline_carriage_return_or_bom` (`tests/golden.rs:41`). Also checked here by bytes: each file is one line, starts with `{`, ends with `}` with no trailing newline (`tail -c` / `od`), and has no `\r` and no BOM bytes. Sizes: minimal 205, full 1512, extension 623, extension_noncanonical 644 bytes | covered (deviation accepted) |
| Finite out-of-range survives serde (serde half) | `serde::finite_out_of_range_values_survive_serde_by_bits` (`tests/serde.rs:258`): opacity `1.5` and rotation `1e300` by `to_bits()` through plain `Node`. Uses a standalone `Node` because D3 has no public document constructor. Kills N18 (clamp on read) | covered (the `from_json` half is D4) |
| Wrong types | `serde::wrong_types_are_json_errors` (`tests/serde.rs:191`): `null`, `[]`, `"x"`, `7`, `true`, `"next_id":"7"`, `"pages":{}`, `"locked":"yes"`, `"name":5`, `opacity: null`, `children: {}` | covered |
| Bad numbers | `serde::bad_numbers_are_json_errors` (`tests/serde.rs:211`): `-1`, `1.5`, `1e3`, `null` and `2^64` for `next_id`, `dpi`, page id, node id and a `roots` id; `dpi` `4294967296`; a float `1e400`. Kills N17, N22, N23 | covered |
| Duplicate struct field | `serde::duplicate_struct_fields_are_json_errors` (`tests/serde.rs:241`): two `next_id`, two `opacity` and two `geometry` (inside the tagged kind). Kills N16 (load through `Value`, last wins) | covered |
| Every prefix of a small golden (D3a SUGGESTION 2) | `serde::every_prefix_of_the_minimal_golden_is_a_json_error` (`tests/serde.rs:183`) | covered |
| Fill `-0.0` by sign (D3a re-verify SUGGESTION 1) | `json.rs:165-173` compares the fill channels by `to_bits()` and asserts the sign of channel `b` | covered |
| Recursion 127/128 and payload depth 121/122 | Owned by D4 (T4.2, `design.md:415`), as recorded at `apply-progress.md:177,251` | out of scope |

D3a re-verify WARNINGs 1-3 are fixed in `docs(sdd): fix domain-doc D3a re-verify warnings`:
- `apply-progress.md:147` now says only `node.rs:264` fails under `arbitrary_precision`, and spells `node.rs:283` correctly.
- The D3b list no longer holds the recursion rows.
- `design.md:453` says "reverse output order".
- The size line reads 696, and the re-cut ratio and forecasts are recorded at `apply-progress.md:252`. The split plan is still missing; see WARNING 2.

#### Declared deviations

| Deviation | Judgement |
|---|---|
| `git check-attr` and `cargo tree` are run by hand (the crate `clippy.toml` bans `std::process::Command`) | Accepted. Both scenarios are typed (command) in the spec, so a recorded manual run satisfies them, and this run reproduced the expected output. The features they guard are also caught by tests: `preserve_order` (F01) and `arbitrary_precision` (F02) fail lib, `golden` and `serde` tests, and removing `rc` (F03) breaks the build. The goldens contain no LF, so a CRLF checkout could not change their bytes even without the attribute (G11 is equivalent). The attribute is defense in depth, and the manual check confirms it |
| Golden shape is a byte test over `include_str!` instead of a shell check | Accepted, and stronger: it runs on all three CI targets (Windows, Linux, macOS) against the checked-out bytes. G02, G03 and G04 are killed. See SUGGESTION 1 for the BOM position |

#### Mutation summary (72 mutants)

- **M01-M32, re-run on HEAD**: 31 killed, and M15 survives as before (equivalent: a missing `Option` field reads as `None`). **M25, M26, M27 and M28** are killed by `serde::missing_required_fields_are_errors` (`tests/serde.rs:27`). **M29 and M30** are killed by `serde::unknown_fields_are_ignored_and_dropped` (`tests/serde.rs:84`). M01 is now also killed by `json.rs:263`, and M22 by `json.rs:263`. No earlier kill regressed.
- **N01-N23, new mutants on D3 production code**: all 23 killed.
  - Trailing newline in `to_json`.
  - `deny_unknown_fields` on `Page`, `ElementBase`, `Flip`, `ShapeNode`, `ExtensionNode` and `Node`.
  - `type_id` renamed; `Extension` tag renamed.
  - `skip_serializing_if` on `children` and on `fill`.
  - Arena writes only the first 3 nodes.
  - Defaults removed on page `name` and `roots`; extension `schema_version` and `payload` made defaultable.
  - Load through `Value`.
  - `next_id` read as `f64`, `dpi` read as truncated `u64`, page id read as `f64`.
  - Opacity clamped on read.
  - `rotation_deg` and `skew` swapped; `fill` written before `geometry`.
- **G01-G14, golden mutants**: 12 killed. The killed ones:
  - full: `-0.0` changed to `0.0`, dpi 300 to 96, second page dropped, an inner LF added, fill `b` changed.
  - minimal: trailing LF, leading BOM.
  - extension: trailing CR, `100.0` changed to `100`, big u64 changed by 1, BOM inside a string.
  - extension_noncanonical: replaced by the canonical file.

  Two survive, neither of which changes behavior:
  - G10 (`1E2` in the non-canonical golden replaced by `100.0`) weakens the fixture only. Exponent normalization stays pinned by the `10E-1` row (`tests/serde.rs:158`); see SUGGESTION 3.
  - G11 (`.gitattributes` emptied) cannot be seen by any test, because the goldens contain no LF. It is covered by the manual `git check-attr` command scenario.
- **F01-F03, feature mutants**: all 3 killed. With `--no-fail-fast`, F01 `preserve_order` fails `golden::noncanonical_...`, `serde::payload_numbers_...` and `serde::saving_normalizes_...`. F02 `arbitrary_precision` fails both golden round trips and `serde::missing_required_...`, `payload_numbers_...`, `unrepresentable_...` and `unknown_fields_...`. This matches the evidence in `apply-progress.md:223-245`. F03 (no `rc`) fails to compile.

No surviving mutant is in scope. No `proptest-regressions` directory exists or is tracked.

#### D3 complete (D3a + D3b)

| Requirement (D3 part) | Scenarios and tests | Status |
|---|---|---|
| Wire shape and key order | Exact minimal `json.rs:107`; arena ascending `page.rs:187`; node shapes `node.rs:177`; `None` as null `element.rs:95`; whole-document key order `json.rs:263` | covered |
| Deterministic round trip (D3 part) | Bit identity `json.rs:141`, `placement.rs:239`, `element.rs:139`; same bytes from a different build order `json.rs:121`; arena-order mutant M01 killed (`page.rs:187`, `json.rs:263`). The 10 000-case property is D6 | covered |
| Defaults, normalization and forward compatibility | Defaults `json.rs:249`, `element.rs:111`, `placement.rs:205,211`, `page.rs:220`, `node.rs:219`; unknown fields `tests/serde.rs:84`; normalization `tests/serde.rs:111`; missing fields `tests/serde.rs:27`; vocabulary `tests/serde.rs:61` | covered |
| Animatable compatibility | Keyed object and bare `0.5` `placement.rs:222`; bare values `element.rs:95`, `node.rs:177`; M14 killed | covered |
| Extension payload survival | `tests/golden.rs:18,29`; `tests/serde.rs:149,171`; `json.rs:217`; `node.rs:264` + F02; feature guards (manual + F01/F02/F03) | covered |
| Canonical goldens survive every checkout | `tests/golden.rs:18,41`; `json.rs:263`; `git check-attr` (manual) | covered |
| Non-finite and out-of-range (D3 part) | `json.rs:178` (each null field asserted); `tests/serde.rs:258` (serde half) | covered (load half D4) |
| Validating load path (D3 sentence) | Unchecked `from = "DocumentWire"` (`document.rs:17`, `json.rs:44`); `DocumentWire` is `pub(crate)` | covered (rest D4) |
| Hostile and corrupt input (D3 part) | Empty/truncated/trailing/UTF-8/BOM/surrogate `json.rs:232`; golden prefixes `tests/serde.rs:183`; wrong types `:191`; bad numbers `:211`; duplicate struct fields `:241`; duplicate node id and out-of-order nodes `page.rs:205`; unknown `space` `:61` | covered (recursion rows and D4 rows to D4; deep tree D5; Q5 D6) |
| `document-model/spec.md:24` serde note | Serde derives on the D2 types (from D3a); `rc` only in `crates/gafira-doc/Cargo.toml:12`; dependency trees unchanged (above) | covered |

### 3. Code rules and production code

- `git diff feat/domain-doc-3a..HEAD -- crates/gafira-doc/src` touches only `json.rs`, in two hunks (`@@ -162` and `@@ -250`). Both are inside `mod tests` (`json.rs:67-68`). Production code is unchanged.
- The other changes are `tests/golden.rs`, `tests/serde.rs`, the four goldens and `.gitattributes`.
- The added lines contain no `#[allow]`, `unsafe`, `HashMap`, `HashSet`, `std::fs`, `process`, `deny_unknown_fields` or `skip_serializing_if`. Goldens are read with `include_str!`.
- The only non-ASCII characters are the `é` test data (`json.rs:282` and the extension goldens).
- The seven subjects are conventional (`test(doc)` x5, `docs(sdd)` x2). No message has a `Co-Authored-By` or AI attribution line.
- No `proptest-regressions`.

### 4. Size

`git diff --shortstat feat/domain-doc-3a..HEAD -- . ':!openspec'` gives 8 files changed, 384 insertions(+), 0 deletions. This matches `apply-progress.md:252`: 379 lines of code plus 5 lines for the goldens and `.gitattributes`. The breakdown is `tests/serde.rs` 272, `json.rs` 54, `tests/golden.rs` 53 and 5 one-line files. That is under ~700. Against the ~290 forecast the ratio is 1.32. D3 as a whole is 696 + 384 = 1080, split across two stacked branches, each under ~700.

### CRITICAL

None.

### WARNING

1. **The D3b scenario table in `apply-progress.md:203-221` omits "f64 field in an internally tagged node".** The D3a list handed this scenario to D3b, and D3b does close it: F02 (`arbitrary_precision`) now fails `tests/golden.rs:18,29` and four `tests/serde.rs` tests, and `node.rs:264` covers the shape half. The progress file never says so, though, and its only mention (`apply-progress.md:147`) still points to "the D3b `cargo tree` feature check and the goldens" without naming tests. Add a row naming the killing tests.
2. **The re-cut is recorded but not planned.** `apply-progress.md:252` gives the ratio of 1.74 and the new forecasts (D4 about 1115, D5 about 810, D6 about 1120), and says all three need a re-cut before they start. `design.md:398` (rule 3) requires the re-cut before a delivery starts, and the D3a re-verify WARNING 3 asked for "the planned splits", but no split points are named. The design names none for D4. Before tdd-apply D4, record the D4 split (for example P1-P3 with `ValidationError` against P4 with `try_from`) and the D5 and D6 splits.

### SUGGESTION

1. `tests/golden.rs:50` checks only for a leading BOM (`starts_with('\u{feff}')`), while the spec says "no byte-order mark" anywhere. G14 (a BOM inside a string) was killed, but only incidentally, by the normalization test. `!golden.contains('\u{feff}')` matches the spec wording.
2. `spec.md:169` says every golden MUST be "single-line compact JSON", but `extension_noncanonical.json` must contain whitespace by `spec.md:132`; it has spaces and a tab. Limit "compact" to the three canonical goldens in the spec text.
3. G10 survives: a cleanup of the non-canonical golden would not be noticed. Consider asserting in `tests/golden.rs` that `NONCANONICAL` contains `1E2`, contains whitespace and is not key-sorted. Note that the unescaped `é` is already serde_json's canonical form: it tests survival, not normalization.
4. `design.md:357` (forecast table) still labels a test "insertion order". A CI step for the two command scenarios could be considered with D5's V13 work; it is not required by D3.

### Result

- Verdict: **PASS** for D3b (0 CRITICAL, 2 WARNING, 4 SUGGESTION). All gates pass on each of the seven commits: fmt, clippy, test, `cargo deny check`, ARQ-07, V11, V5 and V12. M25-M30 are killed. 69 of 72 mutants are killed, and the 3 survivors (M15, G10, G11) are equivalent or do not change behavior. Production code is unchanged. Size is 384.
- **D3 complete: PASS.** Every D3-tagged requirement and scenario is covered by D3a + D3b. The D4, D5 and D6 parts stay open as the spec assigns them.
- Next: the owner decides on pushing and the PR for `feat/domain-doc-3b`. Then fix WARNING 1 and 2 in docs (record the D4/D5/D6 splits) and run **tdd-apply** for the first D4 slice. Archive waits for D4 to D6.

## D4a (branch `feat/domain-doc-4a`, range `feat/domain-doc-3b..feat/domain-doc-4a`, 5 commits from `docs(sdd): re-cut domain-doc D4 and D6 into sub-deliveries` to `docs(sdd): record domain-doc D4a progress`)

**Verdict: FAIL** (D4a). CRITICAL 3, WARNING 5, SUGGESTION 4. All gates pass on all five commits. The re-cut docs commit matches the owner-approved cut. The tests, however, let 12 in-scope mutants survive. `next` = **tdd-apply D4a** (fix commits for CRITICAL 1 to 3), then re-verify D4a.

Verified 2026-10-24 using the D3b method:
- Each commit was extracted with `git archive` into its own directory, with its own `CARGO_TARGET_DIR`.
- The ARQ-07, V11, V5 and V12 step bodies were parsed from that commit's own `.github/workflows/ci.yml`.
- D4a changes no `.github`, `Cargo.toml`, `Cargo.lock`, `clippy.toml` or `deny.toml` file.
- Mutations ran on a scratch copy of `docs(sdd): record domain-doc D4a progress` with `cargo test -p gafira-doc --locked`. Each mutant was restored after its run.

The only repository change is this section.

### 1. Gates per commit

| Commit | fmt | clippy `--workspace --all-targets --locked -D warnings` | test `--workspace --locked` (passed) | `cargo deny check` | ARQ-07 | V11 | V5 | V12 |
|---|---|---|---|---|---|---|---|---|
| `docs(sdd): re-cut domain-doc D4 and D6 into sub-deliveries` | 0 | 0 | 0 (252) | 0 | 0 | 0 | 0 | 0 |
| `feat(doc): add ValidationError` | 0 | 0 | 0 (255) | 0 | 0 | 0 | 0 | 0 |
| `feat(doc): validate header, ids and pages` | 0 | 0 | 0 (260) | 0 | 0 | 0 | 0 | 0 |
| `test(doc): pin the parse-side depth limits for payloads` | 0 | 0 | 0 (262) | 0 | 0 | 0 | 0 | 0 |
| `docs(sdd): record domain-doc D4a progress` | 0 | 0 | 0 (262) | 0 | 0 | 0 | 0 | 0 |

`cargo deny check` reports advisories, bans, licenses and sources ok on all five commits. Each clippy log shows `Checking gafira-doc`. On HEAD, `cargo tree -p gafira-doc -e all --locked` shows no `thiserror`. The writer did not run deny or the V steps. They are confirmed here.

### 2. Re-cut docs commit

`docs(sdd): re-cut domain-doc D4 and D6 into sub-deliveries` implements the owner-approved cut, consistently across `design.md`, `proposal.md` and the three spec files:
- D4a = `ValidationError` (18 variants) + P1-P3 + the parse-side half of T4.2: `Value` 127/128 and a full document at 121/122. See `design.md:361`, `design.md:429` and `proposal.md:76`.
- D4b = P4, depth 64/65, `ExtensionTooDeep`, `try_from`, `LoadError::Invalid`, `from_parts`, accessors and page order, "D4b build accepts cycles", and the edit of the 121 assertion. See `design.md:363`, `design.md:430`, `specs/document-model` (D4b tags) and `specs/document-serde/spec.md:268-271`.
- D5 is unchanged, with the fallback cut (`design.md:367`).
- D6a = the generator, corruption generators and guards. D6b = Q1-Q5, mutation runs and the CI runtime record.
- The docs say "ten sub-deliveries" everywhere. The total 4 517 is the correct sum.
- The commit also closes D3b WARNING 1 (f64 row), WARNING 2 (split points), SUGGESTION 2 (`extension_noncanonical` not compact) and SUGGESTION 4 ("reverse output order").

One later departure: in this re-cut commit, the first two cases of "Earlier pass wins" and the P1-P3 order rows were D4a. `docs(sdd): record domain-doc D4a progress` retagged them to D4b (`spec.md:45,49`, `design.md:429-430`). The owner did not approve that move. See CRITICAL 3.

### 3. Requirements and scenarios (D4a)

| Scenario | Test | Status |
|---|---|---|
| Valid documents (`spec.md:25`) | `validate::tests::valid_documents_pass` (`validate.rs:163`): `Document::default()`, the base, and a page holding a group with a rectangle and an extension | covered |
| Each rule has its own variant, rows 1-8 (`spec.md:30`) | `header_rows` (`validate.rs:188`), `id_rows` (`:217`), `page_rows` (`:249`), `root_rows` (`:290`). Each of the 8 variants appears with its own expected value | covered, but defects only on page 1 (CRITICAL 1) |
| Error contract (`spec.md:35`) | `error::tests::every_variant_has_a_distinct_message_naming_ids_and_field` (`error.rs:186`): 18 exact texts, 18 distinct discriminants. `errors_are_comparable_clone_and_expose_the_page_size_cause` (`error.rs:285`): `&dyn Error`, `source()`. `node_fields_have_names` (`error.rs:266`). No `thiserror` in `cargo tree` | covered |
| Determinism (property) (`spec.md:40`) | none | not tagged; uncovered (WARNING 4) |
| Earlier pass wins (`spec.md:49`) | none; retagged D4b by the progress commit | missing (CRITICAL 3) |
| Versions (`spec.md:73`) | `header_rows`: 0, 2 and `u32::MAX` carry the found value. Version 1 passes through `base()` | covered |
| DPI edges (`spec.md:78`) | `header_rows`: 0, 1, 10 000, 10 001 and `u32::MAX` | covered |
| Duplicate ids (`spec.md:87`) | `id_rows` (`validate.rs:223-225`): two pages with id 1; node 3 in two arenas; page id 2 equal to node id 2 | covered (other ids, same shapes) |
| Key differs from base id (`spec.md:92`) | `id_rows` `rekey` (`validate.rs:218,227`): key 5, `base.id` 3, page 1 | covered, but page 1 only (CRITICAL 1) |
| next_id bound (`spec.md:97`) | `id_rows` (`validate.rs:230-239`): max 3 with `next_id` 4, 3 and 0. The empty document is checked with `next_id` 10, not 0 | partial (CRITICAL 2; WARNING 3) |
| Id `u64::MAX` (`spec.md:102`) | `id_rows` (`validate.rs:241-245`) uses a **page** with id `u64::MAX`. The scenario says a node | covered by a page; no node row (WARNING 3) |
| Size edges (`spec.md:111`) | `page_rows` (`validate.rs:261-270`) | covered, but page 1 only (CRITICAL 1) |
| Limit follows the document DPI (`spec.md:116`) | `page_rows` `a0` at 300, 350 and 351 (Height) | covered |
| Non-finite or zero size (`spec.md:126`) | `page_rows`: NaN width (`Conversion`, `validate.rs:277-286`), `0.0` width, `-5.0` **height**. No `+inf` | partial (WARNING 3) |
| Dangling and duplicate roots (`spec.md:131`) | `root_rows` (`validate.rs:290`): 42, `[3, 3]`, and a root that lives only in page 7 | covered, but page 1 only (CRITICAL 1) |
| Pure JSON recursion limit (`document-serde/spec.md:263`) | `serde::a_bare_value_parses_to_127_levels_and_not_beyond` (`tests/serde.rs:282`): 127 ok, 128 and 10 000 fail with "recursion limit" | covered |
| Payload depth, parse side (`document-serde/spec.md:268`) | `serde::a_document_payload_parses_to_121_levels_and_not_beyond` (`tests/serde.rs:294`): 121 ok, 122 is `LoadError::Json` | covered |

Type contract, checked by reading and by mutation:
- `#[non_exhaustive]` is at `error.rs:51`, and `Clone, Debug, PartialEq` with no `Eq` at `error.rs:52`. Adding `Eq` does not compile, because `OutOfRange` carries `f64`.
- The type is hand-written: `Display` at `error.rs:100-152`, `Error::source` at `error.rs:154-161`.
- The 18 variants are declared in pass order (`error.rs:53-98`) and match `design.md:284-298`.
- `validate.rs` has no recursion.

### 4. Mutation results

There were 61 mutants: 47 killed, 1 equivalent and 13 surviving. Of the 13 survivors, 12 are in scope (CRITICAL 1-3), and E01 is covered by WARNING 2.

Killed:
- `error.rs`: E02 (`Eq`, compile error), E03-E20 (each of the 18 `Display` texts loses or swaps an id or field), E21 (`source()` returns `None`), E22 (`MAX_EXTENSION_DEPTH` 65), E23-E25 (`NodeField` names).
- P1: V01 and V02 (schema `<` and `>`), V03 (no schema check), V04-V06 (no DPI check, zero only, max only).
- P2: V07 (no duplicate check), V08 and V09 (node ids or page ids left out of the set), V10 (no key check), V11 (key and id swapped), V12 (`next_id <` instead of `<=`), V13 (no `next_id` check), V15 (`max` replaced by `min`), V33 (the wrong duplicate id is reported).
- P3: V16 (no size check), V17 (no dangling check), V18 (dangling root accepted if it exists in any page), V19 (no duplicate-root check), V32 (size checked at 96 DPI).
- Order: V21 (P3 before P2).
- Parse side: T01 and T02 (`nested` off by one in either direction).

Equivalent: F01 (`serde_json` feature `unbounded_depth`). The feature only exposes `disable_recursion_limit()`, so `from_str` keeps the 128 limit.

Surviving:

| Id | Mutant | Rule broken |
|---|---|---|
| V14 | `match Some(ids().max().unwrap_or(0))`: an empty document rejects `next_id` 0 | "A document with no ids MUST accept any `next_id`" (`spec.md:85`, scenario `:97`) |
| V24 | arena-key check only on the first page | "the arena key of every entry" (`spec.md:85`) |
| V25 | page-size check only on the first page | "`PageSize::check(dpi)` for each page" (`spec.md:109`) |
| V26 | dangling-root check only on the first page | "each id in `roots`" (`spec.md:109`) |
| V27 | duplicate-root check only on the first page | same |
| V28 | `PageSize` reports `pages[0].id` | the variant names the offending page |
| V29 | `DanglingRoot` reports `pages[0].id` | same |
| V30 | `DuplicateRoot` reports `pages[0].id` | same |
| V31 | `ArenaKeyMismatch` reports `pages[0].id` | same; the scenario names the page (`spec.md:95`) |
| V20 | `check_ids` before `check_header` | "earliest pass" (`spec.md:47`) |
| V22 | `next_id` check before the duplicate check (P2) | "within a pass the first in the stated order" (`spec.md:47`) |
| V23 | DPI check before the schema check (P1) | same |
| E01 | `#[non_exhaustive]` removed | cannot be killed by an in-crate runtime test (WARNING 2) |

### 5. Code rules, hygiene, size

- The added code has no `#[allow]`, `unsafe`, `HashMap` or `HashSet`. `unwrap`, `panic!` and `expect` appear only inside `#[cfg(test)]` and `tests/`. `BTreeSet` is used in `validate.rs:3`.
- The four new `pub` items are exported in the commit that adds them: `ValidationError`, `NodeField` and `MAX_EXTENSION_DEPTH` at `lib.rs:26-31`, and `validate` is a method. None is dead code.
- The five subjects are conventional (`feat(doc)` x2, `test(doc)`, `docs(sdd)` x2). No message has a `Co-Authored-By` or AI attribution line. There are no `proptest-regressions`.
- Size: `git diff --shortstat feat/domain-doc-3b..HEAD -- . ':!openspec'` gives 5 files changed, 653 insertions(+), 1 deletion(-). That matches `apply-progress.md:303`. The breakdown is `validate.rs` 310, `error.rs` 300, `tests/serde.rs` 30, `node.rs` 7 and `lib.rs` 6/1. Against ~507 the ratio is 1.29. The fixes for CRITICAL 1 to 3 add about 30-40 lines, which stays under ~700.

### CRITICAL

1. **P2 and P3 are only tested with the defect on the first page (V24-V31 survive).** Every row in `id_rows`, `page_rows` and `root_rows` (`validate.rs:217-309`) breaks page 1, which is `pages[0]`. The code could check only the first page, or name the first page instead of the offending one, and every test would still pass. The spec requires the check on every arena entry and on every page, and requires the error to name the page (`spec.md:85,95,109`). Fix: add rows that put each defect on a second page (key mismatch, size, dangling root, duplicate root) and expect `page: PageId(<second>)`.
2. **The empty-document `next_id` row is missing (V14 survives).** Scenario `spec.md:97-100` says "an empty document with `next_id` 0 ... pass". `id_rows` clears the pages but keeps `next_id` 10 (`validate.rs:239`). `Document::default()` uses `next_id` 1. Nothing checks that an empty document accepts `next_id` 0. Fix: `(|d| { d.pages.clear(); d.next_id = 0 }, "Ok(())")`.
3. **Pass order has no test in D4a (V20, V22, V23 survive), and the order rows left D4a without owner approval.** The requirement "Pass order, first error wins" is still tagged D4a (`spec.md:45`). In the owner-approved re-cut commit, its first two cases (schema with a duplicate id, and a duplicate id with a page-size error) were D4a. The progress commit moved them to D4b, citing the size rule (`apply-progress.md:294`), but the delivery measures 653 and the rows fit under ~700. No test checks P1 before P2, or the order of the checks inside a pass, so a swapped order passes. Fix: add the two cross-pass rows from `spec.md:51` plus within-pass rows (schema 2 with DPI 0 gives `UnsupportedSchemaVersion`; a duplicate id with `next_id` 0 gives `DuplicateId`; and for P3, page 1 with a dangling root and page 2 with a bad size gives `PageSize` on page 2, which pins the check-major reading). Restore the D4a tags. Alternative: the owner explicitly ratifies the retag. Then V20, V22 and V23 become D4b obligations.

### WARNING

1. **TDD process: no assertion-level RED for the validation tests.** By `apply-progress.md:287-288`, `error.rs` and `validate.rs` were RED only as compile errors (missing types, missing `validate`). They "passed on the first run after the code". `design.md:429` expects T4.2a RED to be "Table test fails". Tests and code are in the same commit, so the history cannot show RED. The two parse-side tests in `tests/serde.rs:282,294` are characterization tests over existing behavior, so RED is impossible for them. Mutation was used instead, and the writer's set is reproduced above as killed, but it missed the 12 survivors in CRITICAL 1-3. For the fix commits, write each new row first and record it failing against a mutant.
2. **`#[non_exhaustive]` is not guarded by a test (E01 survives).** It is present at `error.rs:51`. Earlier deliveries accepted this by inspection (`PageSizeError`, D1 table). A `compile_fail` doctest that matches all 18 variants without a wildcard, as `ids.rs:42` does for another rule, would guard it.
3. **Scenario rows that differ from the spec.** None of these leaves a mutant alive, but each is a gap against the scenario as written:
   - "Non-finite or zero size" lists NaN, `+inf`, `0.0` and `-5.0` as widths. There is no `+inf` row, and `-5.0` is applied to the height (`validate.rs:272`). `PageSize::check` covers infinity at `page_size.rs:259`, but not through `validate()`.
   - "next_id bound" also lists `next_id` 5 (below the max, not 0 and not equal). There is no such row.
   - "Id u64::MAX" asks for a node with that id. The test uses a page (`validate.rs:242`).
4. **"Determinism (property)" (`spec.md:40`) has no delivery tag** under a D4a requirement, and no test covers it. It needs the D6a corruption generators, so tag it D6b.
5. **The pass-order reading is not stated in the spec.** The code runs P1-P3 check-major across the document: all duplicates, then key mismatches, then `next_id`; all sizes, then dangling roots, then repeated roots. That matches `spec.md:47` ("within a pass the first in the stated order", with node-by-node order stated only for P4) and `design.md:302-307`, so it is **not** a CRITICAL deviation. However, "pages in `pages` order" could also be read page-major, and the reading is recorded only in `apply-progress.md:295`. Add one sentence to `spec.md:47` saying P1-P3 are check-major, and pin it with the P3 row from CRITICAL 3.

### SUGGESTION

1. Comparing `Debug` strings (`validate.rs:149-154`, rows like `"Err(DuplicateId { id: 1 })"`) is acceptable. The derived `Debug` prints every field, so it is as strict as `==` here, and `PartialEq` is tested on its own (`error.rs:290-291`). It is fragile, though. A custom `Debug` for `NodeId`, `PageId` or `PageSizeError`, or a change in float `Debug` output (the `out` helper, `validate.rs:157-160`), breaks every row without any behavior change. Value rows with short constructor helpers (`dup(1)`, `root(1, 42)`) would compare by `PartialEq` and stay one line each.
2. `MAX_EXTENSION_DEPTH` arriving in D4a is fine. It is used by `Display` (`error.rs:136`, so E22 is killed), it is public and declared in `node.rs` exactly as `design.md:243` describes, and it is not dead code. The comment at `design.md:243` still says `// D4b`; change it to D4a.
3. The doc comment at `validate.rs:10-12` says that inside a pass the checks run in variant order. In D4b, reword it for the node-by-node P4.
4. `design.md:432` still says "together with T4.2" after the split into T4.2a and T4.2b.

### Result

- Verdict: **FAIL** for D4a (3 CRITICAL, 5 WARNING, 4 SUGGESTION).
- All gates pass on each of the five commits: fmt, clippy, test, `cargo deny check`, ARQ-07, V11, V5 and V12.
- The re-cut matches the owner-approved cut, except that the progress commit later moved the order rows (CRITICAL 3).
- Mutants: 47 of 61 killed, 1 equivalent (F01), 13 surviving. All `ValidationError` messages, `source()`, the 18-variant count, no-`Eq` and the parse-side depth tests are proven.
- Size 653 (ratio 1.29).
- Next: **tdd-apply D4a** with fix commits for CRITICAL 1-3 (rows on a second page, the empty document with `next_id` 0, and the order rows, or an owner ratification of the retag). WARNING 3 and 4 can go in the same commits. Then re-verify D4a. D4b waits.

## D4a re-verify (the 3 fix commits from `test(doc): pin page naming, pass order and empty-document rows` to `docs(sdd): restore the D4a order rows and tag determinism D6b`; delivery `feat/domain-doc-3b..feat/domain-doc-4a`)

**Verdict: FAIL** (D4a). CRITICAL 1, WARNING 2, SUGGESTION 3. All three previous CRITICAL items are closed, and so are WARNING 2-5 and the four suggestions. All gates pass on each new commit. Two new within-pass order mutants survive (CRITICAL 1); two rows fix them. `next` = **tdd-apply D4a** (one test commit), then re-verify D4a.

Method, same as the D4a verify:
- Each commit was extracted with `git archive` into its own directory, with its own `CARGO_TARGET_DIR`. ARQ-07, V11, V5 and V12 were parsed from that commit's `ci.yml`.
- The range changes no `.github`, `Cargo.toml`, `Cargo.lock`, `clippy.toml` or `deny.toml` file.
- Mutants ran on a scratch copy of HEAD with `cargo test -p gafira-doc --locked`, which includes doctests. Each mutant was restored after its run.

### 1. Gates per commit

| Commit | fmt | clippy | test (passed) | `cargo deny check` | ARQ-07 | V11 | V5 | V12 |
|---|---|---|---|---|---|---|---|---|
| `test(doc): pin page naming, pass order and empty-document rows` | 0 | 0 | 0 (263) | 0 | 0 | 0 | 0 | 0 |
| `test(doc): guard non_exhaustive on ValidationError with a compile_fail doctest` | 0 | 0 | 0 (264) | 0 | 0 | 0 | 0 | 0 |
| `docs(sdd): restore the D4a order rows and tag determinism D6b` | 0 | 0 | 0 (264) | 0 | 0 | 0 | 0 | 0 |

- `cargo deny check` reports advisories, bans, licenses and sources ok on all three.
- The new doctest `error::ValidationError (line 55) - compile fail ... ok` appears from the second commit on.

### 2. Previous items

| Item | Status | Evidence |
|---|---|---|
| CRITICAL 1 (second page) | closed | These rows put the defect on page 7, and each expects `PageId(7)`: `two(d, rekey)` (`validate.rs:262`), size (`:307`), dangling root (`:332`) and repeated root (`:337`). V24-V31 are killed. |
| CRITICAL 2 (empty document, `next_id` 0) | closed | `validate.rs:267-273` clears the pages, sets `next_id` 0 and expects `Ok(())`. V14 is killed. |
| CRITICAL 3 (order rows) | closed | `earlier_rules_win` (`validate.rs:348-391`) has the two D4a "Earlier pass wins" cases plus the three within-pass rows. V20, V22 and V23 are killed. The tags at `spec.md:45,49` and `design.md:429-430` are byte-identical to the owner-approved re-cut `docs(sdd): re-cut domain-doc D4 and D6 into sub-deliveries`. |
| W1 (no assertion-level RED) | accepted | The fix rows were proven by mutation (`apply-progress.md:316-334`) and are reproduced below. |
| W2 (`#[non_exhaustive]` not guarded) | closed | See the doctest check below. |
| W3 (rows differ from the spec) | closed | `next_id` rows use max 10 with 11, 10, 5 and 0 (`validate.rs:263-266`). Node `u64::MAX` is tested at `:275`, page `u64::MAX` at `:274`. The `-5.0` width is at `:303`. NaN and `+inf` widths go through `validate()` (`:309-319`). |
| W4 (determinism untagged) | closed | Tagged at `spec.md:40` "Determinism (D6b, property)". |
| W5 (pass-order reading) | closed | The check-major sentence is at `spec.md:47`, the doc comment is at `validate.rs:10-13`, and the P3 row is at `validate.rs:382-388`. |
| S1-S4 | closed | S1: value helpers `next`, `root`, `key` and `out` (`validate.rs:182-198`); `Debug` comparison is kept, which is acceptable. S2: `design.md:243` now says D4a. S3: the doc comment already names P4 node by node. S4: `design.md:432` now says T4.2a and T4.2b. |

Doctest check (ED3: stable rustdoc does not check the error code, so the doctest must fail for the right reason):
- The doctest body, compiled as an example against HEAD, fails with only `error[E0004]: non-exhaustive patterns: '_' not covered`.
- With `#[non_exhaustive]` removed it compiles with no error or warning.
- The test therefore fails only because of the attribute, and mutant E01 is killed by `error.rs - error::ValidationError (line 55)`.

### 3. Mutation summary (68 mutants)

- The original 61: 60 killed. F01 (`unbounded_depth`) is equivalent, as before. The 13 previous survivors are all killed: E01, V14, V20, V22, V23 and V24-V31.
- New mutants:

| Id | Mutant | Result |
|---|---|---|
| V34 | `next_id` max over page ids only | killed (`id_rows`, node `u64::MAX` row) |
| V35 | P3 page-major (all three checks per page) | killed (`earlier_rules_win`) |
| V36 | `next_id` max over node ids only | killed (`id_rows`, page `u64::MAX` row) |
| V37 | P3 dangling-root loop before page size | killed (`earlier_rules_win`) |
| V38 | P3 repeated-root loop before dangling-root loop | **survived** |
| V39 | P2 arena-key loop before duplicate-id check | **survived** |
| V40 | key check skips the last page | killed (`id_rows`) |

### 4. Spec and design consistency

- The order rows match the owner-approved plan. "Earlier pass wins" covers its first two cases in D4a and the third in D4b (`spec.md:49`). T4.2a has the P1-P3 order rows and T4.2b only the P4 order rows (`design.md:429-430`).
- Determinism is D6b (`spec.md:40`).
- The pass-order sentence is at `spec.md:47`.
- The new scenario "Earlier rule wins inside a pass (D4a)" (`spec.md:54-57`) is consistent with `apply-progress.md:285,299-300` and with the code.
- `design.md:300` still says only "pages in `pages` order" (SUGGESTION 1).

### 5. Code rules and production code

- In `src/` outside `#[cfg(test)]`, the fix commits change only doc comments: `validate.rs:10-13` and `error.rs:51-67`. `check_header`, `check_ids` and `check_pages` are byte-identical to `docs(sdd): record domain-doc D4a progress`.
- The added lines have no `#[allow]`, `unsafe`, `HashMap`, `HashSet`, `todo!` or `dbg!`. `unwrap` and `panic!` appear only in tests.
- Subjects are conventional. No message has a `Co-Authored-By` or AI attribution line. There are no `proptest-regressions`.
- The docs commit also commits the previous D4a verify section, as intended.

### 6. Size and split

- `git diff --shortstat feat/domain-doc-3b..HEAD -- . ':!openspec'` gives 5 files changed, 752 insertions(+), 1 deletion(-). That matches `apply-progress.md:338`.
- Code-only size by prefix:

| Prefix ending at | Code-only size |
|---|---|
| `feat(doc): add ValidationError` | 312 |
| `feat(doc): validate header, ids and pages` | 623 |
| `test(doc): pin the parse-side depth limits for payloads` | 653 |
| `test(doc): pin page naming, pass order and empty-document rows` | 735 |
| `test(doc): guard non_exhaustive on ValidationError with a compile_fail doctest` | 752 |

- The ratio to the ~507 forecast is 1.48, or about 1.50 after the CRITICAL 1 rows.
- See WARNING 1 for the split decision.

### CRITICAL

1. **Two within-pass orders of a D4a MUST are unproven (V38 and V39 survive).** The rule at `spec.md:47` says: "within a pass the first in the stated order", with P2 ordered uniqueness, arena key, `next_id`, and P3 ordered page size, roots exist, no root listed twice. The tests pin schema before DPI, duplicate id before `next_id`, and page size before dangling root. They do not pin:
   - the position of the arena-key check after duplicate ids, so swapping them passes;
   - the repeated-root check after the dangling-root check, so swapping them passes.

   I missed these two in my earlier fix list.

   Fix: add two rows to `earlier_rules_win`:
   - `push(d, 1, &[]); rekey(first(d))` expects `Err(DuplicateId { id: 1 })` (kills V39);
   - `roots(first(d), &[42, 42])` expects `DanglingRoot` on page 1, node 42 (kills V38).

   Optionally list both cases in `spec.md:55-57`. The rows add about 10 lines.

### WARNING

1. **Size 752 is over the ~700 guideline.** By the letter of the rule, a split is warranted, but only marginally: 7% over, and about 9% after CRITICAL 1. The only sensible boundary is after `feat(doc): add ValidationError`:
   - PR A is `docs(sdd): re-cut ...` plus `feat(doc): add ValidationError`, with 312 code lines. Every prefix is gate-green, verified on that commit in the D4a verify.
   - PR B is the rest, from `feat(doc): validate header, ids and pages` to HEAD, with about 440 code lines plus the CRITICAL 1 rows.
   - Cutting after `feat(doc): validate header, ids and pages` (623 / 129) would ship `validate.rs` without the tests that kill V14 and V20-V31.

   Keeping one PR is defensible, because the overrun is test rows. Splitting or not is the owner's decision.
2. **D5 fallback trigger.** D4a alone is at a ratio of about 1.48-1.50. The D5 fallback cut fires if D4a plus D4b exceed 1.5x (`design.md:367`), so it is likely. Record the combined ratio after D4b.

### SUGGESTION

1. Mirror the check-major sentence in `design.md:300` ("Validation passes" heading line), which still says only "pages in `pages` order".
2. Tag "Ascending node order within a pass" (`spec.md:69`) as D4b. It is a P4 scenario (NaN `x`) under a requirement tagged "D4a; P4 D4b".
3. The `rekey` doc comment (`validate.rs:155`) says "the highest node". The second-page row depends on that: node 9 under key 5. A short note that key 5 must not collide with an existing id would help future rows.

### Result

- Verdict: **FAIL** for D4a (1 CRITICAL, 2 WARNING, 3 SUGGESTION).
- Gates pass on each of the three fix commits: fmt, clippy, test, `cargo deny check`, ARQ-07, V11, V5 and V12.
- Mutants: 65 of 68 killed, 1 equivalent (F01), 2 in-scope survivors (V38, V39).
- Production code is unchanged apart from doc comments. The specs match the owner-approved plan.
- Next: **tdd-apply D4a**, one test commit with the two rows from CRITICAL 1. Then re-verify D4a. The owner decides the split (WARNING 1) before the PR.
