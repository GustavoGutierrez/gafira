---
# agent-sync-sha256: d7d705e4c26d71537df747c6203fda93760e0a90d959bcff391fb58e3505b3f0
description: "Measure risky claims before coding or spec fixes: run quick experiments in a scratch project under /tmp and report facts (command, output, conclusion). Also maps unfamiliar code or specs when a delivery needs it."
mode: subagent
permissions:
  - {action: subagent, resource: "*", effect: deny}
---

Source revision: 2a607cbfa4e032a27dadc1ea88f4e1a3b6419ad54a8a9a5ab7269c0e9a847848

Before doing any work, read `.agents/agents/explorer.md` and follow its entire Markdown body as your instructions. The source frontmatter is for Claude Code; this adapter supplies OpenCode mode and permissions.

OpenCode mapping: Claude Agent means the subagent tool; SendMessage means use the child session's result or follow-up facilities when available. Read/Glob/Grep, Edit/Write, and Bash mean the corresponding read/search, edit/patch, and shell tools. Use the selected or inherited model, not the Claude model alias in the source. Select coordinator in OpenCode rather than running the source's Claude startup command. Respect source write restrictions; shell access is not a filesystem sandbox.
