# Verify report: layout-core D4

Current state: round 3 at `553aaf3`, depth V2, mode full, tier T4 (`mutation: archive`, no `mutants` report): **PASS** for D4, D5 to D7 remaining. 0 CRITICAL, 1 WARNING, 2 SUGGESTION. Per class: implementation 1 (WARNING); architecture 0; ambiguity 0; claim 0. Next: `tdd-apply`. C4, W5 (by the owner), W6 and S4 are closed. W7 (`follow-up`) records two commits that fail clippy alone; the owner decides on it.

## Round 3

Closure round, the last allowed for D4. Commit `553aaf3`, depth V2, mode full, tier T4 (`mutation: archive`, no `mutants` report, so no mutation finding). Gates ran on a clean clone and probes on clean exports, outside the repo. No new independent mutants.

### WARNING

**W7. Two commits do not pass clippy alone.** class: `implementation`, `follow-up`. Ref: `AGENTS.md` "Strict TDD", `9ab2e9f`, `8c261f3`, `crates/gafira-layout/tests/grid.rs:401-402`.
The fact is confirmed on clean exports (table below). Both fail `clippy::manual_is_multiple_of` in a test, and `6943995` fixes it. This breaks "every commit must pass alone", and only the owner can waive it or decide to rewrite history.
It is a WARNING and not a gate failure because V2 gates run at HEAD, per-commit gates belong to V3, and the rules class this as commit hygiene. fmt and tests pass on both commits, and the source fix is not what fails. The cost is two red clippy points in `develop` history once merged, which matters only to a bisect that runs clippy.
Two related points:
- `grid.proptest-regressions` was added in `8c261f3` and removed in `6943995`. It leaves no trace at HEAD.
- The notes say the implementer's gate script passed while clippy failed. That script hid a failure, so check it before D5. Every verdict here comes from exit codes taken directly.

| Commit | fmt | clippy | test |
|---|---|---|---|
| `9ab2e9f` | 0 | **101** | 0; 992 passed |
| `8c261f3` | 0 | **101** | 0; 994 passed |
| `6943995` | 0 | 0 | 0; 994 passed |
| `553aaf3` (HEAD) | 0 | 0 | 0; 994 passed |

### SUGGESTION

**S2, open, `follow-up`.** V10 still survives; no change since round 2.

**S5. Round 2 understated the trade-off of the C4 fix.** Ref: `crates/gafira-layout/src/grid.rs:60-61`, D5 snap.
A caller that writes a major edge as `k × step / subdivisions` loses that line. Round 2's 1.0 % is reproduced exactly (23 303 of 2 230 255), but that denominator includes minor edges, which never lose a line. Over major edges alone the figure is 55 190 of 842 229 views (6.6 %). The row gives `k × paso` for a major line, so the code is right. D5 and any caller should compute major edges that way.

### Closure

| Finding | State | Evidence |
|---|---|---|
| C4 | **closed** | See below. |
| W5 | closed by the owner | Size accepted, 2026-11-01. Figures under "Size". |
| W6 | closed | `fd_03_d_non_finite_position_is_reported_at_dpi_zero` and `arq_110_a_nan_margin_is_non_finite_at_dpi_zero` use the row's values and assert the exact field. Not mutated, per the closure scope. The other row examples at `dpi = 0` were already tested. |
| S4 | closed | The notes now say src 25, tests 210, and that the old 1 370 counted the verify artifacts. Their 1 235 is off by one against round 2's table, which is immaterial. |
| S2 | open, `follow-up` | See above. |

The owner's "W8, S2" follow-ups: W8 is a D3 finding (10 mm in px refused at some DPIs), not a D4 one, so it is not counted here. S2 has two meanings: the D3 S2 (no code holds the 5 % default of D-38) and the D4 S2 (V10). Both stay `follow-up`, so the reading does not change this verdict. `archive` must carry both.

### C4

Reproduction (step 4.8 px, 3 subdivisions, zoom 4, view 0 to 52.8): 34 lines, the last a major at exactly 52.8. The mirror view from −52.8 to 0 gives 34 lines, the first a major at exactly −52.8.

