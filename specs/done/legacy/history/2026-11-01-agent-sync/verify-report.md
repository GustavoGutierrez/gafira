# Verify report: agent-sync

Round 3 (closure) | depth V1 | mode standard | level L2 | mutation archive | tier T0

Verified commit: `bc0ef6893d52619c4f6adae4bbbbb5b4d73fd6e9` on `chore/agent-sync`.

**Verdict: PASS** — 0 CRITICAL, 2 WARNING (`follow-up`), 0 SUGGESTION. AC1–AC6 are covered. The round 1 and round 2 blocking findings are closed.

No Rust crate changed. Mutation is `archive`; T4 does not apply because there is no touched Rust crate. No cargo-mutants or hand-mutant run was performed. The cross-platform CI matrix was not run locally; the change touches no Rust, Cargo, CI, or action-pin files.

## Acceptance criteria

| Criterion | Status | Evidence |
|---|---|---|
| AC1 | covered | `.claude/agents` is a tracked symlink to `../.agents/agents`. OpenCode V2 API resolved the coordinator as primary and all eight project agents as subagents. |
| AC2 | covered | Location-scoped agent API queries showed named coordinator delegation rules and no Claude model aliases. Round 2's isolated live test invoked `explorer`; the child session recorded a completed read of `.agents/agents/explorer.md` and inherited the selected model. Other agents were not live-executed. |
| AC3 | covered | The sync tests cover deterministic generation, source drift, check-only operation and updates. `python3 tools/sync_agents.py --check` exited 0 at the verified commit. |
| AC4 | covered | Sync tests cover unmanaged collisions, edited destinations, unsupported sources, stale adapters, symlinks and rollback. Independent scratch probes in round 2 confirmed a second-replacement failure restored destinations, a staging failure preserved them, and a stale symlink caused sync and `--check` to exit 2. |
| AC5 | covered | `.agents/README.md` documents synchronization, coordinator selection, model selection, permissions and recovery limitations; `.opencode/opencode.example.jsonc` is opt-in. |
| AC6 | covered | Round 3's independent location-scoped OpenCode API probe resolved project-local model overrides for all eight subagents while keeping adapter system prompts and permissions unchanged. An unset explorer had no agent model; OpenCode V2 documents parent-session inheritance. `.agents/README.md:49–52` documents the separately selected coordinator session model. |

## Gates and evidence at round 3

| Command or check | Result |
|---|---|
| `GAFIRA_TEST_OPENCODE=1 PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p 'test_agent*.py' -v` | exit 0; 19 tests passed, including the OpenCode API test |
| `PYTHONDONTWRITEBYTECODE=1 python3 tools/sync_agents.py --check` | exit 0; no output |
| `cargo fmt --check` | exit 0 |
| `CARGO_TARGET_DIR="$SCRATCH_TARGET" CARGO_INCREMENTAL=0 cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| `CARGO_TARGET_DIR="$SCRATCH_TARGET" CARGO_INCREMENTAL=0 cargo test --workspace --locked --quiet` | exit 0 |
| `git diff --check 84ff0ed bc0ef68` | exit 0 |
| `git status --porcelain=v1` | clean after verification |

The scratch API probe copied the source and adapters outside the worktree, wrote only a scratch `.opencode/opencode.jsonc`, initialized the location with `opencode api get '/api/agent?location%5Bdirectory%5D=…'`, and queried each `/api/agent/<id>` at that location. Both `probe/<agent>` and `probe/<agent>#high` model references resolved. Running sync and `--check` in the copy exited 0 without changing adapter hashes or the config. A second config left `explorer` unset and overrode `verify`; the API returned no explorer model and the configured verify model. These placeholder models were not invoked or validated against a provider. No credentials or model generation were used in round 3.

The focused API test itself created and removed a temporary nested Git repository in the worktree, including an empty scratch commit; it did not alter tracked files or the user branch. The separate independent probe used an external scratch directory.

## Artifact integrity and size

- `a65a30b` committed the original acceptance record alone before `7cd2477` implemented it. `a436246` separately recorded the escalation; `69a82aa` separately corrected the Flow and added AC6 before `bc0ef68` implemented AC6. Neither implementation commit changed the acceptance record.
- Original sync work unit: `git diff --shortstat 84ff0ed 39ab42b` = 644 net added lines. Model-selection unit: `git diff --shortstat 39ab42b bc0ef68` = 128 additions and 5 deletions. The combined change is 767 net added lines; see W1.

## Findings and limitations

### CRITICAL

None remain at round 3.

### WARNING

- **W1** class: implementation, `follow-up`. ref: `AGENTS.md:148`, delivery size. The two work units are individually below the approximately 700-line delivery signal, but their combined net diff is 767 lines. The coordinator must preserve or explicitly assess the delivery boundaries before integration.
- **W2** class: implementation, `follow-up`. ref: `.agents/agents/coordinator.md:101`, `69a82aa`. The separate acceptance amendment states why and touches only the acceptance file, but its subject is `docs(agent-sync): specify project-local model selection`, not the prescribed `docs(agent-sync): light spec amend` wording.

Other documented limitations: synchronization is not crash-atomic across files; a crash, power loss or failed rollback can require manual recovery. OpenCode combines Write, Edit and patch under the `edit` action, and shell is not a filesystem sandbox. Global or managed policy can further restrict runtime actions. Source instructions still depend on the model reading and following them. Round 2's real delegation was constrained by hard-deny write/shell policy and proved only the coordinator-to-explorer path.

## Historical rounds and closure

### Round 1 — FAIL at `7cd2477`

Depth V1, light/L1, mutation off, T0. Focused sync tests passed (10 tests), `--check` exited 0, and API discovery and a safe live coordinator-to-explorer delegation succeeded. Independent probes identified four blocking findings:

- **C1** class: implementation. `tools/sync_agents.py:60–82`: generated permissions broadened Claude tool allowlists, including explorer's Write-only source.
- **C2** class: implementation. `tools/sync_agents.py:132–139`: injected write failures could leave partially updated or truncated destination files.
- **C3** class: implementation. `tools/sync_agents.py:126–129`: a stale destination symlink was ignored and could leave `--check` exiting 0.
- **C4** class: implementation. `apply-progress.md:3`, `AGENTS.md:18–31`: the recorded light/L1 flow needed escalation after the measured size and safety findings.

Cargo gates were not run in round 1. Round 1 live delegation used an isolated read-only scratch session, performed source reads and returned the explorer result; it did not prove the full workflow.

### Round 2 — FAIL at `39ab42b`

Depth V1, standard/L2, mutation off as then recorded, T0. All 16 focused Python tests and `--check` passed. Independent staging and replacement failure injections preserved destination bytes, and the stale-symlink reproduction exited 2 for both sync and check. OpenCode V2 API showed the deny-first and specific allow rules. A new isolated model-driven coordinator-to-explorer delegation completed; the child session actually read its source. Workspace fmt, clippy and tests passed.

C1–C3 closed. C4 was only partially closed: the flow had been raised to standard/L2 but still selected `mutation: off` by the coordinator, contrary to `AGENTS.md:90`. This remaining blocker was closed by `69a82aa`, which records `mutation: archive` and notes that no Rust crate is touched.

next: archive (pass follow-ups W1 and W2 to the archive step)
