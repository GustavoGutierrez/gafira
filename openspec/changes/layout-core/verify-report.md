# Verify report: layout-core D3

Change `layout-core`, delivery D3, round 1, depth V2, mode full, tier T4 (`mutation: archive`, so no `mutants` report and no mutation finding).

Commit verified: `f2dafd1`, the tip of `feat/layout-core-d3`. Verdict: **FAIL**. Counts: 4 CRITICAL, 8 WARNING, 2 SUGGESTION. Per class: implementation 8 (4 CRITICAL, 4 WARNING); ambiguity 4 (WARNING); architecture 0; claim 0.

The D2 report is in git history at `1c68734`; this file now covers D3.

The request named `a13ffde`. That commit was amended into `f2dafd1` five seconds later. `git diff a13ffde f2dafd1` shows two changes: the removal of `crates/gafira-layout/tests/rects.proptest-regressions` and one reworded line in the notes. Source and tests are identical, so every result here applies to both, except the seed-file check.

## CRITICAL

**C1. `node_bounds` aborts the process on a document that loads and validates.** class: `implementation`. Ref: `crates/gafira-layout/src/bounds.rs:18` (`walk`), ARQ-110 ("No hay `panic`").
`walk` recurses once per level, and nothing limits nesting depth. `crates/gafira-doc/src/validate.rs` `a_deep_chain_validates_without_overflow` accepts depth 100 000 and states the reason: a recursive walk overflows the 2 MiB test-thread stack. Measured on a 2 MiB test thread:

| Build | Depth | Result |
|---|---|---|
| debug | 2 500 | ok |
| debug | 3 000 and above | `has overflowed its stack`, SIGABRT |
| release | 20 000, 100 000 | `has overflowed its stack`, SIGABRT |

End to end, in release: a crafted 20 000-level page goes through `Document::from_json` and `validate()` with `Ok`, then `node_bounds` aborts. No `Err` is returned and it cannot be caught, so opening a hostile or damaged file with a bounds query takes the app down. The fix must also deal with `path.contains` (`bounds.rs:19`), which is O(depth) per level, so O(depth²) on such a chain.

**C2. ARQ-111 is not proved.** class: `implementation`. Ref: `crates/gafira-layout/tests/rects.rs:281`, `tests/bounds.rs:177`, ARQ-111.
The criterion says each crate runs every operation of its first slice twice and compares the bytes.

| Operation | Run twice | Compared by |
|---|---|---|
| `node_bounds` | yes, one input | `to_bits`, correct |
| `safe_rect`, `bleed_rect`, `overflow` | yes, in the property | `prop_assert_eq`, so `==`, not bytes |
| `margin_rect` | **never** | none |

Two mutants prove the gap. M1, `margin_rect` returning a different `y1` on every second call, survives. M2, `overflow` flipping the sign of a zero excess on every second call, survives, because `-0.0 == 0.0`. The tests also passed at the implementer's stub, which the notes state; G3 therefore has no RED for this criterion.

The code itself looks deterministic: it has no state, threads, hashing or entropy. The missing part is the proof the criterion demands.

**C3. An intermediate infinity is hidden and the call returns `Ok`.** class: `implementation`. Ref: `crates/gafira-layout/src/rects.rs:124`, `rects.rs:80`, ARQ-110, 02 §4.17.2 row "Suma que desborda".
ARQ-110 says a non-finite intermediate result gives `Err`. Two paths break it:

- `overflow`: `excess` clamps `-inf` to 0 before the check at `rects.rs:135`. `overflow(Rect(MAX, 0, MAX, 1), Rect(-MAX, 0, 0, 1))` computes `-MAX - MAX = -inf` and returns `Ok`.
- `margin_rect`: `page.width()` overflows to `inf` and the comparison at `rects.rs:80` swallows it. `margin_rect(Rect(-MAX, 0, MAX, 100), margins 10, 96)` returns `Ok`. `safe_rect` on the same page returns `Err(NonFinite)`, so the two functions disagree.

