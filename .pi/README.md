# Pi agents for Gafira

`.agents/agents/` is the source of truth for every runtime. Pi reads the
definitions generated from it, so Pi, Claude Code and OpenCode all follow the
same instructions.

```
.agents/agents/<name>.md   source (Claude Code format)
.pi/agents/<name>.md       generated Pi subagent definition
.pi/prompts/coordinator.md generated Pi prompt template (main thread)
.pi/subagents.json         per-agent model selection (project local)
```

## Synchronize

After changing a source, run from the project root:

```sh
python3 tools/sync_pi_agents.py
python3 tools/sync_pi_agents.py --check
```

Commit the changed definitions with the source. `--check` exits nonzero for
drift without writing. The script rejects unsupported source formats, symlinks
and unmanaged or manually edited destination collisions before applying
changes. It reports generated destinations with no source and never deletes
them: inspect and remove obsolete definitions yourself, then rerun. Do not edit
generated files; edit their sources instead.

A generated definition does not copy the body of its source. It points at the
source file, and the agent reads it at run time, so editing an agent's
instructions takes effect immediately and needs no resynchronization. Only the
frontmatter is derived, and every definition records its revision:

```text
Source frontmatter revision: 8f14e45fceea167a5a36dedd4bea2543
```

When the source frontmatter changes (a new tool, a new description), the
definition must be regenerated. Compare the recorded revision with the current
one to spot that without running the tool:

```sh
python3 tools/sync_pi_agents.py --revision verify
```

## Run the coordinator

Pi has no `--agent` flag, and a Pi child session cannot delegate. The
coordinator therefore runs on the main thread as a prompt template:

```text
/coordinator
```

The template expands the coordinator's instructions into the current session.
The main thread then delegates with `subagent_run`, naming the project agent
it wants: `specifier`, `architect`, `explorer`, `tdd-apply`, `mutants`,
`verify`, `archive` or `manual-writer`.

The other agents are ordinary Pi subagents, dispatchable by name from any Pi
session in this project.

## Choose project-local models

`.pi/subagents.json` selects the model per agent. It is project local: Pi
merges it over the user-level agent configuration, and this project never
writes to the user agent directory.

```json
{
  "default_model": "opencode-go/deepseek-v4.1-flash",
  "model_profiles": {
    "tdd-apply": { "model": "opencode-go/deepseek-v4.1-flash", "effort": "high" },
    "explorer": { "model": "opencode-go/minimax-m3.1-flash", "effort": "medium" }
  }
}
```

`model` is `provider/id`; `effort` is one of `off`, `minimal`, `low`, `medium`,
`high`, `xhigh`, `max`. An agent without a profile inherits `default_model`.
Remove an agent's entry to send it back to the inherited model.

Both configured models are served by `opencode-go`:

| Model | Context | Cost (in/out per Mtok) | Notes |
|---|---|---|---|
| `opencode-go/deepseek-v4.1-flash` | 1M | 0.15 / 0.6 | Default. Supports `low`, `high` and `max` effort; `medium` and `xhigh` are not mapped. |
| `opencode-go/minimax-m3.1-flash` | 1M | 0.15 / 0.6 | Supports every effort level. |

The `minimax` provider also serves `minimax/MiniMax-M3.1-Flash` directly with
the same price, and it accepts the `medium` effort level that
`opencode-go/deepseek-v4.1-flash` does not.

Free endpoints in the catalog are not usable here: `minimax-m3.1-flash-free`
and `longcat-2.5-preview-free` answer `Model is unavailable`, and
`iceman-contributor-free` and the `muse-spark-*-contributor` models are
blocked by the workspace privacy setting because they train on request data.
`opencode-go/beam-free` responds and costs nothing, but it is the weakest of
the available models and is not used by default.

## Tool mapping

The source frontmatter names Claude Code tools. The generated definitions use
Pi's names:

| Claude Code | Pi |
|---|---|
| `Read` | `read` |
| `Edit` | `edit` |
| `Write` | `write` |
| `Glob` | `find` |
| `Grep` | `grep` |
| `Bash` | `bash` |

`Agent` and `SendMessage` have no child-session equivalent in Pi and are
dropped. `verify` and `mutants` keep read-only tool sets; `explorer` keeps
write access because its spike file is part of the flow.

## Limitations

The generated definitions describe the same work as the sources, not the same
runtime: a Pi child session cannot launch other children, and Pi has no
equivalent of Claude Code's `Agent` permission model. The coordinator's
workflow stays valid because it runs on the main thread. Shell is not a
sandbox: an agent with `bash` can write outside its intended paths, exactly as
in the other runtimes.

## Tests

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p 'test_pi_agent*.py' -v
```
