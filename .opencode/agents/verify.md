---
# agent-sync-sha256: a89f79ca6219a2a3550e5f7da999c8e27566074d61a809cb5584932ec08c01b6
description: "Verify a delivery or whole change against its specs: run gates, check each requirement and scenario against code and tests, return PASS, FAIL or BLOCKED."
mode: subagent
permissions:
  - {action: subagent, resource: "*", effect: deny}
  - {action: edit, resource: "*", effect: deny}
---

Source revision: 2a996d48d85d5f4dc95e1dcd9d59b29acb750fb9d184bb61b48deca570a85929

Before doing any work, read `.agents/agents/verify.md` and follow its entire Markdown body as your instructions. The source frontmatter is for Claude Code; this adapter supplies OpenCode mode and permissions.

OpenCode mapping: Claude Agent means the subagent tool; SendMessage means use the child session's result or follow-up facilities when available. Read/Glob/Grep, Edit/Write, and Bash mean the corresponding read/search, edit/patch, and shell tools. Use the selected or inherited model, not the Claude model alias in the source. Select coordinator in OpenCode rather than running the source's Claude startup command. Respect source write restrictions; shell access is not a filesystem sandbox.
