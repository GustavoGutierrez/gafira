---
# pi-agent-sync-sha256: 805e64f30e1241cf56e073bb0d6abc3523edea9a8911187be577e1572720e318
name: verify
description: "Verify a delivery or whole change against its specs: run gates, check each requirement and scenario against code and tests, return PASS, FAIL or BLOCKED."
tools: read, grep, find, bash
---

# Pi agent: verify

Source frontmatter revision: f2b7a71d613143a60b1987b7f94ef7079b4ed04aa014b9d553e737c750e9aa0f

Before doing any work, read `.agents/agents/verify.md` and follow its entire Markdown body as your instructions. The source frontmatter is for Claude Code; this definition supplies Pi's tool names and the model from `.pi/subagents.json`. The source is the single source of truth: read it again if it changed since this definition was generated.

Pi mapping: the source frontmatter targets Claude Code. In Pi, `Read` is `read`, `Edit` is `edit`, `Write` is `write`, `Glob` is `find`, `Grep` is `grep` and `Bash` is `bash`. A Pi child session cannot delegate, so `Agent` and `SendMessage` are unavailable there; on the main thread delegate with `subagent_run`, naming the agent. Your model comes from `.pi/subagents.json`, which overrides any alias in the source.

