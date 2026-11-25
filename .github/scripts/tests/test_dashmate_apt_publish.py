"""Ask the apt publisher to publish a release's debs, from the Publish Dashmate debs to apt job."""
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
RELEASE = ROOT / '.github/workflows/release.yml'
JOB = 'publish-dashmate-apt'
ROLE = 'arn:aws:iam::854439639386:role/dashmate-apt-release'
STATE_MACHINE = 'arn:aws:states:us-west-2:854439639386:stateMachine:dashmate-apt-publish'
CHECK_STEP = 'Check the release tag'
PUBLISH_STEP = 'Publish the debs to the apt repository'
CREDENTIALS_STEP = 'Get credentials for the apt publisher'
RUN_ID = '19876543210'
# The sha256 of the SHA256SUMS the checksums job attached, as its job output hands it over.
SUMS_SHA256 = '3f0c' + 'ab' * 30

# `aws stepfunctions` as the runner's AWS CLI answers it. start-execution answers with the new
# execution's ARN, or fails with AWS_START_FAIL. Each describe-execution takes the next status of
# AWS_STATUSES (comma-separated; ERR fails the call, and the last status repeats), with
# AWS_ERROR and AWS_CAUSE on a closed execution that did not succeed. The log has one JSON list of
# arguments per call.
AWS_STUB = '''#!/usr/bin/env python3
import json, os, sys
args = sys.argv[1:]
with open(os.environ['AWS_LOG'], 'a') as log:
    log.write(json.dumps(args) + '\\n')
def option(name):
    return args[args.index(name) + 1]
if args[:2] == ['stepfunctions', 'start-execution']:
    if os.environ.get('AWS_START_FAIL'):
        print('An error occurred (AccessDeniedException) when calling the StartExecution operation', file=sys.stderr)
        sys.exit(254)
    assert args[args.index('--query') + 1] == 'executionArn' and option('--output') == 'text', args
    machine = option('--state-machine-arn').replace(':stateMachine:', ':execution:')
    print(f"{machine}:{option('--name')}")
    sys.exit(0)
if args[:2] == ['stepfunctions', 'describe-execution']:
    statuses = os.environ['AWS_STATUSES'].split(',')
    counter = os.environ['AWS_LOG'] + '.describes'
    count = int(open(counter).read()) if os.path.exists(counter) else 0
    open(counter, 'w').write(str(count + 1))
    status = statuses[min(count, len(statuses) - 1)]
    if status == 'NOT_JSON':
        print('<html>gateway timeout</html>')
        sys.exit(0)
    if status == 'ERR':
        print('An error occurred (ThrottlingException) when calling the DescribeExecution operation', file=sys.stderr)
        sys.exit(254)
    assert option('--output') == 'json', args
    answer = {'executionArn': option('--execution-arn'), 'status': status, 'input': '{}'}
    if status not in ('RUNNING', 'SUCCEEDED', 'PENDING_REDRIVE'):
        answer.update(error=os.environ.get('AWS_ERROR', 'States.TaskFailed'),
                      cause=os.environ.get('AWS_CAUSE', 'SHA256SUMS does not list dashmate_5.1.0-1_amd64.deb'))
    print(json.dumps(answer, indent=4))
    sys.exit(0)
sys.exit(2)
'''


def job(name):
    """The text of one job of release.yml, up to the next job."""
    text = RELEASE.read_text()
    start = text.index(f'\n  {name}:\n')
    end = re.search(r'\n  [a-z][a-z0-9-]*:\n', text[start + 1:])
    return text[start:start + 1 + end.start()] if end else text[start:]


def job_lines():
    return job(JOB).splitlines()


def step_block(name):
    """The lines of a step of the job, from its `- name:` to the next step."""
    lines = job_lines()
    start = lines.index('      - name: ' + name)
    block = [lines[start]]
    for line in lines[start + 1:]:
        if line.strip() and len(line) - len(line.lstrip()) <= 6:
            break
        block.append(line)
    return block


def mapping(lines, header, indent):
    """The `key: value` entries under `header`, with plain or single-quoted values."""
    if header not in lines:
        return {}
    entries = {}
    for line in lines[lines.index(header) + 1:]:
        if not line.startswith(' ' * indent) or line[indent] == ' ':
            break
        if line.strip().startswith('#'):
            continue
        key, value = line.strip().split(': ', 1)
        entries[key] = value[1:-1] if value[:1] == "'" else value
    return entries


