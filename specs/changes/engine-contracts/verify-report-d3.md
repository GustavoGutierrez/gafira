# Verify report: `engine-contracts`, delivery D3, round 1

- Verdict: **FAIL**
- Commit verified: `ac73e8aaa25d23ee221bfbcf1e22b840c9a1a192` on `feat/engine-contracts-d3`, base `9faa98e`
- Round 1 (first verify of this code). Depth V2, mode full, tier T4 at close, `mutation: archive`
- Counts: 3 CRITICAL, 1 WARNING, 2 SUGGESTION
- Per class, routed: implementation 3. Follow-up, not routed: ambiguity 1

The production code matches the C11 tables: no defect found, and every probe passes. The failure is in the tests. Three independent hand mutants on critical code survive the crates' whole suites, so `to_message` has rules that nothing in the repo proves.

## CRITICAL

### C1. NaN routed to the wrong key survives (hand mutant H1)
- class: implementation
- ref: `crates/gafira-pathops/tests/message.rs:59,74,122`; rule: `crates/gafira-pathops/src/error.rs` arms `InvalidStrokeWidth`/`InvalidTolerance { NonFinite(_) }`
- Rule: C11, `NonFinite(_)` covers every class, no arguments, and the class stays in `Display`.
- Mutant: new arms for `NonFinite(NonFiniteValue::Nan)` return `*_not_positive` with `value = Decimal(NaN)`, on both width and tolerance. It breaks the key and leaks a NaN into an argument. It is also the natural slip, since NaN is "not positive".
- Result: `cargo test -p gafira-pathops`: exit 0, 161 passed, including all four `arq_102_*` tests. Survived.
- Cause: the fixtures use `Infinity` (width) and `NegativeInfinity` (tolerance), and the loop in `arq_102_the_non_finite_class_and_the_geometry_text_stay_out_of_the_message` runs over those two classes only. NaN never reaches those two arms, although that test's name claims the rule. Only `NonFiniteCoordinate` is walked over all three classes.

### C2. Saturation below `i64::MAX` survives (hand mutant H2)
- class: implementation
- ref: `crates/gafira-pathops/tests/message.rs:140-143`; rule: `error.rs` `OpenSubpath` arm
- Rule: C11, `subpath = Int` by `i64::try_from`, saturating at `i64::MAX`. A value that fits an `i64` keeps its value.
- Mutant: `i32::try_from(*subpath).map(i64::from).unwrap_or(i64::MAX)`: every index above `i32::MAX` reports `i64::MAX`.
- Result: exit 0, 161 passed. Survived.
- Cause: the cases are `0`, `3` and `usize::MAX`; nothing sits near a boundary. The expected value at `usize::MAX` also repeats the production expression, so it is not independent. An index above `i32::MAX` is far off any real path; the finding stands because `to_message` is listed as critical code (`design.md`, "Critical code") and step 4b leaves no discretion.

### C3. Lost sign of an invalid tolerance survives (hand mutant H3)
- class: implementation
- ref: `crates/gafira-pathops/tests/message.rs:65-69`; rule: `error.rs` `InvalidTolerance { NotPositive }` arm
- Rule: C11, `value = Decimal`, the number the error carries.
- Mutant: `Decimal(value.abs())` in `tolerance_not_positive`: a tolerance of `-3` is shown as `3`.
- Result: exit 0, 161 passed. Survived.
- Cause: the only fixture is `0.0`, which cannot tell the value from its absolute value, or from a hard-coded `0.0`. The stroke-width row, with `-1.5`, is not exposed to this.

Proof these are test gaps, not equivalent mutants: with my probes restored, each mutant fails them (H1: 4 probes, H2: `subpath_boundaries`, H3: `real_routes`), while the implementer's `message` binary stays at 4 passed, 0 failed under all three.

Each needs a test change only; the code stays. T4 at close does not replace these: H1 and H3 are shapes `cargo-mutants` would not generate.

## WARNING

### W1. The technical manual now contradicts the V5 table for `gafira-pathops`
- class: ambiguity (follow-up: `follow_up: true`, does not route)
- ref: `docs/manuals/tecnico/chapters/02-architecture.md:26,89`, and the same lines in `chapters-es/`
- Both rows say `gafira-pathops` depends on `gafira-geom` alone. After D3 that is false. D3 could not fix it: the design's delivery scope leaves the manual out, and the manual changes only through `manual-writer` when the owner asks. Those tables are already behind in other ways (no `gafira-layout` or `gafira-anim` row). The spec itself is fine: 02 §2.2 allows the edge.
- Marked follow-up so it reaches `archive` and does not divert this FAIL to `specifier`.

