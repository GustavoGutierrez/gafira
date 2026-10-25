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

## D4a third verify

Range `feat/domain-doc-3b..feat/domain-doc-4a` after the rebase onto `chore/lite-sdd-methodology` (README plus one `AGENTS.md` line; code trees unchanged). New commits: `876bc6f test(doc): pin duplicate id before arena key and dangling before repeated root`, `c592f63 docs(sdd): record the D4a second verify fix and mirror the check-major rule`. The rebase changed every SHA, so every commit in the range was gated again.

### Gates

Each commit was extracted alone (`git archive`) with its own `CARGO_TARGET_DIR`. Steps: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, plus ARQ-07, V11 (forbid unsafe), V5 and V12 run from that commit's own `ci.yml`. All 8 steps returned rc=0 on every commit; no test failed.

| # | Commit | Tests passed |
|---|---|---|
| 1 | `453cebb` docs(sdd): re-cut D4 and D6 | 252 |
| 2 | `ab1e753` feat(doc): add ValidationError | 255 |
| 3 | `3e2911f` feat(doc): validate header, ids and pages | 260 |
| 4 | `b1e0813` test(doc): parse-side depth limits | 262 |
| 5 | `30ba40a` docs(sdd): D4a progress | 262 |
| 6 | `9fbe53e` test(doc): page naming, pass order, empty document | 263 |
| 7 | `07cd4b4` test(doc): non_exhaustive compile_fail doctest | 264 |
| 8 | `2f29db7` docs(sdd): restore order rows, tag D6b | 264 |
| 9 | `876bc6f` test(doc): duplicate before key, dangling before repeated | 264 (2 new rows in the existing `earlier_rules_win` table) |
| 10 | `c592f63` docs(sdd): second verify fix | 264 |

`cargo deny check`: advisories ok, bans ok, licenses ok, sources ok on every commit.

### Mutants

All 68 mutants (the original 61 plus V34-V40) ran on HEAD: 67 killed, 1 survived. The survivor is F01 (`unbounded_depth`), which is equivalent. No pattern was missing.
- V38 (repeated-root loop before dangling-root loop) and V39 (arena-key loop before duplicate-id check) now fail `validate::tests::earlier_rules_win`. Both were killed by a failing test, not a build error.
- V34, V36 and V40 fail `id_rows`. V35 and V37 fail `earlier_rules_win`.
- E01 fails the `compile_fail` doctest. E02 (derive `Eq`) is killed by a build error, as in the earlier verifies.

### Docs and code

- `876bc6f` changes only `validate.rs` lines 155 and 382-394, both inside `mod tests` (which starts at `validate.rs:93`). Production code (`validate.rs:1-91`) is unchanged.
- `design.md:300` states the check-major rule (all duplicate ids, then key mismatches, then `next_id`; all page sizes, then dangling roots, then repeated roots; only P4 node by node). This matches `spec.md:47`, `apply-progress.md:300` and the code order at `validate.rs:43-87`.
- `spec.md:69`: "Ascending node order within a pass" is tagged `(D4b, unit)`.
- `apply-progress.md:340-351` lists V38 and V39 with the correct killing rows and reports size 765. Both match this run.
- All three suggestions from the second verify are done, including the `rekey` comment at `validate.rs:155`.

### Size and split

- Code only, `git diff --shortstat feat/domain-doc-3b..HEAD -- . ':!openspec'`: 5 files changed, 765 insertions(+), 1 deletion(-). This matches `apply-progress.md:351`.
- Split after `feat(doc): add ValidationError`:
  - PR A is `453cebb..ab1e753`: 312 code lines added, 1 deleted (`error.rs` 300, `lib.rs` 6, `node.rs` 7). Including openspec it is 9 files, +410/-87.
  - PR B is `ab1e753..HEAD`: 453 code lines added, 0 deleted (`validate.rs` 405, `serde.rs` 30, `error.rs` 17, `lib.rs` 1). Including openspec it is 8 files, +816/-6.
- Every prefix of both PRs (commits 1-10) is gate-green; see the table above.

### CRITICAL

None.

### WARNING

1. **Size 765 is over the ~700 guideline (`AGENTS.md:38`)** by about 9%. The split above is clean: A is 313 code lines, B is 453, and every prefix is green. Keeping one PR is still defensible because the overrun is test rows. The owner decides.
2. **D5 fallback trigger (`design.md:367`).** D4a alone measures 765 against a forecast of ~507, a ratio of 1.51. That is already above x1.5 before D4b, so the D4a plus D4b check will probably fire the D5a/D5b cut. Record the combined ratio after D4b.

### SUGGESTION

1. The "Commits" list in `apply-progress.md:264-273` stops at commit 8, and "Gates" (`apply-progress.md:310-312`) does not record runs for commits 6-10. Add commits 9 and 10 and the per-commit gate results from this verify.
2. `apply-progress.md:351`, "Size: see the final report line from ...", reads awkwardly. Stating the number directly is enough.
3. `design.md:300` puts a capitalized sentence after "order):". Folding it into the parenthetical, or starting a new sentence, would read better. This is cosmetic.

### Result

- Verdict: **PASS** for D4a (0 CRITICAL, 2 WARNING, 3 SUGGESTION).
- Gates are green on all 10 commits of the range, including the two new ones.
- Mutants: 67 of 68 killed, 1 equivalent (F01), 0 in-scope survivors.
- Production code is unchanged apart from comments.
- Next: the owner decides the split (WARNING 1) and opens the PR or PRs. Then **tdd-apply D4b**.

## D4b (branch `feat/domain-doc-4b`, range `feat/domain-doc-4a..feat/domain-doc-4b`, 3 commits from `feat(doc): validate node fields and extension depth` to `docs(sdd): record domain-doc D4b progress`)

**Verdict: FAIL** (D4b). CRITICAL 1, WARNING 4, SUGGESTION 3. All gates pass on each of the three commits, and cargo-mutants reports 0 missed and 0 timeout. Two hand mutations of the P4 check order survive (CRITICAL 1). Four table rows fix them. `next` = **tdd-apply D4b** (one test commit), then re-verify D4b.

Method:
- Each commit was extracted with `git archive` into its own scratch directory, with its own `CARGO_TARGET_DIR`. The ARQ-07, V11 ("Domain crates forbid unsafe code"), V5 and V12 steps were parsed from that commit's `ci.yml`, which is byte-identical across the range.
- Mutation follows the new policy in `AGENTS.md` ("Testing and mutation policy"). cargo-mutants ran `--in-diff` (see the `mutants` agent report). Verify hand-mutated only the spec rules that cargo-mutants cannot express (6 mutations, listed below).

### Gates per commit

| # | Commit | fmt | clippy `-D warnings` | test `--workspace --locked` | `cargo deny check` | ARQ-07 | V11 | V5 | V12 |
|---|---|---|---|---|---|---|---|---|---|
| 1 | `feat(doc): validate node fields and extension depth` | ok | ok | ok, **270** passed (doc lib 98, serde 13, golden 3, doctests 2) | ok | ok | ok | ok | ok |
| 2 | `feat(doc): validate on every deserialization` | ok | ok | ok, **274** passed (doc lib 99, serde 16, golden 3, doctests 2) | ok | ok | ok | ok | ok |
| 3 | `docs(sdd): record domain-doc D4b progress` | ok | ok | ok, 274 passed | ok | ok | ok | ok | ok |

The count of 274 after commit 2 matches `apply-progress.md`. Commit 1 had not been totaled before; it is 270. The other crates are unchanged: cli 5+6, color 63+10+21, geom 31+7+11. `cargo deny check` gives "advisories ok, bans ok, licenses ok, sources ok" on all three commits.

### Mutants

- cargo-mutants (`mutants` agent report, `--in-diff` against `feat/domain-doc-4a`): 47 mutants, 42 caught, **0 missed, 0 timeout**, 5 unviable (`LoadError::source` with a leaked default, three `numbers` constant-vector replacements, `&&`→`||` in the `if let` chain). By file, caught: `validate.rs` 37, `json.rs` 4, `document.rs` 1. Process note from that report: `--cargo-arg=--locked` conflicts with `additional_cargo_test_args = ["--locked"]` in `.cargo/mutants.toml`, so the run dropped the flag. Tests still ran with `--locked`. The `AGENTS.md` flag line should be corrected (SUGGESTION 3).
- Hand mutations (run on a scratch copy of HEAD with `cargo test -p gafira-doc --locked`; the copy was restored and diffed clean afterwards):

| Id | Spec rule | Mutation | Result |
|---|---|---|---|
| H1 | P4 order: 9 `NonFinite` before 10 `OutOfRange` | swap the two `if let` blocks (`validate.rs:161-170`) | killed: `node_value_rows`, `nodes_are_checked_in_ascending_id_and_check_order` |
| H2 | P4 order: 13 `ChildrenOnLeaf` before 14 `DanglingChild` | check dangling children before the leaf check (`validate.rs:180-185`) | **survived** |
| H3 | P4 order: 11 `EmptyExtensionTypeId` before 12 `ExtensionTooDeep` | compute depth and reject before the empty `type_id` check (`validate.rs:172-178`) | **survived** |
| H4 | Ascending node order, and the third "Earlier pass wins" case | `page.nodes.values().rev()` in `check_nodes` (`validate.rs:99`) | killed: `earlier_rules_win`, `nodes_are_checked_in_ascending_id_and_check_order` |
| H5 | Depth boundary 64/65 | `depth > MAX_EXTENSION_DEPTH + 1` (`validate.rs:176`) | killed: `extension_rows` |
| H6 | Spec scenario "Revert mutation is caught" (`try_from` effectively `from`) | delete `doc.validate()?;` in `TryFrom<DocumentWire>` (`json.rs:48-60`) | killed: `json::tests::load_errors_...` and 4 `serde` tests, including `plain_serde_cannot_build_an_invalid_document` |

Not run, but the result is clear from the tests: swapping check 10 with 11 (an extension with an out-of-range value and an empty `type_id`) and check 12 with 13 (a too-deep extension that has children) would also survive. No test row combines those two defects on one node.

The "Very deep payload" scenario asks for a recursive-walk mutation to be recorded, not exercised. Recorded here: `depth` (`validate.rs:191-205`) is iterative (explicit stack), as the code shows. A recursive rewrite would still pass the 1 000-level row. The spec accepts this.

### Requirement and scenario coverage

