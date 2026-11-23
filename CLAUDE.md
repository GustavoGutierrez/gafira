# Gafira: instructions for Claude Code

Read `AGENTS.md` first; it defines the methodology, the rules and the gates.

## Implementing a spec

The main agent implements specs directly, following `AGENTS.md` and the `gafira-specs` skill: read the spec and the code, write the failing tests first for critical logic, implement, run the gates of "Before calling something done", then update the spec's status banner and `specs/README.md`. There is no coordinator and no delegation chain. The `manual-writer` agent is used only when the owner asks to change the technical manual.

## Overrides for this repo

These take precedence over the user-level `CLAUDE.md` orchestrator rules:

- Use the simple spec flow in `AGENTS.md`. Do not run `gentle-ai`, `sdd-*` agents or skills, Engram saves for SDD artifacts, or its review/receipt commands for this project.
- No absolute or user paths in tracked files. Skills are referenced by name and resolved through `.atl/skill-registry.md` (gitignored) with the local `skill-registry` skill.
- State lives in the spec's status banner, in `specs/README.md` and in git.
- Keep the user-level rules on language, short answers, one question at a time, and no AI attribution in commits.