Round 2's five sweeps. View counts match round 2 exactly. The mutant column restores `k × step / subdivisions` for majors and gives round 2's figures back line for line.

| Steps | Views | Lost, HEAD | Lost, mutant |
|---|---|---|---|
| 1 to 500 px, in tenths | 1 258 641 | 0 | 80 237 |
| mm at 96 dpi | 49 851 | 0 | 3 909 |
| mm at 300 dpi | 17 474 | 0 | 1 294 |
| in at 96 dpi, in tenths | 16 101 | 0 | 1 182 |
| pt at 96 dpi | 105 825 | 0 | 5 617 |

Mutant against the crate's tests: killed by exactly the two new tests, `fd_07_a_major_line_on_a_view_edge_is_kept_for_a_non_integer_step_with_subdivisions` and `fd_07_a_majors_sit_exactly_on_multiples_of_the_step`. It is the only source change on that export. Hand mutants: 1 run, 1 killed.

### Minor lines and duplicates

| Check | Scope | Result |
|---|---|---|
| Same minor lines as before the fix | 600 000 random views, 7 741 538 minor lines, digest of their bits | identical at `ba6c91d` and HEAD (`2eb1dd304dafef1e`) |
| Every line in the five sweeps | 1 447 892 views | 0 wrong bits, wrong `major` flags, duplicates or order faults |
| Independent scan | the same random views: negative positions, the `2^k` fallback, four units, edges on lines | 0 missing, 0 extra, 0 wrong flags, 0 duplicates, 0 out of order |
| Minor line on a view edge | 1 799 131 views | 0 lost, at both commits |

The same scan on `ba6c91d` reports 260 971 missing and 246 865 extra: those are the majors one ulp off, so the oracle does see the old defect.

The two inputs the removed regressions file shrank to pass on the public API at HEAD, and with that file restored on the export `tests/grid.rs` stays green. `tests/grid.rs` at 500 000 cases per property passes. At 20 000 cases `fd_07_fallback_...` first stopped at proptest's limit of 1 024 rejected inputs, with no failing input; with the limit raised it passes. That is the harness, on an unchanged test, and the default 256 cases stay far from the limit, so it is not a finding.

### Size against `develop`

| Part | Insertions | Deletions |
|---|---|---|
| Source | 288 | 1 |
| Tests | 940 | 0 |
| Notes | 82 | 0 |
| Spec | 4 | 1 |
| Verify artifacts at `553aaf3` | 228 | 480 |
| Total | 1 542 | 482 |

This round added 66 insertions and 5 deletions in `crates`. Accepted by the owner.

### Acceptance map

16 criteria, 16 covered. Changes since round 2: FD-07 (a) and row "Líneas" are covered (C4 closed), and row "DPI" is covered (W6 closed). The rest are unchanged. Left out on purpose, as in round 1: FD-03 (c), FD-06 (D5); FD-05, FD-09 (D3); FD-11 (D6); ARQ-114 (D7).

### Gates and integrity

Run on a clean clone at `553aaf3`. The tree stayed clean after the run. Linux only: the Windows and macOS CI jobs did not run here, as in earlier rounds.

| Check | Result |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0; 994 passed, 0 failed |
| `cargo test -p gafira-layout --locked --release` | 0 |
| `cargo deny check` | 0 |
| CI policy: format, ARQ-07, forbid unsafe, V5, V12, V13, V14 | all 0, run from `ci.yml` |

- Slice card: `f08d005` touches only `apply-progress.md`, and nothing above the D4 section changed.
- Spec: no fix commit touched `docs/specs`. Its only D4 commits are still `33ab1c5`, `463b777` and `b7738c0`.
- Regressions files: no `*.proptest-regressions` differs from `develop` at HEAD (9 on both sides, none in `gafira-layout`).
- Branch: `ci.yml`, `.cargo`, `Cargo.toml`, `Cargo.lock` and `deny.toml` unchanged; no absolute path added; no attribution lines; nothing changed outside `gafira-layout`, `docs/specs` and the change folder.

