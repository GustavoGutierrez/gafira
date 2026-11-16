# verify-report: pi-agent-sync

- Round: 2 (closure round for the findings of round 1)
- Verified commit: `696c0e9afbaf13b09de8ddab0364bec2c487064f` (`chore/pi-agent-sync`, 2 commits on top of merge-base `d3148bf`: `f5d7cdd`, `696c0e9`)
- Mode: light | Tier: T0 | Depth: V1 | Mutation: off (no `mutants` report exists; none was run)
- Acceptance source: Light spec block at the top of `openspec/changes/pi-agent-sync/apply-progress.md` (AC1 to AC6)
- Delivery size: 14 files, 910 insertions, 0 deletions (`git diff --shortstat develop...HEAD`; three-dot, so the commits `develop` gained after the branch was cut are not counted). No Rust, no Cargo file, nothing under `crates/`, `apps/`, `docs/specs/` or `openspec/changes/archive/`.
- Scope of this round: closure of C1, W1, W2 and W3, the gates at HEAD, and the artifact-integrity checks. No independent mutants, no repeat of the round-1 probes.

## Verdict: PASS

Zero CRITICAL. C1, W2 and W3 are closed and proved closed; W1 survives as a WARNING with a much smaller, non-mechanical residual, so it is carried as a follow-up rather than a FAIL. All six acceptance criteria are proved against the repository, and every gate that can observe this delivery passes.

## Closure of the round-1 findings

| Item | Status | Proof in this round |
|---|---|---|
| C1 `implementation` (user path `~/.pi/agent/subagents.json` in `.pi/README.md`) | **closed** | `.pi/README.md:66-68` now reads "It is project local: Pi merges it over the user-level agent configuration, and this project never writes to the user agent directory." A scan of all 14 delivered files for a home-directory marker (`~`, `$HOME`, `/home/`, `/Users/`, `/root/`) returns no user or absolute path. The only leftover matches are `~700` (a number in the architect description, inherited from the source) and `/tmp` inside `.pi/agents/explorer.md:4`, which is the source's own description text and is explicitly permitted by AC6. |
| W1 `implementation` (1292 insertions against the ~700 ceiling) | **open (residual)** | 1292 to 910 insertions: the ~900 lines of verbatim source copies are gone, and the definitions are now 15 lines each (8 agents, 120 lines) plus a 13-line coordinator prompt. The residual is 285 lines of script, 261 of tests, 133 of README, 38 of JSON and 60 of change documentation. Still above the ~700 signal and no owner OK is recorded in `apply-progress.md`. Cause removed, measure not. Carried as W1 below, marked follow-up. |
| W2 `implementation` (commit `ed5a4ee` failed the gate alone) | **closed** | History rewritten to two commits. A clean `git archive 696c0e9` export into a scratch tree (touched after extraction) gives `python3 tools/sync_pi_agents.py --check` exit 0 ("generated Pi agent definitions are up to date"), `python3 -m unittest discover -s tests -p 'test_pi_agent*.py'` 26 tests OK, and `ruff check tools tests` exit 0. The feat commit now passes the delivery gate on its own tree. |
| W3 `ambiguity` (AC6 contradicted AC2 on `/tmp`) | **closed** | The committed Light spec (`f5d7cdd`) carries the scoped criteria: AC2 requires that the definition "points at its source file so the agent follows the current source instructions" (no body copy), and AC6 scopes the path rule to "no absolute or user path of its own", with "a path that a source file already contains stays the source's own text". The delivery complies: the generated files carry only repository-relative paths (`.agents/agents/<name>.md`, `.pi/subagents.json`), and `/tmp` reaches `.pi/agents/explorer.md` only through the derived description of `.agents/agents/explorer.md:3-5`. The contradiction is gone because the copy requirement is gone. |

Integration check that follows from W2 and W3: `develop` is 11 commits ahead of the merge base and four sources differ there (`archive.md`, `coordinator.md`, `tdd-apply.md`, `verify.md`). I merged `develop`'s versions of those four sources over a scratch copy of the branch's `.pi/` and re-ran `--check`: exit 0. The revision hash covers the frontmatter only, and those four sources changed their bodies and not their frontmatter, so the merge produces no drift and `develop` will not need a resync at merge time.

## Gates

