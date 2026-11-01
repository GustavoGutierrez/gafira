## Light spec
Flow: light | level: L1 | pack: agent/skill definitions | mutation: off | tier: T0 | verify: V1 | decided-by: coordinator | reason: Bounded developer tooling for one runtime; no Rust, no product behavior, no format change.
Scope: Make the project agents of `.agents/agents/` usable in Pi, with a project-local synchronization tool and a project-local per-agent model selection. The sources stay the single source of truth for every runtime.
Non-goals: Changing `.agents/agents/` sources, changing global Pi configuration, unifying this tool with the OpenCode adapter, changing product specs, ADRs, or the Rust workspace.
Acceptance criteria:
  AC1 Given the project agent sources, When `python3 tools/sync_pi_agents.py` runs from the project root, Then it writes `.pi/agents/*.md` for every non-coordinator agent and `.pi/prompts/coordinator.md` for the coordinator, and it reads and writes only inside the project. [command]
  AC2 Given the generated definitions, When Pi discovers the project agents, Then each agent exposes Pi tool names (`read`, `edit`, `write`, `find`, `grep`, `bash`), points at its source file so the agent follows the current source instructions, and `verify` and `mutants` keep read-only tool sets. [command]
  AC3 Given a changed source, When `python3 tools/sync_pi_agents.py --check` runs, Then it reports the drift and exits nonzero without writing; a changed source frontmatter is drift that needs a resynchronization, a changed source body is not, because the definition points at the source; a destination that was edited by hand is reported as a conflict and never overwritten. [command]
  AC4 Given `.pi/subagents.json`, When Pi resolves an agent's profile, Then each project agent gets the model and effort configured for it, the file is project local, and no global agent directory is read or written. [command]
  AC5 Given the coordinator, When a Pi session in this project runs it, Then it is available on the main thread as `/coordinator` and can delegate to the other project agents by name. [manual]
  AC6 Given the diff, Then it touches no `crates/`, `apps/`, `docs/specs/` or `openspec/changes/archive/` path, and it introduces no absolute or user path of its own: the paths it writes are the repository-relative source and configuration paths it references, and a path that a source file already contains stays the source's own text. [command]
Plan: Write the failing tests, implement the synchronization script and the model configuration, generate the definitions, document the usage, then verify the acceptance criteria independently.
Risk and reversibility: Local tooling only; delete `.pi/`, `tools/sync_pi_agents.py` and the test file to revert. Owner decisions touched: none.
Escalation triggers watched: source format ambiguity, a model that cannot be resolved in the Pi catalog, or a delivery size beyond the light-mode signal.

## Apply: Pi project agents

Status: done. Branch `chore/pi-agent-sync`. No Rust touched; cargo gates not run. CI has no step that scans `.pi/`, so no policy step applied.

The OpenCode adapter for the same sources lives in a separate branch and is not
touched here; both tools read `.agents/agents/` and write to different
destinations (`.pi/` and `.opencode/`).

### The definitions point at the source, they do not copy it

A generated definition carries the derived frontmatter (name, description and
Pi tool names) and a pointer to the source file. The agent reads the source at
run time and follows it. Two consequences:

- Editing an agent's instructions needs no resynchronization: the definition
  points at the source, and the source is the single source of truth.
- Only a frontmatter change (a new tool, a new description) is drift that
  needs a resynchronization, and every definition records the frontmatter
  revision it was built from so that drift is visible by reading the file.

`python3 tools/sync_pi_agents.py --revision <agent>` prints the current
revision of a source, to compare against the recorded one.

### The verification round 1 findings

Verify round 1 returned FAIL with one CRITICAL and three WARNING items. The
CRITICAL item was a user path written into a tracked file, now removed. The
WARNING items were the delivery size, a commit that did not pass the gates
alone, and a contradiction between two acceptance criteria. The size and the
copy semantics had the same cause: copying the source body into the
definition. Pointing at the source instead removed both, and it also made a
source body edit a non-event for the generated files.

| AC | Evidence |
| --- | --- |
| AC1 | `python3 tools/sync_pi_agents.py` writes 8 definitions under `.pi/agents/` plus `.pi/prompts/coordinator.md`; the diff touches no path outside `.pi/`, `tools/`, `tests/` and this change folder |
| AC2 | The runtime resolves 8 project agents with no diagnostic; `verify` and `mutants` resolve to the read-only tool sets `read, grep, find, bash` and `bash, read, grep, find`; every definition names the source file it reads |
| AC3 | `--check` exits 1 and names the stale destination after a frontmatter change; it exits 0 after a body change, because the definition points at the source; it exits 2 without writing when a destination was edited by hand; `tests/test_pi_agent_sync.py` covers all three |
| AC4 | `.pi/subagents.json` resolves each project agent to its configured model and effort through the runtime's own resolver; the script writes only inside the project and the global agent directory is never touched |
| AC5 | `loadPromptTemplates` from the Pi runtime discovers `coordinator` with its description and body; the main thread delegates with `subagent_run`, and the agent definitions are discovered by the runtime from `.pi/agents/` |
| AC6 | The diff touches no `crates/`, `apps/`, `docs/specs/` or `openspec/changes/archive/` path. The paths the definitions write are repository-relative (` .agents/agents/<agent>.md`, `.pi/subagents.json`); the `/tmp` path appears only inside the source description it derives from, and the user path that verify round 1 found is gone |

Tests: `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p 'test_pi_agent*.py'` runs 26 tests, all passing. `ruff check` is clean on the script and the tests.

Models verified by probe (`pi -p --model <provider/id>`): `opencode-go/deepseek-v4.1-flash` and `opencode-go/minimax-m3.1-flash` both answer. `opencode-go/beam-free` also answers and costs nothing, but it is the weakest model in the catalog and is not the default. The free endpoints `minimax-m3.1-flash-free` and `longcat-2.5-preview-free` answer `Model is unavailable`, and `iceman-contributor-free` and `muse-spark-*-contributor` are blocked by the workspace privacy setting because they train on request data.
