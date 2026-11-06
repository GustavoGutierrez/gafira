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

---

# Verify report — `gafira-engine` delivery `E2` (bus transport), round 2 (closure)

Verdict: **FAIL** — 1 CRITICAL, 2 WARNING, 0 SUGGESTION.

Round-1 findings: E2-C1, E2-W1, E2-W3, E2-W4, E2-W5 and E2-W6 are closed. E2-W2 (size) is still open
and is now worse. One new CRITICAL (E2-C2) comes from the probe the coordinator asked for, and one
new follow-up WARNING (E2-W7).

```json
{ "schema": 1, "change": "gafira-engine", "delivery": "E2", "rounds": [ {
  "round": 2, "commit": "e340de1", "mode": "full", "tier": "T1", "depth": "V1",
  "verdict": "FAIL", "next": "tdd-apply",
  "counts": {"critical": 1, "warning": 2, "suggestion": 0},
  "classes": {"implementation": 2},
  "findings": [
    {"id": "E2-C1", "severity": "CRITICAL", "class": "implementation",
     "ref": "crates/gafira-engine/src/engine.rs:217-230, :433-446",
     "summary": "Closed: the facade test builds the channel with a counting waker and asserts one wake; HM1 (facade ignores the injected waker) and HM1b (sink never wakes) both fail it.",
     "follow_up": false, "status": "closed"},
    {"id": "E2-W1", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-engine/src/event.rs ac4_every_variant_reaches_the_sink_once",
     "summary": "Closed: all six variants, LocaleChanged and JobProgress included, one count each and a total of six.",
     "follow_up": false, "status": "closed"},
    {"id": "E2-W2", "severity": "WARNING", "class": "implementation",
     "ref": "git diff --shortstat 962e95d e340de1 -- crates Cargo.toml Cargo.lock",
     "summary": "Size: E2 now has 762 changed lines (697+, 65-), above the ~700 ceiling. The fix round alone has 402 against a forecast of ~120 (168 at x1.4). This needs the owner's explicit OK.",
     "follow_up": false, "status": "open"},
    {"id": "E2-W3", "severity": "WARNING", "class": "architecture",
     "ref": "bus.rs:26-68 EventChannel; engine.rs:217-230; design P11",
     "summary": "Closed: the root builds EventChannel and Gafira::new consumes it; p11_an_event_published_before_the_engine_exists_is_drained_first proves it.",
     "follow_up": false, "status": "closed"},
    {"id": "E2-W4", "severity": "WARNING", "class": "ambiguity",
     "ref": "design P12, design.md:105,142; engine.rs:156-158",
     "summary": "Closed: P12 moves ToMessage to E3b and UserError to E11; the EngineError doc points to P12; no mapping code.",
     "follow_up": false, "status": "closed"},
    {"id": "E2-W5", "severity": "WARNING", "class": "claim",
     "ref": ".odd/state.json E1-AC1..E1-AC6 (6a49e48)",
     "summary": "Closed (coordinator): the six E1 criteria are field-for-field equal to develop's round-2 E1 text (ARQ-107 step in E1-AC2, ptr_eq in E1-AC5).",
     "follow_up": false, "status": "closed"},
    {"id": "E2-W6", "severity": "WARNING", "class": "implementation",
     "ref": "commits 91084a1 -> e340de1",
     "summary": "Closed: the fix-round notes are in their own docs commit, after the code and append-only; the card is untouched. Round-1 history stays as recorded.",
     "follow_up": false, "status": "closed"},
    {"id": "E2-C2", "severity": "CRITICAL", "class": "implementation",
     "ref": "crates/gafira-engine/src/bus.rs:144-148 (and the claim at :155-156)",
     "summary": "A progress that opens the window does not drop the older pending one, so a stale fraction arrives after a newer one (0.1, 0.9, 0.2). The observer ends on 0.2. Breaks AC3 'the last one' and the code's own 'never backwards' claim.",
     "follow_up": false, "status": "open"},
    {"id": "E2-W7", "severity": "WARNING", "class": "ambiguity",
     "ref": "02 §5.2:1321,1335; card AC3; bus.rs:72-75 (one pending slot)",
     "summary": "The coalescing key is unspecified. One pending slot is shared across JobIds, so job 1 overwrites job 2's pending progress and job 2's last fraction never arrives. Is coalescing per job?",
     "follow_up": true, "status": "open"}
  ],
  "gates": {"fmt": 0, "clippy": 0, "test": 0, "deny": 0, "ci_policy": 0},
  "mutation": null,
  "hand_mutants": {"run": 2, "killed": 2},
  "criteria": {"total": 6, "covered": 5},
  "size": {"lines": 762, "forecast": 448}, "report": "e2-verify-report-r2.md" } ] }
```

`criteria.covered` is 5 because AC3 has tests, but the probe shows the code violates it (E2-C2).
`hand_mutants` lists HM1 (the requested re-application) and HM1b. The probes are reported
separately and are not counted as mutants.

## Inputs

- Worktree `/tmp/gafira-engine-e2`, branch `feat/gafira-engine-e2`, HEAD `e340de1`, merge base with
  `develop` `90603a6`.
- Flow: `full | level: L3 | pack: new capability | mutation: off | tier: hand mutants only | verify:
  V1 | decided-by: owner`. Nothing ran mutants and nothing was expected to (`mutation: null`), so
  that is not a finding.
- Acceptance source: E2 Slice card and "E2 fix round" (`apply-progress.md`), design P11, P12 and
  the fix-round table F1-F6 (`ad8f058`), 02 §5.2 (1321, 1331, 1335) and §5.4 (1356).
