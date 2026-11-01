# Verify report: layout-core D5-D7

## Current state

Round 3 at `4acfffb`, closure round after the T4 fix cycle, depth V2, mode full, tier T4 (`mutation: archive`): **FAIL**. 2 CRITICAL (7 timeout mutants in critical code), 3 WARNING (all `follow-up`), 6 SUGGESTION. Per class: implementation 5 (2 CRITICAL, 3 WARNING); ambiguity 0; architecture 0; claim 0. All deliveries are done, and the code has no defect: every gate, per-commit gate and removed-code probe passes. The FAIL comes from the mutation rule alone. Next: `tdd-apply`. This is the third verify round of this delivery, so the coordinator stops and asks the owner first (`AGENTS.md`, Delivery rules). C6 holds the owner's choice.

The D4 report is in git history at `9383597`; this file covers D5-D7, one delivery by owner decision (Slice card amend `a584943`). The follow-ups of every delivery are listed under "Follow-ups for archive".

## Round 3

Closure only. No independent hand mutants. Reports read, not re-run. Gates ran on a clean clone. Per-commit gates and probes ran on clean exports, outside the repo. The worktree stayed at `4acfffb` and clean.

### CRITICAL

**C5. Two grid mutants time out although assertions catch them.** class: `implementation`. Ref: `src/grid.rs:82:35`, `src/grid.rs:83:34` (`axis_lines`, FD-07 (a), (c)), `tests/grid.rs`, `apply-progress.md:352`.
The notes say that no assertion can fail first. That is false for these two. Run alone, 3 tests fail on an assertion. Two of them sit in the `grid` binary beside the two tests that do not finish, so the binary never reports, and cargo never reaches the third. With `fd_07_a_mm_steps_keep_a_line_on_either_view_edge` and `fd_07_c_at_most_2000_lines_at_any_zoom_on_a_4k_screen` skipped, `cargo test --test grid` gives exit 101 in 2 to 3 s on each mutant. This is the `guides_huge` case again: move those two tests to a binary sorted after `grid`, and re-run T4 on the crate.

**C6. Five loop mutants never end, and the rule has no exception for them.** class: `implementation`. Ref: `src/attractors.rs:61:34`, `69:31`, `src/bounds.rs:59:23`, `src/grid.rs:115:31`, `116:22`; `AGENTS.md` testing policy.
Each one hangs in the first call that reaches its loop, and no assertion can fail first: 0 tests fail. They are detected only by the clock of `cargo-mutants`. See the decision below. Three ways to close it, none tried:
- Rewrite the loops so these mutants fail on an assertion or stop existing: an iterator over children and roots, a bounded doubling.
- Add a watchdog test in a binary that runs first.
- The owner amends the rule so that a mutant shown not to finish counts as detected. This needs the owner, and I cannot grant it here.

### Timeouts: decision

All 7 are in critical code and all 7 are CRITICAL. `AGENTS.md` lists as critical "any spec rule with an invariant" and transforms, and as non-critical `Display` text, trivial accessors and test helpers. These functions implement the row "Elementos que atraen", `node_bounds` and FD-07 (c). The `mutants` agent's "non-critical" has no basis in that list. `AGENTS.md` says `verify` fails on a timeout mutant in critical code, without exception, so the implementer's explanation does not change the verdict.

**Is a mutant that hangs detected? Yes, in fact: no run with it goes green, so none of the 7 can pass unnoticed. But no test reports it.** The evidence of detection is a clock outside the tests. Run by hand, `cargo test` just hangs, and `ci.yml` sets no `timeout-minutes`.

Every test of the crate run alone on a clean export of each mutant, limit 15 s. Baseline: 175 of 175 pass.

| Mutant | Pass | Fail on assertion | Do not finish | Confirmed |
|---|---|---|---|---|
| `attractors.rs:61:34` `+=` to `*=` | 162 | 0 | 13 | killed at 300 s; child index stays 0 |
| `attractors.rs:69:31` `+=` to `*=` | 158 | 0 | 17 | killed at 300 s; root index stays 0 |
| `bounds.rs:59:23` `+=` to `*=` | 158 | 0 | 17 | killed at 300 s; child index stays 0 |
| `grid.rs:115:31` `*` to `/` | 159 | 0 | 16 | killed at 280 s; the product only falls, and `multiple` ends at infinity |
| `grid.rs:116:22` `*=` to `/=` | 159 | 0 | 16 | killed at 280 s; `multiple` ends at 0 |
| `grid.rs:82:35` `/` to `*` | 169 | **3** | 3 | fails: C5 |
| `grid.rs:83:34` `/` to `*` | 169 | **3** | 3 | fails: C5 |

The implementer's claims:
- Five truly never end: true, by the code and by the runs above.
- `grid.rs:82` and `83` iterate a very large finite range: the size depends on the input. I did not measure it; three tests do not finish in 15 s and the rest are unaffected.
- "The loop sits in the first call of nearly every test": false. At most 17 of 175 tests reach a hang.
- "No assertion can fail first": true for the five, false for the two.

`grid.rs:115` and `116` first went through a run that overlapped another patch on the same export. I discarded that run and rebuilt both alone; the row shows the clean run. The rest of the table was not affected.

### WARNING