| Gate | Result | Note |
|---|---|---|
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p 'test_pi_agent*.py' -v` | exit 0 | 26 tests, OK. Also exit 0 in the clean export of `696c0e9`. |
| `python3 tools/sync_pi_agents.py --check` | exit 0 | "generated Pi agent definitions are up to date", at HEAD and in the export. |
| `ruff check tools tests` | exit 0 | All checks passed. |
| `cargo fmt --check` | exit 0 | No Rust in the delivery, but the gate was run at HEAD as instructed. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 | Finished with no warnings. |
| `cargo test --workspace --locked` | exit 0 | Whole workspace green, doc-tests included. |
| `cargo deny check` | exit 0 | advisories, bans, licenses, sources ok. |
| CI policy steps (`.github/workflows/ci.yml`) | not applicable (`null`) | Every step of the `policy` job (UI dependency check, `unsafe` ban, layer table, `clippy.toml` sync, `EditToken`, property tests) reads only `crates/`, and the `test` job runs cargo. Nothing in CI reads `.pi/`, `tools/` or `tests/`, so no policy step can observe this delivery. Not a block: their subjects are untouched. |
| Disk before the run | ok | 48 GB free on `/`; no cargo export, one `git archive` export of `696c0e9` and two small scratch trees under `/tmp`. |

## Artifact integrity

- **Light spec commit (G0)** `f5d7cdd docs(pi-agent-sync): specify project-local Pi agent definitions` touches exactly one file, `openspec/changes/pi-agent-sync/apply-progress.md` (+14), and precedes the only implementation commit. Correct.
- **Light spec block unchanged afterwards**: `696c0e9` appends from line 15 (`## Apply:` and the `| AC | Evidence |` table); `diff` of the `f5d7cdd` blob against the `HEAD` blob reports a single hunk, `14a15,60`, so lines 1 to 14 are byte-identical. Only two commits in the branch touch the file, so no implementer commit rewrote the block.
- **No forbidden path**: `git diff --name-only develop...HEAD` lists 14 files, none under `crates/`, `apps/`, `docs/specs/` or `openspec/changes/archive/`. The non-goal "no source change" holds: `.agents/agents/` is untouched by both commits.
- **Commit hygiene**: both messages are conventional (`docs(...)`, `feat(...)`), carry a scope/body, and no commit has an AI attribution or a `Co-Authored-By` trailer.
- Working tree clean before and after this verify, apart from the two artifacts this stage owns (`verify-report.md`, `verification.json`), which are still untracked (S6).

## Per-criterion proof (every evidence claim in apply-progress.md checked against the repository)

| AC | Status | Proof found |
|---|---|---|
| AC1 | covered | Independent probe on a scratch tree holding only `.agents/` and `tools/`: `python3 tools/sync_pi_agents.py` writes exactly `.pi/agents/{architect,archive,explorer,manual-writer,mutants,specifier,tdd-apply,verify}.md` and `.pi/prompts/coordinator.md`, nine artifacts, and adds no other file. The coordinator gets a prompt, no agent definition. Code reading of `tools/sync_pi_agents.py`: reads only `<root>/.agents/agents/*.md`, writes only under `<root>/.pi/` with the temp file created in the destination directory (`write_atomic`); no other filesystem access and no hard-coded path. `test_generate_writes_agents_and_coordinator_prompt` states the same rule. |
| AC2 | covered | The eight definitions carry Pi names only (`explorer` `read, grep, find, bash, write`; `verify` `read, grep, find, bash`; `mutants` `bash, read, grep, find`; the rest `read, edit, write, find, grep, bash`), and no Claude tool name survives on a `tools:` line. Each body is a pointer, not a copy: "read `.agents/agents/<name>.md` and follow its entire Markdown body as your instructions", plus the Pi mapping note. Read against the real parser (`gentle-pi/lib/agents-config.ts`: `parseFrontmatter`, `parseAgentDefinition`): the frontmatter delimiter matches, the `# pi-agent-sync-sha256:` line does not match the `key: value` regex and is skipped, the body is non-empty, and `tools` parses to the listed names, so all 8 definitions resolve with no error. |
| AC3 | covered | Three independent probes on a scratch tree: appending a paragraph to a source **body** gives `--check` exit 0 ("up to date"); adding `Edit` to `verify`'s source **tools** gives exit 1 with `stale destination: .../verify.md`; hand-editing a generated destination with an extra tool gives exit 2, `unmanaged or edited destination: ...`, and the file is left byte-identical. The suite states the same rules with five tests (`test_a_body_change_needs_no_resynchronization`, `test_a_frontmatter_change_needs_a_resynchronization`, `test_check_does_not_write`, `test_check_is_idempotent_and_detects_drift`, `test_unmanaged_destination_is_reported_and_not_overwritten`, `test_edited_generated_destination_is_a_conflict`). |
| AC4 | covered | `.pi/subagents.json` has `default_model`, `default_thinking_level` and a `model_profiles` entry with `model` + `effort` for each of the eight subagents. The keys match the runtime resolver (`gentle-pi/lib/agents-config.ts:264-297`: `default_model`, `default_effort ?? default_thinking_level`, per-agent `effort ?? thinking`), its `THINKING_LEVELS` are exactly the seven named in `.pi/README.md`, and the project file is read from `<cwd>/.pi/subagents.json` and merged over the agent-home copy, so nothing requires the global directory. Both model ids exist in `~/.pi/agent/models-store.json` under `opencode-go`, with a 1M context window as the README table says. `test_config_does_not_leak_global_state` and `test_every_project_agent_has_a_model_profile` cover the file. |
| AC5 | covered (manual) | `.pi/prompts/coordinator.md` carries a `description` frontmatter field, a non-empty body and no `name`, so `loadPromptTemplates` (pi runtime `dist/core/prompt-templates.js` + `dist/utils/frontmatter.js`, which parses the block with `yaml`) names it from the file and reads the description, with the marker line parsed as a YAML comment. The discovery probe is round 1's and is not repeated at V1; delegation (`subagent_run` by name) is a runtime property, and the eight definitions are discoverable (AC2). |
| AC6 | covered | `git diff --name-only develop...HEAD` lists no `crates/`, `apps/`, `docs/specs/` or `openspec/changes/archive/` path. A path scan over the 14 delivered files finds only repository-relative paths written by the tooling and, in `.pi/agents/explorer.md`, the `/tmp` that its source description already contains, which AC6 permits. No user path and no absolute path of the delivery's own. |

