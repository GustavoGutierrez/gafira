# Verify report: domain-time (`gafira-time`), deliveries D0-D4

Round 1. Branch `feat/domain-time-1`, HEAD `3b4e58f` (base `26a2928`; merges into current `main` `e95e56d` with no conflicts, disjoint files). Flow: full | tier T3 | verify V3 (from the `Flow:` line). Mode standard/full, independent verifier. No `mutants` report was handed over; mutation numbers below are the implementer's claim, partly reproduced (see Mutation).

Verdict: **FAIL** (2 CRITICAL, 2 WARNING, 4 SUGGESTION).

## Method

- All work on `git archive` exports under the session scratchpad (`vdt/`), each with its own `CARGO_TARGET_DIR`; nothing in the worktree was modified except this report and `verification.json`.
- Independent probes (13 tests, `zz_probe.rs`) were written from the delta specs before the implementer's tests were read, and run on an export with the implementer's tests removed (HEAD and the pre-refactor commit `04e1b8c`): 13/13 pass on both.
- Gates per commit on clean exports of every crate-touching commit plus HEAD: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, and the `policy` job of `ci.yml` replayed step by step from each commit's own `ci.yml` (Format, ARQ-07, forbid unsafe, V5, V12, V13).
- Negative CI policy runs (6) and hand mutants (11) on separate scratch exports.

## Gates

| Commit | Role | fmt | clippy | test | deny | policy |
|---|---|---|---|---|---|---|
| `9a060bf` | D0 skeleton | 0 | 0 | 0 (436) | 0 | 0 |
| `3d5ace9` | spec fix | 0 | 0 | 0 (436) | 0 | 0 |
| `5e0b08d` | D1 flicks | 0 | 0 | 0 (455) | 0 | 0 |
| `25afa83` | D2 rates | 0 | 0 | **101** (472 pass, 1 fail) | 0 | 0 |
| `f80d182` | D3 ranges | 0 | 0 | 0 (499), flaky, see C2 | 0 | 0 |
| `2a57705` | D2 test fix | 0 | 0 | 0 (499) | 0 | 0 |
| `04e1b8c` | D4 timecode | 0 | 0 | 0 (523) | 0 | 0 |
| `15eb6cd` | refactor | 0 | 0 | 0 (523) | 0 | 0 |
| `c64825d` | tests | 0 | 0 | 0 (525) | 0 | 0 |
| `23af75e` | refactor | 0 | 0 | 0 (525) | 0 | 0 |
| `3b4e58f` | HEAD (docs) | 0 | 0 | 0 (525) | 0 | 0 |

Negative policy runs (scratch exports of HEAD), all fire as intended: dropping the `gafira-time  -` V5 row (`has clippy.toml but no V5 row`); adding a `gafira-geom` dependency (`must not depend on gafira-geom`); adding `tokio` (`forbidden tokio`); appending a line to `crates/gafira-time/clippy.toml` (V12 diff); removing `#![forbid(unsafe_code)]` (forbid step); adding `Instant::now`, `HashMap` and `thread::spawn` to `src/` (clippy rc 101, `disallowed_methods`/`disallowed_types`). Control on unmodified HEAD: all rc 0.

ARQ-03 (three targets) cannot be exercised locally with GitHub CI disabled; the crate is `std`-only integer code with no `cfg` or platform API, so the risk is low but unproven on Windows and macOS.

## Findings

### CRITICAL

**C1. Delta spec changed by an implementer commit after the first implementation commit.**
class: ambiguity
ref: `3d5ace9` (`openspec/changes/domain-time/specs/time-model/spec.md`, "Seconds and milliseconds are exact"), after `9a060bf` (D0 implementation)
`apply-progress.md` (D1 deviation) records that `tdd-apply` changed the 1,234 ms example from 870,854,400 to 870,710,400 flicks after its RED test exposed it. The new value is correct (1,234 x 705,600 = 870,710,400, checked independently by probe `p02`), the test asserts it, and no stale copy of the old value remains in specs, design, spike or code. The process rule in `verify.md` step 3 still makes this CRITICAL: the specifier (or the coordinator with a stated reason) must ratify the amendment. No code change is needed.

