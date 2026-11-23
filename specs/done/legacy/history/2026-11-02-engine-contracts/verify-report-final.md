# Verify report: engine-contracts, change-level final verify

Commit a2dcf20 (`chore/engine-contracts-t4`, on `develop` 7c4951d). Mode full, depth V2, `mutation: archive`, tier T4.
Round: no round was given and no `verification-final.json` existed, so this is round 1 of the final record, with closure-round scope (`verify.md`): no independent probes, no independent hand mutants.

## Result

- Verdict: **FAIL**
- Counts: 1 CRITICAL, 6 WARNING (all follow-up), 2 SUGGESTION
- Per class: implementation 4 (1 CRITICAL, 3 WARNING), ambiguity 3 (3 WARNING). Routed: implementation. Follow-up, not routed: implementation 3, ambiguity 3
- Cause of the FAIL, the only one: the `gafira-doc` T4 timeout is not a hanging mutant. No code or test defect was found.

## Gates

Clean export of a2dcf20, own `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`. Each exit code read from its own file.

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (1 273 passed, 0 failed, 0 ignored) |
| `cargo deny check` | 0 |
| CI policy: ARQ-07, forbid unsafe, V5, V12, V13, V14 | 0, 0, 0, 0, 0, 0 (each step run alone from `ci.yml`, in the clean worktree) |

## Findings

### CRITICAL

**F-C1. The `gafira-doc` T4 timeout is explained by load, not a hang.**
- class: implementation
- ref: `crates/gafira-doc/src/validate.rs:61:29`, `replace match guard self.next_id <= max_id with false in Document::check_ids`
- `validate.rs` is critical code (validation, ids).
- T4 log: the test phase ran 37.0 s, the auto limit. Its whole output is `Blocking waiting for file lock on package cache`, twice, then `Timeout`. No test binary started, so T4 holds no result for this mutant.
- Re-run on the same source (limit 600 s): caught in 0.5 s. `validate::tests::id_rows` and `document::tests::from_parts_validates` fail on assertions. The mutant ended, so it is not hanging.
- Lock contention is measured: 135 of the `gafira-doc` mutant logs contain that line, and the five other mutants of the `validate.rs` cluster spent 13.6 to 31.6 s in the test phase against a median of 0.7 s.
- Rule: `verify.md` step 4, AGENTS "Hanging mutants" and ADR 0004 say a timeout load explains stays a finding, and step 4 says it stays CRITICAL. They carve out no exception for a timeout later caught, and `rerun-doc` is a 2-mutant run, not a T4 report. So I apply the rule as written.
- Substance: the mutant is killed by two tests. **No code or test change is owed.**
- Closure: a new whole-crate T4 on `gafira-doc` in which this mutant is caught, then a closure verify. Other cargo runs on the machine should wait. The same lock can recur at `--jobs 8`, and the auto limit is only 37 s, so a raised `--timeout` is worth passing. Accepting `rerun-doc` as closure instead would be an owner decision.

### WARNING

**F-W1. 2 `Display` survivors, non-critical.** class: implementation, follow-up.
`contract/cancel.rs:68:9` (`Cancelled`) and `contract/params.rs:325:9` (`ParamError`), `fmt` replaced by `Ok(Default::default())`. Classification confirmed: AGENTS lists `Display`/`Error` text as non-critical unless the spec fixes it; `design.md` says "`Display` text is not critical" and fixes only "English, log only"; neither string appears in `02` or `design.md`; ARQ-102 keeps `Display` from the UI. Effect of the gap: an empty local log line. Everything else in both files is caught (7 and 48).

**F-W2. 10 testkit survivors, non-critical.** class: implementation, follow-up.
Classification confirmed: all are test helpers or trivial accessors, none in `progress.rs`, whose 56 mutants are all caught, so the negative controls `design.md` requires stand.

| Survivor | Why non-critical |
|---|---|
| `golden.rs:108:9` `Image::width` with 0, 1; `:113:9` `Image::height` with 0, 1; `:123:9` `Image::bytes` with 3 values (7) | Trivial accessors in a test helper, called by no test |
| `golden.rs:51:9` `Display for GoldenError` | `Display` text of a test helper |
| `lib.rs:326:18` `+` to `*` in `axis_length` | Generator input: one mode loses its `1.0` floor |
| `harness.rs:84:9` delete `failure_persistence` | Proptest config: decides whether a failure writes a regression file, not whether it fails |