- Closure round: it ran the gates, the artifact integrity checks and the closure of the round-1
  ids. It spent no independent-mutant budget. The only probe is the one the coordinator asked for
  (the implementer's observation).

## Gates at HEAD `e340de1` (workspace scope)

`df -h /`: 163 GB free. `CARGO_TARGET_DIR` under the scratchpad (`e2r2-target`),
`CARGO_INCREMENTAL=0`.

| Gate | Command | Exit |
|---|---|---|
| fmt | `cargo fmt --check` | 0 |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| test | `cargo test --workspace --locked` | 0. 198 suites, 2520 passed, 0 failed, 0 ignored. `gafira-engine` lib: 20 passed |
| deny | `cargo deny check` | 0 (advisories, bans, licenses, sources ok). No manifest changed in the fix round |
| ci_policy | the `run` steps of the `policy` job: Format, ARQ-07, forbid unsafe, V5, V12, ARQ-107, V13, V14 | 0 each. ARQ-107 prints `gafira-engine: layer 3, exempt from the Locale search` |

Skipped: "Testkit ports feature compiles". E2 does not touch `gafira-testkit`, as in round 1.

## Artifact integrity (steps 3, 3c, 3d)

- The Slice card has not changed since `962e95d`. `git diff 962e95d e340de1 -- apply-progress.md`
  has zero removed lines. `e340de1` is one append-only hunk (`## E2 fix round`) after `## Notes — E2`.
- The implementer's code commits `7c1df86` and `91084a1` touch only `crates/gafira-engine/src/*`.
  No implementer commit touches `.odd/state.json`, `design.md` or the card.
- The coordinator commits `6a49e48` and `779b8ed` touch only `.odd/state.json`, and `b5111f6` only
  `verify-report.md`. The architect commit `ad8f058` touches only `design.md`. In the outcome
  entry, only evidence, verify and `next_action` changed. Criterion ids and texts are intact.
- E2-W5: for the six `E1-AC*` objects of `.odd/state.json`, a JSON comparison with `develop:.odd/state.json` shows them all
  equal. The differences are whitespace only.

## Closure of the round-1 findings

| Id | Evidence | Status |
|---|---|---|
| E2-C1 | See the hand mutants below. `w5_…_and_wakes_it` asserts `wakes == 1` (engine.rs:444) | closed |
| E2-W1 | `event.rs` `ac4_every_variant_reaches_the_sink_once`: 6 variants, a count per publish, total `VARIANTS = 6`. It counts on a `CountingSink`, not `NoopSink`, which is a deviation the implementer documented and is acceptable because a `NoopSink` cannot be counted | closed |
| E2-W2 | 762 changed lines for the delivery, above the ~700 ceiling. The fix round: `7c1df86` 111+/33-, `91084a1` 196+/62- = 402 lines against ~120 forecast. The design's size note records only "558 before the fix round" | **open** (owner OK needed) |
| E2-W3 | `EventChannel::{new, bus, receiver}`, `EventBus: Clone`, `Gafira::new(.., events: EventChannel)`, the bus.rs header cites P11, and the `p11_…` test drains the pre-engine event first, in order | closed |
| E2-W4 | P12 in the design. `EngineError` doc (engine.rs:156-158) points to `E3b` and `E11`. No mapping code | closed |
| E2-W5 | see artifact integrity | closed (coordinator) |
| E2-W6 | notes in `e340de1`, after `91084a1` and append-only. The card is not edited | closed |

The round-1 suggestions were also checked. S1 is fixed and proved by
`s1_an_idle_flush_does_not_hold_back_the_next_progress`. S2 is fixed: the window decision and
`send` happen under one lock and the waker runs after the lock is released, proved by
`s2_two_bus_clones_…` and `s2_the_waker_is_called_after_the_lock_is_released`. S3 is not needed
because this round re-ran the gates.

## Hand mutants (scratch export of `e340de1`, own target dir, `cargo test -p gafira-engine --locked --lib`)

| # | Mutant | Result |
|---|---|---|
| HM1 (requested; implementer's #1) | `Gafira::new` discards the given channel and builds `EventChannel::new(Arc::new(NoopWaker), ports.clock)` | **killed**: `w5_…_and_wakes_it` (engine.rs:446) and `p11_…` (engine.rs:461) fail, 18 passed / 2 failed |
| HM1b (waker-specific) | `ChannelSink::deliver` never calls `wake()` (events are still delivered) | **killed**: `w5_…_and_wakes_it` fails at the wake-count assertion (engine.rs:444), plus 5 bus tests |

Both files were restored and checked with `cmp` against the worktree. The final run on the export
passed 20 tests.

## The implementer's observation: judged

> "a progress that opens the window does not clear an older pending one, so a stale fraction can
> still leave later"

The observation is real. The defect has been in the code since round 1 (`642d14d`, same branch
logic), and round 1 missed it. The probe below was a test added on the export only:

```
t=0    publish p(0.1)   -> window open, delivered
t=50   publish p(0.2)   -> inside window, pending = p(0.2)
t=150  publish p(0.9)   -> window open, delivered; pending still p(0.2)
       publish DocumentChanged  (or: t=250 flush)
seen = [p(0.1), p(0.9), p(0.2), DocumentChanged]      // flush variant: [p(0.1), p(0.9), p(0.2)]
```

Both probes fail. The observer's last `JobProgress` is 0.2, but the latest published is 0.9.

**Decision: this violates AC3 and the rule §5.2 sets for the coalescing. The finding is CRITICAL,
class `implementation` (E2-C2).**

- AC3 says "`JobProgress` is coalesced to 10 Hz and the last one always arrives". The intent of
  coalescing, as stated in the module header (bus.rs:6, "keeps only the latest fraction of the
  window"), is that what the observer ends on is the latest fraction. Here a superseded fraction
  arrives after the newest one, so the value the UI keeps is stale. When `JobFinished` lands in
  `E7` it is a state event, so it would carry the stale fraction out right before itself.
- bus.rs:155-156 states "so the observer never sees progress go backwards". That is false as soon
  as a window-opening progress meets an older pending one, and the design marks "the coalescing
  window" and "the ordering of the drain" as critical code for E2.
- It is a code defect, not only a test gap. Per the closure-round rule, a probe that shows a real
  code defect stays CRITICAL. The fix is a few lines. When a progress opens the window, it
  supersedes the pending one: clear `pending` (it is older) before sending. The RED test is the
  first probe above, plus a hand mutant that leaves `pending` in place.

## New finding details

- **E2-C2**, class: implementation, critical code. Location: `bus.rs:144-148`, the window-open
  branch of `ChannelSink::publish`. It sends the new progress and leaves `window.pending` untouched.
  Effect: a stale fraction arrives after a newer one, through the next state event or flush.
  Expected: after `p(0.9)` opens the window, nothing older than `p(0.9)` leaves.
- **E2-W7**, class: ambiguity, follow-up. `Window` has one `pending` slot for all jobs. Probe:
  `p(job1,0.1)`, `p(job2,0.5)`, `p(job1,0.4)`, then wait 100 ms and flush. The result is
  `[p(1,0.1), p(1,0.4)]`, and job 2's last progress never arrives. §5.2 (1335) says only "10 Hz
  coalescing of `JobProgress`". `JobProgress` carries a `JobId` (1321) and `EngineConfig` allows
  more than one export job, but neither the spec, the card nor the design says whether the window
  and the "last one" are per job. This does not block E2 today, because only one producer exists.
  The architect or specifier should settle it before `E7` or `ExportService` produces progress
  for parallel jobs. If the key is decided before the C2 fix, C2 can be fixed per job in the same
  change.

## Size (E2-W2)

`git diff --shortstat 962e95d e340de1 -- crates Cargo.toml Cargo.lock`: 7 files, 697+, 65-, which
is 762 changed lines. The forecast was 320 (448 at x1.4). The ceiling is ~700 (AGENTS.md:99, :162),
so the owner's explicit OK is needed. The fix round was 402 lines against ~120.

## Build hygiene

The target dirs `e2r2-target` (6.4 GiB) and `e2r2-export-target` were freed with
`cargo clean --target-dir`. The export `e2r2-export` stays in the scratchpad with its sources
restored, because `rm -rf` is denied. The worktree was never modified: `git status` is clean and
HEAD is `e340de1`.

next: tdd-apply (implementation: E2-C2, round 3 closure; E2-W2 size needs the owner's OK via the
coordinator; E2-W7 ambiguity is a follow-up for the architect/specifier and does not block)

---

# Verify report — `gafira-engine` delivery `E2` (bus transport), round 3 (closure)

Verdict: **PASS** — 0 CRITICAL, 2 WARNING (both follow-up), 0 SUGGESTION.

E2-C2 is closed by `12ea73c`. The round-2 closures (E2-C1, W1, W3, W4, W5, W6) still hold. E2-W2
was closed by the owner at 762 lines. One new follow-up WARNING (E2-W8): the fix round adds 44
lines, so the delivery is now 802 lines, above the 762 the owner accepted. E2-W7 stays a follow-up.

```json
{ "schema": 1, "change": "gafira-engine", "delivery": "E2", "rounds": [ {
  "round": 3, "commit": "cbe874e", "mode": "full", "tier": "T1", "depth": "V1",
  "verdict": "PASS", "next": "tdd-apply",
  "counts": {"critical": 0, "warning": 2, "suggestion": 0},
  "classes": {},
  "findings": [
    {"id": "E2-C2", "severity": "CRITICAL", "class": "implementation",
     "ref": "crates/gafira-engine/src/bus.rs:145-149",
     "summary": "Closed: a window-opening progress clears pending before sending; two new AC3 tests and the round-2 probes pass; hand mutant (a) fails both new tests.",
     "follow_up": false, "status": "closed"},
    {"id": "E2-C1", "severity": "CRITICAL", "class": "implementation", "ref": "engine.rs:433-446",
     "summary": "Closed (unchanged since round 2).", "follow_up": false, "status": "closed"},
    {"id": "E2-W1", "severity": "WARNING", "class": "implementation", "ref": "event.rs:195-226",
     "summary": "Closed (unchanged since round 2).", "follow_up": false, "status": "closed"},
    {"id": "E2-W2", "severity": "WARNING", "class": "implementation", "ref": "93c0be8",
     "summary": "Closed: owner accepted 762 lines on 2026-11-06.", "follow_up": false, "status": "closed"},
    {"id": "E2-W3", "severity": "WARNING", "class": "architecture", "ref": "bus.rs:33 EventChannel; engine.rs:217; design P11",
     "summary": "Closed (unchanged since round 2).", "follow_up": false, "status": "closed"},
    {"id": "E2-W4", "severity": "WARNING", "class": "ambiguity", "ref": "engine.rs:157-158; design P12",
     "summary": "Closed (unchanged since round 2).", "follow_up": false, "status": "closed"},
    {"id": "E2-W5", "severity": "WARNING", "class": "claim", "ref": ".odd/state.json E1-AC1..6",
     "summary": "Closed (coordinator, round 2).", "follow_up": false, "status": "closed"},
    {"id": "E2-W6", "severity": "WARNING", "class": "implementation", "ref": "12ea73c -> cbe874e",
     "summary": "Closed: still holds; the round-3 notes are a separate docs commit, after the code and append-only.", "follow_up": false, "status": "closed"},
    {"id": "E2-W7", "severity": "WARNING", "class": "ambiguity",
     "ref": "02 §5.2:1321,1335; card AC3; bus.rs:72-75 (one pending slot)",
     "summary": "The coalescing key is unspecified: one pending slot is shared across JobIds. Is coalescing per job? Settle it before E7 or ExportService.",
     "follow_up": true, "status": "open"},
    {"id": "E2-W8", "severity": "WARNING", "class": "implementation",
     "ref": "git diff --shortstat 962e95d cbe874e -- crates Cargo.toml Cargo.lock",
     "summary": "Size: E2 is now 802 changed lines (737+, 65-), 40 above the 762 the owner accepted. The fix is 44 lines (37 of them tests). Record it in the design table and tell the owner.",
     "follow_up": true, "status": "open"}
  ],
  "gates": {"fmt": 0, "clippy": 0, "test": 0, "deny": 0, "ci_policy": 0},
  "mutation": null,
  "hand_mutants": {"run": 1, "killed": 1},
  "criteria": {"total": 6, "covered": 6},
  "size": {"lines": 802, "forecast": 448}, "report": "e2-verify-report-r3.md" } ] }
```

## Inputs

- Worktree `/tmp/gafira-engine-e2`, branch `feat/gafira-engine-e2`, HEAD `cbe874e` (fix `12ea73c`,
  notes `cbe874e`).
- Flow: `full | L3 | new capability | mutation: off | hand mutants only | V1 | owner`. There was no
  `mutants` run and none was expected, so `mutation: null` is not a finding.
- This is a closure round. It ran the gates, the artifact checks, the closure of E2-C2 with the
  probes from round 2 plus one stream probe, and the requested hand mutant (a). It spent no
  independent-mutant budget.

## Gates at HEAD `cbe874e` (workspace scope)

`df -h /`: 163 GB free. Target dir `e2r3-target` under the scratchpad, `CARGO_INCREMENTAL=0`.

| Gate | Command | Exit |
|---|---|---|
| fmt | `cargo fmt --check` | 0 |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| test | `cargo test --workspace --locked` | 0. 198 suites, 2522 passed (round 2 had 2520; +2 new), 0 failed, 0 ignored. `gafira-engine` lib: 22 passed |
| deny | `cargo deny check` | 0 (advisories, bans, licenses, sources ok). No manifest changed since `e340de1` |
| ci_policy | `run` steps of the `policy` job, extracted from `ci.yml`: Format, ARQ-07, forbid unsafe, V5, V12, ARQ-107, V13, V14 | 0 each. ARQ-107: `gafira-engine: layer 3, exempt from the Locale search` |

Skipped: "Testkit ports feature compiles (ADR 0013)". E2 does not touch `gafira-testkit` (same as
rounds 1 and 2).

## Artifact integrity (steps 3, 3c, 3d)

- `12ea73c` touches only `crates/gafira-engine/src/bus.rs`. `cbe874e` touches only
  `apply-progress.md`, as one appended section (`## E2 fix round 2`).
- The Slice card is intact: `git diff 962e95d cbe874e -- apply-progress.md` has 0 removed lines.
- Commits since round 2 that touch `.odd/state.json` or `design.md`: only the coordinator's
  `273c3e6` and `93c0be8`. `93c0be8` changes only `status`, `next_action` and `verify` (E2-W2
  closed with the owner's OK). The criterion ids, texts and types are unchanged. No implementer
  commit touched the outcome entry, the design or the card.

## E2-C2: closure

The fix is at `bus.rs:145-149`. When a `JobProgress` opens the window, the code sets
`window.last_progress = Some(now)` and `window.pending = None`, then calls `send`. The comment on
the state-event path (155-158) now matches the behaviour.

**Do the two new tests encode AC3 correctly?** Yes. Both publish this sequence with the injected
clock:

```
t=0     p(0.1)  window open  -> delivered
t=0     p(0.9)  inside       -> pending = 0.9
t=100   p(0.2)  window open  -> delivered; with the fix, pending 0.9 is discarded
then    DocumentChanged      (test 1)    or   t=200 flush   (test 2)
expected fractions: [0.1, 0.2]
```

- "Newest" means the last one published. That is 0.2. The test lists the fractions in falling
  order on purpose, so arrival order can be told apart from size. The observer must end on 0.2,
  and 0.9 is older than 0.2 (published earlier), so it must never arrive after 0.2. `[0.1, 0.2]`
  satisfies both parts of AC3: the observer ends on the newest fraction and nothing older arrives
  after something newer. The expected value is derived from AC3, not fitted to the fix.
- The RED output `left [0.1, 0.2, 0.9]` is exactly the pre-fix behaviour for this sequence: the
  stale 0.9 leaves on the state event, or on the flush at t=200 when the window is open. The record
  is consistent.
- The alternative order, sending the pending 0.9 before 0.2 (`[0.1, 0.9, 0.2]`), would also end
  on the newest fraction. But it delivers two progress events at t=100, inside one window, which
  breaks the 10 Hz half of AC3. So discarding the pending one, which the test requires, is the
  correct choice, not an over-specification.
- Limit: `fractions` filters out non-progress events, so the new tests do not assert that the
  `DocumentChanged` itself arrives. AC4 already covers that
  (`ac4_ten_state_events_produce_ten_deliveries`). Not a finding.

**Probes** (scratch export of `cbe874e`, own target dir, tests added to the export only):

| Probe | Sequence | Result |
|---|---|---|
| round-2 probe, state-event variant | t0 p(0.1), t50 p(0.2), t150 p(0.9), DocumentChanged; expect `[0.1, 0.9]` | pass |
| round-2 probe, flush variant | same, then t250 flush; expect `[0.1, 0.9]` | pass |
| round-3 stream probe | 15 rising fractions at irregular steps (0-200 ms). Ends with a flush after one window, then again with a state event. Asserts strictly increasing arrivals and that the last one equals the last published | pass (both endings) |

**Hand mutant (a)** (the implementer's #a, requested): remove `window.pending = None;`. Result:
**killed**. 5 tests failed, 20 passed: both `ac3_a_new_window_…` tests (`left [0.1, 0.2, 0.9]`,
`right [0.1, 0.2]`) and all three probes (`left [0.1, 0.9, 0.2]`). The export file was then
restored and matched the worktree with `cmp`.

## Round-2 closures re-checked

`git diff --stat e340de1 cbe874e -- crates/gafira-engine/src` changes only `bus.rs`, and only by
the fix and the two tests. `engine.rs` and `event.rs` are byte-identical to round 2, so the
following still hold:

- E2-C1: `w5_…_and_wakes_it` asserts one wake (engine.rs:444).
- E2-W1: `ac4_every_variant_reaches_the_sink_once`, `VARIANTS = 6` (event.rs:195-226).
- E2-W3: `EventChannel` (bus.rs:33), `Gafira::new` takes it (engine.rs:217), and `p11_…` passes.
- E2-W4: the `EngineError` doc points to `E3b`/`E11`/P12 (engine.rs:157-158).
- E2-W6: still holds for this round's commits.

## Criterion map (AC1-AC6)

| AC | Proof | Status |
|---|---|---|
| AC1 | `w5_publishing_through_the_facade_reaches_the_observer_and_wakes_it`, `p11_…` | covered |
| AC2 | `ac2_the_waker_is_called_once_per_delivery`, `s2_the_waker_is_called_after_the_lock_is_released`, wake assertion in `w5_…` | covered |
| AC3 | `ac3_job_progress_coalesces_…`, `ac3_a_state_event_flushes_…`, `ac3_a_flush_inside_the_window_…`, the two new `ac3_a_new_window_…` tests, `s1_…`, `s2_two_bus_clones_…` | covered (the E2-C2 defect is fixed) |
| AC4 | `ac4_ten_state_events_produce_ten_deliveries`, `ac4_every_variant_reaches_the_sink_once` | covered |
| AC5 | gate output in the commit notes (`cbe874e`) and this round's workspace gates | covered |
| AC6 | `cargo deny check` exits 0; manifests unchanged | covered |

Product IDs: none (card). Nothing was left out.

## Findings

### WARNING

- **E2-W7**, class: ambiguity, follow-up, open. Carried over from round 2 and not routed. The
  coalescing key is unspecified: one `pending` slot is shared by all `JobId`s. The architect or
  specifier should settle this before `E7` or `ExportService` produces progress for parallel jobs.
- **E2-W8**, class: implementation, follow-up, open (new). `git diff --shortstat 962e95d cbe874e --
  crates Cargo.toml Cargo.lock` gives 7 files, 737+ and 65-, which is 802 changed lines. The owner
  accepted 762 (`93c0be8`), and this round adds 44 lines (42+, 2-), 37 of them tests. The amount
  is small and it is the fix the owner authorised. But the measured size now exceeds the number
  the owner accepted, so the coordinator should record 802 in the design's delivery table and
  mention it to the owner. It does not block the verdict.

## Build hygiene

The target dirs `e2r3-target` (6.4 GiB) and `e2r3-export-target` (174 MiB) were freed with
`cargo clean --target-dir`. The export `e2r3-export` stays in the scratchpad with `bus.rs`
restored, because `rm -rf` is denied. The worktree was never modified: `git status` is clean and
HEAD is `cbe874e`.

next: tdd-apply (E2 PASS: merge `feat/gafira-engine-e2` into `develop`, then the next delivery;
follow-ups E2-W7 (ambiguity, for the architect/specifier before E7) and E2-W8 (record 802 lines,
tell the owner))

---

# Verify report — `gafira-engine` delivery `E3a` (history dirty sets), round 1

round: 1 | commit: 778e494 | base: cad7d0c | mode: full | level: L3 | tier: hand mutants only | depth: V1 | mutation: off

**Verdict: PASS** (delivery; deliveries `E3b` to `E3d` and `E5` remain)

| id | severity | class | ref | summary | follow-up | status |
|---|---|---|---|---|---|---|
| E3a-S1 | SUGGESTION | - | crates/gafira-edit/tests/dirty_sets.rs:103-118 | AC4 proves the merge by `len() == 3`; the set assertion alone cannot tell a merged entry from an unmerged one, because `SetProperty` merges only on an equal `MergeKey` (same page). Keep the `len` assertion when this test is edited. | no | open |
| E3a-S2 | SUGGESTION | - | crates/gafira-edit/tests/dirty_sets.rs:141-153 | AC6 covers a failure before any mutation (index check is the first line of `InsertPage::apply`). A command failing after a partial write on the copy would add a second case; not required by the card. | no | open |

Counts: critical 0, warning 0, suggestion 2. Per class: none (suggestions carry no class).

## Gates at HEAD (`778e494`, workspace scope, scratch `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`)

| gate | exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (199 suites, 2529 passed, 0 failed, 0 ignored; matches the notes) |
| CI policy: ARQ-07, forbid unsafe, V5, V12, ARQ-107, V13 (EditToken, incl. positive control and re-export check), V14 | 0 each |
| CI policy: testkit `ports` feature (clippy + test) | 0 |
| `cargo deny check` | 0 |

Free disk before builds: 147 GB. Both target dirs freed with `cargo clean`; worktree clean at `778e494`. The source-only scratch export `e3a-export` stays in the scratchpad (`rm -rf` is denied).

## Artifact integrity (step 3c)

- `cad7d0c docs(gafira-engine): slice card for E3a` touches only `specs/changes/gafira-engine/apply-progress.md` (+42).
- `d1d6641` touches only `crates/gafira-edit/src/history.rs` and `crates/gafira-edit/tests/dirty_sets.rs`.
- `778e494` appends the notes after the card (+36, pure addition after the card's last line); the card block is unchanged since `cad7d0c`.
- Public API diff of `gafira-edit` (`git diff cad7d0c HEAD -- crates/`, `pub` lines): only `History::newest_touches` and `History::undone_touches`, both `&self`, read-only. The other change is a private `use` of `DirtySet` (already public). No `execute`/`undo`/`redo` signature or `Command` impl changed (card non-goals hold).

## Criteria map

| AC | Proof | Status |
|---|---|---|
| AC1 | `ac1_after_a_changed_execute_the_newest_set_is_the_applied_one`: `InsertPage` (set empty before `apply`, XP4), full pair compared | covered |
| AC2 | `ac2_undo_reports_the_undone_entry_and_the_new_top`: two distinct entries, then down to `(None, Some(first))` | covered |
| AC3 | `ac3_redo_reports_the_redone_entry_and_the_next_redo_entry`: three redos, each pair compared, ends `(Some, None)` | covered |
| AC4 | `ac4_a_merged_gesture_reports_the_merged_entry` | covered (see doubt 1) |
| AC5 | `ac5_an_empty_history_reports_nothing`, `ac5_an_unchanged_execute_leaves_both_sets` (asserts `Ok(Unchanged)`) | covered |
| AC6 | `ac6_a_command_that_fails_to_apply_leaves_both_sets` | covered (see doubt 2) |
| AC7 | gates above, workspace scope | covered |

Product IDs: 02 §5.1, §4.3, ARQ-30 (card). Every test in `dirty_sets.rs` carries `// ARQ-30` (7 tests, `git grep ARQ-30`). ARQ-33 is not a card ID; its "one entry per gesture" rule is what AC4 relies on and is exercised there. None left out.

## Coordinator doubts

1. **AC4 merges.** The test opens one gesture and executes three `SetProperty::page(second, PageName(..), Some(gesture))` at 1 s, 2 s and 3 s, each `Ok(Changed)`. `History::len()` is `self.undo.len()` (history.rs:194); after two `InsertPage` entries, `len() == 3` holds only if the three renames became one entry. The 1 s gaps exceed `MERGE_WINDOW` (500 ms), so the merge comes from the gesture branch of `merge_into_newest` (`old.gesture == open`), not the time window. `SetProperty` is the merging command (`merge_key` returns `Some`; set_property.rs:433). So it is a merged entry, and `newest_touches()` on it is asserted as `(Some({second}), None)`. Settled: AC4 is covered. Limit noted in E3a-S1.
2. **AC6 failure path.** `InsertPage::new(9, ..)` on a one-page document: `InsertPage::apply` returns `IndexOutOfRange { index: 9, len: 1 }` (pages.rs:52); `History::execute` calls `apply_to_copy(..)?` first (history.rs:99), so the error returns before `redo.clear()` and before any push or merge. The test asserts that exact `Err` and then the full pair `(Some({first}), Some({second}))` over a history with both stacks non-empty, so both sets are proved unchanged, including the redo side a wrong `redo.clear()` would empty. Settled: real `apply` failure, AC6 covered.
3. **Hand mutant 1 re-applied** (scratch export of `778e494`): `newest_touches` reads `self.redo.last()`, `undone_touches` reads `self.undo.last()`. `cargo test -p gafira-edit --locked --test dirty_sets`: 6 failed, 1 passed (`ac5_an_empty_history_reports_nothing`, as the notes say). E.g. `ac1`: left `(None, Some({PageId(1)}))`, right `(Some({PageId(1)}), None)`. Killed, matches the record.
4. **API diff**: only the two additive methods (above).

## Mutation

`mutation: off`: no `mutants` report; not a finding. Hand mutants: 1 run (spot-check of recorded mutant 1), 1 killed. V1 adds no independent hand mutants.

## Size

`git diff --shortstat cad7d0c..d1d6641`: 2 files, 171 insertions, 1 deletion (code 19, tests 153). Delivery total to `778e494`: 207 insertions, 1 deletion. Forecast 90 (126 at x1.4); 171 is x1.9 of the raw forecast, inside the x1.4 to x1.9 range AGENTS.md records and far below ~700.

```json
{ "schema": 1, "change": "gafira-engine", "delivery": "E3a", "rounds": [ {
  "round": 1, "commit": "778e494", "mode": "full", "tier": "hand mutants only", "depth": "V1",
  "verdict": "PASS", "next": "tdd-apply",
  "counts": {"critical": 0, "warning": 0, "suggestion": 2},
  "findings": [
    {"id": "E3a-S1", "severity": "SUGGESTION", "ref": "crates/gafira-edit/tests/dirty_sets.rs:103-118", "summary": "AC4 merge proved by len()==3; the set assertion alone cannot distinguish merged from unmerged (same MergeKey page). Keep the len assertion.", "follow_up": false, "status": "open"},
    {"id": "E3a-S2", "severity": "SUGGESTION", "ref": "crates/gafira-edit/tests/dirty_sets.rs:141-153", "summary": "AC6 covers a pre-mutation apply failure only; a post-partial-write failure case is optional.", "follow_up": false, "status": "open"} ],
  "gates": {"fmt": 0, "clippy": 0, "test": 0, "deny": 0, "ci_policy": 0},
  "mutation": null,
  "hand_mutants": {"run": 1, "killed": 1},
  "criteria": {"total": 7, "covered": 7},
  "size": {"lines": 171, "forecast": 126}, "report": "e3a-verify-report-r1.md" } ] }
```

next: tdd-apply (next delivery `E3b`)

---

# Verify report: `gafira-engine` delivery `E3b` (session, edits, document version)

round: 1 (first verify of this code)
commit: 9c16cc5 (branch `feat/gafira-engine-e3`, worktree `/tmp/gafira-engine-e3`), base 9df403d
mode: full | level: L3 | pack: new capability | mutation: off | tier: hand mutants only | depth: V1

**Verdict: PASS** (0 CRITICAL, 4 WARNING, 2 SUGGESTION). The delivery has no CRITICAL finding;
the change still has deliveries left (`E3c` onward). The size is above the ~700-line ceiling, and
going past it needs the owner's explicit OK (E3b-W1).

| id | severity | class | ref | summary | follow-up | status |
|---|---|---|---|---|---|---|
| E3b-W1 | WARNING | implementation | `git diff --shortstat 5df8a7a..3bde884` | Size: 734 authored lines (717+/17-), above 588 forecast and ~700 ceiling; needs owner OK. Notes' 417 prod/317 test split is wrong: measured ~350 prod / ~367 test | no | open |
| E3b-W2 | WARNING | ambiguity | 02 §5.4 line 1357; `project.rs` `new_document` | D1: §5.4 names `ProjectService::new`; clippy `new_ret_no_self` (verified on a scratch export) plus the no-`#[allow]` rule forbid it. The spec comment needs to name `new_document`, or another name needs to be picked | yes | open |
| E3b-W3 | WARNING | implementation | AC4; `project.rs` `every_page_and_node` | D5: the whole-document set is proven only for the empty doc. Mutant "return `DirtySet::default()`" survives. It is equivalent in E3b, because `new` only builds empty docs. `E3c` `open` must assert a non-empty set | yes | open |
| E3b-W4 | WARNING | ambiguity | AC2; `edit.rs` undo/redo; `gafira-edit/src/history.rs:236-252` | D4: the undo/redo `Err` half of AC2 cannot be tested through the engine. `History` errs only on a compacted entry, and the engine fixes the 512 MiB budget. Proved by inspection only: `?` comes before the publish | yes | open |
| E3b-S1 | SUGGESTION | - | P15; `session.rs` `Session` | D2: `Session` has only `history`, `clean_version`. `path`/`read_only`/`project_key`/autosave are left out because they would be dead code under `-D warnings`. Accepted; the `E3c`/`E3d` cards should list them | - | open |
| E3b-S2 | SUGGESTION | - | P15 "Modified"; `project.rs` `is_modified` | D3: `is_modified()` is `false` with no session. Sensible, and `ac8` asserts it, but P15 does not say it. Record it in P15 | - | open |

## Gates (HEAD 9c16cc5, workspace scope)

| gate | exit | evidence |
|---|---|---|
| `cargo fmt --check` | 0 | clean |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | no warning |
| `cargo test --workspace --locked` | 0 | 199 suites, 2544 passed, 0 failed |
| `cargo deny check` | 0 | advisories ok, bans ok, licenses ok, sources ok |
| CI policy (ci.yml `policy` job) | 0 | ARQ-07 CLI, forbid(unsafe), V5 layer table, V12 clippy.toml sync, ARQ-107, V13 EditToken, V14 property tests, testkit `ports` feature clippy + test: all exit 0 |

Purity: `crates/gafira-engine/src` has no `EditToken`, no `std::thread`, no `std::env` (one doc
comment only says it is banned), no `Instant::now`/`SystemTime`. `EditService` takes `at` from
`Clock::monotonic()` (`edit.rs` `execute`). The worktree is clean after the run. Both scratch
target dirs were removed with `cargo clean`.

## Artifact integrity

- `9df403d` (slice card, coordinator) touches only `specs/changes/gafira-engine/apply-progress.md` (+58).
- No implementer commit changed the card. `9c16cc5` (notes) appends 80 lines and deletes none;
  `5df8a7a`, `dc28f3c` and `3bde884` do not touch the file.
- `5df8a7a` is a pure move. After normalising indentation, the only differences between the
  removed and added lines are `pub(crate)` visibility, explicit imports in place of globs, the
  `mod fakes;` / `#[cfg(test)]` declaration, a reflowed `load` signature and module/doc comments.
  No behaviour changed.

## Acceptance map

| AC | proof | status |
|---|---|---|
| AC1 | `edit.rs` `ac1_a_changed_execute_...`, `ac1_a_merged_execute_...`, `ac1_ac4_undo_...`, `ac1_ac4_redo_...`; `project.rs` `ac1_ac4_new_bumps_once_...` (exact version, whole event vec) | covered |
| AC2 | `ac2_an_unchanged_execute_...`, `ac2_a_failed_execute_...`, `ac2_an_empty_undo_or_redo_...`, `ac2_gestures_...` (empty drain plus the next bump's exact version) | covered except undo/redo `Err` (E3b-W4) |
| AC3 | `ac3_the_version_strictly_increases_across_a_second_new` (versions `1..=9`) | covered |
| AC4 | the undo/redo/execute tests compare whole `DirtySet`s; `new` gives `{}` | covered for execute/undo/redo; whole-document set only for the empty doc (E3b-W3) |
| AC5 | `ac5_a_gesture_of_many_steps_is_one_entry` | covered |
| AC6 | `ac6_new_drops_the_history_before_it` | covered |
| AC7 | `ac7_without_a_session_every_edit_call_is_refused_...` (all 5 methods, empty drain) | covered |
| AC8 | `project.rs` `ac8_modified_after_an_edit_and_after_undoing_it` | covered |
| AC9 | `engine.rs` `ac9_every_engine_error_has_its_key_...` (6 variants, delegation for `Repository`/`Store`); the `to_message` match has no `_` arm | covered |
| AC10 | `assert_send_sync::<Gafira>()` (`engine.rs:461`) compiles | covered |
| AC11 | gates above | covered |

Product IDs: the tests carry `// ARQ-30`, `// ARQ-33`, `// ARQ-102`. Covered: ARQ-30
(execute/undo/redo through `History` only), ARQ-33 (AC5), ARQ-102 (`ToMessage` half; `UserError` is
out of scope, `E11`), §5.2 (1317: `DocumentChanged { version, dirty }`), §5.4 (1357
`projects(&self)`, `new` as `new_document`, E3b-W2; 1359 `edit(&mut self)` with execute, undo,
redo, begin/end_gesture). §5.1: `at` comes from the clock. §7.3: keys per variant. Left out by
the card: `open`/`save` (`E3c`), autosave (`E3d`), `UserError` (`E11`).

Critical points (design "Critical code" row `E3b`):
- P15 bump table: matches `edit.rs`/`project.rs` exactly. Bumps on `Changed` (merged or not),
  undo/redo `Some` and a successful `new`. No bump on `Unchanged`, `Err`, `None`, the gestures or
  a failed `new` (the `?` runs before `changed`).
- Strict monotonicity: one `AtomicU64` per engine, `fetch_add(1)`, and publishing happens under the
  session lock, so versions reach the bus in order.
- Dirty sets: `newest_touches` after execute/redo, `undone_touches` after undo (P14).
- History boundary: `new_document` replaces `Session` and the old `History` is dropped.
- `NoSession`: `with_session` refuses before running or publishing anything.

## Deviations judged

- D1: valid; the lint was verified (rename to `new` makes `clippy -D warnings` fail with
  `new_ret_no_self`). Spec wording is a follow-up: E3b-W2.
- D2: accepted (dead-code rule): E3b-S1.
- D3: accepted, unspecified: E3b-S2.
- D4: the claim was verified in `history.rs` (snapshot entries cannot fail). Not testable through
  the engine as designed: E3b-W4.
- D5: confirmed by a surviving mutant that is equivalent in E3b: E3b-W3.
- D6: no finding. `gafira-geom` is in the engine's V5 row, V5 reads only the normal tree, and
  `cargo deny` passes.
- D7: no finding. It matches P12 (every variant has a key in `error.engine.`), and dropping the
  detail argument avoids prose in a message (ARQ-102).

## Hand mutants (scratch export of HEAD, own target dir)

| # | mutant | result |
|---|---|---|
| 1 (recorded) | bump on an `Unchanged` execute | killed by `ac2_an_unchanged_execute_neither_bumps_nor_publishes` |
| 3 (recorded) | reset the version to 0 before the bump in `new_document` | killed by `ac3_the_version_strictly_increases_across_a_second_new` |
| 4 (recorded) | `newest_touches` after `undo` | killed by `ac1_ac4_undo_bumps_once_with_the_undone_set` |
| P1 (probe for D5) | `new_document` publishes `DirtySet::default()` | survived; equivalent in E3b (E3b-W3) |

Each of the three recorded mutants fails only its own named test, which answers the concern about
"RED only at the shared setup step". Totals: 4 run, 3 killed, 1 equivalent survivor.

## Size

`git diff --shortstat 5df8a7a..3bde884`: 10 files, 717 insertions, 17 deletions, so 734 authored
lines. Counting by `#[cfg(test)]` boundary: about 350 production (including docs and manifest)
and about 367 test lines inserted. `9df403d..3bde884`, which includes the pure move, is 865/152.
The forecast was 420 (588 at x1.4), and the ceiling is ~700.

```json
{ "schema": 1, "change": "gafira-engine", "delivery": "E3b", "rounds": [ {
  "round": 1, "commit": "9c16cc5", "mode": "full", "tier": "hand", "depth": "V1",
  "verdict": "PASS", "next": "tdd-apply",
  "counts": {"critical": 0, "warning": 4, "suggestion": 2},
  "findings": [
    {"id": "E3b-W1", "severity": "WARNING", "class": "implementation", "ref": "git diff --shortstat 5df8a7a..3bde884", "summary": "734 authored lines over the ~700 ceiling and 588 forecast; owner OK needed; notes' prod/test split wrong (~350/~367)", "follow_up": false, "status": "open"},
    {"id": "E3b-W2", "severity": "WARNING", "class": "ambiguity", "ref": "02 §5.4 line 1357", "summary": "ProjectService::new cannot exist under clippy new_ret_no_self and no #[allow]; spec comment should name new_document", "follow_up": true, "status": "open"},
    {"id": "E3b-W3", "severity": "WARNING", "class": "implementation", "ref": "AC4; project.rs every_page_and_node", "summary": "whole-document set proven only for the empty doc; default-set mutant survives (equivalent in E3b); E3c open must assert a non-empty set", "follow_up": true, "status": "open"},
    {"id": "E3b-W4", "severity": "WARNING", "class": "ambiguity", "ref": "AC2; edit.rs undo/redo", "summary": "undo/redo Err cannot be reached through the engine (compaction needs > 512 MiB); proved by inspection only", "follow_up": true, "status": "open"}
  ],
  "gates": {"fmt": 0, "clippy": 0, "test": 0, "deny": 0, "ci_policy": 0},
  "mutation": null,
  "hand_mutants": {"run": 4, "killed": 3},
  "criteria": {"total": 11, "covered": 11},
  "size": {"lines": 734, "forecast": 588}, "report": "e3b-verify-report-r1.md" } ] }
```

classes (CRITICAL/WARNING): implementation 2, ambiguity 2.

next: tdd-apply (next delivery `E3c`, after the owner's OK on size, E3b-W1; route E3b-W2/W4 to specifier as follow-ups)

---

# Verify report: gafira-engine, delivery E3c1 (open + private fakes)

round: 1 | commit: e1fef7f | branch: feat/gafira-engine-e3 | mode: full | level: L3 | tier: hand mutants only | depth: V1 | mutation: off

**Verdict: PASS** (E3c1; deliveries remain, E3c2 next)

| id | severity | class | ref | summary | follow-up | status |
|---|---|---|---|---|---|---|
| E3b-W3 | WARNING (earlier) | implementation | project.rs `ac9_open_bumps_once_lists_the_whole_loaded_document_and_drops_the_old_history` | The whole-document `DirtySet` of `open` is now compared in full on a NON-empty document (pages {1,4}, nodes {2,3,5}). Hand mutant B (open drops the nodes) is killed. | - | closed |
| E3c1-S1 | SUGGESTION | - | apply-progress.md Notes E3c1 D3, and the measured lines | D3 says `create_dir_all` and `read_dir` of `MemoryFs` return `Unsupported`. In the code, `create_dir_all` returns `Ok(())` and `read_dir` lists the folder (fakes.rs). The production split is 33 insertions (engine.rs 2, project.rs 31 above `#[cfg(test)]`), not 31 (29 + 2). Fix the notes the next time they are touched. | no | open |
| E3c1-S2 | SUGGESTION | - | fakes.rs `MemoryRepository::save` | The P16 write path (the repository writes through `MemoryFs`) has no test in E3c1. Its proof is AC1/AC2 in E3c2 and the amend lists those there, so this is not a finding here. E3c2 verify should check it. | no | open |

Counts: critical 0, warning 0, suggestion 2. Per class: none (CRITICAL/WARNING).

## Gates (export of e1fef7f, scratch target dir, CARGO_INCREMENTAL=0)

| gate | exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (199 suites, 2548 passed, 0 failed) |
| CI policy: ARQ-07, forbid(unsafe), V5, V12, ARQ-107, V13 (EditToken), V14 | 0 each (run in the worktree, read-only) |
| `cargo deny check` | 0 (no manifest change; run anyway) |
| Testkit `ports` feature step | not run: does not apply, because the delivery does not touch gafira-testkit |

## Artifact integrity (step 3c)

- `fb3c814` (slice card) and `4f742b3` (card amend; it states why: ~800 lines against the ~700 ceiling, owner split) each touch only `specs/changes/gafira-engine/apply-progress.md`.
- `16280b5` (the code) touches only `crates/gafira-engine/src/{engine,project,fakes}.rs`.
- `e1fef7f` (notes) only appends after the amend: the diff has zero `-` lines.
- No implementer commit changed the card or the amend.

## Criteria (E3c1: AC8, AC9, AC12, AC13)

| AC | status | proof |
|---|---|---|
| AC8 | covered | `ac8_an_open_that_fails_to_load_leaves_the_old_session` and `ac8_an_open_of_an_invalid_document_leaves_the_old_session`, through `assert_old_session_kept`. They check: nothing published, still modified, the old document (1 page), the old history (undo is `Some`), and the version did not move (the undo publishes version 3). |
| AC9 | covered | The `ac9_...` test checks: exactly one `RepoCall::Load(TARGET)`, one `DocumentChanged` at version 3 with the full `DirtySet` of a non-empty document, clean after open, the loaded document equal to the fixture, and no undo across open. |
| AC12 | covered | `git grep -nE "std::fs\|std::thread\|std::env\|rand::\|thread::spawn" -- crates/gafira-engine/src` finds only the existing doc comment at engine.rs:96. `open` reads only through `ProjectRepository::load`. The CI steps are green. |
| AC13 | covered | The gates above, at workspace scope. |

Design checks:
- **P18**: load, then `History::new` (whose `?` maps to `InvalidDocument`), then lock + publish + swap.
- **P15**: the version bumps once, `clean_version` is set to the new version, and the history is dropped.
- **P16 (fakes are private)**: `lib.rs:21` has `#[cfg(test)] mod fakes;` and every item is `pub(crate)`. `MemoryRepository` reads and writes through `MemoryFs`. The write-side trace covers create, write, sync_all, replace and remove.
- **`invalid_document()`**: it never names `EditToken` (`git grep EditToken -- crates/gafira-engine` finds nothing, and V13 is green). It builds the invalid document through `DocMut` inside a failing `Command`.
- `read_only` is ignored until E3c2 (D2, as the amend says).

## Hand mutants (scratch export, `cargo test -p gafira-engine --locked project::`)

| # | mutant | result | killed by |
|---|---|---|---|
| 7 (spot-check of the recorded list) | `*self.shared.lock() = None` before `History::new` | killed | `ac8_an_open_of_an_invalid_document_leaves_the_old_session` ("still modified") |
| A | publish (`shared.changed`) before `History::new` validates | killed | `ac8_an_open_of_an_invalid_document...` ("a failed open publishes nothing") |
| B | `open` publishes only the pages (nodes cleared) | killed | `ac9_...` (the `DirtySet` comparison) |
| C | keep the old history on `open` instead of dropping it | killed | `ac9_...` (the loaded-document equality) |

hand_mutants: run 4, killed 4. mutation: null (`mutation: off`).

## Size

`git diff --shortstat fb3c814..16280b5`: 3 files, 496 insertions, 10 deletions.
- Production: about 33 insertions.
- Tests: about 463 insertions (fakes.rs 353/4, the tests in project.rs about 110).
- The E3c forecast was 380 (532 at x1.4). E3c1 alone is under the ~700 stop line.

## Hygiene

- The worktree is clean at e1fef7f.
- `stash@{0}` was not touched.
- Both scratch target dirs were removed with `cargo clean`.

next: tdd-apply (E3c2)

---

# Verify report: `gafira-engine` delivery `E3c2` (atomic save, read-only refusal)

round: 1
commit: 99c38bc (worktree `/tmp/gafira-engine-e3`, branch `feat/gafira-engine-e3`, base `2f99df8`)
mode: full | level: L3 | pack: new capability | mutation: off | tier: hand mutants only | depth: V1 (decided-by: owner)

**Verdict: PASS** for this delivery. It has 0 CRITICAL, 0 WARNING and 3 SUGGESTION findings. The change still has deliveries to come (`E3d` and later), so it is not done.

| id | severity | class | ref | summary | follow-up | status |
|---|---|---|---|---|---|---|
| E3c2-S1 | SUGGESTION | - | `project.rs` `ac5_a_read_only_session_edits_undoes_and_redoes_like_any_other` | AC5 never asserts that the session it edits is read-only. Mutant C (`open` drops `read_only`) passes AC5 and only AC6 kills it. Adding `save() == Err(ReadOnly)` at the end of AC5 would make the test prove this alone. | no | open |
| E3c2-S2 | SUGGESTION | - | `project.rs` `temporary_for` (D2) | The counter is per engine and starts at 0, so two engines or processes saving one file both use `<file>.tmp-0`. This is allowed by the owner's answer ("unique name in the target folder", P18) and the file lock is out of scope (ARQ-57). Recheck when ARQ-57 lands. | no | open |
| E3c2-S3 | SUGGESTION | - | design XP5, `repository.rs` doc comment (SP6) | SP6 is only a written contract so far: no real `ProjectRepository` adapter checks that it syncs the file. Keep XP5 open for `gafira-format`. | no | open |

## Gates (at HEAD 99c38bc, workspace scope, external `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`)

| gate | exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0: 199 suites, 2558 passed, 0 failed (matches the notes) |
| CI policy job, all 10 `run` steps taken from `ci.yml` (ARQ-07, forbid unsafe, V5, V12, ARQ-107, V13 EditToken, V14, testkit `ports` feature) | 0 each |
| `cargo deny check` (no manifest change, run anyway) | 0 |

Before the run, `df -h /` showed 146 GB free. Both target dirs were cleaned afterwards with `cargo clean --target-dir`. The worktree is clean at HEAD `99c38bc` and holds no stash.

## Artifact integrity

- Slice card `fb3c814` and amend `4f742b3` touch only `specs/changes/gafira-engine/apply-progress.md`. The amend states why (size, split chosen by the owner).
- Implementer commits: `5297f6d` (`engine.rs`), `72d34f6` (`engine.rs`, `fakes.rs`, `project.rs`, `session.rs`), `adad43a` (`gafira-ports/src/repository.rs`), `99c38bc` (notes; it only adds lines to `apply-progress.md`, 86+/0-). No implementer commit changed the card, `design.md` or `specs/02`.
- The `gafira-ports` change is doc-only: the diff touches only `///` lines on `ProjectRepository::save` and changes no signature.
- Commit order: code, then the port doc, then the notes in a separate commit.

## Acceptance criteria

| AC | proof | status |
|---|---|---|
| AC1 ARQ-50 fault at every op | `ac1_...` fixes the trace (Create, Write x2, SyncAll, Replace) exactly, then injects a fault at each of the 5 indices and requires the target to keep its old bytes; with no fault, the new bytes in full | covered |
| AC2 order, no `replace` after an error | `ac2_...`: exact repo calls `[Save(tmp-0)]`, exact trace; faults 0-3 give trace up to the fault, then `Remove`, with no `Replace` | covered |
| AC3 `remove`, first error kept, still modified | `ac3_...` table: [3], [4], [3,4], [4,5]; checks the error's origin and text, the exact trace, whether the temporary is left behind, still modified, nothing published | covered |
| AC4 clean at the snapshot | `ac4_...`: clean after save, no event; an edit after the save makes it modified again; an edit between snapshot and `replace` (hook) leaves it modified | covered |
| AC5 ARQ-55(a) | `ac5_...`: read-only and writable runs give the same versions [1,2,3,4] and the same page count (see S1) | covered |
| AC6 ARQ-55(b) | `ac6_...`: `ReadOnly`, empty fs trace, empty repo calls, target bytes unchanged, still modified | covered |
| AC7 `NoSession` / `NoPath` | `ac7_...`: both errors, empty trace and calls | covered |
| AC10 distinct temporaries in the folder | `ac10_...`: exact traces through `/work/poster.gafira.tmp-0` then `tmp-1` | covered |
| AC11 keys | `engine.rs` `ac11_...`: `read_only`, `no_path`, `io` with no argument | covered |
| AC12 purity | `git grep -nE "std::fs\|std::thread\|std::env\|rand\|thread::spawn\|EditToken" -- crates/gafira-engine/src` finds only the doc comments `engine.rs:96` and `project.rs:130`. CI V13 passes. The temporary name comes from an `AtomicU64` | covered |
| AC13 gates | above | covered |
| E3c1-S2 | `MemoryRepository::save` writes through `MemoryFs` (`fakes.rs:450-459`: `fs.create`, 2 `write_all`, `sync_all`). The repository's 4 ops appear in the AC1/AC2 traces and get faults injected | closed |
| `project_key` rule (P18) | `a_save_that_finishes_after_the_session_was_replaced_leaves_the_new_one_alone` | covered |

Every test checks exact values (traces, bytes, versions, error origin). Covered: 11 of 11 criteria, plus E3c1-S2.

## Hand mutants (re-applied by verify on a `git archive` export of `99c38bc`, `cargo test -p gafira-engine --locked --lib`, reverted; the restored file was checked byte for byte)

| # | mutant | killed by | failure |
|---|---|---|---|
| 1 | `replace` even after `repository.save` failed | ac1, ac2, ac3 | ac3: trace has `Replace(tmp-0, TARGET)` where `Remove(tmp-0)` was expected |
| 4 | the `remove` error replaces the first | ac3 | `("filesystem","fault at 4")` vs `("repository","fault at 3")` |
| 6 | `read_only` checked after the repository's save | ac6 | trace `[Create, Write, Write, SyncAll]` vs `[]` |
| A (own) | `remove(tmp)` only after a repository error, not after a failed `replace` | ac3 | trace ends at `Replace`, `Remove` missing |
| B (own) | temporary = file name only (outside the target's folder) | ac1, ac2, ac3, ac10 | `poster.gafira.tmp-0` vs `/work/poster.gafira.tmp-0` |
| C (own) | `open` drops `LoadedProject::read_only` | ac6 | `ReadOnly` expected (AC5 passes, see S1) |

Run 6, killed 6. The implementer's list (#1-6, #8) plus X1-X3 is recorded with the test that kills each mutant. This round re-applied #1, #4 and #6, and each failed as recorded.

## Critical points

- Order `save(tmp)` then `replace(tmp, path)`, and no `replace` after an error: `write_then_replace` uses `?`. Proved by AC2 and mutant 1.
- `remove(tmp)` runs on any failure and its error is discarded (`let _`). Proved by AC3 and mutants 4 and A.
- The target is unchanged at every fault: AC1.
- `ReadOnly` comes before any port call: the check happens under the lock before `temporary_for` or any port use. Proved by AC6 and mutant 6.
- `clean_version` is set only after `Ok` and only when the `project_key` matches. Proved by AC3, AC4 and the session-replaced test (implementer probes 5, X2, X3).
- `open` keeps `read_only`: AC6 and mutant C.
- Distinct temporaries in the folder: AC10 and mutant B.
- Purity: no `std::fs`, threads, `std::env`, `rand` or `EditToken`.

## Deviations

- D1 (`project_key` = `<clock seconds>-<counter>`, `ProjectService` holds `Clock`). This follows P13. Only equality is read, and the counter keeps keys unique within an engine. Accepted.
- D2 (temporary = whole target path + `.tmp-<n>`). Appending to the whole path keeps the parent directory, so the result is `<target dir>/<file name>.tmp-<n>`. That is exactly P18's `<file name>.tmp-<n>` in the target's folder, and it meets §7.2 step 1 (line 1568: a unique name in the same directory, how it is generated is not fixed). Mutant B shows the folder is pinned by tests. Accepted (see S2).
- D3 (`save` neither bumps nor publishes). This matches the P15 table and AC3/AC4 assert it. Accepted.
- D4 (AC5 not RED). This is legitimate: the behaviour came before this delivery, since no edit path reads `read_only`, so a stub cannot fail the test. Probe X1 shows the assertion can fail. Accepted, with S1 to make the test self-sufficient.
- D5 (`Gafira::projects` doc). Doc-only. Accepted.

## Size

`git diff --shortstat 2f99df8..adad43a`: 5 files, 408 insertions, 17 deletions (425 authored). With the notes (`..99c38bc`): 6 files, 494 insertions, 17 deletions. The forecast was ~300, so this is x1.42, under the ~700 ceiling. The notes' production/test split matches `git diff --numstat`.

```json
{ "schema": 1, "change": "gafira-engine", "delivery": "E3c2", "rounds": [ {
  "round": 1, "commit": "99c38bc", "mode": "full", "tier": "hand", "depth": "V1",
  "verdict": "PASS", "next": "tdd-apply",
  "counts": {"critical": 0, "warning": 0, "suggestion": 3},
  "findings": [],
  "gates": {"fmt": 0, "clippy": 0, "test": 0, "deny": 0, "ci_policy": 0},
  "mutation": null,
  "hand_mutants": {"run": 6, "killed": 6},
  "criteria": {"total": 11, "covered": 11},
  "size": {"lines": 425, "forecast": 300}, "report": "e3c2-verify-report-r1.md" } ] }
```

next: tdd-apply (next delivery, `E3d`)
