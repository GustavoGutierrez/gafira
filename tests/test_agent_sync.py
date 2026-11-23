"""Tests for tools/sync_agents.py. Every test runs on a temporary tree."""

import hashlib
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "tools" / "sync_agents.py"
RUNTIMES = (".claude", ".opencode", ".pi")
KINDS = ("agents", "skills")

AGENT = ('---\nname: {name}\ndescription: "Describe {name}: in detail."\n'
         "mode: subagent\nsubagent_mode: task\n---\n\nInstructions.\n")


def adapter(marker="# agent-sync-sha256: ", body="description: x\n---\n\nPointer.\n"):
    return "---\n" + marker + hashlib.sha256(body.encode()).hexdigest() + "\n" + body


class AgentSyncTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.agents = self.root / ".agents" / "agents"
        self.agents.mkdir(parents=True)
        (self.root / ".agents" / "skills" / "demo").mkdir(parents=True)
        (self.root / ".agents" / "skills" / "demo" / "SKILL.md").write_text("skill\n")
        self.write_agent("manual-writer")

    def write_agent(self, name, text=None):
        (self.agents / f"{name}.md").write_text(text or AGENT.format(name=name), encoding="utf-8")

    def run_sync(self, *args):
        return subprocess.run([sys.executable, str(SCRIPT), "--root", str(self.root), *args],
                              text=True, capture_output=True, check=False)

    def snapshot(self):
        state = {}
        for path in sorted(self.root.rglob("*")):
            key = str(path.relative_to(self.root))
            if path.is_symlink():
                state[key] = "link:" + os.readlink(path)
            elif path.is_file():
                state[key] = path.read_bytes()
            else:
                state[key] = "dir"
        return state

    def assert_linked(self):
        for runtime in RUNTIMES:
            for kind in KINDS:
                path = self.root / runtime / kind
                self.assertTrue(path.is_symlink(), path)
                self.assertEqual(os.readlink(path), os.path.join("..", ".agents", kind))
                self.assertEqual(path.resolve(), (self.root / ".agents" / kind).resolve())

    def test_creates_relative_links_and_is_idempotent(self):
        self.assertEqual(self.run_sync().returncode, 0)
        self.assert_linked()
        self.assertTrue((self.root / ".pi" / "skills" / "demo" / "SKILL.md").is_file())
        self.assertTrue((self.root / ".opencode" / "agents" / "manual-writer.md").is_file())
        before = self.snapshot()
        result = self.run_sync("--check")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.run_sync().returncode, 0)
        self.assertEqual(before, self.snapshot())

    def test_check_reports_missing_links_without_writing(self):
        before = self.snapshot()
        result = self.run_sync("--check")
        self.assertEqual(result.returncode, 1)
        self.assertIn("missing link", result.stdout)
        self.assertIn(".opencode", result.stdout)
        self.assertEqual(before, self.snapshot())

    def test_wrong_link_is_reported_and_repaired(self):
        self.assertEqual(self.run_sync().returncode, 0)
        path = self.root / ".pi" / "skills"
        path.unlink()
        path.symlink_to(Path("..") / ".agents" / "agents")
        result = self.run_sync("--check")
        self.assertEqual(result.returncode, 1)
        self.assertIn("wrong link", result.stdout)
        self.assertEqual(self.run_sync().returncode, 0)
        self.assert_linked()

    def test_absolute_link_is_replaced_by_a_relative_one(self):
        self.assertEqual(self.run_sync().returncode, 0)
        path = self.root / ".claude" / "agents"
        path.unlink()
        path.symlink_to(self.agents)
        self.assertEqual(self.run_sync("--check").returncode, 1)
        self.assertEqual(self.run_sync().returncode, 0)
        self.assert_linked()

    def test_generated_adapters_are_removed_and_replaced_by_links(self):
        for runtime, marker in ((".opencode", "# agent-sync-sha256: "),
                                (".pi", "# pi-agent-sync-sha256: ")):
            directory = self.root / runtime / "agents"
            directory.mkdir(parents=True)
            (directory / "manual-writer.md").write_text(adapter(marker))
            (directory / "orphan.md").write_text(adapter(marker))
        prompts = self.root / ".pi" / "prompts"
        prompts.mkdir()
        (prompts / "coordinator.md").write_text(adapter("# pi-agent-sync-sha256: "))
        before = self.snapshot()
        result = self.run_sync("--check")
        self.assertEqual(result.returncode, 1)
        self.assertIn("generated directory instead of link", result.stdout)
        self.assertIn("obsolete generated directory", result.stdout)
        self.assertEqual(before, self.snapshot())
        self.assertEqual(self.run_sync().returncode, 0)
        self.assert_linked()
        self.assertFalse(prompts.exists())
        self.assertFalse((self.agents / "orphan.md").exists())
        self.assertEqual(self.run_sync("--check").returncode, 0)

    def test_edited_adapter_is_a_conflict_and_nothing_changes(self):
        directory = self.root / ".opencode" / "agents"
        directory.mkdir(parents=True)
        (directory / "manual-writer.md").write_text(adapter() + "Hand edit.\n")
        before = self.snapshot()
        result = self.run_sync()
        self.assertEqual(result.returncode, 2)
        self.assertIn("unmanaged content", result.stderr)
        self.assertEqual(before, self.snapshot())

    def test_real_skills_directory_is_a_conflict(self):
        skills = self.root / ".pi" / "skills" / "mine"
        skills.mkdir(parents=True)
        (skills / "SKILL.md").write_text("my skill\n")
        before = self.snapshot()
        self.assertEqual(self.run_sync().returncode, 2)
        self.assertEqual(self.run_sync("--check").returncode, 2)
        self.assertEqual(before, self.snapshot())

    def test_runtime_directory_symlink_is_a_conflict(self):
        (self.root / "elsewhere").mkdir()
        (self.root / ".pi").symlink_to(self.root / "elsewhere")
        self.assertEqual(self.run_sync().returncode, 2)
        self.assertEqual(list((self.root / "elsewhere").iterdir()), [])

    def test_unrelated_runtime_files_are_kept(self):
        (self.root / ".pi").mkdir()
        (self.root / ".pi" / "subagents.json").write_text("{}\n")
        (self.root / ".opencode").mkdir()
        (self.root / ".opencode" / "opencode.jsonc").write_text("{}\n")
        self.assertEqual(self.run_sync().returncode, 0)
        self.assertEqual((self.root / ".pi" / "subagents.json").read_text(), "{}\n")
        self.assertEqual((self.root / ".opencode" / "opencode.jsonc").read_text(), "{}\n")

    def test_runtime_specific_frontmatter_is_rejected_before_any_write(self):
        cases = {
            "claude-tools": "tools: Read, Edit, Bash\n",
            "claude-model": "model: sonnet\n",
        }
        for name, extra in cases.items():
            with self.subTest(name=name):
                self.write_agent("manual-writer", AGENT.format(name="manual-writer")
                                 .replace("subagent_mode: task\n", f"subagent_mode: task\n{extra}"))
                before = self.snapshot()
                result = self.run_sync()
                self.assertEqual(result.returncode, 2)
                self.assertIn("frontmatter keys must be exactly", result.stderr)
                self.assertEqual(before, self.snapshot())

    def test_invalid_frontmatter_values_are_rejected(self):
        base = AGENT.format(name="manual-writer")
        cases = {
            "folded description": base.replace('"Describe manual-writer: in detail."',
                                               ">\n  Folded."),
            "inner quote": base.replace("in detail.", 'in "detail".'),
            "pi rejects opencode mode": base.replace("subagent_mode: task", "subagent_mode: subagent"),
            "primary mode": base.replace("mode: subagent\n", "mode: primary\n", 1),
            "name mismatch": base.replace("name: manual-writer", "name: other"),
            "empty body": base.replace("Instructions.\n", ""),
            "missing subagent_mode": base.replace("subagent_mode: task\n", ""),
        }
        for label, text in cases.items():
            with self.subTest(label=label):
                self.write_agent("manual-writer", text)
                result = self.run_sync("--check")
                self.assertEqual(result.returncode, 2, result.stdout)
                self.assertFalse((self.root / ".pi").exists())

    def test_non_markdown_or_symlinked_agent_source_is_rejected(self):
        os.symlink("manual-writer.md", self.agents / "alias.md")
        self.assertEqual(self.run_sync().returncode, 2)
        (self.agents / "alias.md").unlink()
        (self.agents / "notes.txt").write_text("x\n")
        self.assertEqual(self.run_sync().returncode, 2)

    def test_missing_source_directory_fails(self):
        shutil.rmtree(self.root / ".agents" / "skills")
        result = self.run_sync()
        self.assertEqual(result.returncode, 2)
        self.assertIn("missing source directory", result.stderr)

    def test_repository_agents_are_valid_shared_definitions(self):
        """The real sources pass the validator (read only, nothing is written)."""
        shutil.rmtree(self.agents)
        shutil.copytree(ROOT / ".agents" / "agents", self.agents)
        self.assertEqual(self.run_sync().returncode, 0)
        self.assertEqual(self.run_sync("--check").returncode, 0)


if __name__ == "__main__":
    unittest.main()