| Requirement / scenario | Evidence | Status |
|---|---|---|
| Node values: opacity edges (0.0, -0.0, 1.0, 1e-45 pass) | `validate.rs:570` `node_value_rows` rows 0-3 | covered |
| Opacity out of range (3 × `OutOfRange`, NaN/±inf → `NonFinite`) | `node_value_rows` rows 4-9 | covered |
| Placement values table (X, Y, RotationDeg, Skew `NonFinite`; W, H, Skew ±90 `OutOfRange`) | `node_value_rows` rows 10-17 | covered |
| Placement values accepted (w ±0, rotation 720, skew ±89.999) | `node_value_rows` rows 18-22 | covered |
| Fill values (NaN channel, NaN alpha, alpha 1.5, extended RGB passes) | `node_value_rows` rows 23-26 (plus `LinearSrgb` rows at `:667`, `:674`) | covered |
| Extension `type_id` ("" fails, "x" passes) | `validate.rs:702` `extension_rows` (tail assertion and every "x" row) | covered |
| Payload depth at the limit (64, 65 arrays, 65 objects, scalar, `[]`) | `extension_rows` rows 0-4; H5 killed | covered |
| Very deep payload (1 000 levels, iterative) | `extension_rows` row 5 (`:714`); see note above | covered |
| Children on a leaf (Shape, Extension), dangling 99, child only in another page | `validate.rs:732` `child_rows` rows 0-3 | covered |
| Not clamped (opacity 1.5 bit-identical after the error) | `validate.rs:693` `validation_does_not_clamp` | covered |
| `validate()` MUST NOT recurse (depth walk) | `validate.rs:191-205` | covered (by code) |
| Pass order, P4 node by node, first in the stated order (9 to 14) | ascending id: H4 killed; 9 before 10: H1 killed; 11 before 12, 13 before 14: **H2, H3 survive**; 10 before 11, 12 before 13: no row | **partially covered → CRITICAL 1** |
| Earlier pass wins, third case (opacity on node 2 vs dangling child on node 3 → `OutOfRange`) | `validate.rs:482-489`, a row in `earlier_rules_win` | covered |
| D4b build accepts cycles | `validate.rs:820` `a_self_parenting_group_still_passes_until_d5` | covered (D5 inverts it) |
| Ascending node order (NaN x on nodes 3 and 8 → node 3) | `validate.rs:776` row 0 | covered |
| Each rule has its own variant, rows 9-14 | `node_value_rows`, `extension_rows`, `child_rows` give `NonFinite`, `OutOfRange`, `EmptyExtensionTypeId`, `ExtensionTooDeep`, `ChildrenOnLeaf`, `DanglingChild` | covered |
| Valid documents (group with rectangle and extension) now pass through P4 | `validate.rs:317` `valid_documents_pass` | covered |
| Validating load: `try_from = "DocumentWire"` | `document.rs:17`, `json.rs:48-60`; H6 killed | covered |
| `DocumentWire` not public | `pub(crate) struct DocumentWire` (`json.rs`) | covered |
| Plain serde cannot build an invalid document | `tests/serde.rs:332` | covered |
| Shape versus validation (truncated → `Json`; schema 2 → `Invalid(UnsupportedSchemaVersion { found: 2 })`) | `json.rs:327` `load_errors_name_their_cause_and_expose_the_inner_error` | covered |
| `LoadError` error contract (`Invalid` and `Json`: display and `source()`) | `json.rs:327` (`Invalid`); `json.rs:241` `unreadable_bytes_are_json_errors_with_a_source` (`Json`, from D3) | covered |
| Revert mutation is caught | H6 | covered |
| Finite out-of-range survives serde, not load (D4b half) | `tests/serde.rs:345` row `"opacity":1.5` (observed `Invalid(OutOfRange { node: 3, Opacity, 1.5 })`) | covered |
| Hostile values (dpi 0, 10001, schema 0 → `Invalid`; unknown `space` → `Json`) | `tests/serde.rs:345` rows 0-2 and the `space` tail | covered |
| Duplicate id across pages and against a page id (integration) → `Invalid(DuplicateId)` | `tests/serde.rs:345` rows 4-5 assert only `Invalid(_)`; row 4 is a duplicate **page** id, not a node id in two page arenas | **partial → WARNING 1** |
| Payload depth inside a full document (121 → `Invalid(ExtensionTooDeep { depth: 121 })`, 122 → `Json`) | `tests/serde.rs:294-307` | covered |
| Deep payload rejected by validation and by load; 64 reopens identical | `extension_rows`; `tests/serde.rs:311` | covered (SUGGESTION 1) |
| Document root: `from_parts` accessors and page order 3, 1, 2; unknown → `None` | `document.rs:167` `from_parts_keeps_page_order_and_looks_pages_up` | covered |
| `from_parts` validates (`next_id` 1, page 5 → `NextIdNotAboveMax`) | `document.rs:181` `from_parts_validates` (exact variant, `next_id: 1, max_id: 5`) | covered |
| D3 fixtures still pass through `try_from` | `tests/golden.rs` (3) and `tests/serde.rs` (the D3 tests) unchanged and green on commit 2; `json.rs` D3 unit tests green with the `doc_with` fix | covered |
| `from_json` returns the same variant as `validate()` | `json.rs:65-68` (deserializes `DocumentWire`, `try_from`, `map_err(LoadError::Invalid)`); the observed variants equal the `validate()` variants | covered |

### Out-of-spec fixture fix in `json.rs` (`doc_with`, `next_id` 3 → 20)

Correct and justified. `doc_with` (`json.rs:101-109`) builds a page from the given nodes, and `round_trip_is_idempotent_and_independent_of_build_order` passes `group(10)`. With `next_id = 3` that document breaks P2 (`NextIdNotAboveMax { next_id: 3, max_id: 10 }`), so it could only round-trip through the unchecked D3 `from`. The highest id any caller passes is 10, so 20 is valid for all of them. No byte-exact expectation built from `doc_with` contains `"next_id":3`; the only such literal is `minimal_document_has_exact_bytes` (`json.rs:122`), which uses `test_document(3, ..)` with no nodes and stays valid. The fix restores the spirit of the D3 fixture rule ("every fixture is a valid document") and does not weaken any assertion. The two other D3-era rewrites are justified too: the `schema_version` 2 round-trip test became the shape-versus-validation test, and `page_lookup_and_order` now builds through `from_parts`.

### Code rules

- No `#[allow]`, no `HashMap`/`HashSet`, no `unsafe` added. The new `unwrap()` calls are all inside tests.
- No AI attribution lines in the three commit messages.
- No `proptest-regressions` directory is tracked.
- The `design.md` change in commit 3 only lowercases "In" after "order):" (D4a SUGGESTION 3).

### Size

- Code only, `git diff --shortstat feat/domain-doc-4a..HEAD -- . ':!openspec'`: 5 files changed, 553 insertions(+), 23 deletions(-), so 576 changed lines. This matches `apply-progress.md`. Commit 1 alone is 423/3. The ratio to the ~387 forecast is 1.49. Under the ~700 guideline.
- D4a plus D4b, summed per delivery: 766 + 576 = 1342 against 894, a ratio of **1.501**.
- Cumulative diff `feat/domain-doc-3b..HEAD`: 1314/20, which is 1334 changed lines (1.492), because D4b rewrites a few D4a lines.
- The CRITICAL 1 rows add about 15 lines, which puts both measures at or above 1.5. See WARNING 4.

### CRITICAL

1. **Two P4 check orders of a MUST are unproven (H2 and H3 survive). Two more have no row.** `spec.md:47` says that for each node "its checks in the listed order (9 to 14)", and `design.md:307` says the same. The tests pin ascending ids (H4), and they pin `NonFinite` before `OutOfRange` (H1). They do not pin:
   - 11 before 12: an `Extension` with `type_id = ""` **and** a 65-level payload must give `EmptyExtensionTypeId`. Today depth-first passes all tests (H3).
   - 13 before 14: a `Shape` with `children = [99]` (absent) must give `ChildrenOnLeaf`. Today dangling-first passes all tests (H2), because every leaf row uses child 3, which exists (`validate.rs:735-740`).
   - 10 before 11: an `Extension` with `type_id = ""` and `opacity = 2.0` must give `OutOfRange` (Opacity).
   - 12 before 13: an `Extension` with a 65-level payload and `children = [3]` must give `ExtensionTooDeep`.

   Fix: add these four rows, for example in `nodes_are_checked_in_ascending_id_and_check_order` (`validate.rs:776`). Each row must fail with its mutation applied. This follows the D4a precedent (V38/V39).

### WARNING

1. **Load-side duplicate-id scenario only partly proven** (`tests/serde.rs:345-366`). The scenario requires `LoadError::Invalid(DuplicateId { .. })` for "two pages whose arrays both contain id 9" and for "a node id equal to a page id". The rows assert only `Invalid(_)`. Row `"id":10,` → `"id":1,` makes two **pages** with id 1, not a node id shared by two arenas. The observed results are correct (`DuplicateId { id: 1 }` and `DuplicateId { id: 10 }`), and the in-memory cross-arena case is covered in `validate.rs` (D4a `id_rows`). Still, the integration THEN clause is not asserted. Assert the variant for those rows, and add a JSON row with a node id present in two pages' `nodes`.
2. **Test scope: `Display` text pinned beyond the spec** (`json.rs:334`, `:336`). The spec asks only that both errors "display text" and that `source()` returns the inner error. The exact prefix `"invalid document: "` is non-critical text that the spec does not fix. Asserting non-empty text plus the `source()` identity is enough.
3. **Test scope: redundant rows.** `extension_rows` row `json!([[[[0]]]])` (`validate.rs:716`) adds nothing beyond the 64-level pass row. `node_value_rows` row `f32::MAX` blue (`validate.rs:679-683`) repeats the extended-range acceptance already proven by `r = -0.5, g = 1.75`. Low cost; consider removing them.
4. **D5 fallback trigger is at the boundary** (`design.md:367`). The summed ratio is 1.501 now and will be about 1.52 after the CRITICAL 1 rows. The cumulative-diff ratio is 1.492 now and about 1.51 after the fix. Either way the "> ×1.5" condition holds after the fix, so expect the D5a/D5b cut after D5's first commit. The owner decides.

### SUGGESTION