**F-W3. ARQ-102 pending list omits the other error enums.** class: ambiguity, follow-up.
`design.md` and the Slice card name catalog, UI, lint and `UserError`. ARQ-102 also binds every enum of §7.3. Eight production enums have no `ToMessage`: `ValidationError`, `LoadError`, `PageLayoutError`, `PageSizeError`, `CommandError`, `ConversionError`, `TimeError`, `PixelBufferError`. The row defers them ("con el change que los adapte"), so this is no breach, but no list names them and no change owns them. The row's "hoy ... ningún error implementa `ToMessage`" is also out of date.

Carried follow-ups, all still open at HEAD:

- **D1-W1.** class: ambiguity, follow-up. Flow-change records the level only, not the pack and reason change.
- **D2b-W3.** class: implementation, follow-up. The atomicity tests lose power on small or busy machines; a deterministic proof needs `loom`.
- **D3-W1.** class: ambiguity, follow-up. Manual still says `gafira-pathops` depends on `gafira-geom` alone (`02-architecture.md`, lines 26 and 89, both languages).

Closed this round, not counted:

**F-W4. 2 `IRect` survivors are equivalent.** class: implementation, closed. See "Exclusion".

### SUGGESTION

- S1. The hang evidence for the five layout mutants lives only in the scratchpad, and `apply-progress.md` says just "confirmed hanging". Copy the five file:line rows, the limit and the reason into the record before the session ends.
- S2. Drop the "Load" column of the T4 table in `apply-progress.md`: it re-counts the timeout, which is why the row seems to add up to 251.

## T4

All five crates of `design.md`. Sources equal 7c4951d (`crates`, `Cargo.toml`, `Cargo.lock`, `.cargo` compared file by file), and a2dcf20 changes only `.cargo/mutants.toml` and `apply-progress.md`, so the run stands for HEAD.

| Crate | Found | Caught | Missed | Unviable | Timeout | Sum | Wall |
|---|---|---|---|---|---|---|---|
| gafira-doc | 250 | 190 | 2 | 57 | 1 | 250 | 672 s |
| gafira-geom | 87 | 69 | 2 | 16 | 0 | 87 | 44 s |
| gafira-pathops | 128 | 106 | 0 | 22 | 0 | 128 | 72 s |
| gafira-layout | 454 | 437 | 0 | 12 | 5 | 454 | 363 s |
| gafira-testkit | 251 | 214 | 10 | 27 | 0 | 251 | 674 s |
| Total | 1 170 | 1 016 | 14 | 134 | 6 | 1 170 | 1 825 s |

Critical missed: 0 real; the 2 `IRect` ones are equivalent. Critical timeouts: 6, of which 5 are confirmed hanging and 1 is F-C1.

### gafira-doc count: reconciled, no discrepancy

`outcomes.json` gives total 250, missed 2, caught 190, timeout 1, unviable 57, and 190 + 2 + 57 + 1 = 250. The 3 lines in the missed and timeout lists are 2 + 1, one file each. The file also holds 251 outcomes, because the baseline is the extra entry. The 251 in `apply-progress.md` comes from its "Load" column, which counts the one timeout a second time. `rerun-doc` shows "2 mutants" because its filter took both `with true` and `with false` of `validate.rs:61:29`; the 2 missed `Display` mutants were not in it.

### Layout timeouts: 5 confirmed hanging

Re-run limit 300 s against a 2 s baseline; each timed out again, with tests actually started and still running past 60 s, so load does not explain them. Reading the code gives the reason each cannot end. The sibling mutants (`-=`, `+`, `+=`) are all caught.

| Mutant | Why it does not terminate |
|---|---|
| `attractors.rs:61:34` `top.next *= 1` | Stays 0, re-enters the first child for ever |
| `attractors.rs:69:31` `next_root *= 1` | Stays 0, the finished root loops through `continue` |
| `bounds.rs:59:23` `top.1 *= 1` | Stays 0, re-folds the same child for ever |
| `grid.rs:115:31` `major_on_screen / multiple` | The left side only shrinks, 0 at infinity, so `< 6` always holds |
| `grid.rs:116:22` `multiple /= 2.0` | `multiple` falls to 0, so the product stays below 6 |

No assertion can come first: each mutant loops inside the function under test.

### Exclusion

