import unittest
from unittest.mock import patch
from scripts.pr_review import notifications as n


def snapshot():
    return {'generated_at': '2026-09-11T00:00:00Z', 'complete': True,
            'repositories': [{'repository': 'dashpay/a', 'complete': True}],
            'roster': ['Alice', 'ALICE2', 'Bob'], 'workload': [],
            'pull_requests': [
                {'repository': 'dashpay/a', 'number': 1, 'author': 'Bob', 'state': 'ready-for-human',
                 'reviewers': ['Alice'], 'title': '<!channel>', 'url': 'https://github.com/dashpay/a/pull/1'},
                {'repository': 'dashpay/b', 'number': 1, 'author': 'Alice2', 'state': 'waiting-bots',
                 'reviewers': ['Bob'], 'blockers': ['bot not complete'], 'title': 'blocked'}]}


class NotificationsTests(unittest.TestCase):
    def config(self):
        return {'version': 1, 'channel_id': 'C123', 'users': {'Alice': 'U123', 'ALICE2': 'U123', 'Bob': 'U456'}}

    def test_one_dm_for_shared_identity_and_no_premature_invitation(self):
        plan = n.build_delivery_plan(snapshot(), self.config())
        self.assertEqual(plan['errors'], [])
        self.assertEqual([m['destination'] for m in plan['messages']], ['C123', 'U123'])
        text = str(plan['messages'][1])
        self.assertIn('dashpay/a#1', text)
        self.assertIn('dashpay/b#1', text)
        self.assertIn('Your PR blockers', text)
        self.assertTrue(all(b['text']['type'] == 'plain_text' for b in plan['messages'][1]['payload']['blocks']))

    def test_missing_mapping_is_visible_and_blocks_all_delivery(self):
        config = self.config()
        config['users']['Alice'] = None
        plan = n.build_delivery_plan(snapshot(), config)
        self.assertTrue(any('Alice' in e for e in plan['errors']))
        with patch.dict('os.environ', {'PR_REVIEW_SLACK_ENABLED': 'true', 'PR_REVIEW_SLACK_BOT_TOKEN': 'secret'}):
            with patch.object(n.urllib.request, 'urlopen') as request:
                with self.assertRaises(ValueError):
                    n.deliver(plan)
                request.assert_not_called()

    def test_unconfigured_destinations_still_allow_reviewing_exact_message_text(self):
        config = {'version':1,'channel_id':None,'users':{'Alice':None,'ALICE2':None,'Bob':None}}
        plan = n.build_delivery_plan(snapshot(),config)
        self.assertTrue(plan['errors'])
        self.assertTrue(any(m['kind']=='channel' for m in plan['messages']))
        personal = [m for m in plan['messages'] if m['kind']=='personal']
        self.assertTrue(any('alice' in m['github_logins'] for m in personal))
        self.assertIn('dashpay/a#1',str(plan['messages']))
        self.assertTrue(all(m['destination'] is None for m in plan['messages']))

    def test_partial_failure_shown_without_claiming_empty(self):
        data = snapshot()
        data['complete'] = False
        data['repositories'][0].update(complete=False, error='unavailable')
        plan = n.build_delivery_plan(data, self.config())
        self.assertIn('unavailable', str(plan['messages']))

    def test_ambiguous_delivery_stops_without_retry_or_secret(self):
        plan = n.build_delivery_plan(snapshot(), self.config())
        with patch.dict('os.environ', {'PR_REVIEW_SLACK_ENABLED': 'true', 'PR_REVIEW_SLACK_BOT_TOKEN': 'secret'}):
            with patch.object(n.urllib.request, 'urlopen', side_effect=TimeoutError('secret')) as request:
                result = n.deliver(plan)
        self.assertEqual(request.call_count, 1)
        self.assertEqual(result[0]['state'], 'uncertain')
        self.assertEqual(result[1]['state'], 'not-attempted')
        self.assertNotIn('secret', str(result))

    def test_success_is_not_repeated_when_later_delivery_is_rejected(self):
        from unittest.mock import MagicMock
        first = MagicMock()
        first.__enter__.return_value.status = 200
        first.__enter__.return_value.read.return_value = b'{"ok":true,"ts":"123.456"}'
        second = MagicMock()
        second.__enter__.return_value.status = 200
        second.__enter__.return_value.read.return_value = b'{"ok":false,"error":"invalid_auth"}'
        plan = n.build_delivery_plan(snapshot(), self.config())
        with patch.dict('os.environ', {'PR_REVIEW_SLACK_ENABLED': 'true', 'PR_REVIEW_SLACK_BOT_TOKEN': 'secret'}):
            with patch.object(n.urllib.request, 'urlopen', side_effect=[first, second]) as request:
                result = n.deliver(plan)
        self.assertEqual(request.call_count, 2)
        self.assertEqual([r['state'] for r in result], ['delivered', 'error'])
        self.assertEqual(result[0]['timestamp'], '123.456')

    def test_empty_personal_digest_is_skipped(self):
        data = snapshot()
        data['pull_requests'] = []
        plan = n.build_delivery_plan(data, self.config())
        self.assertEqual([m['kind'] for m in plan['messages']], ['channel'])

    def test_delivery_disabled_by_default(self):
        with patch.dict('os.environ', {}, clear=True):
            with patch.object(n.urllib.request, 'urlopen') as request:
                with self.assertRaises(ValueError):
                    n.deliver(n.build_delivery_plan(snapshot(), self.config()))
                request.assert_not_called()

    def test_channel_includes_author_blockers_without_review_invitation(self):
        plan = n.build_delivery_plan(snapshot(), self.config())
        channel = str(plan['messages'][0])
        self.assertIn('bot not complete', channel)
        self.assertIn('Author action needed', channel)

    def test_slack_config_rejects_noninteger_version_and_unknown_fields(self):
        for config in [dict(self.config(), version=1.0), dict(self.config(), version=True),
                       dict(self.config(), extra=True)]:
            with self.subTest(config=config):
                plan = n.build_delivery_plan(snapshot(), config)
                self.assertTrue(plan['errors'])
                self.assertEqual(plan['messages'], [])

    def test_preview_reviews_are_described_as_computed_not_requested(self):
        plan = n.build_delivery_plan(snapshot(), self.config())
        text = str(plan['messages'][1])
        self.assertIn('Reviews awaiting you (computed policy)', text)
        self.assertNotIn('Your requested reviews', text)
