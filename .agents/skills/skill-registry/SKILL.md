---
name: skill-registry
description: >
  Create, read or extend the local skill registry `.atl/skill-registry.md`
  (gitignored). Trigger: resolve a skill name to its SKILL.md path, refresh the
  registry, register a new skill, or "skill not found".
---

Tracked files name skills by NAME only. Paths live in `.atl/skill-registry.md`, generated locally by `.agents/skills/skill-registry/refresh.sh` (bash, coreutils/awk/sed/find; no `gentle-ai`).

## Commands (run from anywhere in the repo)

| Need | Command |
|---|---|
| Create / refresh | `.agents/skills/skill-registry/refresh.sh` |
| Resolve names | `.agents/skills/skill-registry/refresh.sh resolve NAME...` prints `name<TAB>path`, or `name<TAB>unavailable` (exit 1) |
| Register a skill | put it in a scanned dir, or `.agents/skills/skill-registry/refresh.sh add DIR` (DIR = skills root or one skill dir; remembered in `.atl/extra-sources`) |

## Rules

- Sources: project `.claude/skills`, `.agents/skills`, then user dirs under `$HOME` (see "Sources scanned" in the registry). Name from SKILL.md front matter; duplicates: project wins, then first source.
- Missing registry or name: `resolve` refreshes once and retries. Still missing: report the name as unavailable. Never guess a path.
- Subagents cannot run the Skill tool: whoever launches one resolves the names and passes the paths; every agent reads each `SKILL.md` before working.
- Never copy resolved paths into tracked files.
