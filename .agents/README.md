# Project agents in Claude Code and OpenCode

`.agents/agents/` is the source of truth. Claude Code uses the existing
`.claude/agents` symlink directly. OpenCode V2 uses generated adapters in
`.opencode/agents/`. After changing a source, run from the project root:

```sh
python3 tools/sync_agents.py
python3 tools/sync_agents.py --check
```

Commit the changed adapters with the source. `--check` exits nonzero for drift
without writing. The sync also rejects unsupported source formats, symlinks and
unmanaged or manually edited destination collisions before applying changes.
It reports generated destinations with no source and never deletes them: inspect
and remove obsolete adapters yourself, then rerun. Unrelated user agents are
left alone. Do not edit generated files; edit their sources instead. The marker
detects accidental edits, not malicious tampering.

Synchronization stages and flushes complete new files and keeps hard-linked
backups before replacing any target. On a write or replace error it rolls back
installed files; if rollback itself fails it reports the backup directory for
manual recovery. Filesystem support for hard links is required. Do not run
concurrent syncs or edit destinations during a sync. A process crash, power loss,
or rollback failure can leave a mixed generation and staging files; inspect the
reported/stale `.opencode/.agent-sync-*` directory and recover manually. This is
not a crash-atomic multi-file transaction.

## Use the coordinator

In OpenCode's agent selector, choose `coordinator` as the primary agent, or run:

```sh
opencode run --agent coordinator "Continue the current Gafira delivery"
```

OpenCode keeps the selected session model; subagents inherit it unless a user
configures an override. The adapters do not use Claude model aliases. Each
adapter instructs its agent to read the corresponding source body before work;
this relies on the model following that instruction and on source read access.
Claude-specific Agent, SendMessage and tool names are mapped to OpenCode's
subagent, child-result and corresponding tools in the adapter.

The coordinator can call the named project subagents and OpenCode's `general`
agent for the existing agent-definition workflow. Adapters deny unspecified
actions, including MCP, `execute`, `skill`, web and child delegation; only the
coordinator may ask questions or delegate. Agents read skill files directly as
their sources specify. Shell and external-directory access require approval;
`verify` uses shell to write its reports. `mutants` and `verify` deny edit tools.

OpenCode combines Write, Edit and patch under `edit`: explorer's edit permission
is limited to `openspec/changes/*/spike.md`, but cannot enforce Write-only
within that path. Its experiments use shell for scratch files. Shell is **not a
sandbox**, so path rules do not prevent shell writes. Agent permission rules
are evaluated after global permission rules: the explicit read/other allowances
can override global denies; use hard-deny policies for non-overridable limits.
The adapters deny common environment/key reads, but cannot reproduce arbitrary
user-specific global read rules. If global instructions require MCP or skills,
these adapters deny those tools and the workflow must be adjusted explicitly.
Actual model-driven delegation and source reading depend on the runtime; the
sync script does not verify them. Use `/reload` if definitions are cached.

Tests: `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p test_agent_sync.py -v`.
