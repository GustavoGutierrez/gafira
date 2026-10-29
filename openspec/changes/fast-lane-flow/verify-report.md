# Verify report: fast-lane-flow (D1, round 1)

- Commit verified: `91f9b71` (branch `chore/fast-lane-flow`, base `main`)
- Mode: light | pack: agent/skill definitions | mutation: off (T0, no `mutants` report) | depth: V1
- Acceptance source: Light spec block in `apply-progress.md` (AC1 to AC7)
- Verdict: **PASS** (0 CRITICAL, 6 WARNING, 3 SUGGESTION)

## Gates at HEAD

| Gate | Command | Exit |
|---|---|---|
| fmt | `cargo fmt --check` | 0 |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| test | `cargo test --workspace --locked` | 0 (all suites `ok`, 0 failed) |
| deny | `cargo deny check` | 0 (advisories, bans, licenses, sources ok) |
| CI policy | `policy` job steps of `.github/workflows/ci.yml` (ARQ-07, forbid unsafe, V5, V12, V13, V14) | 0 each |
| AC2 | `cargo mutants --list -p gafira-geom \| head` (cargo-mutants 27.1.0) | 0, lists mutants; config with `profile = "mutants"` parses |
| AC2 | `cargo build --profile mutants -p gafira-geom` | 0 |
| AC7 | `python3 docs/manuals/tecnico/manual.py check` (`--lang en` and `--lang es`) | 0 / 0: 11 chapters, mirror ok, no absolute paths, links resolve |

Mutation: `mutation: off`, no run, not a finding. Hand mutants: none (V1).

## Light spec compliance (step 3b)

- `23fc095 docs(fast-lane-flow): light spec` touches only `openspec/changes/fast-lane-flow/apply-progress.md` (16 insertions).
- Later commits to that file: only `91d70d2`, which appends `## Apply progress` below line 16; lines 1 to 16 (the Light spec block) are byte-identical at HEAD. No `light spec amend` commit.
- No `Co-Authored-By`, Claude, Anthropic or "Generated" lines in any commit message of `main..HEAD`.
- No absolute or user paths in the 26 changed files (`/home/`, `/Users/`, user name, `~/.claude`: no match).

## Acceptance criteria map

| AC | Evidence | Status |
|---|---|---|
| AC1 Flow line with `mutation: off\|archive\|delivery`, default `archive`, `off` runs nothing | `AGENTS.md` lines 20, 29; `.agents/agents/coordinator.md` Recording paragraph and pack table; `.agents/agents/mutants.md` Inputs; `.agents/agents/verify.md` Inputs and step 1b; ADR 0002 section 1; manual ch. 10 "Mutation toggle" and Flow line (EN/ES) | covered |
| AC2 `mutants` profile with `debug = "none"`, set in `.cargo/mutants.toml`, `--jobs 8`; `--list -p gafira-geom` succeeds | `Cargo.toml` `[profile.mutants]` (`inherits = "test"`, `debug = "none"`); `.cargo/mutants.toml` `profile = "mutants"`; `--jobs 8` in `AGENTS.md`, `mutants.md`, manual ch. 09; commands above exit 0 | covered |
| AC3 Product IDs via Slice card satisfying G0, no proposal/delta spec, `specifier` only on ambiguity | `coordinator.md` "Fast lane: the Slice card" and G0 row; `tdd-apply.md` Inputs and step 1; `verify.md` steps 1, 3, 3c; `specifier.md` "When you are used" and step 3; `product-specs/SKILL.md` section 4; resume rule (`coordinator.md` line 100, `docs/specs/README.md`) | covered |
| AC4 New crate no longer forces full; `architect` only for open design decision or ordered deliveries | `AGENTS.md` line 19 floors; `coordinator.md` floor table; `architect.md` "When you are used"; ADR 0002 section 4. See W2, W3 on the "ordered deliveries" clause | covered (with warnings) |
| AC5 Verify default V1 docs/tooling, V2 code, V3 only on owner request | `AGENTS.md` line 27; `coordinator.md` pack table and mode defaults; `verify.md` Inputs; ADR 0002 section 5; manual ch. 10 depth table. See W4 | covered (with warning) |
| AC6 Archive moves folder, updates `docs/specs/README.md` status table with IDs, no delta merge; tests carry IDs | `archive.md` steps 0 and 2b; `AGENTS.md` line 79 (test ID tagging) and "Archive merge"; `tdd-apply.md` step 4; `verify.md` step 3c; `product-specs/SKILL.md` section 5 | covered |
| AC7 ADR 0002 supersedes parts of ADR 0001 (marked); manual chapters on methodology and tooling match | `docs/adr/0002-fast-lane.md` (accepted, "What ADR 0001 keeps"); ADR 0001 status "accepted, partly superseded by 0002" plus top note; `docs/adr/README.md` index; manual ch. 01, 09, 10, 11 in EN and ES; `manual.py check` exit 0. See W4 | covered (with warning) |