**W6. The exclusion of `snap.rs:98:13` is not equivalent: its reason is false.** class: `implementation`, `follow-up`. Ref: `.cargo/mutants.toml:20-21`, `src/snap.rs:98`, `src/snap.rs:210`.
The comment says two tied candidates are the same, so replacing one by the other changes nothing. `0.0 == -0.0` ties, and `coordinate` is returned without normalizing. Two targets of one kind at `0.0` and `-0.0` give different bits in `target.coordinate`.
- Brute force against HEAD: 190 472 of 16 000 000 results differ. In a second run of 4 000 000, all 47 508 differences are that sign alone; offset, kind and every non-zero coordinate never differ.
- It is a WARNING, as W3 was in round 1: the row says that two targets with the same coordinate and kind are one, comparison is exact `f64`, and no rule fixes the sign of a chosen target.
- `coordinate: c.coordinate + 0.0` would make the claim true, or the comment can say what differs. It is a one-line change, best done with C5.

**W2. Grid target past about 1e15 px.** class: `implementation`, `follow-up`. Unchanged from round 2. See "The 2^51 test".

**W5. No test pins the page centre with the magnet off.** class: `implementation`, `follow-up`. Unchanged from round 2.

### SUGGESTION

**S1, S2, S3, S5.** Open as in round 2. `apply-progress.md` line numbers in S5 are unchanged, as the file only grew.

**S6.** `.cargo/mutants.toml:6` says each regex matches exactly one mutant. The `align` entry matches two. Harmless; the sentence is wrong.

**S7. Stale notes of the T4 round.** `apply-progress.md:352` holds the claims refuted above. `:360` calls timeouts "caught". `:333` gives 452 / 434, and the final figures are 453 / 435 after the exclusions were narrowed.

### Survivors of the first T4

The request says 21 critical survivors. The first reports hold 23 in `gafira-layout` and 1 in `gafira-doc`, 24 in all, so I checked all 24.

| First T4 mutant | Closed by |
|---|---|
| `align.rs:63:37` `/` to `*` | Caught in the new report |
| `attractors.rs:83:43` `\|=` to `&=` | Caught |
| `error.rs:36:9` `Display` | Caught |
| `rects.rs:140` (4), `rects.rs:155:32` | Caught, 5 of 5 |
| `snap.rs:169:38` `+` to `-`, `+` to `*` | Moved to `168:38`, caught, 2 of 2. Only the 2^51 test kills them |
| `gafira-doc` `error.rs:181:13` | Caught in `t4r` |
| `align.rs:107:9` `[]` arm | Removed |
| `guides.rs:110:30` gap `+ 0.0` | Removed |
| `snap.rs:172` (3) tie clause | Removed; the new `171:46` mutants are caught, 3 of 3 |
| `snap.rs` `beats` (5) | Rewritten as a tuple; `98:13` `==` and `>` are caught, `<=` is excluded (W6) |
| `align.rs:62:37`, `63:37` `/` to `%` | Excluded, true |
| `grid.rs:113:27` | Excluded, true |

All closed, W6 aside. The 8 `guides.rs` timeouts of the first T4 are caught now.

### Exclusions

`cargo mutants --list --workspace` at `4acfffb`, with and without the file: 1 929 against 1 921. The 8 removed are exactly the 4 older entries and the 4 claimed here, and no entry reaches another crate. Each has a reason. The lines are pinned by number, so an edit brings a mutant back as visible, which fails safe.

| Exclusion | Equivalent | Brute force against HEAD |
|---|---|---|
| `align.rs:62:37` and `63:37` `/` to `%` | **True** | 8 000 000 `align` and `distribute` calls with extreme boxes, 547 786 of them errors: 0 differ. The sum is finite or infinite, never NaN, after the number check |
| `grid.rs:113:27` `<` to `<=` | **True** | 15 556 759 + 3 889 043 calls of `grid_lines` and `snap`: 0 differ. The second run has 2 258 162 with a screen spacing of exactly 6.0. All branches give the same `Level` there |
| `snap.rs:98:13` `<` to `<=` | **False in bits** | W6 |

### Removed code

HEAD against `78906a8`, in one process, comparing every output by bits, errors included.

| Removed | Rows | Cases | Differ |
|---|---|---|---|
| `[]` arm of `align` | FD-11 | 16 000 000, more than half of them empty or single, 1 174 216 errors | 0 |
| Gap `+ 0.0` | FD-04, zero rows | 8 000 000, gaps of `-0.0`, subnormal and huge, in all units | 0 |
| `beats` as a tuple | D-16, FD-06 | 16 000 000 `snap` calls, half on 8 coordinates to force ties, `-0.0` included, 1 523 519 errors | 0 |
| `grid_target` tie clause | FD-06 | 16 000 000 `snap` calls on the grid alone, at ties and at midpoints, past 1e25 px | 0 |
| `grid_target` tie clause, direct copy | FD-06 | 168 201 642, of which 15 198 672 exact midpoints | 0; the clause never fired and no pair of neighbours was out of order |

The zero rows hold. No `-0.0` offset differs, and on the mutants `snap.rs:164:51` (`+ 0.0` of `k`) and the `+ 0.0` of each function are caught. FD-11 and the D-16 tests pass.

### The 2^51 test

