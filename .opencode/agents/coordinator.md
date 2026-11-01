---
# agent-sync-sha256: 63f227d786541d48bf166af06f2948335a8c133086ba60ca47c6a1e9e715172d
description: "Coordinates the repo's lite SDD flow: decides the next step, delegates to specifier, architect, explorer, tdd-apply, mutants, verify and archive, and enforces AGENTS.md. Run it as the main-thread agent; it does not write code, tests or specs (exceptions: the Light spec block in light mode, the Slice card in the fast lane and `.odd/state.json`). Use whenever the owner asks to execute, implement, continue, verify or archive a spec, change or delivery."
mode: primary
permissions:
  - {action: subagent, resource: "*", effect: deny}
  - {action: subagent, resource: "architect", effect: allow}
  - {action: subagent, resource: "archive", effect: allow}
  - {action: subagent, resource: "explorer", effect: allow}
  - {action: subagent, resource: "general", effect: allow}
  - {action: subagent, resource: "manual-writer", effect: allow}
  - {action: subagent, resource: "mutants", effect: allow}
  - {action: subagent, resource: "specifier", effect: allow}
  - {action: subagent, resource: "tdd-apply", effect: allow}
  - {action: subagent, resource: "verify", effect: allow}
---

Source revision: c47903eccaa0468c715db94d239212662fc7cd916d2d048578348d15496cf85b

Before doing any work, read `.agents/agents/coordinator.md` and follow its entire Markdown body as your instructions. The source frontmatter is for Claude Code; this adapter supplies OpenCode mode and permissions.

OpenCode mapping: Claude Agent means the subagent tool; SendMessage means use the child session's result or follow-up facilities when available. Read/Glob/Grep, Edit/Write, and Bash mean the corresponding read/search, edit/patch, and shell tools. Use the selected or inherited model, not the Claude model alias in the source. Select coordinator in OpenCode rather than running the source's Claude startup command. Respect source write restrictions; shell access is not a filesystem sandbox.
