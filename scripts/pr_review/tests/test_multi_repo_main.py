import contextlib
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock, patch

from scripts.pr_review import main
from scripts.pr_review.tests.test_policy import fixture, NOW


class MultiRepositoryTests(unittest.TestCase):
    def test_same_pr_number_in_different_repositories_keeps_identity(self):
        policy, pr = fixture()
        for repo in ['dashpay/platform', 'dashpay/rust-dashcore']:
            policy['repository'] = repo
            rows = main.evaluate_snapshots(policy, [pr], [pr], [pr], NOW)
            self.assertEqual(rows[0]['repository'], repo)
            self.assertEqual(rows[0]['number'], 1)

    def test_shared_evaluation_preserves_ambiguous_head_blocker(self):
        policy, pr = fixture()
        rows = main.evaluate_snapshots(policy, [pr,dict(pr,number=2)], [pr], [pr], NOW)
        self.assertEqual(rows[0]['status'], 'error')
        self.assertIn('shares this head', rows[0]['blockers'][-1])

    def test_shared_engine_reads_target_checkout_policy(self):
        policy, pr = fixture()
        policy['repository'] = 'dashpay/rust-dashcore'
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'packages/drive').mkdir(parents=True)
            (root / '.github').mkdir()
            (root / '.github/pr-review-policy.json').write_text(json.dumps(policy))
            with patch.object(main,'collect',return_value=([pr],[pr],[pr])), contextlib.redirect_stdout(io.StringIO()) as output:
                self.assertEqual(main.run(['report','--repo',policy['repository'],
                                          '--repository-root',str(root),'--format','json']),0)
            self.assertEqual(json.loads(output.getvalue())['pull_requests'][0]['repository'],policy['repository'])

    def test_cross_repository_writer_token_is_rejected_before_api_calls(self):
        environment = {'GITHUB_ACTIONS':'true','GITHUB_REPOSITORY':'dashpay/platform',
                       'PR_REVIEW_AUTOMATION_ENABLED':'true'}
        with patch.dict(os.environ,environment), patch.object(main,'GitHub') as api, contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit):
                main.run(['sync','--repo','dashpay/rust-dashcore','--apply'])
            api.assert_not_called()


if __name__ == '__main__':
    unittest.main()
