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
