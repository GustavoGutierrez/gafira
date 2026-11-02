# Verify report: engine-contracts, delivery D2b, round 1

- Commit verified: bf66b2a (branch `feat/engine-contracts-d2b`, base 3cebb86). Depth V2, mode full, `mutation: archive`, no `mutants` report (not a finding).
- Verdict: **FAIL**. CRITICAL 3, WARNING 2, SUGGESTION 1. Per class: implementation 4, ambiguity 1.
- Finding ids C1 to C3 are this report's. "Decision C3", "decision C13" name `design.md`.

## Gates (each exit code read alone, on a clean export of HEAD)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (1265 tests, 98 result lines all ok) |
| `cargo deny check` | 0 |
| CI policy, 7 `run` steps of job `policy`, each alone (format, ARQ-07, forbid unsafe, V5, V12, V13, V14) | 0 each |

Not run here: the Windows and macOS legs of the CI matrix. Linux x86_64 only.

## Findings

### CRITICAL

**C1. class: implementation.** Hand mutant A survives on critical code. `next_generation` as a separate `load` and `store` instead of `fetch_add` (the token rule in `design.md`) passes all 19 tests of `contract_cancel.rs` and `contract_checkpoint.rs`. Two racing calls then give two live tokens. Ref: `crates/gafira-doc/tests/contract_cancel.rs`. The only cross-thread test, `arq_112_cancel_is_seen_across_threads`, joins before it reads, so it proves no atomicity. Killer: 8 threads on a `Barrier`, 3000 rounds, assert one live token. That probe failed 3 of 3 runs on the mutant in debug, within 0.03 s, and passes on HEAD.

**C2. class: implementation.** Hand mutant B survives on critical code. `cancel` as "load, compare with `issued`, store `issued + 1`" instead of `fetch_max` passes the same 19 tests. It is sequentially identical, so only a race shows it, and it is not equivalent: loom gives a stale cancel that overwrites two `next_generation` calls, and the newest token reads as cancelled. Ref: `crates/gafira-doc/tests/contract_cancel.rs`. Killer without a new dependency: a handshake stress (`vprobe_b.rs` in the probe export), a worker cancelling while the main thread opens two generations: 44 hits in 3 000 000 rounds on the mutant, 0 on HEAD, 4.6 s in release. It may need more rounds in debug. Adding `loom` as a dev-dependency is an owner decision.

**C3. class: implementation.** Real defect in the double, shown by a probe. `CancelAt::report` (`crates/gafira-testkit/src/progress.rs:106`) pushes under one lock and reads the length under another, so two parallel reports can step over `k` and the cancel never fires. Decision C3 makes `Progress` `Sync` for parallel workers. Measured, 1000 checks each:

| Operation | `pass` | `NotCancelled` | `ReportsAfterCancel` |
|---|---|---|---|
| 2 workers, unserialized, equal fractions | 617 | **383** | 0 |
| 4 workers, unserialized | 360 | 314 | 326 |
| 8 workers, unserialized | 328 | 315 | 357 |
| 2 workers, serialized `checkpoint` | 1000 | 0 | 0 |
| 8 workers, serialized `checkpoint` | 1000 | 0 | 0 |

The 2-worker row meets ARQ-112 (at most 1 report after `k`, non-decreasing) and still gets a false `NotCancelled`. Reach is narrow, and the failure is loud: an operation that serializes its reports never hits it, and unserialized reports with distinct fractions would flake on `Decreasing` anyway. No probe gave a false pass. Severity follows the rule that a probe showing a real code defect is CRITICAL. The fix is to push and read the length under one lock.

### WARNING

**W1. class: ambiguity.** `design.md` gives two answers for `Ok(false)` with fewer than `k` reports and a log that does not end at `1.0`. The rule "the return value is judged before its log" and the row "`Ok(false)` after the cancel" give `OutputAfterCancel`; the row "Cancelled run" says "fewer than `k` reports means the cancel never fired: `NotCancelled { k }`", with no condition on the return value. Observed: empty log at `k = 1` gives `OutputAfterCancel { k: 1 }`; 3 reports at `k = 5` gives `OutputAfterCancel { k: 5 }`; short and ending at `1.0` gives `NotCancelled { k: 1 }` (both rules agree). Ref: `progress.rs:155`, recorded by the implementer under "Deviations and judgments". Both outcomes fail the operation, so no bad operation passes; only the diagnosis differs. The code now names a cancel that never happened, against its own variant doc ("stopped after a cancel at `k`"). Recommendation: `NotCancelled`, with a condition added to the rule. The row was written by `architect` (de57b41), so the coordinator may prefer that stage.

