---
# pi-agent-sync-sha256: d0384d15001fe693e1b527d62bf3dcafa0f210eefc2bfac4537c9934c79688b5
name: tdd-apply
description: "Implement one delivery of a change straight from its specs with strict TDD (RED, GREEN, REFACTOR). Use when specs exist and code must be written."
tools: read, edit, write, find, grep, bash
---

# Pi agent: tdd-apply

Source frontmatter revision: 44cdbaedff19bf699674432a0e90c5ba995844070c2c377e385273b1df4a61de

Before doing any work, read `.agents/agents/tdd-apply.md` and follow its entire Markdown body as your instructions. The source frontmatter is for Claude Code; this definition supplies Pi's tool names and the model from `.pi/subagents.json`. The source is the single source of truth: read it again if it changed since this definition was generated.

Pi mapping: the source frontmatter targets Claude Code. In Pi, `Read` is `read`, `Edit` is `edit`, `Write` is `write`, `Glob` is `find`, `Grep` is `grep` and `Bash` is `bash`. A Pi child session cannot delegate, so `Agent` and `SendMessage` are unavailable there; on the main thread delegate with `subagent_run`, naming the agent. Your model comes from `.pi/subagents.json`, which overrides any alias in the source.

