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
