## Light spec

Flow: light | level: L1 | pack: agent/skill definitions | mutation: off | tier: T0 | verify: V1 | decided-by: coordinator | reason: Bounded developer tooling; no product or critical-code change.
Flow-change: light -> standard | level: L2 | pack: agent/skill definitions | mutation: archive | tier: T0 | verify: V1 | decided-by: coordinator | reason: Independent review found permission and partial-write safety gaps; the delivery exceeds the ~300-line standard-mode signal. No Rust crate is touched, so no T4 run applies.

Scope: Make `.agents/agents/` the source of truth for Claude Code and OpenCode agent definitions, with a repeatable synchronization tool and usage documentation.
Non-goals: Migrate arbitrary external runtimes, change product behavior, change global OpenCode configuration, or prescribe user model choices.

Acceptance criteria:
- AC1 Given the source agents, when synchronization runs, then Claude Code retains its existing source link and OpenCode discovers `coordinator` as primary and the other project agents as subagents. [command]
- AC2 Given the coordinator's project workflow, when it delegates in OpenCode, then the named project subagents are permitted and receive their source instructions; inherited models do not assume Claude-only aliases. [command]
- AC3 Given changed source instructions, when synchronization runs again, then generated OpenCode definitions update deterministically; a check mode detects drift without writing. [command]
- AC4 Given unmanaged destination files or unsupported source definitions, when synchronization runs, then it reports the conflict without overwriting user data. [command]
- AC5 Given the project setup, when a developer reads the instructions, then they can synchronize, select the coordinator, and understand supported runtimes and limitations. [manual]
- AC6 Given project-local model settings, when OpenCode loads the project, then each subagent can use its configured provider/model without changing global configuration; the coordinator's session-model selection behavior is documented. [command]

Plan: Write failing tests, implement synchronization and runtime adaptation, generate definitions, document usage, and independently verify acceptance criteria.
Risk and reversibility: Local tooling only; remove generated definitions and the script to revert. Owner decisions touched: none.
Escalation triggers watched: source format ambiguity, unavailable runtime permissions, destructive synchronization, or excessive delivery size.
