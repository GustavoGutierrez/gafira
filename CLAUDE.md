# Gafira: instructions for Claude Code

Read `AGENTS.md` first; it defines the development methodology, OpenSpec layout and agents.

## Executing a specification

When the user asks to execute, implement, continue, verify or archive a spec, a change or a delivery, the main thread acts as the **coordinator** defined in `.agents/agents/coordinator.md` (subagents cannot launch subagents, so the main thread does the coordinating):

- Do not implement inline. Delegate to the repo agents in `.agents/agents/`: `specifier`, `architect`, `explorer`, `tdd-apply`, `mutants`, `verify`, `archive`, `manual-writer` (also reachable as `.claude/agents/`).
- Follow the route and the enforcement rules in `coordinator.md`. One writer at a time. Stop for owner decisions and the `develop` -> `main` merge. Each change and delivery branches from `develop`; after verify PASS the coordinator merges it into `develop` locally, runs the gates, pushes `develop` and keeps one open PR `develop` -> `main` (ADR 0003).
- If the session was started with `--agent coordinator`, that is already the case.

## Overrides for this repo

These take precedence over the user-level `CLAUDE.md` orchestrator rules:

- Use the lite flow in `AGENTS.md` ((specify) -> spec -> (architect) -> tdd-apply -> verify -> archive). Do not run `gentle-ai`, `sdd-*` agents or skills, Engram saves for SDD artifacts, or its review/receipt commands for this project.
- No absolute or user paths in tracked files. Skills are referenced by name; the coordinator resolves them through `.atl/skill-registry.md` (gitignored) with the local `skill-registry` skill.
- State lives in `openspec/changes/<change>/` and git.
- Keep the user-level rules on language, short answers, one question at a time, and no AI attribution in commits.