**W2. class: implementation.** Four independent mutants of the checker survive `crates/gafira-testkit/tests/progress.rs`. The checker is a test helper (`design.md`, "Critical code"), so these are not CRITICAL.

| Mutant | Rule left unpinned |
|---|---|
| C: fewer-than-`k` check before the return value | The W1 case; pin it once W1 is settled |
| D: `ReportsAfterCancel` before the scan | "Then `ReportsAfterCancel`": no fake has 2 or more extra reports and a scan fault (my probe killed it) |
| E: fewer-than-`k` check after the scan | Row order for `Ok(true)`; also survives my probes |
| J: `fractions.contains(&1.0)` for "ends at `1.0`" | `Ok(false)` with a `1.0` in the log but not last; also survives my probes |

### SUGGESTION

**S1.** The body of dbd6997 says "18 of 20 tests"; that commit holds 21 tests. Record only.

## Owner's points

- **`>` mutant: equivalent, implementer's claim confirmed.** Applied to the source: all implementer tests and all token probes pass. The counter only rises and `issued` is always a value the counter has reached. Any handle moved between threads in safe Rust carries a happens-before edge, so a relaxed load cannot fall below `issued`. Loom confirms this on the copy of the rules (`relaxed_load_never_below_issued_after_publication`, with `!=` and `>` read side by side). A difference needs a `u64` wrap, unreachable through the API.
- **`Ordering::Relaxed`: sound for the rules as designed.** Loom: 7 models pass, covering a stale cancel against one and two generations, racing `next_generation`, racing cancels, cancel never reverting on a reader, and a cancel seen after a Release/Acquire signal. Negative control: loom fails with `cancel` as `fetch_add`. Limits: loom is run on a copy of the rules, and it does not model every relaxed behaviour; hardware stress ran on x86_64 only, no aarch64 run.
- **Concurrency on HEAD: no defect in `CancelToken`.** Model of 2 985 984 operation sequences against a reference, debug and release; 8 thread stress probes passed, plus the handshake stress. The gap is in the tests (C1, C2), not the code.
- **`Ok(false)` with fewer than `k`:** W1 and mutant C.
- **Size:** 289 source lines and 756 test lines, about 1 045 against a forecast of 270 (x1.4: 380). Noted, no finding, as the owner said.

## Criteria

| Criterion | State | Proof |
|---|---|---|
| ARQ-112 on doubles: conforming fake passes, each violation named | covered | 21 `arq_112_*` in `progress.rs`, all 8 variants; independent probes agree |
| `CancelToken` acceptance list | covered | `contract_cancel.rs`, model probe |
| `checkpoint` | covered | `contract_checkpoint.rs`, probe |
| V5, `contract` imports, no `Locale` | covered | V5 exit 0, `contract_imports.rs` includes `cancel.rs`, `git grep Locale` 0 hits in `gafira-doc` |
| Token rules (`fetch_add`, `fetch_max`, atomic) | **failing** | Code right; tests do not hold atomicity (C1, C2) |
| Decision C3 (`&self`, `Send + Sync`, `f32`, parallel workers) | **failing** | Signature right; the double races (C3) |
| C5, ADR 0012 (`Cancelled`, key, no arguments, `?` through `From`) | covered | `arq_102_cancelled_is_the_job_key_with_no_arguments`, probe |
| Decision C13 | partial | Covered when the cancel fired; W1 when it did not |
| ARQ-112 as written, work-model fraction, D-43 | covered for D2b | No clock in the D2b sources; nothing outside ARQ-112 takes a token; real operation, 1-thread sequence and reference input arrive with `gafira-segment` (`design.md`) |

Product IDs: ARQ-112 named by 39 tests, ARQ-102 by 1. Left out by design: ARQ-112 on a real operation.

Design claim checked: `i / 50` in `f32` gives 19 rises above `0.02f32`, worst 0.02000004. Confirmed.

## Artifact integrity

- Slice card (07947c5, amended 7b8c2cd) is unchanged by D2b: `git diff de57b41 HEAD` on `apply-progress.md` only adds lines.
- `design.md` changed only in de57b41, before the first implementation commit. No implementer commit touched `design.md`, `specs/02-arquitectura-software.md`, `specs/00-decisiones.md` or `docs/`.
- `.cargo/mutants.toml`: no exclusion added.

## Hand mutants

Spot-check, item 2: `cancel` as `fetch_add(1)`: killed by `arq_112_a_stale_cancel_leaves_the_new_generation_live`, as recorded.

