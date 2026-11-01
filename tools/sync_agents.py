#!/usr/bin/env python3
"""Synchronize Gafira's Claude agent sources to OpenCode V2 adapters."""

import argparse
import hashlib
import json
import os
import re
import sys
import tempfile
from pathlib import Path


MARKER = "# agent-sync-sha256: "
ALLOWED_TOOLS = {"Agent", "SendMessage", "Read", "Edit", "Write", "Glob", "Grep", "Bash"}
MODELS = {"opus", "sonnet", "haiku", "inherit"}


def digest(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def parse_source(path):
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
        if not field or field[1] in fields:
            raise ValueError(f"unsupported frontmatter: {path}: {line}")
        key, value = field.groups()
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
    name = fields["name"]
    tools = [tool.strip() for tool in fields["tools"].split(",")]
    if (not re.fullmatch(r"[a-z][a-z0-9-]*", name) or name != path.stem
            or not fields["description"] or not match.group(2).strip()
            or fields["model"] not in MODELS or len(tools) != len(set(tools))
            or not set(tools) <= ALLOWED_TOOLS or not {"Read", "Bash"} <= set(tools)
            or (name != "coordinator" and {"Agent", "SendMessage"} & set(tools))):
        raise ValueError(f"unsupported agent definition: {path}")
    return fields, tools, digest(text)


def rule(action, resource, effect):
    if action == "*":
        action = '"*"'
    return f'  - {{action: {action}, resource: {json.dumps(resource)}, effect: {effect}}}\n'


def render(fields, tools, source_hash, names):
    name = fields["name"]
    body = f'description: {json.dumps(fields["description"], ensure_ascii=False)}\n'
    body += f'mode: {"primary" if name == "coordinator" else "subagent"}\n'
    rules = [rule("*", "*", "deny"), rule("external_directory", "*", "ask"),
             rule("subagent", "*", "deny")]
    for tool, action in (("Read", "read"), ("Glob", "glob"), ("Grep", "grep")):
        if tool in tools:
            rules.append(rule(action, "*", "allow"))
    if "Read" in tools:
        for pattern in ("*.env", "*.env.*", "**/.env", "**/.env.*", "**/*.key", "**/*.pem",
                        "**/.ssh/**", "**/.aws/credentials", "**/.config/gh/hosts.yml",
                        "**/.credentials/**", "**/credentials.json", "**/secrets/**", "**/Library/Keychains/**"):
            rules.append(rule("read", pattern, "deny"))
    if "Bash" in tools:
        rules.append(rule("shell", "*", "ask"))
    if {"Edit", "Write"} & set(tools):
        target = "openspec/changes/*/spike.md" if name == "explorer" else "*"
        rules.append(rule("edit", target, "allow"))
    else:
        rules.append(rule("edit", "*", "deny"))
    if name == "coordinator":
        rules.append(rule("question", "*", "allow"))
        for child in sorted(set(names) | {"general"}):
            if child != name:
                rules.append(rule("subagent", child, "allow"))
    body += "permissions: [" + ", ".join(r.strip()[2:] for r in rules) + "]\n"
    body += "---\n\n"
    body += f"Source revision: {source_hash}\n\n"
    body += (f"Before doing any work, read `.agents/agents/{name}.md` and follow its entire "
             "Markdown body as your instructions. The source frontmatter is for Claude Code; "
             "this adapter supplies OpenCode mode and permissions.\n\n")
    body += ("OpenCode mapping: Claude Agent means the subagent tool; SendMessage means "
             "use the child session's result or follow-up facilities when available. "
             "Read/Glob/Grep, Edit/Write, and Bash mean the corresponding read/search, "
             "edit/patch, and shell tools. Use the selected or inherited model, not the "
             "Claude model alias in the source. Select coordinator in OpenCode rather "
             "than running the source's Claude startup command. For Write-only sources, use write, "
             "not edit or patch. Respect source write restrictions; shell "
             "access is not a filesystem sandbox.\n")
    return "---\n" + MARKER + digest(body) + "\n" + body


def managed(text):
    prefix = "---\n" + MARKER
    if not text.startswith(prefix):
        return False
    line, sep, body = text[len(prefix):].partition("\n")
    return bool(sep and re.fullmatch(r"[a-f0-9]{64}", line) and line == digest(body))


def install(dest, expected, changes, existing):
    dest.mkdir(parents=True, exist_ok=True)
    stage = Path(tempfile.mkdtemp(prefix=".agent-sync-", dir=dest.parent))
    installed = []
    preserve = False
    try:
        for index, name in enumerate(changes):
            target = dest / name
            if name in existing:
                os.link(target, stage / f"old-{index}")
            with (stage / f"new-{index}").open("wb") as file:
                file.write(expected[name].encode("utf-8"))
                file.flush()
                os.fsync(file.fileno())
            if name in existing:
                os.chmod(stage / f"new-{index}", target.stat().st_mode & 0o777)
        for index, name in enumerate(changes):
            os.replace(stage / f"new-{index}", dest / name)
            installed.append((index, name))
    except OSError as error:
        failures = []
        for index, name in reversed(installed):
            try:
                if name in existing:
                    os.replace(stage / f"old-{index}", dest / name)
                else:
                    (dest / name).unlink()
            except OSError as rollback_error:
                failures.append(f"{name}: {rollback_error}")
        if failures:
            preserve = True
            raise OSError(f"{error}; rollback failed: {', '.join(failures)}; backups: {stage}") from error
        raise
    finally:
        if not preserve:
            for path in stage.iterdir():
                path.unlink()
            stage.rmdir()


def sync(root, check):
    source = root / ".agents" / "agents"
    dest = root / ".opencode" / "agents"
    for path in (root / ".agents", source, root / ".opencode", dest):
        if path.is_symlink() or (path.exists() and not path.is_dir()):
            raise ValueError(f"unsafe directory: {path}")
    if not source.is_dir():
        raise ValueError(f"missing source directory: {source}")
    agents = {}
    for path in sorted(source.iterdir()):
        if path.is_symlink() or not path.is_file() or path.suffix != ".md":
            raise ValueError(f"unsupported source: {path}")
        fields, tools, source_hash = parse_source(path)
        agents[fields["name"]] = (fields, tools, source_hash)
    if "coordinator" not in agents or "Agent" not in agents["coordinator"][1]:
        raise ValueError("missing coordinator or delegation capability")
    expected = {name + ".md": render(*agent, sorted(agents)) for name, agent in agents.items()}
    existing = {p.name: p for p in dest.iterdir()} if dest.exists() else {}
    conflicts = [f"unsafe destination symlink: {path}" for path in existing.values() if path.is_symlink()]
    changes = []
    for name, content in expected.items():
        path = existing.get(name)
        if path is None:
            changes.append(name)
        elif path.is_symlink() or not path.is_file():
            conflicts.append(f"unsafe destination: {path}")
        else:
            old = path.read_text(encoding="utf-8")
            if not managed(old):
                conflicts.append(f"unmanaged or edited destination: {path}")
            elif old != content:
                changes.append(name)
    for name, path in existing.items():
        if name not in expected and not path.is_symlink() and path.is_file():
            if path.read_text(encoding="utf-8", errors="replace").startswith("---\n" + MARKER):
                conflicts.append(f"stale destination without source: {path}")
    if conflicts:
        raise ValueError("\n".join(conflicts))
    for name in changes:
        print(f"{'stale' if name in existing else 'missing'} destination: {dest / name}")
    if check:
        return 1 if changes else 0
    if changes:
        install(dest, expected, changes, existing)
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--check", action="store_true", help="report drift without writing")
    args = parser.parse_args()
    try:
        return sync(args.root, args.check)
    except (OSError, UnicodeError, ValueError) as error:
        print(error, file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
