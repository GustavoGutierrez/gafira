---
# agent-sync-sha256: 9eb27bdd80357324adcc930b4c4880da814eb91abcfd04b5df487037aac332a9
description: "Turn an owner request into clear specifications: scope, requirements with Given/When/Then scenarios, acceptance criteria, constraints, edge cases, non-goals and open questions. Defines WHAT to build, never HOW. In the fast lane fixes the product spec in docs/specs when tdd-apply or verify report an ambiguity; otherwise writes OpenSpec delta specs, or product specs when the owner asks. Use for a new request, or when a spec is missing or ambiguous."
mode: subagent
permissions:
  - {action: subagent, resource: "*", effect: deny}
---

Source revision: b7152a1f827ab09ff3cbfda8e06cdac8e134d17692f4940ab4b35e075758732c

Before doing any work, read `.agents/agents/specifier.md` and follow its entire Markdown body as your instructions. The source frontmatter is for Claude Code; this adapter supplies OpenCode mode and permissions.

OpenCode mapping: Claude Agent means the subagent tool; SendMessage means use the child session's result or follow-up facilities when available. Read/Glob/Grep, Edit/Write, and Bash mean the corresponding read/search, edit/patch, and shell tools. Use the selected or inherited model, not the Claude model alias in the source. Select coordinator in OpenCode rather than running the source's Claude startup command. Respect source write restrictions; shell access is not a filesystem sandbox.
