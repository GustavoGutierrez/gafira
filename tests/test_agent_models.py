"""Per-runtime model settings stay outside the shared agent files."""

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from urllib.parse import quote


ROOT = Path(__file__).resolve().parents[1]
TEMPLATE = ROOT / ".opencode" / "opencode.example.jsonc"
PI_CONFIG = ROOT / ".pi" / "subagents.json"
SYNC = ROOT / "tools" / "sync_agents.py"


def agent_names():
    return sorted(path.stem for path in (ROOT / ".agents" / "agents").glob("*.md"))


class SharedAgentModelTests(unittest.TestCase):
    def test_shared_agents_pin_no_model(self):
        for name in agent_names():
            text = (ROOT / ".agents" / "agents" / f"{name}.md").read_text(encoding="utf-8")
            front = text.split("---\n")[1]
            self.assertNotIn("model:", front, name)


class OpenCodeModelTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="agent-model-test-", dir=ROOT)
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        shutil.copytree(ROOT / ".agents", self.root / ".agents")
        self.sync()

    def sync(self, *args):
        subprocess.run([sys.executable, str(SYNC), "--root", str(self.root), *args],
                       check=True, capture_output=True, text=True)

    def test_template_has_no_active_model(self):
        text = TEMPLATE.read_text(encoding="utf-8")
        config = json.loads("\n".join(line for line in text.splitlines()
                                     if not line.lstrip().startswith("//")))
        self.assertEqual(config["agents"], {})
        self.assertNotIn("model", config)
        self.assertIn('"model": "provider/model#variant"', text)

    def test_sync_preserves_manual_project_model_settings(self):
        path = self.root / ".opencode" / "opencode.jsonc"
        text = '{"agents":{"manual-writer":{"model":"openrouter/vendor/model#high"}}}\n'
        path.write_text(text, encoding="utf-8")
        self.sync()
        self.sync("--check")
        self.assertEqual(path.read_text(encoding="utf-8"), text)

    @unittest.skipUnless(os.environ.get("GAFIRA_TEST_OPENCODE") == "1", "requires OpenCode API")
    def test_opencode_loads_shared_agents_and_project_model_overrides(self):
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        subprocess.run(["git", "-C", str(self.root), "-c", "user.name=Test", "-c",
                        "user.email=test@example.invalid", "-c", f"core.hooksPath={self.root}",
                        "commit", "-q", "--allow-empty",
                        "-m", "test: initialize scratch project"], check=True)
        query = "?location%5Bdirectory%5D=" + quote(str(self.root), safe="")

        def api(method, path):
            result = subprocess.run(["opencode", "api", method, path + query],
                                    text=True, capture_output=True)
            if result.returncode:
                self.fail(f"OpenCode API {path}: {result.stdout} {result.stderr}")
            if path == "/api/agent" or not result.stdout:
                return None
            data = json.loads(result.stdout)
            return data.get("data", data)

        self.addCleanup(lambda: subprocess.run(
            ["opencode", "api", "delete", "/api/debug/location" + query],
            capture_output=True, check=False))
        api("get", "/api/agent")
        for name in agent_names():
            agent = api("get", f"/api/agent/{name}")
            self.assertEqual(agent["mode"], "subagent", name)
            self.assertTrue(agent["description"], name)
        config = self.root / ".opencode" / "opencode.jsonc"
        config.write_text(json.dumps({"agents": {
            "manual-writer": {"model": "openrouter/vendor/model#high"},
        }}), encoding="utf-8")
        api("delete", "/api/debug/location")
        api("get", "/api/agent")
        writer = api("get", "/api/agent/manual-writer")
        self.assertEqual(writer["model"],
                         {"providerID": "openrouter", "id": "vendor/model", "variant": "high"})
        self.assertEqual(writer["mode"], "subagent")


class PiModelConfigTests(unittest.TestCase):
    """The Pi model configuration stays project local and per agent."""

    def setUp(self):
        self.data = json.loads(PI_CONFIG.read_text(encoding="utf-8"))

    def test_config_has_model_profiles(self):
        self.assertIsInstance(self.data.get("model_profiles"), dict)
        self.assertTrue(self.data["model_profiles"], "model_profiles is empty")

    def test_every_project_agent_has_a_model_profile(self):
        for name in agent_names():
            self.assertIn(name, self.data["model_profiles"], f"{name} has no model profile")

    def test_profiles_use_a_provider_qualified_model(self):
        for name, profile in self.data["model_profiles"].items():
            model = profile["model"]
            self.assertIn("/", model, f"{name} model is not provider qualified")
            self.assertFalse(model.startswith("/"), f"{name} model looks like a local path")
            self.assertIn(profile["effort"],
                          ("off", "minimal", "low", "medium", "high", "xhigh", "max"))


if __name__ == "__main__":
    unittest.main()
