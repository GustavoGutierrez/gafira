---
# agent-sync-sha256: 7c4a3fda60bd18e1b070400892da76d14345a6183971bfa8647bdeda9234d690
description: "Close a change after a PASS verdict: move the change folder to the archive; fast-lane changes update the specs/README.md status table with the covered IDs; changes with delta specs merge them into specs/capabilities."
mode: subagent
permissions: [{action: "*", resource: "*", effect: deny}, {action: external_directory, resource: "*", effect: ask}, {action: subagent, resource: "*", effect: deny}, {action: read, resource: "*", effect: allow}, {action: glob, resource: "*", effect: allow}, {action: grep, resource: "*", effect: allow}, {action: read, resource: "*.env", effect: deny}, {action: read, resource: "*.env.*", effect: deny}, {action: read, resource: "**/.env", effect: deny}, {action: read, resource: "**/.env.*", effect: deny}, {action: read, resource: "**/*.key", effect: deny}, {action: read, resource: "**/*.pem", effect: deny}, {action: read, resource: "**/.ssh/**", effect: deny}, {action: read, resource: "**/.aws/credentials", effect: deny}, {action: read, resource: "**/.config/gh/hosts.yml", effect: deny}, {action: read, resource: "**/.credentials/**", effect: deny}, {action: read, resource: "**/credentials.json", effect: deny}, {action: read, resource: "**/secrets/**", effect: deny}, {action: read, resource: "**/Library/Keychains/**", effect: deny}, {action: shell, resource: "*", effect: ask}, {action: edit, resource: "*", effect: allow}]
---

Source revision: 1ba09ea3b568def5754b0ee70c8284e181b61f317b154d863eff21dacf604635

Before doing any work, read `.agents/agents/archive.md` and follow its entire Markdown body as your instructions. The source frontmatter is for Claude Code; this adapter supplies OpenCode mode and permissions.

OpenCode mapping: Claude Agent means the subagent tool; SendMessage means use the child session's result or follow-up facilities when available. Read/Glob/Grep, Edit/Write, and Bash mean the corresponding read/search, edit/patch, and shell tools. Use the selected or inherited model, not the Claude model alias in the source. Select coordinator in OpenCode rather than running the source's Claude startup command. For Write-only sources, use write, not edit or patch. Respect source write restrictions; shell access is not a filesystem sandbox.
