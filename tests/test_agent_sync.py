import hashlib
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "tools" / "sync_agents.py"


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


if __name__ == "__main__":
    unittest.main()
