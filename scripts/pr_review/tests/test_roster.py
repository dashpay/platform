"""Pin the responsibility decisions that differ from the original team roster."""

import json
from pathlib import Path
import unittest

from scripts.pr_review.policy import codeowners, validate_policy


class RosterTests(unittest.TestCase):
    def test_sheet_roles_do_not_promote_reviewers_or_broad_teams(self):
        root = Path(__file__).resolve().parents[3]
        policy = json.loads((root / '.github/pr-review-policy.json').read_text())
        validate_policy(policy, root)
        self.assertEqual(policy['fallback'], {
            'owners': ['QuantumExplorer', 'shumkov'], 'reviewers': []})
        paths = {path: area for area in policy['areas'] for path in area['paths']}
        self.assertEqual(paths['packages/rs-drive/']['owners'], ['QuantumExplorer'])
        self.assertEqual(paths['packages/rs-drive/']['reviewers'], ['shumkov'])
        self.assertEqual(paths['packages/rs-platform-wallet/']['owners'], ['llbartekll'])
        self.assertEqual(set(paths['packages/rs-platform-wallet/']['reviewers']),
                         {'ZocoLini', 'HashEngineering', 'romchornyi'})
        self.assertEqual(paths['packages/rs-dapi/']['owners'], ['lklimek'])
        self.assertTrue(paths['packages/dashmate/']['unresolved'])
        generated = codeowners(policy)
        self.assertNotIn('@dashpay/', generated)
        self.assertNotIn('@strophy', generated.lower())
        self.assertNotIn('@silvanassss', generated.lower())
        self.assertNotIn('/.github/workflows/', generated)
        self.assertEqual((root / 'CODEOWNERS').read_text(), generated)


if __name__ == '__main__':
    unittest.main()
