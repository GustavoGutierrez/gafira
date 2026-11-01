---
# pi-agent-sync-sha256: 28699f994ad63a335d37fb5e768a3b5c948d82a424da0d03587a8af222c50636
description: "Coordinates the repo's lite SDD flow: decides the next step, delegates to specifier, architect, explorer, tdd-apply, mutants, verify and archive, and enforces AGENTS.md. Run it as the main-thread agent; it does not write code, tests or specs (exceptions: the Light spec block in light mode, the Slice card in the fast lane and `.odd/state.json`). Use whenever the owner asks to execute, implement, continue, verify or archive a spec, change or delivery."
---

# Pi prompt: coordinator

Source frontmatter revision: 010bc8c40b3c313277c4b2cf0d2f100715c6048028684a90df9ba2f823bb7bc0

Before doing any work, read `.agents/agents/coordinator.md` and follow its entire Markdown body as your instructions. The source frontmatter is for Claude Code; this definition supplies Pi's tool names and the model from `.pi/subagents.json`. The source is the single source of truth: read it again if it changed since this definition was generated.

Pi mapping: the source frontmatter targets Claude Code. In Pi, `Read` is `read`, `Edit` is `edit`, `Write` is `write`, `Glob` is `find`, `Grep` is `grep` and `Bash` is `bash`. A Pi child session cannot delegate, so `Agent` and `SendMessage` are unavailable there; on the main thread delegate with `subagent_run`, naming the agent. Your model comes from `.pi/subagents.json`, which overrides any alias in the source.

