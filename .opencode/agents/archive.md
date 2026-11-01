---
# agent-sync-sha256: 3417b645e0be52625ca2465c48de261c8ba2a0adec78c54e21d37d973e7e1a71
description: "Close a change after a PASS verdict: move the change folder to the archive; fast-lane changes update the docs/specs/README.md status table with the covered IDs; changes with delta specs merge them into openspec/specs."
mode: subagent
permissions:
  - {action: subagent, resource: "*", effect: deny}
---

Source revision: 3254d8098b335928497db2793d07892df58a95509a56f64ea57a7cab4a19f5f2

Before doing any work, read `.agents/agents/archive.md` and follow its entire Markdown body as your instructions. The source frontmatter is for Claude Code; this adapter supplies OpenCode mode and permissions.

OpenCode mapping: Claude Agent means the subagent tool; SendMessage means use the child session's result or follow-up facilities when available. Read/Glob/Grep, Edit/Write, and Bash mean the corresponding read/search, edit/patch, and shell tools. Use the selected or inherited model, not the Claude model alias in the source. Select coordinator in OpenCode rather than running the source's Claude startup command. Respect source write restrictions; shell access is not a filesystem sandbox.