## SUGGESTION

- S1. In `arq_102_an_open_subpath_index_saturates_at_i64_max`, use a literal expected value (`i64::MAX` under `cfg(target_pointer_width = "64")`) when closing C2, and add `i64::MAX as usize` and `i64::MAX as usize + 1`.
- S2. Before merging, the coordinator should note that `develop` moved to `1dd177c` since the base. `git diff --name-only 9faa98e develop` shows no overlap with `ci.yml`, `Cargo.lock`, `gafira-pathops` or `gafira-layout`. Gates here ran on the base-derived commit, as asked.

## Gates (clean export, own target dir, `CARGO_INCREMENTAL=0`; each exit read from its own file)

| Gate | Exit | Log read |
|---|---|---|
| `cargo fmt --check` | 0 | empty |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | 0 warning or error lines |
| `cargo test --workspace --locked` | 0 | 89 result lines, 1168 passed, 0 failed, 0 ignored; D3's six `arq_102_*` tests listed as ok |
| `cargo deny check` | 0 | advisories, bans, licenses, sources ok |
| CI: no UI deps (ARQ-07) | 0 | |
| CI: forbid unsafe | 0 | |
| CI: V5 layer table | 0 | |
| CI: V12 clippy copies | 0 | |
| CI: V13 EditToken | 0 | |
| CI: V14 property tests | 0 | |

CI steps were extracted from `ci.yml` at this commit and run with `bash -eo pipefail`, as Actions does. The export got a throwaway `git init` so V13 and V14 could use `git grep` and `git ls-files`. Only the Linux matrix entry ran; Windows and macOS were not run. Per-commit gates are V3 and were not run: the pass claimed for `c2bb774` rests on `apply-progress.md`. `ac73e8a` is docs-only, so `edc4b7c` has the same code as the verified commit.

## Artifact integrity (D1's late G0)

The late-record exception does not reach D3.

| Check | Result |
|---|---|
| `07947c5` slice card and `7b8c2cd` amend are ancestors of `9faa98e` (`git merge-base --is-ancestor`) | exit 0, both |
| `9faa98e` is the direct parent of the first D3 commit `c2bb774` | yes |
| Slice card touches only `apply-progress.md` | yes, both commits |
| Amend states why | yes: records the owner's acceptance of the late record |
| Card block (lines 1-13) identical at base and HEAD | `diff` exit 0 |
| Implementer commits changed the card | no. D3 adds only `@@ -80,0 +81,52`; `6507bda` touches line 34 onward |
| `design.md` (acceptance, C11) | last changed `7da03f3`, ancestor of `6bede8b`; no diff to the base, none in D3 |
| Card's stated order "precedes D2a, D2b and D3" | true for D3 by ancestry, not by dates |
| D3 file scope | only `gafira-layout`, `gafira-pathops`, `ci.yml`, `Cargo.lock`, `apply-progress.md` |
| V5 row, `Cargo.toml`, `Cargo.lock` in one commit (ADR 0007) | yes, `edc4b7c` |
| `.cargo/mutants.toml` | unchanged, no exclusions to re-check |

## Acceptance map

| # | Criterion | Proof | State |
|---|---|---|---|
| 1 | `LayoutError` C11 table, every variant | `arq_102_every_layout_variant_gives_its_table_key_and_arguments`; probe `table_c11_exact_and_shape`; item 7 spot-check | covered |
| 2 | `PathOpError` C11 table, every row | per-variant test; probes `table_c11_exact`, `real_routes` | code right, proof weak: C1, C3 |
| 3 | `OpenSubpath` saturation | `arq_102_an_open_subpath_index_saturates_at_i64_max`; probe `subpath_boundaries` | code right, proof weak: C2 |
| 4 | Class and `Geometry` text stay out of the message (ADR 0012 rule 3) | named test; probe `no_prose_or_class_in_message` | code right, proof weak: C1 |
| 5 | Keys valid and distinct, `error.<area>.<variant>` | `*_keys_are_valid_and_distinct`; probe `adr12_key_shape` | covered |
| 6 | Tests never call `Display` | read: the only `to_string()` calls are on `&str` fixtures | covered |
| 7 | Exhaustive `match`, no `_` arm (ADR 0012 rule 2) | no wildcard in either body; negatives N1-N3 | covered |
| 8 | Edge and V5 row (ADR 0007) | CI V5 exit 0; own negative exit 1 | covered |
| 9 | Hand-written errors, no `thiserror` (ADR 0012 rule 5) | 0 `thiserror` in `Cargo.lock`; only one lock line added | covered |