## CRITICAL

None.

## WARNING

- **W1** `class: implementation` — *(carried from round 1, residual, follow-up)* Delivery size is 910 insertions against the ~700-line signal in `AGENTS.md`, with no owner OK recorded in `apply-progress.md`. The round-1 cause is gone: the definitions no longer copy the source body, which removed roughly 900 lines and made the delivery 30% smaller. What remains is 285 lines of new script, 261 of tests, 133 of README, 38 of JSON and 60 of change documentation, none of it duplicated prose. Still, the measure is what the rule names, so it stays open as a signal for the coordinator to put to the owner in one line at archive (explicit OK) or to split in a later round. ref: `git diff --shortstat develop...HEAD`; `AGENTS.md` "Delivery rules".

## SUGGESTION

- **S1** `class: implementation` — *(carried, open)* No CI step runs the Python tests or `sync_pi_agents.py --check`, so a definition goes stale on `develop` whenever a source frontmatter changes and nobody runs the tool locally. One ubuntu step in the `policy` job running `python3 tools/sync_pi_agents.py --check` and the unittest command closes the loop. ref: `.github/workflows/ci.yml`.
- **S2** `class: ambiguity` — *(carried, open)* AC4 says "each project agent gets the model and effort", while the coordinator deliberately has no profile (it runs on the main thread with the session model). "each subagent" would match the design; `test_every_project_agent_has_a_model_profile` already excludes the coordinator explicitly.
- **S3** `class: implementation` — *(carried, out of scope)* `verify` and `mutants` keep the read-only source tool sets (`read, grep, find, bash`), while the verify instructions require writing `verify-report.md` and `verification.json`. Shell covers it (no sandbox), and the fix belongs in `.agents/agents/`, which this change must not touch.
- **S4** `class: claim` — *(carried)* The model probing recorded in `apply-progress.md` (both models answer; the free endpoints do not) and the README claim that `opencode-go/deepseek-v4.1-flash` does not map `medium` and `xhigh` were not re-run in this round; `models-store.json` carries no per-model effort mapping, so they cannot be checked without a request. Presence of both ids in the catalog is confirmed.
- **S5** — `.odd/state.json` holds only the `odd-lite` outcome; this change has a folder and a committed Light spec, so it is not a G0 gap, but a resumed session has no pointer to it in the ODD state. The coordinator's call whether the file needs an entry while a folder exists.
- **S6** — `verify-report.md` and `verification.json` from round 1 were still untracked; the round-2 versions are also untracked. The coordinator should commit both with the change before archiving.

## Mutation budget

`mutation: off` and tier T0: no `mutants` report exists and no mutation was run. V1 runs no hand mutants, and this closure round spends none, so `hand_mutants` is `null`.

next: archive
