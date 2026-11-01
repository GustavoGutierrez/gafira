---
# agent-sync-sha256: 0efe551ea21424cfbf5bd0582b3f3e1e4afea0e5bc5550ed3056d8f30ecd46aa
description: "Implement one delivery of a change straight from its specs with strict TDD (RED, GREEN, REFACTOR). Use when specs exist and code must be written."
mode: subagent
permissions: [{action: "*", resource: "*", effect: deny}, {action: external_directory, resource: "*", effect: ask}, {action: subagent, resource: "*", effect: deny}, {action: read, resource: "*", effect: allow}, {action: glob, resource: "*", effect: allow}, {action: grep, resource: "*", effect: allow}, {action: read, resource: "*.env", effect: deny}, {action: read, resource: "*.env.*", effect: deny}, {action: read, resource: "**/.env", effect: deny}, {action: read, resource: "**/.env.*", effect: deny}, {action: read, resource: "**/*.key", effect: deny}, {action: read, resource: "**/*.pem", effect: deny}, {action: read, resource: "**/.ssh/**", effect: deny}, {action: read, resource: "**/.aws/credentials", effect: deny}, {action: read, resource: "**/.config/gh/hosts.yml", effect: deny}, {action: read, resource: "**/.credentials/**", effect: deny}, {action: read, resource: "**/credentials.json", effect: deny}, {action: read, resource: "**/secrets/**", effect: deny}, {action: read, resource: "**/Library/Keychains/**", effect: deny}, {action: shell, resource: "*", effect: ask}, {action: edit, resource: "*", effect: allow}]
---

Source revision: 461d5960e13c5212b1bae00c8bfe8df76eb0637d797d079cbceabc637a7de3bd

Before doing any work, read `.agents/agents/tdd-apply.md` and follow its entire Markdown body as your instructions. The source frontmatter is for Claude Code; this adapter supplies OpenCode mode and permissions.

OpenCode mapping: Claude Agent means the subagent tool; SendMessage means use the child session's result or follow-up facilities when available. Read/Glob/Grep, Edit/Write, and Bash mean the corresponding read/search, edit/patch, and shell tools. Use the selected or inherited model, not the Claude model alias in the source. Select coordinator in OpenCode rather than running the source's Claude startup command. For Write-only sources, use write, not edit or patch. Respect source write restrictions; shell access is not a filesystem sandbox.