`fd_06_past_2_pow_51_the_target_is_a_line_that_grid_lines_draws_nearest`: **keep it.** It is the only killer of both `snap.rs:168:38` mutants: without it both pass the crate's tests, and alone it fails on both. The row has no range and asks for the nearest drawn line, so the test does not go against the spec, although the doc comment declines the guarantee there. Dropping it alone leaves two survivors in critical code. If the owner gives the row a range, drop the test and `k + 2.0` together; that decision closes W2 too.

### Mutation reports

| Crate | At | Found | Caught | Missed | Timeout | Unviable | Against HEAD |
|---|---|---|---|---|---|---|---|
| `gafira-layout` | `4acfffb` | 453 | 435 | 0 | 7 | 11 | Same 453 names; run started after the commit |
| `gafira-doc` | `778f6b9` | 163 | 133 | 0 | 0 | 30 | Same 163 names; no source change since |
| `gafira-edit` | `78906a8` | 295 | 247 | 0 | 0 | 48 | Same 295 names; no `gafira-edit` change since |

One correction to the request: `gafira-doc` did change since `78906a8`. `9bc73d5` adds a test inside `#[cfg(test)]` of `src/error.rs`, with no non-test line, so the `gafira-edit` report still stands. Between `778f6b9` and `4acfffb` only `.cargo/mutants.toml` and `apply-progress.md` changed.

### Gates and integrity

Gates at `4acfffb` on a clean clone: `cargo fmt --check` 0; clippy 0; `cargo test --workspace --locked` 0 (1 107 passed, 0 failed); `cargo deny --offline check` 0. The clone stayed clean. CI policy, all 0, read from `ci.yml`: Format, ARQ-07, `forbid(unsafe_code)`, V5, V12, V13, V14. Disk: 49 GB free at the start, 32 GB at the lowest.

Per commit on clean exports, each with its own target folder:

| Commit | fmt | Clippy | Test | Passed |
|---|---|---|---|---|
| `9bc73d5` | 0 | 0 | 0 | 1 099 |
| `ba75462` | 0 | 0 | 0 | 1 101 |
| `1087905` | 0 | 0 | 0 | 1 102 |
| `80b77b9` | 0 | 0 | 0 | 1 106 |
| `edfe888` | 0 | 0 | 0 | 1 106 |
| `b2d3439` | 0 | 0 | 0 | 1 106 |
| `2845183` | 0 | 0 | 0 | 1 107 |

The first four had no standalone run before. They pass alone. `3b3b803`, `27eb59c`, `778f6b9` and `4acfffb` change no code. `1087905` scans the neighbours downwards and `edfe888` restores the ascending scan, so the net diff is the clause alone.

Integrity:
- `*.proptest-regressions`: 9 tracked before and after, none new at any commit, none left by the runs.
- No tracked file deleted.
- No change in `docs/`, `design.md`, `.github`, `deny.toml`, `Cargo.toml` or `Cargo.lock`.
- Slice card: lines 1 to 10 of `apply-progress.md` hash the same at `78906a8` and HEAD; that file has 0 removed lines.
- No absolute path, `#[allow]`, `unsafe` or `mutants::skip` added, and no AI attribution in commit bodies.

Size: 13 files, +235 -29; `crates` alone 11 files, +198 -29.

### Follow-ups for archive

| From | Item | Class |
|---|---|---|
| D2 W1, D3 W8 | 10 mm in px is refused at 550 of 10 000 integer DPIs | ambiguity |
| D2 S2 | Test redundancy, about 130 to 150 lines | suggestion |
| D3 S2 | No code holds the 5 % default of D-38 | suggestion |
| D3 S3 | `error.rs` wording on `field` and `NoArea` | suggestion |
| D4 W7 | `9ab2e9f` and `8c261f3` fail clippy alone; only the owner can waive it | implementation |
| D4 S2 | V10: the `major_on_screen <= 0.0` guard survives | suggestion |
| D4 S5 | A major edge written as `k × step / subdivisions` loses its line; callers must use `k × step` | suggestion |
| D5-D7 W2 | Grid target past about 1e15 px; the note's figures are off | implementation |
| D5-D7 W5 | No test for the page centre with the magnet off | implementation |
| D5-D7 W6 | `snap.rs:98:13` exclusion, if not fixed with C5 | implementation |
| D5-D7 S1 | `snap_rotation` past 2^53 degrees; the row has no range | suggestion |
| D5-D7 S2 | `grid_lines` against D4 at `\|x0\| >= 1e15` | suggestion |
| D5-D7 S3 | `benches/layout.rs:4` names a nightly job that does not exist | suggestion |
| D5-D7 S5, S7 | Stale text in `apply-progress.md` and `tests/snap.rs` | suggestion |
| D5-D7 S6 | `.cargo/mutants.toml:6` says one mutant per regex | suggestion |
| Slice card | Tasks T1 and T2 of `design.md` | task |

Round 3 next: tdd-apply (C5, C6, W6 with them), after the owner's answer on C6, then `mutants` T4 on `gafira-layout` and a closure verify. Closure of C5 and C6 needs a new T4 report that contains the fix.

## Round 2

Round 2 at `517c3cc`: **PASS**, 0 CRITICAL, 2 WARNING, 4 SUGGESTION. Its next step was T4 at change close. Kept as the record.

Closure only: gates, artifact integrity, closure of C1 to C4, W1, W3, W4, and the W2 decision. No new independent mutants. Every probe and mutant ran on clean exports, outside the repo.

### Closure

