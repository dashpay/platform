"""Exercise the Dashmate macOS notarization rehearsal and the action it shares with the release."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[3]
WORKFLOW = ROOT / '.github/workflows/release-dashmate-macos-rehearsal.yml'
ACTION = ROOT / '.github/actions/macos-sign-notarize/action.yaml'
SHA = 'bb78fa2220ae0d58f5a9b5c0f6b0027a292cc3d0'

# Stand-ins for the tools the steps call. `base64 --decode -o` is the macOS
# form, so it is replaced as well and the tests also run on Linux.
STUBS = {
    'gh': '''
        echo "$2" >> "$GH_LOG"
        [ -z "${GH_FAIL:-}" ] || { echo 'gh: Not Found (HTTP 404)' >&2; exit 1; }
        case "$2" in
          */artifacts\\?name=*) printf '%s' "$GH_ARTIFACTS" ;;
          *) printf '%s' "$GH_RUN" ;;
        esac
    ''',
    'base64': '''
        cat > /dev/null
        printf '%s\\n' "${DECODED_KEY:------BEGIN PRIVATE KEY-----}" > "${@: -1}"
    ''',
    'xcrun': '''
        echo "$1 $2" >> "$XCRUN_LOG"
        case "$1 $2" in
          'notarytool submit')
            [ "${SUBMIT_EXIT:-0}" = 0 ] || { echo 'Error: HTTP status code: 401.' >&2; exit "$SUBMIT_EXIT"; }
            printf 'Submission ID received\\n  id: abc-123\\nProcessing complete\\n  id: abc-123\\n  status: %s\\n' "${NOTARY_STATUS:-Accepted}"
            ;;
          'stapler staple') exit "${STAPLE_EXIT:-0}" ;;
        esac
    ''',
    'sleep': '',
}


def step_script(path, name, indent):
    """Return the `run` script of a step; `indent` is the column of its `- name:`."""
    text = path.read_text()
    start = text.index(' ' * indent + '- name: ' + name + '\n')
    tail = text[start:].split(' ' * (indent + 2) + 'run: |\n', 1)[1]
    lines = []
    for line in tail.splitlines():
        if line.strip() and not line.startswith(' ' * (indent + 4)):
            break
        lines.append(line)
    return textwrap.dedent('\n'.join(lines))


class StepTestCase(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        (self.tmp / 'bin').mkdir()
        for name, body in STUBS.items():
            stub = self.tmp / 'bin' / name
            stub.write_text('#!/bin/bash\n' + textwrap.dedent(body))
            stub.chmod(0o755)

    def run_step(self, path, name, indent, **env):
        env = dict(os.environ, PATH=f"{self.tmp / 'bin'}{os.pathsep}{os.environ['PATH']}", **env)
        return subprocess.run(['bash', '-c', step_script(path, name, indent)],
                              capture_output=True, text=True, env=env, cwd=self.tmp)

    def log(self, name):
        path = self.tmp / name
        return path.read_text().splitlines() if path.exists() else []


class RehearsalWorkflowTests(StepTestCase):
    def run_guard(self, ref, repository='dashpay/platform', protected=True):
        return self.run_step(WORKFLOW, 'Require a protected branch', 6, GITHUB_REPOSITORY=repository,
                             GITHUB_REF=ref, GITHUB_REF_PROTECTED=str(protected).lower()).returncode

    def test_should_run_only_from_a_protected_branch_of_the_repository(self):
        self.assertEqual(self.run_guard('refs/heads/v5.0-dev'), 0)
        for ref, repository, protected in [('refs/heads/attacker', 'dashpay/platform', False),
                                           ('refs/tags/v5.0.0-beta.3', 'dashpay/platform', True),
                                           ('refs/heads/v5.0-dev', 'unknown/platform', True)]:
            with self.subTest(ref=ref, repository=repository):
                self.assertNotEqual(self.run_guard(ref, repository, protected), 0)

    def run_source_check(self, run_id='37807803284', path='.github/workflows/release.yml',
                         event='release', artifacts=({'expired': False},), **env):
        return self.run_step(
            WORKFLOW, 'Check the source run', 6, RUN_ID=run_id, GITHUB_REPOSITORY='dashpay/platform',
            GITHUB_OUTPUT=str(self.tmp / 'output'), GH_LOG=str(self.tmp / 'gh.log'),
            GH_RUN=json.dumps({'path': path, 'event': event, 'head_sha': SHA}),
            GH_ARTIFACTS=json.dumps({'artifacts': list(artifacts)}), **env)

    def test_should_take_the_macos_packages_of_a_release_run(self):
        self.assertEqual(self.run_source_check().returncode, 0)
        self.assertEqual(self.log('output'), ['head_sha=' + SHA])
        self.assertTrue(self.log('gh.log')[-1].endswith('/artifacts?name=dashmate-macos-' + SHA))

    def test_should_reject_a_run_that_a_release_did_not_start(self):
        for case in [{'run_id': '12; echo pwned'}, {'event': 'workflow_dispatch'},
                     {'path': '.github/workflows/pr.yml'}, {'artifacts': ()},
                     {'artifacts': ({'expired': True},)}, {'GH_FAIL': '1'}]:
            with self.subTest(case=case):
                result = self.run_source_check(**case)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('::error::', result.stdout)
                self.assertEqual(self.log('output'), [])


class SignNotarizeActionTests(StepTestCase):
    SECRETS = {'BUILD_CERTIFICATE_BASE64': 'cert', 'NOTARY_API_KEY_BASE64': 'key',
               'NOTARY_API_KEY_ID': 'id', 'NOTARY_API_ISSUER_ID': 'issuer'}

    def test_should_require_the_certificate_and_the_notary_key(self):
        self.assertEqual(self.run_step(ACTION, 'Check the inputs', 4, **self.SECRETS).returncode, 0)
        for name in self.SECRETS:
            with self.subTest(missing=name):
                result = self.run_step(ACTION, 'Check the inputs', 4, **dict(self.SECRETS, **{name: ''}))
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(f'The MACOS_{name} secret is not set.', result.stdout)

    def notarize(self, **env):
        packages = self.tmp / 'release-artifacts' / 'macos'
        packages.mkdir(parents=True)
        for name in ['dashmate-arm64.pkg', 'dashmate-x64.pkg']:
            (packages / name).write_text('pkg')
        result = self.run_step(ACTION, 'Notarize MacOS Release Build', 4, PACKAGES_PATH='release-artifacts',
                               RUNNER_TEMP=str(self.tmp), XCRUN_LOG=str(self.tmp / 'xcrun.log'),
                               **dict(self.SECRETS, **env))
        self.assertFalse((self.tmp / 'notary_api_key.p8').exists())
        return result

    def test_should_notarize_and_staple_every_package(self):
        result = self.notarize()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(result.stdout.count('::notice::Apple accepted'), 2)
        self.assertEqual(self.log('xcrun.log'), ['notarytool submit', 'stapler staple'] * 2)

    def test_should_fail_when_apple_rejects_a_package(self):
        result = self.notarize(NOTARY_STATUS='Invalid')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('::error::Apple did not report', result.stdout)
        self.assertEqual(self.log('xcrun.log'), ['notarytool submit', 'notarytool log'])

    def test_should_fail_when_notarytool_fails(self):
        result = self.notarize(SUBMIT_EXIT='1')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('::error::notarytool failed', result.stdout)
        self.assertEqual(self.log('xcrun.log'), ['notarytool submit'])

    def test_should_fail_before_submitting_when_the_key_is_not_a_p8(self):
        result = self.notarize(DECODED_KEY='garbage')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('::error::Could not decode', result.stdout)
        self.assertEqual(self.log('xcrun.log'), [])

    def test_should_only_warn_when_the_ticket_cannot_be_stapled(self):
        result = self.notarize(STAPLE_EXIT='65')
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(result.stdout.count('::warning::Could not staple'), 2)
        self.assertEqual(self.log('xcrun.log').count('stapler staple'), 10)


if __name__ == '__main__':
    unittest.main()
