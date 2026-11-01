---
# pi-agent-sync-sha256: 7429f40290cae9d6a570235b947395c6efa815abb36106c4c757ebcb6645252d
name: archive
description: "Close a change after a PASS verdict: move the change folder to the archive; fast-lane changes update the specs/README.md status table with the covered IDs; changes with delta specs merge them into specs/capabilities."
tools: read, edit, write, find, grep, bash
---

# Pi agent: archive

Source frontmatter revision: e79e4fe80b3526cb802b04aa4df33a1cac1bf27cc7fb5cd9e6d397e0c6213a1e

Before doing any work, read `.agents/agents/archive.md` and follow its entire Markdown body as your instructions. The source frontmatter is for Claude Code; this definition supplies Pi's tool names and the model from `.pi/subagents.json`. The source is the single source of truth: read it again if it changed since this definition was generated.

Pi mapping: the source frontmatter targets Claude Code. In Pi, `Read` is `read`, `Edit` is `edit`, `Write` is `write`, `Glob` is `find`, `Grep` is `grep` and `Bash` is `bash`. A Pi child session cannot delegate, so `Agent` and `SendMessage` are unavailable there; on the main thread delegate with `subagent_run`, naming the agent. Your model comes from `.pi/subagents.json`, which overrides any alias in the source.