| Finding | State | Proof |
|---|---|---|
| C1 grid target `-0.0` | **Closed** | `floor() + 0.0` in `grid_target`. The round 1 sweep gives 0 of 2 310 300 differing bits against `grid_lines` (was 8), with 1 133 042 negative and 189 415 ties; reference `-0.0` gives coordinate bits `0`. Mutant without `+ 0.0`: killed by `fd_06_a_grid_target_is_never_negative_zero_on_any_level` |
| C2 `n < 2` | **Closed** | Survivor killed by `d_17_two_wide_overlapping_free_elements_give_no_error_and_no_movement`. It still differs from HEAD in 39 698 of 3 000 000 results, same as round 1, so the test is what changed. Two free boxes: 0 errors in 2 000 000 random calls |
| C3 box checks after DPI | **Closed** | Survivor killed by `the_box_of_every_input_is_checked_before_the_dpi_rule`. The moving and page variants are killed too. Probe: each of the four boxes gives its `InvalidBox` at `dpi = 0` |
| C4 row contradiction | **Closed** | `ab3cd7a` makes attracting nodes and all their descendants the input and states the example in the same A to E tree. The row no longer contradicts itself, and it names the `node_bounds` row. Code unchanged and matches: E hidden or locked with NaN gives `Err x` moving C or with nothing selected, and `Ok` moving D, B or A. Pinned by `arq_110_a_hidden_or_locked_child_of_an_attracting_group_is_read_and_one_outside_is_not`. The tree oracle gives 0 differences |
| W1 magnet off | **Closed** | `ab3cd7a` and `971d7c4` state it; `snap` returns right after the ranges and the step on screen. All four examples of the row give (0, 0) inactive and `Err` active. Probe: 2 000 000 inactive calls with extreme finite boxes, guides, thresholds and DPIs, all `Ok` with `+0.0` and no target. Moving the return back to its old place is killed by `arq_110_an_inactive_magnet_computes_no_centre_quotient_guide_or_distance`. One gap: W5 |
| W3 `-0.0` in `distribute` | **Closed** | `+ 0.0` in `moved`; the note and comment now hold. The round 1 case gives bits `0`; 0 of 4 000 000 random calls give `-0.0`. Mutant killed by `fd_11_a_zero_offset_is_positive_zero_also_when_the_gap_underflows` |
| W4 unnamed criteria | **Closed** | `fd_03_c_the_magnet_snaps_to_a_guide_within_the_threshold_and_not_beyond` and `fd_11_d_four_elements_with_one_locked_distribute_the_three_free_ones`; both assert the criterion |
| S4 row wording | Closed by the spec | `971d7c4` rewrites the example to `x0 = -MAX`, `x1 = 0`. `apply-progress.md` still lists S4 as not done (S5) |

### W2: confirmed limit

Coordinator decision: accepted, `follow-up`, not open. Probe: `snap` against the row's own oracle, a `grid_lines` view around the reference, at the smallest spacing the zoom allows (about 0.19 px), with all subdivisions, units and DPIs.

| Reference, px | Cases | Wrong distance | Bits differ from the oracle |
|---|---|---|---|
| 1e14 to 1e15 | 17 211 019 | 0 | 0 |
| 1e15 to 1.6e15 | 24 090 797 | 0 | 89, the first at 1.02e15, each a tie that takes the larger line |
| 1.6e15 to 1.7e15 | 3 441 600 | 16, the first at 1.69e15 | 195 |
| 2^51 to 2^52 (first sweep, wider steps) | 588 128 | 13 266 | 30 111 |

- **The doc comment of `grid_target` is true.** It is the nearest line up to about 1e15 px, and past about 2^51 no fixed window is.
- **The corrected claims in `apply-progress.md` are true.** Item 14: `[k, k+1]` is not equivalent and is killed by the named test. The mutant first differs at `5.47e14` here (73 of 1 500 000), so "about 6e14" is loose but fair. Item 16: `n < 2` is corrected and the mutant is killed.
- **The W2 line of "Fix round 1" has two wrong figures and one omission. None changes the decision:**
  - "From about 1.3e15 (about 2^51)": those two numbers are not the same point. 2^51 is 2.25e15, and 1.3e15 is about 2^50.2. I found no wrong distance at 1.3e15; the first is at 1.69e15, about `2^53 * 0.1875`, where the quotient stops being exact at the smallest spacing. The band `[2^50, 2^51)` is the likely source of the 2^51 label.
  - The tie rule fails a little earlier than the distance: from 1.02e15 a tie can take the larger line, just above the stated 1e15. The note gives no figure for this.
  - "4.5e10 times the largest page": with `MAX_PX` 16 384, 1e15 is 6.1e10 times the page and 1.3e15 is 7.9e10. 4.5e10 would be 7.4e14.

### WARNING

**W2. Grid target past about 1e15 px.** class: `implementation`, `follow-up`. Ref: `src/snap.rs:158`, `apply-progress.md` "Fix round 1". Accepted limit, stated in the doc comment; see above. The note can be corrected at archive. If the owner wants exactness past the limit, the row needs a range first.

**W5. No test pins the page centre with the magnet off.** class: `implementation`, `follow-up`. Ref: `tests/snap.rs` `arq_110_an_inactive_magnet_computes_no_centre_quotient_guide_or_distance`, row "Entradas del Imán" ("ni centros"). The test covers moving, element, threshold, distance and guide, but no page box whose centre overflows. The code is right: the return comes before every centre, and the probe above includes such pages. Found by reading the test; no mutant was run for it, per the closure limit. New test gap after round 1, so not FAIL.

