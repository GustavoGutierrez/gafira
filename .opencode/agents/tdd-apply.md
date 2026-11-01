---
# agent-sync-sha256: 74071d0b79c34cd523b51a6e2b4ef615c7e772c2711a326c39ba1cb846deafea
description: "Implement one delivery of a change straight from its specs with strict TDD (RED, GREEN, REFACTOR). Use when specs exist and code must be written."
mode: subagent
permissions:
  - {action: subagent, resource: "*", effect: deny}
---

Source revision: 461d5960e13c5212b1bae00c8bfe8df76eb0637d797d079cbceabc637a7de3bd

Before doing any work, read `.agents/agents/tdd-apply.md` and follow its entire Markdown body as your instructions. The source frontmatter is for Claude Code; this adapter supplies OpenCode mode and permissions.

OpenCode mapping: Claude Agent means the subagent tool; SendMessage means use the child session's result or follow-up facilities when available. Read/Glob/Grep, Edit/Write, and Bash mean the corresponding read/search, edit/patch, and shell tools. Use the selected or inherited model, not the Claude model alias in the source. Select coordinator in OpenCode rather than running the source's Claude startup command. Respect source write restrictions; shell access is not a filesystem sandbox.
