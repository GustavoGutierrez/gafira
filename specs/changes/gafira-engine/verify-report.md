# Verify — `gafira-engine`, delivery `E1` (scaffold), round 1

Depth V1 (Flow line, `decided-by: owner`), mode `standard`, tier T1 "hand mutants only",
`mutation: off`. No `mutants` report expected and none read.

```json
{ "schema": 1, "change": "gafira-engine", "delivery": "E1", "rounds": [ {
  "round": 1, "commit": "ae0e76efe0d349c7ee659e18577960dbe736e576", "mode": "standard",
  "tier": "T1", "depth": "V1", "verdict": "FAIL", "next": "architect",
  "counts": {"critical": 1, "warning": 4, "suggestion": 3},
  "classes": {"architecture": 1, "implementation": 3},
  "findings": [
    {"id": "C1", "severity": "CRITICAL", "class": "architecture",
     "ref": ".github/workflows/ci.yml:184 (ARQ-107 step) / specs/changes/gafira-engine/design.md XP1",
     "summary": "ARQ-107 policy step exits 1: it greps every V5-table crate for Locale and exempts only gafira-ports, so gafira-engine fails. Design XP1 claimed the step leaves layer 3 alone.",
     "follow_up": false, "status": "open"},
    {"id": "W1", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-engine/src/engine.rs:410-423 (AC5)",
     "summary": "AC5 'keeps the ports it was given' is unasserted: the test checks config and registries only, never engine.ports(). Its second test has no assertion and cannot fail.",
     "follow_up": false, "status": "open"},
    {"id": "W2", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-engine/src/event.rs:129-136 (AC4)",
     "summary": "AC4 'the no-op sink accepts every variant' exercises 4 of 5 variants and omits LocaleChanged; it asserts nothing, so it can only fail by not compiling.",
     "follow_up": false, "status": "open"},
    {"id": "W3", "severity": "WARNING", "class": "implementation",
     "ref": "specs/changes/gafira-engine/apply-progress.md ## Notes; commit 2271ab5",
     "summary": "The hand-mutant record AGENTS.md requires (item plus killing test) is missing: Notes is the unfilled template and the implementer body is the subject line. Both announced mutants are killed by reading.",
     "follow_up": false, "status": "open"},
    {"id": "W4", "severity": "WARNING", "class": "implementation",
     "ref": "commit 2271ab5",
     "summary": "654 lines against the 230 forecast (322 at x1.4), about 437 non-test. Under the ~700 ceiling, so no owner OK is triggered, but 1.9x the forecast for authored lines.",
     "follow_up": false, "status": "open"},
    {"id": "S1", "severity": "SUGGESTION", "class": "architecture",
     "ref": "specs/changes/gafira-engine/design.md O1",
     "summary": "The purity is sound for E3 (clock, files and repository are ports) and does not by itself break E7, but O1 is still unanswered and E7's API depends on the answer.",
     "status": "open"},
    {"id": "S2", "severity": "SUGGESTION", "class": "claim",
     "ref": ".odd/state.json criteria[E1-AC2]",
     "summary": "The E1-AC2 entry says passed on V5 plus the clippy.toml cmp, omitting the ARQ-107 step, which is a third consumer of the V5 table and is red. The entry needs correcting.",
     "status": "open"},
    {"id": "S3", "severity": "SUGGESTION", "class": "implementation",
     "ref": "crates/gafira-engine/src/event.rs:29",
     "summary": "EngineEvent is not #[non_exhaustive]. Harmless today (no consumer exists, verified), worth deciding when E2 lands the first external matcher.",
     "status": "open"}
  ],
  "gates": {"fmt": 0, "clippy": 0, "test": 0, "deny": 0, "ci_policy": 1},
  "mutation": null,
  "hand_mutants": {"run": 0, "killed": 0},
  "criteria": {"total": 6, "covered": 5},
  "size": {"lines": 654, "forecast": 230}, "report": "verify-report.md" } ] }
```

## Gates at HEAD `ae0e76e`

`--workspace` scope per ADR 0017 (the delivery touches `Cargo.toml`, `Cargo.lock` and `ci.yml`).

| Gate | Command | Exit |
|---|---|---|
| fmt | `cargo fmt --check` | 0 |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| test | `cargo test --workspace --locked` | 0 |
| deny | `cargo deny check` (`advisories ok, bans ok, licenses ok, sources ok`) | 0 |
| ci_policy | the nine `run` steps of the `policy` job, each under `bash -euo pipefail` | **1** |

Test counts: **2496 passed, 0 failed, 0 ignored** across 198 report lines. `gafira-engine`'s own
unit binary: **7 passed** (3 `ac3_*`, 2 `ac4_*`, 2 `ac5_*`); its doc-tests: 0.

Policy steps (all nine ran; `checkout`, the toolchain install and the `cargo-deny-action` step are
not checks and were handled separately — deny ran as `cargo deny check`):

```
Format: 0
CLI has no UI dependencies (ARQ-07): 0
Domain crates forbid unsafe code: 0
Domain crates respect the layer table (V5): 0
Domain clippy.toml copies are in sync (V12): 0
Layers 0-2 know no language (ARQ-107): 1
EditToken only in the edit crates (V13): 0
Every core command has a property test (V14): 0
Testkit ports feature compiles (ADR 0013): 0
```

The extractor `/tmp/gafira-policy-run.py` was re-checked against the file: the `policy` job has 12
steps, of which 10 are `run` and one (`Install toolchain`) is correctly skipped; the nine remaining
`run` steps are the checks above. No check was skipped.

## Findings