Total 9, covered 6, weak 3. ADR 0012 rule 4 (cancellation) does not apply: neither crate takes a `CancelToken` yet.

Product IDs (step 3c): ARQ-102 (engines) is the only ID of D3, named by six tests in the two `message.rs` files. Left out on purpose: ARQ-102 catalog, UI text, `gafira-ui` lint and `UserError` (design, "Still pending"); ARQ-107 (D1); ARQ-98 and ARQ-110 (D2a); ARQ-112 (D2b).

## Probes (step 1c, written before reading the implementer's tests; scratch export only)

9 of 9 pass. Layout: exact table with shape, real routes through `safe_rect`, object safety. Pathops: exact table over all three non-finite classes, key shape, no prose or class in the message, subpath boundaries around `i64::MAX`, object safety and `Clone`, real routes through `union` and `stroke_outline`, with finite `Decimal` values confirmed on every route that carries one.

Negative probes, each with a `Display` arm added so only `to_message` can fail:

| Probe | Result |
|---|---|
| N1, new `LayoutError` variant | E0004 at `error.rs:55`, the `to_message` match |
| N2, new `PathOpError` variant | E0004 at `error.rs:155`, inside the crate despite `#[non_exhaustive]` |
| N3, new `InvalidNumber` cause | E0004 at `error.rs:155` |
| V5 row without `gafira-color gafira-doc` | exit 1, both `must not depend on` errors. Re-run independently of recorded item 8 |

## Hand mutants (step 4b)

Judged against the crate's own suite, my probes removed. Run 4, killed 1. Export sources were restored after each one (byte count confirmed).

| Id | Mutant | Result |
|---|---|---|
| S1 | Spot-check of recorded item 7: layout `NonFinite` field from `self.to_string()` | killed by `arq_102_every_layout_variant_gives_its_table_key_and_arguments` |
| H1 | NaN width and tolerance routed as `*_not_positive` with a NaN value | **survived**, C1 |
| H2 | `OpenSubpath` saturates over the `i32` range | **survived**, C2 |
| H3 | `tolerance_not_positive` drops the sign | **survived**, C3 |

Recorded items 1 to 8 were not repeated; they do not cover H1 to H3.

## Mutation

`mutation: archive`, no report for this delivery; not a finding. T4 on `gafira-pathops` and `gafira-layout` stays due at change close.

## Size

`git diff --shortstat 9faa98e HEAD`: 8 files, 331 insertions, 1 deletion. Without `apply-progress.md`: 7 files, 279 insertions, as recorded. Forecast 200 (x1.4: 280). Under the ~700 ceiling.

next: tdd-apply

---

# Verify report: `engine-contracts`, delivery D3, round 2 (closure)

- Verdict: **PASS** for delivery D3. The change is not done: D2a is unmerged and D2b is not started, so this is the "PASS with deliveries remaining" route
- Commit verified: `04b21310fff88d199405cead4ea44d60b7d2034f` on `feat/engine-contracts-d3`, base `9faa98e`
- Round 2. Depth V2, mode full, tier T4 at close, `mutation: archive`
- Counts: 0 CRITICAL, 1 WARNING (follow-up), 1 SUGGESTION
- Per class, routed: none. Follow-up, not routed: ambiguity 1

## Closure of earlier findings

Same mutants as round 1, same text, on a clean export of HEAD, judged by `cargo test --locked -p gafira-pathops --no-fail-fast` with no probes in the tree. Baseline first: exit 0, 162 passed. Every mutant built; `error.rs` was confirmed byte-identical to HEAD afterwards with `cmp`.

| Finding | Mutant | Exit | Passed | Killed by | State |
|---|---|---|---|---|---|
| C1 | H1 NaN width and tolerance routed to `*_not_positive` | 101 | 161 | `arq_102_the_non_finite_class_and_the_geometry_text_stay_out_of_the_message` | closed |
| C2 | H2 saturation over the `i32` range | 101 | 161 | `arq_102_an_open_subpath_index_saturates_at_i64_max` | closed |
| C3 | H3 `tolerance_not_positive` drops the sign | 101 | 161 | `arq_102_a_not_positive_number_keeps_its_value_and_sign` | closed |