### SUGGESTION

**S1, S2, S3.** Open as in round 1, unchanged. The implementer left them for the spec, follow-up or owner.

**S5. Stale text left by the fixes.**
- `apply-progress.md:294` still says every centre is checked "with the magnet off" and that the spec's example is a zero-size box. Both are out of date after W1 and `971d7c4`. Line 319 still lists S4 as not done.
- `apply-progress.md:287`, item 14, ends in a leftover "And, old text: ...".
- `tests/snap.rs` says "the guide of the last case" but reads `cases[3]`, which stopped being the last case in `00b1bcd`.

### Mutants re-run

Run 8, killed 8. These are closure re-runs, no new independent ones. Baseline: 167 tests of the crate pass.

| Mutant | Origin | Killed by |
|---|---|---|
| `n < 2` for `n < 3` | Survivor, C2 | `d_17_two_wide_*` |
| Margin and element box checks after DPI | Survivor, C3 | `the_box_of_every_input_*` |
| Moving box check after DPI | C3 | `the_box_of_every_input_*`, `the_order_of_checks_*` |
| Page box check after DPI | C3 | `the_box_of_every_input_*` |
| Neighbours `[k, k+1]` | Survivor, W2 | `fd_06_the_grid_target_is_the_nearest_line_*` |
| `grid_target` without `+ 0.0` | Reverts C1 | `fd_06_a_grid_target_is_never_negative_zero_*` |
| `moved` without `+ 0.0` | Reverts W3 | `fd_11_a_zero_offset_*` |
| Inactive return back after the computed values | Reverts W1 | `arq_110_an_inactive_magnet_computes_no_*` |

### Round 1 probes

All re-run at `517c3cc` with the same seeds; nothing regressed.
- FD-06, D-16, FD-11, rotation, determinism, the row examples, attractors and the 100 000 level chains: all pass (empty selection 91 ms in release).
- `node_bounds` against D4: 0 differences.
- `grid_lines` against D4: 7 040 differences, all at `|x0| >= 1e15`, the same as S2. That probe's own `assert_eq!(bad, 0)` fails by design, as it did in round 1.
- Past `2^51`, `grid_huge_references` gives the same counts as round 1.

### Acceptance map

Criteria: 32 of 32 covered; FD-11(c) stays out of this crate (row "Primer corte"). Changes from round 1:

| Item | Round 1 | Round 2 |
|---|---|---|
| Row DPI | weak, C3 | covered, `the_box_of_every_input_is_checked_before_the_dpi_rule` |
| D-17 | weak, C2 | covered, `d_17_two_wide_*` |
| Row Destino en la Cuadrícula | failing, C1 | covered up to the W2 limit, `fd_06_a_grid_target_is_never_negative_zero_*`, `fd_06_the_grid_target_is_the_nearest_line_*` |
| Row Elementos que atraen | contradictory, C4 | covered, `arq_110_a_hidden_or_locked_child_*` |
| Row Entradas del Imán | covered, W1 | covered, `arq_110_an_inactive_magnet_*`, W5 |
| FD-03(c), FD-11(d) | covered, W4 | covered and named |

### Gates and integrity

Gates at `517c3cc` on a clean clone: `cargo fmt --check` 0; clippy 0; `cargo test --workspace --locked` 0 (1 098 passed, 0 failed); `cargo deny --offline check` 0 (advisories, bans, licenses, sources ok). The clone stayed clean. Disk: 55 GB free at the start, 54 GB at the end; build folders freed.

CI policy, all 0, read from `ci.yml` on the clean clone: Format, ARQ-07, `forbid(unsafe_code)`, V5, V12, V13, V14.

Per commit on clean exports:

| Commit | fmt | Clippy | Test | Passed |
|---|---|---|---|---|
| `d19a59a` | 0 | 0 | 0 | 1 092 |
| `0554dfa` | 0 | 0 | 0 | 1 095 |
| `3ee32de` | 0 | 0 | 0 | 1 098 |
| `00b1bcd` | 0 | 0 | 0 | 1 098 |

`0554dfa` passes alone, so the deviation in `apply-progress.md` has no effect. `517c3cc` changes notes only and is the clean clone above.

Integrity:
- Slice card: lines 1 to 10 of `apply-progress.md` hash the same at `a584943` and HEAD.
- `docs/specs`: since the first implementation commit, only `ab3cd7a` and `971d7c4` touched it. Both touch only `02-arquitectura-software.md`, and no implementer commit touched it.
- No tracked file deleted, no `*.proptest-regressions`, and no change in `.cargo`, `.github`, `deny.toml`, `Cargo.toml` or `Cargo.lock` since round 1.
- No absolute paths, `#[allow]` or `unsafe` added, and no AI attribution in commit bodies.

Size, figures only: `git diff --shortstat develop -- crates Cargo.toml Cargo.lock` gives 17 files, 4 319 insertions, 43 deletions. Fix round: 6 files, +312 -19 (source +28 -19, tests +284), matching the implementer's.

## Round 1

Round 1 at `a77292d`: **FAIL**, 4 CRITICAL, 4 WARNING, 4 SUGGESTION. Kept as the record of what round 2 closed.

