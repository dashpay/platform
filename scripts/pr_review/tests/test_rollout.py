from pathlib import Path
import tempfile
import unittest

from scripts.pr_review.rollout import write_bundle
from scripts.pr_review.main import run


class RolloutTests(unittest.TestCase):
    def test_bundle_uses_pinned_shared_engine_and_target_policy(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder) / 'packet'
            write_bundle('dashpay/tenderdash','a'*40,root)
            workflow = (root / '.github/workflows/pr-review-policy.yml').read_text()
            self.assertIn('pr-review-reusable.yml@'+'a'*40,workflow)
            self.assertIn('engine_revision: '+'a'*40,workflow)
            self.assertIn('"dashpay/tenderdash"',(root / '.github/pr-review-policy.json').read_text())
            self.assertIn('* @lklimek @shumkov',(root / '.github/CODEOWNERS').read_text())
            self.assertFalse((root / 'scripts').exists())

    def test_generated_bundle_passes_effective_codeowners_check(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder) / 'packet'
            write_bundle('dashpay/tenderdash', 'a' * 40, root)
            (root / 'CODEOWNERS').write_text('obsolete overridden root file')
            self.assertEqual(run(['codeowners', '--check', '--repo', 'dashpay/tenderdash',
                                   '--repository-root', str(root)]), 0)

    def test_bundle_never_overwrites_existing_work(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder)
            (path / 'existing').write_text('preserve')
            with self.assertRaises(ValueError):
                write_bundle('dashpay/tenderdash','a'*40,path)
            self.assertEqual((path / 'existing').read_text(),'preserve')

    def test_bundle_rejects_unpinned_engine(self):
        with tempfile.TemporaryDirectory() as folder:
            with self.assertRaises(ValueError):
                write_bundle('dashpay/tenderdash','main',Path(folder)/'packet')


if __name__ == '__main__':
    unittest.main()
