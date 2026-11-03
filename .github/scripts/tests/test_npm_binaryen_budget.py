"""Execute only the CI build wrapper with a credential-free, offline yarn stub."""
import os
from pathlib import Path
import subprocess
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[3]
ACTION = ROOT / '.github/actions/npm-release-build/action.yaml'


def build_script():
    step = ACTION.read_text().split('    - name: Build packages\n', 1)[1]
    step = step.split('\n    - name:', 1)[0]
    run = step.split('      run: ', 1)[1]
    if not run.startswith('|\n'):
        return run.splitlines()[0]
    lines = []
    for line in run.splitlines()[1:]:
        if line.strip() and not line.startswith('        '):
            break
        lines.append(line)
    return textwrap.dedent('\n'.join(lines))


class BinaryenBudgetTests(unittest.TestCase):
    def run_build(self, budget=None, cargo_jobs='20', exit_code=0):
        env = {key: value for key, value in os.environ.items()
               if key not in ('BINARYEN_CORES', 'BASH_ENV', 'ENV')
               and not key.startswith('BASH_FUNC_')}
        env.update(CARGO_BUILD_JOBS=cargo_jobs, CARGO_BUILD_PROFILE='release',
                   MOCK_YARN_EXIT=str(exit_code))
        if budget is not None:
            env['BINARYEN_CORES'] = budget
        stub = '''yarn() {
          printf 'args=%s\n' "$*"
          printf 'budget=%s\n' "${BINARYEN_CORES-unset}"
          printf 'profile=%s\n' "$CARGO_BUILD_PROFILE"
          return "$MOCK_YARN_EXIT"
        }
        '''
        return subprocess.run(['bash', '-c', stub + build_script()], env=env,
                              capture_output=True, text=True, timeout=5)

    def test_should_default_unconfigured_ordinary_builds_to_two_cores(self):
        result = self.run_build()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('budget=2\n', result.stdout)

    def test_should_default_empty_budgets_to_two_cores(self):
        result = self.run_build(budget='')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('budget=2\n', result.stdout)

    def test_should_preserve_explicit_disposable_runner_budgets(self):
        for budget in ['1', '2', '4', '20']:
            with self.subTest(budget=budget):
                result = self.run_build(budget=budget)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn('budget=' + budget + '\n', result.stdout)

    def test_should_not_derive_each_optimizer_pool_from_cargo_parallelism(self):
        for cargo_jobs in ['1', '8', '32']:
            with self.subTest(cargo_jobs=cargo_jobs):
                result = self.run_build(cargo_jobs=cargo_jobs)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn('budget=2\n', result.stdout)

    def test_should_preserve_release_build_command_and_profile(self):
        result = self.run_build()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, 'args=build\nbudget=2\nprofile=release\n')
        self.assertIn('        CARGO_BUILD_PROFILE: release\n', ACTION.read_text())

    def test_should_propagate_application_build_failure_without_retry(self):
        result = self.run_build(exit_code=17)
        self.assertEqual(result.returncode, 17)
        self.assertEqual(result.stdout.count('args=build\n'), 1)

    def test_should_reject_invalid_budgets_before_building(self):
        for budget in ['0', '-1', '1.5', 'many', '2; echo unexpected']:
            with self.subTest(budget=budget):
                result = self.run_build(budget=budget)
                self.assertNotEqual(result.returncode, 0)
                self.assertNotIn('args=', result.stdout)
                self.assertIn('must be a positive integer', result.stderr)


if __name__ == '__main__':
    unittest.main()