| id | severity | class | where | what |
|---|---|---|---|---|
| C1 | CRITICAL | architecture | `ci.yml` ARQ-107 step; design XP1 | The ARQ-107 step exits 1 on `gafira-engine mentions Locale`. The step iterates **every** crate in the V5 table and exempts `gafira-testkit` and `gafira-ports` only, so registering the engine with a `LocaleChanged` event turns the job red. |
| W1 | WARNING | implementation | `engine.rs:410` | AC5's "keeps the ports it was given" is not asserted: `engine.ports()` is never read by any test, and `ac5_publishing_through_the_facade_reaches_the_bus` contains no assertion (it can only fail by not compiling). |
| W2 | WARNING | implementation | `event.rs:129` | AC4's "the no-op sink accepts every variant" publishes `DocumentChanged`, `LinksMissing`, `LicenseChanged`, `BackendChanged` — not `LocaleChanged`. The test asserts nothing: `NoopSink::publish` is `{}`, so it cannot fail at runtime. |
| W3 | WARNING | implementation | `apply-progress.md` Notes, `2271ab5` | The hand-mutant record is missing. `## Notes` is still the template line and the implementer commit body is the subject only. AGENTS.md requires item plus killing test. By reading, both announced mutants *would* be killed: dropping the empty-path check makes `expect_err` panic; dropping the sink call leaves the counting sink at 0. |
| W4 | WARNING | implementation | `2271ab5` | 654 inserted lines against 230 forecast (322 at x1.4): about 437 non-test (364 code + 47 verbatim `clippy.toml` + 26 manifests/workflow) and about 217 of tests and doubles. Below the ~700 ceiling; the coordinator decides. |
| S1 | SUGGESTION | architecture | design O1 | Purity judgement, below. |
| S2 | SUGGESTION | claim | `.odd/state.json` | `E1-AC2` is recorded `passed` on a proof that skips the ARQ-107 step. |
| S3 | SUGGESTION | implementation | `event.rs:29` | `EngineEvent` has no `#[non_exhaustive]`. |

### C1 — the ARQ-107 step does not leave layer 3 alone

```
::error::gafira-engine mentions Locale (ARQ-107):
crates/gafira-engine/src/event.rs:14:use gafira_ports::i18n::{LanguagePreference, ResolvedLocale};
crates/gafira-engine/src/event.rs:37:    LocaleChanged {
crates/gafira-engine/src/event.rs:41:        resolved: ResolvedLocale,
Layers 0-2 know no language (ARQ-107): exit=1
```

The crate is right and the step is the problem. `EngineEvent::LocaleChanged` is 02 §5.2 (1319,
1326) and design P3 deliberately takes it in `E1` because `LanguagePreference`/`ResolvedLocale`
already exist in `gafira-ports`. The step's own name and ARQ-107 scope the ban to **layers 0-2**,
but its loop runs over every V5 row and greps each crate for `Locale`, exempting only
`gafira-testkit` (dev-only) and `gafira-ports` ("layer 3, exempt from the Locale search"). Every
other layer-3 crate passes only because it happens not to mention the word.

Design **XP1** asserted the opposite — "the ARQ-107 step leaves layer 3 alone" — and listed itself
under "Needs explorer", so no probe ever ran it. The false assumption is the root cause: `E1` is
the first delivery that puts a layer-3 crate with a locale-carrying event into the table.

Two ways out, both decisions rather than repairs:

- (a) exempt `gafira-engine` from the `Locale` search in the ARQ-107 step, as `gafira-ports` is,
  with the reason that §5.2's `LocaleChanged` is the engine's contract and ARQ-107 binds layers
  0-2. One line in a file `E1` already owns; V5, V12 and the step's dependency check are untouched.
- (b) drop `LocaleChanged` from `E1` and take it in `E10` with `SettingsService`. That keeps the
  step unchanged but contradicts P3 and leaves §5.2's vocabulary incomplete for longer.

ADR 0016 settles that a layer-3 crate carries the copy (P1 is sound), but it says nothing about the
`Locale` search, so it does not authorise either option on its own.

**AC2 as worded still passes** — it names only the V5 and V12 snippets, and both exit 0. That is
the second half of the problem: the criterion's proof surface omits a third consumer of the V5
table, which is why the state entry could record `passed` while the job is red (S2).

### S1 — is the purity a reasonable choice, and does it break `E3`/`E7`?

The V5 row is exact. `cargo tree -p gafira-engine -e normal,build --target all` lists
`gafira-ports gafira-doc gafira-edit gafira-geom gafira-color gafira-time gafira-raster` and the
crate itself — the seven internal dependencies the row declares, no more and no fewer. So V5's
comparison is satisfied by construction, not by luck, and V12's copy is byte-identical (same md5,
`diff` empty, V12 exit 0).

Purity is the right trade for the engine, on the same ground ADR 0016 used twice:

- `E3` is unaffected. `ProjectService` saving, autosave debounce and recovery all go through
  `ports::ProjectRepository` and `ports::FileSystem`; time comes from `ports::Clock`, which `E1`
  already injects. Nothing in `E3` needs `std::fs`, `Instant::now` or a spawned thread.
- `E7` is the one real constraint, and it is a scheduling problem, not a purity problem. §6.4's
  retries and §6.3's one-job-at-a-time pool can be driven on the caller's tick (P8), or the engine
  can call `spawn` on a scheduler trait it does not own (O1 option c). Both keep the lint set
  intact. Option (b) of O1 — an exception to `clippy.toml` — is the only one that costs a second
  canonical lint set, which is exactly what ADR 0016 rejected in writing.
- The engine is pull-based in the same way as `gafira-media`, so the precedent is already in the
  codebase rather than invented here.

The residual risk is not the purity but that **O1 is still unanswered by the owner** and `E7`'s API
shape depends on the answer. Answer it before `E7` starts, not after.

### Artifact integrity (steps 3, 3c, 3d)

- **Slice card** (`901e66c`, `docs(gafira-engine): design and slice card for the scaffold`) touches
  only `specs/changes/gafira-engine/apply-progress.md` and `design.md`. One commit for two
  coordinator docs in a full-mode folder is one reviewable unit, and no rule covers the design
  commit; the card is untouched afterwards (`git log` over the file shows only `901e66c`). No
  implementer commit changed the card or the design. The card declares "Product IDs: none", so the
  per-ID `git grep` check is vacuous here — correctly: `E1` adds no user-visible capability.
- **`.odd/state.json`** has the `gafira-engine` entry with the six typed criteria `E1-AC1`…`E1-AC6`,
  each with `status` and `evidence`, plus `flow`, `scope`, `non_goals` and `next_action`. It was
  written by `dcb2cdd` (before the implementation) and `ae0e76e` (the evidence), both coordinator
  `docs(...)` commits. The implementer commit `2271ab5` **does not touch the file** — verified by
  `git log -- .odd/state.json`. Nothing was smuggled in.