---

## Round 2

Closure round. No new independent-mutant budget. All probes ran on scratch copies outside the repo.

### CRITICAL

**C4. Residual of C1: a major line on a view edge is still lost when subdivisions show and the step is not an integer.** class: `implementation`. Ref: `crates/gafira-layout/src/grid.rs:58`, FD-07 (a), row "Líneas de `grid_lines`".
The row places lines at `k × paso` and at `k × paso / subdivisiones`, and it includes both view edges. The code computes every line as `k × paso / subdivisiones`, majors too. Multiplying before dividing can land one ulp away from `k × paso`, and the exact `contains` filter then drops the line.

Reproduction on the public API: step 4.8 px, 3 subdivisions, zoom 4, view from 0 to 52.8. `11 × 4.8` is `52.8`, a major line on the edge. The code computes `33 × 4.8 / 3 = 52.800000000000004`, so 33 lines come back, not 34, and the last is the minor line at 51.2.

Views from 0 to `j × step`, with the minor grid showing:

| Steps | Views | Lost a line |
|---|---|---|
| 1 to 500 px, in tenths | 1 258 641 | 80 237 (6.4 %) |
| mm at 96 dpi | 49 851 | 3 909 (7.8 %) |
| mm at 300 dpi | 17 474 | 1 294 (7.4 %) |
| in at 96 dpi, in tenths | 16 101 | 1 182 (7.3 %) |
| pt at 96 dpi | 105 825 | 5 617 (5.3 %) |

Why the tests miss it: `fd_07_a_every_line_...` uses integer steps, where `k × s` is exact, and `fd_07_a_mm_steps_...` uses 1 subdivision, where the two formulas are the same.

Trial fix, not committed: `(k / every) as f64 * unit` when `k % every == 0`. All five sweeps above drop to 0 lost and the crate suite stays green. One trade-off: a caller that computes a major edge as `k × step / subdivisions` instead of `k × step` then misses that line in 23 303 of 2 230 255 views (1.0 %). That formula is not the one the row gives for a major line. A new test needs a non-integer step with subdivisions.

### WARNING

**W5. Size, still open.** class: `implementation`. The owner decides; not a blocker. Figures are under "Size".

**W6. Two acceptance examples of the DPI row have no test.** class: `implementation`, `follow-up`. Ref: row "DPI de `bleed_rect`, ...", `tests/guides_position.rs:36`, `tests/rects.rs:253`.
With `dpi = 0`, the row requires `Err` of non-finite for a NaN position in `place_guide` and a NaN margin in `margin_rect`. The tests use those NaN values only at 96 dpi. The code is right: the probe on the public API passes all five functions, including an inverted box at `dpi = 0`.

### SUGGESTION

**S2, stays open.** V10 still survives. The round 3 notes leave it as is and no spec rule covers it; no action needed.

**S4.** The round 3 notes split the 235 inserted lines as src 34, tests 201; `git diff --numstat` gives src 25, tests 210. Their "1 370 insertions with these notes" also counts the 135 lines of the verify artifacts.

### Closure

| Finding | State | Evidence |
|---|---|---|
| C1 | **closed for its reproductions; rule still open, C4** | Step 7, 3 subdivisions, zoom 4, view 0 to 35 gives 16 lines, the last a major at 35. Integer steps with whole-multiple views: 0 lost of 1 264 176. A mm step (no subdivisions) with an edge on the line: 0 lost of 830 574. |
| C2 | closed | V1, V2 and V5 killed. V4 as written is no longer a mutant; the tolerance itself is pinned, see below. |
| C3 | closed | V3 killed, on both axes and on the x axis alone. |
| W1 | closed | Notes corrected. V11 killed. Removing the raw `finite(gap)` or `finite(margin)` is killed by `fd_04_dpi_is_checked_...`. Removing the dpi check in `grid_lines` is killed by `fd_07_dpi_is_checked_...`. |
| W2 | closed | `fd_07_c*` alone now fails 5 of 5 under V6 and 5 of 5 under V7 (it was 2 of 5 and 0 of 5), through `fd_07_c_the_densest_cases_stay_under_the_limit`. The property test for the `2^k` fallback did not change, and the direct case replaces it. |
| W3 | closed | 31 951 zooms, the view computed as `screen / zoom`: 0 `Err` at the origin, 0 at three panned corners. Worst excess 5.8e-11 px, well under `ε`. The tolerance adds no line: 1 002 at most, also with offset views. |
| W4 | closed | Modules public; rows "DPI", "Líneas" and "Lado de página" state the rest; V9 killed. |
| S1 | closed | Both tests use the row's literal values. |
| S2 | open, accepted | See above. |
| S3 | closed | Status and notes current; S4 is a new, smaller slip. |

