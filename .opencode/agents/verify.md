---
# agent-sync-sha256: 4d610cb25dfdbf39b17c75ee4a095bfa808cea1608fd02779287f5c29f984488
description: "Verify a delivery or whole change against its specs: run gates, check each requirement and scenario against code and tests, return PASS, FAIL or BLOCKED."
mode: subagent
permissions: [{action: "*", resource: "*", effect: deny}, {action: external_directory, resource: "*", effect: ask}, {action: subagent, resource: "*", effect: deny}, {action: read, resource: "*", effect: allow}, {action: glob, resource: "*", effect: allow}, {action: grep, resource: "*", effect: allow}, {action: read, resource: "*.env", effect: deny}, {action: read, resource: "*.env.*", effect: deny}, {action: read, resource: "**/.env", effect: deny}, {action: read, resource: "**/.env.*", effect: deny}, {action: read, resource: "**/*.key", effect: deny}, {action: read, resource: "**/*.pem", effect: deny}, {action: read, resource: "**/.ssh/**", effect: deny}, {action: read, resource: "**/.aws/credentials", effect: deny}, {action: read, resource: "**/.config/gh/hosts.yml", effect: deny}, {action: read, resource: "**/.credentials/**", effect: deny}, {action: read, resource: "**/credentials.json", effect: deny}, {action: read, resource: "**/secrets/**", effect: deny}, {action: read, resource: "**/Library/Keychains/**", effect: deny}, {action: shell, resource: "*", effect: ask}, {action: edit, resource: "*", effect: deny}]
---

Source revision: 7b425324879621d943e6a0fbf7639e92480623b5e5f471f8272951fb10d99616

Before doing any work, read `.agents/agents/verify.md` and follow its entire Markdown body as your instructions. The source frontmatter is for Claude Code; this adapter supplies OpenCode mode and permissions.

OpenCode mapping: Claude Agent means the subagent tool; SendMessage means use the child session's result or follow-up facilities when available. Read/Glob/Grep, Edit/Write, and Bash mean the corresponding read/search, edit/patch, and shell tools. Use the selected or inherited model, not the Claude model alias in the source. Select coordinator in OpenCode rather than running the source's Claude startup command. For Write-only sources, use write, not edit or patch. Respect source write restrictions; shell access is not a filesystem sandbox.