- **Registration diffs are minimal**: `Cargo.toml` one line, `Cargo.lock` the nine-line package
  entry, `ci.yml` the single V5 row. No unrelated edits in the implementation commit.

### The four flagged items, judged

1. **Purity imposed on a layer-3 crate** — the row is exact, the purity is consistent with ADR 0016
   and will not break `E3`; it constrains `E7` to a host-pumped or port-injected scheduler. O1 is
   the decision that must be taken, and it is currently unanswered. See S1. The row itself is not
   the problem; the `Locale` grep is (C1).
2. **`EngineEvent` born incomplete** — "no consumers yet" is **true**: `git grep EngineEvent`
   outside the crate returns nothing, and no crate depends on `gafira-engine`. The `DocVersion` gap
   is recorded as open question **O2** in the design and cited as such in `event.rs:26-27`; there is
   no TODO, FIXME or `unimplemented!` anywhere in the crate. It is an honest, tracked gap, not a
   buried one.
3. **`EngineError` / `Display` provisional** — no caller matches on the text. The three format
   strings exist only in the `Display` impl (`engine.rs:172-174`), no test asserts them, and the
   docs mark the text as Planned for `E2`. Safe to change.
4. **Size** — 654 lines for `2271ab5` against 230 forecast (W4). Under the ceiling, so no owner OK
   is required; the coordinator decides whether the overrun warrants a note. The bulk is honest: 47
   lines are the verbatim `clippy.toml` copy and about 217 are tests and the six port doubles, which
   `E1` must write because `gafira-testkit`'s doubles would be a dev-dependency `E1` does not take.

## AC1–AC6 map

| AC | What proves it | State |
|---|---|---|
| AC1 — builds with `#![forbid(unsafe_code)]`, no `std::fs`/`net`/thread/clock import | `lib.rs:14` carries the attribute (and the policy step "Domain crates forbid unsafe code" exits 0); `clippy` exits 0 at `--all-targets`; `grep -rnE 'std::(fs\|net\|thread\|process\|env)\|SystemTime\|Instant::now\|spawn' crates/gafira-engine/src/` matches only two doc comments. **Limit, from reading:** with zero violating calls, clippy's pass cannot distinguish an active ban from an absent one; the enforcement is inherited from the byte-identical canonical copy that 15 crates already rely on, and V12 exit 0 proves the copy is in place. No probe was run: V1 owes none. | **covered** (enforcement inherited, not probed) |
| AC2 — the V5 and V12 steps accept the new row and the copy | Both snippets run literally: V5 exit 0, V12 exit 0. Also verified independently: the tree is exactly the seven declared dependencies and the two `clippy.toml` files share an md5. **But** a third consumer of the V5 table (ARQ-107) exits 1 — see C1. | **partial** (V5 0, V12 0, ARQ-107 1) |
| AC3 — `EngineConfig::new` rejects an empty data directory with `InvalidConfig`, keeps the limits | `ac3_an_empty_data_directory_is_rejected` (asserts `InvalidConfig` on `""`), `ac3_the_limits_the_caller_gives_are_kept` (memory limit, export jobs, data dir), `ac3_a_fresh_configuration_runs_one_export_job_and_declares_no_limit` (defaults). Both directions of the branch, so a dropped check fails `expect_err`. | **covered** |
| AC4 — `publish` reaches the sink exactly once per event; the no-op sink accepts every variant | `ac4_publish_reaches_the_sink_exactly_once` proves the first half with a counting sink (it can fail: 0 or 2 both trip the assertion). The second half is weak — 4 of 5 variants, no assertion (W2). | **covered, with a gap** |
| AC5 — `Gafira` is `Send + Sync` and keeps the ports it was given | `assert_send_sync::<Gafira>()` is a real compile-time proof (it fails to compile if the bound is lost). Config and registries are asserted. The ports are not (W1). | **covered, with a gap** |
| AC6 — no `gafira-i18n` and no ICU crate in the tree | `cargo tree -p gafira-engine -e normal,build --target all -f '{p}'` lists no `gafira-i18n` and no `icu_*` (grep returns nothing). The engine imports `gafira_ports::i18n` — a module of a layer-3 crate already in the tree, not the banned crate. The ARQ-107 step's dependency half passed for the same reason. | **covered** |

`Ports` correctly has **no** `Default` impl, so P4's "a missing port must not compile" holds by
construction. `Gafira::new` returns `Result` but can never fail in `E1`; that is §5.4's signature,
not a defect — worth remembering when a mutant cannot kill it.

## Not owed, not run

- No `mutants` report: `mutation: off` (P10). Not a finding.
- No hand mutants: V1 owes none. The two the Slice card announces are killed by reading; the
  deviation is that they were not recorded (W3).
- No independent probes or per-commit exports: V1.

next: architect (architecture: C1; implementation: W1 W2 W3 W4; also correct E1-AC2 in `.odd/state.json`)

---

# Verify — `gafira-engine`, delivery `E1` (scaffold), round 2 (closure)

Depth V1 (Flow line), mode `standard`, tier T1, `mutation: off`. Closure round: the gates of step 2,
the artifact-integrity checks of steps 3, 3b/3c and 3d, and closure of each round-1 finding, with
probes limited to that. The round-1 block above is unchanged; this block is the round's record.