**C2. The D2 delivery commit fails `cargo test --workspace --locked`; the D3 commit fails it intermittently.**
class: implementation
ref: `crates/gafira-time/tests/rates.rs` `the_tick_divides_every_accepted_frame_duration` at `25afa83` and `f80d182`
The D2 property asserted that one second is a whole number of frames, which is false for accepted rates. The per-commit gate run on a clean export of `25afa83` failed with the minimal input `num = 350, den = 3`. With the original regression file reset before each run, the property failed in 9 of 20 runs; with the later seed (`19845/243`) it fails every time. `f80d182` has the same `rates.rs` and passed this run by chance. The fix in `2a57705` is correct (`duration * num == FLICKS_PER_SECOND * den`; the spec never stated the false property, and it is consistent with design D-2). But AGENTS.md requires every commit to pass alone, and stacked PRs are cut at commit boundaries. The `apply-progress.md` claim that "every commit before the mutation fix round passed the gates alone" is refuted. Remedy: fold `2a57705` into `25afa83` (a history rewrite needs the coordinator's or owner's OK), or get an explicit owner waiver for the two broken intermediate commits, provided no PR is cut at D2 or D3.

### WARNING

**W1. Weak tests: several criteria are proved at a single rate, or by random properties that are mostly vacuous.**
class: implementation
ref: `tests/rates.rs` `frames_round_trip_when_representable` (24 fps only), `samples_round_trip_when_representable` (48 kHz only), `accepted_frame_rates_*` (random `num/den`, mostly rejected pairs); `tests/timecode.rs` `non_drop_parse_round_trip` (25 fps only)
- The timecode scenario "Nominal frame rate" requires that a frames field equal to the nominal rate be rejected at 24000/1001, 30000/1001, 60000/1001 and 24.5 fps. The suite checks the `nominal_fps()` values, but checks rejection only at 24 fps.
- Hand mutant HM07 (parse bound `num/den` instead of the nominal rate) was killed only by the random drop-frame round-trip properties, with no deterministic test behind it.
- My probes cover these cases deterministically and pass (`p05`, `p06`, `p07`, `p10`: 23.976, 29.97, 59.94, 245/3, 24.5, 120, 1/1 and 705,600,000/1 fps), so the behaviour is right and only the tests are weak.

**W2. Timecode parse leniency is not decided by the spec.**
class: ambiguity
ref: `timecode` spec, "Non-drop timecode" ("Parsing MUST accept exactly that shape"); `src/timecode.rs:129-135`
The code accepts `00:00:00:005` and `00:00:00:0023` (extra leading zeros in FF), `000:00:00:00`, and `-00:00:00:00` (a sign on zero). Each parses to the canonical value, and `format()` never emits these forms, so round trips and the drop-frame bijection hold. The spec fixes neither the FF width (it needs 3 digits from 100 fps upwards) nor whether a non-canonical form or `-0` must be rejected. The specifier should decide, for example "FF has exactly max(2, digits(nominal-1)) digits; `-` only before a non-zero label", or allow these forms explicitly.

### SUGGESTION

- S1. The requirement "`Timecode` MUST NOT be serialized" is proved only by the missing derive (my probe `p09` checks it through trait resolution). A `trybuild` compile-fail test (already a workspace dependency) would pin it.
- S2. "Checked arithmetic" requires associativity and commutativity when the result is representable. Nothing tests them; it is trivially true for `i64` checked operations. Add a property, or drop the sentence when merging.
- S3. `design.md` D-10 still says the intersection does a "non-empty check", which `15eb6cd` removed. That refactor and `23af75e` are behaviour-preserving: an empty range has `start == end`, so `max(starts) >= min(ends)`, and `x < 0` is the same as `x.is_negative()`. Probes `p08` (50,000 random pairs including empty ranges) and `p10`-`p12` pass on both `04e1b8c` and HEAD. Update the design line at archive.
- S4. Mutation evidence: `cargo mutants --list` at HEAD reproduces the claimed per-file counts exactly (error 1, flicks 18, range 24, rate 47, timecode 139, lib 0; 229 in total). Caught, unviable and missed were not reproduced here, by policy. The independent `mutants` T3 report must back the "0 missed" claim before archive. The implementer's observation that `--jobs 4` let workers share a stale target is a tooling claim; `explorer` should confirm it before AGENTS.md changes its `--jobs 4` default.

## Hand mutants (11, all killed by the implementer's suite; each also killed by the probes)

