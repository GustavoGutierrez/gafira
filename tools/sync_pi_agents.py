#!/usr/bin/env python3
"""Synchronize Gafira's Claude agent sources into Pi agent definitions.

`.agents/agents/*.md` is the source of truth for every runtime. This script
derives the Pi-specific artifacts from it:

    .agents/agents/<name>.md   source (Claude Code format)
    .pi/agents/<name>.md       generated Pi subagent definition
    .pi/prompts/coordinator.md generated Pi prompt template (main thread)

Pi reads `.pi/agents/*.md` as project-scoped subagents and `.pi/subagents.json`
for the per-agent model. Both live in this project: nothing is written to the
user agent directory and no global configuration is read or changed.

The coordinator is a prompt template, not a subagent. A Pi child session cannot
delegate, so the coordinator has to run on the main thread; `/coordinator`
expands its instructions there.

Usage:
    python3 tools/sync_pi_agents.py            # write generated definitions
    python3 tools/sync_pi_agents.py --check    # fail on drift, write nothing
"""

import argparse
import hashlib
import json
import os
import re
import sys
import tempfile
from pathlib import Path

MARKER = "# pi-agent-sync-sha256: "
SOURCE_DIR = Path(".agents/agents")
AGENT_DIR = Path(".pi/agents")
PROMPT_DIR = Path(".pi/prompts")
COORDINATOR = "coordinator"

# Claude Code tool names to Pi tool names. Pi's `find` takes a glob pattern,
# which is what Claude's `Glob` does.
CLAUDE_TOOLS = {
    "Read": "read",
    "Edit": "edit",
    "Write": "write",
    "Glob": "find",
    "Grep": "grep",
    "Bash": "bash",
}
CLAUDE_ONLY_TOOLS = {"Agent", "SendMessage"}
ALLOWED_CLAUDE_TOOLS = set(CLAUDE_TOOLS) | CLAUDE_ONLY_TOOLS
SOURCE_MODELS = {"opus", "sonnet", "haiku"}

SOURCE_POINTER = (
    "Before doing any work, read `.agents/agents/{name}.md` and follow its entire "
    "Markdown body as your instructions. The source frontmatter is for Claude "
    "Code; this definition supplies Pi's tool names and the model from "
    "`.pi/subagents.json`. The source is the single source of truth: read it "
    "again if it changed since this definition was generated.\n\n"
)

MAPPING_NOTE = (
    "Pi mapping: the source frontmatter targets Claude Code. In Pi, `Read` is "
    "`read`, `Edit` is `edit`, `Write` is `write`, `Glob` is `find`, `Grep` is "
    "`grep` and `Bash` is `bash`. A Pi child session cannot delegate, so "
    "`Agent` and `SendMessage` are unavailable there; on the main thread "
    "delegate with `subagent_run`, naming the agent. Your model comes from "
    "`.pi/subagents.json`, which overrides any alias in the source.\n"
)