| Mutant | Repo tests | Independent probes | Finding |
|---|---|---|---|
| A `next_generation` non-atomic | **survived** | killed | C1 |
| B `cancel` non-atomic | **survived** | killed by loom and the handshake stress; 200 000-round race probe missed it | C2 |
| C fewer-than-`k` first | **survived** | survived; it turns the literal-row probe green | W2 |
| D `ReportsAfterCancel` before the scan | **survived** | killed | W2 |
| E fewer-than-`k` after the scan | **survived** | survived | W2 |
| J `contains` for "ends at" | **survived** | survived | W2 |

Run 7 (spot-check plus 6), killed by the repo tests 1. The `>` check is apart. On HEAD every probe passes except `owner_ok_false_fewer_than_k`, which holds the literal row of W1.

## Evidence on disk

Exports `gates/` and `probe/`, probes in `probe/crates/*/tests/vprobe_*.rs`, loom crate `loomcheck/`, logs `gate-*.log`, `mutants.log`. Build folders were freed.

next: specifier (ambiguity: W1), then tdd-apply (implementation: C1, C2, C3, W2)

---

# Verify report: engine-contracts, delivery D2b, round 2 (closure)

- Commit verified: 5b0b447 (branch `feat/engine-contracts-d2b`, base 3cebb86). Depth V2, mode full, `mutation: archive`, no `mutants` report (not a finding). Tier T4 for the critical code; no T4 survivor was open, so no closure here depends on a report.
- Verdict: **PASS**. CRITICAL 0, WARNING 1 (follow-up), SUGGESTION 0. Per class: implementation 1 (follow-up). All 5 findings of round 1 closed; S1 corrected in the record.
- All four deliveries of the change (D1, D2a, D3, D2b) have now been verified. The T4 run of `mutation: archive` is still owed before close.
- Round 1 next-line note: its `next` named `specifier` for W1; the coordinator routed W1 to `architect`, which settled it in 75a066d.

## Gates (each exit code read alone, on a clean clone of HEAD)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (1272 tests, 98 result lines all ok, 0 FAILED) |
| `cargo deny check` | 0 |
| CI policy, 7 `run` steps of job `policy`, each alone | 0 each |

Not run: the Windows and macOS legs of the CI matrix. Linux x86_64 only.

## Closure

| Finding | Status | Evidence |
|---|---|---|
| C1 | **closed** | Mutant A failed 100 of 100 (whole binary 100 of 100) on 20 idle CPUs, 100 of 100 on 4 (whole binary 93 of 100). Test: `arq_112_racing_next_generation_calls_leave_exactly_one_live_token` |
| C2 | **closed**, see W3 | The exact round 1 mutant (`==`) failed 200 of 200 on 20 idle CPUs and 200 of 200 on 4 (whole binary 199 of 200). Test: `arq_112_cancelling_while_generations_advance_never_revives_an_older_token` |
| C3 | **closed** | One lock in `CancelAt::report`, as design.md now requires. Two-lock mutant failed 30 of 30 idle, on 2 CPUs and under both loads (0 of 30 on 1 CPU, expected: no parallel reports) |
| W1 | **closed** | 75a066d settles it: fewer than `k` is the first check, whatever the return value. Code and tests match; the Round 1 row is pinned for both return values |
| W2 | **closed** | Reapplied on HEAD, each failed its named test: C (short check after the return value), D (extras before the scan), E (short check after the scan), J (`contains`) |
| S1 | corrected | `apply-progress.md`, fix round section |

### The replaced C2 test: real coverage

The test drops the handshake of round 1. Three threads cancel the newest published token while the main thread advances 200 000 generations, and it asserts that no older token in a window of 4 is live.