### Hand mutants

20 run on the export, all reverted: 19 killed, V10 survived.

| Mutant | Result |
|---|---|
| Spot-check, item (5), `c > 1` | killed: `fd_04_d_...`, `fd_04_page_side_limit_...` |
| V1 | killed: `fd_04_d_...` |
| V2 | killed: `fd_04_f_zero_gap_...` |
| V3 both axes, V3 x only | killed: `fd_03_guides_are_measured_...` |
| Replaces V4: `ε` at 1e-5, 1e-7 | killed: `fd_07_view_screen_size_...` |
| Replaces V4: `ε` at 0 | killed: the same test and `fd_07_c_at_most_2000_...` |
| V5 | killed: `fd_04_page_side_limit_...` |
| V6 | killed: 4 tests |
| V7 | killed: 6 tests |
| V8 | killed: `arq_110_finite_input_...` and 8 more |
| V9 | killed: `fd_07_lines_at_negative_...`, `fd_07_a_every_line_...` |
| V10 | survived, S2 |
| V11 both, first bound, last bound | killed: both `fd_07_a_*` tests |
| Raw `finite(gap)`, raw `finite(margin)` removed | killed: `fd_04_dpi_is_checked_...` |
| dpi check removed in `grid_lines` | killed: `fd_07_dpi_is_checked_...` |

### RED of round 3

Reproduced by running the HEAD tests on older source.
- On `17ad901` source: E0603 on `grid` and on `guides`. The other compile errors (E0277, E0282, E0689) follow from those unresolved imports.
- On `bde75e9` source (modules public, old code): 6 tests fail, the 6 the notes name.

W4 is the only compile-error RED. One nuance: the notes head this list "assertion failures", but `fd_07_c_at_most_2000_...` fails through `unwrap()` on `Err(view)`. That is still a runtime failure on behaviour, so it is not a second case of W4's kind. The other 5 fail on assertions, with the values the notes record where they give any.

### Root re-exports

Fine, note only. `grid` and `guides` being public makes the spec paths real. The root re-exports duplicate them but match the crate's other functions, and the tests use both forms.

### Size against `develop`

| Part | Insertions | Deletions |
|---|---|---|
| Source | 285 | 1 |
| Tests | 882 | 0 |
| Notes | 65 | 0 |
| Spec | 4 | 1 |
| Verify artifacts | 135 | 517 |
| Total | 1 371 | 519 |

Source and tests come to 1 167 insertions, against the ~700 ceiling and a forecast of 340 (475 at x1.4). Tests alone pass the ceiling. The owner decides; not a blocker. The verify artifacts' deletions are the D3 report this file replaced.

### Acceptance map, changes since round 1

16 criteria, now counting the rows "Líneas" and "DPI": 13 covered, 3 partial.

| Criterion | State | Proof |
|---|---|---|
| FD-07 (a) | partial | C4 |
| Row "Líneas de `grid_lines`" | partial | C4; its two literal examples are tested |
| Row "DPI" | partial, code right | W6 |
| FD-04 (d), FD-04 (f), FD-07 (c), rows "Lado de página" and "Vista" | covered | C2, W2 and W3 closed; the row "Vista" literal examples and its property are tested |

The other criteria are unchanged from round 1.