1. `tests/serde.rs:316-319` compares the reloaded 64-level document with `PartialEq`, but the scenario says "reloads bit-identical". The payload holds no floats, so this is sound. Also comparing `to_json()` bytes before and after the reload would state the claim literally.
2. `apply-progress.md` "Gates": record commit 1's total (270) and the `cargo deny`/ARQ-07/V5/V11/V12 results from this verify. Add the hand-mutation table once CRITICAL 1 is fixed.
3. `AGENTS.md` mutation flags: `--cargo-arg=--locked` conflicts with `.cargo/mutants.toml` (`additional_cargo_test_args = ["--locked"]`). Drop the flag from the documented command (owner/coordinator change; outside this delivery's code).

### Result

- Verdict: **FAIL** for D4b (1 CRITICAL, 4 WARNING, 3 SUGGESTION).
- Gates are green on all three commits: fmt, clippy, test (270 / 274 / 274), `cargo deny check`, ARQ-07, V11, V5 and V12.
- Mutants: cargo-mutants 42/47 caught, 0 missed, 0 timeout, 5 unviable. Hand mutations: 4/6 killed, and H2 and H3 survive (CRITICAL 1).
- Next: **tdd-apply D4b**, one test commit with the four P4 order rows (CRITICAL 1), ideally with WARNING 1 and 2. Then re-verify D4b.

## D4b re-verify (the 4 fix commits from `test(doc): pin the per-node check order of P4` to `docs(sdd): record domain-doc D4b verify fixes`; delivery `feat/domain-doc-4a..feat/domain-doc-4b`)

**Verdict: PASS** (D4b). CRITICAL 0, WARNING 1, SUGGESTION 1. The stack was rebased onto a methodology commit that touches only `.agents/agents/mutants.md` and `AGENTS.md`. `next` = **tdd-apply D5**, after the owner decides on the D5a/D5b cut (WARNING 1).

Method: each commit was checked out as a detached worktree in the scratchpad, with its own `CARGO_TARGET_DIR`. The ARQ-07, V11, V5 and V12 steps were parsed from that commit's `ci.yml` and run. Hand mutations were applied as `patch` files to a scratch worktree of HEAD and reverted after each run. The worktree diffed clean afterwards.

### Gates per commit

| # | Commit | fmt | clippy `-D warnings` | test `--workspace --locked` | `cargo deny check` | ARQ-07 | V11 | V5 | V12 |
|---|---|---|---|---|---|---|---|---|---|
| 1 | `test(doc): pin the per-node check order of P4` | ok | ok | ok, **274** (doc lib 99, serde 16, golden 3, doctests 2) | ok | ok | ok | ok | ok |
| 2 | `test(doc): assert the duplicate-id variant on load and compare reopened bytes` | ok | ok | ok, **275** (serde 17) | ok | ok | ok | ok | ok |
| 3 | `test(doc): drop display-prefix and redundant validation rows` | ok | ok | ok, 275 | ok | ok | ok | ok | ok |
| 4 | `docs(sdd): record domain-doc D4b verify fixes` | ok | ok | ok, 275 | ok | ok | ok | ok | ok |

The other crates are unchanged: cli 5+6, color 63+10+21, geom 31+7+11. The totals match `apply-progress.md` (274, 275, 275).

### Previous items

| Item | Fix | Status |
|---|---|---|
| CRITICAL 1 (P4 check order) | Rows 3 to 6 of `nodes_are_checked_in_ascending_id_and_check_order` (`validate.rs:806-840`). Re-applied as hand mutations (table below); each one fails exactly its row | **closed** |
| WARNING 1 (load duplicate id) | `tests/serde.rs:368-391` `duplicate_ids_are_invalid_on_load` asserts `Invalid(DuplicateId { id })` with the exact id for three rows: two pages with id 1; node 5 renamed to 10, which equals page id 10; and node 5 (`/pages/0/nodes/3`) also pushed into page 10's `nodes`, so the id is in two arenas. This matches `specs/document-serde/spec.md:253-256` (id 5 instead of 9; P2 fires before any P4 issue of the copy). The two `Invalid(_)` rows were removed from `well_formed_documents_that_break_a_rule_are_invalid_not_json`, with no loss of coverage | **closed** |
| WARNING 2 (display prefix) | `json.rs:334-340`: non-empty text, plus `source()` downcasts to `UnsupportedSchemaVersion { found: 2 }` | **closed** |
| WARNING 3 (redundant rows) | `json!([[[[0]]]])` and the `f32::MAX` blue row were removed. Each rule they touched is still covered elsewhere: the 64-level pass, the mixed object/array pass, the scalar and `[]` rows (`extension_rows`, `validate.rs:697`), and extended RGB passes (`validate.rs:661`, `-0.5`/`1.75`). H5 (64/65 boundary) is still killed by `extension_rows`, per cargo-mutants (0 missed) | **closed** |
| WARNING 4 (D5 split trigger) | Measured below. The ratio is now clearly above 1.5 | open, owner decision (WARNING 1 below) |
| SUGGESTION 1 (bytes on reopen) | `tests/serde.rs:318-321` also compares `to_json()` before and after the reload | done |
| SUGGESTION 2 (progress gates) | `apply-progress.md` records 270/274 and the fix-commit totals, plus the hand-mutation table | done |
| SUGGESTION 3 (`--locked` flag) | `AGENTS.md` "Testing and mutation policy" now says not to pass `--locked` again (methodology commit in the base) | done |

### Mutants

- cargo-mutants (`mutants` agent report `mutants-D4b-r2`, `--in-diff`, gafira-doc): 47 found, 42 caught, **0 missed, 0 timeout**, 5 unviable. Wall time about 2 min. These are the same 47 as the first run; the fix commits change only tests.
- Hand mutations (4, all spec ordering rules that cargo-mutants cannot express; `cargo test -p gafira-doc --locked`):

| Id | Rule | Mutation | Result |
|---|---|---|---|
| R1 | 10 `OutOfRange` before 11 `EmptyExtensionTypeId` | empty `type_id` check moved above the range check | killed (row 3): left `EmptyExtensionTypeId`, right `OutOfRange { Opacity, 2.0 }` |
| H3 | 11 before 12 `ExtensionTooDeep` | depth check above the empty `type_id` check | killed (row 4): left `ExtensionTooDeep { depth: 65 }`, right `EmptyExtensionTypeId` |
| R2 | 12 before 13 `ChildrenOnLeaf` | leaf check moved above the extension block | killed (row 5): left `ChildrenOnLeaf`, right `ExtensionTooDeep { depth: 65 }` |
| H2 | 13 before 14 `DanglingChild` | dangling-child check inserted above the leaf check | killed (row 6): left `DanglingChild { child: 99 }`, right `ChildrenOnLeaf` |

Together with H1 and H4 from the first D4b verify, every adjacent pair in the P4 order (9 to 14) and the ascending node order is now pinned.

### Production code and code rules

- `git diff 2a03826..079cc95 -- . ':!openspec'` touches only `json.rs` from line 331 (`mod tests` starts at `:77`), `validate.rs` from line 676 (`mod tests` starts at `:207`) and `tests/serde.rs`. Production code is unchanged.
- No `#[allow]`, `HashMap`/`HashSet` or `unsafe` added. No AI attribution in any of the 7 commit messages of `feat/domain-doc-4a..HEAD`. No tracked `proptest-regressions`.

### Size

- Code only, `git diff --shortstat feat/domain-doc-4a..HEAD -- . ':!openspec'`: 5 files, 611 insertions, 23 deletions, so **634** changed lines (ratio 1.64 to ~387). This matches `apply-progress.md` and is under ~700.
- D4a plus D4b, summed per delivery: 766 + 634 = 1400 against 894, a ratio of **1.566**, which confirms the reported figure. The cumulative diff `feat/domain-doc-3b..HEAD` is 1372/20 = 1392 (1.557).

### CRITICAL

None.

### WARNING

1. **D5 fallback trigger fires** (`design.md:367`). The ratio is 1.566 (summed) or 1.557 (cumulative), both above 1.5. Plan the D5a/D5b cut. The owner decides.

### SUGGESTION

1. `tests/serde.rs:383` `assert_ne!(text, FULL)` cannot fail for the `shared_node.to_string()` row, because re-serialized compact JSON with an extra node always differs from the golden. The real guard is the exact `DuplicateId { id: 5 }` match. Harmless; no change needed.

### Result

- Verdict: **PASS** for D4b (0 CRITICAL, 1 WARNING, 1 SUGGESTION).
- Gates are green on all four fix commits: fmt, clippy, test (274 / 275 / 275 / 275), `cargo deny check`, ARQ-07, V11, V5 and V12.
- Mutants: cargo-mutants 42/47 caught, 0 missed, 0 timeout, 5 unviable. Hand mutations: 4/4 killed (R1, H3, R2, H2).
- Next: the owner decides the D5 split and opens the D4b PR. Then **tdd-apply D5** (D5a first if cut).

## D5a (branch `feat/domain-doc-5a`, range `feat/domain-doc-4b..feat/domain-doc-5a`, 3 commits from `docs(sdd): cut domain-doc D5 into D5a and D5b` to `docs(sdd): record domain-doc D5a progress`)

**Verdict: PASS** (D5a). CRITICAL 0, WARNING 4, SUGGESTION 3. The change is not complete (D5b, D6a, D6b remain), so `next` = **tdd-apply D5b**.

Method: each commit was checked out as a detached worktree in the scratchpad, with its own `CARGO_TARGET_DIR`. The ARQ-07, V11, V5 and V12 steps were parsed from that commit's `ci.yml` and run. Hand mutations were applied to a scratch worktree of HEAD and reverted after each run. All worktrees were removed afterwards.

### Gates per commit

| # | Commit | fmt | clippy `-D warnings` | test `--workspace --locked` | `cargo deny check` | ARQ-07 | V11 | V5 | V12 |
|---|---|---|---|---|---|---|---|---|---|
| 1 | `docs(sdd): cut domain-doc D5 into D5a and D5b` | ok | ok | ok, **275** (no code change) | ok | ok | ok | ok | ok |
| 2 | `feat(doc): validate parents and reachability` | ok | ok | ok, **278** (doc lib 101, serde 18) | ok | ok | ok | ok | ok |
| 3 | `docs(sdd): record domain-doc D5a progress` | ok | ok | ok, 278 | ok | ok | ok | ok | ok |

The totals match `apply-progress.md`. The slowest suite is `tests/serde.rs` at 0.22 s (debug), which includes the 10 000-group load. The 10 000-group unit test runs inside the 0.04 s doc lib suite.

### D5 cut (docs commit)

The commit applies the owner-approved cut. D5a is P5 (15 to 17) plus P6 (18) plus the 10 000-deep chain, and D5b is `EditToken`/`DocMut` (which removes the `alloc_id` gate) plus V13. The change now has eleven sub-deliveries, and the labels agree across `design.md` (A5, gate table, file map, P5/P6 rows, size table, chain, branches, commit plan, T5.1 to T5.3, Spec Alignment, Q list), `proposal.md:68` and the delivery table, and all five delta specs. No untagged `D5` is left. Stale items: WARNING 3 and SUGGESTION 1.

### Requirements and scenarios

| Requirement / scenario | Evidence | Status |
|---|---|---|
| Parents and reachability (`document-validation/spec.md:195`): P5 counts edges per page, 15/16/17 and their payloads | `validate.rs:110-139` (`check_parents`), `:162-168` (`parent_edges`) | covered |
| P6 is an iterative DFS that names the lowest unreached id, with no recursion in either pass | `validate.rs:142-158`: an explicit `Vec` stack and a `BTreeSet`, and `keys().find` returns the lowest id. No call in either pass recurses (checked by reading the code). See WARNING 1 | covered |
| Second parent (two groups; one group twice) `:199` | `parent_and_reachability_rows` (`validate.rs:937`) rows 2 and 3 | covered |
| Root with a parent `:204` | same test, row 4 (`RootHasParent{3,2}`) and row 5 (a root that lists itself) | covered |
| Orphan `:209` | rows 6 and 7 (the second on page 2) | covered |
| Cycles (non-root self-parent, ring, subtree below a ring) `:214` | rows 8 to 10. Row 10 expects `Cycle(2)`, a node off the ring | covered |
| Each rule has its own variant, rows 15 to 18 `:30` | the four distinct variants in the same table | covered |
| Earlier pass wins over parents and cycles `:59` | `parent_checks_and_passes_run_in_order` (`validate.rs:985`) rows 4, 6 and 7 (`OutOfRange`, `Orphan`, `MultipleParents`) | covered |
| Pass order inside P5 (15 before 16 before 17) and across pages | the same test, rows 0 to 3. See WARNING 2 | covered |
| D4b build accepts cycles, now inverted `:64` | The D4b test was removed. Its document (a root that lists itself) is row 5 (`RootHasParent`), and the non-root self-parent is row 8 (`Cycle`) | covered |
| Deep chain without overflow `:221` | `a_deep_chain_validates_without_overflow` (`validate.rs:1046`) | covered (see WARNING 1) |
| Deep tree, integration (`document-serde/spec.md:278`) | `tests/serde.rs:395` loads the chain, then loads it again without roots and expects `Invalid(Orphan)`, which shows the load ran P5 | covered |
| Fixture rule (every D3 fixture is valid under P5/P6) | the full suite passes on commit 2 (golden 3, serde 18) | covered |

### Declared deviations

1. **Self-parenting scenario text** (`spec.md:64-67`, changed in the docs commit). This fixes an inconsistent spec. It does not change behavior. The D4b fixture's group is a root. The normative text already decides the result: a root inside any `children` list gives `RootHasParent` (`:197`), P5 runs before P6 (`:47`), and the Cycles scenario already says "not a root, so it is its own only parent" (`:215`). No owner decision is needed. Note: the old line said "removed or inverted", not `Cycle`, so `apply-progress.md` misquotes it (SUGGESTION 2).
2. **P5 inner order (check-major)**. The order is not settled explicitly. `spec.md:47` says "within a pass the first in the stated order" and then spells out check-major for P1 to P3 and node-by-node for P4. P5 gets no such sentence. The design's P5 row (`design.md:308`) only lists 15, 16, 17 and, unlike P4 (`:307`), does not say "node by node". The natural reading of "first in the stated order" plus the P1 to P3 precedent supports the writer's choice. Rows 0 to 3 pin it. WARNING 2: the owner should confirm it.
3. **`child_rows` fixture** (`validate.rs:820`). The old "Ok" row made node 3 both a root and a child, which is `RootHasParent` under P5. The new tree (root 2, child 3) keeps the row's purpose: a group with a child in the same arena passes. Correct.
4. **10 000-chain unit test was not RED**. This is acceptable as a guard. Strict TDD RED is shown by the other three tests (recorded in `apply-progress.md`). P1 to P4 never recurse, so this test could not fail before the change. However, see WARNING 1: it also cannot fail under a recursive DFS.

### Mutants

- cargo-mutants (`mutants` agent report `mutants-D5a`, `--in-diff`, gafira-doc): 17 found, 14 caught, **0 missed, 0 timeout**, 3 unviable, wall time 43 s. The caught mutants cover `validate`/`check_parents`/`check_reachability` replaced by `Ok(())`, `>` to `>=`/`==`/`<`, `&&` to `||`, deleted `!`, `parent_edges` stubs and `+=` to `*=`/`-=`.
- Hand mutations (6; `cargo test -p gafira-doc --locked`):

| Id | Rule | Mutation | Result |
|---|---|---|---|
| M1 | 15 before 16 | the `RootHasParent` loop moved above the `MultipleParents` loop | killed (`parent_checks_and_passes_run_in_order`) |
| M2 | 16 before 17 | the `Orphan` loop moved above the `RootHasParent` loop | killed (same test) |
| M3 | P5 before P6 | `check_reachability` called before `check_parents` | killed (`parent_and_reachability_rows`, `parent_checks_...`) |
| M4 | P4 before P5 | `check_parents` called before `check_nodes` | killed (`child_rows`, `nodes_are_checked_...`, `parent_checks_...`) |
| M5 | lowest unreached id | `keys().rev().find` (highest unreached id) | killed (`parent_and_reachability_rows`, `parent_checks_...`) |
| M6 | "both passes MUST work without recursion" | the DFS replaced by a recursive `visit` | **survived**: both 10 000-chain tests pass in debug and release (WARNING 1) |

### Production code and code rules

- No `#[allow]`, `HashMap`/`HashSet`, `unsafe`, `expect`, `panic!` or `todo!` added. `BTreeMap`/`BTreeSet` are used. No AI attribution in the 3 commit messages. No `proptest-regressions` added.
- Test scope: no accessor or `Display` tests were added. The second `Ok` row of `parent_and_reachability_rows` (roots 2 and 3, edge 2 to 4) overlaps the first `Ok` row and `child_rows:820` (SUGGESTION 3). The `Orphan` assertion in `tests/serde.rs` is not redundant: without it, the load test would also pass on D4b.

### Size

Code only, `git diff --shortstat feat/domain-doc-4b..HEAD -- . ':!openspec'`: 2 files, 257 insertions, 9 deletions, so **266** changed lines (ratio 1.24 to ~215). This matches `apply-progress.md` and is under ~700.

### CRITICAL

None.

### WARNING

1. **The 10 000-chain tests do not prove "without recursion"** (`spec.md:197`, `:221`; `validate.rs:1046`; `tests/serde.rs:395`). A recursive DFS (M6) passes both tests, because 10 000 frames fit in the 2 MiB test-thread stack. The current code is iterative (checked by reading it), and the scenarios are met as written: the spec chose 10 000 as the evidence. Not FAIL. Suggestion for the owner or D6: run the unit chain inside `std::thread::Builder::new().stack_size(64 * 1024)`, or raise the depth, so that a recursive regression fails.
2. **P5 inner order is chosen, not specified** (`spec.md:47`, `design.md:308`). The code is check-major (all `MultipleParents` over all pages, then `RootHasParent`, then `Orphan`), and rows 0 to 3 pin it. The owner should confirm. Then add one sentence, like the P1 to P3 one, to `spec.md:47` (before archive).
3. **Stale count in `proposal.md:110`**: the risk table still says "Ten sub-deliveries". The docs commit missed it. Make it "Eleven".
4. **Change total arithmetic** (`design.md:373`, `proposal.md:68`, `design.md:582`): the new sum 595 + 335 + 465 + 690 + 507 + 387 + 215 + 420 + 540 + 365 is **4 519**, not 4 517 (D5a plus D5b is 635, not the former 633, as `design.md:368` itself says). Docs only.

### SUGGESTION

1. The docs commit also changed the meaning of the self-parent scenario (deviation 1), not only its labels. That is correct, but the cut entry in `apply-progress.md` lists only relabeling. Mention the scenario fix there next time.
2. `apply-progress.md` (D5a, Deviations) says the old scenario said "then fails with `Cycle`". It said "removed or inverted". Reword it.
3. Drop or merge the second `Ok` row of `parent_and_reachability_rows` (overlaps row 0 and `child_rows:820`). Harmless.

### Result

- Verdict: **PASS** for D5a (0 CRITICAL, 4 WARNING, 3 SUGGESTION).
- Gates are green on all three commits: fmt, clippy, test (275 / 278 / 278), `cargo deny check`, ARQ-07, V11, V5 and V12.
- Mutants: cargo-mutants 14/17 caught, 0 missed, 0 timeout, 3 unviable. Hand mutations: 5/6 killed. M6 (recursive DFS) survived, which is WARNING 1.
- Next: the owner confirms the P5 order (WARNING 2) and opens the D5a PR. Then **tdd-apply D5b**. WARNINGs 3 and 4 can ride along with D5b's docs.

## D5b (branch `feat/domain-doc-5b`, range `main..feat/domain-doc-5b`, base `main` at 777231d)

Commits checked: "chore: let the coordinator push branches", "docs(sdd): settle P5 order and fix D5a verify items", "test(doc): prove the reachability walk is not recursive", "feat(doc): add EditToken and DocMut", "ci: restrict EditToken to the edit crates", "docs(sdd): record domain-doc D5b progress", "chore: add git-workflow-and-versioning and the integration rule". During this verify the owner added "chore(skills): vendor git-workflow-and-versioning as a project skill" (`.agents/skills/git-workflow-and-versioning/SKILL.md`, `skills-lock.json`; no code). It was checked for paths and rule conflicts only. The owner then added "docs: show the logo and splash screen in the README" (`README.md` only; both image paths exist, relative, no attribution). Gates do not depend on it.

### Gates per commit

Each commit was checked out alone in its own worktree with its own target directory. Every CI step body was extracted from that commit's `ci.yml` and run under `bash --noprofile --norc -eo pipefail`.

| Commit | fmt | clippy `-D warnings` | test (passed / failed) | `cargo deny check` | ARQ-07 | V11 (forbid unsafe) | V5 | V12 | V13 |
|---|---|---|---|---|---|---|---|---|---|
| test(doc): prove the reachability walk is not recursive | ok | ok | 278 / 0 | ok | ok | ok | ok | ok | absent |
| feat(doc): add EditToken and DocMut | ok | ok | 295 / 0 | ok | ok | ok | ok | ok | absent |
| ci: restrict EditToken to the edit crates | ok | ok | 295 / 0 | ok | ok | ok | ok | ok | rc 0 |
| chore: add git-workflow-and-versioning and the integration rule (tip) | ok | ok | 295 / 0 | ok | ok | ok | ok | ok | rc 0 |

295 = 278 - 1 removed unit test + 10 in `tests/edit.rs` + 8 `compile_fail` doctests. This matches `apply-progress.md`. The docs-only commits change no code. Note: a first run that shared one target directory across worktrees failed clippy on stale fingerprints (all worktrees were created at the same moment, so the sources looked older than the artifacts). That was a harness artifact; the per-commit target re-run above is the result.

### V13 (re-run by verify)

The `ci.yml` body is byte-identical to the design block (`design.md:150-173`, 24 lines, `diff` empty). It was run from a clone of the branch, git 2.34.1. Untracked scratch files were used and reverted.

| Case | Result | Spec |
|---|---|---|
| E0 clean tree | rc 0 | ok |
| E10 no `crates/gafira-edit/` | rc 0, stdout and stderr empty | ok |
| E1 `// EditToken` in `apps/gafira-cli/src/main.rs` | rc 1, names the file | ok |
| E2 constructor name in a `gafira-color` test | rc 1 | ok |
| E3 `pub struct EditTok {` | rc 1, positive-control message | ok |
| E4a pattern `EditToken(` | rc 1, `git grep failed (rc=128)` | ok |
| E4b `allowed='^crates/(gafira-doc'` | rc 1, `filter grep failed (rc=2)` | ok |
| E5 run from `crates/`; non-repository copy | rc 1 (positive control); rc 1 (`rc=128`) | ok |
| E6 `pub use gafira_doc::EditToken as Cap;` | rc 1, re-export message | ok |
| E7a one-line group, E7b multi-line group, E8 `pub type`, E9 glob | rc 1 on each, re-export message | ok |
| E12 `pub(crate) use` | rc 0 | ok |
| E14 mention in `gafira-edit` without re-export, and in `gafira-testkit` (`crates/gafira-doc/tests/edit.rs` is in the real tree) | rc 0 | ok |
| E11 two-step alias, E13 `lend`, E15a `pub use gafira_doc as d;`, E15b `pub use gafira_doc::edit;` | rc 0 on each | known limits 1, 2 and 6, as specified |

**Known limits of V13, recorded as NOT met** (`edit-capability/spec.md`, "V13 CI step"): (1) two-step alias; (2) a public function, field or wrapper handing out `EditToken`/`&EditToken`; (3) macro-generated re-export; (4) `;` in a comment or string splitting a record; (5) inference in a consumer that never names the token; (6) crate or module re-export. Review covers them until `trybuild` arrives in C3.

**Deferred to C3:** the `trybuild` ARQ-30 compile-failure test from `gafira-ui`. No third-party package was added: `Cargo.lock` gains only `gafira-doc` against the pre-change lock (parent of "feat(doc): add gafira-doc crate skeleton"), and `main..HEAD` does not touch `Cargo.lock` or any manifest.

### Requirements and scenarios (edit-capability, D5b)

| Requirement / scenario | Evidence | Status |
|---|---|---|
| EditToken: no `Clone`/`Copy`/`Default`/`Debug`, private field, only the hidden constructor, no other public fn returns it | `edit.rs:39-48`. The only `-> EditToken` is `edit.rs:45`. No `derive` on the struct. | covered |
| Only the allowed crates name it | V13 E0. `grep` finds no `EditToken` outside `crates/gafira-doc` | covered |
| Not Clone / Not Copy / No struct literal (compile-fail doctests) | `edit.rs:18`, `:25`, `:34` pass as `compile_fail`. Compiled by hand in a scratch crate: **E0599**, **E0382**, **E0451**, one error each (the intended reason). A positive control (`edit(&t).alloc_node_id()`) compiles. | covered (regression aids, ED3) |
| Hidden from the docs | `cargo doc -p gafira-doc --no-deps --locked`. `__new_for_gafira_edit_only` is absent from every file under `target/doc` outside `src/`; it is present only in `src/gafira_doc/edit.rs.html`, as expected. | covered |
| `Document::edit`/`DocMut` API (`alloc_*`, `set_dpi`, `pages_mut`, `page_mut`, `node_mut`, `as_doc`); exclusive borrow; no implicit validation | `edit.rs:94-146` | covered |
| Edit through the capability | `tests/edit.rs:57` `edit_builds_a_valid_document`. See WARNING 1 | covered |
| One token, many edits | `tests/edit.rs:71` | covered |
| No write without a token (`doc.dpi =`, `pages_mut`, `edit()`) | doctests `edit.rs:58`, `:65`, `:72`. By hand: **E0616**, **E0599**, **E0061** | covered (regression aids) |
| Shared `Arc` cannot be mutated | doctests `edit.rs:79`, `:87`. By hand: **E0616** (privacy, not mutability) and **E0308**. See SUGGESTION 2 | covered (regression aids) |
| Invalid state allowed until validated | `tests/edit.rs:82` (`Orphan`) | covered |
| Copy on write keeps sharing | `tests/edit.rs:148` (edited page and node differ; sibling node and the other page are `ptr_eq`). Hand mutations H2 and H3 are killed. | covered |
| Whole-value replacement documented | rustdoc `document.rs:13-16` and `edit.rs:52-56` (`*doc = other`, `EditService` in C3) | covered (manual) |
| Shared counter 5, 6, 7 then 8 | `tests/edit.rs:96` | covered |
| Last issuable id | `tests/edit.rs:107` | covered |
| Exhaustion, no wrap (node and page) | `tests/edit.rs:115`. Hand mutation H4 is killed. | covered |
| `IdSpaceExhausted` `Copy`/`Eq`/`Display`/`Error` | `ids.rs:118`, `ids.rs:124` (D2b) | covered |
| Allocation outside `DocMut` not public | `document.rs:73` `pub(crate) fn alloc_id`, ungated | covered |
| Readers hold `Arc<Document>`: copy-on-write handle | `tests/edit.rs:172` | covered |
| Changing the DPI (design file map) | `tests/edit.rs:135` | covered |
| V13 CI step (edit-capability and `workspace-tooling/spec.md:13`) | `ci.yml` V13 step, E-table above | covered (known limits not met, as specified) |
| `trybuild` deferred; lock unchanged except `gafira-doc` | see above | covered / deferred to C3 |

### D5a follow-ups and owner decisions

- D5a WARNING 1 (deep chain): `validate.rs:1049` now uses 100 000 groups. Hand mutation H1 (P6 replaced by a recursive `visit`) **aborts the test binary with a stack overflow in both debug and release** (`cargo test` rc 101, SIGABRT). The test takes 0.46 s in debug and 0.07 s in release. The deviation is accepted: `std::thread::Builder` is banned in domain crates (ARQ-02) and `#[allow]` is forbidden, so a deeper chain is the only lever. The failure is an abort, not a `FAILED` line, but CI still goes red.
- D5a WARNING 2 (P5 order): `document-validation/spec.md:47` now says P5 is check-major over all pages, and the design's P5 row (`design.md:308`) states the same. Applied.
- D5a WARNINGs 3 and 4: "Eleven" is at `proposal.md:110`, and 4 519 is at `design.md:373`, `:582` and `proposal.md:68`. Applied. D5a SUGGESTION 2 (misquote) is applied at `apply-progress.md:452`. SUGGESTIONS 1 and 3 were not taken (harmless).
- Push rule: `AGENTS.md:50-51`, `CLAUDE.md:10`, `coordinator.md:20` and `:25`, and `README.md:57` agree (the coordinator pushes and opens the PR after verify PASS; the owner merges, bottom-up, with merge commits, and a PR is retargeted to `main` after its base merges). `.claude/settings.json` has no `git push` deny. See WARNING 2 for one stale line.
- Paths: `git grep -n '/home/\|~/'` hits two binary assets and `openspec/changes/workspace-scaffold/design.md:333` (`~/.cargo/registry/...`). That line predates this branch and is a generic Cargo-home path, not a user path (SUGGESTION 4). The vendored skill and `skills-lock.json` contain no such paths.

### Declared deviations

1. Deep chain at 100 000 instead of a small-stack thread: accepted (see above, H1).
2. `Document::edit` in `edit.rs`, and `alloc_id` stays `pub(crate)` with the gate removed: accepted. This matches the design's dead-code table (`design.md:66`). The design sketch at `design.md:264` still shows `#[cfg(test)]` with the note "gate removed in D5b", which is consistent.
3. `allocator_increments_then_exhausts` removed: accepted. Its three assertions (value, increment, exhaustion without change) are covered by `tests/edit.rs:96`, `:107` and `:115` through the public API, and H4 proves the exhaustion path.
4. `node_mut` on a missing node in an existing page copies the page on write: accepted. It changes no value, and the spec's sharing rule is about the edited path. A caller that probes for missing nodes un-shares one page; that is a performance note, not a contract breach.

### Mutants

- cargo-mutants (`mutants-D5b`, `--in-diff`, gafira-doc): 13 mutants, 6 caught, **0 missed, 0 timeout**, 7 unviable (wall time 36 s). Caught: `set_dpi` -> `()`, `pages_mut` -> leaked empty vec, `page_mut` -> `None`, `==` -> `!=` in `page_mut`, `node_mut` -> `None`, `as_doc` -> leaked default. Unviable: `Default` on `EditToken`/`DocMut`/`NodeId`/`PageId`/`Page`/`Node` (no such impls).
- `alloc_id`'s body is not a changed line (only its `#[cfg(test)]` gate was removed), so `--in-diff` did not mutate it. It is covered by the three allocation tests and by H4. The archive whole-crate run will mutate it.
- Hand mutations (4; only rules cargo-mutants cannot express):

| Id | Rule | Mutation | Result |
|---|---|---|---|
| H1 | P6 "without recursion" (D5a W1) | iterative DFS replaced by a recursive `visit` | killed (stack overflow abort, debug and release) |
| H2 | copy on write along the edited path only | `page_mut` first calls `Arc::make_mut` on every page | killed (`an_edit_copies_only_its_path`) |
| H3 | same, at node level | `node_mut` un-shares every node of the page | killed (same test) |
| H4 | exhaustion leaves `next_id` unchanged | counter wraps, then returns `Err` | killed (`an_exhausted_counter_fails_and_does_not_wrap`) |

### Code rules and test scope

- No `#[allow]`, `HashMap`/`HashSet`, `unsafe`, `expect`, `panic!` or `todo!` in production code (the `unwrap`/`todo!` lines are inside `compile_fail` doctests). The only crate-root change is `mod edit;` plus the re-export (`lib.rs:12`, `:26`). No `proptest-regressions`. No AI attribution in any commit message.
- Test scope: no accessor, derive or `Display` tests were added. `a_missing_page_or_node_is_none` tests the `None` contract and kills the `page_mut` `==` -> `!=` mutant. `a_cloned_node_handle_does_not_make_the_document_mutable` mostly exercises `std::sync::Arc`, but the spec requires the scenario, so it is kept. `sample()` carries `assert_ne!(first, second)` (`tests/edit.rs:51`), which is redundant with `ids_come_from_one_counter` (SUGGESTION 3).

### Size

Code only, `git diff --shortstat main..HEAD -- crates .github`: 6 files, 362 insertions, 16 deletions, so **378** changed lines. Per commit: 4, 348 and 26. This matches `apply-progress.md` (ratio 0.9 to ~420), and it is under ~700.

### CRITICAL

None.

### WARNING

1. **"Edit through the capability" test does not follow the scenario's WHEN** (`tests/edit.rs:57`; spec "inserts a group as a root through `page_mut`"). `edit_builds_a_valid_document` builds the page with its root first and pushes it through `pages_mut` (`add_page`, `tests/edit.rs:30-38`). It never calls `page_mut`. Inserting a root through `page_mut` and then validating `Ok` is covered elsewhere: `sample()` at `:48-50`, plus `assert_eq!(doc.validate(), Ok(()))` at `:138`. So the behavior is proven, but the scenario maps to two tests. Add one `page_mut` root insertion to `edit_builds_a_valid_document` with D6a, or note the mapping.
2. **Stale skills row** `AGENTS.md:103`: "Preparing a PR (owner opens it) | `branch-pr`" contradicts the new rule at `AGENTS.md:50-51` (the coordinator opens PRs after verify PASS). Reword it to "Preparing a PR (coordinator opens it after verify PASS)".

### SUGGESTION

1. `design.md:365` and `:434` (T5.1) still say "10 000-deep chain" for the unit test, which is now 100 000 (`validate.rs:1049`). The spec's 10 000 (`document-validation/spec.md:222`) is still met. Align the design text with D6a's docs.
2. The `compile_fail` doctest at `edit.rs:79` fails on field privacy (E0616), not on the shared `Arc`: `Arc::get_mut` on a unique `Arc` would succeed. It is a regression aid (ED3), so this is acceptable. A clearer snippet clones the `Arc` and tries `doc.edit(..)` through `&*arc` (E0596).
3. Drop the redundant `assert_ne!(first, second)` in the `sample()` helper (`tests/edit.rs:51`).
4. `openspec/changes/workspace-scaffold/design.md:333` uses `~/.cargo/registry`. It is generic and predates this branch; `$CARGO_HOME/registry` would satisfy the no-user-paths rule literally.
5. The vendored `git-workflow-and-versioning` skill (owner commit) records source and hash in `skills-lock.json` but not its license. Note the upstream license next to the lock entry, to keep the permissive-only rule auditable.

### Result

- Verdict: **PASS** for D5b (0 CRITICAL, 2 WARNING, 5 SUGGESTION). D6a and D6b remain, so the change is not complete.
- Gates are green on every code commit and on the tip: fmt, clippy, tests 278 / 295 / 295 / 295, `cargo deny`, ARQ-07, V11, V5, V12, and V13 (once present). V13 E0 to E15 behave as specified, and limits 1 to 6 are recorded as not met. `trybuild` is deferred to C3.
- Mutants: cargo-mutants 6/13 caught, 0 missed, 0 timeout, 7 unviable. Hand mutations: 4/4 killed.
- Next: the coordinator pushes `feat/domain-doc-5b` and opens its PR against `main`. Then **tdd-apply D6a**. WARNINGs 1 and 2 and SUGGESTION 1 can ride along with D6a.

## D6a1 (branch `feat/domain-doc-6a`, range `feat/domain-doc-5b..feat/domain-doc-6a`, 2 commits: "docs: apply D5b verify follow-ups", "test(doc): add bounded document generator and its coverage guard")

Base note: `feat/domain-doc-5b` is merged on `origin/main` (PR #19, 32693ef). `feat/domain-doc-6a` is not on `main` or on `origin` yet, so its PR is still to be opened. The local `main` (777231d) is behind `origin/main`.

### Gates per commit

Each commit was checked out in its own worktree. Every policy step body was extracted from that commit's `ci.yml` and run, plus `cargo deny check`. The first pass shared one target directory, and on the later commits it reused stale fingerprints: they showed 296 tests at 159a451 and no corruption guard. That was a harness artifact. The re-run touched every `.rs` file first, so each commit was really rebuilt. The numbers below come from that re-run.

| Commit | fmt | clippy `-D warnings` | test (passed / failed) | `cargo deny` | ARQ-07 | V11 | V5 | V12 | V13 |
|---|---|---|---|---|---|---|---|---|---|
| f16015f docs: apply D5b verify follow-ups | ok | ok | 295 / 0 | ok | ok | ok | ok | ok | ok |
| 29f3b86 test(doc): add bounded document generator and its coverage guard | ok | ok | 296 / 0 | ok | ok | ok | ok | ok | ok |

296 = 295 + `generated_documents_reach_every_feature_within_the_bounds`. Workspace `cargo test` takes 6 to 8 s. No `proptest-regressions/` is committed or produced (`git ls-files`, and `git status` was clean after the runs). Commit authors and bodies carry no AI attribution. No `#[allow]`, `HashMap` or `HashSet` was added (the guards use `BTreeSet`). `EditToken` appears only under `crates/gafira-doc/` (V13 ok).

Size, code only (`git diff --shortstat feat/domain-doc-5b feat/domain-doc-6a -- crates .github`): 3 files, 492 insertions, 3 deletions = **495**. This matches `apply-progress.md` and `design.md:369`. It is under 700.

### D5b follow-ups (f16015f)

| Item | Evidence | Status |
|---|---|---|
| D5b WARNING 1: `page_mut` in "Edit through the capability" | `tests/edit.rs:57-73`: allocate ids, push an empty page through `pages_mut`, insert the root through `page_mut`, `validate() == Ok` | applied |
| D5b WARNING 2: PR row | `AGENTS.md:103` "Preparing a PR (coordinator opens it, owner merges)" | applied |
| D5b SUGGESTION 1: 100 000 in the design | `design.md:365` and T5.1 `:436` match `validate.rs:1049` (`DEPTH = 100_000`). The spec's 10 000 (`document-validation/spec.md:222`) is still met. | applied |
| D5b SUGGESTION 3: redundant `assert_ne!` | removed from `sample()` (`tests/edit.rs:41-53`) | applied |
| D5b SUGGESTION 4: `$CARGO_HOME` | `openspec/changes/workspace-scaffold/design.md:333` | applied |
| D5b SUGGESTION 5: skill license | `.agents/skills/git-workflow-and-versioning/SOURCE.md`: upstream, lock hash, MIT (from the GitHub license API), with a note that the upstream LICENSE text was not copied | applied (see SUGGESTION 2) |

### Requirements and scenarios (D6a1)

| Requirement / scenario | Evidence | Status |
|---|---|---|
| Generator is dev-only code in `tests/common`, built through `DocMut` with an `EditToken` | `tests/common/mod.rs:282-331` (`doc.edit(&token)`, `alloc_node_id`, `set_dpi`, `pages_mut`). Only `tests/properties.rs:3` includes `mod common;` | covered |
| 1 to 3 pages plus an empty-document case | `:209` `1..=MAX_PAGES`, `:334-339` `Just(Document::default())`. Guard asserts `<= 3` (`:768`) and reaches "empty document" | covered |
| 0 to 12 nodes per page; depth at most 4; only groups have children | `:181`, `:309-321`. Guard `:778`, `:790`, `:820`, `:824`, and `validate() == Ok` on every draw (`:767`) | covered (hand mutations H2, H3) |
| Ids from the counter in shuffled order, `next_id` gap 0 to 3 | `:289-297` (sort by random keys), `:292-294`. The guard checks only gap `<= 3` (`:828`) | covered by code; shuffle and gap > 0 not guarded (WARNING 1) |
| Payload depth at most 3, at most 4 entries, strings at most 8 chars including non-ASCII, quote, backslash and controls | `:75-104`. Guard `:821-822` checks depth, entries and lengths | bounds covered; character classes not guarded (WARNING 1) |
| Valid documents only | `validate() == Ok` on all 20 000 draws (`:767`) | covered |
| Scenario "Generator coverage guard" (20 000 deterministic draws reach `-0.0`, subnormal, px 1.0 and 16 384.0, opacity 0.0/-0.0/1.0, depth >= 3, extension, group with children, several pages, empty page) | `:763-847` (`TestRunner::deterministic()`, `DRAWS = 20_000`) | covered (hand mutation H1) |
| Scenario "Generated documents are valid" (property, 256 cases) | D6b (Q2). Already asserted per guard draw | deferred to D6b |
| "every corruption variant" in the guard list | D6a2 | see D6a2 |

### Deviation judged

- `tests/properties.rs` created in D6a1 holding only `mod common;` (design said D6b): **accepted**. Without a test target that includes `common`, the guard would not run and the generator would be dead code. `design.md:104` was updated in the D6a2 docs commit.

### Hand mutations (D6a1 scope; run on the tip worktree, reverted)

| # | Mutation | Result |
|---|---|---|
| H1 | Drop `Just(-0.0_f32)` from `opacity()` (`:39`): a feature never generated | killed: `never reached: ["opacity -0.0"]` (`:845`) |
| H2 | `vec(node_spec(), 0..=MAX_NODES + 1)` (`:181`): node bound off by one | killed: `page.nodes.len() <= MAX_NODES` (`:778`) |
| H3 | `depths[i] <= MAX_TREE_DEPTH` (`:311`): depth bound off by one | killed: `depth <= MAX_TREE_DEPTH` (`:790`) |

### WARNING

1. **Some generator MUSTs have no guard** (`tests/common/mod.rs:289-297`, `:828`, `:75-83`). Three things are never checked: that the shuffle actually reorders ids (replacing the `sort_by_key` at `:296` with the identity would survive, by inspection), that a gap greater than 0 is ever produced (removing the loop at `:292-294` would survive), and that the special characters are ever reached. The spec's guard list (`document-model/spec.md:311`) does not name them, so this is not a spec breach. D6b's Q1 relies on the shuffle ("build-order independence"). Add "ids out of order", "gap > 0" and "special char" to `wanted` with D6b. Test helper, non-critical.

### SUGGESTION

1. The `#[cfg(test)] mod guards` in `tests/common/mod.rs:707` is redundant in a test target. It is harmless, but any future target that adds `mod common;` will run both 20 000-draw guards again. Keep `common` included only from `properties.rs`, or say so in the module doc.
2. `SOURCE.md` takes MIT from the GitHub API without the license text. MIT requires the notice to travel with copies, so vendor the upstream `LICENSE` next to `SKILL.md`.

### Result

- Verdict: **PASS** for D6a1 (0 CRITICAL, 1 WARNING, 2 SUGGESTION).

## D6a2 (branch `feat/domain-doc-6a2`, range `feat/domain-doc-6a..feat/domain-doc-6a2`, 2 commits: "test(doc): add corruption generators and their guard", "docs(sdd): record domain-doc D6a and split it into D6a1 and D6a2")

### Gates per commit (same method as D6a1, rebuilt)

| Commit | fmt | clippy `-D warnings` | test (passed / failed) | `cargo deny` | ARQ-07 | V11 | V5 | V12 | V13 |
|---|---|---|---|---|---|---|---|---|---|
| 159a451 test(doc): add corruption generators and their guard | ok | ok | 297 / 0 | ok | ok | ok | ok | ok | ok |
| 64ff07e docs(sdd): record domain-doc D6a and split it into D6a1 and D6a2 (tip) | ok | ok | 297 / 0 | ok | ok | ok | ok | ok | ok |

297 = 296 + `every_corruption_is_produced_and_breaks_its_own_rule`. `cargo test -p gafira-doc --test properties` runs both guards in 6.3 s (debug, parallel), and the workspace in 9 to 10 s, which is normal test time. No `proptest-regressions/` is committed or produced. No AI attribution. No `#[allow]`.

Size, code only (`git diff --shortstat feat/domain-doc-6a HEAD -- crates .github`): 1 file, 385 insertions, 3 deletions = **388**. It is under 700. D6a1 plus D6a2 against `feat/domain-doc-5b` is 874 insertions and 3 deletions, which matches `apply-progress.md`.

### Requirements and scenarios (D6a2, `document-validation` "Validation properties" and the model guard list)

| Requirement / scenario | Evidence | Status |
|---|---|---|
| A corruption step for each of the 21 listed rules | `Corruption` (`tests/common/mod.rs:343-368`), 24 kinds: duplicate id split into across pages and page equals node, page size into below and above, `w`/`h` into two kinds | covered |
| Duplicate id across two pages or as page id = node id | `:527-542` | covered |
| Applying one yields exactly the matching variant | `matches` (`:399-429`). The `OutOfRange` kinds also check `field`. The guard asserts it on all 20 000 draws (`:853-854`) | covered for 23 kinds; `UnsupportedSchema` via `from_json` (WARNING 1) |
| Wire clause `from_json(to_json(d)) == Err(Invalid(same))` except `ArenaKeyMismatch` and `NonFinite` | `Corrupted.json` is produced (`:690-694`). Asserting it is Q3 in D6b | deferred to D6b |
| Scenario "Coverage guards and mutations": every variant reached by the guard | `:849-862` (deterministic, 20 000 draws, `never produced` check) | covered |
| Same scenario: the planned property mutations (drop P6, per-page parents, clamp opacity) | D6b | deferred to D6b |
| Scenarios "Corruption is detected exactly" and "Corruption survives the wire" (property) | D6b (Q3) | deferred to D6b |

### Deviations judged

- **`UnsupportedSchema` wire-only:** the implementation is **acceptable**, but the spec needs a fix (WARNING 1). No public path holds a `Document` with `schema_version != 1`. `schema_version` is `pub(crate)` with only a getter (`document.rs:20`, `:31`), and both `from_parts` and `Default` set `SCHEMA_VERSION` (`document.rs:62`, `:83`, `:95`). So patching the JSON and reading `from_json` (`tests/common/mod.rs:446-455`, `:691-693`) is the strongest observable test. The `replacen` is safe because `schema_version` is the first wire key (`json.rs:41`).
- **Out-of-range kinds and page size split into separate kinds:** **accepted**. Each value range is reached separately, and the `OutOfRange` matcher pins the field.
- **Shuffled ids via random sort keys** (`:295-297`): **accepted**. The ids stay a permutation of the counter output, and the shrinking stays clean. The shuffle itself is not guarded (D6a1 WARNING 1).

### Relabel D6a -> D6a1/D6a2 (64ff07e)

Consistent in the specs (`document-model/spec.md:7` and `:309`, `document-serde/spec.md:7`, `document-validation/spec.md:7` and `:226`), in `proposal.md:68` and `:80-82`, in `design.md:103-104`, `:369-371`, `:385`, `:400-401`, `:439-440`, `:566` and `:585`, and in `apply-progress.md:532-567` ("twelve", the chain D5b ← D6a1 ← D6a2 ← D6b). Leftovers are WARNING 2 and SUGGESTION 1.

### Hand mutations (D6a2 scope; reverted)

| # | Mutation | Result |
|---|---|---|
| H4 | `SkewRange` writes in-range values `[89.0, -89.0, 0.0, 1.0, -1.0]` (`:614`): the corruption no longer corrupts | killed: `a corruption is detected` (`:853`) |
| H5 | `WidthRange` writes `h` (`:620` tests `HeightRange`): wrong variant | killed: `WidthRange produced OutOfRange { .. field: H .. }` (`:854`) |

H2 also broke the corruption guard (`keys` index at `:296`). Total for D6a: 5 of 5 killed.

### WARNING

1. **Spec gap: the `validate()` clause cannot hold for `schema_version = 2`** (`document-validation/spec.md:228`, "Applying one to a generated document MUST make `validate()` return exactly the matching variant"; `design.md:460` Q3). By construction no in-memory `Document` has schema 2, so the clause is unsatisfiable through the public API. The spec must change, not the code. Before or with D6b, amend both places as follows: `schema_version = 2` is wire-only, the corruption is applied to `to_json(d)`, `from_json` MUST return `Err(LoadError::Invalid(UnsupportedSchemaVersion { found: 2 }))`, and there is no `validate()` clause. This mirrors the two wire-clause exclusions. D6b's Q3 must use `observed()` (or the equivalent), never `doc.validate()`, for this kind.
2. **Stale "Eleven"** at `proposal.md:111` (risk table: "Eleven sub-deliveries of ≤ 700 lines each"). It should say twelve.

### SUGGESTION

1. `design.md:370` says D6a2 is "379 measured", but the shortstat and `apply-progress.md` give 388. `design.md:386` lists the branches without `-6a2`.
2. `Corruption::matches` accepts either `DuplicateId` kind for both duplicate forms (`:404-406`). That is correct for the variant, but checking the reported id would prove each form hits its own path.

### Mutants

`mutants` report (`scratchpad/mutants-D6a/report.md`): `cargo mutants -p gafira-doc --in-diff` found **0 mutants** ("No mutants to filter"). The diff touches only `crates/gafira-doc/tests/`. 0 caught, 0 missed, 0 timeout, 0 unviable. Nothing critical is affected. The guards' strength was checked instead with hand mutations H1 to H5, and all 5 were killed.

### Result

- Verdict: **PASS** for D6a2 (0 CRITICAL, 2 WARNING, 2 SUGGESTION). The change is not complete because D6b remains.
- Next: the coordinator pushes `feat/domain-doc-6a` and `feat/domain-doc-6a2` and opens their PRs, one after the other per the integration rule. Then **tdd-apply D6b**. D6a2 WARNING 1 (spec fix for schema 2) must land before or in D6b's first commit. D6a1 WARNING 1, D6a2 WARNING 2 and the SUGGESTIONS can ride along.

## D6b (branch `feat/domain-doc-6b`, range `c2bf347..feat/domain-doc-6b`, 4 commits: "docs: apply D6a verify follow-ups", "test(doc): guard id order, id gaps and special characters", "test(doc): add ARQ-10 and ARQ-13 properties", "docs(sdd): record domain-doc D6b")

**Verdict: PASS** (D6b). CRITICAL 0, WARNING 2, SUGGESTION 2.

### Gates per commit

Each commit was checked out in its own fresh `git clone --shared` under the session scratchpad, at a distinct path, so every package was rebuilt. Each policy step body was extracted from that commit's `ci.yml` with a YAML parser and run under `bash -e -o pipefail`, plus `cargo deny check`.

| Commit | fmt | clippy `-D warnings` | test (passed / failed) | `cargo deny` | ARQ-07 | V11 | V5 | V12 | V13 |
|---|---|---|---|---|---|---|---|---|---|
| b84abd1 docs: apply D6a verify follow-ups | ok | ok | 297 / 0 | ok | ok | ok | ok | ok | ok |
| c650717 test(doc): guard id order, id gaps and special characters | ok | ok | 297 / 0 | ok | ok | ok | ok | ok | ok |
| 2a4e8a4 test(doc): add ARQ-10 and ARQ-13 properties | ok | ok | 303 / 0 | ok | ok | ok | ok | ok | ok |
| 3442ee0 docs(sdd): record domain-doc D6b (tip) | ok | ok | 303 / 0 | ok | ok | ok | ok | ok | ok |

303 = 297 + 6 properties (Q1, Q2, Q3, Determinism, Q4, Q5). No `proptest-regressions` file is committed. No commit carries AI attribution (all authored by the owner, no trailer). The diff adds no `#[allow]`, no `HashMap`/`HashSet`, no `unsafe` and no `std::env`. It does not touch `Cargo.toml` or `Cargo.lock`.

**Runtime** (`cargo test --test properties --locked -p gafira-doc`, tip, 20 cores, 8 tests):

| Build | parallel | `--test-threads=1` |
|---|---|---|
| debug | 6.7 s | 17.5 s |
| release | 2.3 s | 6.3 s |

Every run is under 60 s. Q1 keeps `ProptestConfig::with_cases(10_000)` (`tests/properties.rs:71`). These numbers match `apply-progress.md`.

**Size**, code only (`git diff --shortstat c2bf347..HEAD -- crates .github`): 2 files, 206 insertions, 8 deletions = **214** (forecast ~365). This matches `apply-progress.md`. Whole branch: 8 files, 293 / 17.

### D6a1 / D6a2 follow-ups (b84abd1, c650717)

| Item | Evidence | Status |
|---|---|---|
| D6a2 W1: schema 2 is wire-only | `document-validation/spec.md:228` and `:237`; `design.md:460` | closed |
| D6a2 W2: "Twelve" | `proposal.md:111` | closed |
| D6a2 S1: 388 measured, `-6a2` branch | `design.md:370`, `:386` | closed |
| D6a2 S2: duplicate-id kinds pin the reported id | `tests/common/mod.rs:407` (`DuplicateId { id } if Some(*id) == duplicate`), set at `:539`, `:547`; used through `explained_by` (`:449`) by the guard and by Q3 | closed |
| D6a1 W1: guards for shuffle, gap and special characters | `tests/common/mod.rs:840` ("ids out of order"), `:879` ("id gap"), `:771-781` (quote, backslash, control, non-ASCII), all in `wanted` (`:896-901`). The writer killed each one with a mutation (constant sort key, gap loop `0..0`, text limited to `a..z`) | closed |
| D6a1 S1: redundant `#[cfg(test)]` | removed from `mod guards` (`:724`); module doc `:3` says only `properties.rs` includes it | closed |
| D6a1 S2: vendored LICENSE | `.agents/skills/git-workflow-and-versioning/LICENSE` (MIT, Addy Osmani), with `SOURCE.md:5` updated | closed |

### Requirements and scenarios (D6b)

| Requirement / scenario | Evidence | Status |
|---|---|---|
| `document-serde` "Deterministic round trip (ARQ-10)": Round trip idempotent (property, 10 000 cases) | `round_trip_is_idempotent_and_bit_identical` (`tests/properties.rs:75`): bytes equal after save, load, save; `bit_identical` (`:56`) | covered (H1 killed) |
| Same requirement: bit identity of every float | node floats and opacity by `to_bits()` (`:35-52`), payload floats by bits (`:17-33`), the rest by `==` plus `Debug` (`:66-67`) | covered (deviation 2 accepted) |
| `document-serde` "Arena order mutation is caught" (D3 unit, recorded as property mutation) | H4: Q1 stays green; `full_golden_equals_the_built_document` and `arena_is_written_as_an_ascending_array` fail | covered (by unit tests, as the design now says) |
| `document-validation` / `document-model`: Valid generated documents (property, 256) | `generated_documents_validate` (`:86`) | covered (H2 killed) |
| Corruption is detected exactly (property; schema 2 through `from_json`) | `corruption_is_detected_exactly_and_survives_the_wire` (`:93`) via `observed()` and `explained_by` | covered (see SUGGESTION 1 on `found`) |
| Corruption survives the wire (except `ArenaKeyMismatch`, `NonFinite`) | same test, `:97-104`, requires `Err(LoadError::Invalid(e))` with the same variant | covered (H3 and H5 killed) |
| Determinism (D6b, property) | `validation_is_deterministic` (`:109`) | covered (pure function; by inspection) |
| Q4 canonical payload | `payload_text_is_canonical` (`:115`) | covered (H1 killed) |
| `document-serde` "Corruption never panics" (Q5) | `one_byte_of_damage_is_an_error_or_a_valid_document` (`:124`): 3 goldens through `include_str!`, replace / insert / delete | covered (WARNING 1 on power) |
| "Coverage guards and mutations": remove P6, parents across pages, clamp opacity | P6 and clamp: killed (the writer killed P6; verify re-ran clamp as H5). Parents across pages: recorded as not observable, because P2 `DuplicateId` runs over all pages before P5 and rejects the only distinguishing input | covered (one mutation recorded with its reason, as the scenario allows) |

### Deviations judged

1. **Design text rewritten to the current mutation policy** (`design.md:466`): **accepted**. H4 confirms that reversing the arena keeps Q1 green and that the unit tests catch it. The cross-page parent mutation cannot be observed, for the P2 reason above. The `document-validation` scenario explicitly allows recording an undetected mutation with its reason.
2. **Q1 compares floats by `to_bits()` and the rest by `==` and `Debug`**: **accepted**. For finite floats, Rust's `Debug` prints the shortest round-trip digits and the sign of zero, so equal text means equal bits. NaN never appears: Q2 shows that every generated document is valid, and validation rejects non-finite values.
3. **CI per-runner timings come only after the push**: **accepted**. `design.md:516` already lists this as "not verifiable today", and no spec requirement fixes a runtime. It stays open until the PR (WARNING 2).

### Hand mutations (verify; scratch clone, each reverted, `git status` clean, the regression file deleted after each)

| # | Mutation | Target | Result |
|---|---|---|---|
| H1 | Drop `float_roundtrip` from the workspace `serde_json` | Q1, Q4 | killed: both fail |
| H2 | Generator `opacity()` also emits `Just(1.5)` | Q2 alone | killed: `Err(OutOfRange { field: Opacity, value: 1.5 })` |
| H3 | `try_from(DocumentWire)` skips `validate()` (`json.rs:58`) | Q3, Q5 | Q3 killed (`MultipleParents reloaded as Ok(..)`). Q5 caught it in 8 of 11 runs at 256 cases, and always at `PROPTEST_CASES=20000` (`Orphan`) (WARNING 1) |
| H4 | Arena serialized in reverse (`arena.rs:15` `.rev()`) | Q1 | survives, as the design records; 2 unit tests fail |
| H5 | Opacity clamped to `[0, 1]` on read (`element.rs:21`) | Q3 wire clause | killed: `OpacityRange reloaded as Ok(..)`; Q1 green, as expected |

### Mutants

The `mutants` report (`scratchpad/mutants-D6b/report.md`) gives **0 mutants** ("No mutants to filter"), because the diff touches only `crates/gafira-doc/tests/` and docs. It reports 0 caught, 0 missed, 0 timeout and 0 unviable, so no critical code is affected. The strength of the properties was checked with H1 to H5 instead.

### CRITICAL

None.

### WARNING

1. **Q5 is a weak detector of a missing load validation** (`tests/properties.rs:124-141`). Most random single-byte edits break the JSON syntax. With validation removed (H3), Q5 failed in only 8 of 11 runs of 256 cases. The scenario's assertion is correct, and Q3's wire clause plus the D4b unit tests catch H3 every time, so this is not a coverage gap. Suggested fix: draw `byte` partly from digits and structural characters, or raise Q5 to about 2 000 cases (still well under 60 s). Test code, non-critical.
2. **CI per-runner runtime still to record** (`design.md:468`, `:516`, `:531`; `proposal.md:82` "CI runtime record"). After the push, the coordinator records the `properties` time for each of the three runners in the PR. If the slowest runner exceeds 60 s, reduce the bounds, never Q1's cases.

### SUGGESTION

1. `Corruption::matches` accepts `UnsupportedSchemaVersion { .. }` (`tests/common/mod.rs:429`), while the spec requires `{ found: 2 }` (`document-validation/spec.md:228`). The value is pinned by the unit test `json.rs:332`. Pinning `found: 2` in the matcher would make Q3 match the spec word for word.
2. `design.md:105` names `crates/gafira-doc/proptest-regressions/`, but proptest writes `crates/gafira-doc/tests/properties.proptest-regressions`, as observed in the H runs. The D6b row (`design.md:372-373`) also still shows only the ~365 forecast, without the 214 measured.

### Result

- Verdict: **PASS** for D6b (0 CRITICAL, 2 WARNING, 2 SUGGESTION).

## Change complete (domain-doc)

**Verdict: PASS.** All twelve sub-deliveries are done and verified PASS. Every requirement and scenario in the five delta specs is covered by some delivery. Nothing is left uncovered.

### Deliveries

| Delivery | Final verify | Line |
|---|---|---|
| D1 (1a, 1b) | PASS | `verify-report.md:143` |
| D2a | PASS (re-verify) | `:301` |
| D2b | PASS (re-verify) | `:506` |
| D3a | PASS (re-verify) | `:741` |
| D3b | PASS | `:889` |
| D4a | PASS (third verify) | `:1215` |
| D4b | PASS (re-verify) | `:1410` |
| D5a | PASS | `:1502` |
| D5b | PASS | `:1630` |
| D6a1 | PASS | `:1700` |
| D6a2 | PASS | `:1762` |
| D6b | PASS | this file, D6b section |

`apply-progress.md` records all twelve as done, and D6b ends with status "done".

### Scenario coverage across the five specs

There are 190 scenarios. Each scenario title was matched against this report, then the 50 titles without a literal match were checked by hand: they appear under paraphrased rows. Examples: D1 `document-model` rows at `:72-95`, D4b `document-validation` rows at `:1259-1292`, D5b `edit-capability` rows at `:1548-1570`, the V13 E-table at `:1528-1544`, and the `workspace-tooling` table at `:32-58`. None is uncovered.

Command and manual scenarios, all executed or recorded in earlier deliveries:

- `workspace-tooling`: N0 to N5, D-N0 to D-N3, H-N1 to H-N6, V11 missing forbid, "Hardened V12 timing", workspace membership, entries present, clean tree (D1, `:32-58`). V13 "Step present and green" and "Each commit stays green" were checked on every D5b commit and again on each D6b commit above.
- `document-model`: normal tree, dev tree bounded, no kurbo or new package (D1, `:68-70`).
- `document-serde`: feature guards, text conversion off, golden shape (D3a/D3b, `:658-659`).
- `edit-capability`: E0 to E15 (D5b, `:1528-1544`), "Hidden from the docs" (`:1555`), "Whole-value replacement documented" (manual, `:1563`), "Lock unchanged except gafira-doc" and "Deferral recorded" (D5b "Deferred to C3" paragraph, `:1546`). D6b does not touch `Cargo.lock`.

Known limitations, recorded as specified and still not met:

- V5 skips a crate without `clippy.toml`. This is deferred to `xtask check-layers`.
- V13 has six textual limits. Review covers them until C3.
- The `trybuild` ARQ-30 test is deferred to C3.

Open, non-blocking items:

- D6b WARNING 2: CI per-runner timing.
- The design's "Holder questions" Q1 to Q5 (`design.md:580-584`) are owner confirmations, not spec scenarios.

### Next

`next` = **archive**. AGENTS.md sets two conditions:

1. Archive `workspace-scaffold`, then `domain-color`, before `domain-doc`, because `domain-doc/specs/workspace-tooling` targets their base spec.
2. Archive first requests a whole-crate `cargo-mutants` run.

Before that, push `feat/domain-doc-6b`, open its PR and record the CI timings (W2).
