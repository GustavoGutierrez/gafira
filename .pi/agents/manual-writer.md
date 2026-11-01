---
# pi-agent-sync-sha256: 1d2d35a41b0737574b24a0924f7082647651f204032e498948c1943cd1685248
name: manual-writer
description: "Update, modify, extend or regenerate the technical manual (docs/manuals/tecnico/): find the chapter, read the code or spec as the source of truth, edit the English chapter, mirror the Spanish one, check, build and commit. Use whenever the owner asks to change the technical manual."
tools: read, edit, write, find, grep, bash
---

# Pi agent: manual-writer

Source frontmatter revision: f974f6d6efabf74b234c6ee1bbe2c025ddf9683139ef75c81ac267880a609337

Before doing any work, read `.agents/agents/manual-writer.md` and follow its entire Markdown body as your instructions. The source frontmatter is for Claude Code; this definition supplies Pi's tool names and the model from `.pi/subagents.json`. The source is the single source of truth: read it again if it changed since this definition was generated.

Pi mapping: the source frontmatter targets Claude Code. In Pi, `Read` is `read`, `Edit` is `edit`, `Write` is `write`, `Glob` is `find`, `Grep` is `grep` and `Bash` is `bash`. A Pi child session cannot delegate, so `Agent` and `SendMessage` are unavailable there; on the main thread delegate with `subagent_run`, naming the agent. Your model comes from `.pi/subagents.json`, which overrides any alias in the source.

