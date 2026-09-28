"""Exercise runner routing, stale candidates and safe environment exports."""
import base64
import copy
import importlib.util
import json
import os
import sys
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

    def select(self, event=None, kind="rust", arch=None, validation=False):
        self.event.write_text(json.dumps(event if event is not None else {"pull_request": self.pr}))
        with patch.dict(os.environ, {"GITHUB_EVENT_PATH": str(self.event)}), \
             patch.object(runner, "api", side_effect=lambda path: self.responses[path]):
            runner.select(self.manifest, kind, self.output, 0, arch, validation)
        return dict(line.split("=", 1) for line in self.output.read_text().splitlines())

    def test_non_pr_and_unchanged_pr_use_existing_pool(self):
        self.assertEqual(json.loads(self.select({})["labels"]), ["self-hosted", "Linux", "rust-ci"])
        self.responses["pulls/4702/files?per_page=100&page=1"] = [{"filename": "Cargo.lock"}]
        self.assertEqual(self.select()["image_changed"], "false")

    def test_exact_candidate_includes_head_digest_and_kind(self):
        output = self.select()
        labels = json.loads(output["labels"])
        self.assertEqual(labels[-1], f"platform-image-pr-4702-{HEAD}-{DIGEST[7:]}-rust")
        self.assertEqual(output["image_changed"], "true")

    def test_npm_candidates_and_ordinary_pool_have_distinct_labels(self):
        labels = json.loads(self.select(kind="npm")["labels"])
        self.assertEqual(labels[-1], f"platform-image-pr-4702-{HEAD}-{DIGEST[7:]}-npm")
        self.assertEqual(json.loads(self.select({}, kind="npm")["labels"]), ["self-hosted", "npm-pr"])

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

    def test_arm64_validation_never_consumes_amd64_candidate_status(self):
        self.responses[f"commits/{HEAD}/status"]["statuses"] = []
        output = self.select(arch="ARM64")
        self.assertEqual(json.loads(output["labels"]), ["self-hosted", "Linux", "ARM64", "rust-ci"])

    def test_arm64_validation_rejects_merge_tree_drift(self):
        arm = runner.read_manifest(ROOT / runner.ARM64_MANIFEST)
        self.responses["pulls/4702/files?per_page=100&page=1"] = [{"filename": runner.ARM64_MANIFEST}]
        remote = copy.deepcopy(arm)
        remote["recipe_revision"] = "e" * 40
        self.responses[f"contents/{runner.ARM64_MANIFEST}?ref={HEAD}"] = {
            "content": base64.b64encode(json.dumps(remote).encode()).decode()}
        with patch.object(runner, "read_manifest", return_value=arm):
            with self.assertRaisesRegex(ValueError, "rebase"):
                self.select(arch="ARM64")
        self.responses[f"contents/{runner.ARM64_MANIFEST}?ref={HEAD}"] = {
            "content": base64.b64encode(json.dumps(arm).encode()).decode()}
        with patch.object(runner, "read_manifest", return_value=arm):
            self.assertEqual(self.select(arch="ARM64")["image_changed"], "true")

    def test_arm64_cannot_be_requested_for_android(self):
        with self.assertRaisesRegex(ValueError, "only supported for Rust"):
                self.select(kind="kotlin", arch="ARM64")

    def test_arm64_validation_pool_is_not_ordinary_capacity(self):
        labels = json.loads(self.select({}, arch="ARM64", validation=True)["labels"])
        self.assertEqual(labels, ["self-hosted", "Linux", "ARM64", "rust-ci-validation"])
        self.assertNotIn("rust-ci", labels)
        with self.assertRaisesRegex(ValueError, "validation-only pool"):
            self.select({}, validation=True)

    def test_runtime_manifest_keeps_native_macos_and_amd64_separate(self):
        for os_name, arch, expected in [
            ("Linux", "ARM64", runner.ARM64_MANIFEST),
            ("Linux", "X64", runner.MANIFEST),
            ("macOS", "ARM64", runner.MANIFEST),
        ]:
            with self.subTest(os=os_name, arch=arch), \
                 patch.dict(os.environ, {"RUNNER_OS": os_name, "RUNNER_ARCH": arch}):
                self.assertEqual(runner.runtime_manifest(), expected)

    def test_arm64_verify_uses_exact_contract_without_android_exports(self):
        with patch.dict(os.environ, {"RUNNER_OS": "Linux", "RUNNER_ARCH": "ARM64",
                                     "GITHUB_ENV": str(self.output)}), \
             patch.object(sys, "argv", ["runner-image.py", "verify"]), \
             patch.object(runner.subprocess, "run") as verify:
            runner.main()
        verify.assert_called_once_with(["ci-image-contract", "verify", runner.ARM64_MANIFEST], check=True)
        values = dict(line.split("=", 1) for line in self.output.read_text().splitlines())
        self.assertEqual(values["CI_CARGO_NEXTEST_VERSION"], "0.9.144")
        self.assertFalse(any("ANDROID" in name or "NDK" in name for name in values))

    def test_shared_rust_toolchain_does_not_drift_between_architectures(self):
        amd = self.manifest["requirements"]
        arm = runner.read_manifest(ROOT / runner.ARM64_MANIFEST)["requirements"]
        self.assertEqual(arm["versions"], {k: v for k, v in amd["versions"].items() if k != "cargo_ndk"})
        for key in ("rust_version", "rust_manifest_sha256", "apt_snapshot", "java_major", "client_codegen"):
            self.assertEqual(amd[key], arm[key], key)

    def test_rejected_image_contract_stops_before_environment_export(self):
        with patch.dict(os.environ, {"RUNNER_OS": "Linux", "RUNNER_ARCH": "ARM64",
                                     "GITHUB_ENV": str(self.output)}), \
             patch.object(sys, "argv", ["runner-image.py", "verify"]), \
             patch.object(runner.subprocess, "run", side_effect=runner.subprocess.CalledProcessError(1, "ci-image-contract")):
            with self.assertRaises(runner.subprocess.CalledProcessError):
                runner.main()
        self.assertFalse(self.output.exists())


if __name__ == "__main__":
    unittest.main()
