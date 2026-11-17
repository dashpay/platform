"""Keep the JS package lint and test jobs complete and runnable.

A package shares the Node-only test job only while its tests need neither a
browser nor Docker, no package sits in two groups, and every package the paths
filters know must map to a real workspace, or it would silently never be tested.
"""
import importlib.util
import json
import os
from pathlib import Path
import re
import tempfile
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[3]
FILTERS = ROOT / ".github/package-filters"
spec = importlib.util.spec_from_file_location("js_packages", ROOT / ".github/scripts/js-packages.py")
js = importlib.util.module_from_spec(spec)
spec.loader.exec_module(js)


def workspaces():
    """Map each JS workspace name to its parsed package.json."""
    found = {}
    for path in ROOT.glob("packages/*/package.json"):
        manifest = json.loads(path.read_text())
        found[manifest["name"]] = manifest
    return found


def expand(scripts, command, seen=()):
    """Return a yarn script's text with every script it runs through yarn."""
    text = scripts[command]
    nested = [
        expand(scripts, name, seen + (command,))
        for name in re.findall(r"yarn (?:run )?([\w:.-]+)", text)
        if name in scripts and name not in seen + (command,)
    ]
    return " ".join([text, *nested])


class JsPackagesTest(unittest.TestCase):
    def test_should_batch_only_packages_tested_without_a_browser_or_docker(self):
        manifests = workspaces()
        for package in dict(js.GROUPS)[js.NODE_ONLY]:
            with self.subTest(package=package):
                manifest = manifests[package]
                command = expand(manifest["scripts"], js.TEST_COMMANDS.get(package, "test"))
                self.assertNotIn("karma", command)
                self.assertNotIn("docker", command)
                dependencies = {**manifest.get("dependencies", {}), **manifest.get("devDependencies", {})}
                self.assertNotIn("dockerode", dependencies)

    def test_should_map_every_filtered_package_to_a_workspace(self):
        known = js.packages_in_filter()
        self.assertIn("dash", known)
        self.assertIn("@dashevo/dapi", known)
        self.assertEqual(set(known), set(js.packages_in_filter(FILTERS / "js-packages-direct.yml")))
        manifests = workspaces()
        for package in known:
            with self.subTest(package=package):
                self.assertIn(package, manifests)
                command = js.TEST_COMMANDS.get(package, "test")
                self.assertIn(command, manifests[package]["scripts"])
                self.assertIn("lint", manifests[package]["scripts"])
        grouped = [package for _, members in js.GROUPS for package in members]
        self.assertEqual(len(grouped), len(set(grouped)))
        for package in (*grouped, *js.TEST_COMMANDS, *js.NIGHTLY_ONLY):
            self.assertIn(package, known)

    def test_should_test_each_group_in_one_job_and_dashmate_alone(self):
        jobs = js.test_jobs([
            "@dashevo/dpns-contract",
            "@dashevo/wasm-sdk",
            "dash",
            "@dashevo/dapi",
            "dashmate",
            "@dashevo/evo-sdk",
        ])
        self.assertEqual(
            jobs,
            [
                {"name": "dashmate", "packages": [{"package": "dashmate", "command": "test:ci"}]},
                {
                    "name": js.NODE_ONLY,
                    "packages": [
                        {"package": "@dashevo/dpns-contract", "command": "test"},
                        {"package": "@dashevo/dapi", "command": "test"},
                    ],
                },
                {
                    "name": "Evo SDK packages",
                    "packages": [
                        {"package": "@dashevo/wasm-sdk", "command": "test"},
                        {"package": "@dashevo/evo-sdk", "command": "test"},
                    ],
                },
                {"name": "dash", "packages": [{"package": "dash", "command": "test"}]},
            ],
        )

    def test_should_name_a_job_with_one_package_after_it(self):
        jobs = js.test_jobs(["@dashevo/platform-test-suite"])
        self.assertEqual(
            jobs,
            [{
                "name": "@dashevo/platform-test-suite",
                "packages": [{"package": "@dashevo/platform-test-suite", "command": "test:unit"}],
            }],
        )
        self.assertEqual(js.test_jobs([]), [])

    def test_should_test_every_pull_request_package_in_four_jobs(self):
        tested, _ = js.select("pull_request", True, [], [], js.packages_in_filter())
        jobs = js.test_jobs(tested)
        self.assertEqual(len(jobs), 4)
        self.assertEqual(sorted(item["package"] for job in jobs for item in job["packages"]), sorted(tested))

    def test_should_lint_only_tested_packages_that_changed_directly(self):
        self.assertEqual(
            js.select("push", False, ["dash", "@dashevo/dapi"], ["@dashevo/dapi"], js.packages_in_filter()),
            (["dash", "@dashevo/dapi"], ["@dashevo/dapi"]),
        )

    def test_should_keep_nightly_only_packages_off_pull_requests(self):
        changed = ["@dashevo/wasm-dpp", "dash"]
        known = js.packages_in_filter()
        self.assertEqual(js.select("pull_request", False, changed, changed, known), (["dash"], ["dash"]))
        self.assertEqual(js.select("schedule", False, changed, changed, known), (changed, changed))
        tested, linted = js.select("workflow_dispatch", True, [], [], known)
        self.assertEqual(tested, known)
        self.assertEqual(linted, known)

    def test_should_run_every_package_when_a_pull_request_changes_these_jobs(self):
        known = js.packages_in_filter()
        tested, linted = js.select("pull_request", True, [], [], known)
        expected = [package for package in known if package not in js.NIGHTLY_ONLY]
        self.assertEqual(tested, expected)
        self.assertEqual(linted, expected)

    def test_should_write_the_plan_outputs(self):
        env = {
            "EVENT_NAME": "pull_request",
            "CHANGED": '["@dashevo/wasm-dpp","dashmate","@dashevo/dpns-contract"]',
            "CHANGED_DIRECT": '["@dashevo/dpns-contract"]',
            "WORKFLOWS_CHANGED": "false",
        }
        with tempfile.TemporaryDirectory() as directory, mock.patch.dict(os.environ, env), \
                mock.patch("builtins.print"):
            output = Path(directory) / "output"
            js.plan(output)
            values = dict(line.split("=", 1) for line in output.read_text().splitlines())
        self.assertEqual(json.loads(values["packages"]), ["dashmate", "@dashevo/dpns-contract"])
        self.assertEqual(json.loads(values["lint"]), [{"package": "@dashevo/dpns-contract", "command": "lint"}])
        self.assertEqual([job["name"] for job in json.loads(values["test-jobs"])], ["dashmate", "@dashevo/dpns-contract"])

    def test_should_run_every_package_and_fail_after_the_last(self):
        results = {"a": 0, "b": 3, "c": 0}
        calls = []

        def fake_run(argv, stdin):
            calls.append(argv)
            return mock.Mock(returncode=results[argv[2]])

        entries = [js.entry(package, "test") for package in results]
        with mock.patch.object(js.subprocess, "run", fake_run), mock.patch("builtins.print") as printed:
            self.assertEqual(js.run(entries), 1)
        self.assertEqual(calls, [["yarn", "workspace", package, "test"] for package in results])
        self.assertIn(mock.call("Failed:", "b"), printed.call_args_list)
        with mock.patch.object(js.subprocess, "run", fake_run), mock.patch("builtins.print"):
            self.assertEqual(js.run([js.entry("a", "lint")]), 0)
        with self.assertRaises(ValueError):
            js.run([])


if __name__ == "__main__":
    unittest.main()