Gates ran on a clean clone. Probes and hand mutants ran on clean exports, outside the repo. Probes were written from the specs, before the implementer's tests were opened.

### CRITICAL

**C1. A reference of `-0.0` gives a grid target of `-0.0`.** class: `implementation`. Ref: `crates/gafira-layout/src/snap.rs:164-167`, `src/grid.rs:62-71`, row "Destino en la Cuadrícula".
With reference `-0.0`, `(at / spacing).floor()` is `-0.0`, so line `k` sits at `-0.0`. `grid_lines` casts `k` through `i64` and gives `+0.0`. The row's property requires the same bits.
- Probe: step 10, reference `-0.0`: coordinate bits `8000000000000000`, `grid_lines` bits `0`. The offset is right.
- Sweep against `grid_lines`: 2 310 300 cases (1 133 042 negative, 189 415 ties), 8 differ, and all 8 have a reference of exactly `-0.0`. Every other case matches bit for bit.
- It is reachable: `(-0.3_f64).round()` is `-0.0`.
- The impact is one sign bit, and the value compares equal. It is CRITICAL because it fails a stated acceptance property. The test property draws `around` from `-5000..5000`, which never yields `-0.0`.

**C2. `n < 2` for `n < 3` is not equivalent, and it survives.** class: `implementation`. Ref: `src/distribute.rs:63`, FD-11 / D-17 "con menos de 3 libres no hace nada", `apply-progress.md` D5 item 16.
The code is right; the claim and the test are not. With two free boxes the mutant computes the gap, which can overflow when wide boxes overlap, because `first_end` can be `+MAX` while `last_start` is negative.
- `[(-MAX, MAX), (-MAX, MAX)]`: HEAD gives two zero offsets; the mutant gives `Err(NonFinite { field: "gap" })`. The same holds with a locked box between them.
- Adversarial comparison with HEAD: 39 698 of 3 000 000 results differ.
- All 156 tests of the crate pass on the mutant.

**C3. Box checks of margins and elements after the DPI rule survive.** class: `implementation`. Ref: `src/snap.rs:140-148`, row "DPI", `tests/snap.rs:346`.
The code is right: on HEAD, `dpi = 0` with `x1 < x0` gives `InvalidBox` for `moving`, `page`, `margins` and `element`. The row orders all box checks before the DPI rule, but the test proves it for `moving` only. Moving the `margins` and `element` checks after the DPI rule passes all 156 tests.

**C4. A hidden or locked child of an attracting group: the row contradicts itself.** class: `ambiguity`. Ref: 02 row "Elementos que atraen", `src/attractors.rs:33-34`, `tests/attractors.rs:357`.
Answer to the question asked: the row's literal text does not allow what the code does, and it does not allow the alternative either.
- "Quedan fuera ... los nodos ocultos o bloqueados" and "un NaN en un nodo que queda fuera no cambia el resultado" say the NaN must not matter.
- "Cada caja es la de `node_bounds`" and "el grupo ... por la unión de sus hijos" make the group read that child, and `node_bounds` gives `Err` on it.
- Probe: A holds B and C, B holds D and E, E is hidden with a NaN `x`. Empty selection gives `Err(NonFinite { field: "x" })`. Selecting C gives the same error. A locked E gives the same error. Selecting D gives `Ok`, because A and B are out.
- One hidden corrupt child stops the magnet for the whole page. The acceptance examples only cover NaN in the selection and its ancestors, and the implementer pinned one reading in a test.

The specifier must choose one:
- Keep `node_bounds` and state the exception.
- Have the group's box skip nodes that are left out, which also changes finite cases: a hidden child at 500 today widens the box to `x1 = 501`.

### WARNING

**W1. Which computed checks run with the magnet off is invented.** class: `ambiguity`. Ref: `src/snap.rs:240-266`, row "Entradas del Imán".
The row says that an inactive magnet does "las comprobaciones de entrada y de rango" and gives (0, 0). HEAD also runs some computed checks. All inputs here are finite and in range:

| Magnet off | HEAD |
|---|---|
| Moving box at `x = 1e308`, centre overflows | `Err(moving)` |
| Element box at `x = 1e308`, centre overflows | `Err(element)` |
| Threshold `1e308` at zoom 0.05 | `Err(threshold)` |
| Distance that would overflow | `Ok`, not computed |

Row "Suma que desborda" can be read either way, so the row should state it. `align` and `distribute` have no such problem: their row applies the rules "si la caja no interviene".

**W2. Grid neighbours `[k, k+1]`: the equivalence claim is false.** class: `implementation`, `follow-up`. Ref: `src/snap.rs:166`, `apply-progress.md` D5 item 14.
The mutant passes all tests but differs from HEAD in 73 of 1 500 000 adversarial cases, the smallest at reference `5.47e14` px, so the neighbours do change the result. It is a WARNING because that is about 3e10 times the largest page, and no difference appeared below it. HEAD itself stops being the nearest line from about `2^51` (2 of 200 000 cases there, 2 064 at `2^55`).

**W3. A `-0.0` offset is reachable in `distribute`.** class: `implementation`. Ref: `src/distribute.rs:83-85`, `apply-progress.md` "Zero offsets are `+0.0` in all four functions".
Boxes `(-1, -0.0)`, `(0, 5e-324)`, `(0, 1)` in equal-gap mode give `dx = -0.0` on the middle box. The gap underflows to `-0.0`. The spec only says 0, so behaviour is acceptable, but the comment and the note are false, and the `+ 0.0` was removed as unreachable.

