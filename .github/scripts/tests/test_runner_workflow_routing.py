"""Regression contracts for image-only changes reaching real Rust consumers."""
import fnmatch
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[3]


class ImageWorkflowRoutingTests(unittest.TestCase):
    def setUp(self):
        self.workflow = (ROOT / '.github/workflows/tests.yml').read_text()
        self.callee = (ROOT / '.github/workflows/tests-rs-workspace.yml').read_text()

    def filters(self, name):
        section = self.workflow.split('        id: filter-rs-workflows\n', 1)[1].split('\n      - ', 1)[0]
        match = re.search(r'^            ' + re.escape(name) + r':\n((?:              - .+\n)+)', section, re.M)
        self.assertIsNotNone(match, name + ' image routing filter is missing')
        return [line.strip()[2:] for line in match[1].splitlines()]

    def job(self, name):
        match = re.search(r'^  ' + re.escape(name) + r':\n(.*?)(?=^  [a-zA-Z0-9_-]+:|\Z)', self.workflow, re.M | re.S)
        self.assertIsNotNone(match, name + ' caller is missing')
        return match[1]

    def test_image_only_inputs_trigger_rust_workflow_filter(self):
        patterns = self.filters('rs-workflows')
        for path in ('.github/runner-requirements.json', '.github/runner-requirements.arm64.json',
                     '.github/scripts/runner-image.py', '.github/actions/rust/action.yaml'):
            with self.subTest(path=path):
                self.assertTrue(any(fnmatch.fnmatchcase(path, p) for p in patterns), path)
        self.assertIn("needs.changes.outputs.rs-workflows-changed == 'true'", self.job('rs-workspace-tests'))

    def test_arm64_manifest_has_a_dedicated_full_workspace_caller(self):
        self.assertEqual(self.filters('arm64-image'), ['.github/runner-requirements.arm64.json'])
        self.assertIn('arm64-image-changed: ${{ steps.filter-rs-workflows.outputs.arm64-image }}', self.workflow)
        caller = self.job('rs-arm64-image-tests')
        for line in ('needs: changes', "if: ${{ needs.changes.outputs.arm64-image-changed == 'true' }}",
                     'uses: ./.github/workflows/tests-rs-workspace.yml', 'runner-architecture: ARM64',
                     'validate-arm64-image: true', 'doctests-changed: true', 'shielded-changed: true', 'coverage: false'):
            self.assertIn(line, caller)
        self.assertNotIn('continue-on-error:', caller)

    def test_image_environment_changes_invalidate_shielded_skip(self):
        regexes = re.findall(r'grep -qE \'([^\']+)\'', self.workflow)
        patterns = [p for p in regexes if 'rust-toolchain' in p]
        self.assertEqual(len(patterns), 1)
        for path in ('.github/runner-requirements.json', '.github/runner-requirements.arm64.json',
                     '.github/scripts/runner-image.py'):
            self.assertIsNotNone(re.search(patterns[0], path), path)

    def test_hosted_routing_contracts_are_in_the_changes_job(self):
        changes = self.job('changes')
        self.assertIn('python3 .github/scripts/runner-image.py env', changes)
        self.assertIn('python3 -m unittest discover -s .github/scripts/tests -v', changes)

    def test_new_caller_keeps_the_existing_callee_fork_guard(self):
        for guard in ("github.event_name != 'pull_request'",
                      'github.event.pull_request.head.repo.full_name == github.repository',
                      "github.event.pull_request.head.repo.owner.login == 'thepastaclaw'"):
            self.assertIn(guard, self.callee)
        self.assertIn('validate-arm64-image:', self.callee)
        self.assertIn('runner-architecture:', self.callee)
        self.assertIn('uses: ./.github/actions/rust', self.callee)
        self.assertIn('runner-image.py verify', (ROOT / '.github/actions/rust/action.yaml').read_text())

    def test_contract_failure_pointer_names_existing_resources(self):
        action = (ROOT / '.github/actions/rust/action.yaml').read_text()
        self.assertNotIn('.github/SELF_HOSTED_RUNNER.md', action)
        for path in ('.github/runner-requirements.json', '.github/workflows/runner-image-candidate.yml'):
            self.assertIn(path, action)
            self.assertTrue((ROOT / path).is_file())


if __name__ == '__main__':
    unittest.main()
