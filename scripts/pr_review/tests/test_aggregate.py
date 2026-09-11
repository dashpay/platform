import base64
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock, patch
from scripts.pr_review import aggregate as a


class AggregateTests(unittest.TestCase):
    def policy(self, repo):
        return {'version': 1, 'repository': repo, 'max_active_prs': 5, 'target_branches': ['dev'],
                'fallback': {'owners': ['Alice'], 'reviewers': []}, 'areas': []}

    def test_repo_numbers_distinct_counts_and_partial_failures(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            entries = []
            for name in ['a', 'b', 'c']:
                repo = 'dashpay/' + name
                (root / (name + '.json')).write_text(json.dumps(self.policy(repo)))
                entries.append({'repository': repo, 'policy': name + '.json', 'mode': 'preview'})
            context = [{'number': x, 'author': 'Alice', 'draft': False, 'state': 'open', 'base': 'dev'} for x in range(3)]
            with patch.object(a.main, 'collect', side_effect=[(context, [], []), (context, [], []), a.GitHubError('unavailable')]):
                with patch.object(a.main, 'evaluate_snapshots', create=True,
                                  side_effect=lambda policy, *args: [{'repository': policy['repository'], 'number': 1, 'author': 'Alice', 'state': 'waiting-bots'}]):
                    data = a.collect_snapshot({'version': 1, 'repositories': entries}, root, lambda repo: Mock())
        self.assertFalse(data['complete'])
        self.assertEqual(len(data['pull_requests']), 2)
        self.assertEqual(data['workload'][0]['total'], 6)
        self.assertTrue(data['workload'][0]['over_limit'])
        self.assertEqual(data['roster'], ['Alice'])
        self.assertFalse(data['repositories'][2]['complete'])

    def test_active_policy_comes_only_from_default_branch(self):
        api = Mock()
        api.request.side_effect = [{'default_branch': 'release/dev'},
                                   {'encoding': 'base64', 'content': base64.b64encode(json.dumps(self.policy('dashpay/a')).encode()).decode()}]
        policy = a.load_policy({'repository': 'dashpay/a', 'mode': 'active', 'policy': 'ignored.json'}, Path('/tmp'), api)
        self.assertEqual(policy['repository'], 'dashpay/a')
        self.assertIn('ref=release%2Fdev', api.request.call_args.args[1])

    def test_registry_rejects_duplicate_repos_and_path_escape(self):
        entry = {'repository': 'dashpay/a', 'policy': '../outside.json', 'mode': 'preview'}
        with self.assertRaises(ValueError):
            a.validate_registry({'version': 1, 'repositories': [entry]})
        entry['policy'] = 'a.json'
        with self.assertRaises(ValueError):
            a.validate_registry({'version': 1, 'repositories': [entry, entry]})

    def test_filtered_scope_cannot_send_partial_shared_digest(self):
        with self.assertRaises(SystemExit):
            a.run(['report', '--repo', 'dashpay/a', '--send-slack'])
        with self.assertRaises(SystemExit):
            a.run(['report', '--user', 'Alice', '--send-slack'])

    def test_active_remote_identity_mismatch_fails_closed(self):
        api = Mock()
        api.request.side_effect = [{'default_branch': 'dev'}, {'encoding': 'base64',
            'content': base64.b64encode(json.dumps(self.policy('dashpay/other')).encode()).decode()}]
        with self.assertRaises(ValueError):
            a.load_policy({'repository': 'dashpay/a', 'mode': 'active'}, Path('/tmp'), api)

    def test_registry_rejects_noninteger_version_unknown_fields_and_bad_entries(self):
        valid = {'repository': 'dashpay/a', 'policy': 'a.json', 'mode': 'preview'}
        invalid = [
            {'version': 1.0, 'repositories': [valid]},
            {'version': True, 'repositories': [valid]},
            {'version': 1, 'repositories': []},
            {'version': 1, 'repositories': [None]},
            {'version': 1, 'repositories': [valid], 'extra': True},
            {'version': 1, 'repositories': [dict(valid, extra=True)]},
        ]
        for registry in invalid:
            with self.subTest(registry=registry), self.assertRaises(ValueError):
                a.validate_registry(registry)
