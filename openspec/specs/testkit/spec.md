# Testkit Specification

## Purpose

`gafira-testkit` is the shared test support crate: the bounded document
generator (`document()`), the corruption generators (`corrupt`, `corrupted()`,
`Corruption`, `Corrupted`) and the command law harness (`prop_command`,
`try_prop_command`). The T4 mutation run of the `edit-history` archive found 143
mutants in the crate: 84 caught, 23 unviable, 36 missed
(`openspec/changes/archive/2026-10-25-edit-history/archive-report.md`, "Mutation
results (T4)"). One miss is a documented equivalent that cannot be excluded. The
other 35 were deferred to this change. One of those 35, mutant 13
(`lib.rs:235:18`), is now accepted as a second documented equivalent (owner
decision 2026-10-26, see "Closed decisions" C2).

The archived figures did not reproduce. The Step 0 baseline of this change, a
package-scoped T4 run on the current tree, found 143 mutants: 82 caught, 23
unviable and 38 missed. That is the 36 above plus mutants 37 and 38 at
`lib.rs:652:80`, in the `ExtensionTooDeep` arm of `corrupt` (C5). With mutant
13 and mutant 36 accepted as equivalents, the target is 36 kills.

This change is test-only. It adds or strengthens tests under
`crates/gafira-testkit/tests/` until those 36 mutants are caught. The tests pin
behaviour the crate already has. No source file of any crate changes. This is a
new capability, so there is no main spec to modify.

Scenario types: unit, property, integration, command, manual. No delivery ids:
the owner's request does not split the work, and no design is planned.

Mode: standard (set by the coordinator). Pack: test-only hardening.

## Baseline

Rows 1 to 36 come from the archived T4 run of 2026-10-25, cargo-mutants 27.1.0,
143 mutants, started 20:00:53Z and finished 20:11:10Z. The `mutants` agent ran
it, with output in the session scratchpad (directory `mutants-t4c`), outside the
repo. Its counts (84 caught, 36 missed) match the archive report. The run was
package-scoped (`cargo test --package=gafira-testkit --locked`), which is the
AGENTS.md T4 form.

Rows 37 and 38 come from the Step 0 baseline of this change (2026-10-26). That
run used the same package-scoped command on the branch base. It reported 143
mutants, 82 caught, 23 unviable and 38 missed: rows 1 to 36 plus rows 37 and
38. In the same run, `lib.rs:652:80: replace % with * in corrupt` was caught.
The archived 84/36 figures therefore do not reproduce on the current tree, even
though `crates/gafira-testkit/src` has not changed since `b37b7d1`
(2026-10-25 08:12). This spec does not establish why the two runs differ. The
Step 0 figures (82/38) are the baseline of record for this change. Every line
and column below matches the current `crates/gafira-testkit/src/lib.rs`.

| # | Mutant (file:line:col, as cargo-mutants names it) | Group | Requirement |
|---|---|---|---|
| 1 | `lib.rs:59:10: delete - in color` | colour | Generated fills cover every colour space and negative channels |
| 2 | `lib.rs:59:26: delete - in color` | colour | same |
| 3 | `lib.rs:59:38: delete - in color` | colour | same |
| 4 | `lib.rs:63:13: delete match arm 0 in color` | colour | same |
| 5 | `lib.rs:64:13: delete match arm 1 in color` | colour | same |
| 6 | `lib.rs:233:9: delete match arm 2 in axis_length` | page size | Generated px page sides reach every size band |
| 7 | `lib.rs:234:9: delete match arm 3 in axis_length` | page size | same |
| 8 | `lib.rs:233:18: replace + with * in axis_length` | page size | same |
| 9 | `lib.rs:233:18: replace + with - in axis_length` | page size | same |
| 10 | `lib.rs:234:23: replace - with + in axis_length` | page size | same |
| 11 | `lib.rs:234:23: replace - with / in axis_length` | page size | same |
| 12 | `lib.rs:235:18: replace + with - in axis_length` | page size | same |
| 13 | `lib.rs:235:18: replace + with * in axis_length` | equivalent | Accepted equivalent (C2); not a target |
| 14 | `lib.rs:235:30: replace * with + in axis_length` | page size | same as 6 |
| 15 | `lib.rs:235:30: replace * with / in axis_length` | page size | same as 6 |
| 16 | `lib.rs:238:26: replace == with != in axis_length` | page size | same as 6 |
| 17 | `lib.rs:410:9: replace Corruption::matches -> bool with true` | guard | The corruption guards reject a foreign error |
| 18 | `lib.rs:458:9: replace Corrupted::explained_by -> bool with true` | guard | same |
| 19 | `lib.rs:572:70: replace % with / in corrupt` | sub-case | `corrupt` sub-cases follow the pick table |
| 20 | `lib.rs:572:70: replace % with + in corrupt` | sub-case | same |
| 21 | `lib.rs:580:35: replace == with != in corrupt` | sub-case | same |
| 22 | `lib.rs:585:37: replace / with % in corrupt` | sub-case | same |
| 23 | `lib.rs:585:37: replace / with * in corrupt` | sub-case | same (exact mapping only; C3) |
| 24 | `lib.rs:604:78: replace / with % in corrupt` | sub-case | same |
| 25 | `lib.rs:604:78: replace / with * in corrupt` | sub-case | same (exact mapping only; C3) |
| 26 | `lib.rs:606:28: replace % with / in corrupt` | sub-case | same |
| 27 | `lib.rs:606:28: replace % with + in corrupt` | sub-case | same |
| 28 to 33 | `lib.rs:607:21` to `lib.rs:612:21: delete match arm 0` to `5 in corrupt` | sub-case | same |
| 34 | `lib.rs:634:51: delete - in corrupt` | sub-case | same |
| 35 | `lib.rs:634:69: delete - in corrupt` | sub-case | same |
| 36 | `harness.rs:84:9: delete field failure_persistence from struct Config expression in try_prop_command` | equivalent | Accepted equivalent; not a target |
| 37 | `lib.rs:652:80: replace % with / in corrupt` | sub-case | `corrupt` sub-cases follow the pick table (C5) |
| 38 | `lib.rs:652:80: replace % with + in corrupt` | sub-case | same |

All file paths are under `crates/gafira-testkit/src/`.

## Requirements

### Requirement: Test-only scope

The change MUST NOT modify any file under `crates/*/src/`, any `Cargo.toml`,
`Cargo.lock`, `.cargo/mutants.toml`, `clippy.toml`, `deny.toml` or CI
configuration. In particular, the `#[cfg(test)] mod guards` block in
`crates/gafira-testkit/src/lib.rs` stays byte-identical. New and strengthened
tests MUST live under `crates/gafira-testkit/tests/` and MUST use only the
public API of `gafira-testkit`, its dependencies and `proptest`. The change MUST
NOT add an `exclude_re` entry or `#[mutants::skip]` (AGENTS.md, "Testing and
mutation policy").

The new oracles MUST be deterministic: the same build gives the same verdict on
every run, whatever `PROPTEST_CASES` is set to. Where an oracle samples
generated values, it MUST use a deterministic runner with a fixed number of
draws. The existing generator guard uses 20 000 draws (`document-model`,
"Generator and model properties").

Every commit MUST pass `cargo fmt --check`,
`cargo clippy --workspace --all-targets --locked -- -D warnings` and
`cargo test --workspace --locked` on its own (AGENTS.md, "Strict TDD").

#### Scenario: Source untouched (command)
- GIVEN the branch tip and its base on `main`
- WHEN `git diff --name-only <base>...HEAD` runs
- THEN every listed path is under `crates/gafira-testkit/tests/` or `openspec/changes/testkit-oracle/`

#### Scenario: Gates per commit (command)
- GIVEN each commit of the change, checked out alone
- WHEN the three gate commands run
- THEN all three exit 0

#### Scenario: Oracles ignore PROPTEST_CASES (command)
- GIVEN the final tree
- WHEN `cargo test -p gafira-testkit --locked --test generator --test corruption` runs with `PROPTEST_CASES` unset, and again with `PROPTEST_CASES=1`
- THEN both runs pass, and each oracle test in `tests/generator.rs` and `tests/corruption.rs` runs the same number of draws in both
- AND the scenario does not cover the test targets that existed before this change. In particular, `tests/harness.rs` is outside it (C6)

#### Scenario: No exclusion added (command)
- GIVEN the final tree
- WHEN `.cargo/mutants.toml` is compared with the base, and the tree is searched for `mutants::skip`
- THEN the file is identical and the search finds nothing

### Requirement: Baseline confirmed before the first test commit

Before the first test commit, the T4 run MUST be repeated on the branch base. The
run MUST reproduce the baseline table: 38 missed, with the same names. The
result MUST be recorded in `openspec/changes/testkit-oracle/apply-progress.md`.
This is the RED evidence for the whole change. The run MUST follow the
constraints in "Mutation run constraints". If the list differs, the work MUST
stop and the difference MUST go to the coordinator. This step is done: the Step
0 run found the 38 rows of the baseline table, and the coordinator extended the
table to match (C5).

#### Scenario: Baseline reproduced (command)
- GIVEN the branch base and at least 20 GB free on `/`
- WHEN the T4 command runs
- THEN it reports 143 mutants, 82 caught, 38 missed, 0 timeout and 23 unviable, and `missed.txt` holds the 38 names in the baseline table

### Requirement: Generated fills cover every colour space and negative channels

Over the deterministic draws of `document()`, the shape fills MUST include at
least one `Color::Srgb`, one `Color::LinearSrgb` and one `Color::DisplayP3`.
Each of the red, green and blue channels MUST also be strictly negative in at
least one generated fill. These features already hold in the current generator.
The channel range comes from `lib.rs` `color()`, -4.0 to 4.0 per channel.

#### Scenario: Every colour space is generated (unit)
- GIVEN the deterministic draws of `document()`
- WHEN the fills of all shape nodes are collected
- THEN each of the three `Color` variants appears at least once
- AND the test fails when the arm for space 0 or space 1 is removed (mutants 4 and 5)

#### Scenario: Negative channels are generated (unit)
- GIVEN the same draws
- WHEN each fill's red, green and blue channels are read
- THEN each channel is below 0.0 in at least one fill
- AND the test fails when the minus sign of any one of the three channel ranges is removed (mutants 1 to 3)

### Requirement: Generated px page sides reach every size band

Over the deterministic draws of `document()`, a page whose `size` unit is
`Unit::Px` on the side in question MUST reach each band below. Each band names
the generator mode that produces it in `axis_length`. Each band except the
exact limits MUST be reached at least 10 times. That bound stops the random mode
from satisfying a band by chance: the random mode lands in a band about once in
16 383 random sides, which is about 0.18 expected hits in 20 000 documents. The
band oracles MUST use at least 1 000 deterministic draws and SHOULD use 20 000,
the same as the existing guard.

Measured (explorer N2, release build, `TestRunner::deterministic()`, public API
only): 20 000 documents give 14 666 px sides. On the unmutated generator the
bands hold 3 027 (exactly 1.0), 2 937 (exactly 16 384.0), 2 891 in (1, 2),
2 897 in (16 383, 16 384) and 2 914 in the random band. With 1 000 documents
the two near-limit bands hold 145 and 160. Under each of mutants 6 to 12 and
14 to 16, the band that mutant breaks holds 0 sides at 20 000 documents. Run
time is about 5.4 s for 20 000 documents in the dev profile, and 0.27 s for
1 000. Bands:

| Band | Side value in px | Mode in `axis_length` |
|---|---|---|
| Lower limit | exactly 1.0 | 0, and the px-only "no clamp" rule |
| Upper limit | exactly 16 384.0 | 1, and the px-only "no clamp" rule |
| Just above the lower limit | strictly between 1.0 and 2.0 | 2 |
| Just below the upper limit | strictly between 16 383.0 and 16 384.0 | 3 |
| Random in range | strictly between 2.0 and 16 383.0, and not 100.0 | 4 (100.0 is the fallback size, so it is excluded) |

For the exact limits, only px-unit pages count. Pages in other units can also
reach 1.0 px or 16 384.0 px after conversion. That is why mutant 16 survived
the existing guard.

#### Scenario: Exact limits on px pages (unit)
- GIVEN the deterministic draws of `document()`
- WHEN the sides of pages whose unit is `Unit::Px` are collected
- THEN a side of exactly 1.0 and a side of exactly 16 384.0 are both present
- AND the test fails when the px test in `axis_length` is inverted (mutant 16)

#### Scenario: Bands near the limits (unit)
- GIVEN the same draws
- WHEN the px sides are counted per band
- THEN the two bands near the limits each have at least 10 sides
- AND the test fails for mutants 6 to 11

#### Scenario: Random band (unit)
- GIVEN the same draws
- WHEN the px sides are counted per band
- THEN the random band has at least 10 sides
- AND the test fails for mutants 12, 14 and 15

#### Scenario: Shifted random band is an accepted equivalent (manual)
- GIVEN mutant 13 (`1.0 + frac * 16 383.0` becomes `1.0 * frac * 16 383.0`, which moves the random band from [1, 16 384] to [0, 16 383])
- WHEN the band oracles above run against it
- THEN they pass (every count, minimum and maximum over 20 000 draws equals the unmutated values), the mutant stays in the T4 missed list, and no test pins exact generated values to kill it (C2)

### Requirement: The corruption guards reject a foreign error

`Corruption::matches(error, duplicate)` MUST return `false` for an error of a
different `ValidationError` variant. For the `OutOfRange` kinds (opacity,
alpha, skew, width, height) it MUST also return `false` for a different
`NodeField`. For the two duplicate-id kinds it MUST return `false` for a
`DuplicateId` with a different id. `Corrupted::explained_by(error)` MUST give
the same answer as `self.kind.matches(error, self.duplicate)`. This is the
rejecting side of the existing `document-validation` requirement "Validation
properties": the property "Corruption is detected exactly" would pass
vacuously if the guard accepted every error.

#### Scenario: A foreign error is rejected (property)
- GIVEN a generated document, and two corruption kinds `K` and `J` whose expected errors differ (different variant, field or duplicate id)
- WHEN `corrupt(doc, K, pick)` and `corrupt(doc, J, pick)` are built, and the first is asked to explain the second's observed error
- THEN `explained_by` returns `false`, and `K.matches` with that error and the first's `duplicate` returns `false`
- AND the test fails when either function returns `true` unconditionally (mutants 17 and 18)

#### Scenario: Shared variants (unit)
- GIVEN `DuplicateIdAcrossPages` and `DuplicateIdPageAndNode`, which both map to `DuplicateId`, and `PageSizeBelow` and `PageSizeAbove`, which both map to `PageSize`
- WHEN each is asked about the other's error
- THEN the page-size pair accepts it, and the duplicate-id pair accepts it only when the ids are equal

### Requirement: `corrupt` sub-cases follow the pick table

`corrupt(doc, kind, pick)` documents that "`pick` chooses the sub-case". For the
kinds below, the sub-case MUST follow this table, for every `pick` in
`0..=999` (the range `corrupted()` draws from). The table states the current
behaviour of `crates/gafira-testkit/src/lib.rs`. It is part of the crate's
contract for these kinds only, and tests pin it (owner decision 2026-10-26,
C3). A future edit to these sub-cases needs a spec change. The exact mapping is
needed for mutants 23 (`lib.rs:585:37`) and 25 (`lib.rs:604:78`). Both keep
the same set of sub-cases over a full period and change only which pick
reaches which one.

| Kind | Sub-case chosen by `pick` | Observable on the corrupted document |
|---|---|---|
| `NextIdNotAboveMax` | offset `pick % 3` | the injected node's id equals `doc.next_id() + offset` |
| `PageSizeBelow` | value `[0.5, 0.0, 0.999][pick % 3]` px | the added page has that value on one side and 100 px on the other |
| `PageSizeAbove` | value `[16 384.5, 20 000.0, 1e9][pick % 3]` px | same |
| `PageSizeBelow`, `PageSizeAbove` | side: width when `(pick / 3)` is even, else height | which side holds the bad value |
| `NonFinite` | field `[x, y, w, h, rotation_deg, skew, opacity][pick % 7]` | the `NodeField` of the `NonFinite` error, and the only non-finite field of the injected node |
| `NonFinite` | value `[NaN, +inf, -inf][(pick / 7) % 3]` | the class of that field's value (NaN, positive infinity, negative infinity) |
| `SkewRange` | skew `[90.0, -90.0, 91.0, 1e3, -1e3][pick % 5]` | the injected node's `placement.skew` |
| `ExtensionTooDeep` | depth `65 + pick % 3` (C5) | the injected extension node's payload is that many nested arrays, each holding one element, around `null`. Its container depth is 65, 66 or 67 |

#### Scenario: Next-id offset (unit)
- GIVEN a generated document and each `pick` in `0..=5`
- WHEN `corrupt(doc, NextIdNotAboveMax, pick)` runs
- THEN the offset is 0, 1, 2, 0, 1, 2
- AND the test fails for mutants 19 and 20

#### Scenario: Page-size value and side (unit)
- GIVEN a generated document and each `pick` in `0..=5`
- WHEN `corrupt` runs with `PageSizeBelow` and with `PageSizeAbove`
- THEN Below gives a value under 1 px and Above a value over 16 384 px, each value is the one the table names, picks 0 to 2 put it on the width and picks 3 to 5 on the height
- AND the test fails for mutants 21, 22 and 23

#### Scenario: Non-finite field and value (unit)
- GIVEN a generated document and each `pick` in `0..=20`
- WHEN `corrupt(doc, NonFinite, pick)` runs and the result is validated
- THEN the error names the field the table gives for `pick`, that field holds the value class the table gives, and every one of the 21 field and value pairs appears once
- AND the test fails for mutants 24 to 33

#### Scenario: Skew values (unit)
- GIVEN a generated document and each `pick` in `0..=4`
- WHEN `corrupt(doc, SkewRange, pick)` runs
- THEN the skews are 90.0, -90.0, 91.0, 1e3 and -1e3 in that order
- AND the test fails for mutants 34 and 35

#### Scenario: Extension payload depth (unit)
- GIVEN a generated document and each `pick` in `0..=5`
- WHEN `corrupt(doc, ExtensionTooDeep, pick)` runs and the payload of the injected extension node is measured
- THEN its container depth is 65, 66, 67, 65, 66, 67 in that order, and validation reports `ExtensionTooDeep`
- AND the test fails for mutants 37 and 38 (the `% -> *` mutant at the same position is already caught)

#### Scenario: Periodicity across the drawn range (property)
- GIVEN any `pick` in `0..=999` and any generated document
- WHEN `corrupt` runs for each kind in the table
- THEN the observable sub-case equals the table's entry for that `pick`

### Requirement: Mutation run constraints

Every `cargo mutants` run of this change (baseline, per-commit checks, final T4)
MUST:
- check free space with `df -h /` first, and stop and report when less than
  20 GB is free;
- use the default per-job target directories: no shared `CARGO_TARGET_DIR`,
  and never the workspace `target/`;
- not pass `--locked` (`.cargo/mutants.toml` supplies it);
- not set `RUSTFLAGS=-D warnings`, and use the default `cargo test` runner;
- write its output under the session scratchpad, never inside the repo.

The final run MUST be a full run, without `--iterate` (ADR 0001, section 5).

#### Scenario: Pre-run disk check (manual)
- GIVEN a mutation run is about to start
- WHEN `df -h /` reports less than 20 GB free
- THEN the run does not start and the coordinator is told

### Requirement: Acceptance: T4 on gafira-testkit

On the final tree, the T4 run on `gafira-testkit` MUST report 0 timeouts. Its
missed list MUST hold exactly these two accepted equivalents, and nothing else:

- mutant 36,
  `crates/gafira-testkit/src/harness.rs:84:9: delete field failure_persistence from struct Config expression in try_prop_command`.
  It is a documented equivalent that cargo-mutants 27.1.0 cannot exclude
  (archive report of `edit-history`);
- mutant 13,
  `crates/gafira-testkit/src/lib.rs:235:18: replace + with * in axis_length`.
  It is accepted as an equivalent (C2), and it cannot be excluded either: an
  `exclude_re` entry is out of scope by "Test-only scope".

The new caught and unviable counts MUST be recorded. The number of mutants MUST
stay 143, because no source changes. Compared with the Step 0 baseline of 82,
the caught count MUST grow by 36 to 118 (143 - 23 unviable - 2 missed).

The T4 command is the package-scoped AGENTS.md form (owner decision 2026-10-26,
C1): `cargo mutants -p gafira-testkit --jobs 4 -o <scratchpad dir>`. A
`--test-workspace=true` run is not part of acceptance.

#### Scenario: Only the two equivalents survive (command)
- GIVEN the final tree and at least 20 GB free on `/`
- WHEN `cargo mutants -p gafira-testkit --jobs 4 -o <scratchpad dir>` runs
- THEN it reports 143 mutants, 118 caught, 23 unviable, 2 missed and 0 timeouts, and `missed.txt` holds exactly the two lines above

#### Scenario: Each oracle has RED evidence (manual)
- GIVEN `apply-progress.md`
- WHEN it is read
- THEN each new or strengthened test names the baseline mutants it kills, and the evidence shows each of those mutants failing that test (a T3 run on `lib.rs`, or a hand mutation on a scratch export)

## Edge cases

- **A different proptest version or seed.** The deterministic stream would
  change. The band oracles use counts of at least 10 against expected counts in
  the thousands, so they stay valid. The pick table does not depend on the RNG.
- **The empty document.** `corrupt` adds a first page when the document has
  none. The sub-case oracles MUST hold for the empty document as well. For
  `NextIdNotAboveMax`, the reference is the `next_id` of the corrupted
  document.
- **`NonFinite` on opacity.** The value is cast to `f32`. NaN and both
  infinities survive the cast, so the value class is still observable.
- **Mutant 16 and pages in other units.** The exact-limit oracle MUST filter on
  the page's unit. Without the filter it stays green under the mutant, which is
  how the existing guard behaves.
- **Run time.** Every mutant reruns the crate's tests. New oracles that draw
  20 000 documents add about 5.4 s (dev profile, N2) to every mutant's test
  time. 1 000 draws cost about 0.27 s and still meet the bound of 10. The T4 wall time MUST be
  recorded beside the baseline's 10 min 17 s.

## Non-goals

- Changing generator, corruption or harness behaviour, or refactoring
  `harness.rs:84` to remove the StructField mutant. The archive report lists
  that refactor as a separate owner option.
- Mutants in other crates, or making `gafira-doc` and `gafira-edit` tests kill
  testkit mutants.
- Pinning sub-cases of corruption kinds that are not in the pick table
  (`NextIdNotAboveMax`, `PageSizeBelow`, `PageSizeAbove`, `NonFinite`,
  `SkewRange` and `ExtensionTooDeep`). For example, the values that
  `OpacityRange`, `AlphaRange`, `WidthRange`, `HeightRange` and `InvalidDpi`
  pick stay unpinned.
- Fixing the stale `document-model` text that places the generator in
  `crates/gafira-doc/tests/common` (C4; a separate docs change).
- Killing mutant 13 with a seed-pinned golden test, a draw count in the
  millions, or a source change (C2).
- Making `cargo-mutants` a CI gate.
- Fixing the flake in the existing test
  `tests/harness.rs::each_broken_law_is_named_with_a_counterexample` at
  `PROPTEST_CASES=1` (C6; accepted follow-up).

## Closed decisions

There are no open questions left. The owner decided C1 to C4 on 2026-10-26.
The coordinator decided C5 (under C3) and C6 the same day. All six are binding for this change.

- **C1. Acceptance command.** T4 is the package-scoped AGENTS.md form,
  `cargo mutants -p gafira-testkit --jobs 4 -o <scratchpad dir>`, the same form
  that produced the baseline. `--test-workspace=true` is not used for
  acceptance. Because the workspace test set includes the package's tests, a
  package-scoped pass also implies a workspace-scoped pass.
- **C2. Mutant 13 (`lib.rs:235:18`, `+` to `*`) is a second documented
  equivalent** (option A). The target becomes 36 kills with C5 (34 before it), and acceptance allows
  exactly two missed mutants: `harness.rs:84:9` and `lib.rs:235:18`. Evidence
  is in Needs explorer N1.
- **C3. The `corrupt` pick-to-sub-case table is a contract** (option A), pinned
  by tests. It kills mutants 23 (`lib.rs:585:37`) and 25 (`lib.rs:604:78`).
- **C4. The stale `document-model` text** that places the generator in
  `crates/gafira-doc/tests/common` is out of scope. A separate docs change will
  fix it.
- **C6. Existing harness flake at `PROPTEST_CASES=1` is an accepted
  follow-up.** This is a coordinator decision of 2026-10-26. With
  `PROPTEST_CASES=1`, the existing test
  `tests/harness.rs::each_broken_law_is_named_with_a_counterexample` fails in
  about half the runs. With the variable unset, it passes. That file predates
  this change, which is test-only, so the file is out of scope. "Oracles
  ignore PROPTEST_CASES" therefore covers only the oracles this change adds
  (`tests/generator.rs`, `tests/corruption.rs`). The flake stays open as a
  candidate for a later fix. The per-commit gates run with the variable unset
  and are not affected.
- **C5. `ExtensionTooDeep` joins the pick table.** This is a coordinator
  decision of 2026-10-26, taken under C3. The Step 0 baseline found two misses
  the archived run did not list: `lib.rs:652:80`, `%` to `/` and `%` to `+` in
  `nested(65 + pick % 3)`. The payload depth `65 + pick % 3` is now part of the
  pick-table contract and is pinned by "Extension payload depth". The target is
  36 kills, and acceptance is 143 mutants, 118 caught, 23 unviable, 2 missed and
  0 timeouts.

## Needs explorer (settled)

Explorer run xp3, 2026-10-26. It used a scratch copy of the worktree, release
build, `TestRunner::deterministic()` over `gafira_testkit::document()`, public
API only, and each mutant applied at its line and column.

- **N1. Settled: mutant 13 is not killable by a statistic of the draws.**
  Over 20 000 draws every count, minimum and maximum equals the unmutated
  generator's. The random band holds 2 914 sides in both, and the page-size
  range is 1 to 16 384 px in both. Paired values differ by exactly 1.0, so
  only a test that pins stream values can see the shift. The shift is 1 on a
  spread of about 4 730, so a mean or histogram oracle would need about 4e9
  documents.

  The only discriminator that does not pin values is the 100 x 100 px fallback
  page. It appears when a random px side falls in [0, 1), which is about 1 in
  16 383 random px sides. The unmutated generator never produces it. At 2 000 000
  documents the mutant gave 13 such pages and the base gave 0. That run took
  about 200 s in release. Under T4 (dev profile, the suite rerun for every
  viable mutant) it would cost about 14 h of CPU. This settles C2.
- **N2. Settled: a count of at least 10 kills mutants 6 to 11.** At 20 000
  documents the unmutated bands hold about 2 891 sides in (1, 2) and 2 897 in
  (16 383, 16 384). At 1 000 documents they hold 145 and 160. Each of mutants 6
  to 11 leaves the band it breaks at 0. The random-band count (2 914 against 0)
  kills mutants 12, 14 and 15. The px-only exact-limit count (3 027 and 2 937
  against 0) kills mutant 16.