```json
{ "schema": 1, "change": "gafira-engine", "delivery": "E1", "rounds": [ {
  "round": 2, "commit": "35e4302d33873970c05c9cd273ffabf36fdf9085", "mode": "standard",
  "tier": "T1", "depth": "V1", "verdict": "PASS", "next": "tdd-apply",
  "counts": {"critical": 0, "warning": 2, "suggestion": 1},
  "classes": {"implementation": 1},
  "findings": [
    {"id": "C1", "severity": "CRITICAL", "class": "architecture",
     "ref": ".github/workflows/ci.yml:180 (ARQ-107 exemption) / specs/changes/gafira-engine/design.md:299 (XP1)",
     "summary": "ARQ-107 exits 1: it greps every V5-table crate for Locale and exempts only gafira-ports, so gafira-engine fails. Design XP1 claimed the step leaves layer 3 alone.",
     "follow_up": false, "status": "closed"},
    {"id": "W1", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-engine/src/engine.rs:400-436 (AC5)",
     "summary": "AC5 'keeps the ports it was given' is unasserted: the test checks config and registries only, never engine.ports(). Its second test has no assertion and cannot fail.",
     "follow_up": false, "status": "closed"},
    {"id": "W2", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-engine/src/event.rs:183-195 (AC4)",
     "summary": "AC4 'the no-op sink accepts every variant' exercises 4 of 5 variants and omits LocaleChanged; it asserts nothing, so it can only fail by not compiling.",
     "follow_up": false, "status": "closed"},
    {"id": "W3", "severity": "WARNING", "class": "implementation",
     "ref": "specs/changes/gafira-engine/apply-progress.md ## Notes; commit 2271ab5",
     "summary": "The hand-mutant record AGENTS.md requires (item plus killing test) is missing: Notes is the unfilled template and the implementer body is the subject line.",
     "follow_up": false, "status": "closed"},
    {"id": "W4", "severity": "WARNING", "class": "implementation",
     "ref": "commits 2271ab5 + 0306b9c/e6db241",
     "summary": "Size: 654 lines against the 230 forecast at the delivery commit, 705 at HEAD. Under the ~700 ceiling before the fix round; the coordinator and the owner decide. Not re-judged.",
     "follow_up": false, "status": "open"},
    {"id": "S1", "severity": "SUGGESTION", "class": "architecture",
     "ref": "specs/changes/gafira-engine/design.md O1",
     "summary": "The purity is sound for E3 and does not by itself break E7, but O1 is still unanswered and E7's API depends on the answer. Owner's call; not re-judged.",
     "status": "open"},
    {"id": "S2", "severity": "SUGGESTION", "class": "claim",
     "ref": ".odd/state.json criteria[E1-AC2]",
     "summary": "The E1-AC2 entry says passed on V5 plus the clippy.toml cmp, omitting the ARQ-107 step, a third consumer of the V5 table and red at the time.",
     "status": "closed"},
    {"id": "S3", "severity": "SUGGESTION", "class": "implementation",
     "ref": "crates/gafira-engine/src/event.rs:31",
     "summary": "EngineEvent is not #[non_exhaustive]. Harmless today (no consumer exists, verified), worth deciding when E2 lands the first external matcher.",
     "status": "closed"},
    {"id": "W5", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-engine/src/engine.rs:247-249 (Gafira::publish); apply-progress.md:66; commit 0306b9c",
     "summary": "Gafira::publish untested after the AC5 test was deleted: gutting its body leaves all 6 engine tests green. The stated rationale is false — an in-crate counting bus observes delivery in E1 today.",
     "follow_up": true, "status": "open"}
  ],
  "gates": {"fmt": 0, "clippy": 0, "test": 0, "deny": 0, "ci_policy": 0},
  "mutation": null,
  "hand_mutants": {"run": 2, "killed": 1},
  "criteria": {"total": 6, "covered": 6},
  "size": {"lines": 705, "forecast": 230}, "report": "verify-report.md" } ] }
```

`counts` and `classes` are the findings open at the end of this round; `hand_mutants` is the H1
spot-check plus one probe beyond it (below), counted so the survivor is visible.

## Gates at HEAD `35e4302`, engine row present in the V5 table

`--workspace` scope per ADR 0017. `CARGO_TARGET_DIR=/tmp/gafira-engine-v2-target`,
`CARGO_INCREMENTAL=0`.

| Gate | Command | Exit |
|---|---|---|
| fmt | `cargo fmt --check --all` | 0 |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| test | `cargo test --workspace --locked` | 0 |
| deny | `cargo deny check` (`advisories ok, bans ok, licenses ok, sources ok`) | 0 |
| ci_policy | the nine `run` steps of the `policy` job | 0 |

Test counts: **2495 passed, 0 failed, 0 ignored** across 198 binaries. `gafira-engine`'s unit
binary: **6 passed**; its doc-tests: 0.

Policy steps — all nine ran, none skipped:

```
Format: 0
CLI has no UI dependencies (ARQ-07): 0
Domain crates forbid unsafe code: 0
Domain crates respect the layer table (V5): 0
Domain clippy.toml copies are in sync (V12): 0
Layers 0-2 know no language (ARQ-107): 0
EditToken only in the edit crates (V13): 0
Every core command has a property test (V14): 0
Testkit ports feature compiles (ADR 0013): 0
```

The extractor `/tmp/gafira-policy-run.py` was re-checked against the file: the `policy` job has 12
steps, 10 with `run`, one of them (`Install toolchain`) skipped, so the nine above are the checks.
No check was skipped.

**ARQ-107 is green because of the engine row, not despite it.** Three checks, not one:

- Positive: the step run alone prints `gafira-engine: layer 3, exempt from the Locale search` — the
  engine row is iterated (`ci.yml:111`) and reaches the exemption arm (`ci.yml:180-183`).
- Negative control, the same script with the name removed from the list (`gafira-ports` only, i.e.
  the pre-merge step), exits **1** and lists five `Locale` hits under `crates/gafira-engine`. So the
  exemption is load-bearing, not decorative, and the green is not the row having been skipped.
- The dependency half of the step (the `banned` grep and `cargo tree`) runs before the exemption arm
  for every crate, the engine included; nothing is bypassed but the file search.

## Closure table