def digest(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def parse_frontmatter_block(text):
    """Return the raw frontmatter block of a source agent file."""
    match = re.fullmatch(r"---\n(.*?)\n---\n(.*)", text, re.DOTALL)
    if not match:
        raise ValueError("unsupported frontmatter")
    return match.group(1)


def parse_source(path):
    """Return (name, description, tools, revision) for one source agent file.

    The revision covers the frontmatter only. The body is not copied into the
    generated definition, so editing the body changes no generated file.
    """
    text = path.read_text(encoding="utf-8")
    match = re.fullmatch(r"---\n(.*?)\n---\n(.*)", text, re.DOTALL)
    if not match:
        raise ValueError(f"unsupported frontmatter: {path}")
    lines = match.group(1).splitlines()
    fields = {}
    index = 0
    while index < len(lines):
        line = lines[index]
        field = re.fullmatch(r"([a-z]+): (.*)", line)
        if not field or field.group(1) in fields:
            raise ValueError(f"unsupported frontmatter: {path}: {line}")
        key, value = field.group(1), field.group(2)
        index += 1
        if value == ">":
            parts = []
            while index < len(lines) and lines[index].startswith("  "):
                parts.append(lines[index].strip())
                index += 1
            value = " ".join(parts)
        fields[key] = value
    if set(fields) != {"name", "description", "model", "tools"}:
        raise ValueError(f"unsupported fields: {path}")
    # Pi's agent frontmatter parser is line based: it strips the surrounding
    # quotes and does not unescape. A quote or a backslash inside the
    # description would therefore survive as a literal escape, so refuse it
    # instead of writing a definition that reads differently than it looks.
    if '"' in fields["description"] or "\\" in fields["description"]:
        raise ValueError(f"unsupported description characters: {path}")
    name = fields["name"]
    tools = [tool.strip() for tool in fields["tools"].split(",")]
    if (not re.fullmatch(r"[a-z][a-z0-9-]*", name) or name != path.stem
            or not fields["description"] or not match.group(2).strip()
            or fields["model"] not in SOURCE_MODELS or len(tools) != len(set(tools))
            or not set(tools) <= ALLOWED_CLAUDE_TOOLS or not {"Read", "Bash"} <= set(tools)
            or (name != COORDINATOR and CLAUDE_ONLY_TOOLS & set(tools))):
        raise ValueError(f"unsupported agent definition: {path}")
    return name, fields["description"], tools, digest(match.group(1))


def pi_tools(claude_tools):
    """Map Claude tool names to Pi tool names, preserving source order."""
    mapped = []
    for tool in claude_tools:
        if tool in CLAUDE_ONLY_TOOLS:
            continue
        pi_tool = CLAUDE_TOOLS[tool]
        if pi_tool not in mapped:
            mapped.append(pi_tool)
    return mapped


def render_agent(name, description, tools, revision):
    """Render the Pi subagent definition for one agent.

    The definition does not copy the source body. It points at the source file
    and the agent reads it at run time, so a source edit takes effect without
    regenerating anything. Only the frontmatter is derived, and it is what the
    revision hash covers.
    """
    front = f"name: {name}\n"
    front += f"description: {json.dumps(description, ensure_ascii=False)}\n"
    front += f"tools: {', '.join(tools)}\n"
    content = front + "---\n\n"
    content += f"# Pi agent: {name}\n\n"
    content += f"Source frontmatter revision: {revision}\n\n"
    content += SOURCE_POINTER.format(name=name)
    content += MAPPING_NOTE + "\n"
    return "---\n" + MARKER + digest(content) + "\n" + content


def render_prompt(name, description, revision):
    """Render the Pi prompt template that puts the agent on the main thread."""
    front = f"description: {json.dumps(description, ensure_ascii=False)}\n"
    content = front + "---\n\n"
    content += f"# Pi prompt: {name}\n\n"
    content += f"Source frontmatter revision: {revision}\n\n"
    content += SOURCE_POINTER.format(name=name)
    content += MAPPING_NOTE + "\n"
    return "---\n" + MARKER + digest(content) + "\n" + content


def managed(text):
    """Report whether a destination file still carries this script's marker."""
    prefix = "---\n" + MARKER
    if not text.startswith(prefix):
        return False
    line, sep, body = text[len(prefix):].partition("\n")
    return bool(sep and re.fullmatch(r"[a-f0-9]{64}", line) and line == digest(body))


def write_atomic(path, content):
    path.parent.mkdir(parents=True, exist_ok=True)
    handle, temporary = tempfile.mkstemp(dir=str(path.parent), prefix=".pi-agent-sync-")
    try:
        with os.fdopen(handle, "w", encoding="utf-8") as stream:
            stream.write(content)
        # mkstemp creates 0600; generated definitions are ordinary tracked files.
        os.chmod(temporary, 0o644)
        os.replace(temporary, path)
    except BaseException:
        if os.path.exists(temporary):
            os.unlink(temporary)
        raise


def collect(root):
    """Return the generated artifacts this script owns, keyed by path."""
    artifacts = {}
    for source in sorted((root / SOURCE_DIR).glob("*.md")):
        if source.is_symlink() or not source.is_file():
            raise ValueError(f"unsafe source: {source}")
        name, description, tools, revision = parse_source(source)
        if name == COORDINATOR:
            artifacts[root / PROMPT_DIR / f"{name}.md"] = render_prompt(
                name, description, revision)
        else:
            artifacts[root / AGENT_DIR / f"{name}.md"] = render_agent(
                name, description, pi_tools(tools), revision)
    return artifacts


def synchronize(root, check):
    """Write or check the generated artifacts.

    A destination that this script did not generate, or that was edited by
    hand, is a conflict: it is reported and never overwritten. A destination
    that still carries the marker but no longer matches the source is drift,
    and it is regenerated.
    """
    artifacts = collect(root)
    conflicts = []
    changes = []
    for path, content in sorted(artifacts.items()):
        if not path.exists():
            changes.append(path)
            continue
        if path.is_symlink() or not path.is_file():
            conflicts.append(f"unsafe destination: {path}")
            continue
        current = path.read_text(encoding="utf-8")
        if not managed(current):
            conflicts.append(f"unmanaged or edited destination: {path}")
        elif current != content:
            changes.append(path)
    for directory in (root / AGENT_DIR, root / PROMPT_DIR):
        if not directory.is_dir():
            continue
        for path in sorted(directory.glob("*.md")):
            if path in artifacts:
                continue
            if managed(path.read_text(encoding="utf-8", errors="replace")):
                conflicts.append(f"stale destination without source: {path}")
    if conflicts:
        raise ValueError("\n".join(conflicts))
    for path in changes:
        state = "stale" if path.exists() else "missing"
        print(f"{state} destination: {path}")
    if check:
        return 1 if changes else 0
    for path in changes:
        write_atomic(path, artifacts[path])
        print(f"wrote {path}")
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".", help="project root (default: .)")
    parser.add_argument("--check", action="store_true",
                        help="report drift without writing")
    parser.add_argument("--revision", metavar="AGENT",
                        help="print the frontmatter revision of a source agent")
    args = parser.parse_args()
    root = Path(args.root).resolve()
    if not (root / SOURCE_DIR).is_dir():
        print(f"error: missing source directory: {root / SOURCE_DIR}", file=sys.stderr)
        return 2
    if args.revision:
        source = root / SOURCE_DIR / f"{args.revision}.md"
        if not source.is_file():
            print(f"error: no source agent: {source}", file=sys.stderr)
            return 2
        print(digest(parse_frontmatter_block(source.read_text(encoding="utf-8"))))
        return 0
    try:
        status = synchronize(root, args.check)
    except (OSError, UnicodeError, ValueError) as error:
        print(error, file=sys.stderr)
        return 2
    if status == 0 and args.check:
        print("generated Pi agent definitions are up to date")
    return status


if __name__ == "__main__":
    sys.exit(main())
