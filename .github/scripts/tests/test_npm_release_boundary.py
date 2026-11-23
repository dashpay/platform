"""Exercise caller/runtime guards and preserve the PR/release cache boundary."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[3]


def step_script(filename, name):
    workflow = (ROOT / '.github/workflows' / filename).read_text()
    start = workflow.index('      - name: ' + name)
    tail = workflow[start:].split('        run: |\n', 1)[1]
    lines = []
    for line in tail.splitlines():
        if line.strip() and not line.startswith('          '):
            break
        lines.append(line)
    return textwrap.dedent('\n'.join(lines))


class ReleaseBoundaryTests(unittest.TestCase):
    def run_guard(self, event, ref, repository='dashpay/platform', protected=False):
        script = step_script('release-npm-build.yml', 'Reject untrusted release callers')
        return subprocess.run(['bash', '-c', script], capture_output=True, text=True,
                              env=dict(os.environ, GITHUB_REPOSITORY=repository,
                                       GITHUB_EVENT_NAME=event, GITHUB_REF=ref,
                                       GITHUB_REF_PROTECTED=str(protected).lower())).returncode

    def test_should_allow_release_tags_and_protected_branch_dry_runs(self):
        for event, ref in [('release', 'refs/tags/v4.2.0-beta.5'),
                           ('workflow_dispatch', 'refs/tags/v4.2.0-beta.5'),
                           ('workflow_dispatch', 'refs/heads/v4.2-dev'),
                           ('workflow_dispatch', 'refs/heads/v4.3-dev')]:
            with self.subTest(event=event, ref=ref):
                self.assertEqual(self.run_guard(event, ref, protected=True), 0)

    def test_should_reject_prs_forks_and_unprotected_dispatches(self):
        for event, ref in [('pull_request', 'refs/pull/5068/merge'),
                           ('pull_request_target', 'refs/heads/v4.2-dev'),
                           ('workflow_dispatch', 'refs/heads/attacker'),
                           ('push', 'refs/heads/v4.2-dev'),
                           ('release', 'refs/heads/v4.2-dev')]:
            with self.subTest(event=event, ref=ref):
                self.assertNotEqual(self.run_guard(event, ref), 0)
        self.assertNotEqual(self.run_guard('release', 'refs/tags/v4.2.0', 'unknown/platform'), 0)
        self.assertNotEqual(self.run_guard('workflow_dispatch', 'refs/heads/v4.2-dev'), 0)

    def test_should_reject_persistent_wrong_attempt_and_wrong_kind_runners(self):
        for kind, filename in [('npm', 'release-npm-build.yml'), ('kotlin', 'release-kotlin-sdk.yml')]:
            script = step_script(filename, 'Verify disposable release runner')
            env = dict(os.environ, GITHUB_RUN_ID='123', GITHUB_RUN_ATTEMPT='2',
                       DASH_RELEASE_RUNNER='1', DASH_RELEASE_RUN_ID='123',
                       DASH_RELEASE_RUN_ATTEMPT='2', DASH_RELEASE_KIND=kind)
            self.assertEqual(subprocess.run(['bash', '-c', script], env=env).returncode, 0)
            for key, value in [('DASH_RELEASE_RUNNER', ''), ('DASH_RELEASE_RUN_ID', '124'),
                               ('DASH_RELEASE_RUN_ATTEMPT', '1'), ('DASH_RELEASE_KIND', 'rust')]:
                with self.subTest(kind=kind, key=key):
                    self.assertNotEqual(subprocess.run(['bash', '-c', script],
                                                       env=dict(env, **{key: value})).returncode, 0)

    def test_should_route_both_release_builds_away_from_persistent_ci(self):
        for kind, filename in [('npm', 'release-npm-build.yml'), ('kotlin', 'release-kotlin-sdk.yml')]:
            workflow = (ROOT / '.github/workflows' / filename).read_text()
            build = workflow.split('  attach-release:', 1)[0]
            self.assertIn('group: platform-release-builds', build)
            self.assertIn('platform-release-${{ github.run_id }}-${{ github.run_attempt }}-' + kind, build)
            self.assertNotIn('runs-on: [self-hosted, kotlin-ci]', build)
            self.assertNotIn('Linux, X64, npm-build', build)
            self.assertLess(build.index('Verify disposable release runner'), build.index('uses: actions/checkout'))
        caller = (ROOT / '.github/workflows/release.yml').read_text()
        self.assertIn('uses: ./.github/workflows/release-npm-build.yml', caller)
        self.assertNotIn('release-npm-build.yml@v4.2-dev', caller)

    def test_should_keep_pr_caching_enabled_but_exclude_it_from_releases(self):
        node = (ROOT / '.github/actions/nodejs/action.yaml').read_text()
        cache_input = node.split('  cache:\n', 1)[1].split('  node-version:', 1)[0]
        self.assertIn('default: "true"', cache_input)
        self.assertIn("if: inputs.cache == 'true'", node)
        self.assertIn('uses: actions/cache@', node)
        npm = (ROOT / '.github/actions/npm-release-build/action.yaml').read_text()
        self.assertIn("cache: ${{ env.DASH_RELEASE_RUNNER != '1' }}", npm)
        kotlin = (ROOT / '.github/workflows/release-kotlin-sdk.yml').read_text()
        gradle_steps = kotlin.split('uses: gradle/actions/setup-gradle@')[1:]
        self.assertEqual(len(gradle_steps), 2)
        for gradle in gradle_steps:
            self.assertIn('cache-disabled: true', gradle.split('\n      - ', 1)[0])
        # The ordinary image-validation workflow retains its own cache name
        # and does not opt into the release runtime marker.
        validation = (ROOT / '.github/workflows/npm-runner-validation.yml').read_text()
        self.assertIn('cache-name: npm-validation-target', validation)
        self.assertNotIn('DASH_RELEASE_RUNNER:', validation)

    def test_should_keep_kotlin_dry_runs_out_of_both_publishing_jobs(self):
        kotlin = (ROOT / '.github/workflows/release-kotlin-sdk.yml').read_text()
        attach = kotlin.split('  attach-release:', 1)[1].split('    steps:', 1)[0]
        maven = kotlin.split('  maven-central-deploy:', 1)[1].split('    steps:', 1)[0]
        self.assertIn('if: ${{ !inputs.dry_run }}', attach)
        self.assertIn('if: ${{ !inputs.dry_run &&', maven)
        self.assertIn("github.ref == format('refs/tags/{0}', inputs.tag)", maven)
        self.assertIn('environment: maven-central', maven)


# Answers `gh api <path> --jq <filter>` with GH_ARTIFACTS through real jq, as gh would; GH_FAIL
# makes the API call fail, and GH_RAW answers with that text instead.
GH_STUB = '''#!/bin/bash
echo "$*" >> "$GH_LOG"
[ -z "${GH_FAIL:-}" ] || { echo 'gh: Server Error (HTTP 502)' >&2; exit 1; }
[ "$1" = api ] && [ "$3" = --jq ] || exit 2
[ -z "${GH_RAW:-}" ] || { echo "$GH_RAW"; exit 0; }
printf '%s' "$GH_ARTIFACTS" | jq -r "$4"
'''


@unittest.skipUnless(shutil.which('jq'), 'needs jq')
class EarlierBuildCheckTests(unittest.TestCase):
    """The npm build skips building when an earlier attempt of the run already uploaded the packages."""

    STEP = 'Check for packages built by an earlier attempt'

    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        (self.tmp / 'bin').mkdir()
        (self.tmp / 'bin/gh').write_text(GH_STUB)
        (self.tmp / 'bin/gh').chmod(0o755)

    def run_check(self, artifacts, **overrides):
        output = self.tmp / 'output'
        output.write_text('')
        env = dict(os.environ, PATH=f"{self.tmp / 'bin'}{os.pathsep}{os.environ['PATH']}",
                   GITHUB_REPOSITORY='dashpay/platform', GITHUB_RUN_ID='123', GITHUB_OUTPUT=str(output),
                   GH_TOKEN='token', ARTIFACT='js-build-abc', GH_LOG=str(self.tmp / 'gh.log'),
                   GH_ARTIFACTS=json.dumps({'artifacts': artifacts}))
        env.update(overrides)
        result = subprocess.run(['bash', '-c', step_script('release-npm-build.yml', self.STEP)],
                                capture_output=True, text=True, env=env)
        return result, output.read_text().splitlines()

    def test_should_report_whether_the_run_holds_the_packages(self):
        live = {'name': 'js-build-abc', 'expired': False}
        for artifacts, exists in [([live], 'true'), ([], 'false'), ([dict(live, expired=True)], 'false')]:
            with self.subTest(artifacts=artifacts):
                result, output = self.run_check(artifacts)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(output, [f'exists={exists}'])
        # asked of the API for this run and name; nothing is downloaded
        self.assertEqual((self.tmp / 'gh.log').read_text().splitlines()[0].split(' --jq ')[0],
                         'api repos/dashpay/platform/actions/runs/123/artifacts?name=js-build-abc')

    def test_should_fail_rather_than_rebuild_when_it_cannot_tell(self):
        for overrides in [{'GH_FAIL': '1'}, {'GH_ARTIFACTS': 'not json'}, {'GH_RAW': 'oops'}, {'GH_RAW': '1oops'},
                          {'GH_ARTIFACTS': '{"artifacts": {}}'}, {'GH_ARTIFACTS': '{"artifacts": [{"name": "x"}]}'}]:
            with self.subTest(overrides=overrides):
                result, output = self.run_check([], **overrides)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(output, [])

    def test_should_keep_the_output_its_later_steps_read(self):
        workflow = (ROOT / '.github/workflows/release-npm-build.yml').read_text()
        step = workflow.split('      - name: ' + self.STEP, 1)[1].split('\n      - ', 1)[0]
        self.assertIn('id: check-artifact', step)
        self.assertIn('ARTIFACT: js-build-${{ github.sha }}', step)
        self.assertEqual(workflow.count("steps.check-artifact.outputs.exists != 'true'"), 4)
        self.assertNotIn('download-artifact', workflow)
        # listing a run's artifacts needs actions: read, granted by the caller too
        self.assertRegex(workflow, r'\npermissions:\n  contents: read\n  actions: read\n')
        caller = (ROOT / '.github/workflows/release.yml').read_text()
        build_npm = caller.split('\n  build-npm:\n', 1)[1].split('\n  release-npm:\n', 1)[0]
        self.assertRegex(build_npm, r'permissions:\n      contents: read\n      actions: read\n')
