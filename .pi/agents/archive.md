---
# pi-agent-sync-sha256: 22a51e2664754a06f0e7df485c5dbede52c914d93385c755e7a4d83d87662c75
name: archive
description: "Close a change after a PASS verdict: move the change folder to the archive; fast-lane changes update the docs/specs/README.md status table with the covered IDs; changes with delta specs merge them into openspec/specs."
tools: read, edit, write, find, grep, bash
---

# Pi agent: archive

Source frontmatter revision: 9b59c6896c398ed41bbe24051b2449d207649f65e352a421a26c54a0d90e85b9

Before doing any work, read `.agents/agents/archive.md` and follow its entire Markdown body as your instructions. The source frontmatter is for Claude Code; this definition supplies Pi's tool names and the model from `.pi/subagents.json`. The source is the single source of truth: read it again if it changed since this definition was generated.

Pi mapping: the source frontmatter targets Claude Code. In Pi, `Read` is `read`, `Edit` is `edit`, `Write` is `write`, `Glob` is `find`, `Grep` is `grep` and `Bash` is `bash`. A Pi child session cannot delegate, so `Agent` and `SendMessage` are unavailable there; on the main thread delegate with `subagent_run`, naming the agent. Your model comes from `.pi/subagents.json`, which overrides any alias in the source.