- It holds the right property. A non-atomic `cancel` moves the counter backwards, and a backward move onto a held token is a revival, so a failure here is the same defect loom showed. It sees the revival rather than the "newest lost" symptom of round 1, which it cannot assert, because here the newest token is legitimately cancelled by the cancellers.
- It is sound. The counter only rises on correct code, so a live older token cannot appear. 0 false failures in all the runs below.
- Three variants checked, not one: `==` (the round 1 mutant), `<=` (the implementer's) and load-max-store (new, added to rule out a test tuned to one spelling; no independent budget intended). On an idle machine `==` and `<=` are killed on 20 and 4 CPUs, and load-max-store on 20 and 2 CPUs (not run on 4). `apply-progress.md` gave a count for `<=` only; `==` now has one.
- Limit: W3.

## Stress on HEAD: no flakiness

0 failures in 2 100 runs, 0 timeouts. Debug test binaries built from a clean export; machine has 20 CPUs.

| Condition | Three tests, 100 runs each | Whole `contract_cancel` binary |
|---|---|---|
| Idle, 20 CPUs | 300 of 300 pass, max 0.28 s | 200 of 200 |
| 40 spinners on 20 CPUs | 300 of 300, max 3.3 s | |
| 4 CPUs with 8 spinners | 300 of 300, max 2.41 s | |
| Idle, 4 CPUs | | 200 of 200 |
| Idle, 3 CPUs | | 200 of 200 |
| Idle, 2 CPUs | 300 of 300 | |
| 1 CPU | 300 of 300 | |

The three tests: `arq_112_racing_next_generation_...`, `arq_112_cancelling_while_generations_advance_...`, `arq_112_parallel_reports_...`. Slowest single run 3.3 s under load, against the 0.2 s stated, which is a test time and not a failure.

## Mutant kill rate by condition (runs failed)

| Condition | A (C1 test) | B `==` (C2 test) | B `<=` (C2 test) | B load-max-store | C3 two locks |
|---|---|---|---|---|---|
| Idle, 20 CPUs | 100/100 | 200/200 | 30/30 | 50/50 | 30/30 |
| Idle, 4 CPUs | 100/100 | 200/200 | | | |
| Idle, 4 CPUs, whole binary | 93/100 | 199/200 | 99/100 | | |
| Idle, 3 CPUs | 97/100 | 171/200 | | | |
| Idle, 3 CPUs, whole binary | 31/100 | 145/200 | 30/100 | | |
| Idle, 2 CPUs | 30/30 | 5/50 | 3/30 | 50/50 | 30/30 |
| 1 CPU | 1/30 | 0/50 | 0/30 | 10/50 | 0/30 |
| 40 spinners on 20 CPUs | 27/30 | 16/50 | 2/30 | 50/50 | 30/30 |
| 4 CPUs with 8 spinners | 4/30 | 1/50 | 1/30 | 50/50 | 30/30 |

## Findings

### WARNING

**W3. class: implementation, follow-up.** The two atomicity tests lose most of their power on a small or busy machine. They never fail wrongly, but a regression to the non-atomic form could pass a run. Worst realistic cases from the table: `==` caught 2% of runs on 4 CPUs under load and 10% on 2 CPUs; mutant A caught 31% as the whole binary on 3 CPUs. Ref: `crates/gafira-doc/tests/contract_cancel.rs`. GitHub runners were not measured; those figures are `taskset` on this x86_64 machine, and a hosted runner may be 3 or 4 vCPUs and shared. So "hit on every run" in `apply-progress.md` holds on an idle machine of 4 or more CPUs only. A likely cause for the C2 test, from reading it and not measured: the main thread's own `fetch_add` covers a rollback of one generation before it checks, so only a cancel 2 or more generations stale shows. A deterministic proof needs `loom`, which is an owner decision. Not FAIL: a new test gap after round 1 is a follow-up, and the code at HEAD is right (`fetch_add`, `fetch_max`).

## Criteria

9 of 9 covered. The two rows that failed in round 1 are now covered: token rules (C1, C2) and decision C3 (C3). Decision C13 moves from partial to covered (W1). Product IDs: ARQ-112 and ARQ-102 named by tests; ARQ-112 on a real operation left out by design, it arrives with `gafira-segment`.

## Artifact integrity

- Slice card unchanged: `git diff bf66b2a HEAD` on `apply-progress.md` only adds lines, and 5b0b447 edits only lines added by dc263fc in the same fix round.
- `design.md` changed in D2b only in de57b41 and 75a066d, both before the fix commits and neither by an implementer commit; 75a066d is the allowed upstream change and touches only that file.
- Fix commits ff353fa and fb5dc6f touch only `crates/`. `specs/00-decisiones.md`, `specs/02-arquitectura-software.md`, `docs/` and `.cargo/` unchanged since de57b41. No exclusion added.
- Worktree clean after the run; all mutants ran on a scratch export.

## Hand mutants

Closure only, reapplied from round 1, plus `<=` from `apply-progress.md` and load-max-store as a C2 variant: 9 run, 9 killed on an idle machine. Rates in the table above.

## Evidence on disk

Under `r2/` next to this report: `logs/*.log`, `logs/*.rc`, `logs/matrix.jsonl`, `logs/matrix2.out`, `scripts/`, `bins/`. Build folders were freed.

next: archive (after the T4 run owed by `mutation: archive`). Follow-up to carry: W3.
