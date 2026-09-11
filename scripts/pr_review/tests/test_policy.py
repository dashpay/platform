import copy
import unittest

from scripts.pr_review.policy import admit, codeowners, evaluate, fingerprint, validate_policy


HEAD = 'a' * 40
NOW = '2026-09-11T12:00:00Z'


def fixture():
    policy = dict(version=1, repository='dashpay/platform', max_active_prs=5,
                  target_branches=['v4.2-dev'], fallback={'owners': ['fallback'], 'reviewers': []},
                  areas=[dict(id='drive', paths=['packages/drive/'], owners=['owner'], reviewers=['reviewer'])])
    pr = dict(number=1, author='owner', head=HEAD, base='v4.2-dev', base_sha='b'*40,
              created_at='2026-09-10T00:00:00Z', draft=False, state='open', url='url', title='title',
              files=[{'filename': 'packages/drive/a.rs'}],
              reviews=[dict(id=1,user='thepastaclaw',state='COMMENTED',commit_id=HEAD,
                            submitted_at='2026-09-11T10:00:00Z',
                            body=f'<!-- thepastaclaw-review-phase v1 phase=final sha={HEAD} -->'),
                       dict(id=2,user='coderabbitai[bot]',state='APPROVED',commit_id=HEAD,
                            submitted_at='2026-09-11T10:00:00Z',body='')],
              comments=[dict(id=3,user='owner',body=f'/self-reviewed {HEAD}',
                             created_at='2026-09-11T11:00:00Z',updated_at='2026-09-11T11:00:00Z')],
              threads=[], requested_reviewers=[], permissions={'owner':'write','reviewer':'write','fallback':'write'},
              controller_state=None, labels=[], complete=True)
    return policy, pr


