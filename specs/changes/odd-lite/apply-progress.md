## Light spec
Flow: light | pack: agent/skill definitions | mutation: off | tier: T0 | verify: V1 | decided-by: owner
Reason: owner asked to adopt `docs/methodology/odd-ligero-migracion.md` through safe, reversible adjustments; the brief settles the cross-cutting choices, the ADR records them. No Rust, no public API or format change.
Scope: phases A and B of the brief, on instructions and agent definitions only. Phase C (pilot) is delivery D3 of `layout-core`, run afterwards under the coordinator.
Non-goals: Rust code; product spec rewrites or repartition into capability specs; deleting or renaming agents, skills, ADRs, or OpenSpec history; new wrapper skills or commands; a database; deprecating the current flow (phase D, after the pilot).
Acceptance criteria:
  AC1 Given the repo, When the audit is read, Then `docs/adr/0011-odd-lite.md` holds the inventory of brief §3 and a table "component -> keep/adapt/merge/deprecate -> reason -> impact -> check". [manual]
  AC2 Given `AGENTS.md`, When diffed against `develop`, Then it gains an "ODD lite" section adapted from brief §6 and no existing safety, architecture, TDD, testing, delivery or Git rule is lost. [command]
  AC3 Given `coordinator.md`, When a request arrives, Then it classifies L0-L3 by impact, uncertainty and reversibility, each level maps to the existing modes and agents, and every hard floor, gate and owner stop still applies. [manual]
  AC4 Given new work on a capability that an existing spec covers, When it runs, Then no `openspec/changes/<change>/` folder, proposal, design or tasks file is required by default; L3 keeps design and ADR. [manual]
  AC5 Given resumable state, When work resumes, Then one small tracked `.odd/state.json` holds outcome, spec, criteria status and next action, the coordinator checks branch, diff and tests first, and `layout-core` keeps its folder until archive. [command]
  AC6 Given `verify.md`, When a critical check cannot run, Then the verdict can be BLOCKED with escalation to the owner; PASS and FAIL rules, gates and the `verification.json` schema stay compatible. [manual]
  AC7 Given the brief's `odd-spec`, `odd-run` and `odd-verify`, When the ADR is read, Then each maps to existing roles (`specifier` with `product-specs`, `coordinator` with `tdd-apply`, `verify`), every agent file still exists, and `mutants` stays mandatory on critical code by risk. [command]
  AC8 Given the ADR, When read, Then it states status `proposed` until the pilot ends, the retained flows, the rollback steps, and "not measured" where no baseline exists. [manual]
  AC9 Given the diff, Then it touches no `crates/`, `apps/`, `docs/specs/` or `openspec/changes/archive/` path, and no absolute or user path enters a tracked file. [command]
  Amended for phase D (owner request, 2026-11-01; W1 to W3 of ADR 0011, delivery `chore/odd-lite-phase-d`). AC2 to AC9 hold unchanged for it.
  AC10 (W1) Given a `verify` run on an outcome with no folder, When the coordinator records it, Then the entry's `verify` field holds, besides round, commit, verdict and next, every CRITICAL and WARNING finding of the verdict (`id`, `severity`, `class`, `ref`, `summary`, `follow_up`, `status` open or closed) and one `{round, verdict, classes}` row per run, and the schema is stated in `AGENTS.md` "Outcome state" and `coordinator.md`; the record is committed on the delivery branch before any routing, on FAIL and BLOCKED too. [manual]
  AC11 (W1) Given a new session with only the repo and the empty scratchpad, When it resumes that outcome, Then the rules say where to read the entry (the delivery branch wins over `develop`), `coordinator.md` routes from the stored classes and applies the two-FAIL and three-round stops from the stored rows, `verify.md` takes the open findings as closure-round input and needs no earlier report, and `archive.md` takes follow-ups from the entry. [manual]
  AC12 (W2) Given `coordinator.md`, When read, Then the "never lowers a pack default" rule names `mutation: off` for L0 and L1 with no critical code as its only exception, and the "heaviest of" rule states what it fixes (mode) and where verify depth and agents come from; `AGENTS.md` agrees. [manual]
  AC13 (W3) Given `verify.md` step 3d, When read, Then it names the product ID check and the finding classes of step 3c for the outcome entry path. [manual]
  AC14 Given the diff, Then `python3 tools/sync_agents.py --check` exits 0, `python3 -m json.tool .odd/state.json` exits 0, an entry without the new fields stays valid, the ADR records W1 to W3 as resolved and stays `proposed`, and `.odd/state.json` keeps `schema: 1`. [command]
  Amended for the `specs/` consolidation (owner request, 2026-11-01; ADR 0011 "consolidate into `specs/`", delivery `chore/specs-consolidation`). AC9's `docs/specs/` and `openspec/changes/archive/` limits do not apply to it; its limits are AC15 to AC20.
  AC15 Given the tree, Then `docs/specs/` content (with `backlog/` and `archive/`) is in `specs/`, `openspec/specs/` in `specs/capabilities/`, `openspec/changes/archive/` in `specs/history/`, active folders in `specs/changes/`, every move done with `git mv` so `git log --follow` crosses it, and `openspec/` and `docs/specs/` no longer exist. [command]
  AC16 Given `openspec/config.yaml`, Then every rule still in force lives in `AGENTS.md` and nothing cites the file; its stale statements (no code yet) are dropped. [command]
  AC17 Given L3 and `flow: legacy` work, Then the rules state that a change folder lives in `specs/changes/<change>/` and closes into `specs/history/<date>-<change>/`. [manual]
  AC18 Given the historical record, Then `specs/history/`, `specs/capabilities/` and ADRs 0001 to 0010 keep their text byte for byte (only renames) and are exempt from the reference check; ADR 0011 keeps its From column and states this exemption. [command]
  AC19 Given every other tracked file, When `git grep -nE 'openspec|docs/specs'` runs with those exemptions, Then the only hits are named in the delivery notes as deliberate (ADR 0011's mapping; the `openspec` schema word in nothing else), and every path a rule, skill, manual or tool cites exists; `specs/README.md` "Como continuar" describes the outcome route, its pending-ID command runs and excludes product documents. [command]
  AC20 Given the diff, Then `python3 tools/sync_agents.py --check` exits 0, the agent sync tests pass, `manual.py check` and `build` exit 0, no `crates/` or `apps/` file changes, no absolute or user path enters a tracked file, and the cargo gates pass. [command]
Plan: 1 audit and ADR 0011; 2 `AGENTS.md` and `CLAUDE.md`; 3 `coordinator.md` router and state; 4 `verify.md` BLOCKED; 5 `specifier.md` and `tdd-apply.md` only if the state or spec rule needs it; 6 `.odd/state.json`; 7 ADR index. Phase D: 8 W1 in `AGENTS.md`, `coordinator.md`, `verify.md`, `archive.md`; 9 W2 and W3; 10 `.opencode` regenerated; 11 ADR notes. Consolidation: 12 `git mv` alone; 13 references and rules; 14 `.opencode`; 15 manual with `manual-writer`.
Risk and reversibility: low; instructions only, one revert restores the flow.        Owner decisions touched: none (D-nn are product decisions; ADR 0001 to 0004 amended, not removed).
Escalation triggers watched: ambiguity, public API/format, domain invariant, size, architecture

## Apply: phases A and B

Status: done. Branch `chore/odd-lite`. No Rust touched; cargo gates not run. CI has no step that scans docs or tracked paths, so no policy step applied.

| AC | Evidence |
|---|---|
| AC1 | `docs/adr/0011-odd-lite.md`, "Inventory" (brief §3, items 1 to 7) and "Disposition" |
| AC2 | `AGENTS.md` section "ODD lite". `git diff --word-diff=porcelain develop...HEAD -- AGENTS.md CLAUDE.md` removes only 10 short word-diff fragments (for example `git.`, `touched.`, `PASS or`), each reworded in place; no rule line deleted |
| AC3 | `coordinator.md` "Risk level" table, ahead of the floors table; floors, packs, gates and owner stops unchanged except G0 and G4 gaining the entry and BLOCKED |
| AC4 | `AGENTS.md` "Change folder rule"; `coordinator.md` "Acceptance record" and "Outcome state"; G0 lists the outcome entry; L3 and full keep a folder |
| AC5 | `.odd/state.json` tracked (`git ls-files .odd`), not ignored (`git check-ignore` exit 1), `python3 -m json.tool` exit 0; resume rule in `coordinator.md` and `AGENTS.md`; `layout-core` folder rule in both |
| AC6 | `verify.md` verdict text, `next: owner`, step 3d, optional `blocked` field, gate `null`; schema stays 1, PASS and FAIL rules unchanged |
| AC7 | ADR "Brief names"; `ls .agents/agents` lists all 9; `mutants.md` and `coordinator.md` keep T4 mandatory on critical code |
| AC8 | ADR status `proposed`, "Retained flows", "Rollback", "Metrics" with "not measured" |
| AC9 | `git diff --name-only develop...HEAD` has no `crates/`, `apps/`, `docs/specs/` or `openspec/changes/archive/` path (grep exit 1); no `/home/` in added lines (grep exit 1) |

Measured lines: 311 insertions, 52 deletions, 13 files, from `6e549ed` to the last code commit (`git diff --shortstat`). This is above the ~300 light signal: mostly the new ADR (132 lines) and the `AGENTS.md` section. Mode stays as the owner decided.

Deviations
- The plan named `specifier.md` and `tdd-apply.md`. `archive.md`, `explorer.md`, `mutants.md` and skill `product-specs` also got small edits, because each would otherwise contradict the change folder rule or the mutation decision.
- Gap filled, not decided in the brief: with no folder, `tdd-apply` notes go in the last commit's body, `verify` writes its two files under the scratchpad, and the entry's `verify` field keeps round, commit, verdict and next. The ADR lists the thinner audit trail as a risk.
- `Flow:` gains an optional `level` field; older lines stay valid.
- The entry gains optional G0 fields beyond brief §10 (`flow`, `scope`, `non_goals`, criterion `text` and `type`, `verify`), used only when it is the G0 artifact. The two seeds use the §10 fields alone.
- `layout-core` `spec_id` names `04`, the source of most of its IDs; the Slice card also cites `02`.
- Left stale, outside AC9 or this scope, listed in the ADR for phase D: "Cómo continuar" in `docs/specs/README.md` and the manual's `10-methodology` chapters.

## Apply: specs consolidation

Status: done, verify round 1 FAIL on C1 and W13, closed by this section and the ADR 0011 fix. Branch `chore/specs-consolidation`. No Rust touched.

Commits: `f792bdf` renames only (109, all R100); `633e063` references, rules, tool, test, `.opencode`; `4e51eff` stale archive claims, `.gitignore` entry `__pycache__/`; `6c2b2ee` and `70c5c09` manual, by `manual-writer`.

Measured lines: 428 insertions, 316 deletions, 744 changed lines, 706 without `.opencode` (`git diff --shortstat develop...70c5c09`, before the verify report). This is above the ~700 signal. Owner OK given 2026-11-01: "me importa completarlas, no el tamaño del commit".

| AC | Evidence |
|---|---|
| AC15 | `f792bdf`: 109 renames, R100. `git ls-tree -d HEAD` finds neither old root. `git log --follow --oneline specs/02-arquitectura-software.md` lists 44 commits at `633e063`, crossing the move |
| AC16 | The deleted config file's rules live in `AGENTS.md` ("Strict TDD", "Code rules", "Specs in this repo"). Its stale statements and tool versions were dropped. Only ADR 0011 still cites it (lines 111 and 115) |
| AC17 | `AGENTS.md` "Change folder rule" and layout tree: `specs/changes/<change>/`, closing into `specs/history/<date>-<change>/` |
| AC18 | `git diff -M --numstat develop...HEAD` on the exempt paths: 96 files, 0 insertions, 0 deletions; ADRs 0001 to 0010 are not in the diff. ADR 0011 keeps its From column, gains the resolved note (line 115), stays `proposed` |
| AC19 | Hits below. Path script: 402 paths in 47 live files, 56 missing, none caused by the move (planned or generated files, templates, crate-relative paths). `specs/README.md` "Cómo continuar" describes the outcome route; its pending-ID command runs and returns 69 of 122 ARQ IDs on `02`. Searching all of `specs` returns 0, which is why product documents stay out. `coordinator.md` carries the same fix |
| AC20 | `python3 tools/sync_agents.py --check` 0; `python3 -m pytest tests/test_agent_sync.py -q` 16 passed (RED first: 1 failed, `deny` against `allow`); no `crates/` or `apps/` change; no absolute path in added lines (grep exit 1). This writer ran no cargo gate; `verify` round 1 ran them (exit 0, 1107 passed) and `manual.py check` and `build` (exit 0) |

Remaining hits of the old names, all deliberate (case-insensitive for the tool name, same exemptions as AC19; `docs/manuals/` has none):

| Where | Lines | Reason |
|---|---|---|
| `docs/adr/0011-odd-lite.md` | 104, 108-111, 113, 115 | the mapping: its From column, the section around it and the resolved note, kept as the record |
| `specs/changes/odd-lite/apply-progress.md` | 5, 10, 15, 22, 23, 24, 27, 47, 57 | Light spec block and earlier notes, dated record; the block is not edited |
| `specs/changes/odd-lite/verify-report.md` | 34, 49, 88, 150, 170, 172, 280, 285 | past verify rounds, dated evidence |
| `specs/changes/odd-lite/verify-report.md` | 335, 342, 353, 386, 393, 409, 418, 419 | the report of verify round 1 of this delivery, which quotes the old names |
| `specs/changes/odd-lite/verification.json` | 100 | finding summary of that same round |
| `CHANGELOG.md` | 11, 24 | past release text, capitalised tool name |
| `docs/methodology/odd-ligero-migracion.md` | 35 | owner's brief, names the third-party tool |

This section names those hits without quoting them, so it adds none.

Deviations
- `.gitignore` gained `__pycache__/` (coordinator request): the sync tests left untracked bytecode.
- `specs/README.md`: the Detalle row's "Slice card" became the outcome entry, and the intro gained a paragraph on `capabilities/`, `changes/` and `history/`.
- `AGENTS.md` gained "Conversation with the owner is in neutral Spanish" from the config file.
- ADR 0011: lines outside the mapping section moved to the new paths, including the inventory.
- `AGENTS.md` archive merge: the generic rule on a delta against an unarchived base stays; only its named changes and order went (`4e51eff`).
