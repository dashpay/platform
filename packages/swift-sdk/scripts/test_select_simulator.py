#!/usr/bin/env python3
"""Selector and fully mocked entrypoint regressions; no Apple tools are invoked."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

from select_simulator import select_simulator


SCRIPTS = Path(__file__).resolve().parent
FIXTURE = SCRIPTS / "fixtures/simulators-ios-18-6-and-27.json"
OLD_RUNTIME = "com.apple.CoreSimulator.SimRuntime.iOS-18-6"
NEW_RUNTIME = "com.apple.CoreSimulator.SimRuntime.iOS-27-0"
OLD_ID = "CDD3885F-8364-4DFB-AF77-AC5426856EFC"
NEW_ID = "9F88E007-4C07-4D61-A6D3-D19E3747A0F8"
OTHER_ID = "00000000-0000-4000-8000-000000000001"


def inventory():
    return json.loads(FIXTURE.read_text())


def device(name="iPhone 16 Pro", udid=OLD_ID, available=True):
    return {"name": name, "udid": udid, "isAvailable": available}


class SimulatorSelectionTests(unittest.TestCase):
    def test_should_resolve_old_runtime_name_without_latest_os_assumption(self):
        data = inventory()
        self.assertEqual(select_simulator(data, name="iPhone 16 Pro"), OLD_ID)
        self.assertNotIn("iPhone 16 Pro", [d["name"] for d in data["devices"][NEW_RUNTIME]])

    def test_should_choose_newest_runtime_then_name_independent_of_input_order(self):
        data = inventory()
        self.assertEqual(select_simulator(data), NEW_ID)
        data["devices"] = {key: list(reversed(value))
                           for key, value in reversed(list(data["devices"].items()))}
        self.assertEqual(select_simulator(data), NEW_ID)

    def test_should_compare_runtime_versions_numerically(self):
        data = {"devices": {
            "com.apple.CoreSimulator.SimRuntime.iOS-9-3": [device()],
            "com.apple.CoreSimulator.SimRuntime.iOS-18-10": [device(udid=NEW_ID)],
            "com.apple.CoreSimulator.SimRuntime.iOS-18-9": [device(udid=OTHER_ID)],
        }}
        self.assertEqual(select_simulator(data), NEW_ID)

    def test_should_resolve_duplicate_names_by_runtime_then_udid(self):
        data = {"devices": {
            OLD_RUNTIME: [device()],
            NEW_RUNTIME: [device(udid=NEW_ID), device(udid=OTHER_ID)],
        }}
        for reverse in (False, True):
            with self.subTest(reverse=reverse):
                if reverse:
                    data["devices"][NEW_RUNTIME].reverse()
                self.assertEqual(select_simulator(data, name="iPhone 16 Pro"), OTHER_ID)
                self.assertEqual(select_simulator(data, udid=OLD_ID), OLD_ID)

    def test_should_validate_udid_and_require_both_overrides_to_match(self):
        self.assertEqual(select_simulator(inventory(), udid=OLD_ID.lower()), OLD_ID)
        self.assertEqual(select_simulator(inventory(), name="iPhone 16 Pro", udid=OLD_ID), OLD_ID)
        for name, udid in (("iPhone 17", OLD_ID), ("", OTHER_ID),
                           ("", "not-a-uuid"), ("", OLD_ID + ",OS=latest")):
            with self.subTest(name=name, udid=udid), self.assertRaises(ValueError):
                select_simulator(inventory(), name=name, udid=udid)

    def test_should_require_exact_name_without_fallback(self):
        for name in ("iPhone 16 Pro ", "iphone 16 pro", "iPhone Missing", "iPhone 16 P"):
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "No available"):
                select_simulator(inventory(), name=name)
        # Explicit names may select non-iPhones or custom-named iOS simulators.
        self.assertEqual(select_simulator(inventory(), name="CI iPhone 16"),
                         "DFF66162-1A94-4322-864E-C399D11B89D0")

    def test_should_reject_unavailable_overrides_and_skip_unavailable_defaults(self):
        data = {"devices": {OLD_RUNTIME: [device()], NEW_RUNTIME: [device(udid=NEW_ID, available=False)]}}
        self.assertEqual(select_simulator(data), OLD_ID)
        with self.assertRaisesRegex(ValueError, "No available"):
            select_simulator(data, udid=NEW_ID)
        data["devices"][OLD_RUNTIME][0]["isAvailable"] = False
        for name in ("", "iPhone 16 Pro"):
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "No available"):
                select_simulator(data, name=name)

    def test_should_reject_missing_iphone_and_non_ios_inventory(self):
        for data in ({"devices": {}}, {"devices": {OLD_RUNTIME: [device(name="iPad Test")]}},
                     {"devices": {"com.apple.CoreSimulator.SimRuntime.tvOS-27-0": [device()]}}):
            with self.subTest(data=data), self.assertRaisesRegex(ValueError, "No available"):
                select_simulator(data)

    def test_should_fail_closed_on_malformed_schema_or_duplicate_ids(self):
        cases = [None, [], {}, {"devices": []}, {"devices": {OLD_RUNTIME: {}}}]
        for changes in ({"udid": ""}, {"udid": "broken"}, {"udid": None},
                        {"name": ""}, {"name": 42}, {"isAvailable": "true"},
                        {"isAvailable": 1}, {"isAvailable": None}):
            cases.append({"devices": {OLD_RUNTIME: [dict(device(), **changes)]}})
        missing_availability = device()
        del missing_availability["isAvailable"]
        cases.extend([
            {"devices": {OLD_RUNTIME: [missing_availability]}},
            {"devices": {OLD_RUNTIME: [None]}},
            {"devices": {OLD_RUNTIME: [device()], NEW_RUNTIME: [device()]}},
        ])
        for data in cases:
            with self.subTest(data=data), self.assertRaises(ValueError):
                select_simulator(data)


FAKE_COMMAND = r'''#!/usr/bin/env python3
import json
import os
from pathlib import Path
import sys

command = Path(sys.argv[0]).name
args = sys.argv[1:]
with open(os.environ["SIM_TEST_CALLS"], "a") as log:
    log.write(json.dumps([command, *args]) + "\n")
if command == "xcrun":
    assert args == ["simctl", "list", "devices", "available", "--json"]
    print(Path(os.environ["SIM_TEST_INVENTORY"]).read_text())
    sys.exit(int(os.environ.get("SIM_TEST_XCRUN_STATUS", "0")))
assert command in ("build-step", "swift", "xcodebuild"), command
'''


class SimulatorEntrypointTests(unittest.TestCase):
    def run_entrypoint(self, data=None, *, raw=None, name="", udid="", xcrun_status=0):
        with tempfile.TemporaryDirectory(prefix="swift simulator test ") as temp:
            root = Path(temp)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            (root / "scripts").mkdir()
            shutil.copyfile(SCRIPTS.parent / "run_tests.sh", root / "run_tests.sh")
            shutil.copyfile(SCRIPTS / "select_simulator.py", root / "scripts/select_simulator.py")
            (root / "build_ios.sh").write_text('exec build-step "$@"\n')
            # Stub every macOS/build command; accidental keychain use fails closed.
            for name_of_command in ("xcrun", "build-step", "swift", "xcodebuild", "security"):
                command = bin_dir / name_of_command
                command.write_text(FAKE_COMMAND)
                command.chmod(0o755)
            fixture = root / "inventory.json"
            fixture.write_text(raw if raw is not None else json.dumps(inventory() if data is None else data))
            calls_path = root / "calls.jsonl"
            calls_path.touch()
            env = dict(os.environ)
            env.pop("CI", None)
            env.pop("GITHUB_ACTIONS", None)
            env.update({
                "PATH": str(bin_dir) + os.pathsep + env["PATH"],
                "SIM_NAME": name, "SIM_UDID": udid,
                "SIM_TEST_INVENTORY": str(fixture), "SIM_TEST_CALLS": str(calls_path),
                "SIM_TEST_XCRUN_STATUS": str(xcrun_status),
            })
            result = subprocess.run(["bash", str(root / "run_tests.sh")],
                                    env=env, capture_output=True, text=True, timeout=30)
            return result, [json.loads(line) for line in calls_path.read_text().splitlines()]

    def test_should_pass_exact_id_destination_for_default_name_and_udid(self):
        for overrides, selected in (({}, NEW_ID), ({"name": "iPhone 16 Pro"}, OLD_ID),
                                     ({"udid": OLD_ID.lower()}, OLD_ID),
                                     ({"name": "iPhone 16 Pro", "udid": OLD_ID}, OLD_ID)):
            with self.subTest(overrides=overrides):
                result, calls = self.run_entrypoint(**overrides)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(calls, [
                    ["xcrun", "simctl", "list", "devices", "available", "--json"],
                    ["build-step", "--target", "tests", "--profile", "dev"],
                    ["swift", "test"],
                    ["xcodebuild", "test", "-project", "SwiftExampleApp/SwiftExampleApp.xcodeproj",
                     "-scheme", "SwiftExampleApp", "-skip-testing:SwiftExampleAppUITests",
                     "-destination", "platform=iOS Simulator,id=" + selected],
                ])

    def test_should_stop_before_builds_on_bad_inventory_overrides_or_simctl_failure(self):
        unavailable = {"devices": {OLD_RUNTIME: [device(available=False)]}}
        cases = ({"raw": "not JSON"}, {"raw": ""}, {"raw": "null"},
                 {"data": {"devices": {}}}, {"name": "iPhone Missing"},
                 {"udid": "invalid"}, {"name": "iPhone 17", "udid": OLD_ID},
                 {"data": unavailable, "udid": OLD_ID}, {"xcrun_status": 37})
        for options in cases:
            with self.subTest(options=options):
                result, calls = self.run_entrypoint(**options)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(calls, [["xcrun", "simctl", "list", "devices", "available", "--json"]])
                if not options.get("xcrun_status"):
                    self.assertIn("Cannot select simulator:", result.stderr)


if __name__ == "__main__":
    unittest.main()