### Gates and integrity

Run on a clean clone at `ba6c91d`. The tree stayed clean after the run.

| Check | Result |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0; 990 passed, 0 failed |
| `cargo test -p gafira-layout --locked --release` | 0 |
| `cargo deny check` | 0 |
| CI policy: format, ARQ-07, forbid unsafe, V5, V12, V13, V14 | all 0, run from `ci.yml` |

- Slice card: `f08d005` touches only `apply-progress.md`, and nothing above the D4 section changed.
- Spec: since D4 began, `docs/specs` was changed only by `33ab1c5` (round 1 record), `463b777` and `b7738c0`, each touching only `02-arquitectura-software.md`. No fix commit touched it.
- Branch: `ci.yml`, `.cargo`, `Cargo.toml` and `Cargo.lock` unchanged; no `*.proptest-regressions` change (9 on both sides); no absolute path added; no attribution lines; nothing changed outside `gafira-layout`, `docs/specs` and the change folder.

The next round is the third and last of D4 under the three-round rule.

---

## Round 1

Change `layout-core`, delivery D4, round 1, depth V2, mode full, tier T4 (`mutation: archive`, so no `mutants` report and no mutation finding).

Commit verified: `47617dc`, the tip of `feat/layout-core-d4`. Verdict: **FAIL**. Counts: 3 CRITICAL, 5 WARNING, 3 SUGGESTION. Per class: implementation 6 (3 CRITICAL, 3 WARNING); ambiguity 2 (WARNING); architecture 0; claim 0. `next`: `specifier`, then `tdd-apply`.

The D3 report is in git history at `d608bbd`; this file now covers D4.

## CRITICAL

**C1. `grid_lines` drops a line that lies exactly on the view edge.** class: `implementation`. Ref: `crates/gafira-layout/src/grid.rs:51-52`, FD-07 (a), row "Vista de `grid_lines`".
`axis_lines` bounds `k` with `(lo / spacing).ceil()` and `(hi / spacing).floor()`. That quotient rounds, so it can land just past the integer, and the line at `k` is never tried. The edge is inclusive: the row's own example (641 lines from 0 to 3840) counts it, and the function's doc says `lo..=hi`.

| Input | Expected | Got | Why |
|---|---|---|---|
| step 7 px, 3 subdivisions, zoom 4, view 0 to 35 | major line at 35 (`5 × step`) | missing | `35 / (7/3)` is `14.999999999999998`, floor 14 |
| step 1 mm, 150 dpi, zoom 1, view ending on line `k = 13` (153.5433 px, spacing 11.81 px) | that line | missing | quotient `12.999999999999998`, with no subdivisions at all |

Measured: integer px steps 1 to 500, whole-multiple views: 9 329 major lines lost over 387 235 views. Steps in mm: 3 980 of 76 360 views lost a line. A view edge exactly on a line: 21 798 of 152 721 tries lost it. Interior lines are right: 300 000 random views matched an independent scan with 0 differences.
The impact grows in D5: FD-07 snaps to "la línea mostrada más cercana", so a lost line is also a lost snap target.
Why the tests missed it: `fd_07_a` (`tests/grid.rs:118`) has `prop_assume!((hi / step_px).fract() > 1e-6 ...)`, which throws away exact multiples, and its oracle at line 120 is `floor(hi / step) + 1`, the same formula as the code.
Fix hint: widen the loop bounds (`floor` for `first`, `ceil` for `last`) and let the existing `contains` filter decide. On the export that gives 0 lost lines in all three measurements and the whole crate suite stays green. It needs a test that keeps exact multiples.

**C2. The exact rules are not proved; four hand mutants survive.** class: `implementation`. Ref: `crates/gafira-layout/tests/{grid,guides_generate}.rs`, FD-04, rows "Lado de página de `generate`", "Vista de `grid_lines`".
The code is right here: probes P3 and P7 use `next_up` and `next_down` and pass. The tests stand far enough from each limit that a tolerance goes unseen.