`'irect\.rs:(23|32):20: replace > with >= in IRect::(width|height)$'`, reason present.

- Equivalent: `self.x1 > self.x0` and `>=` differ only at `x1 == x0`, where `abs_diff` is 0, the value of the `else` arm. The same holds for height. No input tells them apart.
- Exact match: over all five crates' `mutants.json` it matches these 2 and no other. `cargo mutants --list -p gafira-geom` at HEAD gives 85 against 87, the 2 gone and nothing new. The 8 neighbours on those lines, `==`, `<` and the return replacements, stay listed, and none was missed.

## Acceptance (`design.md`)

| Item | Proof | Status |
|---|---|---|
| `MsgArg`, `msg::Unit`, `Message` | `msg::tests::arq_102_every_arg_variant_and_unit_builds_and_compares` | covered |
| `msg!` 0, 1, 2 args, trailing comma, bad key | `tests/msg_macro.rs`, `msg.rs` `compile_fail` doctest line 93 | covered |
| `is_valid_key` table | `tests/msg_key.rs` (2 tests) | covered |
| ARQ-102 per-variant fixtures | `gafira-layout/tests/message.rs`, `gafira-pathops/tests/message.rs`, `contract_param_error.rs`, `contract_cancel.rs` | covered |
| ARQ-112 on doubles | `gafira-testkit/tests/progress.rs` (26 tests) | covered |
| `CancelToken` | `contract_cancel.rs` | covered |
| `checkpoint` | `contract_checkpoint.rs` | covered |
| `ParamSchema` | `contract_param_schema.rs` | covered |
| `IRect` | `irect::tests` (7 tests) | covered |
| V5, imports, `Locale` | V5 exit 0, `adr_0005_contract_imports_only_msg_color_serde_and_std`, no `Locale` in `crates` (one lowercase `locale` in a `msg.rs` comment) | covered |

10 of 10 covered. Delivery rounds: D1, D2a, D2b, D3 each closed at PASS.

Pending, by design:

- ARQ-102: key present in both catalogs, UI shows catalog text and never `Display`, the `gafira-ui` lint, `UserError` with no visible `String`. Named in `design.md` and the Slice card. Not named: the eight enums of F-W3.
- ARQ-112: the real operation, `gafira-segment`: cancel at `k` on `Segmenter::segment`, the 0.02 step on the reference input, two 1-thread runs with the same sequence. Named in `design.md`.

## Artifact integrity (step 3c)

- `07947c5 docs(engine-contracts): slice card` touches only `apply-progress.md`. `7b8c2cd ... slice card amend` too, and is the only later commit touching the card.
- `design.md` last changed at 75a066d, a coordinator commit that settles rules.
- Product IDs: ARQ-98 (labels), ARQ-102, ARQ-107, ARQ-110 (params), ARQ-112 are each named by tests.
- Left out, with reason: ARQ-98 `check-i18n` (no `xtask` yet, out of scope); ARQ-107 CI step, owned by `layer-2` SC1 (P9). Its checks hold at HEAD by direct search, and `Cargo.lock` has no `icu_`.

## Earlier findings

Every CRITICAL of D1, D2a, D2b and D3 is closed. Open: only the three carried follow-ups above.

## Not run

Independent probes and hand mutants (closure scope). SP items of `design.md` were not re-checked. No mutation was run by `verify`; `cargo mutants --list` only lists.

next: tdd-apply (implementation: F-C1). No code or test change is owed; the step owed is a new `mutants` T4 on `gafira-doc`, then a closure verify, then `archive`.

---

# Round 2: closure of F-C1

Commit a2dcf20 (`chore/engine-contracts-t4`). Mode full, depth V2, `mutation: archive`, tier T4. Closure scope: gates, artifact integrity, F-C1. No independent probes or hand mutants. The round 1 sections above are unchanged and stay as the record of that round.

## Result

- Verdict: **PASS**
- Counts: 0 CRITICAL, 6 WARNING (all follow-up, carried), 2 SUGGESTION (carried)
- Per class: routed none. Follow-up, not routed: implementation 3, ambiguity 3
- F-C1 is closed. All deliveries are done (D1, D2a, D2b, D3 at PASS).

## Gates