**W4. Two criteria have no test that names them.** class: `implementation`. Ref: Slice card, `AGENTS.md` testing policy.
FD-03(c) and FD-11(d) are covered in substance by `fd_06_a_five_px_from_a_guide_the_edge_lands_on_it` and `d_17_locked_elements_do_not_count_and_do_not_move`, and the IDs FD-03 and FD-11 are named elsewhere. No test or comment names these two criteria, so a search on them finds nothing. It is not the CRITICAL of step 3c, which applies per ID.

### SUGGESTION

**S1.** `snap_rotation` is not the nearest multiple from `9.0e15` degrees (`2^53`): 37 704 of about 4 000 000 oracle cases, for example 7e16 gives `7.000000000000001e16`, 16 from the angle. No test reaches this, and below that magnitude the exact oracle has no difference. The row has no range, so a sentence in the spec would settle it. `f64::MAX` gives `Err(angle)`, which fits the intermediate overflow rule.

**S2.** `grid_lines` differs from D4 only at `|x0| >= 1e15`, past `2^53` lines: 7 040 of 1 500 000 inputs, in the `major` flag among positions that already repeat. See "D4 behaviour".

**S3.** `benches/layout.rs:4` says a nightly job checks the budgets. No such workflow or `perf-budgets.toml` exists, and criterion gives no p99 for the `bench_snap_1000` row. ARQ-40 is outside this slice and ARQ-114 only asks for the names; the comment should not imply the job exists.

**S4.** The row example "caja en `x = −MAX`" gives its reason as the distance. A zero-size box fails earlier at the centre, naming `moving`. `Rect(-MAX, 0, 0, 0)` gives `distance`. Both are non-finite errors, so only the wording differs.

### Independent checks asked for

**Per-commit gates:** all pass on clean exports (table under "Gates and integrity").

**Priority and ties (D-16):** all four adjacent pairs and the exact comparison hold on probe. The spot-check is killed.

**Grid target against `grid_lines`:** every row example passes; non-integer steps, negative `k`, mm, in, pt, cm and the shared level all hold bit for bit, except C1. There is no view limit.

**D4 behaviour:**
- `grid_lines` against the D4 code in process: 653 137 Ok pairs and 846 863 Err pairs, with 7 040 differences, all at `|x0| >= 1e15` (S2). Error kinds and order match.
- `node_bounds` against D4 on 400 000 random graphs with cycles, dangling children, shared nodes and bad numbers: 0 differences.

**Cache:** no stale or wrong entry. An independent tree oracle over 300 000 `attractors` calls, 21 844 of them with errors, gives 0 differences. The cache only lives inside one call.

**`attractors`, 100 000 levels:**

| Case | Debug | Release |
|---|---|---|
| Chain, empty selection | 616 ms | 79 ms |
| Chain, selection deep, root or middle | 281 to 291 ms | 32 to 34 ms |
| Chain closed into a loop, gives `Cycle` | 224 ms | 30 ms |
| Comb of 50 000 levels (99 999 boxes) | 581 ms | 74 ms |

Ancestors of the selection hold, including the row's A, B, C, D, E case and NaN in D or A.

**`distribute`:** the stable sort ties `-0.0` and `0.0` in input order. An oracle over 300 000 random calls gives a gap spread of `8.5e-13`. Offset overflow gives `Err`. See C2 and W3.

**Equivalence claims:**

| Claim | Result |
|---|---|
| `n < 2` for `n < 3` | **False**, C2 |
| Grid neighbours `[k, k+1]` | **False** at `>= 5.47e14` px, W2 |
| Reference order before coordinate | Holds: 0 of 1 500 000 adversarial cases differ. The proof holds generally: the later reference is itself closer to the larger target, whether or not a centre lies between them |

**Implementer's choices:**

| Choice | Finding |
|---|---|
| Check order, threshold before zoom | Note: unspecified detail; the DPI row is respected |
| Centres always checked | `align`, `distribute`: allowed. `snap` with the magnet off: W1 |
| Inactive magnet computes no distance | Allowed by the row; part of W1 |
| Zero offsets `+0.0` | Note: spec says 0; the claim is false, W3 |
| `attractors` in document order | Note: unspecified, deterministic |
| `margins: None` left to the caller | Note: the row makes it optional, and `snap` reads no document |
| Cycle or dangling child in a node left out gives `Err` | Note: the row speaks only of NaN; such a tree fails validation |
| Selected id in the arena but unreachable | Note: not an error; only a missing id is |

**Benches:** the four names match §6.7, each registered once. Quick run: `bench_guides_generate` 360 µs, `bench_layout_rects` 25 ns, `bench_snap_1000` 30 µs, `bench_grid_lines` 7.8 µs; each is under its figure on this machine, which is not the reference one. `criterion` 0.8 has `default-features = false`.

**`Cargo.lock`:** +33 entries (32 new names, 1 new version), none removed. `criterion`, `half`, `ciborium`, `rayon` and `plotters` do not appear in the normal or build graph of the workspace. §9.2 lists `criterion` as dev. The V5 row needs no change.

### Hand mutants

Run 4, killed 3. No recorded list was repeated.

