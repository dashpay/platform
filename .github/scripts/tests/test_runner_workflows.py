"""Keep the v4.3 image adoption scoped to provisioning, not test or trust skips."""
import json
from pathlib import Path
import re
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[3]


class RunnerWorkflowTests(unittest.TestCase):
    def text(self, name):
        return (ROOT / '.github' / name).read_text()

    def test_should_retain_same_repo_and_existing_trusted_fork_guards(self):
        for name in ('kotlin-sdk-build.yml', 'tests-rs-workspace.yml'):
            workflow = self.text('workflows/' + name)
            guard = ("github.event_name != 'pull_request'\n"
                     "      || github.event.pull_request.head.repo.full_name == github.repository\n"
                     "      || github.event.pull_request.head.repo.owner.login == 'thepastaclaw'")
            self.assertEqual(workflow.count(guard), 2, name)
            self.assertIn('needs: select-runner', workflow)
            self.assertIn('runs-on: ${{ fromJSON(needs.select-runner.outputs.labels) }}', workflow)
            self.assertIn('persist-credentials: false', workflow)

    def test_should_verify_prebaked_tools_instead_of_mutating_android_sdk(self):
        kotlin = self.text('workflows/kotlin-sdk-build.yml')
        self.assertNotIn('uses: android-actions/setup-android', kotlin)
        executable_lines = '\n'.join(line for line in kotlin.splitlines()
                                     if not line.lstrip().startswith('#'))
        self.assertNotIn('sudo ', executable_lines)
        self.assertIn('python3 .github/scripts/runner-image.py verify', kotlin)
        self.assertIn('ci-android-emulator bash ../../.github/scripts/kotlin-instrumented-tests.sh', kotlin)
        self.assertIn('docs/sdk/KOTLIN_SWIFT_SHARED_PARITY_SPEC.md', kotlin)
        for task in ('./build_android.sh --abi x86_64 --profile dev --verify',
                     ':sdk:assembleDebug :sdk:testDebugUnitTest',
                     ':app:assembleDebug :app:testDebugUnitTest',
                     ':sdk:compileDebugAndroidTestKotlin'):
            self.assertIn(task, kotlin)
        helper = self.text('scripts/kotlin-instrumented-tests.sh')
        self.assertIn('./gradlew :sdk:connectedDebugAndroidTest --stacktrace "$@"', helper)
        self.assertIn("grep -q 'deviceLocked=0'", helper)

    def test_should_pin_controller_independently_and_keep_hosted_contract_tests(self):
        candidate = self.text('workflows/runner-image-candidate.yml')
        revisions = re.findall(r'(?:yml@|control_revision: )([0-9a-f]{40})', candidate)
        self.assertEqual(len(revisions), 2)
        self.assertEqual(revisions[0], revisions[1])
        self.assertIn('pull_request_target:', candidate)
        self.assertNotIn('actions/checkout', candidate)
        tests = self.text('workflows/runner-contract-tests.yml')
        self.assertIn('runs-on: ubuntu-24.04', tests)
        self.assertNotIn('secrets:', tests)
        self.assertNotIn('self-hosted', tests)

    def test_should_match_target_rust_version_on_both_architectures(self):
        toolchain = tomllib.loads((ROOT / 'rust-toolchain.toml').read_text())
        for name in ('runner-requirements.json', 'runner-requirements.arm64.json'):
            manifest = json.loads(self.text(name))
            self.assertEqual(manifest['requirements']['rust_version'], toolchain['toolchain']['channel'])