These need coordinates above about 9e307, and the returned numbers are finite. They still break an explicit acceptance row, so the requirement is marked failing rather than covered. A related case: `left + right` overflowing gives `NoArea` instead of a non-finite error. It is an `Err`, so it is noted only.

**C4. The ARQ-110 tests are weaker than the criterion.** class: `implementation`. Ref: `crates/gafira-layout/tests/rects.rs:264`, `rects.rs:293`, ARQ-110.
The field table is complete and my independent table agrees with it. The rest of the criterion has gaps. Three hand mutants of the "Suma que desborda" rule survive:

| Mutant | Rule | Result |
|---|---|---|
| M3a | `bleed_rect` result check removed | survives |
| M3b | `safe_rect` result check removed | survives |
| M3c | `overflow` result check kept only for `left` | survives |

- `rects.rs:264` says "A sum that overflows is an error", but `bleed_rect(huge, px(f64::MAX), 96.0)` fails on the bleed range and never reaches the sum. A real sum case exists: `bleed_rect(Rect(0, 0, MAX, MAX), 10 mm, 1e306)`. It returns `Err(NonFinite)` on the real code; with M3a it reaches the unchecked sum.
- The property omits `margin_rect` and `node_bounds`, and checks 8 of the 12 values of the operations it does run. ARQ-110 asks that every numeric output be finite.

## WARNING

**W1. Strict TDD order was not followed.** class: `implementation`. Ref: `AGENTS.md` "Strict TDD", `tdd-apply.md` step 2, G3, notes deviation (a).
The implementation was written first and RED was produced later by stubbing the bodies, so this is a stated departure from "write the test, run it, see it fail". G3 is met in form for every criterion except ARQ-111: failing assertions are recorded, not compile errors. Stub RED shows a test can fail; it does not show the test drove the code, and it says nothing about branches the stub hides. C2 to C4 are that gap: all of them are spec rows that a test written first from the row would have exercised. It is not repeatable after the fact. The fix rounds should run real RED.

**W2. One commit carries the whole delivery.** class: `implementation`. Ref: `f2dafd1`, `AGENTS.md` "One commit per reviewable work unit", ADR 0007.
ADR 0007 requires only the row, the `clippy.toml` and the `Cargo.toml` entries in one commit, because V5 fails otherwise. It does not require source, tests and notes there. A natural split passes each gate alone: crate skeleton with error type, row and `clippy.toml`; `bounds` with its tests; `rects` with its tests; notes (D1 and D2 kept notes in their own commit). 871 lines in one commit is also well above the ~400 line trigger of the `chained-pr` skill.

**W3. Size is above the ceiling, with no owner OK recorded.** class: `implementation`. Ref: `AGENTS.md` "Delivery rules". Measured below. The owner decides; alone it is not a blocker. The fixes for C1 to C4 will add lines.

**W4. Rounding can return a rectangle with no area.** class: `implementation`. Ref: `crates/gafira-layout/src/rects.rs:80`, FD-05 "Validación".
`margin_rect` checks the sum before subtracting. With `x0 = 1e16`, width 4, left margin 3, it returns `Ok` with `x0 == x1`. `safe_rect` does the same at 49 % on that page. It needs coordinates above 2^53, where a px has no resolution. No test covers it.

**W5. A non-empty group with a NaN in its own placement returns `Ok`.** class: `ambiguity`. Ref: `bounds.rs`, 02 §4.17.2 "NaN o ±∞ en cualquier entrada", ADR 0009 rule 3.
02 says any input fails the whole call; ADR 0009 says nothing reads that box. Both readings are defensible, and `validate()` rejects the value on load. The specifier should say which holds.

