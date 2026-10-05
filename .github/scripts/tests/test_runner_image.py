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
from urllib.error import URLError
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location("runner_image", ROOT / ".github/scripts/runner-image.py")
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)
# Minimal projections of public REST responses captured 2026-09-28 for PR 5151.
# /status has Simple Commit Status objects (no creator); /statuses has creator.
FIXTURE = json.loads((Path(__file__).parent / "fixtures/candidate-status-pr5151.json").read_text())
HEAD = FIXTURE["combined"]["sha"]
DIGEST = FIXTURE["statuses"][0]["description"]
STATUS_PATH = f"commits/{HEAD}/statuses?per_page=100&page=1"
RUN_PATH = f"actions/runs/{FIXTURE['publisher_run']['id']}"


class SelectorTests(unittest.TestCase):
    def setUp(self):
        # Historical publisher fixtures bind the legacy contract, not a future branch default.
        self.manifest = runner.read_manifest(Path(__file__).parent / "fixtures/legacy-runner-requirements.json")
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.event = Path(self.temp.name) / "event.json"
        self.output = Path(self.temp.name) / "output"
        self.pr = {"number": 5151, "state": "open", "changed_files": 1,
                   "head": {"sha": HEAD}}
        self.responses = {
            "pulls/5151": self.pr,
            "pulls/5151/files?per_page=100&page=1": [{"filename": runner.MANIFEST}],
            f"contents/{runner.MANIFEST}?ref={HEAD}": {
                "content": base64.b64encode(json.dumps(self.manifest).encode()).decode()},
            f"commits/{HEAD}/status": copy.deepcopy(FIXTURE["combined"]),
            STATUS_PATH: copy.deepcopy(FIXTURE["statuses"]),
            RUN_PATH: copy.deepcopy(FIXTURE["publisher_run"]),
        }

    def api_response(self, path):
        response = self.responses[path]
        if isinstance(response, Exception):
            raise response
        return response

    def status_calls(self):
        return [call.args[0] for call in self.api.call_args_list
                if call.args[0].startswith("commits/")]

    def select(self, event=None, kind="rust", arch=None, validation=False, wait_seconds=0):
        self.event.write_text(json.dumps(event if event is not None else {"pull_request": self.pr}))
        with patch.dict(os.environ, {"GITHUB_EVENT_PATH": str(self.event)}), \
             patch.object(runner, "api", side_effect=self.api_response) as api:
            self.api = api
            runner.select(self.manifest, kind, self.output, wait_seconds, arch, validation)
        return dict(line.split("=", 1) for line in self.output.read_text().splitlines())

    def test_non_pr_and_unchanged_pr_use_existing_pool(self):
        self.assertEqual(json.loads(self.select({})["labels"]), ["self-hosted", "Linux", "rust-ci"])
        self.responses["pulls/5151/files?per_page=100&page=1"] = [{"filename": "Cargo.lock"}]
        self.assertEqual(self.select()["image_changed"], "false")

    def test_exact_candidate_includes_head_digest_and_kind(self):
        self.assertNotIn("creator", FIXTURE["combined"]["statuses"][0])
        for kind in ("kotlin", "rust", "npm"):
            with self.subTest(kind=kind):
                output = self.select(kind=kind)
                self.assertEqual(json.loads(output["labels"]), ["self-hosted", "Linux", "X64",
                                 f"platform-image-pr-5151-{HEAD}-{DIGEST[7:]}-{kind}"])
                self.assertEqual(output["image_changed"], "true")
                self.assertEqual(self.status_calls(), [STATUS_PATH])

    def test_should_keep_npm_ordinary_pool_labels(self):
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
        for conclusion in (None, "failure", "cancelled"):
            with self.subTest(conclusion=conclusion):
                self.responses[RUN_PATH]["conclusion"] = conclusion
                with self.assertRaisesRegex(ValueError, "not published"):
                    self.select()
                self.assertFalse(self.output.exists())
        self.responses[STATUS_PATH] = []
        with self.assertRaisesRegex(ValueError, "not published"):
            self.select()

    def test_other_workflow_cannot_supply_candidate_status(self):
        for field, value in (("path", ".github/workflows/tests.yml"), ("event", "pull_request")):
            with self.subTest(field=field):
                self.responses[RUN_PATH] = dict(FIXTURE["publisher_run"], **{field: value})
                with self.assertRaisesRegex(ValueError, "Unexpected candidate"):
                    self.select()
                self.assertFalse(self.output.exists())

    def test_should_reject_missing_null_malformed_or_wrong_creator_without_fallback(self):
        # The real combined response is also a regression case: no creator.
        missing = FIXTURE["combined"]["statuses"][0]
        good = FIXTURE["statuses"][0]
        candidates = [missing] + [dict(good, creator=creator) for creator in (
            None, {}, "github-actions[bot]", [], 42,
            {"login": None}, {"login": "untrusted-user"},
        )]
        for candidate in candidates:
            with self.subTest(creator=candidate.get("creator", "absent")):
                self.responses[STATUS_PATH] = [candidate, good]
                with self.assertRaisesRegex(ValueError, "trusted publisher"):
                    self.select()
                self.assertEqual(self.status_calls(), [STATUS_PATH])
                self.assertNotIn(RUN_PATH, [call.args[0] for call in self.api.call_args_list])
                self.assertFalse(self.output.exists())

    def test_should_reject_invalid_digest_or_publisher_url_without_fallback(self):
        good = FIXTURE["statuses"][0]
        for field, value, error in (
            ("description", "sha256:abc", "immutable digest"),
            ("description", "latest", "immutable digest"),
            ("target_url", "https://github.com/other/platform/actions/runs/7", "publishing workflow"),
            ("target_url", good["target_url"] + "/jobs/1", "publishing workflow"),
        ):
            with self.subTest(field=field, value=value):
                self.responses[STATUS_PATH] = [dict(good, **{field: value}), good]
                with self.assertRaisesRegex(ValueError, error):
                    self.select()
                self.assertFalse(self.output.exists())

    def test_should_block_older_success_when_newest_is_pending_failure_or_error(self):
        good = FIXTURE["statuses"][0]
        for state in ("pending", "failure", "error"):
            with self.subTest(state=state):
                self.responses[STATUS_PATH] = [dict(good, state=state), good]
                with self.assertRaisesRegex(ValueError, "not published"):
                    self.select()
                self.assertEqual(self.status_calls(), [STATUS_PATH])
                self.assertNotIn(RUN_PATH, [call.args[0] for call in self.api.call_args_list])
                self.assertFalse(self.output.exists())

    def test_should_use_first_success_without_unnecessary_pagination(self):
        good = FIXTURE["statuses"][0]
        older = dict(good, description="sha256:" + "e" * 64)
        self.responses[STATUS_PATH] = [good] + [older] * 99
        labels = json.loads(self.select()["labels"])
        self.assertEqual(labels[-1], f"platform-image-pr-5151-{HEAD}-{DIGEST[7:]}-rust")
        self.assertEqual(self.status_calls(), [STATUS_PATH])

    def test_should_shadow_canonical_success_with_newer_case_variant(self):
        good = FIXTURE["statuses"][0]
        for state in ("success", "pending", "failure", "error"):
            with self.subTest(state=state):
                newer = dict(good, context=good["context"].lower(), state=state)
                self.responses[STATUS_PATH] = [newer] + [good] * 99
                error = "exact publisher context" if state == "success" else "not published"
                with self.assertRaisesRegex(ValueError, error):
                    self.select()
                self.assertEqual(self.status_calls(), [STATUS_PATH])
                self.assertNotIn(RUN_PATH, [call.args[0] for call in self.api.call_args_list])
                self.assertFalse(self.output.exists())

    def test_should_select_first_match_on_later_page_before_validation(self):
        good = FIXTURE["statuses"][0]
        self.responses[STATUS_PATH] = [dict(good, context="unrelated")] * 100
        page2 = STATUS_PATH.replace("&page=1", "&page=2")
        for state in ("pending", "success"):
            with self.subTest(state=state):
                self.responses[page2] = [dict(good, state=state)] + [good] * 99
                if state == "pending":
                    with self.assertRaisesRegex(ValueError, "not published"):
                        self.select()
                    self.assertFalse(self.output.exists())
                else:
                    self.assertEqual(self.select()["image_changed"], "true")
                self.assertEqual(self.status_calls(), [STATUS_PATH, page2])

    def test_should_require_exact_context_and_stop_at_page_exhaustion(self):
        good = FIXTURE["statuses"][0]
        unrelated = [dict(good, context=context) for context in (
            "Runner image candidate / PR 51510", "Runner image candidate / PR 5151 suffix",
            "Runner image candidate / PR 5151 ", "Runner image candidate / PR 515",
        )]
        page2 = STATUS_PATH.replace("&page=1", "&page=2")
        for last_page in ([], unrelated):
            with self.subTest(last_page_size=len(last_page)):
                self.responses[STATUS_PATH] = unrelated * 25
                self.responses[page2] = last_page
                with self.assertRaisesRegex(ValueError, "not published"):
                    self.select()
                self.assertEqual(self.status_calls(), [STATUS_PATH, page2])
                self.assertFalse(self.output.exists())

    def test_should_fail_closed_on_status_api_failure(self):
        page2 = STATUS_PATH.replace("&page=1", "&page=2")
        for failed_path in (STATUS_PATH, page2):
            with self.subTest(failed_path=failed_path):
                self.responses[STATUS_PATH] = [dict(FIXTURE["statuses"][0], context="other")] * 100
                self.responses[failed_path] = URLError("status API unavailable")
                with self.assertRaisesRegex(URLError, "status API unavailable"):
                    self.select()
                self.assertFalse(self.output.exists())

    def test_should_retry_pending_status_and_publisher_run(self):
        good = FIXTURE["statuses"][0]
        for pending in ("status", "run"):
            with self.subTest(pending=pending):
                self.responses[STATUS_PATH] = [dict(good, state="pending"), good] if pending == "status" else [good]
                self.responses[RUN_PATH]["conclusion"] = None if pending == "run" else "success"

                def publish(_seconds):
                    self.responses[STATUS_PATH] = [good]
                    self.responses[RUN_PATH]["conclusion"] = "success"

                with patch.object(runner.time, "monotonic", return_value=0), \
                     patch.object(runner.time, "sleep", side_effect=publish) as sleep:
                    self.assertEqual(self.select(wait_seconds=60)["image_changed"], "true")
                sleep.assert_called_once_with(20)
                self.assertEqual(self.status_calls(), [STATUS_PATH, STATUS_PATH])
                self.assertEqual([call.args[0] for call in self.api.call_args_list].count("pulls/5151"), 3)

    def test_should_reject_head_change_or_closure_during_candidate_retry(self):
        good = FIXTURE["statuses"][0]
        for kind in ("rust", "kotlin", "npm"):
            for change in ("head", "closed"):
                with self.subTest(kind=kind, change=change):
                    self.pr["state"] = "open"
                    self.pr["head"]["sha"] = HEAD
                    event = {"pull_request": copy.deepcopy(self.pr)}
                    self.responses[STATUS_PATH] = [dict(good, state="pending")]

                    def publish_after_pr_changes(_seconds):
                        self.responses[STATUS_PATH] = [good]
                        if change == "head":
                            self.pr["head"]["sha"] = "e" * 40
                        else:
                            self.pr["state"] = "closed"

                    with patch.object(runner.time, "monotonic", return_value=0), \
                         patch.object(runner.time, "sleep", side_effect=publish_after_pr_changes):
                        with self.assertRaisesRegex(ValueError, "PR changed before selecting"):
                            self.select(event, kind=kind, wait_seconds=60)
                    self.assertFalse(self.output.exists())

    def test_should_fail_closed_if_final_candidate_head_check_is_unavailable(self):
        original = self.api_response

        def fail_after_publisher_validation(path):
            response = original(path)
            if path == RUN_PATH:
                self.responses["pulls/5151"] = URLError("final PR check unavailable")
            return response

        with patch.object(self, "api_response", side_effect=fail_after_publisher_validation):
            with self.assertRaisesRegex(URLError, "final PR check unavailable"):
                self.select()
        self.assertFalse(self.output.exists())

    def test_environment_export_rejects_multiline_values_before_writing(self):
        self.manifest["requirements"]["versions"]["protoc"] = "32.0\nINJECTED=yes"
        with self.assertRaisesRegex(ValueError, "newline-free"):
            runner.export_environment(self.manifest, self.output)
        self.assertFalse(self.output.exists())

    def test_arm64_validation_never_consumes_amd64_candidate_status(self):
        self.responses[STATUS_PATH] = []
        output = self.select(arch="ARM64")
        self.assertEqual(json.loads(output["labels"]), ["self-hosted", "Linux", "ARM64", "rust-ci"])

    def test_arm64_manifest_change_does_not_use_stale_ordinary_arm64_runners(self):
        self.responses["pulls/5151/files?per_page=100&page=1"] = [{"filename": runner.ARM64_MANIFEST}]
        output = self.select()
        self.assertEqual(json.loads(output["labels"]), ["self-hosted", "Linux", "X64", "rust-ci"])
        self.assertEqual(output["image_changed"], "false")
        # Mac-backed ARM64 capacity remains available to ordinary, unrelated PRs.
        self.responses["pulls/5151/files?per_page=100&page=1"] = [{"filename": "Cargo.lock"}]
        self.assertEqual(json.loads(self.select()["labels"]), ["self-hosted", "Linux", "rust-ci"])

    def test_both_manifest_changes_still_require_exact_amd64_candidate(self):
        self.responses["pulls/5151/files?per_page=100&page=1"] = [
            {"filename": runner.MANIFEST}, {"filename": runner.ARM64_MANIFEST}]
        output = self.select()
        self.assertEqual(json.loads(output["labels"]), ["self-hosted", "Linux", "X64",
                         f"platform-image-pr-5151-{HEAD}-{DIGEST[7:]}-rust"])
        self.assertEqual(output["image_changed"], "true")

    def test_arm64_validation_rejects_merge_tree_drift(self):
        arm = runner.read_manifest(ROOT / runner.ARM64_MANIFEST)
        self.responses["pulls/5151/files?per_page=100&page=1"] = [{"filename": runner.ARM64_MANIFEST}]
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
        # This checks current contracts; historical publisher tests keep the frozen fixture.
        amd = runner.read_manifest(ROOT / runner.MANIFEST)["requirements"]
        arm = runner.read_manifest(ROOT / runner.ARM64_MANIFEST)["requirements"]
        self.assertEqual(arm["versions"], {k: v for k, v in amd["versions"].items() if k != "cargo_ndk"})
        for key in ("rust_version", "rust_manifest_sha256", "apt_snapshot", "java_major", "client_codegen"):
            self.assertEqual(amd[key], arm[key], key)

    def test_should_detect_live_amd64_drift_despite_frozen_publisher_fixture(self):
        manifests = {ROOT / name: runner.read_manifest(ROOT / name)
                     for name in (runner.MANIFEST, runner.ARM64_MANIFEST)}
        manifests[ROOT / runner.MANIFEST]["requirements"]["java_major"] += 1
        with patch.object(runner, "read_manifest", side_effect=manifests.__getitem__):
            with self.assertRaises(AssertionError):
                self.test_shared_rust_toolchain_does_not_drift_between_architectures()

    def test_should_allow_matching_live_updates_despite_frozen_publisher_fixture(self):
        manifests = {ROOT / name: runner.read_manifest(ROOT / name)
                     for name in (runner.MANIFEST, runner.ARM64_MANIFEST)}
        for manifest in manifests.values():
            manifest["requirements"]["java_major"] += 1
        with patch.object(runner, "read_manifest", side_effect=manifests.__getitem__):
            self.test_shared_rust_toolchain_does_not_drift_between_architectures()

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
