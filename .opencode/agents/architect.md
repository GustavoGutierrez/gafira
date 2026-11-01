---
# agent-sync-sha256: 973658cf634e66f89e44dc663b7a3e1451a23cbf33b2d36d4fa620d76087a24e
description: "Analyse the existing code and specs, decide HOW to build, record technical decisions as ADRs, and break the work into ordered deliveries (~700 lines) with dependencies, parallel or serial. Writes design.md (and proposal.md or tasks only when needed) and ADRs; never production code or tests. Use when a spec leaves a real design decision open, a change needs parallel deliveries or a design-dependent split, or code must be analysed before choosing an approach."
mode: subagent
permissions:
  - {action: subagent, resource: "*", effect: deny}
---

Source revision: be8a80b1498a43671f517027319b5327b30b51b71267b97ba3dc82ec9fa791a9

Before doing any work, read `.agents/agents/architect.md` and follow its entire Markdown body as your instructions. The source frontmatter is for Claude Code; this adapter supplies OpenCode mode and permissions.

OpenCode mapping: Claude Agent means the subagent tool; SendMessage means use the child session's result or follow-up facilities when available. Read/Glob/Grep, Edit/Write, and Bash mean the corresponding read/search, edit/patch, and shell tools. Use the selected or inherited model, not the Claude model alias in the source. Select coordinator in OpenCode rather than running the source's Claude startup command. Respect source write restrictions; shell access is not a filesystem sandbox.