**W6. Skew at or past 90 degrees has no stated behaviour in `node_bounds`.** class: `ambiguity`. Ref: `bounds.rs`, `crates/gafira-doc/src/validate.rs:187`, ARQ-110.
Answer to the question raised: `skew = 90` gives a finite box of about ±8e17 for a 100 px box. That number is rounding noise from `tan(90°)`, and 270 and 1000 are also accepted. It breaks no rule: `validate()` and `SetProperty` both refuse the value, ARQ-110 covers only non-finite input, and plain data does not validate itself. No test pins it. 02 does not say whether engines re-check the range of finite values, so this is a spec gap and not a code defect.

**W7. A zero-size box with zero margins is `NoArea`.** class: `ambiguity`. Ref: `rects.rs:80`, 02 §4.17.2 "Caja de tamaño 0: válida" (cites FD-05), FD-05 "Validación".
`margin_rect(Rect(0, 0, 0, 100), margins 0, 96)` gives `Err(NoArea)`, while `safe_rect` and `bleed_rect` return `Ok` on the same box. The margins remove nothing, so they do not "leave" the box without area, but the code's reading is also tenable.

**W8. 10 mm in px is refused at some DPIs.** class: `ambiguity`, `follow-up`. Ref: FD-05(d), 02 §4.17.2.
This is D2's W1, by owner decision a follow-up. `bleed_rect` has the same behaviour: refused at 550 of 10 000 integer DPIs, first at 13, 21, 26. It uses the same `Length::convert` comparison as `check_bleed`.

## SUGGESTION

**S1.** Replace the six multi-line `add(...)` calls in `tests/bounds.rs` with a `leaf(x, y, w, h)` helper. It saves about 36 lines and changes no coverage.

**S2.** No code holds the 5 % default of D-38: `safe_rect` takes the percentage as an input, as S19 of `design.md` says, and the only trace is the literal `5.0` in tests. Name the owner of the default when the first caller arrives.

## Answers to the questions raised

| Question | Answer |
|---|---|
| Strict TDD | Violated in order, met in form except for ARQ-111 (W1). Test strength: the spec examples and field tables are strong (all acceptance examples agree with my probes, spot-check killed). The rules beyond the examples are weak (5 of 5 independent mutants survived). |
| ARQ-111 proved? | No (C2). |
| `safe_rect` 5.0, `[0, 50)` | Follows 04 and 02 §4.17.2, which write the range in percent. Not invented. |
| `margin_rect(&Margins, dpi)` | Follows 04: "lo reduce por cada margen", "el DPI ... es una entrada", `Page.margins: Margins`. The name comes from `design.md`, not from 02 or 04. It ignores `linked`, which 04 describes as an editing mode of four stored sides. Not invented. |
| `Overflow::exceeds()` | States the rule of 04 ("si algún exceso es `> 0`") and 02 ("sin tolerancia"). An accessor, not new behaviour. |
| Other interface additions | Negative margin as `OutOfRange` matches 02:215. `dpi <= 0` as `OutOfRange` is `Length::convert`'s rule. `UnknownNode` and `Cycle` are required by "no panic". None invented. |
| 90 degree skew | See W6. Not a defect. |
| Single commit, ADR 0007 | ADR 0007 is respected; work-unit commits are not (W2). |
| V5 and V12 | Run on a clean clone: both exit 0, along with every other policy step. |
| Seed files, deletions | None committed relative to `develop`; 0 deleted files in `git diff --diff-filter=D develop f2dafd1`. `a13ffde` did carry a seed file, removed by the amend. The notes' explanation, a mutant run, holds: both seeds pass on the real code, and mutant item 2 (`dx` from the height) reproduces the same shape, `w = 0.0`, `h` large, `mm = 0`, `dpi = 1`. No finding. |
| Size | See "Size". |

## Gates

Run at `f2dafd1` on a fresh clone in the scratchpad, `CARGO_INCREMENTAL=0`, own target dir. The repo tree stayed clean throughout.