| Mutant | Spec | Closest test | Result |
|---|---|---|---|
| V1 `cell < 1.0 - 1e-9` | `c ≥ 1 px` | 9.999 px, so `c = 0.9995` | survived |
| V2 dedup within `1e-6` | "misma posición exacta", gaps can be tiny | property uses `gap >= 0.5` | survived |
| V4 view limit `+ 1e-6` on both axes | "`f64` exacto y sin tolerancia" | `..._with_exact_comparison` stops at +0.01 | survived |
| V5 page side `+ 1e-6` | `≤ MAX_PX`, "por encima, `Err`" | 16 384.01 | survived |

All of this is critical code (size limits, spec rules with an invariant). Tests need `next_up` and `next_down` at each limit, and a pair of guides less than `1e-6` apart that stays two guides.

**C3. No test places a page away from the origin; mutant V3 survives.** class: `implementation`. Ref: `crates/gafira-layout/src/guides.rs:114-115`, FD-03 ("se mide desde la esquina superior izquierda de la página").
`generate` takes a `Rect` and every test uses `x0 = y0 = 0`, apart from overflow cases. V3 adds `page.x0` and `page.y0` to every position and no test fails. The code is right (P12), so one offset-page case closes it.

## WARNING

**W1. Two equivalence claims in the notes are false.** class: `implementation`. Ref: `apply-progress.md` D4 "Equivalent", `crates/gafira-layout/src/guides.rs:98`.
- `ceil`/`floor` swapped: not equivalent. It is the fix for C1, see the table below.
- Raw `finite(gap)` and `finite(margin)` removed: not equivalent. `Length::convert` checks the DPI first, so `generate` with a NaN gap and `dpi = 0` gives `OutOfRange { dpi }` where every other function of the crate gives `NonFinite` for the number. `bleed_rect` and `margin_rect` keep those raw checks, and D3 added ordering tests for exactly this. ARQ-110's table has one bad input at a time, so this is an inconsistency and not a broken criterion. The fix is two lines and one test.

**W2. The FD-07 (c) tests do not reach the worst case.** class: `implementation`. Ref: `tests/grid.rs:147`, `tests/grid.rs:163`.
Judged per the request, because this criterion passed vacuously at the stub. On their own the `fd_07_c_*` tests failed 0 of 5 runs with the `2^k` fallback removed (V7), and 2 of 5 with the 6 px minor check removed (V6). Random steps are mostly large and the fixed cases use zoom 32 and zoom 1, so small steps at low zoom are never tried. The behaviour is protected, because `fd_07_d`, `fd_07_minor_*` and `fd_07_fallback_*` kill both mutants. The bound itself holds: P7 swept 3 201 zooms with dense steps, maximum 1 002 lines. Add step 1 px at zoom 0.05.

**W3. The exact view limit rejects the natural 4K view.** class: `ambiguity`. Ref: `02` row "Vista de `grid_lines`", `tests/grid.rs:41-50`, FD-07 (c).
This is the answer on the `next_down` trim. A UI computes `view = screen / zoom`, and that rounds. With 3840×2160, over 31 951 zooms from 0.05 to 32 in steps of 0.001: `Err(view)` at 3 682 zooms (11.5 %) with the view at the origin, and at 13 245 (41.5 %) with a pan offset. The round zoom 0.9 is one: `3840 / 0.9 * 0.9` is `3840.0000000000005`. The code follows the row as written, so this is not an implementation defect, and it is not the owner-accepted case: the screen here is exactly 3840×2160, not larger. On that screen the grid would vanish at those zooms. The trim in `screen()` is needed for the tests to pass and hides this. The cap `3800 / zoom` in `fd_07_a` hides nothing further about the bound, since `fd_07_c` runs up to the limit. Owner or specifier decision: keep it exact and tell the caller to trim, or define the limit another way.

