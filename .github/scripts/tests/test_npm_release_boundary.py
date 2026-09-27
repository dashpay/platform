"""Execute the protected reusable workflow's guard against caller contexts."""
import os
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[3]


def release_guard():
    workflow = (ROOT / '.github/workflows/release-npm-build.yml').read_text()
    # The guard must run before checkout or any caller-controlled source.
    start = workflow.index('      - name: Reject untrusted release callers')
    end = workflow.index('      - uses: softwareforgood/', start)
    block = workflow[start:end]
    script = block.split('        run: |\n', 1)[1]
    return '\n'.join(line[10:] for line in script.splitlines() if line.strip())


class ReleaseBoundaryTests(unittest.TestCase):
    def run_guard(self, event, ref, repository='dashpay/platform'):
        return subprocess.run(['bash', '-c', release_guard()], capture_output=True, text=True,
                              env=dict(os.environ, GITHUB_REPOSITORY=repository,
                                       GITHUB_EVENT_NAME=event, GITHUB_REF=ref)).returncode

    def test_should_allow_release_tags_and_protected_branch_dry_runs(self):
        for event, ref in [('release', 'refs/tags/v4.2.0-beta.5'),
                           ('workflow_dispatch', 'refs/tags/v4.2.0-beta.5'),
                           ('workflow_dispatch', 'refs/heads/v4.2-dev')]:
            with self.subTest(event=event, ref=ref):
                self.assertEqual(self.run_guard(event, ref), 0)

    def test_should_reject_prs_forks_and_unprotected_dispatches(self):
        for event, ref in [('pull_request', 'refs/pull/5068/merge'),
                           ('pull_request_target', 'refs/heads/v4.2-dev'),
                           ('workflow_dispatch', 'refs/heads/attacker'),
                           ('push', 'refs/heads/v4.2-dev'),
                           ('release', 'refs/heads/v4.2-dev')]:
            with self.subTest(event=event, ref=ref):
                self.assertNotEqual(self.run_guard(event, ref), 0)
        self.assertNotEqual(self.run_guard('release', 'refs/tags/v4.2.0', 'unknown/platform'), 0)
