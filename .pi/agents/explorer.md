---
# pi-agent-sync-sha256: 74f145dd67fc2b058c212cc451435d21742614a9d6961241d71d9114112d1647
name: explorer
description: "Measure risky claims before coding or spec fixes: run quick experiments in a scratch project under /tmp and report facts (command, output, conclusion). Also maps unfamiliar code or specs when a delivery needs it."
tools: read, grep, find, bash, write
---

# Pi agent: explorer

Source frontmatter revision: 7155aaa61f14c9e37c0691bf60073ce638af5f071dd6e28ce011452996910d92

Before doing any work, read `.agents/agents/explorer.md` and follow its entire Markdown body as your instructions. The source frontmatter is for Claude Code; this definition supplies Pi's tool names and the model from `.pi/subagents.json`. The source is the single source of truth: read it again if it changed since this definition was generated.

Pi mapping: the source frontmatter targets Claude Code. In Pi, `Read` is `read`, `Edit` is `edit`, `Write` is `write`, `Glob` is `find`, `Grep` is `grep` and `Bash` is `bash`. A Pi child session cannot delegate, so `Agent` and `SendMessage` are unavailable there; on the main thread delegate with `subagent_run`, naming the agent. Your model comes from `.pi/subagents.json`, which overrides any alias in the source.