**W4. API points the spec does not state.** class: `ambiguity`. Ref: FD-03, FD-04, `02` §4.17.2, `crates/gafira-layout/src/lib.rs:10-13`.
- FD-04 names `gafira-layout::guides::generate`, and FD-07 `gafira-layout::grid`; both modules are private, and the path is `gafira_layout::generate`. Either the modules or the spec should change.
- `dpi ≤ 0` is `Err` in `place_guide`, `generate` and `grid_lines`, but the DPI row covers only `bleed_rect` and `margin_rect`, and "Rangos" says an engine checks only ranges with a row. The check comes from `Length::convert` and is sensible; the row is missing.
- Lines at negative positions, pinned by `fd_07_lines_cover_a_view_that_starts_off_the_page`, while the row calls `view` "una región de la página". The `major` flag on them is unproved: V9 (`k >= 0 && ...`) survives.
- `generate` always returns px whatever unit `gap` and `margin` use. FD-03 keeps "la unidad en que se escribió" for typed guides and says nothing on generated ones.
- `place_guide` and its tuple return are invented shape, consistent with FD-03(a), with no conflict. `GuideSpec`, the added `dpi` (FD-04 states it as an input), `grid_lines` (named by the row), `GridLine`/`GridLines` and `pub(crate) convert` raise no objection.

**W5. Size.** class: `implementation`. Ref: AGENTS.md Delivery rules. 1 057 insertions against `develop`, ~700 ceiling, forecast 340 (475 at x1.4): source 269, tests 747, notes 39, spec 2. The ceiling is passed by the tests alone. Redundant test lines are few, about 25: `fd_03_a_typed_value_survives_the_extreme_dpis` (11, the property covers it) and the zoom and view NaN cases in `fd_07_view_is_checked_after_...` (about 8). Trimming will not reach the ceiling, and the fixes above add lines. Owner decides; not a blocker on its own.

## SUGGESTION

**S1.** Two row examples are tested in another form: 641 + 361 uses step 12 with 2 subdivisions, not step 6 with 1, and the zoom 33 case uses 100 000 px, not 1000×1000. Both pass on P7. Using the row's literal values costs no lines.

**S2.** Removing the `major_on_screen <= 0.0` guard (V10) survives. It is reachable only with a subnormal `dpi`, there is `Err` either way and only the field kind changes, and no spec rule covers it. Not treated as a critical survivor.

**S3.** The D4 notes are stale: "Status: partial", "one open question" and 879 inserted lines predate round 2, and round 2 has no gates line.

## Independent checks asked for

**Strict TDD.** Credible, not provable from history. Each feature commit holds tests and code together, so git cannot show the order. Reproduced on an export: `f5a11e0` tests on `33ab1c5` source fail exactly the two named tests, and the property passes as noted; `5333284` tests on `f5a11e0` source fail `fd_07_view_screen_size_...` alone, as noted. With the notes' stub on all three functions, every D4 test the notes list fails, and the only D4 tests that pass are `fd_07_c_at_most_2000_...` and `arq_110_finite_input_gives_finite_output`, the two the notes declare vacuous. The notes are honest here. The weakness is elsewhere: see C1 on the `prop_assume`.

**Vacuous at the stub.** `arq_110_finite_input_gives_finite_output` is proved: V8 is killed by it. `fd_07_c_at_most_2000_...` is only partly proved: see W2.

**General rules beyond the examples.** All correct in the code unless listed; probes P2 (200 000 cases against an independently written reference), P3, P5, P7, P9.

| Rule | Code | Tests |
|---|---|---|
| `2n` per axis | ok | ok |
| Dedup within an axis, not across | ok | across ok; exactness: C2 |
| Order, vertical first | ok | ok |
| No rounding | ok | ok |
| `c ≥ 1` | ok | C2 |
| 25 % and 6 px | ok | ok |
| `2^k` fallback | ok | ok |
| Bound of lines | ok, 1 002 at most | W2 |
| View and page limits | ok | C2 |
| Lines at the view edge | **C1** | **C1** |

**Equivalent mutants in the notes.**