Each mutant fails exactly one test, and it is the test the fix commit names, so the rows in `apply-progress.md` hold for H1, H2 and H3a. Its extra H3b and H3c were not re-run. No independent mutant budget was spent.

`04b2131` changes tests and `apply-progress.md` only: `git diff ac73e8a HEAD` shows no change under any `src`, in `ci.yml`, `Cargo.toml`, `Cargo.lock` or `.cargo`. So the round 1 result that the production code matches C11 still stands. S1 was applied: the expected values are literals, including `i64::MAX as usize` and `i64::MAX as usize + 1`. The 32-bit branch of that test was read, not compiled.

## WARNING

### W1. The technical manual contradicts the V5 table for `gafira-pathops`
- class: ambiguity (follow-up: `follow_up: true`, does not route)
- ref: `docs/manuals/tecnico/chapters/02-architecture.md:26`, as in round 1
- Still open at HEAD, checked on the English table row. It stays a follow-up for `archive`.

## SUGGESTION

- S2, still open and updated. `develop` is now at `b642c1e`. `git diff --name-only 9faa98e develop` shows no overlap with `ci.yml`, `Cargo.lock`, `gafira-pathops`, `gafira-layout` or `specs/changes/engine-contracts/`. The `ARQ-102`-related change in `specs/02` on `develop` is one comment saying `ToMessage` is hand-written, which agrees with D3. A trial merge could not run here (this git's `merge-tree` has no `--write-tree`), so merge conflicts are inferred from the file lists. Gates ran on the base-derived commit, not on a merge.

## Gates (clean export, own target dir, `CARGO_INCREMENTAL=0`; each exit read from its own file)

| Gate | Exit | Log read |
|---|---|---|
| `cargo fmt --check` | 0 | empty |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | 0 warning or error lines |
| `cargo test --workspace --locked` | 0 | 89 result lines, 1169 passed, 0 failed, 0 ignored; one more than round 1, the new C3 test |
| `cargo deny check` | 0 | advisories, bans, licenses, sources ok |
| CI: no UI deps (ARQ-07) | 0 | |
| CI: forbid unsafe | 0 | |
| CI: V5 layer table | 0 | |
| CI: V12 clippy copies | 0 | |
| CI: V13 EditToken | 0 | |
| CI: V14 property tests | 0 | |

CI step scripts are those extracted in round 1; `ci.yml` has no diff since then. Only the Linux matrix entry ran; Windows and macOS did not. Per-commit gates are V3 and were not run.

## Artifact integrity

| Check | Result |
|---|---|
| `07947c5` slice card and `7b8c2cd` amend are ancestors of `9faa98e` | exit 0, both |
| `9faa98e` is the direct parent of the first D3 commit `c2bb774` | yes |
| Slice card and amend touch only `apply-progress.md` | yes, both commits |
| Card block (lines 1-13) identical at base and HEAD | `diff` exit 0 |
| Implementer commits changed the card | no. `ac73e8a` adds `@@ -80,0 +81,52`, `04b2131` adds `@@ -132,0 +133,16`; `c2bb774` and `edc4b7c` do not touch it |
| `design.md`, other spec files, `docs/`, `.cargo/mutants.toml` | no diff from the base |
| D3 file scope | only `gafira-layout`, `gafira-pathops`, `ci.yml`, `Cargo.lock`, `apply-progress.md` |

## Acceptance map

Criteria 2, 3 and 4 move from weak to covered through the closures above. Criteria 1 and 5 to 9 are unchanged from round 1. Total 9, covered 9.

Product IDs: ARQ-102 (engines) is still the only ID of D3, now named by seven tests in the two `message.rs` files. Left out as in round 1.

## Mutation

`mutation: archive`, no report for this delivery; not a finding. T4 on `gafira-pathops` and `gafira-layout` stays due at change close and is a G5 condition.

## Size

`git diff --shortstat 9faa98e HEAD`: 8 files, 397 insertions, 1 deletion. Without `apply-progress.md`: 7 files, 329 insertions, 1 deletion, x1.65 over the forecast of 200 and above its x1.4 figure of 280, inside the x1.4 to x1.9 range `AGENTS.md` records. Under the ~700 ceiling.

next: tdd-apply