class PolicyTests(unittest.TestCase):
    def test_owner_exemption_does_not_apply_to_reviewer(self):
        p, pr = fixture()
        self.assertEqual(evaluate(p, pr, NOW, NOW)['status'], 'success')
        pr['author'] = pr['comments'][0]['user'] = 'reviewer'
        result = evaluate(p, pr, NOW, NOW)
        self.assertEqual(result['state'], 'ready-for-human')
        self.assertEqual(result['reviewers'], ['owner'])

    def test_rename_requires_source_and_destination_coverage(self):
        p, pr = fixture()
        pr['files'] = [{'filename':'packages/drive/a.rs', 'previous_filename':'root.rs'}]
        self.assertEqual(evaluate(p,pr,NOW,NOW)['reviewers'], ['fallback'])

    def test_bot_failure_blocks_even_owner(self):
        p, pr = fixture()
        for update in [{'state':'CHANGES_REQUESTED'}, {'commit_id':'c'*40}, {'body':'preliminary'}]:
            changed = copy.deepcopy(pr)
            changed['reviews'][0].update(update)
            self.assertEqual(evaluate(p,changed,NOW,NOW)['state'], 'waiting-bots')

    def test_self_review_requires_unedited_author_confirmation_after_bots(self):
        p, pr = fixture()
        for update in [{'user':'reviewer'}, {'body':'/self-reviewed old'},
                       {'updated_at':NOW}, {'created_at':'2026-09-11T09:00:00Z','updated_at':'2026-09-11T09:00:00Z'}]:
            changed = copy.deepcopy(pr)
            changed['comments'][0].update(update)
            self.assertEqual(evaluate(p,changed,NOW,NOW)['state'], 'waiting-self-review')

    def test_fixed_owner_pr_returns_to_objecting_reviewers_queue(self):
        p, pr = fixture()
        pr['reviews'].append(dict(id=4,user='reviewer',state='CHANGES_REQUESTED',commit_id='c'*40,
                                 submitted_at='2026-09-11T10:30:00Z',body=''))
        self.assertEqual(evaluate(p,pr,NOW,NOW)['reviewers'], ['reviewer'])
        pr['reviews'][-1]['submitted_at'] = '2026-09-11T11:30:00Z'
        self.assertEqual(evaluate(p,pr,NOW,NOW)['state'], 'waiting-author')

    def test_dismissal_and_stale_approval_do_not_satisfy_human_area(self):
        p, pr = fixture()
        pr['author'] = pr['comments'][0]['user'] = 'reviewer'
        review = dict(id=4,user='owner',state='APPROVED',commit_id=HEAD,submitted_at=NOW,body='')
        pr['reviews'].append(review)
        self.assertEqual(evaluate(p,pr,NOW,NOW)['status'], 'success')
        review['state'] = 'DISMISSED'
        self.assertEqual(evaluate(p,pr,NOW,NOW)['status'], 'pending')
        review.update(state='APPROVED',commit_id='c'*40)
        self.assertEqual(evaluate(p,pr,NOW,NOW)['status'], 'pending')

    def test_sixth_waits_without_blocking_admitted_five(self):
        p, pr = fixture()
        prs = [dict(copy.deepcopy(pr),number=n) for n in range(1,7)]
        prs[-1]['controller_state'] = {'admitted_at':'2026-09-10T01:00:00Z'}
        slots = admit(p,prs,NOW)
        self.assertEqual(set(slots), {1,2,3,4,6})
        self.assertEqual(evaluate(p,prs[4],slots.get(5),NOW)['state'], 'waiting-slot')

    def test_incomplete_or_unresolved_never_succeeds(self):
        p, pr = fixture()
        pr['complete'] = False
        self.assertEqual(evaluate(p,pr,NOW,NOW)['status'], 'error')
        pr['complete'] = True
        p['areas'][0]['unresolved'] = ['Daniel']
        self.assertEqual(evaluate(p,pr,NOW,NOW)['state'], 'configuration-error')

    def test_integer_configuration_does_not_accept_float_lookalikes(self):
        for key,value in [('version',1.0),('max_active_prs',5.0)]:
            policy,_ = fixture()
            policy[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                validate_policy(policy)

    def test_validation_rejects_overlap_and_excluded_identity(self):
        p, _ = fixture()
        validate_policy(p)
        self.assertIn('* @fallback',codeowners(p))
        p['areas'][0]['paths'].append('packages/drive/nested/')
        with self.assertRaises(ValueError): validate_policy(p)
        p, _ = fixture()
        p['areas'][0]['reviewers'] = ['strophy']
        with self.assertRaises(ValueError): validate_policy(p)

    def test_fingerprint_ignores_controller_effects_but_not_evidence(self):
        _, pr = fixture()
        original = fingerprint(pr)
        pr['labels'] = ['ready-for-human']
        pr['requested_reviewers'] = ['reviewer']
        pr['comments'].append(dict(user='github-actions[bot]',body='<!-- platform-pr-review-state-v1 -->'))
        self.assertEqual(original,fingerprint(pr))
        pr['reviews'][0]['state'] = 'CHANGES_REQUESTED'
        self.assertNotEqual(original,fingerprint(pr))

    def test_edited_bot_summary_requires_fresh_author_confirmation(self):
        p, pr = fixture()
        pr['reviews'] = pr['reviews'][:1]
        pr['comments'].append(dict(id=8,user='coderabbitai[bot]',created_at='2026-09-11T10:00:00Z',
                                  updated_at='2026-09-11T10:00:00Z',
                                  body='<!-- final_review_risk_coverage:{"kind":"reviewed","coveredCommitId":"'+HEAD+'"} -->'))
        self.assertEqual(evaluate(p,pr,NOW,NOW)['status'], 'success')
        pr['comments'][-1]['updated_at'] = NOW
        self.assertEqual(evaluate(p,pr,NOW,NOW)['state'], 'waiting-self-review')
        pr['comments'][-1]['body'] = 'Skipped review'
        self.assertEqual(evaluate(p,pr,NOW,NOW)['state'], 'waiting-bots')

    def test_unresolved_bot_and_human_threads_have_different_queue_effects(self):
        p, pr = fixture()
        pr['threads'] = [dict(id='thread',is_resolved=False,author='reviewer',created_at='2026-09-11T10:00:00Z')]
        self.assertEqual(evaluate(p,pr,NOW,NOW)['state'], 'ready-for-human')
        pr['threads'][0]['author'] = 'thepastaclaw'
        self.assertEqual(evaluate(p,pr,NOW,NOW)['state'], 'waiting-bots')

    def test_comment_review_does_not_erase_decisive_approval(self):
        p, pr = fixture()
        pr['author'] = pr['comments'][0]['user'] = 'reviewer'
        pr['reviews'].extend([dict(id=8,user='owner',state='APPROVED',commit_id=HEAD,submitted_at=NOW,body=''),
                              dict(id=9,user='owner',state='COMMENTED',commit_id=HEAD,submitted_at=NOW,body='')])
        self.assertEqual(evaluate(p,pr,NOW,NOW)['status'], 'success')

    def test_roster_permission_loss_blocks_owner_exemption(self):
        p, pr = fixture()
        pr['permissions']['reviewer'] = 'read'
        self.assertEqual(evaluate(p,pr,NOW,NOW)['state'], 'configuration-error')

    def test_ready_age_only_carries_on_same_head(self):
        p, pr = fixture()
        pr['author'] = pr['comments'][0]['user'] = 'reviewer'
        previous = '2026-09-11T11:30:00Z'
        pr['controller_state'] = dict(admitted_at=previous,head=HEAD,state='ready-for-human',ready_since=previous)
        self.assertEqual(evaluate(p,pr,NOW,NOW)['ready_since'], previous)
        pr['controller_state']['head'] = 'b'*40
        self.assertEqual(evaluate(p,pr,NOW,NOW)['ready_since'], NOW)

    def test_draft_releases_sticky_slot_and_retargeted_pr_is_excluded(self):
        p, pr = fixture()
        prs = [dict(copy.deepcopy(pr),number=n) for n in range(1,7)]
        prs[0]['draft'] = True
        prs[0]['controller_state'] = {'admitted_at':NOW}
        self.assertEqual(set(admit(p,prs,NOW)), {2,3,4,5,6})
        prs[1]['base'] = 'another-branch'
        self.assertNotIn(2,admit(p,prs,NOW))

    def test_new_bot_completion_requires_author_to_review_latest_outcome(self):
        p, pr = fixture()
        pr['reviews'].append(dict(pr['reviews'][1],id=20,submitted_at=NOW))
        self.assertEqual(evaluate(p,pr,NOW,NOW)['state'], 'waiting-self-review')

    def test_controller_comment_creation_does_not_invalidate_evidence(self):
        _, pr = fixture()
        before = fingerprint(pr)
        pr['controller_comment_id'] = 55
        self.assertEqual(fingerprint(pr), before)

    def test_outside_area_writer_objection_blocks_owner_and_requests_rereview(self):
        p, pr = fixture()
        pr['permissions']['maintainer'] = 'write'
        pr['reviews'].append(dict(id=40,user='maintainer',state='CHANGES_REQUESTED',commit_id=HEAD,
                                 submitted_at='2026-09-11T10:30:00Z',body=''))
        result = evaluate(p,pr,NOW,NOW)
        self.assertEqual(result['state'], 'ready-for-human')
        self.assertEqual(result['reviewers'], ['maintainer'])

    def test_reopened_pr_does_not_retain_slot_from_previous_open_cycle(self):
        p, pr = fixture()
        prs = [dict(copy.deepcopy(pr),number=n) for n in range(1,7)]
        prs[-1]['controller_state'] = {'admitted_at':'2026-09-10T01:00:00Z'}
        prs[-1]['lifecycle_at'] = '2026-09-11T01:00:00Z'
        self.assertEqual(set(admit(p,prs,NOW)),{1,2,3,4,5})

    def test_coderabbit_quoted_marker_text_is_not_a_completion_receipt(self):
        p, pr = fixture()
        pr['reviews'] = pr['reviews'][:1]
        marker = 'final_review_risk_coverage:{"kind":"reviewed","coveredCommitId":"'+HEAD+'"}'
        for body in [marker, 'Producer example: '+marker,
                     '> <!-- '+marker+' -->', '<!-- '+marker+' trailing garbage -->']:
            changed = copy.deepcopy(pr)
            changed['comments'].append(dict(id=8,user='coderabbitai[bot]',body=body,
                                           created_at='2026-09-11T10:00:00Z',updated_at='2026-09-11T10:00:00Z'))
            with self.subTest(body=body):
                self.assertEqual(evaluate(p,changed,NOW,NOW)['state'],'waiting-bots')


    def test_whole_repository_owner_covers_root_files_and_cross_directory_rename(self):
        policy, pr = fixture()
        policy['areas'][0]['paths'] = ['']
        validate_policy(policy)
        pr['files'] = [{'filename': 'README.md', 'previous_filename': 'nested/old.md'}]
        self.assertEqual(evaluate(policy, pr, NOW, NOW)['status'], 'success')
        self.assertEqual(evaluate(policy, pr, NOW, NOW)['areas'], ['drive'])
        self.assertEqual(codeowners(policy).splitlines()[-1], '* @owner @reviewer')

    def test_whole_repository_prefix_cannot_overlap_other_areas(self):
        policy, _ = fixture()
        policy['areas'].append(dict(id='whole', paths=[''], owners=['whole'], reviewers=[]))
        with self.assertRaisesRegex(ValueError, 'Overlapping'):
            validate_policy(policy)

    def test_named_area_without_owner_is_explicit_configuration_gap(self):
        policy, pr = fixture()
        policy['areas'][0].update(owners=[], unresolved=['Owner missing from source sheet'])
        validate_policy(policy)
        pr['author'] = pr['comments'][0]['user'] = 'fallback'
        result = evaluate(policy, pr, NOW, NOW)
        self.assertEqual(result['status'], 'error')
        self.assertIn('Unresolved identities in drive', result['blockers'])
        self.assertEqual(codeowners(policy).splitlines()[-1], '/packages/drive/ @reviewer')

    def test_empty_unresolved_area_never_emits_native_owner_suppression(self):
        policy, _ = fixture()
        policy['areas'][0].update(paths=[''], owners=[], reviewers=[], unresolved=['Owner missing'])
        validate_policy(policy)
        self.assertEqual([line for line in codeowners(policy).splitlines() if not line.startswith('#')], ['* @fallback'])

    def test_empty_owner_without_explicit_gap_is_rejected(self):
        for unresolved in [None, [], [''], 'missing']:
            policy, _ = fixture()
            policy['areas'][0]['owners'] = []
            if unresolved is not None:
                policy['areas'][0]['unresolved'] = unresolved
            with self.subTest(unresolved=unresolved), self.assertRaises(ValueError):
                validate_policy(policy)


if __name__ == '__main__':
    unittest.main()
