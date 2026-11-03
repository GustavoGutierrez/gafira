"""Tests for the Pi agent synchronization script.

Run with: python3 -m unittest discover -s tests -p 'test_pi_agent*.py' -v
"""

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "tools" / "sync_pi_agents.py"
_spec = importlib.util.spec_from_file_location("sync_pi_agents", SCRIPT)
if _spec is None or _spec.loader is None:
    raise RuntimeError(f"cannot load {SCRIPT}")
sync_pi_agents = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(sync_pi_agents)


class PiAgentSyncTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.source = self.root / ".agents" / "agents"
        self.source.mkdir(parents=True)
        self.write_agent("coordinator", "Agent, SendMessage, Read, Edit, Write, Glob, Grep, Bash")
        self.write_agent("tdd-apply", "Read, Edit, Write, Glob, Grep, Bash")
        self.write_agent("verify", "Read, Grep, Glob, Bash")

    def write_agent(self, name, tools, body="Original instructions.\n", description=None):
        if description is None:
            description = f"Describe {name}\n  in detail."
        (self.source / f"{name}.md").write_text(
            f"---\nname: {name}\ndescription: >\n  {description}\n"
            f"model: opus\ntools: {tools}\n---\n\n{body}", encoding="utf-8"
        )

    def run_sync(self, *args):
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(self.root), *args],
            text=True, capture_output=True, check=False,
        )

    def test_generate_writes_agents_and_coordinator_prompt(self):
        result = self.run_sync()
        self.assertEqual(result.returncode, 0, result.stderr)
        agents = self.root / ".pi" / "agents"
        self.assertEqual(
            sorted(p.name for p in agents.iterdir()),
            ["tdd-apply.md", "verify.md"],
        )
        self.assertTrue((self.root / ".pi" / "prompts" / "coordinator.md").exists())

    def test_generated_agent_uses_pi_tool_names(self):
        self.run_sync()
        content = (self.root / ".pi" / "agents" / "tdd-apply.md").read_text()
        self.assertIn("tools: read, edit, write, find, grep, bash", content)
        for claude_only in ("Read", "Edit", "Write", "Glob", "Grep", "Bash"):
            self.assertNotIn(f"tools: {claude_only}", content)

    def test_verify_agent_has_no_write_tools(self):
        self.run_sync()
        content = (self.root / ".pi" / "agents" / "verify.md").read_text()
        self.assertIn("tools: read, grep, find, bash", content)

    def test_coordinator_prompt_points_at_the_source(self):
        self.assertEqual(self.run_sync().returncode, 0)
        content = (self.root / ".pi" / "prompts" / "coordinator.md").read_text()
        self.assertIn("read `.agents/agents/coordinator.md`", content)

    def test_generated_definitions_do_not_copy_the_source_body(self):
        """A copied body is a copy that goes stale; the definition points at the source."""
        self.assertEqual(self.run_sync().returncode, 0)
        for name in ("tdd-apply", "verify"):
            content = (self.root / ".pi" / "agents" / f"{name}.md").read_text()
            self.assertNotIn("Original instructions.", content)
            self.assertIn(f"read `.agents/agents/{name}.md`", content)

    def test_a_body_change_needs_no_resynchronization(self):
        """Editing the source body must not invalidate a generated definition."""
        self.assertEqual(self.run_sync().returncode, 0)
        self.write_agent("tdd-apply", "Read, Edit, Write, Glob, Grep, Bash",
                         body="Changed instructions.\n")
        self.assertEqual(self.run_sync("--check").returncode, 0)
        content = (self.root / ".pi" / "agents" / "tdd-apply.md").read_text()
        self.assertNotIn("Changed instructions.", content)

    def test_a_frontmatter_change_needs_a_resynchronization(self):
        """The derived frontmatter is what the definition owns, so it must resync."""
        self.assertEqual(self.run_sync().returncode, 0)
        self.write_agent("tdd-apply", "Read, Bash")
        result = self.run_sync("--check")
        self.assertEqual(result.returncode, 1)
        self.assertIn("stale destination", result.stdout)

    def test_check_is_idempotent_and_detects_drift(self):
        self.assertEqual(self.run_sync().returncode, 0)
        self.assertEqual(self.run_sync("--check").returncode, 0)
        self.write_agent("tdd-apply", "Read, Bash")
        result = self.run_sync("--check")
        self.assertEqual(result.returncode, 1)
        self.assertIn("stale destination", result.stdout)

    def test_check_does_not_write(self):
        self.assertEqual(self.run_sync().returncode, 0)
        self.write_agent("tdd-apply", "Read, Bash")
        self.run_sync("--check")
        content = (self.root / ".pi" / "agents" / "tdd-apply.md").read_text()
        self.assertIn("tools: read, edit, write, find, grep, bash", content)

    def test_sync_regenerates_a_stale_generated_file(self):
        self.assertEqual(self.run_sync().returncode, 0)
        self.write_agent("tdd-apply", "Read, Bash")
        self.assertEqual(self.run_sync().returncode, 0)
        content = (self.root / ".pi" / "agents" / "tdd-apply.md").read_text()
        self.assertIn("tools: read, bash", content)

    def test_unmanaged_destination_is_reported_and_not_overwritten(self):
        self.assertEqual(self.run_sync().returncode, 0)
        target = self.root / ".pi" / "agents" / "tdd-apply.md"
        target.write_text("Hand written, not generated.\n")
        result = self.run_sync()
        self.assertEqual(result.returncode, 2)
        self.assertIn("unmanaged or edited destination", result.stderr)
        self.assertEqual(target.read_text(), "Hand written, not generated.\n")

    def test_edited_generated_destination_is_a_conflict(self):
        self.assertEqual(self.run_sync().returncode, 0)
        target = self.root / ".pi" / "agents" / "tdd-apply.md"
        target.write_text(target.read_text().replace(
            "tools: read, edit, write, find, grep, bash",
            "tools: read, edit, write, find, grep, bash, extra"))
        result = self.run_sync()
        self.assertEqual(result.returncode, 2)
        self.assertIn("unmanaged or edited destination", result.stderr)
        self.assertIn("grep, bash, extra", target.read_text())

    def test_generated_file_without_source_is_reported(self):
        self.assertEqual(self.run_sync().returncode, 0)
        orphan = self.root / ".pi" / "agents" / "orphan.md"
        orphan.write_text(sync_pi_agents.render_agent(
            "orphan", "An orphan.", ["read"], "0" * 64))
        result = self.run_sync()
        self.assertEqual(result.returncode, 2)
        self.assertIn("stale destination without source", result.stderr)

    def test_generated_definition_records_its_source_revision(self):
        self.assertEqual(self.run_sync().returncode, 0)
        source = (self.source / "tdd-apply.md").read_text()
        revision = sync_pi_agents.digest(
            sync_pi_agents.parse_frontmatter_block(source))
        content = (self.root / ".pi" / "agents" / "tdd-apply.md").read_text()
        self.assertIn(f"Source frontmatter revision: {revision}", content)

    def test_revision_flag_reports_the_current_source_revision(self):
        self.assertEqual(self.run_sync().returncode, 0)
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(self.root),
             "--revision", "tdd-apply"],
            text=True, capture_output=True, check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        recorded = (self.root / ".pi" / "agents" / "tdd-apply.md").read_text()
        self.assertIn(result.stdout.strip(), recorded)

    def test_a_stale_copy_is_visible_without_running_the_tool(self):
        """The recorded revision lets a reader spot a stale definition by hand."""
        self.assertEqual(self.run_sync().returncode, 0)
        target = self.root / ".pi" / "agents" / "tdd-apply.md"
        recorded = target.read_text().split("Source frontmatter revision: ")[1].split("\n")[0]
        self.write_agent("tdd-apply", "Read, Bash")
        current = subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(self.root),
             "--revision", "tdd-apply"],
            text=True, capture_output=True, check=False,
        ).stdout.strip()
        self.assertNotEqual(recorded, current)

    def test_unsupported_source_is_rejected(self):
        (self.source / "broken.md").write_text(
            "---\nname: broken\ndescription: Broken\nmodel: gpt-9\n"
            "tools: Read, Bash\n---\n\nBody.\n", encoding="utf-8")
        result = self.run_sync()
        self.assertEqual(result.returncode, 2)
        self.assertIn("unsupported agent definition", result.stderr)
        self.assertFalse((self.root / ".pi" / "agents" / "broken.md").exists())

    def test_source_with_duplicate_tool_is_rejected(self):
        self.write_agent("dup", "Read, Read, Bash")
        result = self.run_sync()
        self.assertEqual(result.returncode, 2)
        self.assertIn("unsupported agent definition", result.stderr)

    def test_source_with_quote_in_description_is_rejected(self):
        self.write_agent("quoted", "Read, Bash", description='A "quoted" description.')
        result = self.run_sync()
        self.assertEqual(result.returncode, 2)
        self.assertIn("unsupported description characters", result.stderr)

    def test_description_folds_to_one_line(self):
        self.run_sync()
        content = (self.root / ".pi" / "agents" / "tdd-apply.md").read_text()
        self.assertIn('description: "Describe tdd-apply in detail."', content)

    def test_marker_detects_manual_edit(self):
        self.run_sync()
        target = self.root / ".pi" / "agents" / "tdd-apply.md"
        content = target.read_text()
        target.write_text(content.replace("read, edit", "read, edit, write"))
        self.assertFalse(sync_pi_agents.managed(target.read_text()))

    def test_missing_source_directory_fails(self):
        empty = Path(self.tmp.name) / "empty"
        empty.mkdir()
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(empty)],
            text=True, capture_output=True, check=False,
        )
        self.assertEqual(result.returncode, 2)