Clean export of a2dcf20, own `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`, exported files touched. Each exit code was written to and read from its own file. Build folder freed afterwards. Free disk before the run: 38 GB.

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (1 273 passed, 0 failed, 0 ignored) |
| `cargo deny check` | 0 |
| CI policy: ARQ-07, forbid unsafe, V5, V12, V13, V14 | 0, 0, 0, 0, 0, 0 |

Each policy step was extracted from `ci.yml` and run alone under `bash --noprofile --norc -eo pipefail`, in the worktree, because V13 and V14 need git.

## F-C1: closed

Report: `t4-doc2/out/mutants.out/` under the scratchpad. Copied from `outcomes.json`, not from the log summary: found 250, caught 191, missed 2, timeout 0, unviable 57, and 191 + 2 + 57 + 0 = 250. `run.log`, `caught.txt`, `missed.txt`, `unviable.txt` and the empty `timeout.txt` agree. Wall 588 s. Log `exit 2` is the normal cargo-mutants code for missed mutants.

**The mutant.** `validate.rs:61:29`, `replace match guard self.next_id <= max_id with false`: `CaughtMutant`. Build ok, test phase 0.6 s, exit 101. The test binary ran: `document::tests::from_parts_validates` and `validate::tests::id_rows` fail on assertions, 101 passed, 2 failed. The same two tests as `rerun-doc`.

**Its report's commit contains the code.** cargo-mutants records no commit, so this is shown indirectly, four ways:

- Source root is this worktree (`debug.log`). Its reflog holds a2dcf20 as the last HEAD move, at 23:49:42; the run started 23:57:03 and ended 00:06:51 (-05).
- `git diff HEAD` is empty and no file under `crates`, `Cargo.toml`, `Cargo.lock` or `.cargo` is newer than the commit.
- All 250 mutant diffs apply to the export of a2dcf20 with `patch -F0`: 0 rejected, 0 at an offset.
- a2dcf20 changes only `.cargo/mutants.toml` and `apply-progress.md`, so no fix was owed or expected in the crate (round 1).

**Full T4, not a partial re-run.** No `--file`, `--re`, `--shard` or `--in-diff` filter shows in the run: package `gafira-doc` alone, 16 source files. The 250 names in `mutants.json` equal `cargo mutants --list -p gafira-doc` at HEAD, set for set. Outcome by outcome against the round 1 report, same 250 mutants, one change: this mutant, `Timeout` to `CaughtMutant`. No regression. `rerun-doc` (2 mutants) was not used as evidence.

**Limit.** Auto limit 180 s (`Auto-set test timeout to 180s`), against 37 s in round 1; 8 worker threads in `debug.log`, which matches `--jobs 8`. The argv itself is not logged. Longest test phase 21.5 s, median 0.7 s, so no mutant came near the limit. Lock contention did recur: 145 logs hold the `package cache` line, this mutant's among them, but it no longer reached any verdict.

**Survivors.** The 2 missed are the two of F-W1 (`cancel.rs:68:9`, `params.rs:325:9`), non-critical. Critical missed: 0. Critical timeouts in `gafira-doc`: 0.

Whole-change T4 after this report: found 1 170, caught 1 017, missed 14, timeout 5, unviable 134. The 5 are the confirmed hanging layout mutants; the 2 `IRect` missed are excluded as equivalent. The other four crates' reports stand as in round 1.

## Artifact integrity

The `slice card` and `slice card amend` commits touch only `apply-progress.md`, and no later commit touches the card; nothing changed since round 1. HEAD is the same a2dcf20. `verify-report-final.md` and `verification-final.json` are untracked in the worktree; the coordinator commits them.

## Findings status

| Id | Severity | Class | Status |
|---|---|---|---|
| F-C1 | CRITICAL | implementation | closed |
| F-W4 | WARNING | implementation | closed in round 1 |
| F-W1 | WARNING | implementation | open, follow-up |
| F-W2 | WARNING | implementation | open, follow-up |
| F-W3 | WARNING | ambiguity | open, follow-up |
| D1-W1 | WARNING | ambiguity | open, follow-up |
| D2b-W3 | WARNING | implementation | open, follow-up |
| D3-W1 | WARNING | ambiguity | open, follow-up |

S1 and S2 are carried, not re-checked. S1 still matters before the session ends: the T4 evidence, this report's included, lives in the scratchpad.

## Not run

Independent probes and hand mutants (closure scope). No mutation was run by `verify`; `cargo mutants --list` only lists.

next: archive
