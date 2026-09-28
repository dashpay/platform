"""Exercise CPU discovery and the real build launcher without compiling source."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'run-npm-build.py'
SPEC = importlib.util.spec_from_file_location('npm_build', SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class NpmBuildBudgetTests(unittest.TestCase):
    def test_should_honor_quota_and_affinity(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for maximum, affinity, expected in [('2000000 100000', 32, 20),
                                                 ('2000000 100000', 8, 8),
                                                 ('max 100000', 6, 6),
                                                 ('150000 100000', 8, 1)]:
                with self.subTest(maximum=maximum, affinity=affinity):
                    (root / 'cpu.max').write_text(maximum)
                    self.assertEqual(MODULE.cpu_budget(root, affinity), expected)

    def test_should_support_legacy_quota_and_no_quota(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.assertEqual(MODULE.cpu_budget(root, 8), 8)
            controller = root / 'cpu,cpuacct'
            controller.mkdir()
            (controller / 'cpu.cfs_period_us').write_text('100000')
            (controller / 'cpu.cfs_quota_us').write_text('400000')
            self.assertEqual(MODULE.cpu_budget(root, 32), 4)
            (controller / 'cpu.cfs_quota_us').write_text('-1')
            self.assertEqual(MODULE.cpu_budget(root, 32), 32)

    def test_should_preserve_separate_compile_and_optimizer_budgets(self):
        original = {'CARGO_BUILD_JOBS': '20', 'BINARYEN_CORES': '4', 'RUSTFLAGS': 'unchanged'}
        self.assertEqual(MODULE.build_environment(original, 20), original)
        self.assertEqual(MODULE.build_environment({'CARGO_BUILD_JOBS': '2'}, 8),
                         {'CARGO_BUILD_JOBS': '2', 'BINARYEN_CORES': '8'})

    def test_should_reject_invalid_or_excessive_thread_count(self):
        for value in ['', '0', '-1', '1.5', 'auto', '21', '4; false']:
            with self.subTest(value=value), self.assertRaises(ValueError):
                MODULE.build_environment({'BINARYEN_CORES': value}, 20)

    def test_should_preserve_command_environment_and_failure_exit_status(self):
        # Use an executable scratch directory: some CI workspaces mount /tmp noexec.
        with tempfile.TemporaryDirectory(dir=Path.cwd()) as directory:
            root = Path(directory)
            yarn = root / 'yarn'
            yarn.write_text('#!/usr/bin/env python3\nimport json, os, sys\n'
                            'print(json.dumps({"args":sys.argv[1:], '
                            '"cores":os.environ["BINARYEN_CORES"], '
                            '"cargo":os.environ["CARGO_BUILD_JOBS"]}))\n'
                            'sys.exit(int(os.environ["BUILD_TEST_EXIT"]))\n')
            yarn.chmod(0o700)
            for code in [0, 23]:
                env = dict(os.environ, PATH=str(root) + ':' + os.environ['PATH'],
                           BINARYEN_CORES='1', CARGO_BUILD_JOBS='2', BUILD_TEST_EXIT=str(code))
                result = subprocess.run(['python3', str(SCRIPT)], env=env,
                                        capture_output=True, text=True)
                self.assertEqual(result.returncode, code, result.stderr)
                payload = json.loads(next(line for line in result.stdout.splitlines()
                                          if line.startswith('{')))
                self.assertEqual(payload, {'args': ['build'], 'cores': '1', 'cargo': '2'})
                self.assertIn('NPM build timing: wall=', result.stderr)


if __name__ == '__main__':
    unittest.main()
