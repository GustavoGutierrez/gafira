# Project agents and skills

`.agents/` is the single source of truth for every agent runtime:

```
.agents/agents/<name>.md        shared agent definitions
.agents/skills/<name>/SKILL.md  shared skills
```

Claude Code, OpenCode and Pi read the same files through relative symlinks;
nothing is generated:

| Runtime | Agents | Skills |
|---|---|---|
| Claude Code | `.claude/agents -> ../.agents/agents` | `.claude/skills -> ../.agents/skills` |
| OpenCode | `.opencode/agents -> ../.agents/agents` | `.opencode/skills -> ../.agents/skills` |
| Pi | `.pi/agents -> ../.agents/agents` | `.pi/skills -> ../.agents/skills` |

Never edit agents or skills under `.claude/`, `.opencode/` or `.pi/`; edit
`.agents/`. After any change in `.agents/`, run from the project root:

```sh
python3 tools/sync_agents.py          # create or repair the links
python3 tools/sync_agents.py --check  # exit 1 on drift, 2 on an invalid source or conflict
```

The script creates missing links, repairs wrong or absolute ones and removes
the adapters older versions generated, but only while they carry their unedited
marker. A real file or directory with other content where a link belongs is a
conflict: it is reported and left alone. `--check` changes nothing. On Windows,
Git needs symlink support (`core.symlinks=true` and developer mode) for these
links to check out as links.

## Shared agent frontmatter

One file serves three runtimes, so its frontmatter is limited to keys all of
them accept. The script rejects anything else:

```yaml
---
name: manual-writer              # matches the file name
description: "One line, double quoted, no inner quote or backslash."
mode: subagent                   # OpenCode: subagent, not primary
subagent_mode: task              # Pi: task or background; read before `mode`
---
```

Why: OpenCode V2 drops a file whose `tools` is a string and treats an agent
without `mode` as primary; Pi rejects `mode: subagent` unless `subagent_mode`
is set, and its line parser reads a folded (`>`) description as `>`; Claude
Code ignores keys it does not know. The body names no runtime-specific tool.

Per-runtime settings live in each runtime's own config, never in the shared
file:

- Models. Claude Code: the subagent inherits the session model. OpenCode:
  `.opencode/opencode.jsonc` (template `.opencode/opencode.example.jsonc`,
  `provider/model#variant`). Pi: `.pi/subagents.json` (see `.pi/README.md`).
- Tools and permissions. No allowlist is set: each runtime applies its defaults
  (Claude Code: all subagent tools; OpenCode: its default permission rules; Pi:
  its default tools). Shell access is never a sandbox.

## Agents

`manual-writer` is the only project agent; it updates the technical manual when
the owner asks. Specs are implemented by the main agent directly (`AGENTS.md`,
ADR 0018).

Tests: `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p 'test_agent*.py' -v`.
Set `GAFIRA_TEST_OPENCODE=1` to also check, on a scratch copy, that OpenCode
loads every shared agent as a subagent and applies a project model override.