class PiModelConfigTests(unittest.TestCase):
    """The project model configuration stays project local and per agent."""

    def setUp(self):
        self.root = Path(__file__).resolve().parents[1]
        self.config = self.root / ".pi" / "subagents.json"

    def test_config_exists_and_is_valid_json(self):
        self.assertTrue(self.config.exists(), ".pi/subagents.json is missing")
        data = json.loads(self.config.read_text())
        self.assertIsInstance(data.get("model_profiles"), dict)
        self.assertTrue(data["model_profiles"], "model_profiles is empty")

    def test_every_project_agent_has_a_model_profile(self):
        data = json.loads(self.config.read_text())
        profiles = data["model_profiles"]
        for source in (self.root / ".agents" / "agents").glob("*.md"):
            name = source.stem
            if name == "coordinator":
                continue
            self.assertIn(name, profiles, f"{name} has no model profile")

    def test_profiles_use_a_provider_qualified_model(self):
        data = json.loads(self.config.read_text())
        for name, profile in data["model_profiles"].items():
            model = profile["model"]
            self.assertIn("/", model, f"{name} model is not provider qualified")
            self.assertIn(profile["effort"], ("off", "minimal", "low", "medium", "high", "xhigh", "max"))

    def test_config_does_not_leak_global_state(self):
        """The project must not require a global agent directory to work."""
        data = json.loads(self.config.read_text())
        for name, profile in data["model_profiles"].items():
            self.assertFalse(profile["model"].startswith("/"), f"{name} model looks like a local path")


if __name__ == "__main__":
    unittest.main()