| Claim | Result |
|---|---|
| `ceil`/`floor` swapped | **False**, W1 |
| Raw `finite(gap)` / `finite(margin)` | **False**, W1 |
| `(count - 1) * gap` finite check | True: `usable - inf` fails on `cell` with the same field |
| `major < 6` to `<= 6` | True: the fallback at multiple 1 gives the same lines |
| Fallback multiple from 2 | True: that branch never fits at 1 |
| (15) accumulated positions | Accepted: bytes differ, within the ±0.01 tolerance |

## Hand mutants

Run on a scratch export, all reverted. 12 run, 4 killed.

| ID | Mutant | Result |
|---|---|---|
| Spot-check | item (5) `c >= 1` to `c > 1` | killed: `fd_04_d_...`, `fd_04_page_side_limit_...` |
| V1, V2, V4, V5 | tolerances | survived, C2 |
| V3 | positions offset by the page origin | survived, C3 |
| V6 | minor shown with no 6 px check | killed: `fd_07_d_...`, `fd_07_minor_...` |
| V7 | fallback removed | killed: `fd_07_d_...`, `fd_07_fallback_...`, `fd_07_minor_...` |
| V8 | non-finite guide | killed: `arq_110_finite_input_...`, 2 more |
| V9 | no major flag on negative `k` | survived, W4 (spec silent) |
| V10 | zero-step guard removed | survived, S2 |
| V11 | `ceil`/`floor` swapped | survived, and it fixes C1 |

## Acceptance map

14 criteria: 8 covered, 4 partial, 1 weak, 1 failing.

| Criterion | State | Proof |
|---|---|---|
| FD-03 (a) | covered | `fd_03_a_*`; P1, every DPI 1 to 10 000, worst round trip 7.3e-12 |
| FD-03 (d) | covered | `fd_03_d_*`; `Page.guides` itself: D2 |
| FD-04 (a) | covered | `fd_04_a_twelve_columns_in_1080` |
| FD-04 (d) | partial | `fd_04_d_...`; C2 V1 |
| FD-04 (e) | covered | `fd_04_e_three_by_three_in_1080` |
| FD-04 (f) | partial | `fd_04_f_*`; C2 V2 |
| FD-07 (a) | **failing** | C1 |
| FD-07 (c) | weak | `fd_07_c_*`; W2 |
| FD-07 (d) | covered | `fd_07_d_...` |
| FD-07 (e) | covered | `fd_07_e_...` |
| Row "Lado de página" | partial | `fd_04_page_side_*`, `fd_04_every_ok_...`; C2 V5 |
| Row "Vista" | partial | `fd_07_view_*`; C2 V4 |
| ARQ-110 | covered | `arq_110_*`, `fd_03_d_*`, P8, P10 (400 000 raw-bit inputs: no panic, no non-finite output) |
| ARQ-111 | covered | `arq_111_*`, one per function, bytes compared; P9 |

Slice card IDs left out on purpose: FD-03 (c) and FD-06 are D5; FD-05, FD-09 D3; FD-11 D6; ARQ-114 D7.

## Gates and integrity

| Check | Result |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0; 986 passed, 0 failed |
| `cargo test -p gafira-layout --locked --release` | 0 |
| `cargo deny check` | 0 |
| CI policy: format, ARQ-07, forbid unsafe, V5, V12, V13, V14 | all 0, run from `ci.yml` on a clean clone |

Slice card: commit `f08d005` touches only `apply-progress.md` and precedes the first implementation commit; nothing above the D4 section changed. `33ab1c5` touches only `docs/specs/02-arquitectura-software.md`. No tracked file deleted, no `*.proptest-regressions` added or changed (9 on both sides), `ci.yml` unchanged, no absolute path added, no attribution lines. The commits of this delivery changed no behaviour outside `gafira-layout`.

## Notes for the record

Owner-known and accepted, not findings: the per-axis view limit with no orientation swap; a view above 3840×2160 gives `Err` (rotated or 5K screens); extreme view positions above 2^52 px leave the 2 000 bound unproved.

For D5, from the specifier, not a D4 finding: FD-06 snap cannot call `grid_lines` over a whole page at high zoom. C1 also matters there.

Round 1 next: specifier, tdd-apply

next: tdd-apply