| Gate | Exit | Note |
|---|---|---|
| `cargo fmt --check` | 0 | |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | |
| `cargo test --workspace --locked` | 0 | 944 passed, 0 failed, 0 ignored; the implementer's figure matches |
| CLI has no UI dependencies (ARQ-07) | 0 | |
| Domain crates forbid unsafe code | 0 | |
| Layer table (V5) | 0 | Row matches ADR 0007 |
| `clippy.toml` copies (V12) | 0 | |
| `EditToken` (V13) | 0 | |
| Core command property tests (V14) | 0 | |
| `cargo deny check` | 0 | advisories, bans, licenses, sources ok |

Each policy step was extracted from `.github/workflows/ci.yml` and run as written. Linux only; the Windows and macOS matrix jobs did not run here.

## Artifact integrity (step 3c)

- `f08d005 docs(layout-core): slice card` touches only `apply-progress.md` and precedes all crate commits.
- Slice card lines 1 to 9 are byte-identical at `f08d005` and at HEAD. D3 adds 31 lines to the file and removes none.
- No `slice card amend` commit exists for this change.
- `git grep` finds tests naming FD-05 (b, d, e, f), FD-09 (d), D-28, ADR 0009, ARQ-110 and ARQ-111. D-38 has no named test, but `fd_05_b` and `fd_05_e` exercise 5 %. Left out of D3 by design: every other ID of the card.

## Criteria

| # | Criterion | State | Proof |
|---|---|---|---|
| 1 | FD-05(b) | covered | `fd_05_b_overflow_at_the_safe_edge_boundary`, probe `fd05b` |
| 2 | FD-05(d) | covered | `fd_05_d_*`, probe `fd05d` |
| 3 | FD-05(e) | covered | `fd_05_e_*`, probe `fd05e` |
| 4 | FD-05(f) | covered | `fd_05_f_*`, probe `fd05f`; see W4, W7 |
| 5 | FD-09(d) | covered | `fd_05_f_fd_09_d_*`, probe `fd09d` |
| 6 | D-28 | covered | `d_28_*`, probe `d28_rotation_skew_flip` (45, 30 degrees, skew, flip, size 0) |
| 7 | D-38 | covered | 5 % as input; see S2 |
| 8 | ADR 0009 rules 1 to 3 | covered | `adr_0009_*`, probe `adr9_group` |
| 9 | ARQ-110 field table | covered | `arq_110_every_numeric_input_*`, `arq_110_bad_placements_*`, probe `arq110_table` |
| 10 | ARQ-110 intermediate non-finite, sum row | **failing** | C3, C4 |
| 11 | ARQ-110 property, no panic | **failing** | C1, C4 |
| 12 | ARQ-111 | **missing** | C2 |
| 13 | 02 §2.2, ADR 0007 row, `clippy.toml`, `forbid` | covered | V5, V12, forbid step |
| 14 | §4.17.2 inverted box | covered | `arq_110_inverted_boxes_are_errors` |
| 15 | §4.17.2 size 0 box | covered | `fd_05_f_fd_09_d_*`; see W7 |
| 16 | §4.17.2 warning `> 0`, no tolerance | covered | `fd_05_b_*` |
| 17 | §4.17.2 percent range | covered | `fd_05_e_*` |
| 18 | §4.17.2 bleed range | covered | `fd_05_d_bleed_range_*`; see W8 |

15 of 18 covered.

## Hand mutants

Applied one at a time on the scratch clone with `cargo test -p gafira-layout --locked`, each reverted after its run. `mutation: archive` does not replace T4 at close.

| # | Source | Mutant | Result |
|---|---|---|---|
| S0 | Spot-check, item 4 | Width check `>=` to `>` | Killed by `fd_05_f_margins_that_reach_the_size_leave_no_area` |
| M1 | Independent, ARQ-111 | `margin_rect` differs on every second call | **Survived** |
| M2 | Independent, ARQ-111 bytes | Sign of a zero excess flips on every second call | **Survived** |
| M3a | Independent, sum row | `bleed_rect` result check removed | **Survived** |
| M3b | Independent, sum row | `safe_rect` result check removed | **Survived** |
| M3c | Independent, sum row | `overflow` result check on `left` only | **Survived** |

