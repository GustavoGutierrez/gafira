# Pi agents for Gafira

Pi reads the shared definitions in `.agents/` through symlinks, like Claude
Code and OpenCode (`.agents/README.md`):

```
.pi/agents -> ../.agents/agents   shared agent definitions
.pi/skills -> ../.agents/skills   shared skills (Pi also reads .agents/skills; the same file loads once)
.pi/subagents.json                per-agent model selection (project local)
```

Do not put files in `.pi/agents/` or `.pi/skills/`: edit `.agents/` and run
`python3 tools/sync_agents.py` (and `--check`). Today the only project agent is
`manual-writer` (technical manual); specs are implemented by the main agent
directly (`AGENTS.md`, ADR 0018).

## How Pi reads a shared definition

The subagent loader reads `name`, `description` and the mode, taking
`subagent_mode` before `mode`, so `subagent_mode: task` keeps OpenCode's
`mode: subagent` from being rejected. The shared files set no `tools`, so Pi
starts the agent with its default tools, and no `model`, so the model comes
from `.pi/subagents.json`. A Pi child session cannot delegate; on the main
thread, delegate with `subagent_run`, naming the agent.

## Choose project-local models

`.pi/subagents.json` selects the model per agent. It is project local: Pi
merges it over the user-level agent configuration, and this project never
writes to the user agent directory.

```json
{
  "default_model": "provider/model",
  "model_profiles": {
    "manual-writer": { "model": "provider/model", "effort": "medium" }
  }
}
```

`model` is `provider/id`; `effort` is one of `off`, `minimal`, `low`, `medium`,
`high`, `xhigh`, `max`. An agent without a profile inherits `default_model`.
Every shared agent must have a profile (`tests/test_agent_models.py`).

## Tests

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p 'test_agent*.py' -v
```
