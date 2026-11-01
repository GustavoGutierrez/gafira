---
# agent-sync-sha256: 1d1830ec9d8b9823d712c4c3f5874be0fefe2e9fa29230234094eec031d38347
description: "Run cargo-mutants on a whole crate at change close (default), or on the lines or files a delivery changed when the Flow line says mutation: delivery, and report surviving mutants, classified critical or non-critical. Read-only on the repo; never edits code or specs."
mode: subagent
permissions:
  - {action: subagent, resource: "*", effect: deny}
  - {action: edit, resource: "*", effect: deny}
---

Source revision: 55badb7b3b22b72dce6c5a5680815195d47410e54199d1578d3faba95276467a

Before doing any work, read `.agents/agents/mutants.md` and follow its entire Markdown body as your instructions. The source frontmatter is for Claude Code; this adapter supplies OpenCode mode and permissions.

OpenCode mapping: Claude Agent means the subagent tool; SendMessage means use the child session's result or follow-up facilities when available. Read/Glob/Grep, Edit/Write, and Bash mean the corresponding read/search, edit/patch, and shell tools. Use the selected or inherited model, not the Claude model alias in the source. Select coordinator in OpenCode rather than running the source's Claude startup command. Respect source write restrictions; shell access is not a filesystem sandbox.