| id | Round-1 finding | Proof of closure | Status |
|---|---|---|---|
| C1 | ARQ-107 red; XP1 claimed layer 3 untouched | **Both halves.** Workflow: `ci.yml:180-183` lists `gafira-ports\|gafira-engine`; the step exits 0 with the engine row iterated, and exits 1 without the name (negative control above). Design: `design.md:299` now states that the step iterates every V5 crate, applies the dependency check to all, and skips only the `Locale` search by name — which is what the merged workflow does, line for line. | **closed** |
| W1 | AC5's ports unasserted; second test asserted nothing | `ac5_the_facade_is_send_and_sync_and_keeps_the_ports_it_was_given` now builds the six trait-object `Arc`s, hands clones to `Gafira::new`, reads `engine.ports()` and asserts `Arc::ptr_eq` on all six; the assertion-less test is gone. `Send + Sync` and the config assertions kept. | **closed** |
| W2 | AC4 covered 4 of 5 variants and asserted nothing | `ac4_the_noop_sink_accepts_every_variant` publishes all five through `every_variant()` (LocaleChanged built from `LanguagePreference::System` + `ResolvedLocale`) via a counting sink that forwards to the real `NoopSink`, and asserts `index + 1` per event and `5` in total: a dropped variant fails the count. | **closed** |
| W3 | No hand-mutant record | `apply-progress.md:49-58` has the table (item, killing test, failure message) and the revert statement. I re-applied H1 myself: the named test failed with the recorded text (below). H2 by reading: with `EventBus::publish` returning early the counting sink stays at 0 and `assert_eq!(…, 1)` fails. | **closed** |
| S2 | E1-AC2 recorded `passed` on a proof that skipped ARQ-107 | The entry now names all three consumers of the V5 table and the policy run. Checked independently: exactly three steps read the table (V5, V12, ARQ-107, verified by parsing the workflow), and all three exit 0 with the engine row. | **closed** |
| S3 | `EngineEvent` not `#[non_exhaustive]` | `event.rs:31` carries the attribute; the doc comment gives the reason (the variants keep arriving). No in-tree matcher, so nothing needed a wildcard — clippy 0 confirms. | **closed** |
| W4 | Size 654 vs 230 forecast | Measured, not judged: 654 at `2271ab5`, 705 at HEAD (632 `src` + 47 `clippy.toml` + 26 manifests and workflow); the fix round is `git diff --shortstat e7bb63f..HEAD -- crates/` = 2 files, 76 insertions, 25 deletions, which matches what `apply-progress.md:82-84` claims. | **open — coordinator/owner** |
| S1 | Purity judgement, O1 unanswered | Stated only: the V5 row is still exact (no dependency change this round), the purity argument is unchanged, and O1 is still open. | **open — owner** |

## The three items this round gave to judgement

### 1. The deleted `ac5_publishing_through_the_facade_reaches_the_bus`

**It is housekeeping as to its content, and it is not free.** Two separate questions:

*Did it assert anything?* No. Its whole body was `let engine = Gafira::new(...).expect(...);
engine.publish(EngineEvent::LinksMissing(Vec::new()));` — no assertion, so at runtime it could only
fail by panicking, and the panic would have been `Gafira::new`. Deleting a test that asserts nothing
is housekeeping, and nothing was red, so this is **not** a test removed to make a delivery green.

*Was the stated reason true?* No, and this is where it costs something. `apply-progress.md:66-69`
says the test "could not observe delivery and could only fail by not compiling" because `E1` injects
no sink. That is false, and I proved it rather than arguing it: a 30-line test inside `engine::tests`
builds `Gafira` with `bus: EventBus::new(counting)` — legal, because `mod tests` is a child of
`mod engine` so the private field is in reach and `EventBus::new` is public — calls
`engine.publish(...)` and asserts the count. **It passed.** (Applied to the worktree, run, reverted;
tree clean.) So facade delivery *is* observable in `E1` without touching `Gafira::new`.

