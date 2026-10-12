#!/usr/bin/env python3
"""Exercise run_tests.sh's keychain lifecycle without touching macOS keychains.

Run with python3 -m unittest discover -s packages/swift-sdk/scripts \
    -p test_run_tests_keychain.py -v

The real entrypoint runs against stateful command doubles, including creation's
preference side effects. Native CI must still prove Security.framework access
and run the unchanged Swift and simulator suites.
"""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "run_tests.sh"

FAKE_COMMAND = r'''#!/usr/bin/env python3
import json
import os
from pathlib import Path
import sys

path = Path(os.environ["KEYCHAIN_TEST_STATE"])
state = json.loads(path.read_text())
command = Path(sys.argv[0]).name
args = sys.argv[1:]
operation = args[0] if command == "security" else command
if operation in ("default-keychain", "list-keychains"):
    operation += "-set" if "-s" in args else "-query"
    if "-s" in args and (len(args) == 4 or state["created"] not in args[4:]):
        operation += "-restore"
state["calls"].append(operation)
status = 0
output = ""
error = ""

if operation == "default-keychain-query":
    if state["default"] is None:
        error = "security: SecKeychainCopyDomainDefault user: A default keychain could not be found."
        status = 1
    else:
        output = '    "' + state["default"] + '"'
elif operation == "list-keychains-query":
    output = "\n".join('    "' + item + '"' for item in state["search"])
elif operation == "create-keychain":
    state["created"] = args[-1]
    Path(args[-1]).touch()
    # SecKeychainCreate can change preferences before later setup commands.
    if state["default"] is None:
        state["default"] = args[-1]
    state["search"].append(args[-1])
elif operation.startswith("default-keychain-set"):
    assert args[1:4] == ["-d", "user", "-s"]
    assert len(args) == 4 or (len(args) == 5 and args[4])
    state["default"] = args[4] if len(args) == 5 else None
elif operation.startswith("list-keychains-set"):
    assert args[1:4] == ["-d", "user", "-s"]
    assert all(args[4:])  # An empty argument is not an empty search list.
    state["search"] = args[4:]
elif operation == "delete-keychain":
    Path(args[-1]).unlink()
elif operation == "add-generic-password":
    state["smoke"] = args[args.index("-w") + 1]
elif operation == "find-generic-password":
    output = "wrong value" if state.get("bad_smoke") else state["smoke"]
elif operation == "delete-generic-password":
    state["smoke"] = None
elif operation == "xcrun":
    assert args == ["simctl", "list", "devices", "available", "--json"]
    output = json.dumps({"devices": {"com.apple.CoreSimulator.SimRuntime.iOS-18-6": [
        {"name": "iPhone Test", "udid": "CDD3885F-8364-4DFB-AF77-AC5426856EFC",
         "isAvailable": True},
    ]}})
elif operation in ("build-step", "swift", "xcodebuild"):
    state["build_commands"].append([command, *args])
    if os.environ.get("CI"):
        assert state["default"] == state["created"]
        assert state["search"][0] == state["created"]
        assert state["smoke"] is None
        assert "delete-generic-password" in state["calls"]
else:
    assert operation in ("unlock-keychain", "set-keychain-settings"), operation

# Inject failure after a partial mutation as well as on read-only operations.
if operation in state.get("fail", []):
    status = 37
    error = "injected failure: " + operation
    output = ""
path.write_text(json.dumps(state))
if output:
    print(output)
if error:
    print(error, file=sys.stderr)
sys.exit(status)
'''


