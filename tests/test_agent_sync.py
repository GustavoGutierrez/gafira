import hashlib
import importlib.util
import os
import subprocess
import sys
import tempfile
import unittest
from fnmatch import fnmatchcase
from pathlib import Path
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "tools" / "sync_agents.py"
spec = importlib.util.spec_from_file_location("sync_agents", SCRIPT)
sync_agents = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sync_agents)


class AgentSyncTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.source = self.root / ".agents" / "agents"
        self.dest = self.root / ".opencode" / "agents"
        self.source.mkdir(parents=True)
        self.write_agent("coordinator", "Agent, SendMessage, Read, Edit, Write, Glob, Grep, Bash")
        self.write_agent("verify", "Read, Grep, Glob, Bash")

    def write_agent(self, name, tools, body="Original instructions.\n"):
        (self.source / f"{name}.md").write_text(
            f"---\nname: {name}\ndescription: >\n  Describe {name}\n  in detail.\n"
            f"model: opus\ntools: {tools}\n---\n\n{body}", encoding="utf-8"
        )

    def run_sync(self, *args):
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(self.root), *args],
            text=True, capture_output=True, check=False,
        )

    def test_generate_and_check_are_idempotent(self):
        self.assertEqual(self.run_sync().returncode, 0)
        before = {p.name: p.read_bytes() for p in self.dest.iterdir()}
        self.assertEqual(self.run_sync("--check").returncode, 0)
        self.assertEqual(self.run_sync().returncode, 0)
        self.assertEqual(before, {p.name: p.read_bytes() for p in self.dest.iterdir()})
        coordinator = before["coordinator.md"].decode()
        verify = before["verify.md"].decode()
        self.assertIn('action: "*"', coordinator)
        self.assertIn("mode: primary", coordinator)
        self.assertIn('resource: "verify"', coordinator)
        self.assertIn('resource: "general"', coordinator)
        self.assertIn("mode: subagent", verify)
        self.assertIn("action: edit", verify)
        self.assertIn("action: subagent", verify)
        self.assertNotIn("model:", coordinator)
        self.assertIn(".agents/agents/verify.md", verify)
        self.assertIn("Describe verify in detail.", verify)

    def test_source_change_is_reported_without_writes_and_then_updated(self):
        self.assertEqual(self.run_sync().returncode, 0)
        before = (self.dest / "verify.md").read_bytes()
        self.write_agent("verify", "Read, Grep, Glob, Bash", "Changed instructions.\n")
        result = self.run_sync("--check")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("verify.md", result.stdout + result.stderr)
        self.assertEqual(before, (self.dest / "verify.md").read_bytes())
        self.assertEqual(self.run_sync().returncode, 0)
        self.assertNotEqual(before, (self.dest / "verify.md").read_bytes())

    def test_unmanaged_collision_is_not_overwritten_or_partially_applied(self):
        self.assertEqual(self.run_sync().returncode, 0)
        before = (self.dest / "coordinator.md").read_bytes()
        (self.dest / "verify.md").write_text("My own agent\n")
        self.write_agent("coordinator", "Agent, SendMessage, Read, Edit, Write, Glob, Grep, Bash", "New body\n")
        self.assertNotEqual(self.run_sync().returncode, 0)
        self.assertEqual(before, (self.dest / "coordinator.md").read_bytes())
        self.assertEqual("My own agent\n", (self.dest / "verify.md").read_text())

    def test_edited_managed_file_is_not_overwritten(self):
        self.assertEqual(self.run_sync().returncode, 0)
        path = self.dest / "verify.md"
        path.write_text(path.read_text() + "Manual edit\n")
        before = path.read_bytes()
        self.assertNotEqual(self.run_sync().returncode, 0)
        self.assertEqual(before, path.read_bytes())

    def test_stale_destination_is_reported_never_deleted(self):
        self.assertEqual(self.run_sync().returncode, 0)
        stale = self.dest / "verify.md"
        (self.source / "verify.md").unlink()
        result = self.run_sync()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("verify.md", result.stdout + result.stderr)
        self.assertTrue(stale.exists())

    def test_unrelated_destination_is_preserved(self):
        self.dest.mkdir(parents=True)
        custom = self.dest / "custom.md"
        custom.write_text("User agent\n")
        self.assertEqual(self.run_sync().returncode, 0)
        self.assertEqual(custom.read_text(), "User agent\n")

    def test_unsupported_source_prevents_all_writes(self):
        self.assertEqual(self.run_sync().returncode, 0)
        before = {p.name: hashlib.sha256(p.read_bytes()).digest() for p in self.dest.iterdir()}
        self.write_agent("coordinator", "Agent, SendMessage, Read, Edit, Write, Glob, Grep, Bash", "New body\n")
        self.write_agent("verify", "Read, Unknown")
        self.assertNotEqual(self.run_sync().returncode, 0)
        self.assertEqual(before, {p.name: hashlib.sha256(p.read_bytes()).digest() for p in self.dest.iterdir()})

    def test_symlink_collision_is_never_followed(self):
        self.dest.mkdir(parents=True)
        external = self.root / "external"
        external.write_text("keep")
        (self.dest / "verify.md").symlink_to(external)
        self.assertNotEqual(self.run_sync().returncode, 0)
        self.assertEqual("keep", external.read_text())
        self.assertFalse((self.dest / "coordinator.md").exists())

    def test_check_on_missing_destination_does_not_create_it(self):
        result = self.run_sync("--check")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.dest.exists())

    def test_source_symlink_is_rejected(self):
        (self.source / "verify.md").unlink()
        (self.source / "verify.md").symlink_to(self.source / "coordinator.md")
        self.assertNotEqual(self.run_sync().returncode, 0)
        self.assertFalse(self.dest.exists())

    def test_permissions_use_allowlist_with_explorer_write_scope(self):
        self.write_agent("explorer", "Read, Grep, Glob, Bash, Write")
        self.assertEqual(self.run_sync().returncode, 0)
        import re
        def effect(name, action, resource="*"):
            text = (self.dest / f"{name}.md").read_text()
            rules = re.findall(r'\{action: "?([\w*]+)"?, resource: "([^"]+)", effect: (\w+)\}', text)
            return [e for a, r, e in rules if fnmatchcase(action, a) and fnmatchcase(resource, r)][-1]
        self.assertEqual(effect("coordinator", "question"), "allow")
        self.assertEqual(effect("coordinator", "subagent", "verify"), "allow")
        self.assertEqual(effect("coordinator", "read", "/home/user/.ssh/id_rsa"), "deny")
        for action in ("execute", "skill", "webfetch", "example_mcp_tool"):
            self.assertEqual(effect("coordinator", action), "deny")
        self.assertEqual(effect("verify", "subagent", "explorer"), "deny")
        self.assertEqual(effect("verify", "edit", "src/lib.rs"), "deny")
        self.assertNotEqual(effect("verify", "shell"), "deny")
        self.assertEqual(effect("explorer", "edit", "src/lib.rs"), "deny")
        self.assertEqual(effect("explorer", "edit", "openspec/changes/demo/spike.md"), "allow")

    def test_stale_and_unrelated_destination_symlinks_are_rejected(self):
        self.assertEqual(self.run_sync().returncode, 0)
        (self.source / "verify.md").unlink()
        (self.dest / "verify.md").unlink()
        (self.dest / "verify.md").symlink_to(self.root / "missing")
        before = (self.dest / "coordinator.md").read_bytes()
        result = self.run_sync()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("verify.md", result.stderr)
        self.assertEqual(before, (self.dest / "coordinator.md").read_bytes())
        self.assertTrue((self.dest / "verify.md").is_symlink())
        (self.dest / "verify.md").unlink()
        (self.dest / "custom.md").symlink_to(self.root / "missing")
        self.assertNotEqual(self.run_sync("--check").returncode, 0)
        self.assertTrue((self.dest / "custom.md").is_symlink())

    def test_staging_failure_never_changes_existing_files(self):
        self.assertEqual(self.run_sync().returncode, 0)
        before = {p.name: p.read_bytes() for p in self.dest.iterdir()}
        self.write_agent("verify", "Read, Grep, Glob, Bash", "Changed instructions.\n")
        with patch.object(sync_agents.os, "fsync", side_effect=OSError("disk full")):
            with self.assertRaises(OSError):
                sync_agents.sync(self.root, False)
        self.assertEqual(before, {p.name: p.read_bytes() for p in self.dest.iterdir()})

    def test_replace_failure_rolls_back_old_files_and_new_files(self):
        self.assertEqual(self.run_sync().returncode, 0)
        before = {p.name: p.read_bytes() for p in self.dest.iterdir()}
        self.write_agent("coordinator", "Agent, SendMessage, Read, Edit, Write, Glob, Grep, Bash", "New body\n")
        self.write_agent("explorer", "Read, Grep, Glob, Bash, Write")
        self.write_agent("verify", "Read, Grep, Glob, Bash", "New body\n")
        real_replace = os.replace
        calls = 0
        def fail_third(source, dest):
            nonlocal calls
            calls += 1
            if calls == 3:
                raise OSError("replace failed")
            return real_replace(source, dest)
        with patch.object(sync_agents.os, "replace", side_effect=fail_third):
            with self.assertRaises(OSError):
                sync_agents.sync(self.root, False)
        self.assertEqual(before, {p.name: p.read_bytes() for p in self.dest.iterdir()})

    def test_partial_stage_write_failure_leaves_destinations_unchanged(self):
        self.assertEqual(self.run_sync().returncode, 0)
        before = {p.name: p.read_bytes() for p in self.dest.iterdir()}
        self.write_agent("verify", "Read, Grep, Glob, Bash", "New body\n")
        real_open = Path.open
        class BrokenWrite:
            def __init__(self, path):
                self.file = real_open(path, "wb")
            def __enter__(self):
                return self
            def __exit__(self, *args):
                self.file.close()
            def write(self, data):
                self.file.write(data[:8])
                raise OSError("disk full")
        def open_file(path, *args, **kwargs):
            return BrokenWrite(path) if path.name.startswith("new-") else real_open(path, *args, **kwargs)
        with patch.object(Path, "open", open_file):
            with self.assertRaises(OSError):
                sync_agents.sync(self.root, False)
        self.assertEqual(before, {p.name: p.read_bytes() for p in self.dest.iterdir()})

    def test_failed_rollback_retains_backups(self):
        self.assertEqual(self.run_sync().returncode, 0)
        original = (self.dest / "coordinator.md").read_bytes()
        self.write_agent("coordinator", "Agent, SendMessage, Read, Edit, Write, Glob, Grep, Bash", "New body\n")
        self.write_agent("verify", "Read, Grep, Glob, Bash", "New body\n")
        real_replace = os.replace
        calls = 0
        def fail_from_second(source, dest):
            nonlocal calls
            calls += 1
            if calls >= 2:
                raise OSError("replace failed")
            return real_replace(source, dest)
        with patch.object(sync_agents.os, "replace", side_effect=fail_from_second):
            with self.assertRaisesRegex(OSError, "rollback failed.*backups:"):
                sync_agents.sync(self.root, False)
        stages = list(self.dest.parent.glob(".agent-sync-*"))
        self.assertEqual(len(stages), 1)
        self.assertEqual((stages[0] / "old-0").read_bytes(), original)


if __name__ == "__main__":
    unittest.main()