*Does the gap matter?* Yes, and it is the load-bearing part of this finding. With
`Gafira::publish`'s body replaced by `let _ = event;`, **all six engine tests pass** — the deletion
removed the last thing that touched a public §5.4 method. That is W5, a WARNING `implementation`
marked `follow-up`: a new test gap after round 1, not a FAIL, and the mutant is non-critical (a
forwarding accessor, "trivial accessors" in `AGENTS.md`'s non-critical list).

*Should `E1` inject the sink to prove it now?* I would not. `Gafira::new(ports, registries, config)`
is 02 §5.4's signature and the noop bus is design P2's deliberate choice for this delivery; changing
a public contract to serve a test is the worse trade. The cheap fix is the in-crate test above, or
wait for `E2`, which `apply-progress.md:68-69` already promises. Since `E2` is next and owns the
channel sink, `follow-up` is the honest label — but it must land in `E2`, not be re-promised again.

*Is the workspace drop 2496 → 2495 explained by it alone?* **Yes.** The fix round's code commits
touch only `crates/gafira-engine` (`0306b9c`, `e6db241`), so the engine's unit binary is the only
one that can move; it went 7 → 6, the workspace 2496 → 2495, over the same 198 binaries. Both
rewritten AC4/AC5 tests replaced tests in place (AC4 stayed at 2, AC5 went 2 → 1), so the -1 is
exactly the deletion: no test was added or lost anywhere else.

### 2. The implementer's edit to `.odd/state.json`

**Minimal, and accurate.** `git show 35e4302 -- .odd/state.json` is three evidence strings
(E1-AC2, E1-AC4, E1-AC5), three lines out and three in. `flow`, `scope`, `non_goals`, every `id`,
every `status` (`passed` throughout) and the criterion order are untouched. The E1-AC2 text now
names the three consumers and commits to the policy run; I checked that set is exactly three by
parsing the workflow (`table=` is defined once and read by V5, V12 and ARQ-107 and by nothing else),
that the run is green with the engine row (gates above), and that citing `e7bb63f` is right — the
exemption reached the tree through that merge from the develop side. Nothing in the entry is now
false. (The AC4/AC5 strings also match the code: `ac4_*` is still two tests and
`ac5_the_facade_is_send_and_sync_and_keeps_the_ports_it_was_given` exists.)

**Contract line vs assignment: the landed shape complies; the briefing wording is what should
change.** `tdd-apply.md:21` ends "Never touch `.odd/state.json`", and `AGENTS.md` gives that file's
`status`, `evidence`, `verify` and `next_action` to the coordinator. In git the edit arrives in
`docs(gafira-engine): …` — a coordinator commit — and `git log 2271ab5..HEAD -- .odd/state.json`
shows no `feat`/`test` commit besides it, so step 3d's "no implementer commit touched the file"
holds. The contract is right and should not be weakened: an implementer that edits the evidence for
the criteria it is judged by is the exact boundary the line protects. What should change is the
coordinator's fix brief — "report the corrected evidence; the coordinator commits it" — because an
assignment phrased as "correct E1-AC2" reads as an instruction to edit the file and puts the worker
in conflict with its own contract. If the coordinator did instruct the implementer to make the edit,
then the disagreement is real and the assignment, not the contract, is the thing to fix.

Two smaller observations, neither a finding. The state edit travelled in the same commit as the
implementer's `## Notes` section, so the hand-mutant record's provenance cannot be read from git
(the content is true — H1 reproduced — but `AGENTS.md` wants it in the implementer's own commit).
And `next_action` still reads "independent verify round at V1 for E1 (no round ran yet)", stale
since `ae0e76e`; it is the coordinator's field and closes on its next state write.

### 3. `XP1` rewritten

**It now tells the truth about the step, and the claim is provable from the merged workflow.**
`design.md:299` says: the step iterates **every** crate of the V5 table; applies its dependency
check to **all** of them; skips the `Locale` file search only for the names on the exemption list;
the list holds `gafira-ports` and `gafira-engine`; membership is by name, so a future layer-3 crate
must be added deliberately.

Each clause maps to a line of `ci.yml`, not to prose:

| XP1 clause | Workflow |
|---|---|
| iterates every V5 crate | `crates=$(awk 'NF{print $1}' <<<"$table")` over the table that holds `gafira-engine` (`ci.yml:111`) |
| dependency check on all of them | `cargo tree` + `grep -E "^($banned) v"` run *before* the exemption arm, for every `name` |
| skips only the `Locale` search, by name | `case "$name" in gafira-ports\|gafira-engine) … continue ;; esac` |
| two names, membership deliberate | the list is literal; removing `gafira-engine` turns the step red (negative control) |

The engine's exemption really is one word in `ci.yml`: `git diff` against the develop base shows the
only semantic change is `gafira-ports` → `gafira-ports|gafira-engine`; the `if` became a `case` and
the comment grew, neither of which changes behaviour for `gafira-ports`. The design's rationale cites
ADR 0016; the comment in this tree still cites ADR 0013 alone, but develop's `5138abe` — the comment
change the coordinator said need not be merged — names both, so the design is citing the ADRs the
step will carry once `E1` merges back. No defect; note it so the comment is not "fixed" the other way.

## Hand mutant, reproduced here (not trusted from the record)

H1 applied to the worktree, run, reverted (`git checkout --`), tree verified clean, engine green
again afterwards:

```
test engine::tests::ac3_an_empty_data_directory_is_rejected ... FAILED
thread '…' panicked at crates/gafira-engine/src/engine.rs:372:43:
an empty data directory must be rejected: EngineConfig { data_dir: "", memory_limit: None, export_jobs: 1 }
```

Same message as `apply-progress.md:53`; the recorded `:377` is the pre-mutation line number (the
deletion of the five-line check is what moves it to 372). The record is accurate. No independent
hand mutants: this is a closure round. The one extra mutant I applied — `Gafira::publish` gutted —
survived, and is reported as W5 rather than hidden in the kill count.

## Artifact integrity (steps 3c, 3d)

- **Slice card** — `901e66c` created it in `apply-progress.md` and `design.md`; `35e4302` then edited
  **only** the `## Notes` section below it. No implementer commit touched the card or the design (the
  only commits over those files are `901e66c`, `085b6ea` and `35e4302`, all `docs(...)`). The card
  declares "Product IDs: none", so the per-ID `git grep` check is vacuous here, correctly: `E1` adds
  no user-visible capability.
- **Outcome entry** — `.odd/state.json` was written by `dcb2cdd` (before the implementation), given
  evidence by `ae0e76e`, corrected by `35e4302`; all `docs(...)`. No implementer commit touched it;
  only `evidence` ever changed after creation; `flow`, `scope`, `non_goals`, `id` and `status` are
  intact. Every criterion has a test or command (AC1 clippy + the forbid-unsafe policy step, AC2 the
  three steps, AC3-AC5 the unit tests, AC6 `cargo tree`).
- **Registration diffs** — unchanged this round: the V5 row and the manifests landed in `2271ab5`
  (+1 / +9 / +1 lines) and were not touched since.

## What this round did not do

- No `mutants` report (`mutation: off`), so `"mutation": null`; not a finding.
- No independent hand-mutant list and no per-commit exports: closure round at V1.
- W4 and S1 are stated above and left to the coordinator and the owner.

next: tdd-apply (E1 PASS; next delivery `E2`, which must carry W5 — the facade publish test with the
channel sink — and close it; W4 awaits the coordinator, S1 awaits the owner)

---

# Verify report — `gafira-engine` delivery `E2` (bus transport), round 1

Verdict: **FAIL** — 1 CRITICAL, 6 WARNING, 3 SUGGESTION.

```json
{ "schema": 1, "change": "gafira-engine", "delivery": "E2", "rounds": [ {
  "round": 1, "commit": "07c36f2", "mode": "full", "tier": "T1", "depth": "V1",
  "verdict": "FAIL", "next": "tdd-apply",
  "counts": {"critical": 1, "warning": 6, "suggestion": 3},
  "classes": {"implementation": 4, "architecture": 1, "ambiguity": 1, "claim": 1},
  "findings": [
    {"id": "E2-C1", "severity": "CRITICAL", "class": "implementation",
     "ref": "crates/gafira-engine/src/engine.rs:213-228 (Gafira::new); AC1",
     "summary": "The facade never proves it forwards the injected waker: Gafira::new passing NoopWaker instead survives all 14 tests. AC1's stated proof (assert the waker count) is not in the W5 test.",
     "follow_up": false, "status": "open"},
    {"id": "E2-W1", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-engine/src/event.rs:186-199",
     "summary": "Regression of closed E1 W2: ac4_the_noop_sink_accepts_every_variant again asserts nothing and omits LocaleChanged; it can only fail by not compiling.",
     "follow_up": false, "status": "open"},
    {"id": "E2-W2", "severity": "WARNING", "class": "implementation",
     "ref": "commit 642d14d",
     "summary": "Size: 489 insertions + 69 deletions = 558 changed lines against 320 forecast (448 at x1.4). Under the ~700 ceiling; coordinator and owner decide.",
     "follow_up": false, "status": "open"},
    {"id": "E2-W3", "severity": "WARNING", "class": "architecture",
     "ref": "crates/gafira-engine/src/engine.rs:213-228; 02 §5.2 line 1331, §5.4 line 1356; design P2",
     "summary": "Gafira::new creates the channel itself, but §5.2 has the composition root create it before the engine (startup screen, ARQ-68). The signature deviates from §5.4 and P2 with no design amend.",
     "follow_up": false, "status": "open"},
    {"id": "E2-W4", "severity": "WARNING", "class": "ambiguity",
     "ref": "design.md lines 80, 102, 218 vs apply-progress.md Slice card E2 non-goals",
     "summary": "The design gives E2 the UserError mapping (ARQ-102) and lists ARQ-68/101/102/112; the card says Product IDs none and defers the mapping. The design was not amended.",
     "follow_up": false, "status": "open"},
    {"id": "E2-W5", "severity": "WARNING", "class": "claim",
     "ref": ".odd/state.json, commit 07c36f2 (E1-AC2, E1-AC4, E1-AC5)",
     "summary": "The state commit rewrote E1's evidence to its pre-round-2 text: E1-AC2 again omits the ARQ-107 step (reopens E1 S2); E1-AC4/AC5 lose the count and ptr_eq detail.",
     "follow_up": false, "status": "open"},
    {"id": "E2-W6", "severity": "WARNING", "class": "implementation",
     "ref": "commit 962e95d",
     "summary": "The slice card commit also carries the post-implementation Notes (W5 closed, M1-M4 killed), and all three commits share one timestamp, so history cannot show the card preceded the code.",
     "follow_up": false, "status": "open"},
    {"id": "E2-S1", "severity": "SUGGESTION", "class": "implementation",
     "ref": "crates/gafira-engine/src/bus.rs:122-135",
     "summary": "flush resets the window even when nothing is pending, so a host flushing every frame delays the next immediate JobProgress. Still <= 10 Hz; worth a decision.", "status": "open"},
    {"id": "E2-S2", "severity": "SUGGESTION", "class": "implementation",
     "ref": "crates/gafira-engine/src/bus.rs:94-120",
     "summary": "send() runs outside the window lock, so with concurrent publishers a pending progress can leave before an earlier open one; the 'never backwards' comment holds only for one publisher.", "status": "open"},
    {"id": "E2-S3", "severity": "SUGGESTION", "class": "implementation",
     "ref": "commit 642d14d body; AC5",
     "summary": "AC5 says gate output in the commit notes; the code commit body is subject-only. Covered here by re-running the gates at HEAD (code identical to 642d14d).", "status": "open"}
  ],
  "gates": {"fmt": 0, "clippy": 0, "test": 0, "deny": 0, "ci_policy": 0},
  "mutation": null,
  "hand_mutants": {"run": 5, "killed": 4},
  "criteria": {"total": 6, "covered": 6},
  "size": {"lines": 558, "forecast": 448}, "report": "e2-verify-report.md" } ] }
```

`criteria.covered` counts the six ACs as having a test or command; E2-C1 is that AC1's proof is
weaker than the card itself demands. `hand_mutants` are extra at V1 (V1 owes none): the W5 check,
one spot-check from the recorded list (M3) and three of mine (X1-X3).

## Inputs

- Worktree `/tmp/gafira-engine-e2`, branch `feat/gafira-engine-e2`, HEAD `07c36f2`, base `develop` `0bc90a9`.
- Flow: `full | level: L3 | mutation: off | tier: hand mutants only | verify: V1`. No `mutants`
  run; none expected (`mutation: off`), not a finding.
- Acceptance source: Slice card E2 (`apply-progress.md`), design row E2, 02 §5.2/§5.4/§13.
- Commits: `962e95d` (card + design O4 note + E2 Notes), `642d14d` (code), `07c36f2` (state).

## Gates at HEAD `07c36f2`

`CARGO_TARGET_DIR` in the scratchpad (`e2-target`), `CARGO_INCREMENTAL=0`; disk 158 GB free.
Workspace scope (the commit touches the root `Cargo.toml` and `Cargo.lock`).

| Gate | Command | Exit |
|---|---|---|
| fmt | `cargo fmt --check` | 0 |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| test | `cargo test --workspace --locked` | 0 — 198 suites, 2514 passed, 0 failed, 0 ignored; `gafira-engine` unit: 14 passed, doctests 0 |
| deny | `cargo deny check` (targets: windows-msvc, linux-gnu, aarch64-darwin per `deny.toml`) | 0 — advisories, bans, licenses, sources ok |
| ci_policy | the eight `run` steps of the `policy` job other than the toolchain and testkit-ports steps | 0 each |

Policy steps run: Format, ARQ-07, forbid unsafe, V5, V12, ARQ-107 (`gafira-engine: layer 3,
exempt from the Locale search`), V13, V14 — all exit 0. Skipped: "Testkit ports feature compiles"
(E2 does not touch `gafira-testkit` and `crossbeam-channel` enters only the engine's tree). V5's
forbidden list does not include crossbeam; `cargo tree -p gafira-engine` adds
`crossbeam-channel v0.5.17` and `crossbeam-utils v0.8.23` only.

## W5 (E1 round 2) — CLOSED

On a scratch export of HEAD (`git archive`, touched, own target dir), `Gafira::publish`'s body
replaced by `let _ = event;`:

```
test engine::tests::w5_publishing_through_the_facade_reaches_the_observer ... FAILED
panicked at crates/gafira-engine/src/engine.rs:421:40
test result: FAILED. 13 passed; 1 failed
```

Restored: 14 passed. The worktree was never edited (`git status` clean).

## Criterion map

| AC | Proof | Status |
|---|---|---|
| AC1 publish reaches the observer through the channel | `engine::tests::w5_publishing_through_the_facade_reaches_the_observer` (kills M1) | covered, but the card's "assert the waker count" is missing — E2-C1 |
| AC2 waker once per delivery, never for a coalesced one | `bus::tests::ac2_the_waker_is_called_once_per_delivery`, waker counts in ac3/ac4 tests | covered (sink level only) |
| AC3 10 Hz coalescing, last one arrives | the three `bus::tests::ac3_*` (window, flush at exactly 100 ms, state event carries pending out, flush inside window) | covered |
| AC4 state events never dropped/coalesced | `bus::tests::ac4_ten_state_events_produce_ten_deliveries` | covered |
| AC5 docs-only commits skip cargo; code commit at workspace scope | gates re-run at workspace scope at HEAD (code tree identical to `642d14d`) | covered (E2-S3) |
| AC6 dependency allowed | `cargo deny check` exit 0 on the three targets; 02 §6 line 1801 lists crossbeam-channel VERIFICADA | covered |

Product IDs: the card declares none, so the per-ID `git grep` is vacuous. ARQ-68, ARQ-101,
ARQ-102 and ARQ-112 (design row) are context, not proved by E2: their acceptance tests need the
startup sequence, `SettingsService`, the error mapping and layer-2 `Progress` respectively. The
ARQ-102 deferral contradicts the design (E2-W4).

## Artifact integrity

- `962e95d` touches `apply-progress.md` and `design.md` (the O4 crossbeam correction). A design
  note in the same commit was accepted for E1 in full mode (`901e66c`); not a finding by itself.
  It also contains the `## Notes — E2` section with results that only exist after the code
  (E2-W6).
- No implementer commit touched the card: `642d14d` changes only code and manifests; `07c36f2`
  only `.odd/state.json`.
- `07c36f2` (coordinator) adds `E2-AC1`…`E2-AC6` and rewrites the E1 evidence strings (E2-W5).

## Hand mutants (scratch export, `cargo test -p gafira-engine --locked`)

| # | Mutant | Result |
|---|---|---|
| M1 (W5) | `Gafira::publish` body `let _ = event;` | killed by `w5_…` |
| M3 (spot-check of recorded list) | `window_is_open` returns `true` | killed by `ac2_…`, `ac3_job_progress_…`, `ac3_a_flush_inside_…` |
| X1 | window boundary `>=` -> `>` | killed by `ac3_job_progress_coalesces_…` |
| X2 | `flush` ignores the window | killed by `ac3_a_flush_inside_the_window_delivers_nothing` |
| X3 | `Gafira::new` builds the channel with `NoopWaker`, ignoring the injected waker | **survived** (14 passed) — E2-C1 |

Every mutant was reverted (`cmp` against the saved original); final run on the export 14 passed.

## Findings

### CRITICAL

- **E2-C1** — class: implementation. `Gafira::new` (`engine.rs:213-228`) is the only place the
  injected waker reaches the bus, and no test watches it: X3 survives. The card's AC1 proof reads
  "publish through the facade, receive the event from `events()`, assert the waker count", and the
  W5 test passes `NoopWaker`. In the product the UI would never repaint (02 §5.2 line 1335), and
  that is a spec rule with an invariant (AGENTS.md critical code). Fix: a counting waker in the W5
  test, plus an assertion that its count is 1.

### WARNING

- **E2-W1** — class: implementation. `event.rs:186-199` brings back exactly what E1 W2 closed: the
  "every variant" test publishes 5 of 6 variants (no `LocaleChanged`) to `EventBus::noop()` and
  asserts nothing. E1-AC4 still has a killing test (`ac4_publish_reaches_the_sink_exactly_once`),
  so this is not CRITICAL, but restore the counted every-variant list (now with `JobProgress`).
- **E2-W2** — class: implementation. Size 558 changed lines (`git show --shortstat 642d14d`: 7
  files, 489+, 69-) against 448 forecast; `bus.rs` is 322 of them (about half are tests).
- **E2-W3** — class: architecture. §5.2 (line 1331): "the composition root creates the channel
  **before** the engine, gives the receiver to the startup screen and the sender to the sequence".
  `Gafira::new` creates the channel internally and only takes a waker, so the startup sequence
  (ARQ-68) cannot publish before the engine exists, or into the same queue. `bus.rs:27-29` even says
  the composition root creates the channel first. The signature also departs from §5.4 (1356) and
  from P2 ("without changing this signature"). That is recorded only in a doc comment. Decide
  whether `new` takes an `EventBus` (or a sender) from the root, and amend P2 before callers
  multiply.
- **E2-W4** — class: ambiguity. Design lines 80 and 218 and row E2 (line 102) put the `UserError`
  mapping (ARQ-102) in E2. The card makes it a non-goal ("no `UserError` mapping beyond what `E1`
  shipped") and E2 has no failing service. Move the mapping to the delivery of the first failing
  service in the design.
- **E2-W5** — class: claim. `07c36f2` swaps the round-2 E1 evidence for older text: E1-AC2 is
  "V5 tree check and cmp of clippy.toml" again, which E1 S2 had corrected to include the ARQ-107
  step. E1-AC5 drops "Arc::ptr_eq on the six injected ports". Restore the develop wording.
- **E2-W6** — class: implementation (commit hygiene). The card commit `962e95d` contains the
  `## Notes — E2` block ("W5 is closed", M1-M4 with killing tests), which can only be written after
  `642d14d`. All three commits carry `2026-11-06 11:28:43`. The card was not changed by an
  implementer commit, but history cannot show it was fixed before the code. Keep the card and the
  implementer's notes in separate commits, in order.

### SUGGESTION

- **E2-S1** — `bus.rs:122-135`: when the window is open, `flush` sets `last_progress = now` even
  with nothing pending. A host that flushes every frame closes the window without delivering, so
  the next `JobProgress` waits for a flush instead of going out at once. The rate stays at most
  10 Hz.
- **E2-S2** — `bus.rs:94-120`: the window check and `send` take the lock separately. With two
  publishers, a pending progress can be sent (by a state event or a flush) before an earlier "open"
  progress, so the observer can see the fraction go backwards. Fine for one publisher; document it
  or send under the lock.
- **E2-S3** — AC5 says "gate output in the commit notes", but the body of `642d14d` is just the
  subject line. Covered here by the HEAD re-run.

## Build hygiene

Target dirs `e2-target` and `e2-export-target` under the scratchpad, freed with
`cargo clean --target-dir`. The worktree was never modified.

next: tdd-apply (implementation: E2-C1, E2-W1, E2-W2, E2-W6; non-blocking WARNINGs for the coordinator to route: architecture E2-W3 -> architect, ambiguity E2-W4 -> specifier/design amend, claim E2-W5 -> restore the E1 evidence in `.odd/state.json`)