class KeychainLifecycleTests(unittest.TestCase):
    def run_entrypoint(self, default=None, search=(), *, ci=True, **options):
        with tempfile.TemporaryDirectory(prefix="swift keychain test ") as temp:
            root = Path(temp)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            runner_temp = root / "runner temp"
            runner_temp.mkdir()
            shutil.copyfile(SCRIPT, root / "run_tests.sh")
            (root / "scripts").mkdir()
            shutil.copyfile(SCRIPT.parent / "scripts/select_simulator.py",
                            root / "scripts/select_simulator.py")
            (root / "build_ios.sh").write_text('exec build-step "$@"\n')
            for name in ("security", "build-step", "swift", "xcodebuild", "xcrun"):
                command = bin_dir / name
                command.write_text(FAKE_COMMAND)
                command.chmod(0o755)
            state_path = root / "state.json"
            state_path.write_text(json.dumps({
                "default": default, "search": list(search), "created": None,
                "calls": [], "build_commands": [], "smoke": None, **options,
            }))
            env = dict(os.environ)
            env.pop("CI", None)
            env.pop("GITHUB_ACTIONS", None)
            env.pop("SIM_UDID", None)
            if ci:
                env["CI"] = "true"
            env.update({
                "PATH": str(bin_dir) + os.pathsep + env["PATH"],
                "KEYCHAIN_TEST_STATE": str(state_path),
                "RUNNER_TEMP": str(runner_temp),
                "SIM_NAME": "iPhone Test",
            })
            result = subprocess.run(
                ["bash", str(root / "run_tests.sh")], env=env,
                capture_output=True, text=True, timeout=30,
            )
            state = json.loads(state_path.read_text())
            self.assertEqual(list(runner_temp.iterdir()), [], result.stderr)
            return result, state

    def assert_restored(self, state, default=None, search=()):
        self.assertEqual(state["default"], default)
        self.assertEqual(state["search"], list(search))

    def test_should_run_all_suites_and_restore_empty_or_existing_preferences(self):
        login = "/Users/CI Runner/Library/Keychains/login.keychain-db"
        extra = "/Volumes/Runner Data/extra.keychain-db"
        for default, search in ((None, ()), (None, (extra,)),
                                (login, (login, extra)), (login, ())):
            with self.subTest(default=default, search=search):
                result, state = self.run_entrypoint(default, search)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assert_restored(state, default, search)
                self.assertEqual(state["build_commands"], [
                    ["build-step", "--target", "tests", "--profile", "dev"],
                    ["swift", "test"],
                    ["xcodebuild", "test", "-project",
                     "SwiftExampleApp/SwiftExampleApp.xcodeproj", "-scheme",
                     "SwiftExampleApp", "-skip-testing:SwiftExampleAppUITests",
                     "-destination", "platform=iOS Simulator,id=CDD3885F-8364-4DFB-AF77-AC5426856EFC"],
                ])

    def test_should_abort_on_inspection_errors_without_mutating_preferences(self):
        for operation in ("default-keychain-query", "list-keychains-query"):
            with self.subTest(operation=operation):
                result, state = self.run_entrypoint(fail=[operation])
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("injected failure: " + operation, result.stderr)
                self.assertIsNone(state["created"])
                self.assert_restored(state)
                self.assertEqual(state["build_commands"], [])

    def test_should_restore_preferences_after_partial_setup_or_smoke_failure(self):
        for operation in ("create-keychain", "unlock-keychain", "set-keychain-settings",
                          "list-keychains-set", "default-keychain-set",
                          "add-generic-password", "find-generic-password",
                          "delete-generic-password"):
            with self.subTest(operation=operation):
                result, state = self.run_entrypoint(fail=[operation])
                self.assertEqual(result.returncode, 37, result.stderr)
                self.assert_restored(state)
                self.assertEqual(state["build_commands"], [])
                self.assertEqual(state["calls"][-1], "delete-keychain")

    def test_should_fail_on_smoke_readback_mismatch_and_restore_preferences(self):
        result, state = self.run_entrypoint(bad_smoke=True)
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("read-back did not match", result.stderr)
        self.assertEqual(state["build_commands"], [])
        self.assert_restored(state)

    def test_should_preserve_build_and_test_failures_while_cleaning_up(self):
        for index, operation in enumerate(("build-step", "swift", "xcodebuild"), 1):
            with self.subTest(operation=operation):
                result, state = self.run_entrypoint(fail=[operation])
                self.assertEqual(result.returncode, 37, result.stderr)
                self.assertEqual(len(state["build_commands"]), index)
                self.assert_restored(state)

    def test_should_report_cleanup_failure_without_masking_a_test_failure(self):
        for fail, status in ((["default-keychain-set-restore"], 1),
                             (["swift", "default-keychain-set-restore"], 37)):
            with self.subTest(fail=fail):
                result, state = self.run_entrypoint(fail=fail)
                self.assertEqual(result.returncode, status, result.stderr)
                self.assertEqual(state["calls"][-2:], [
                    "list-keychains-set-restore", "delete-keychain",
                ])

    def test_should_leave_developer_keychains_untouched_outside_ci(self):
        result, state = self.run_entrypoint(ci=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(state["calls"], ["xcrun", "build-step", "swift", "xcodebuild"])
        self.assert_restored(state)


if __name__ == "__main__":
    unittest.main()
