---
# agent-sync-sha256: 45ccd82ea94b07e85478afa5f9d26a525bf27921cbeb65ba35e05a72cc28c8b1
description: "Update, modify, extend or regenerate the technical manual (docs/manuals/tecnico/): find the chapter, read the code or spec as the source of truth, edit the English chapter, mirror the Spanish one, check, build and commit. Use whenever the owner asks to change the technical manual."
mode: subagent
permissions:
  - {action: subagent, resource: "*", effect: deny}
---

Source revision: 8b12e666f897155454c1be61425d91ea799be6c57f29d031479e2cb11e8f5166

Before doing any work, read `.agents/agents/manual-writer.md` and follow its entire Markdown body as your instructions. The source frontmatter is for Claude Code; this adapter supplies OpenCode mode and permissions.

OpenCode mapping: Claude Agent means the subagent tool; SendMessage means use the child session's result or follow-up facilities when available. Read/Glob/Grep, Edit/Write, and Bash mean the corresponding read/search, edit/patch, and shell tools. Use the selected or inherited model, not the Claude model alias in the source. Select coordinator in OpenCode rather than running the source's Claude startup command. Respect source write restrictions; shell access is not a filesystem sandbox.