Criteria: 7 total, 7 covered, 0 missing, 0 failing.

## Cross-file consistency

Checked `AGENTS.md`, `coordinator.md`, `mutants.md`, `verify.md`, `tdd-apply.md`, `specifier.md`, `archive.md`, `architect.md`, `product-specs/SKILL.md`, ADR 0002 and manual chapters 09/10/11 (EN and ES):
- Flow line format `Flow: <mode> | pack | mutation: off|archive|delivery | tier | verify | decided-by | reason`: identical everywhere it is stated.
- Mutation values and default `archive` (T4 per touched crate at change close), `off` = no run, `delivery` = T2/T3 plus T4 only on owner request or flagged critical rule: consistent.
- Verify defaults V1 docs/tooling, V2 code, V3 on owner request: consistent, except the manual light-mode column (W4).
- New-crate floor (standard when `02` describes it; full for a new V5 row, undeclared dependency edge, format version, open cross-cutting choice, parallel deliveries): consistent; residual "delivery breakdown" wording for `architect` remains (W3).
- `--jobs 8`: no stale `--jobs 4` outside ADR 0001 (historical, accepted), ADR 0002 context and archived changes / `openspec/specs/testkit` (history, not to be rewritten).
- EN/ES manual mirrors carry the same values (`jobs 8`, Flow line, `debug = "none"`, V3 mentions, light verify default).

## CRITICAL

None.

## WARNING

- **W1** class: claim. ref: `docs/adr/0002-fast-lane.md:34`, `AGENTS.md` Flags. `--jobs 8` is adopted as faster without a measurement. Archived evidence points the other way for some crates: `engine-pathops` verify-report records 19 load timeouts at `--jobs 4` and reruns at `--jobs 2 --minimum-test-timeout 120`; `domain-time` saw stale shared builds at `--jobs 4`. Measure one T4 run (for example `gafira-edit` or `gafira-pathops`) at `--jobs 8` with the new profile before relying on it.
- **W2** class: ambiguity. ref: `apply-progress.md` AC4. AC4 says `architect` runs when "the slice needs ordered deliveries"; ADR 0002 section 4, `AGENTS.md`, `coordinator.md` and `architect.md` say serial (ordered) deliveries listed in the Slice card need no `architect`, only parallel deliveries or a dependency table do. The implementation follows the owner-accepted ADR; the AC wording contradicts it. A `light spec amend` stating the intended meaning would close it.
- **W3** class: implementation. ref: `AGENTS.md:7`, `AGENTS.md:63`, `.agents/agents/coordinator.md:119`, `.agents/agents/architect.md:8` (description), `docs/manuals/tecnico/chapters/10-methodology.md:14` (and ES). These still say `architect` is used when "a change needs a delivery breakdown", which reads against ADR 0002 section 4 ("serial deliveries in the Slice card need no `architect`"). Qualify as "a delivery breakdown that needs a design decision or parallel work".
- **W4** class: implementation. ref: `docs/manuals/tecnico/chapters/10-methodology.md:49` (and `chapters-es` line 49). The light column of "Default verify depth" says "V1 or V2 per pack", but the pack table of the same chapter (lines 70, 71) and `coordinator.md` give V0 for the docs-only and manual light packs. Write "V0 to V2 per pack".
- **W5** class: implementation. ref: commit `91f9b71`. The coordinator authored an edit to `.agents/agents/coordinator.md` and `docs/specs/README.md`. `coordinator.md` restricts its `Edit`/`Write` to the Light spec block or Slice card and says it never edits files under `docs/specs/`; `AGENTS.md` says agent definitions are written by a delegated agent. Owner-requested, content is in AC3 scope and correct (the `comm` command runs and lists pending IDs), but it bypassed the writer; record the owner instruction in `apply-progress.md` or route such follow-ups to the writer next time.
- **W6** class: implementation. ref: `git diff --shortstat main...HEAD` = 26 files, +506 / -215 (721 lines; 360 excluding the manual mirror and the change folder). This is above the ~300 light-mode size trigger and at the ~700 stop-and-ask signal; the Light spec watches "size" but there is no `Flow-change:` line or recorded owner acknowledgement. Docs-only, owner-decided light mode, so not blocking; record the decision.

## SUGGESTION

- **S1** `apply-progress.md` lists commits only up to `df38377` and "Measured lines" before the manual and resume-rule commits; append `1c8c285`, `91f9b71` and the final size.
- **S2** The resume command (`coordinator.md:100`, `docs/specs/README.md`) greps `openspec crates` only, while `product-specs/SKILL.md` section 5 traces coverage in `crates/ apps/`. Add `apps` so tests in `apps/gafira-cli` count.
- **S3** The Light spec Flow line of this change predates the new format and has no `mutation:` field; the coordinator's input states `mutation: off`. Harmless here; future Light specs should carry the field.

## Covered / left out

All seven ACs covered. Nothing deliberately left out. No product spec IDs (light mode).

next: archive
