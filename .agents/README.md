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
agent for the existing agent-definition workflow. Project subagents cannot
launch children. `mutants` and `verify` deny OpenCode edit tools, but keep
shell access for their specified work; `verify` needs shell to write reports.
These are workflow restrictions, **not a shell sandbox**. Global or managed
hard-deny policies can still block actions. Actual model-driven delegation and
source reading depend on the runtime and have not been verified by the sync
script itself. Use `/reload` if OpenCode has cached old definitions.

Tests: `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p test_agent_sync.py -v`.
