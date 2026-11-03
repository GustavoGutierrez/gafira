---
# pi-agent-sync-sha256: 1ec7a86e2da9c1a5d2b7afa594e64712abd6b1600402bf41c6c50cf51a11dfd4
name: mutants
description: "Run cargo-mutants on a whole crate at change close (default), or on the lines or files a delivery changed when the Flow line says mutation: delivery, and report surviving mutants, classified critical or non-critical. Read-only on the repo; never edits code or specs."
tools: bash, read, grep, find
---

# Pi agent: mutants

Source frontmatter revision: 5646273e2f1eecfb48eb1edef4000b0d37fa538aca0e23944ebbdabfa9dde123

Before doing any work, read `.agents/agents/mutants.md` and follow its entire Markdown body as your instructions. The source frontmatter is for Claude Code; this definition supplies Pi's tool names and the model from `.pi/subagents.json`. The source is the single source of truth: read it again if it changed since this definition was generated.

Pi mapping: the source frontmatter targets Claude Code. In Pi, `Read` is `read`, `Edit` is `edit`, `Write` is `write`, `Glob` is `find`, `Grep` is `grep` and `Bash` is `bash`. A Pi child session cannot delegate, so `Agent` and `SendMessage` are unavailable there; on the main thread delegate with `subagent_run`, naming the agent. Your model comes from `.pi/subagents.json`, which overrides any alias in the source.

