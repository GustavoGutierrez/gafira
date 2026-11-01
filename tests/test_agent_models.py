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
SYNC = ROOT / "tools" / "sync_agents.py"


class AgentModelTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="agent-model-test-", dir=ROOT)
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        shutil.copytree(ROOT / ".agents" / "agents", self.root / ".agents" / "agents")
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
        text = '{"agents":{"verify":{"model":"ollama/qwen3:8b"},'
        text += '"explorer":{"model":"openrouter/vendor/model#high"}}}\n'
        path.write_text(text, encoding="utf-8")
        source = self.root / ".agents" / "agents" / "verify.md"
        source.write_text(source.read_text(encoding="utf-8") + "\nChanged source.\n", encoding="utf-8")
        self.sync()
        self.sync("--check")
        self.assertEqual(path.read_text(encoding="utf-8"), text)
        self.assertNotIn("model:", (self.root / ".opencode" / "agents" / "verify.md").read_text())

    @unittest.skipUnless(os.environ.get("GAFIRA_TEST_OPENCODE") == "1", "requires OpenCode API")
    def test_opencode_api_loads_project_model_overrides(self):
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        subprocess.run(["git", "-C", str(self.root), "-c", "user.name=Test", "-c",
                        "user.email=test@example.invalid", "-c", f"core.hooksPath={self.root}",
                        "commit", "-q", "--allow-empty",
                        "-m", "test: initialize scratch project"], check=True)
        query = "?location%5Bdirectory%5D=" + quote(str(self.root), safe="")
        def api(method, path):
            result = subprocess.run(["opencode", "api", method, path + query], text=True, capture_output=True)
            if result.returncode:
                self.fail(f"OpenCode API {path}: {result.stdout} {result.stderr}")
            output = result.stdout
            if path == "/api/agent":
                return None
            data = json.loads(output) if output else None
            return data.get("data", data) if data else None
        self.addCleanup(lambda: subprocess.run(["opencode", "api", "delete", "/api/debug/location" + query],
                                               capture_output=True, check=False))
        api("get", "/api/agent")
        before = api("get", "/api/agent/tdd-apply")
        explorer_before = api("get", "/api/agent/explorer")
        config = self.root / ".opencode" / "opencode.jsonc"
        config.write_text(json.dumps({"agents": {
            "verify": {"model": "ollama/qwen3:8b"},
            "explorer": {"model": "openrouter/vendor/model#high"},
        }}), encoding="utf-8")
        api("delete", "/api/debug/location")
        api("get", "/api/agent")
        explorer = api("get", "/api/agent/explorer")
        verify = api("get", "/api/agent/verify")
        after = api("get", "/api/agent/tdd-apply")
        self.assertEqual(explorer["model"], {"providerID": "openrouter", "id": "vendor/model", "variant": "high"})
        self.assertEqual(verify["model"], {"providerID": "ollama", "id": "qwen3:8b"})
        self.assertEqual(before, after)
        self.assertEqual(explorer["mode"], "subagent")
        self.assertEqual(explorer["system"], explorer_before["system"])
        self.assertEqual(explorer["permissions"], explorer_before["permissions"])
        config_before = config.read_bytes()
        self.sync()
        self.sync("--check")
        self.assertEqual(config.read_bytes(), config_before)
        self.assertEqual(api("get", "/api/agent/explorer")["model"], explorer["model"])


if __name__ == "__main__":
    unittest.main()
