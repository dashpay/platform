"""Exercise the Dashmate macOS notarization rehearsal and the action it shares with the release."""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[3]
WORKFLOW = ROOT / '.github/workflows/release-dashmate-macos-rehearsal.yml'
RELEASE = ROOT / '.github/workflows/release.yml'
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
    # A submission's id is its package's base name, so NOTARY_REJECT can name the
    # packages Apple rejects; NOTARY_STATUS is the verdict for all the others.
    'xcrun': '''
        echo "$1 $2" >> "$XCRUN_LOG"
        [ -z "${XCRUN_ARGS_LOG:-}" ] || echo "$*" >> "$XCRUN_ARGS_LOG"
        verdict() {
          case " ${NOTARY_REJECT:-} " in
            *" $1 "*) echo Invalid ;;
            *) echo "${NOTARY_STATUS:-Accepted}" ;;
          esac
        }
        case "$1 $2" in
          'notarytool submit')
            [ "${SUBMIT_EXIT:-0}" = 0 ] || { echo 'Error: HTTP status code: 401.' >&2; exit "$SUBMIT_EXIT"; }
            id="$(basename "$3" .pkg)"
            printf 'Submission ID received\\n  id: %s\\n' "$id"
            case " $* " in
              *' --wait '*) printf 'Processing complete\\n  id: %s\\n  status: %s\\n' "$id" "$(verdict "$id")" ;;
              *) printf 'Successfully uploaded file\\n  id: %s\\n  path: %s\\n' "$id" "$3" ;;
            esac
            ;;
          'notarytool wait')
            id="$3"
            while [ "$#" -gt 0 ]; do
              [ "$1" != --key ] || key="$2"
              shift
            done
            # Apple is still polled with the key while any submission is pending.
            [ -f "${key:-}" ] || { echo 'Error: the API key file is gone.' >&2; exit 1; }
            [ "${WAIT_EXIT:-0}" = 0 ] || { echo 'Error: timed out waiting for the submission.' >&2; exit "$WAIT_EXIT"; }
            # An earlier status line; only the report's last one is the verdict.
            printf '  status: %s\\n' "${NOTARY_EARLIER_STATUS:-In Progress}"
            if [ -n "${NOTARY_TIMEOUT:-}" ]; then
              echo 'Timeout of 3600.0 second(s) was reached before processing completed.'
              exit 0
            fi
            printf 'Current status: %s....Processing complete\\n  id: %s\\n  status: %s\\n' "$(verdict "$id")" "$id" "$(verdict "$id")"
            ;;
          'notarytool log') exit "${LOG_EXIT:-0}" ;;
          'stapler staple') exit "${STAPLE_EXIT:-0}" ;;
          'stapler validate') exit "${VALIDATE_EXIT:-0}" ;;
        esac
    ''',
    'security': '''
        echo "$1" >> "$SECURITY_LOG"
        [ "$1" != delete-keychain ] || rm -f "${@: -1}"
    ''',
    'productsign': '''
        cp "$5" "$6"
        printf signed >> "$6"
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

    def packages(self):
        directory = self.tmp / 'release-artifacts' / 'macos'
        directory.mkdir(parents=True)
        paths = [directory / name for name in ['dashmate-arm64.pkg', 'dashmate-x64.pkg']]
        for path in paths:
            path.write_text('pkg')
        return paths


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

    def test_should_take_a_run_with_one_live_copy_of_the_packages(self):
        result = self.run_source_check(artifacts=({'expired': True}, {'expired': False}))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_should_fail_when_a_ticket_is_not_stapled(self):
        self.packages()
        for exit_code, expected in [('0', 0), ('65', 1)]:
            with self.subTest(exit_code=exit_code):
                result = self.run_step(WORKFLOW, 'Check that the tickets are stapled', 6,
                                       XCRUN_LOG=str(self.tmp / 'xcrun.log'), VALIDATE_EXIT=exit_code)
                self.assertEqual(result.returncode, expected, result.stdout + result.stderr)

    def test_should_run_the_steps_the_release_runs(self):
        release, rehearsal = RELEASE.read_text(), WORKFLOW.read_text()
        # The checkout cleans the workspace, so the packages come after it.
        for text, download in [(release, 'Download Dashmate packages'), (rehearsal, 'Download the macOS packages')]:
            self.assertLess(text.index('- name: Check out the signing action'), text.index('- name: ' + download))
        inputs = [re.search(r'uses: \./\.github/actions/macos-sign-notarize\n +with:\n((?: {10}\S.*\n)+)', text).group(1)
                  for text in (release, rehearsal)]
        self.assertEqual(inputs[0], inputs[1])
        runner = re.search(r'runs-on: (\S+)', rehearsal).group(1)
        self.assertIn(f'- package_type: macos\n            os: {runner}\n', release)


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
        result = self.run_step(ACTION, 'Check the inputs', 4, **dict.fromkeys(self.SECRETS, ''))
        self.assertEqual(result.stdout.count('::error::'), len(self.SECRETS))

    def test_should_sign_every_package_and_fail_without_one(self):
        env = {'PACKAGES_PATH': 'release-artifacts', 'RUNNER_TEMP': str(self.tmp)}
        result = self.run_step(ACTION, 'Sign macOS installers', 4, **env)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('::error::No .pkg files found', result.stdout)
        shutil.rmtree(self.tmp / 'release-artifacts', ignore_errors=True)
        packages = self.packages()
        self.assertEqual(self.run_step(ACTION, 'Sign macOS installers', 4, **env).returncode, 0)
        self.assertEqual([path.read_text() for path in packages], ['pkgsigned'] * 2)

    def test_should_clean_up_with_or_without_a_keychain(self):
        env = {'RUNNER_TEMP': str(self.tmp), 'SECURITY_LOG': str(self.tmp / 'security.log')}
        for keychain, calls in [(False, []), (True, ['delete-keychain'])]:
            with self.subTest(keychain=keychain):
                (self.tmp / 'build_certificate.p12').write_text('p12')
                if keychain:
                    (self.tmp / 'app-signing.keychain-db').write_text('keychain')
                self.assertEqual(self.run_step(ACTION, 'Delete the Apple keychain', 4, **env).returncode, 0)
                self.assertEqual(self.log('security.log'), calls)
                self.assertEqual([path.name for path in self.tmp.glob('*.p12')] +
                                 [path.name for path in self.tmp.glob('*.keychain-db')], [])

    def notarize(self, packages=True, **env):
        if packages:
            self.packages()
        result = self.run_step(ACTION, 'Notarize MacOS Release Build', 4, PACKAGES_PATH='release-artifacts',
                               RUNNER_TEMP=str(self.tmp), XCRUN_LOG=str(self.tmp / 'xcrun.log'),
                               XCRUN_ARGS_LOG=str(self.tmp / 'xcrun-args.log'), **dict(self.SECRETS, **env))
        self.assertFalse((self.tmp / 'notary_api_key.p8').exists())
        return result

    def test_should_notarize_and_staple_every_package(self):
        result = self.notarize()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(result.stdout.count('::notice::Apple accepted'), 2)
        self.assertEqual(self.log('xcrun.log').count('stapler staple'), 2)

    def test_should_upload_every_package_before_waiting_for_any(self):
        # Apple notarizes submissions in parallel. Waiting for one package before uploading
        # the next makes the release wait for each notarization in turn, several minutes each.
        result = self.notarize()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.log('xcrun.log'),
                         ['notarytool submit'] * 2 + ['notarytool wait'] * 2 + ['stapler staple'] * 2)
        submits = [line.split() for line in self.log('xcrun-args.log') if line.startswith('notarytool submit ')]
        self.assertEqual(len(submits), 2)
        for args in submits:
            self.assertNotIn('--wait', args)

    def test_should_fail_when_apple_rejects_a_package(self):
        result = self.notarize(NOTARY_STATUS='Invalid')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout.count('::error::Apple did not report'), 2)
        self.assertEqual(self.log('xcrun.log'),
                         ['notarytool submit'] * 2 + ['notarytool wait', 'notarytool log'] * 2)

    def test_should_report_each_rejected_package_and_staple_none(self):
        # The verdict of every package is shown, so one run tells which of them to fix, and
        # nothing is stapled while the release cannot ship.
        for rejected in [['dashmate-arm64'], ['dashmate-x64'], ['dashmate-arm64', 'dashmate-x64']]:
            with self.subTest(rejected=rejected):
                shutil.rmtree(self.tmp / 'release-artifacts', ignore_errors=True)
                for name in ['xcrun.log', 'xcrun-args.log']:
                    (self.tmp / name).unlink(missing_ok=True)
                result = self.notarize(NOTARY_REJECT=' '.join(rejected))
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(result.stdout.count('::error::Apple did not report'), len(rejected))
                for name in ['dashmate-arm64', 'dashmate-x64']:
                    verdict = '::error::Apple did not report' if name in rejected else '::notice::Apple accepted'
                    self.assertRegex(result.stdout, re.escape(verdict) + r' \S*' + re.escape(name + '.pkg'))
                self.assertEqual(self.log('xcrun.log').count('notarytool log'), len(rejected))
                self.assertNotIn('stapler staple', self.log('xcrun.log'))

    def test_should_take_the_verdict_from_the_last_status_line_of_the_report(self):
        result = self.notarize(NOTARY_EARLIER_STATUS='Accepted', NOTARY_STATUS='Invalid')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout.count('::error::Apple did not report'), 2)
        self.assertNotIn('::notice::Apple accepted', result.stdout)
        self.assertNotIn('stapler staple', self.log('xcrun.log'))

    def test_should_fail_when_apple_has_not_finished_before_the_timeout(self):
        # notarytool exits 0 when its wait times out, with no verdict in the report.
        result = self.notarize(NOTARY_TIMEOUT='1')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout.count('::error::Apple did not report'), 2)
        self.assertNotIn('stapler staple', self.log('xcrun.log'))

    def test_should_report_every_verdict_when_a_rejection_log_cannot_be_fetched(self):
        result = self.notarize(NOTARY_REJECT='dashmate-arm64', LOG_EXIT='1')
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stdout, r'::error::Apple did not report \S*dashmate-arm64\.pkg')
        self.assertRegex(result.stdout, r'::notice::Apple accepted \S*dashmate-x64\.pkg')
        self.assertNotIn('stapler staple', self.log('xcrun.log'))

    def test_should_fail_when_waiting_for_a_submission_fails(self):
        # The rejection reason is fetched here too: notarytool may also exit non-zero for a
        # package Apple rejected.
        result = self.notarize(WAIT_EXIT='1')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout.count('::error::notarytool failed'), 2)
        self.assertEqual(self.log('xcrun.log').count('notarytool log'), 2)
        self.assertNotIn('stapler staple', self.log('xcrun.log'))

    def test_should_fail_without_a_package(self):
        result = self.notarize(packages=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('::error::No .pkg files found', result.stdout)
        self.assertEqual(self.log('xcrun.log'), [])

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
