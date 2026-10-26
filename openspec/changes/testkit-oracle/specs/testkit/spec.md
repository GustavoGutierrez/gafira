# Testkit Specification (delta for `testkit-oracle`)

## Purpose

`gafira-testkit` is the shared test support crate: the bounded document
generator (`document()`), the corruption generators (`corrupt`, `corrupted()`,
`Corruption`, `Corrupted`) and the command law harness (`prop_command`,
`try_prop_command`). The T4 mutation run of the `edit-history` archive found 143
mutants in the crate: 84 caught, 23 unviable, 36 missed
(`openspec/changes/archive/2026-10-25-edit-history/archive-report.md`, "Mutation
results (T4)"). One miss is a documented equivalent that cannot be excluded. The
other 35 were deferred to this change.

This change is test-only. It adds or strengthens tests under
`crates/gafira-testkit/tests/` until those 35 mutants are caught. The tests pin
behaviour the crate already has. No source file of any crate changes. This is a
new capability, so there is no main spec to modify.

Scenario types: unit, property, integration, command, manual. No delivery ids:
the owner's request does not split the work, and no design is planned.

Mode: standard (set by the coordinator). Pack: test-only hardening.

## Baseline

Source: the T4 run of 2026-10-25, cargo-mutants 27.1.0, 143 mutants, started
20:00:53Z and finished 20:11:10Z. It was run by the `mutants` agent, with output
in the session scratchpad (directory `mutants-t4c`), outside the repo. Its counts
match the archive report. The run was package-scoped:
`cargo test --package=gafira-testkit --locked`, which is the AGENTS.md T4 form.
`crates/gafira-testkit/src` has not changed since then (last commit `b37b7d1`,
2026-10-25 08:12). Every line and column below matches the current
`crates/gafira-testkit/src/lib.rs`.

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
| 13 | `lib.rs:235:18: replace + with * in axis_length` | page size | **pending** (Open question 2, Needs explorer N1) |
| 14 | `lib.rs:235:30: replace * with + in axis_length` | page size | same as 6 |
| 15 | `lib.rs:235:30: replace * with / in axis_length` | page size | same as 6 |
| 16 | `lib.rs:238:26: replace == with != in axis_length` | page size | same as 6 |
| 17 | `lib.rs:410:9: replace Corruption::matches -> bool with true` | guard | The corruption guards reject a foreign error |
| 18 | `lib.rs:458:9: replace Corrupted::explained_by -> bool with true` | guard | same |
| 19 | `lib.rs:572:70: replace % with / in corrupt` | sub-case | `corrupt` sub-cases follow the pick table |
| 20 | `lib.rs:572:70: replace % with + in corrupt` | sub-case | same |
| 21 | `lib.rs:580:35: replace == with != in corrupt` | sub-case | same |
| 22 | `lib.rs:585:37: replace / with % in corrupt` | sub-case | same |
| 23 | `lib.rs:585:37: replace / with * in corrupt` | sub-case | same (exact mapping only; Open question 3) |
| 24 | `lib.rs:604:78: replace / with % in corrupt` | sub-case | same |
| 25 | `lib.rs:604:78: replace / with * in corrupt` | sub-case | same (exact mapping only; Open question 3) |
| 26 | `lib.rs:606:28: replace % with / in corrupt` | sub-case | same |
| 27 | `lib.rs:606:28: replace % with + in corrupt` | sub-case | same |
| 28 to 33 | `lib.rs:607:21` to `lib.rs:612:21: delete match arm 0` to `5 in corrupt` | sub-case | same |
| 34 | `lib.rs:634:51: delete - in corrupt` | sub-case | same |
| 35 | `lib.rs:634:69: delete - in corrupt` | sub-case | same |
| 36 | `harness.rs:84:9: delete field failure_persistence from struct Config expression in try_prop_command` | equivalent | Accepted equivalent; not a target |

All file paths are under `crates/gafira-testkit/src/`.

## ADDED Requirements

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
- WHEN `cargo test -p gafira-testkit --locked` runs with `PROPTEST_CASES` unset, and again with `PROPTEST_CASES=1`
- THEN both runs pass, and each new oracle test runs the same number of draws in both

#### Scenario: No exclusion added (command)
- GIVEN the final tree
- WHEN `.cargo/mutants.toml` is compared with the base, and the tree is searched for `mutants::skip`
- THEN the file is identical and the search finds nothing

### Requirement: Baseline confirmed before the first test commit

Before the first test commit, the T4 run MUST be repeated on the branch base. The
run MUST reproduce the baseline table: 36 missed, with the same names. The
result MUST be recorded in `openspec/changes/testkit-oracle/apply-progress.md`.
This is the RED evidence for the whole change. The run MUST follow the
constraints in "Mutation run constraints". If the list differs, the work MUST
stop and the difference MUST go to the coordinator.

#### Scenario: Baseline reproduced (command)
- GIVEN the branch base and at least 20 GB free on `/`
- WHEN the T4 command runs
- THEN it reports 143 mutants, 84 caught, 36 missed, 0 timeout and 23 unviable, and `missed.txt` holds the 36 names in the baseline table

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
from satisfying a band by chance. With 20 000 draws the random mode lands in a
band about once in 16 383 random sides. Bands:

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

#### Scenario: Shifted random band (unit, pending)
- GIVEN mutant 13 (`1.0 + frac * 16 383.0` becomes `1.0 * frac * 16 383.0`, which moves the random band from [1, 16 384] to [0, 16 383])
- WHEN the oracles above run against it
- THEN the expected outcome depends on Open question 2 and Needs explorer N1. Until both are settled, this scenario is not an acceptance condition

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
behaviour of `crates/gafira-testkit/src/lib.rs`. It becomes part of the crate's
contract only for these kinds (Open question 3).

| Kind | Sub-case chosen by `pick` | Observable on the corrupted document |
|---|---|---|
| `NextIdNotAboveMax` | offset `pick % 3` | the injected node's id equals `doc.next_id() + offset` |
| `PageSizeBelow` | value `[0.5, 0.0, 0.999][pick % 3]` px | the added page has that value on one side and 100 px on the other |
| `PageSizeAbove` | value `[16 384.5, 20 000.0, 1e9][pick % 3]` px | same |
| `PageSizeBelow`, `PageSizeAbove` | side: width when `(pick / 3)` is even, else height | which side holds the bad value |
| `NonFinite` | field `[x, y, w, h, rotation_deg, skew, opacity][pick % 7]` | the `NodeField` of the `NonFinite` error, and the only non-finite field of the injected node |
| `NonFinite` | value `[NaN, +inf, -inf][(pick / 7) % 3]` | the class of that field's value (NaN, positive infinity, negative infinity) |
| `SkewRange` | skew `[90.0, -90.0, 91.0, 1e3, -1e3][pick % 5]` | the injected node's `placement.skew` |

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
missed list MUST hold exactly the accepted equivalents. Today that is only
mutant 36:
`crates/gafira-testkit/src/harness.rs:84:9: delete field failure_persistence from struct Config expression in try_prop_command`.
It is a documented equivalent that cargo-mutants 27.1.0 cannot exclude
(archive report of `edit-history`). If the owner accepts mutant 13 as a second
equivalent (Open question 2), the list grows to those two lines and nothing
else. The new caught and unviable counts MUST be recorded. The number of
mutants MUST stay 143, because no source changes.

The T4 command is the AGENTS.md form: `cargo mutants -p gafira-testkit --jobs 4
-o <scratchpad dir>`. A `--test-workspace=true` variant is pending Open
question 1.

#### Scenario: Zero non-equivalent misses (command)
- GIVEN the final tree and at least 20 GB free on `/`
- WHEN the T4 command runs
- THEN it reports 143 mutants and 0 timeouts, and `missed.txt` holds only the accepted equivalent lines

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
  20 000 documents add to every mutant's test time. The T4 wall time MUST be
  recorded beside the baseline's 10 min 17 s.

## Non-goals

- Changing generator, corruption or harness behaviour, or refactoring
  `harness.rs:84` to remove the StructField mutant. The archive report lists
  that refactor as a separate owner option.
- Mutants in other crates, or making `gafira-doc` and `gafira-edit` tests kill
  testkit mutants.
- Pinning sub-cases of corruption kinds outside the pick table.
- Fixing the stale `document-model` text that places the generator in
  `crates/gafira-doc/tests/common` (Open question 4).
- Making `cargo-mutants` a CI gate.

## Open questions

1. **Which T4 form decides acceptance?** The request names
   `cargo mutants -p gafira-testkit --test-workspace=true`. AGENTS.md and ADR
   0001 define T4 as `cargo mutants -p <crate> --jobs 4 -o <dir>`, which is
   package-scoped, and the baseline of 36 was measured that way.
   - (a) AGENTS.md form. It matches the baseline. Only the crate's own tests
     can kill its mutants, so the result is stricter. Because the workspace
     test set includes the package's tests, 0 non-equivalent misses here
     implies the same result with `--test-workspace=true`.
   - (b) `--test-workspace=true`. Tests in `gafira-doc` and `gafira-edit` also
     run, so some baseline misses may already be caught before this change.
     That would make the before and after comparison invalid. Each mutant also
     runs the whole workspace test suite, which is slower.
   - **Recommendation: (a).** If the owner wants (b) as well, run it once at
     the end as extra evidence.
2. **Mutant 13 (`lib.rs:235:18`, `+` to `*`) looks equivalent at the level of
   observable behaviour.** It shifts the random band from [1, 16 384] to
   [0, 16 383]. A random side lands in [0, 1) in about 1 of 16 383 draws, and
   only then does the result change (to the 100 px fallback). The two
   distributions differ by one part in 16 383, which a deterministic sample of
   20 000 documents cannot tell apart through a distribution oracle. The only
   certain kill is a golden test that pins exact generated values for a fixed
   seed. Such a test breaks on any generator edit or proptest update, and
   AGENTS.md says to test "behavior of the public contract, not
   implementation".
   - (a) Accept it as a second documented equivalent. Acceptance becomes 2
     missed.
   - (b) Add a seed-pinned golden oracle for this mutant only.
   - (c) Kill it with a source change in a separate change (for example,
     making the random band's bounds explicit). That is not test-only.
   - **Recommendation: (a), after explorer N1 confirms.** This means 34 kills,
     not 35. The owner's figure of "35 killable" assumed it could be killed.
3. **Should the `corrupt` pick table be a contract?** Mutants 23
   (`lib.rs:585:37`, `/` to `*`) and 25 (`lib.rs:604:78`, `/` to `*`) keep the
   same set of sub-cases over a full period: (side, value) and (field, value)
   are still covered once each, just in a different order of picks. Only a
   test that pins the exact pick-to-sub-case mapping can kill them. The other
   sub-case mutants are also killed by weaker coverage oracles.
   - (a) Make the table in "`corrupt` sub-cases follow the pick table" a
     contract, as written. Future edits to these sub-cases then need a spec
     change.
   - (b) Use coverage oracles only and accept mutants 23 and 25 as
     equivalent. Acceptance gains 2 misses.
   - **Recommendation: (a).** `corrupt` is public, `pick` is its documented
     selector, and this is test-support code, so the cost is low.
4. **The stale generator location in `document-model`.** "Generator and model
   properties" still says the generator lives in `crates/gafira-doc/tests/common`.
   It moved to `gafira-testkit` (commit `224b6b6`).
   - (a) Leave it out of this change. Fix it in a docs-only change.
   - (b) Add a MODIFIED delta for `document-model` here.
   - **Recommendation: (a).** It keeps this change test-only and the delta to a
     single capability.

## Needs explorer

- **N1.** Mutant 13 cannot be killed by a deterministic oracle over the 20 000
  draws of `document()` without pinning exact values. Measure this on a scratch
  copy: apply the mutant, run candidate band and count oracles, and check
  whether any page-size statistic differs. The "Shifted random band" scenario
  is pending until the verdict.
- **N2.** Under mutants 6 to 11, the near-limit bands receive fewer than 10 px
  sides in the 20 000 deterministic draws. The 1-in-16 383 estimate is
  analytic. Confirm it by counting the bands with each mutant applied on a
  scratch copy.
