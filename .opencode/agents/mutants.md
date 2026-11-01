---
# agent-sync-sha256: 6886475bf63ea7adf69f9fed63a501e0fa442c319314b749f07e3e1b1f78505c
description: "Run cargo-mutants on a whole crate at change close (default), or on the lines or files a delivery changed when the Flow line says mutation: delivery, and report surviving mutants, classified critical or non-critical. Read-only on the repo; never edits code or specs."
mode: subagent
permissions: [{action: "*", resource: "*", effect: deny}, {action: external_directory, resource: "*", effect: ask}, {action: subagent, resource: "*", effect: deny}, {action: read, resource: "*", effect: allow}, {action: glob, resource: "*", effect: allow}, {action: grep, resource: "*", effect: allow}, {action: read, resource: "*.env", effect: deny}, {action: read, resource: "*.env.*", effect: deny}, {action: read, resource: "**/.env", effect: deny}, {action: read, resource: "**/.env.*", effect: deny}, {action: read, resource: "**/*.key", effect: deny}, {action: read, resource: "**/*.pem", effect: deny}, {action: read, resource: "**/.ssh/**", effect: deny}, {action: read, resource: "**/.aws/credentials", effect: deny}, {action: read, resource: "**/.config/gh/hosts.yml", effect: deny}, {action: read, resource: "**/.credentials/**", effect: deny}, {action: read, resource: "**/credentials.json", effect: deny}, {action: read, resource: "**/secrets/**", effect: deny}, {action: read, resource: "**/Library/Keychains/**", effect: deny}, {action: shell, resource: "*", effect: ask}, {action: edit, resource: "*", effect: deny}]
---

Source revision: 55badb7b3b22b72dce6c5a5680815195d47410e54199d1578d3faba95276467a

Before doing any work, read `.agents/agents/mutants.md` and follow its entire Markdown body as your instructions. The source frontmatter is for Claude Code; this adapter supplies OpenCode mode and permissions.

OpenCode mapping: Claude Agent means the subagent tool; SendMessage means use the child session's result or follow-up facilities when available. Read/Glob/Grep, Edit/Write, and Bash mean the corresponding read/search, edit/patch, and shell tools. Use the selected or inherited model, not the Claude model alias in the source. Select coordinator in OpenCode rather than running the source's Claude startup command. For Write-only sources, use write, not edit or patch. Respect source write restrictions; shell access is not a filesystem sandbox.
