#!/usr/bin/env python3
"""Link every agent runtime to `.agents/`, the single source of truth.

    .agents/agents/<name>.md        shared agent definitions
    .agents/skills/<name>/SKILL.md  shared skills

Claude Code, OpenCode and Pi read the same files through relative symlinks:

    .claude/agents   -> ../.agents/agents     .claude/skills   -> ../.agents/skills
    .opencode/agents -> ../.agents/agents     .opencode/skills -> ../.agents/skills
    .pi/agents       -> ../.agents/agents     .pi/skills       -> ../.agents/skills

Because one file serves three runtimes, its frontmatter may use only keys that
all of them accept (see `FRONTMATTER`); per-runtime settings such as models live
in each runtime's own config. The script also removes the generated adapters
that earlier versions wrote, but only when they still carry their unedited
marker; anything else in the way is a conflict and is left alone.

Usage:
    python3 tools/sync_agents.py            # create or repair the links
    python3 tools/sync_agents.py --check    # report drift, change nothing

Exit status: 0 in sync, 1 drift found by --check, 2 invalid source or conflict.
"""

import argparse
import hashlib
import os
import re
import shutil
import sys
from pathlib import Path

SOURCE = ".agents"
RUNTIMES = (".claude", ".opencode", ".pi")
KINDS = ("agents", "skills")
# Directories that only ever held generated files and are no longer used.
LEGACY_DIRS = (Path(".pi") / "prompts",)
# Markers of the adapters written by the retired generators.
MARKERS = ("# agent-sync-sha256: ", "# pi-agent-sync-sha256: ")

# The only frontmatter keys a shared agent may use, with their allowed values
# (None means free text). Evidence for each runtime:
# - Claude Code ignores unknown keys; `name` and `description` are required.
# - OpenCode V2 drops a file whose `tools` is a string; it needs
#   `mode: subagent` (without it the agent is primary) and ignores `name`,
#   `subagent_mode` and a model alias without a provider.
# - Pi (gentle-pi) accepts only `task` or `background` as the mode; it reads
#   `subagent_mode` before `mode`, so `subagent_mode` shields it from OpenCode's
#   `mode: subagent`. Its parser is line based: a folded (`>`) description
#   reads as ">" and quotes are stripped without unescaping.
FRONTMATTER = {
    "name": None,
    "description": None,
    "mode": {"subagent"},
    "subagent_mode": {"task", "background"},
}


def link_target(kind):
    return Path("..") / SOURCE / kind


def validate_agent(path):
    """Return a list of problems with one shared agent definition."""
    text = path.read_text(encoding="utf-8")
    match = re.fullmatch(r"---\n(.*?)\n---\n(.*)", text, re.DOTALL)
    if not match:
        return [f"{path}: missing frontmatter"]
    problems = []
    fields = {}
    for line in match.group(1).splitlines():
        pair = re.fullmatch(r"([a-z_]+): (.+)", line)
        if not pair or pair.group(1) in fields:
            problems.append(f"{path}: unsupported frontmatter line: {line!r}")
            continue
        fields[pair.group(1)] = pair.group(2)
    if set(fields) != set(FRONTMATTER):
        problems.append(f"{path}: frontmatter keys must be exactly "
                        f"{', '.join(FRONTMATTER)}; found {', '.join(fields) or 'none'}")
    for key, allowed in FRONTMATTER.items():
        value = fields.get(key)
        if value is not None and allowed is not None and value not in allowed:
            problems.append(f"{path}: {key} must be one of {', '.join(sorted(allowed))}")
    name = fields.get("name")
    if name is not None and (name != path.stem or not re.fullmatch(r"[a-z][a-z0-9-]*", name)):
        problems.append(f"{path}: name must be lowercase kebab case and match the file name")
    description = fields.get("description")
    if description is not None:
        inner = description[1:-1]
        if (len(description) < 3 or description[0] != '"' or description[-1] != '"'
                or '"' in inner or "\\" in inner):
            problems.append(f"{path}: description must be one double-quoted line "
                            "with no inner quote or backslash")
    if not match.group(2).strip():
        problems.append(f"{path}: empty body")
    return problems


def validate_sources(root):
    problems = []
    for kind in KINDS:
        directory = root / SOURCE / kind
        if directory.is_symlink() or not directory.is_dir():
            problems.append(f"missing source directory: {directory}")
    if problems:
        return problems
    for path in sorted((root / SOURCE / "agents").iterdir()):
        if path.is_symlink() or not path.is_file() or path.suffix != ".md":
            problems.append(f"unsupported agent source: {path}")
        else:
            problems.extend(validate_agent(path))
    return problems


def generated(path):
    """Report whether a file is an unedited adapter from a retired generator."""
    if path.is_symlink() or not path.is_file():
        return False
    text = path.read_text(encoding="utf-8", errors="replace")
    for marker in MARKERS:
        prefix = "---\n" + marker
        if text.startswith(prefix):
            line, sep, body = text[len(prefix):].partition("\n")
            return bool(sep) and line == hashlib.sha256(body.encode("utf-8")).hexdigest()
    return False


def removable(directory):
    """A real directory may be replaced only if it holds nothing but adapters."""
    return all(generated(path) for path in directory.iterdir())


def plan(root):
    """Return (actions, conflicts); an action is (description, callable)."""
    actions = []
    conflicts = []
    for runtime in RUNTIMES:
        base = root / runtime
        if base.is_symlink() or (base.exists() and not base.is_dir()):
            conflicts.append(f"not a directory: {base}")
            continue
        for kind in KINDS:
            path = base / kind
            target = link_target(kind)
            if path.is_symlink():
                if Path(os.readlink(path)) == target:
                    continue
                actions.append((f"wrong link: {path} -> {os.readlink(path)}",
                                lambda p=path, t=target: (p.unlink(), p.symlink_to(t))))
            elif not path.exists():
                actions.append((f"missing link: {path}",
                                lambda p=path, t=target: (p.parent.mkdir(exist_ok=True),
                                                          p.symlink_to(t))))
            elif path.is_dir() and removable(path):
                actions.append((f"generated directory instead of link: {path}",
                                lambda p=path, t=target: (shutil.rmtree(p), p.symlink_to(t))))
            else:
                conflicts.append(f"real file or directory with unmanaged content "
                                 f"where a link belongs: {path}")
    for legacy in LEGACY_DIRS:
        path = root / legacy
        if path.is_symlink() or not path.exists():
            continue
        if path.is_dir() and removable(path):
            actions.append((f"obsolete generated directory: {path}",
                            lambda p=path: shutil.rmtree(p)))
        else:
            conflicts.append(f"obsolete directory with unmanaged content: {path}")
    return actions, conflicts


def sync(root, check):
    problems = validate_sources(root)
    actions, conflicts = ([], []) if problems else plan(root)
    if problems or conflicts:
        print("\n".join(problems + conflicts), file=sys.stderr)
        return 2
    for description, _ in actions:
        print(description)
    if check:
        return 1 if actions else 0
    for _, apply in actions:
        apply()
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--check", action="store_true", help="report drift without writing")
    args = parser.parse_args()
    try:
        return sync(args.root, args.check)
    except (OSError, UnicodeError) as error:
        print(error, file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