def step_env(name):
    """The step's own env, below the job's env."""
    return dict(mapping(job_lines(), '    env:', 6), **mapping(step_block(name), '        env:', 10))


def step_script(name):
    block = step_block(name)
    script = []
    for line in block[block.index('        run: |') + 1:]:
        if line.strip() and not line.startswith(' ' * 10):
            break
        script.append(line)
    return textwrap.dedent('\n'.join(script))


def evaluate(value, tag, sums_sha256=SUMS_SHA256):
    """`value` with each ${{ expression }} replaced as the runner would; unknown ones raise."""
    known = {'github.event.release.tag_name': tag,
             'needs.release-dashmate-checksums.outputs.sha256sums_sha256': sums_sha256}
    return re.sub(r'\$\{\{\s*(.+?)\s*\}\}', lambda match: known[match.group(1)], value)


class AptPublishJobTests(unittest.TestCase):
    def test_should_run_only_after_sha256sums_is_attached_for_a_published_release(self):
        # The publisher takes the debs only as the release's SHA256SUMS lists them, and the
        # checksums job succeeds only once every package leg, the deb leg among them, attached
        # its packages. A dispatch, npm-test: or not, attaches neither.
        text = job(JOB)
        self.assertIn('\n    needs: release-dashmate-checksums\n', text)
        condition = re.search(r'\n    if: >-\n((?:      .*\n)+)', text).group(1).split()
        self.assertEqual(' '.join(condition), "!cancelled() && github.event_name == 'release'"
                         " && needs.release-dashmate-checksums.result == 'success'")
        checksums = re.search(r'\n    if: >-\n((?:      .*\n)+)', job('release-dashmate-checksums')).group(1)
        self.assertIn("github.event_name == 'release'", checksums)

    def test_should_take_the_tag_from_the_published_release(self):
        self.assertEqual(mapping(job_lines(), '    env:', 6)['TAG'], '${{ github.event.release.tag_name }}')

    def test_should_take_the_sha256sums_digest_from_the_checksums_job(self):
        # The digest of the one SHA256SUMS this run checked and attached, so the publisher can
        # refuse any other manifest the release holds by the time the request is served.
        self.assertEqual(mapping(job_lines(), '    env:', 6)['SHA256SUMS_SHA256'],
                         '${{ needs.release-dashmate-checksums.outputs.sha256sums_sha256 }}')

    def test_should_get_aws_credentials_only_in_the_environment_the_role_trusts(self):
        # The role trusts only this environment, whose deployments are restricted to v* tags.
        # Without contents access the job can neither read nor change the release's assets.
        text = job(JOB)
        self.assertIn('\n    environment: dashmate-apt-publish\n', text)
        self.assertEqual(mapping(job_lines(), '    permissions:', 6), {'id-token': 'write', 'contents': 'none'})

    def test_should_use_only_the_credentials_action_pinned_to_a_commit(self):
        uses = re.findall(r'\n\s+(?:- )?uses: (.+)', job(JOB))
        self.assertEqual(len(uses), 1, uses)
        self.assertRegex(uses[0], r'^aws-actions/configure-aws-credentials@[0-9a-f]{40} # v\d+\.\d+\.\d+$')
        self.assertEqual(mapping(step_block(CREDENTIALS_STEP), '        with:', 10),
                         {'role-to-assume': ROLE, 'aws-region': 'us-west-2'})
        self.assertNotIn('actions/checkout', job(JOB))

    def test_should_check_the_tag_before_getting_credentials(self):
        names = re.findall(r'\n      - name: (.+)', job(JOB))
        self.assertEqual(names, [CHECK_STEP, CREDENTIALS_STEP, PUBLISH_STEP])

    def test_should_publish_only_the_tags_the_check_lets_through(self):
        for step in (CREDENTIALS_STEP, PUBLISH_STEP):
            with self.subTest(step):
                self.assertIn("        if: steps.tag.outputs.publish == 'true'", step_block(step))
        self.assertIn('        id: tag', step_block(CHECK_STEP))

    def test_should_keep_expressions_out_of_the_scripts(self):
        for step in (CHECK_STEP, PUBLISH_STEP):
            with self.subTest(step):
                self.assertNotIn('${{', step_script(step))

    def test_should_give_up_waiting_before_the_job_times_out_and_the_credentials_expire(self):
        timeout = int(re.search(r'\n    timeout-minutes: (\d+)\n', job(JOB)).group(1))
        env = mapping(job_lines(), '    env:', 6)
        wait = int(env['APT_PUBLISH_WAIT_SECONDS'])
        # The action's default role session for OIDC lasts an hour.
        self.assertLess(wait + 5 * 60, timeout * 60)
        self.assertLess(wait, 3600)
        self.assertGreaterEqual(int(env['APT_PUBLISH_POLL_SECONDS']), 5)

    def test_should_be_needed_by_no_other_job(self):
        # A failed publish fails this job only: every release asset stays as the other jobs made it.
        others = RELEASE.read_text().replace(job(JOB), '')
        self.assertNotRegex(others, rf'needs:[^\n]*\b{JOB}\b|\n      - {JOB}\n')

    def test_should_not_be_a_dashmate_writer_for_the_signing_script(self):
        # It changes no asset, so signing may go on while it waits on the publisher's queue.
        name = re.search(r'\n    name: (.+)', job(JOB)).group(1)
        script = (ROOT / 'scripts/release/sign_dashmate_checksums.sh').read_text()
        selector = re.search(r'dashmate_jobs\(\) \{.*?--jq \'(.*?)\' < /dev/null', script, re.S).group(1)
        result = subprocess.run(['jq', '-r', selector], input=json.dumps({'jobs': [
            {'name': name, 'id': 1, 'status': 'in_progress', 'conclusion': None}]}),
            capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, '')