| Mutant | Type | Result |
|---|---|---|
| Page and Element swapped in priority (item 2) | Spot-check | Killed by the recorded `fd_06_f_at_equal_distance_each_kind_beats_every_lower_one` |
| `snap` centre as `lo + (hi - lo) / 2` | Independent | Killed by `arq_110_a_result_that_is_not_finite_is_an_error_even_if_the_value_would_be_finite` |
| Centres mode sorted by start edge | Independent | Killed by 3 tests |
| Margin and element box checks after DPI | Independent | **Survives**, C3 |

The three equivalence spot-checks are counted separately above.

### Acceptance map

| Item | State | Proof |
|---|---|---|
| FD-03(c) | covered, W4 | `fd_06_a_five_px_from_a_guide_the_edge_lands_on_it` |
| FD-06(a) to (i) | covered | `fd_06_a_*` to `fd_06_i_*`, probes |
| FD-06(c) | covered, see C4 for the row | `fd_06_c_*`, oracle |
| FD-11(a), (b), (e), (f) | covered | `fd_11_*` |
| FD-11(d) | covered, W4 | `d_17_locked_elements_do_not_count_and_do_not_move` |
| Row Umbral | covered | `fd_06_the_threshold_is_compared_exactly_and_divided_by_the_zoom`, `threshold_must_be_finite_and_above_zero` |
| Row Zoom | covered | `zoom_range_is_closed_and_checked_without_targets_and_with_the_magnet_off` |
| Row Empate | covered | `fd_06_after_the_kind_the_smaller_coordinate_wins`, `d_16_*` |
| Row DPI | **weak**, C3 | `dpi_must_be_above_zero_even_when_nothing_uses_it`, `the_order_of_checks_*` |
| Row Entradas del Imán | covered, W1 | `arq_110_*`, `d_16_zero_margins_*`, `guides_convert_*` |
| Row Elementos que atraen | **contradictory**, C4 | `arq_110_a_nan_outside_the_attracting_nodes_changes_nothing` |
| Row Destino en la Cuadrícula | **failing**, C1 | `fd_06_the_grid_*`, `fd_06_a_major_line_is_exactly_k_times_the_step` |
| Row Giro del Imán | covered, S1 | `fd_06_i_*` |
| Row Entradas de alinear y distribuir | covered | `fd_11_the_result_has_one_offset_per_box_in_the_input_order`, `d_29_*`, `arq_110_*` |
| Row Tolerancia | covered | `fd_11_a_*`, `fd_11_b_*`, oracle |
| ARQ-110 | covered | `arq_110_*` in five test files |
| ARQ-111 | covered | `arq_111_*` in five test files |
| ARQ-114, §6.7 rows | covered | `arq_114_each_layout_figure_has_a_benchmark_named_after_its_row` |
| D-16 | covered | `d_16_*`, `fd_06_f_*` |
| D-17 | **weak**, C2 | `d_17_*` |
| D-28 | covered | `d_28_*`, `fd_06_c_a_rotated_node_*` |
| D-29 | covered | `d_29_locked_elements_count_in_the_box_but_never_move` |

Criteria: 32 counted with FD letters and rows one by one; 28 covered, 4 not (the two weak rules, one failing, one contradictory). FD-11(c) is out of this crate (row "Primer corte").

### Gates and integrity

Gates at HEAD: `cargo fmt --check` 0; clippy 0; `cargo test --workspace --locked` 0 (1 087 passed, 0 failed); `cargo deny --offline check` 0 (advisories, bans, licenses, sources ok, no warnings). Disk: 60 GB free at the start, 56 GB after the builds.

CI policy, all 0: ARQ-07, `forbid(unsafe_code)`, V5, V12, V13, V14.

Per commit on clean exports:

| Commit | Clippy | fmt |
|---|---|---|
| `4a99a1a` | 0 | 0 |
| `117c1dc` | 0 | 0 |
| `866b8b6` | 0 | 0 |
| `761a2ef` | 0 | 0 |
| `7a1b454` | 0 | 0 |
| `65217bc` | 0 | 0 |
| `b947b02` | 0 | 0 |
| `594f304` | 0 | 0 |
| `0c686e2` | 0 | 0 |
| `a77292d` (notes only) | 0 | 0 |

`cargo test` was not repeated per commit; that belongs to V3.

Integrity:
- No tracked file deleted, and no `*.proptest-regressions` added or changed; the test run left the clone clean.
- `a584943` touches only `apply-progress.md` and states why. The Slice card is unchanged since then: the later diff of that file has 0 removed lines.
- No implementer commit touched `docs/specs`; only `dc6e4d6` and `7aa7aa5` did, and both precede the first implementation commit.
- No absolute paths in the tracked files checked, no `#[allow]`, no `.cargo/mutants.toml` change, no AI attribution in commit bodies.

Size, figures only (no ceiling, owner decision), matching the implementer's: `git diff --shortstat develop -- crates Cargo.toml Cargo.lock` gives 17 files, 4 026 insertions, 43 deletions. Source +804 -42, tests and bench +2 918, `Cargo.lock` +299. Forecast 935 (475 + 335 + 125).

Round 1 next: specifier (C4, W1), then tdd-apply (C1, C2, C3, W2, W3, W4)

Round 2 next: mutants (T4 at change close on `gafira-doc`, `gafira-edit`, `gafira-layout`), then archive

next: tdd-apply
