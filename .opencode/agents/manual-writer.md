---
# agent-sync-sha256: 97fe15902b5d774738d506e61e10e1c7de28d5c089aa84c8bd28828ed7ef6080
description: "Update, modify, extend or regenerate the technical manual (docs/manuals/tecnico/): find the chapter, read the code or spec as the source of truth, edit the English chapter, mirror the Spanish one, check, build and commit. Use whenever the owner asks to change the technical manual."
mode: subagent
permissions: [{action: "*", resource: "*", effect: deny}, {action: external_directory, resource: "*", effect: ask}, {action: subagent, resource: "*", effect: deny}, {action: read, resource: "*", effect: allow}, {action: glob, resource: "*", effect: allow}, {action: grep, resource: "*", effect: allow}, {action: read, resource: "*.env", effect: deny}, {action: read, resource: "*.env.*", effect: deny}, {action: read, resource: "**/.env", effect: deny}, {action: read, resource: "**/.env.*", effect: deny}, {action: read, resource: "**/*.key", effect: deny}, {action: read, resource: "**/*.pem", effect: deny}, {action: read, resource: "**/.ssh/**", effect: deny}, {action: read, resource: "**/.aws/credentials", effect: deny}, {action: read, resource: "**/.config/gh/hosts.yml", effect: deny}, {action: read, resource: "**/.credentials/**", effect: deny}, {action: read, resource: "**/credentials.json", effect: deny}, {action: read, resource: "**/secrets/**", effect: deny}, {action: read, resource: "**/Library/Keychains/**", effect: deny}, {action: shell, resource: "*", effect: ask}, {action: edit, resource: "*", effect: allow}]
---

Source revision: e59ef87d8c1234b1928ebc078dad588d95b03a26163ae08c95b0bf1fa0f8dca7

Before doing any work, read `.agents/agents/manual-writer.md` and follow its entire Markdown body as your instructions. The source frontmatter is for Claude Code; this adapter supplies OpenCode mode and permissions.

OpenCode mapping: Claude Agent means the subagent tool; SendMessage means use the child session's result or follow-up facilities when available. Read/Glob/Grep, Edit/Write, and Bash mean the corresponding read/search, edit/patch, and shell tools. Use the selected or inherited model, not the Claude model alias in the source. Select coordinator in OpenCode rather than running the source's Claude startup command. For Write-only sources, use write, not edit or patch. Respect source write restrictions; shell access is not a filesystem sandbox.
