"""Exercise runner routing, stale candidates and safe environment exports."""
import base64
import copy
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location("runner_image", ROOT / ".github/scripts/runner-image.py")
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)
HEAD = "a" * 40
DIGEST = "sha256:" + "d" * 64


class SelectorTests(unittest.TestCase):
    def setUp(self):
        self.manifest = runner.read_manifest(ROOT / runner.MANIFEST)
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.event = Path(self.temp.name) / "event.json"
        self.output = Path(self.temp.name) / "output"
        self.pr = {"number": 4702, "state": "open", "changed_files": 1,
                   "head": {"sha": HEAD}}
        self.responses = {
            "pulls/4702": self.pr,
            "pulls/4702/files?per_page=100&page=1": [{"filename": runner.MANIFEST}],
            f"contents/{runner.MANIFEST}?ref={HEAD}": {
                "content": base64.b64encode(json.dumps(self.manifest).encode()).decode()},
            f"commits/{HEAD}/status": {"statuses": [{
                "context": "Runner image candidate / PR 4702", "state": "success",
                "creator": {"login": "github-actions[bot]"}, "description": DIGEST,
                "target_url": "https://github.com/dashpay/platform/actions/runs/7",
            }]},
            "actions/runs/7": {"path": ".github/workflows/runner-image-candidate.yml",
                              "event": "pull_request_target", "conclusion": "success"},
        }

    def select(self, event=None):
        self.event.write_text(json.dumps(event if event is not None else {"pull_request": self.pr}))
        with patch.dict(os.environ, {"GITHUB_EVENT_PATH": str(self.event)}), \
             patch.object(runner, "api", side_effect=lambda path: self.responses[path]):
            runner.select(self.manifest, "rust", self.output, 0)
        return dict(line.split("=", 1) for line in self.output.read_text().splitlines())

    def test_non_pr_and_unchanged_pr_use_existing_pool(self):
        self.assertEqual(json.loads(self.select({})["labels"]), ["self-hosted", "rust-ci"])
        self.responses["pulls/4702/files?per_page=100&page=1"] = [{"filename": "Cargo.lock"}]
        self.assertEqual(self.select()["image_changed"], "false")

    def test_exact_candidate_includes_head_digest_and_kind(self):
        output = self.select()
        labels = json.loads(output["labels"])
        self.assertEqual(labels[-1], f"platform-image-pr-4702-{HEAD}-{DIGEST[7:]}-rust")
        self.assertEqual(output["image_changed"], "true")

    def test_new_head_or_closed_pr_rejects_stale_run(self):
        event = {"pull_request": copy.deepcopy(self.pr)}
        self.pr["head"]["sha"] = "e" * 40
        with self.assertRaisesRegex(ValueError, "superseded"):
            self.select(event)
        self.pr["head"]["sha"] = HEAD
        self.pr["state"] = "closed"
        with self.assertRaisesRegex(ValueError, "superseded"):
            self.select(event)

    def test_merge_tree_cannot_mix_requirements_from_both_branches(self):
        self.manifest["recipe_revision"] = "e" * 40
        with self.assertRaisesRegex(ValueError, "rebase"):
            self.select()

    def test_missing_or_incomplete_publisher_cannot_select_image(self):
        self.responses["actions/runs/7"]["conclusion"] = None
        with self.assertRaisesRegex(ValueError, "not published"):
            self.select()
        self.responses[f"commits/{HEAD}/status"]["statuses"] = []
        with self.assertRaisesRegex(ValueError, "not published"):
            self.select()

    def test_other_workflow_cannot_supply_candidate_status(self):
        self.responses["actions/runs/7"]["path"] = ".github/workflows/tests.yml"
        with self.assertRaisesRegex(ValueError, "Unexpected candidate"):
            self.select()

    def test_environment_export_rejects_multiline_values_before_writing(self):
        self.manifest["requirements"]["versions"]["protoc"] = "32.0\nINJECTED=yes"
        with self.assertRaisesRegex(ValueError, "newline-free"):
            runner.export_environment(self.manifest, self.output)
        self.assertFalse(self.output.exists())


if __name__ == "__main__":
    unittest.main()