@unittest.skipUnless(shutil.which('jq'), 'needs jq')
class AptPublishScriptTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        (self.tmp / 'bin').mkdir()
        (self.tmp / 'bin/aws').write_text(AWS_STUB)
        (self.tmp / 'bin/aws').chmod(0o755)
        # Polling sleeps are logged, not slept.
        (self.tmp / 'bin/sleep').write_text(f'#!/bin/sh\necho "$@" >> {self.tmp}/sleeps\n')
        (self.tmp / 'bin/sleep').chmod(0o755)

    def run_step(self, step, tag='v5.1.0', attempt='1', sums_sha256=SUMS_SHA256, **knobs):
        env = {'PATH': f"{self.tmp / 'bin'}{os.pathsep}{os.environ['PATH']}", 'HOME': str(self.tmp),
               'GITHUB_RUN_ID': RUN_ID, 'GITHUB_RUN_ATTEMPT': attempt, 'AWS_LOG': str(self.tmp / 'aws.log'),
               'AWS_STATUSES': 'SUCCEEDED', 'GITHUB_OUTPUT': str(self.tmp / 'github-output')}
        env.update({key: evaluate(value, tag, sums_sha256) for key, value in step_env(step).items()})
        env.update(knobs)
        # sleep is stubbed, so a loop that ignores its bounds would spin forever.
        return subprocess.run(['/bin/bash', '-c', step_script(step)], capture_output=True, text=True, env=env,
                              cwd=self.tmp, timeout=30)

    def aws_calls(self):
        log = self.tmp / 'aws.log'
        return [json.loads(line) for line in log.read_text().splitlines()] if log.exists() else []

    def start_call(self):
        starts = [call for call in self.aws_calls() if call[:2] == ['stepfunctions', 'start-execution']]
        self.assertEqual(len(starts), 1, self.aws_calls())
        return starts[0]

    def option(self, call, name):
        return call[call.index(name) + 1]

    def sleeps(self):
        path = self.tmp / 'sleeps'
        return path.read_text().split() if path.exists() else []

    def outputs(self):
        path = self.tmp / 'github-output'
        return path.read_text().splitlines() if path.exists() else []

    def test_should_publish_finals_and_the_prereleases_the_apt_repository_takes(self):
        for tag in ['v5.1.0', 'v5.1.0-rc.1', 'v5.1.0-dev.11', 'v10.20.30-beta.2', 'v5.1.0-rc']:
            with self.subTest(tag):
                (self.tmp / 'github-output').unlink(missing_ok=True)
                result = self.run_step(CHECK_STEP, tag)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(self.outputs(), ['publish=true'])

    def test_should_skip_with_a_notice_the_prereleases_the_apt_repository_does_not_take(self):
        # The publisher refuses them, so asking would only turn the release run red.
        for tag, label in [('v0.0.0-alpha', 'alpha'), ('v5.1.0-alpha.3', 'alpha'), ('v3.0.1-hotfix.4', 'hotfix'),
                           ('v2.0.0-pr.2650.1', 'pr'), ('v5.1.0-rc1', 'rc1'), ('v5.1.0-devnet.2', 'devnet')]:
            with self.subTest(tag):
                (self.tmp / 'github-output').unlink(missing_ok=True)
                result = self.run_step(CHECK_STEP, tag)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(self.outputs(), ['publish=false'])
                self.assertIn(f'::notice::{tag} is not published to apt: the apt publisher takes finals and dev,'
                              f' beta and rc prereleases only, and refuses {label} ones.', result.stdout)
        self.assertEqual(self.aws_calls(), [])

    def test_should_refuse_anything_else_as_a_tag(self):
        for tag in ['', '5.1.0', 'v5.1.0\n::warning::forged', 'v5.1', 'v5.1.0.1', 'v05.1.0', 'v5.1.0-', 'v5.1.0-rc..1', 'v5.1.0-rc-1',
                    'v5.1.0+build', 'v5.1.0\n', 'v5.1.0\nv5.1.1', 'v5.1.0"}', 'v5.1.0;id', 'v5.1.0-$(id)',
                    ' v5.1.0', 'refs/tags/v5.1.0']:
            with self.subTest(tag):
                result = self.run_step(CHECK_STEP, tag)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(self.outputs(), [])
                # The refused tag is quoted onto the one error line, so it cannot start a line
                # of the log and form a workflow command.
                lines = result.stdout.splitlines()
                self.assertEqual(len(lines), 1, result.stdout)
                self.assertTrue(lines[0].startswith('::error::'), result.stdout)
        self.assertEqual(self.aws_calls(), [])

    def test_should_refuse_a_missing_or_malformed_sha256sums_digest(self):
        for digest in ['', SUMS_SHA256[:-1], SUMS_SHA256.upper(), SUMS_SHA256 + '0', SUMS_SHA256 + '\n',
                       'sha256:' + SUMS_SHA256[7:], 'g' * 64]:
            with self.subTest(digest):
                result = self.run_step(CHECK_STEP, sums_sha256=digest)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(self.outputs(), [])
                self.assertIn('::error::', result.stdout)
        self.assertEqual(self.aws_calls(), [])

    def test_should_start_the_state_machine_with_the_tag_as_json(self):
        # The JSON is built from the tag as data, whatever it holds.
        for tag in ('v5.1.0-rc.1', 'v5.1.0"}, "x": {"', 'v5.1.0\\'):
            with self.subTest(tag):
                (self.tmp / 'aws.log').unlink(missing_ok=True)
                result = self.run_step(PUBLISH_STEP, tag)

                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                start = self.start_call()
                self.assertEqual(self.option(start, '--state-machine-arn'), STATE_MACHINE)
                self.assertEqual(json.loads(self.option(start, '--input')),
                                 {'tag': tag, 'sha256sums_sha256': SUMS_SHA256})

    def test_should_name_each_run_attempt_with_a_name_step_functions_accepts(self):
        names = {}
        for attempt in ('1', '2'):
            (self.tmp / 'aws.log').unlink(missing_ok=True)
            result = self.run_step(PUBLISH_STEP, 'v5.1.0-rc.1', attempt=attempt)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            names[attempt] = self.option(self.start_call(), '--name')
        self.assertEqual(names, {'1': f'v5-1-0-rc-1-{RUN_ID}-1', '2': f'v5-1-0-rc-1-{RUN_ID}-2'})

    def test_should_shorten_the_tag_in_the_name_to_fit_80_characters(self):
        tag = 'v5.1.0-' + '.'.join(['rc1234567'] * 9)
        result = self.run_step(PUBLISH_STEP, tag, attempt='12')

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        name = self.option(self.start_call(), '--name')
        self.assertEqual(len(name), 80)
        self.assertRegex(name, r'^[0-9A-Za-z_-]+$')
        self.assertTrue(name.endswith(f'-{RUN_ID}-12'), name)
        self.assertTrue(name.startswith('v5-1-0-rc1234567-'), name)
        self.assertEqual(json.loads(self.option(self.start_call(), '--input')),
                         {'tag': tag, 'sha256sums_sha256': SUMS_SHA256})

    def test_should_wait_while_the_execution_runs_and_pass_when_it_succeeded(self):
        result = self.run_step(PUBLISH_STEP, AWS_STATUSES='RUNNING,PENDING_REDRIVE,RUNNING,SUCCEEDED')

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(len(self.aws_calls()), 5)
        self.assertEqual(self.sleeps(), [step_env(PUBLISH_STEP)['APT_PUBLISH_POLL_SECONDS']] * 3)
        self.assertIn('apt publish: SUCCEEDED', result.stdout)
        self.assertIn(f':execution:dashmate-apt-publish:v5-1-0-{RUN_ID}-1', result.stdout)

    def test_should_fail_with_the_error_and_cause_when_the_publish_did_not_succeed(self):
        for status in ('FAILED', 'TIMED_OUT', 'ABORTED', 'SOMETHING_NEW', ''):
            with self.subTest(status):
                (self.tmp / 'aws.log.describes').unlink(missing_ok=True)
                result = self.run_step(PUBLISH_STEP, AWS_STATUSES=f'RUNNING,{status}',
                                       AWS_CAUSE='::error::forged\n##[error]forged too # \\# ##[')

                self.assertNotEqual(result.returncode, 0)
                self.assertIn(f'apt publish: {status}', result.stdout)
                self.assertIn('States.TaskFailed', result.stdout)
                # What the execution reports is data: it cannot start a line, so it cannot form a
                # workflow command.
                self.assertIn('forged', result.stdout)
                self.assertFalse([line for line in result.stdout.splitlines() if 'forged' in line
                                  and line.lstrip().startswith('::')], result.stdout)
                # The runner reads a `##[` command anywhere in a line.
                self.assertNotIn('##[', result.stdout)
                details = json.loads(next(line for line in result.stdout.splitlines() if line.startswith('{')))
                self.assertEqual(details['cause'], '::error::forged\n##[error]forged too # \\# ##[')

    @staticmethod
    def build_cause(exit_status, stream='0b1c2d3e-4f50-6172-8394-a5b6c7d8e9f0', command='python3 publish.py',
                    shape='step functions'):
        """The cause of a failed CodeBuild task: the build, as Step Functions hands it over (keys in
        PascalCase, under Build) or as BatchGetBuilds describes it (camelCase, bare)."""
        build = {'id': 'dashmate-apt-publish:' + stream, 'buildStatus': 'FAILED', 'phases': [
            {'phaseType': 'INSTALL', 'phaseStatus': 'SUCCEEDED'},
            {'phaseType': 'BUILD', 'phaseStatus': 'FAILED', 'contexts': [
                {'statusCode': 'COMMAND_EXECUTION_ERROR',
                 'message': f'Error while executing command: {command}. Reason: exit status {exit_status}'}]}],
            'logs': {'groupName': '/aws/codebuild/dashmate-apt-publish', 'streamName': stream}}
        if shape == 'batch get builds':
            return json.dumps(build)

        def pascal(value):
            if isinstance(value, dict):
                return {key[0].upper() + key[1:]: pascal(item) for key, item in value.items()}
            return [pascal(item) for item in value] if isinstance(value, list) else value
        return json.dumps({'Build': pascal(build)})

    def test_should_say_where_the_publisher_logged_why_and_what_its_exit_status_means(self):
        stream = '0b1c2d3e-4f50-6172-8394-a5b6c7d8e9f0'
        cases = {
            # refused, possibly after writing part of it
            '2': 'refused v5.1.0, possibly part-way',
            # went through, then reported a problem that may concern another release
            '1': 'v5.1.0 may be published',
        }
        for shape in ('step functions', 'batch get builds'):
            for exit_status, meaning in cases.items():
                with self.subTest(shape=shape, exit_status=exit_status):
                    (self.tmp / 'aws.log.describes').unlink(missing_ok=True)
                    result = self.run_step(PUBLISH_STEP, AWS_STATUSES='FAILED',
                                           AWS_CAUSE=self.build_cause(exit_status, shape=shape))

                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn(f'publisher exited {exit_status}', result.stdout)
                    self.assertIn(meaning, result.stdout)
                    self.assertIn(f'/aws/codebuild/dashmate-apt-publish, stream {stream}', result.stdout)
                    self.assertIn('REFUSED:' if exit_status == '2' else 'NEEDS A PERSON:', result.stdout)
                    self.assertIn('check the suites before re-running', result.stdout)

    def test_should_read_an_exit_status_only_from_the_publisher_itself(self):
        # The build runs other commands first; their failure says nothing about what was published.
        result = self.run_step(PUBLISH_STEP, AWS_STATUSES='FAILED',
                               AWS_CAUSE=self.build_cause('1', command='python3 --version && gpg --version'))

        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn('publisher exited', result.stdout)
        self.assertIn('/aws/codebuild/dashmate-apt-publish, stream', result.stdout)

    def test_should_still_name_the_execution_when_the_cause_is_not_a_build(self):
        result = self.run_step(PUBLISH_STEP, AWS_STATUSES='FAILED', AWS_CAUSE='States.Timeout')

        self.assertNotEqual(result.returncode, 0)
        self.assertIn(f':execution:dashmate-apt-publish:v5-1-0-{RUN_ID}-1', result.stdout.splitlines()[-1])
        self.assertNotIn('publisher exited', result.stdout)

    def test_should_take_nothing_but_digits_and_a_log_name_from_the_build(self):
        cause = json.loads(self.build_cause('2', stream='x\n::error::forged', shape='batch get builds'))
        cause['phases'][1]['contexts'][0]['message'] = 'Reason: exit status 2; ##[error]forged'
        result = self.run_step(PUBLISH_STEP, AWS_STATUSES='FAILED', AWS_CAUSE=json.dumps(cause))

        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn('##[', result.stdout)
        self.assertFalse([line for line in result.stdout.splitlines() if line.lstrip().startswith('::')
                          and 'forged' in line], result.stdout)
        self.assertNotIn(', stream x', result.stdout)

    def test_should_keep_reading_the_execution_through_failed_calls(self):
        result = self.run_step(PUBLISH_STEP, AWS_STATUSES='ERR,RUNNING,ERR,NOT_JSON,SUCCEEDED')

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('::warning::', result.stdout)

    def test_should_fail_at_the_deadline_while_the_execution_still_runs(self):
        result = self.run_step(PUBLISH_STEP, AWS_STATUSES='RUNNING', APT_PUBLISH_WAIT_SECONDS='0')

        self.assertNotEqual(result.returncode, 0)
        self.assertIn('::error::', result.stdout)
        self.assertIn('may still be running and succeed', result.stdout)

    def test_should_stop_when_the_execution_cannot_be_read_again_and_again(self):
        # An error that will not clear, such as a denied read, fails in a minute, not at the
        # deadline, and does not claim to know the execution is running.
        result = self.run_step(PUBLISH_STEP, AWS_STATUSES='RUNNING,ERR')

        self.assertNotEqual(result.returncode, 0)
        describes = [call for call in self.aws_calls() if call[1] == 'describe-execution']
        self.assertEqual(len(describes), 6)
        self.assertIn('Could not read', result.stdout.splitlines()[-1])
        self.assertNotIn('may still succeed', result.stdout)

    def test_should_fail_when_the_execution_cannot_be_started(self):
        result = self.run_step(PUBLISH_STEP, AWS_START_FAIL='1')

        self.assertNotEqual(result.returncode, 0)
        self.assertEqual([call[1] for call in self.aws_calls()], ['start-execution'])

    def test_should_be_clean_under_shellcheck(self):
        if not shutil.which('shellcheck'):
            self.skipTest('needs shellcheck')
        for step in (CHECK_STEP, PUBLISH_STEP):
            with self.subTest(step):
                result = subprocess.run(['shellcheck', '--shell=bash', '-'], input=step_script(step),
                                        capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == '__main__':
    unittest.main()
