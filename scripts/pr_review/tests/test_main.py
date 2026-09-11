import copy
import os
import unittest
from unittest.mock import Mock, patch

from scripts.pr_review import main
from scripts.pr_review.tests.test_policy import fixture, NOW


class PublicationTests(unittest.TestCase):
    def setUp(self):
        self.pr = {'number': 1, 'author': 'alice', 'head': 'a' * 40,
                   'base': 'v4.2-dev', 'base_sha': 'b' * 40, 'state': 'open',
                   'draft': False, 'labels': [], 'requested_reviewers': [],
                   'controller_comment_id': None, 'reviews': [], 'comments': [],'created_at':NOW}
        self.result = {'number': 1, 'head': 'a' * 40, 'author': 'alice',
                       'state': 'ready-to-merge', 'status': 'success',
                       'blockers': [], 'reviewers': [], 'areas': ['core'],
                       'admitted_at': '2026-09-11T00:00:00Z', 'ready_since': None}
        self.api = Mock()
        self.api.snapshot.return_value = copy.deepcopy(self.pr)
        self.api.open_prs.return_value = [copy.deepcopy(self.pr)]
        self.api.pull.return_value = copy.deepcopy(self.pr)
        self.api.comments.return_value = []
        self.api.activity.return_value = None
        self.policy, _ = fixture()
        self.pr['created_at'] = NOW

    def test_preview_never_mutates_github(self):
        main.publish(self.api, {}, self.pr, self.result, [self.pr], apply=False)
        self.assertEqual(self.api.mock_calls, [])

    def test_review_change_during_publication_cannot_publish_success(self):
        changed = copy.deepcopy(self.pr)
        changed['reviews'] = [{'id': 2, 'state': 'CHANGES_REQUESTED'}]
        self.api.snapshot.side_effect = [copy.deepcopy(self.pr), changed]
        with patch.object(main, 'fingerprint', side_effect=lambda p: repr(p['reviews'])):
            with patch.object(main, 'evaluate', return_value=self.result):
                main.publish(self.api, self.policy, self.pr, self.result, [self.pr], apply=True)
        states = [c.args[1] for c in self.api.post_status.call_args_list]
        self.assertNotIn('success', states)

    def test_new_head_aborts_before_any_mutation(self):
        self.api.pull.return_value['head'] = 'c' * 40
        main.publish(self.api, {}, self.pr, self.result, [self.pr], apply=True)
        self.api.post_status.assert_not_called()
        self.api.upsert_state.assert_not_called()

    def test_user_report_includes_pending_author_work_and_review_requests(self):
        rows = [dict(self.result, state='waiting-bots', title='Owned PR', url='u'),
                dict(self.result, number=2, author='bob', state='ready-for-human', reviewers=['alice'], title='Review PR', url='v')]
        text = main.render_report(rows, '2026-09-11T00:00:00Z', user='alice')
        self.assertIn('Owned PR', text)
        self.assertIn('Review PR', text)
        self.assertIn('waiting-bots', text)

    def test_foreign_controller_history_is_rejected(self):
        state = {'number':2, 'admitted_at':NOW}
        with patch.object(main, 'parse_controller_state', return_value=(state,7)):
            with self.assertRaises(main.GitHubError):
                main.collect(self.api,self.policy)

    def test_collection_failure_revokes_known_heads_only_in_apply(self):
        self.api.snapshot.side_effect = main.GitHubError('missing review evidence')
        with self.assertRaises(main.GitHubError):
            main.collect(self.api,self.policy,apply=True)
        self.assertEqual(self.api.post_status.call_args.args[1], 'error')
        self.api.post_status.reset_mock()
        with self.assertRaises(main.GitHubError):
            main.collect(self.api,self.policy)
        self.api.post_status.assert_not_called()

    def test_excess_admitted_history_is_explicit_error(self):
        candidates = [dict(self.pr,number=n,controller_state={'admitted_at':NOW}) for n in range(1,7)]
        self.assertEqual(main.admission_conflicts(self.policy,candidates), {'alice'})

    def test_changed_other_pr_admission_blocks_success(self):
        other = dict(self.pr,number=2,controller_state=None)
        baseline = [self.pr,other]
        changed = [dict(self.pr,controller_state={'admitted_at':self.result['admitted_at']}),
                   dict(other,controller_state={'admitted_at':NOW})]
        self.api.open_prs.return_value = baseline
        with patch.object(main,'load_histories',side_effect=[baseline,changed]), patch.object(main,'evaluate',return_value=self.result):
            main.publish(self.api,self.policy,self.pr,self.result,baseline,apply=True,candidates=baseline)
        self.assertNotIn('success',[c.args[1] for c in self.api.post_status.call_args_list])

    def test_new_head_before_request_does_not_invite_reviewers(self):
        self.result.update(state='ready-for-human',status='pending',reviewers=['reviewer'])
        other_head = dict(self.pr,head='c'*40)
        self.api.pull.side_effect = [self.pr,self.pr,self.pr,other_head]
        with patch.object(main,'admit',return_value={1:self.result['admitted_at']}):
            main.publish(self.api,self.policy,self.pr,self.result,[self.pr],apply=True)
        self.api.request_reviewers.assert_not_called()

    def test_own_new_admission_is_expected_during_success_revalidation(self):
        baseline = [self.pr]
        after = [dict(self.pr,controller_state={'admitted_at':self.result['admitted_at']})]
        with patch.object(main,'load_histories',side_effect=[baseline,after]), patch.object(main,'evaluate',return_value=self.result):
            main.publish(self.api,self.policy,self.pr,self.result,baseline,apply=True,candidates=baseline)
        self.assertEqual(self.api.post_status.call_args.args[1], 'success')

    def test_event_collection_scopes_to_author_and_closed_trigger_releases_slots(self):
        sibling = dict(self.pr,number=2)
        outsider = dict(self.pr,number=3,author='bob')
        self.api.open_prs.return_value = [sibling,outsider]
        self.api.pull.return_value = dict(self.pr,state='closed')
        _, candidates, _ = main.collect(self.api,self.policy,number=1,reconcile_author=True)
        self.assertEqual([p['number'] for p in candidates],[2])
        self.api.snapshot.assert_called_once_with(2,self.policy)
        self.api.comments.assert_called_once_with(2)

    def test_pr_report_does_not_fetch_sibling_full_evidence(self):
        sibling = dict(self.pr,number=2)
        self.api.open_prs.return_value = [self.pr,sibling]
        _, candidates, _ = main.collect(self.api,self.policy,number=1)
        self.assertEqual(len(candidates),2)
        self.api.snapshot.assert_called_once_with(1,self.policy)

    def test_rotating_periodic_batch_covers_stable_queue(self):
        prs = [dict(self.pr,number=n) for n in range(1,69)]
        seen = set()
        for slot in range(23):
            batch = main.periodic_batch(prs,3,slot*900)
            self.assertEqual(len(batch),3)
            seen.update(p['number'] for p in batch)
        self.assertEqual(seen,set(range(1,69)))

    def test_periodic_collection_bounds_snapshots_and_author_history(self):
        prs = [dict(self.pr,number=n,author=f'user{n}') for n in range(1,69)]
        self.api.open_prs.return_value = prs
        with patch.object(main,'periodic_batch',return_value=prs[:3]):
            _, candidates, _ = main.collect(self.api,self.policy,batch_size=3)
        self.assertEqual(len(candidates),3)
        self.assertEqual(self.api.snapshot.call_count,3)
        self.assertEqual(self.api.comments.call_count,3)

    def test_invalid_configuration_revokes_previous_success_in_authorized_apply(self):
        environment = {'GITHUB_ACTIONS':'true', 'GITHUB_REPOSITORY':'dashpay/platform',
                       'PR_REVIEW_AUTOMATION_ENABLED':'true'}
        with patch.dict(os.environ,environment), patch.object(main,'GitHub',return_value=self.api), \
                patch.object(main,'POLICY') as policy_path:
            policy_path.read_text.return_value = '{broken json'
            with self.assertRaises(ValueError):
                main.run(['sync','--apply'])
        self.assertEqual(self.api.post_status.call_args.args[1],'error')

    def test_invalid_configuration_preview_never_revokes_status(self):
        with patch.object(main,'GitHub',return_value=self.api), patch.object(main,'POLICY') as policy_path:
            policy_path.read_text.return_value = '{broken json'
            with self.assertRaises(ValueError):
                main.run(['sync'])
        self.api.post_status.assert_not_called()

    def test_event_does_not_rescan_unchanged_author_pr_reviews(self):
        prs = [dict(self.pr,number=n,controller_state={'number':n,'admitted_at':NOW} if n<=5 else None)
               for n in range(1,15)]
        self.api.open_prs.return_value = prs
        with patch.object(main,'load_histories',return_value=prs):
            main.collect(self.api,self.policy,number=2,reconcile_author=True)
        self.assertEqual([call.args[0] for call in self.api.snapshot.call_args_list],[2])

    def test_close_event_refreshes_newly_admitted_waiter(self):
        prs = [dict(self.pr,number=n,controller_state={'number':n,'admitted_at':NOW} if n<=5 else None)
               for n in range(1,8) if n!=2]
        self.api.open_prs.return_value = prs
        self.api.pull.return_value = dict(self.pr,number=2,state='closed')
        with patch.object(main,'load_histories',return_value=prs):
            main.collect(self.api,self.policy,number=2,reconcile_author=True)
        self.assertEqual([call.args[0] for call in self.api.snapshot.call_args_list],[6])

    def test_noop_success_rechecks_reviews_after_admission_history_reads(self):
        self.pr['controller_state'] = main.state_record(self.pr,self.result,main.context_fingerprint([self.pr]))
        changed = dict(self.pr,reviews=[{'id':9,'state':'DISMISSED'}])
        self.api.snapshot.side_effect = [self.pr,changed]
        with patch.object(main,'load_histories',return_value=[self.pr]), patch.object(main,'evaluate',return_value=self.result):
            main.publish(self.api,self.policy,self.pr,self.result,[self.pr],apply=True,candidates=[self.pr])
        self.assertNotIn('success',[c.args[1] for c in self.api.post_status.call_args_list])


if __name__ == '__main__':
    unittest.main()
