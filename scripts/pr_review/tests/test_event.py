import unittest

from scripts.pr_review.event import selections


class EventTests(unittest.TestCase):
    def test_schedule_push_and_dispatch_are_bounded(self):
        for kind in ['schedule','push','workflow_dispatch']:
            self.assertEqual(selections(kind,{}),[['--batch-size','3']])

    def test_event_number_only_selects_freshly_refetched_pr(self):
        self.assertEqual(selections('issue_comment',{'issue':{'number':44,'pull_request':{}}}),[['--pr','44']])
        self.assertEqual(selections('issue_comment',{'issue':{'number':44}}),[])
        with self.assertRaises(ValueError):
            selections('pull_request_target',{'pull_request':{'number':'$(secret)'}})

    def test_empty_fork_signal_does_not_trigger_full_sweep(self):
        self.assertEqual(selections('workflow_run',{'workflow_run':{'pull_requests':[]}}),[])


if __name__ == '__main__':
    unittest.main()