M3 is one rule tried at three sites, so the independent list is three rules. Survivors fall under "any spec rule with an invariant" (`AGENTS.md`), hence C2 and C4.

## Probes (step 1c)

Derived from the acceptance sources before reading `tests/`, and run on an export with those tests removed: 15 probe tests, plus deep-chain probes. Passed: the literal examples of FD-05(b, d, e, f) and FD-09(d), D-28 with analytic rotation and skew, ADR 0009 with a stale group box, cycle, unknown child and shared child, the full non-finite table, `dpi` 0 and negative in `bleed_rect` and `margin_rect`, 20 000 random cases per property across magnitudes up to 1e308 with no panic, no non-finite output and no `node_bounds` `Ok` hiding a non-finite corner. Failed: C1 and C3. Printed for review: W4 to W8.

Comparison with the implementer's tests: the literal examples and the field table agree. The weak points are C2 and C4, and neither nesting depth nor huge coordinates in `margin_rect` has a test.

## Size

`git diff --shortstat develop f2dafd1`: 12 files, 871 insertions, 0 deletions.

| Group | Lines |
|---|---|
| `src/` | 277 |
| `tests/` | 488 |
| Crate `Cargo.toml` | 17 |
| `clippy.toml` byte copy | 47 |
| `Cargo.lock` | 9 |
| Root `Cargo.toml`, `ci.yml` | 2 |
| `apply-progress.md` | 31 |

- Without the copy, lock and notes: 784 authored lines. Either way it is above ~700.
- Forecast 420 (x1.4 figure): measured 2.07 times over, or 1.87 times on the 784 figure.
- Tests to source: 1.76 to 1.
- Redundant test lines: about 10 of 488 (2 %), judged by reading, not by deleting and re-running. The clearest case is `bounds.rs:103-109`, which repeats rotation inside a nesting test (about 5 lines), plus the overlap of the 45 and 90 degree cases at `bounds.rs:54-70` (about 5 lines).
- Avoidable without losing coverage: about 36 more lines of `rustfmt` expansion (S1). The bulk of the test code is the ARQ-110 table, which the spec requires.

## ODD lite pilot (ADR 0011, phase C)

- New planning artifacts for D3: none. No proposal, delta spec, tasks file or new card; `design.md` and the Slice card are untouched. The only planning-side change is 31 lines of notes in `apply-progress.md`. `.odd/state.json` is unchanged on this branch, so the D3 entry still reads `pending`.
- Context the delivery needed, as files: Slice card (9 lines), FD-05 and FD-09 rows (about 20 lines), ARQ-110, ARQ-111 and §4.17.2 (about 30 lines), §2.2 (about 30 lines), D-28 and D-38 (about 15 lines), `design.md` (249 lines, of which about 40 apply to D3), ADRs 0007 and 0009 (88 lines). This is what the request listed as the acceptance source. How much of it the implementer loaded, and token counts, were not measured.
- D3 notes: 30 lines, against 31 for D1 and 34 for D2 before their fix rounds.
- Verdict reproducibility: the gates and each probe above can be re-run from the commit. Phase D needs D3 to close PASS, which has not happened; rounds so far: 1.
- Not checked by me: whether the resume checks ran first, which is a coordinator fact.

## Result

Verdict: **FAIL**. 4 CRITICAL, 8 WARNING, 2 SUGGESTION. implementation 8, ambiguity 4, architecture 0, claim 0.

W5 to W7 decide what `node_bounds` and `margin_rect` must return, so they go first; C1 to C4 and W2 to W4 follow. No check was blocked.

next: specifier, tdd-apply