| ID | Delivery | Rule mutated | Killed by |
|---|---|---|---|
| HM01 | D1 | `seconds_floor` truncates instead of floor | `whole_seconds_floor_towards_negative_infinity`, proptest |
| HM02 | D2 | `nominal_fps` ties round down | `nominal_frame_rate_rounds_to_nearest_with_ties_up` |
| HM03 | D2 | `to_frames_floor` truncates | `instants_floor_to_the_containing_frame`, proptest |
| HM04 | D2 | `to_samples_floor` truncates | `instants_floor_to_the_containing_sample` |
| HM05 | D3 | serde `TryFrom` bypasses validation (wrapping end) | `serde_rejects_invalid_ranges` |
| HM06 | D3 | intersection becomes union (min/max swapped) | 3 range tests |
| HM07 | D4 | parse frames bound `num/den` instead of nominal | only the random drop-frame properties (see W1) |
| HM08 | D4 | skipped label at every second (spike's first bug) | exhaustive drop-frame test, properties |
| HM09 | D4 | drop-frame also accepted at 23.976 | `drop_frame_is_rejected_for_unsupported_rates` |
| HM10 | D4 | either separator accepted in either mode | `the_frames_separator_is_part_of_the_mode` |
| HM11 | D4 | hours wrap at 24 | `hours_are_a_running_count_and_never_wrap` |

## Requirement and scenario matrix

15 requirements, 57 scenarios. All 57 are covered by code and tests at HEAD (W1 marks the weak ones); none is missing or failing at HEAD. "Probe" names the independent check.

| Spec / requirement | Scenarios | Implementer test(s) | Probe | Status |
|---|---|---|---|---|
| TM Crate purity (D0) | dependency and lint purity; no clock/thread/I/O API | gates, policy replay; `grep` = 0 hits in `src/` | negative runs N1-N6; `cargo tree` = serde only | covered |
| TM Flicks unit (D1) | divisibility; negative representable | `the_tick_divides_every_required_rate`, `negative_ticks_are_preserved` | p01, p03 | covered |
| TM Construction (D1) | exact s/ms incl. 870,710,400; overflow | `seconds_are_exact`, `milliseconds_are_exact`, `seconds_and_milliseconds_overflow_*` | p02 (exact boundaries ±1) | covered |
| TM Checked arithmetic (D1) | add overflow; add/sub identity (prop); neg at min; zero identity | flicks overflow tests, `add_then_subtract_is_an_identity`, `zero_is_the_default_and_an_identity` | p03 | covered (S2) |
| TM Display conversions (D1) | floor to seconds; lossy f64 doc | `whole_seconds_floor_*`, rustdoc of `as_seconds_f64` | p04; `cargo doc -D missing_docs` clean | covered |
| TM Frame rates (D2) | standard; NTSC durations; non-integral; below 1 fps and zero; equivalent fractions | rates.rs unit tests | p05 (1/1 boundary, 245/3, 120000/2002) | covered |
| TM Frame alignment (D2) | round trip (prop); floor inside frame; boundary; cross-rate floor; overflow | rates.rs | p06 (7 rates; 24 to 25 fps frame 23, where floor 23 differs from nearest 24) | covered (W1) |
| TM Sample alignment (D2) | standard; rejected; round trip (prop); floor between samples | rates.rs | p07 | covered (W1) |
| TM Ranges ordered (D3) | construction; empty; inverted; overflowing end; negative start | ranges.rs | p08 | covered |
| TM Half-open (D3) | boundaries; adjacent; intersection cases; symmetry (prop) | ranges.rs, `intersection_is_symmetric_and_contained` | p08 (pointwise check over 50,000 pairs) | covered |
| TM Serde (D1-D3) | round trip (prop); wire shapes; invalid rejected | serde tests in flicks/rates/ranges | p09 (determinism, normalization on load, missing fields) | covered (S1) |
| TC Validated label (D4) | label to time and back (prop); floor; invalid label | exhaustive drop-frame test, `from_flicks_floors_*`, skipped/field tests | p10-p12 | covered |
| TC Non-drop (D4) | nominal; format; parse round trip (prop); invalid fields; negative; hours > 24; 29.97 drift | timecode.rs | p10 (all nominal cases, 23 malformed inputs) | covered (W1, W2) |
| TC Drop-frame (D4) | ref labels 29.97; ref times; ref 59.94; skipped rejected; tenth minutes; other rates rejected; round trip (prop) | timecode.rs | p11, p12 (21 blocks, both signs, monotone labels, valid-label count equals frame count) | covered |
| TC Separators (D4) | contradictory rejected; mode separator in format | `the_frames_separator_is_part_of_the_mode` | p11 | covered |

Corrections reported by the implementer:
- 1,234 ms: corrected consistently in the spec and the test (C1 is only about who made the change).
- The frame-duration property: false only in the test, never in the spec, design or spike. It is corrected in `2a57705`, and its regression seed is kept (C2 is about the intermediate commits).

## Dependencies, API, hygiene, size

- Runtime dependency: `serde` only; dev: `proptest`, `serde_json`; no other `gafira-*` crate; `cargo deny` all ok at every commit. `.cargo/mutants.toml` untouched (no new exclusions).
- Public API matches design D-9. Every public item is documented (`RUSTDOCFLAGS=-D missing_docs`, broken intra-doc links: clean). No `unwrap`, `expect`, `panic!` or `#[allow]` in `src/`. Every `as` cast is bounded by a stated invariant, and the probes exercise the extremes (`i64::MIN`/`MAX` labels, 705,600,000 fps, `u32::MAX`).
- Commits: conventional, correct types, no AI attribution or `Co-Authored-By` lines.
- Size (`git diff --shortstat`, crates / whole): D0 70 / 82 (code only); D1 435 / 463; D2 618 / 647; D3 346 / 373; D4 696 in its own commit, 702 with `2a57705`; T3 fixes 47. Total 2,211 / 3,209 against a forecast of 2,060. D4 sits at the ~700 ceiling, which is acceptable.

## Mutation

Tier T3 (per changed source file, `--jobs 1`, separate target directories), as claimed in `apply-progress.md`: 229 found, 211 caught, 0 missed, 0 timeout, 18 unviable, about 13 minutes. Only the mutant counts were reproduced (S4). No critical missed mutant is known, but this is not independently confirmed.

next: specifier (C1 ambiguity: ratify the 1,234 ms amendment; W2 parse shape), then tdd-apply (C2 implementation: fold `2a57705` into `25afa83` or get an owner waiver; W1 tests)

---

# Round 2: verify fix round 1 (C1, C2, W1, W2)

Branch `feat/domain-time-1`, HEAD `dbcd799` (round 1 was `3b4e58f`; history is linear). Flow: full | tier T3 | verify V3. Independent verifier, working only on `git archive` exports, each with its own `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL=0`. No build folder was shared between exports. Every test log shows `Compiling gafira-time` on its own tree.

Verdict: **PASS** (0 CRITICAL, 2 WARNING, 3 SUGGESTION). All deliveries D0-D4 are done.

## Commits in this round

| Commit | Type | Files | Role |
|---|---|---|---|
| `dfbf8ae` | docs | `apply-progress.md` | specifier ratification of the 1,234 ms value (C1); owner waiver of C2 |
| `b260baa` | docs | `verify-report.md`, `verification.json` | round 1 report |
| `17dc719` | docs | `specs/timecode/spec.md`, `apply-progress.md` | specifier: new requirement "Timecode text is canonical" (owner Q1-A, Q2-A) answering W2 |
| `67ed2a6` | fix | `src/timecode.rs`, `tests/timecode.rs` | canonical widths for hours and frames; no negative zero |
| `0508a59` | test | `tests/rates.rs`, `tests/timecode.rs` | fixed-case tests answering W1 |
| `589b797` | refactor | `src/timecode.rs` | `is_canonical_width` restated; removes the equivalent mutant |
| `dbcd799` | docs | `apply-progress.md` | fix round record |

The delta-spec change in `17dc719` is the specifier route that round 1 asked for (`next: specifier`). It is not an implementer amendment. No implementer commit (`67ed2a6`, `0508a59`, `589b797`) touches `specs/`. Commits are conventional, with no AI attribution.

## Gates (fresh export and fresh target per commit)

| Commit | fmt | clippy | test | deny | policy |
|---|---|---|---|---|---|
| `17dc719` | 0 | 0 | 0 (525) | 0 | 0 |
| `67ed2a6` | 0 | 0 | 0 (530) | 0 | 0 |
| `0508a59` | 0 | 0 | 0 (536) | 0 | 0 |
| `589b797` | 0 | 0 | 0 (536) | 0 | 0 |
| `dbcd799` (HEAD) | 0 | 0 | 0 (536) | 0 | 0 |

Policy is the `ci.yml` `policy` job replayed step by step: Format, ARQ-07, forbid unsafe, V5, V12, V13. `dfbf8ae` and `b260baa` only touch documents and have the same crate tree as `3b4e58f`, which passed in round 1. `17dc719` was run anyway because it changes a spec: its tree still has the round 1 code, and every gate passes. `.cargo/mutants.toml`, `.github/`, `Cargo.lock` and `deny.toml` are unchanged.

## Independent probes (`zz_probe2.rs`, 7 tests, written from the specs before reading the new tests)

The probes ran on exports with the implementer's tests removed:

| Export | Result |
|---|---|
| HEAD `dbcd799` | 7/7 pass |
| control `3b4e58f` (before the fix) | 4 pass, 3 fail: `q02` accepts `00:00:00:005`; `q04` accepts `-00:00:00:00`; `q05` accepts `000:00:00:00` |

So the probes do tell the old behaviour from the new one.

- `q01` checks the 120 fps frames width: 5, 99, 100 and 119, plus 120 rejected.
- `q02` checks 20 non-canonical texts in both modes, including `0100:00:00:00`, `00:00:00:5`, `-000:00:00:01` and `00:00:00;059` at 59.94.
- `q03` checks 100 hours in both signs, 10 and 99 hours, and 100 hours in drop-frame.
- `q04` checks negative zero at 24, 120, 29.97 DF and 59.94 DF. `-1` and `-2` frames are accepted and keep their exact flicks, and `--` is rejected.
- `q05` is the exhaustive form of the property scenario. For 13 formats (including 49/2, 240, 1,000, 705,600,000 and 1 fps, and both drop-frame rates) and 53 labels per format, it applies every one-character insert, delete and replace over `0-9 : ; -` and every 1-3 zero padding of each field: more than 100,000 derived texts. Each is either rejected or formats back to itself. Every padding is rejected.
- `q06` pins the nominal bound at 8 rates and modes.
- `q07` checks fixed frame and sample round trips, including floors at ±1 tick.

## Comparison with the implementer's tests

- **W2 (canonical text):** covered by the deterministic tests `frames_field_width_at_120_fps`, `extra_leading_zeros_are_rejected` (every case of the spec scenario), `hours_of_one_hundred_or_more_have_no_padding` and `negative_zero_is_rejected`, and by the property `accepted_text_is_canonical`. All match the new scenarios one to one.
- **W1 (weak tests):** now fixed.
  - `frames_round_trip_at_every_rate` covers 8 rates and `samples_round_trip_at_every_standard_rate` covers 3.
  - `non_drop_labels_round_trip_at_every_rate` covers 8 rates, both signs, and checks the expected text.
  - `a_frames_field_equal_to_the_nominal_rate_is_rejected` covers 23.976, 24, 29.97 ND and DF, 59.94 ND and DF, 24.5 and 120.
  - `the_nominal_rate_bounds_the_last_label_of_a_minute` pins HM07 deterministically.
- The implementer notes that `Timecode::new` takes a label count, so "a frames field equal to it is rejected" only applies to parsing. That reading is correct: the scenario talks about a frames field, and `new(nominal)` formats as the next second, which `the_label_after_the_last_frame_rolls_into_the_next_second` checks.

## Hand mutants (5, on the new canonical-text rules; each on its own export and target)

| ID | Rule mutated | Implementer tests that kill it | Probes that kill it |
|---|---|---|---|
| HM12 | negative-zero check removed (`if false`) | `negative_zero_is_rejected` | q04, q05 |
| HM13 | hours width back to `len < 2` (padding allowed) | `extra_leading_zeros_are_rejected`, `accepted_text_is_canonical` | q02, q05 |
| HM14 | frames width back to `len < 2` | `extra_leading_zeros_are_rejected`, `accepted_text_is_canonical` | q02, q05 |
| HM15 | minimum width dropped from `is_canonical_width` (the bound touched by `589b797`) | `malformed_and_out_of_range_fields_are_rejected`, `accepted_text_is_canonical` | q02, q05 |
| HM16 | negative zero tested on the frames field instead of the label (rejects `-01:00:00:00`) | `non_drop_labels_round_trip_at_every_rate`, `non_drop_parse_round_trip` | q03, q05 |

All 5 are killed, and each one by at least one deterministic implementer test.

## Mutation (T3 on the changed source file, reproduced)

`cargo mutants --package gafira-time --file crates/gafira-time/src/timecode.rs --jobs 1` ran on a clean export of `dbcd799`, with cargo-mutants 27.1.0 and `CARGO_INCREMENTAL=0`.

| Found | Caught | Missed | Timeout | Unviable | Wall time |
|---|---|---|---|---|---|
| 143 | 137 | 0 | 0 | 6 | 173 s |

All 144 logs (the baseline plus 143 mutants) show `Compiling gafira-time`. This matches the implementer's second run exactly.

The 6 unviable mutants are `Default::default()` returns for types without `Default`. All 7 `is_canonical_width` mutants are caught, so the equivalent `>` to `>=` mutant from the first run at `0508a59` is gone after `589b797`, which preserves behaviour: HM15 and the gates agree. No critical mutant was missed.

## Round 1 findings, now

| Round 1 finding | Status |
|---|---|
| C1 | **Resolved.** The specifier ratified the value in `dfbf8ae`, a commit that touches only `apply-progress.md`. The value 870,710,400 was rechecked in round 1 (`p02`). |
| C2 | **Waived by the owner** in `dfbf8ae`. History is not rewritten. The condition that no PR is cut at `25afa83` or `f80d182` still applies (see R2-W1). |
| W1 | **Resolved** (see above). |
| W2 | **Resolved.** The specifier decided in `17dc719`, the code changed in `67ed2a6` and was refined in `589b797`, and the tests and probes are green. |
| S1-S3 | Still open as suggestions (see below). |
| S4 | Partly resolved: `timecode.rs` is now reproduced independently (R2-W2). |

## Requirement and scenario matrix (delta)

16 requirements and 62 scenarios: 15 and 57 from round 1, plus "Timecode text is canonical" with 5 scenarios. The amended "Non-drop timecode" text, which now points to the canonical widths and allows `-` only for negative time, is covered by the same tests plus the new ones.

| Scenario (TC "Timecode text is canonical") | Implementer test | Probe | Status |
|---|---|---|---|
| Frames field width at 120 fps | `frames_field_width_at_120_fps` | q01 | covered |
| Extra leading zeros rejected | `extra_leading_zeros_are_rejected` | q02 | covered |
| Hours of 100 or more have no padding | `hours_of_one_hundred_or_more_have_no_padding` | q03 | covered |
| Negative zero rejected | `negative_zero_is_rejected` | q04 | covered |
| Accepted text is canonical (property) | `accepted_text_is_canonical` | q05 (exhaustive) | covered (R2-S1) |
| TC Non-drop "Nominal frame rate", now including rejection at every listed rate | `a_frames_field_equal_to_the_nominal_rate_is_rejected` | q06 | covered (W1 closed) |
| TM frame and sample round trips at more than one rate | `frames_round_trip_at_every_rate`, `samples_round_trip_at_every_standard_rate` | q07 | covered (W1 closed) |

62 of 62 scenarios are covered at HEAD. None is missing or failing.

## Findings

### CRITICAL

None.

### WARNING

**R2-W1. D2 and D3 commits still fail the test gate alone (C2, waived).**
class: implementation
ref: `25afa83`, `f80d182`; waiver in `apply-progress.md` (`dfbf8ae`)
The owner waived it, so it is not blocking. It is kept visible so that the coordinator does not cut a stacked PR at either commit. Any PR must end at `2a57705` or later.

**R2-W2. T3 mutation for `flicks.rs`, `range.rs` and `rate.rs` is still only the implementer's claim.**
class: claim
ref: `apply-progress.md` T3 round, verify round 1 S4
These files are unchanged since `3b4e58f`. Their "0 missed" result (part of 229 found, 211 caught, 18 unviable) was not reproduced, because only `timecode.rs` was in scope this round. The T4 whole-crate run planned at archive covers it, and it should be run independently there.

### SUGGESTION

- **R2-S1.** `accepted_text_is_canonical` draws real frames from ±2·10^9, so its branch "adding `-` to a zero label is always rejected" almost never runs. It also uses 6 formats, while the scenario says "any accepted rate". The fixed test `negative_zero_is_rejected` and probe `q05` (13 formats) cover both points. Biasing `real` towards 0, or adding 240 fps or 1 fps, would make the property carry its own weight.
- **R2-S2.** Round 1 S1 (a `trybuild` compile-fail test for "not serialized") and S2 (associativity and commutativity) are still open. They are optional.
- **R2-S3.** Round 1 S3 is still open: `design.md` D-10 still mentions the removed non-empty check. Also record the canonical-text rule (`is_canonical_width`, no negative zero) in the design at archive.

## Size

| Scope | Lines |
|---|---|
| This round (`17dc719..dbcd799`, crates only) | 280 insertions, 2 deletions (src 9) |
| Whole round (`3b4e58f..dbcd799`) | 508 insertions, 3 deletions |
| Change (`26a2928..dbcd799`, crates only) | 2,489 insertions, against a forecast of 2,060 |

next: archive
