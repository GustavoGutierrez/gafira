# 0011. ODD lite: outcome-first routing on top of the existing flow

- Status: proposed (until the D3 pilot of `layout-core` ends)
- Date: 2026-11-01
- Related: ADR 0001, 0002, 0003, 0004 (amended, not removed), `AGENTS.md`, `.agents/agents/`, `docs/methodology/odd-ligero-migracion.md` (owner's brief), changes `odd-lite`, `layout-core`

## Context

The owner asked to adopt ODD lite (the brief above) through safe, reversible adjustments. The flow already has what the brief asks of an orchestrator, an implementer and a verifier: modes, packs, floors, gates G0 to G5, `verify` and graded mutation. What it lacks is a risk router that is independent of the request type, a resumable record that is not a change folder, and a verdict for a check that could not run. Every change still needs a folder under `specs/changes/`.

## Decision

Add ODD lite on top of the existing machinery, and change no safety rule. Phases A and B of the brief are done by change `odd-lite`; phase C is the pilot below; phase D waits for its result.

1. Levels. The coordinator classifies each request L0 to L3 by impact, uncertainty and reversibility, never by file count. L0 is the light docs-style path, L1 light, L2 standard, L3 full. The mode is the heaviest of floor, level and pack default, so a level can raise rigour and never goes below a floor.
2. Change folder. New work on a capability an existing spec covers needs no `specs/changes/<change>/` by default. Its criteria are an entry in `.odd/state.json`, or the spec row when requirements change, and a committed entry satisfies G0. Full mode, L3 and work no spec covers keep a folder as today.
3. State. `.odd/state.json` is tracked and small, with one object per active outcome. It is a pointer: on resume, branch, diff and tests win over it.
4. Mutation. T4 at close stays mandatory on critical code. L0 and L1 work that touches no critical code may run `mutation: off`. Hand mutants stay on critical code.
5. Verdict. `verify` gains BLOCKED: a critical check could not run. It escalates to the owner, never counts as PASS and costs no round.
6. No wrappers. No new skill or command; natural-language requests stay the entry point (see "Brief names").

### Inventory (brief §3, as of `develop` at `ffb0836`)

| Area | Found |
|---|---|
| Instructions | `AGENTS.md` (methodology), `CLAUDE.md` (Claude Code entry; its overrides beat the user-level file). No `CONTRIBUTING.md`, IDE rules or nested instruction files |
| Agents | 9 in `.agents/agents/` (`.claude/agents` is a symlink). `coordinator` is the main thread and routes; `specifier` and `architect` are upstream and optional; `explorer` measures claims; `tdd-apply` writes code; `mutants` and `verify` are read-only on source; `archive` closes; `manual-writer` is outside the spec flow. Every agent returns to the coordinator |
| Skills | 5 in `.agents/skills/` (`.claude/skills` is a symlink): `gafira-brand`, `git-workflow-and-versioning`, `product-specs`, `skill-registry`, `technical-manual`. Global skills resolve by name through the gitignored `.atl/skill-registry.md`. Subagents read them, they cannot invoke them. No `.claude/commands` |
| Current flow | Fast lane by default (ADR 0002): Slice card, `tdd-apply`, `verify` V2, T4 per touched crate, `archive`. Light spec for other requests. Full adds `architect`. `proposal.md`, `tasks.md` and delta specs are already off by default |
| Docs | `specs/` (one active set, `backlog/`, `archive/`; owner decisions in `00-decisiones.md`), `docs/adr/`, `specs/capabilities/` (record of past changes, no longer grows), `specs/changes/` and its archive. The duplication left is the change folder itself |
| Execution | Agent tool with one writer at a time; worktrees only for parallel deliveries. `.claude/settings.json` holds permissions and no hooks. State was the change folder and git. Scripts: `refresh.sh`, `manual.py`, `generar_manual.py` |
| Tests | 8 crates and 1 app; 911 `#[test]` attributes in 85 files (`grep -rhE '#\[test\]' crates apps --include=*.rs`). `cargo fmt`, clippy, `cargo test`; CI jobs `test` and `policy` (V5, V12, V13, V14; `cargo-deny` is a step of `policy`); CI is disabled for now (follow-up 2). `cargo-mutants` is local and not a CI gate. No CI step scans docs, so `.odd/` meets no policy check |

### Disposition

| Component | Action | Reason | Impact | Check |
|---|---|---|---|---|
| `coordinator` | adapt | single point of decision; needs L0-L3 and the state rule | new router and resume steps; floors and owner stops unchanged; gates G0 and G4 gain text additively | read the floors and gates tables against `develop` |
| `specifier` + skill `product-specs` | adapt | is the brief's `odd-spec` | runs when requirements change; acceptance source wording | `git diff develop -- .agents/agents/specifier.md` |
| `tdd-apply` | keep, small edit | is the implementer of `odd-run` | acceptance source and notes without a folder | "no improvising" lines still present |
| `verify` | adapt | is `odd-verify`; needs BLOCKED | new verdict, `blocked` field, entry checks | old `verification.json` files stay valid |
| `mutants` | keep, small edit | optional by risk, as the brief says | states when `off` is allowed | T4 tier text unchanged |
| `archive` | adapt | must close an outcome with no folder | new no-folder path | steps 1 and 2 unchanged |
| `explorer` | keep, small edit | `spike.md` needs a folder | no-folder runs return the result only | one line |
| `architect`, `manual-writer` | keep | unaffected; full mode keeps its folder | none | no diff |
| Modes, packs, floors, gates, tiers, depths | keep, additive edits | they already do what the brief asks | level sits above them; G0 accepts an outcome entry, G4 rejects BLOCKED, and the Spec and Runs cells of the `AGENTS.md` modes table name the outcome entry | floors and packs tables have no diff; the gates table and the `AGENTS.md` modes table changed additively, with nothing removed |
| Slice card and Light spec | keep, default narrowed | still the form for folders in progress, L3, uncovered work and legacy | fields move to the entry on covered work | `flow: legacy` restores them |
| `specs/changes/<change>/` | adapt | the document the brief removes by default | optional on covered work | AC4 |
| `.odd/state.json` | add | no resumable record existed outside a folder | one tracked file | `python3 -m json.tool .odd/state.json` |
| `.agents/skills/*` other than `product-specs` | keep | not affected | none | no diff |
| Change history, ADRs 0001 to 0004 | keep | history stays | amended below, on acceptance | no diff |
| Judge, Gherkin author, PRP author | not present | the brief names them only "if they exist" | none | none |

Nothing is deprecated. Deprecation belongs to phase D.

### Brief names

| Brief | Maps to |
|---|---|
| `odd-spec` | `specifier` with skill `product-specs` |
| `odd-run` | `coordinator` with `tdd-apply` |
| `odd-verify` | `verify` |
| `/odd` commands | natural-language requests to the coordinator |

### Amendments to ADRs 0001 to 0004

On acceptance these ADRs change only their status and index entry, as ADR 0004 did.

| ADR | Stays | Amended |
|---|---|---|
| 0001 | modes, packs, tiers, depths, gates, `verification.json`, retry caps, coordinator limits | decision record item 3: T4 "in every mode" now excepts L0 and L1 work with no critical code. Section 6: artifacts may live outside a folder. Coordinator write authority extends to `.odd/state.json`. Verdict list gains BLOCKED |
| 0002 | Flow line, mutation toggle, cargo-mutants speed, floors, verify defaults | section 3: on covered work the Slice card fields go in the outcome entry. Section 6: an outcome with no folder has nothing to move. The Flow line gains an optional `level` and `flow: legacy` |
| 0003 | all of it; branches and integration use the outcome id as `<change>` | nothing |
| 0004 | all of it | where `apply-progress.md` is named, a folderless delivery records the same items in the body of its last implementer commit. A BLOCKED verify is not a round |

### Retained flows

The old path stays available as an explicit option until phase D: the owner says `flow: legacy` and the change runs with its folder, Slice card or Light spec and no level. Delta specs, `proposal.md` and `design.md` stay available in full mode or on request. `layout-core` keeps its folder, card and design until archive, and its deliveries D3 to D7 run with minimal notes.

### Phase C: pilot

Delivery D3 of `layout-core`, under the coordinator, with no new artifacts beyond the entry update: no new folder, no new card, no new plan. Evidence to collect: the resume checks ran first, `verify` gave a reproducible verdict, nothing was redrafted.

### Phase D: criteria

Consolidate only when all hold, by owner decision:
- D3 closed PASS with the rules above, and any rule that misfired is fixed.
- One outcome ran end to end with no change folder and reached a `verify` verdict.
- A session resumed from `.odd/state.json` with no duplicated artifact.
- FAIL counts and rounds are no worse than comparable deliveries (D1 two rounds, D2 three).

Then accept this ADR, update ADR statuses, mark what proves redundant as legacy, and bring the docs this change could not touch into line: "Cómo continuar el documento en curso" in `specs/README.md` and `docs/manuals/tecnico/chapters*/10-methodology.md` still describe the Slice card route. History is not deleted without the owner's OK.

### Phase D: requirements

From `verify-report.md`, round 1 of `odd-lite`. All three resolved by delivery `chore/odd-lite-phase-d`.

- W1, resolved: the entry's `verify` field stores findings with their classes and one row per run, committed on the delivery branch before routing, so a later session can check closure, route and apply the stops.
- W2, resolved: `mutation: off` for L0 and L1 with no critical code is the named exception to "never lowers a pack default", and "heaviest of" fixes the mode only; depth and agents come from the pack row and a level only raises them.
- W3, resolved: `verify` step 3d names the product ID check and the finding classes of step 3c.

### Phase D: consolidate into `specs/`

Owner request, 2026-11-01. After `layout-core` is archived, on its own branch, consolidate into one root `specs/` folder and remove `openspec/`.

| From | To |
|---|---|
| `docs/specs/*`, with `backlog/` and `archive/` | `specs/` |
| `openspec/specs/` | `specs/capabilities/` |
| `openspec/changes/archive/` | `specs/history/` |
| `openspec/config.yaml` content | `AGENTS.md`, or `.odd/` if it does not fit |

Use `git mv` to keep history. References to update, at the time of writing and outside the two folders being moved: 29 files cite `openspec/` and 30 cite `docs/specs`, including `AGENTS.md`, agents, skills and the technical manual. `verify` checks that no broken reference remains. No file under `crates/`, `apps/` or `.github/` cites these paths. This task also covers the stale "Cómo continuar el documento en curso" section and the manual chapters named above.

Resolved by delivery `chore/specs-consolidation`. Active folders moved to `specs/changes/<change>/`, which holds L3, full mode, work no spec covers and `flow: legacy`; a change closes into `specs/history/<date>-<change>/`. `openspec/config.yaml` went to `AGENTS.md` and was deleted. `specs/history/`, `specs/capabilities/` and ADRs 0001 to 0010 were only renamed: their text stays byte for byte and they are exempt from the reference check. The From column above is kept as the record.

### Rollback

1. Revert the merge commit of `chore/odd-lite` on `develop`.
2. Delete `.odd/`.
3. Nothing else depends on it. An outcome that ran without a folder keeps its evidence in git; if one is active, copy its criteria to a Light spec or Slice card first.

### Metrics

| Metric | Value |
|---|---|
| Time to first working change, time to verified delivery | not measured |
| Total tokens, documentation tokens | not measured |
| Retries and defects | not measured as a before/after pair; baseline available in `verification.json` of `layout-core` (D2: FAIL, FAIL, PASS) |
| Regressions | not measured |
| Artifacts per change, before | measured, see below |

Artifact counts come from `find <folder> -type f | wc -l` and `find <folder> -type f -exec cat {} + | wc -l` on the working tree. The 10 archived changes hold 3 to 12 files (median 8) and 135 to 4598 lines. The three fast-lane-era ones hold 3 files each (135 to 352 lines). `layout-core` holds 7 files and 1121 lines. The pilot gives no "after" number by itself; no improvement is claimed.

## Consequences

Easier:
- Routine work on a covered capability starts from one entry instead of a folder.
- A resumed session reads one file and checks it against git.
- A check that cannot run has an honest outcome.

Harder and risks:
- `.odd/state.json` can drift from reality. Mitigation: the resume rule; git wins.
- Folderless work has no tracked `verify-report.md`. The entry keeps the findings, classes and rounds until close and git keeps them after it, while the full report stays in the session scratchpad and is lost with it. Mitigation: full mode and L3 keep folders.
- Misclassified levels. Mitigation: floors win, and escalation raises at any time.
- Two paths exist until phase D. Mitigation: `flow: legacy` is explicit and recorded.
- `specs/README.md` and the manual lag until phase D (see above).

## Alternatives considered

- New `odd-spec`, `odd-run`, `odd-verify` skills or commands. Rejected: they would wrap existing agents; the brief warns against wrappers with no value.
- Repartition `specs/` into 3 to 5 capability specs. Rejected for now: a rewrite with no behavior behind it; the brief says not to migrate history by force.
- Replace modes and packs with L0-L3. Rejected: the floors and packs encode real findings; levels select among them.
- Keep state only in the change folder. Rejected: it keeps a folder per change, the cost the brief targets.
- SQLite or an external memory tool. Rejected: the brief says to start with a small file, and state stays in the repo.
